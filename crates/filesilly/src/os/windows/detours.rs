use crate::os::windows::util::{io_error_to_status, normalize_unicode_string_path};
use crate::os::windows::{util, WrappedHandle};
use crate::hook::{LockDetour, SillyHook};
use crate::recursion_guard::RecursionGuard;
use crate::{FileSilly, HeapStream, Stream};
use std::ffi::OsString;
use std::io::SeekFrom;
use std::os::raw::{c_ulong, c_void};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{io, slice};
use tracing::{error, field, trace, trace_span, warn};
use windows::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows::Wdk::Storage::FileSystem::{
    FileBasicInformation, FileFsDeviceInformation, FileFsFullSizeInformationEx,
    FileFsSizeInformation, FileFsVolumeInformation, FilePositionInformation, FileStandardInformation,
    FILE_BASIC_INFORMATION, FILE_INFORMATION_CLASS, FILE_POSITION_INFORMATION,
    FILE_STANDARD_INFORMATION, FS_INFORMATION_CLASS,
};
use windows::Wdk::System::SystemServices::{
    FILE_FS_DEVICE_INFORMATION, FILE_FS_FULL_SIZE_INFORMATION_EX, FILE_FS_SIZE_INFORMATION,
    FILE_FS_VOLUME_INFORMATION, FILE_VIRTUAL_VOLUME,
};
use windows::Win32::Foundation::{
    HANDLE, NTSTATUS, STATUS_INFO_LENGTH_MISMATCH, STATUS_INTERNAL_ERROR,
    STATUS_INVALID_INFO_CLASS, STATUS_INVALID_PARAMETER, STATUS_SUCCESS,
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, FILE_DEVICE_DISK};
use windows::Win32::System::WindowsProgramming::FILE_OPENED;
use windows::Win32::System::IO::{IO_STATUS_BLOCK, PIO_APC_ROUTINE};

type NtCreateFileFn = unsafe extern "system" fn(
    handle: *mut HANDLE,
    desired_access: u32,
    object_attributes: *const OBJECT_ATTRIBUTES,
    io_status_block: *mut IO_STATUS_BLOCK,
    allocation_size: *const u64,
    file_attributes: u32,
    share_access: u32,
    create_disposition: u32,
    create_options: u32,
    ea_buffer: *const c_void,
    ea_length: u32,
) -> NTSTATUS;

pub static NT_CREATE_FILE_HOOK: LockDetour<NtCreateFileFn> =
    LazyLock::new(|| SillyHook::new(c"ntdll.dll", c"NtCreateFile", create_file_detour));

thread_local! {
    static CREATE_FILE_RECURSION_GUARD: RecursionGuard = const { RecursionGuard::new() };
}

unsafe extern "system" fn create_file_detour(
    out_handle: *mut HANDLE,
    desired_access: u32,
    object_attributes: *const OBJECT_ATTRIBUTES,
    io_status_block: *mut IO_STATUS_BLOCK,
    allocation_size: *const u64,
    file_attributes: u32,
    share_access: u32,
    create_disposition: u32,
    create_options: u32,
    ea_buffer: *const c_void,
    ea_length: u32,
) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtCreateFile",
        handle = field::Empty, status = field::Empty
    );
    let _entered = span.enter();

    let Some(result) = CREATE_FILE_RECURSION_GUARD
        .try_with(|r| match r.acquire() {
            Some(_guard) => unsafe { create_file_handler(object_attributes) },
            None => {
                trace!("re-entrant call to NtCreateFile");
                None
            }
        })
        .ok()
        .flatten()
    else {
        // forward unmodified call
        unsafe {
            return NT_CREATE_FILE_HOOK.unwrap().call(
                out_handle,
                desired_access,
                object_attributes,
                io_status_block,
                allocation_size,
                file_attributes,
                share_access,
                create_disposition,
                create_options,
                ea_buffer,
                ea_length,
            );
        }
    };

    let io_status_block = unsafe { io_status_block.as_mut_unchecked() };

    match result {
        Ok(handle) => {
            span.record("handle", field::debug(handle));
            span.record("status", field::debug(STATUS_SUCCESS));

            if !out_handle.is_null() {
                unsafe {
                    *out_handle = handle;
                }
            }

            io_status_block.Anonymous.Status = STATUS_SUCCESS;
            io_status_block.Information = FILE_OPENED as usize;
            STATUS_SUCCESS
        }

        Err(status) => {
            span.record("status", field::debug(status));

            io_status_block.Anonymous.Status = status;
            status
        }
    }
}

unsafe fn create_file_handler(
    object_attributes: *const OBJECT_ATTRIBUTES,
) -> Option<Result<HANDLE, NTSTATUS>> {
    // katie: A real Windows system (at least the copy of Windows 10 that I have installed) always
    // passes an NT object manager path. However, Wine tends to pass other paths, including root
    // local device paths (starting \\?\) and regular Win32 paths (like C:\Whatever).
    let path = unsafe {
        let object_attributes = &*object_attributes;
        assert_eq!(
            object_attributes.Length as usize,
            size_of::<OBJECT_ATTRIBUTES>()
        );

        if !object_attributes.RootDirectory.is_invalid() {
            // TODO: we can resolve the directory path with GetFinalPathNameByHandle but this
            //  hasn't come up in testing
            return None;
        }

        let input_path = &*object_attributes.ObjectName;
        normalize_unicode_string_path(input_path)?
    };

    // Path should always be shaped like an NT object manager path now.
    let base_path = &FileSilly::platform().base_path_nt;
    let relative_path = path.strip_prefix(&base_path[..])?;
    let relative_path = PathBuf::from(OsString::from_wide(relative_path));
    let fixed_path = FileSilly::platform().base_path.join(&relative_path);

    let result = FileSilly::factory().create_stream(&fixed_path);

    let stream = match result {
        Ok(None) => return None,
        Ok(Some(stream)) => stream,
        Err(err) => {
            error!(?err, "stream factory returned an error");
            let status = io_error_to_status(&err);
            return Some(Err(status));
        }
    };

    Some(
        match FileSilly::platform().allocate_handle_for_stream(stream) {
            Ok(handle) => Ok(handle.0),
            Err(err) => {
                error!(?err, "failed to generate fake handle");
                return Some(Err(STATUS_INTERNAL_ERROR));
            }
        },
    )
}

type NtCloseFn = unsafe extern "system" fn(object: HANDLE) -> NTSTATUS;
pub static NT_CLOSE_HOOK: LockDetour<NtCloseFn> =
    LazyLock::new(|| SillyHook::new(c"ntdll.dll", c"NtClose", close_handle_detour));

unsafe extern "system" fn close_handle_detour(handle: HANDLE) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtClose",
        handle = ?handle
    );
    let _entered = span.enter();

    let platform = FileSilly::platform();

    if let Some((_, stream)) = platform.handles.remove(&WrappedHandle(handle)) {
        trace!("closed handle");
        drop(stream);
    }

    unsafe { NT_CLOSE_HOOK.unwrap().call(handle) }
}

type NtReadWriteFileFn = unsafe extern "system" fn(
    raw_handle: HANDLE,
    event_handle: HANDLE,
    apc_routine: PIO_APC_ROUTINE,
    apc_context: *const c_void,
    io_status_block: *mut IO_STATUS_BLOCK,
    buffer: *mut u8,
    length: c_ulong,
    byte_offset: *const i64,
    key: *const c_ulong,
) -> NTSTATUS;

pub static NT_READ_FILE_HOOK: LockDetour<NtReadWriteFileFn> =
    LazyLock::new(|| SillyHook::new(c"ntdll.dll", c"NtReadFile", read_file_detour));

unsafe extern "system" fn read_file_detour(
    handle: HANDLE,
    event_handle: HANDLE,
    apc_routine: PIO_APC_ROUTINE,
    apc_context: *const c_void,
    io_status_block: *mut IO_STATUS_BLOCK,
    buffer: *mut u8,
    length: c_ulong,
    byte_offset: *const i64,
    key: *const c_ulong,
) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtReadFile",
        handle = ?handle, length = %length
    );
    let _entered = span.enter();

    let Some((status, information)) = (unsafe {
        read_write_file_handler(
            handle,
            event_handle,
            apc_routine,
            buffer,
            length,
            byte_offset,
            key,
            |stream, buffer| stream.read(buffer),
        )
    }) else {
        // forward unmodified call
        unsafe {
            return NT_READ_FILE_HOOK.unwrap().call(
                handle,
                event_handle,
                apc_routine,
                apc_context,
                io_status_block,
                buffer,
                length,
                byte_offset,
                key,
            );
        }
    };

    span.record("status", field::debug(status));

    unsafe {
        let r = &mut *io_status_block;
        r.Anonymous.Status = status;
        r.Information = information;
    }

    status
}

pub static NT_WRITE_FILE_HOOK: LockDetour<NtReadWriteFileFn> =
    LazyLock::new(|| SillyHook::new(c"ntdll.dll", c"NtWriteFile", write_file_detour));

unsafe extern "system" fn write_file_detour(
    handle: HANDLE,
    event_handle: HANDLE,
    apc_routine: PIO_APC_ROUTINE,
    apc_context: *const c_void,
    io_status_block: *mut IO_STATUS_BLOCK,
    buffer: *mut u8,
    length: c_ulong,
    byte_offset: *const i64,
    key: *const c_ulong,
) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtWriteFile",
        handle = ?handle, length = %length
    );
    let _entered = span.enter();

    let Some((status, information)) = (unsafe {
        read_write_file_handler(
            handle,
            event_handle,
            apc_routine,
            buffer,
            length,
            byte_offset,
            key,
            |stream, buffer| stream.write(buffer),
        )
    }) else {
        // forward unmodified call
        unsafe {
            return NT_WRITE_FILE_HOOK.unwrap().call(
                handle,
                event_handle,
                apc_routine,
                apc_context,
                io_status_block,
                buffer,
                length,
                byte_offset,
                key,
            );
        }
    };

    span.record("status", field::debug(status));

    unsafe {
        let r = &mut *io_status_block;
        r.Anonymous.Status = status;
        r.Information = information;
    }

    status
}

unsafe fn read_write_file_handler<F>(
    handle: HANDLE,
    event_handle: HANDLE,
    apc_routine: PIO_APC_ROUTINE,
    buffer: *mut u8,
    length: c_ulong,
    byte_offset: *const i64,
    key: *const c_ulong,
    callback: F,
) -> Option<(NTSTATUS, usize)>
where
    F: FnOnce(&mut dyn Stream, &mut [u8]) -> io::Result<usize>,
{
    let stream = FileSilly::platform().get_stream(handle)?;

    // TODO: currently none of these get triggered but they could in theory
    if apc_routine.is_some() {
        warn!("tried to do an async read/write on a fake handle");
        return Some((STATUS_INVALID_PARAMETER, 0));
    }

    if !key.is_null() {
        warn!("key parameter passed for fake handle read/write");
        return Some((STATUS_INVALID_PARAMETER, 0));
    }

    if !event_handle.is_invalid() {
        warn!("passed event to fake handle read/write");
        return Some((STATUS_INVALID_PARAMETER, 0));
    }

    // Callers varyingly provide NULL or a pointer to `-1` as the offset.
    let byte_offset = if byte_offset.is_null() {
        -1
    } else {
        unsafe { *byte_offset }
    };

    let res = {
        let mut guard = stream.lock();
        let res = if byte_offset < 0 {
            Ok(0)
        } else {
            guard.seek(SeekFrom::Start(byte_offset as u64))
        };

        let slice = unsafe { slice::from_raw_parts_mut(buffer, length as usize) };
        res.and_then(|_| callback(&mut *guard, slice))
    };

    Some(match res {
        Ok(read) => (STATUS_SUCCESS, read),

        Err(error) => {
            error!(?error, "failed to seek or read/write fake handle");
            let status = util::io_error_to_status(&error);
            (status, 0)
        }
    })
}

unsafe fn dispatch_information<T, F>(
    stream: &HeapStream,
    file_information: *const c_void,
    length: u64,
    handler: F,
) -> (NTSTATUS, usize)
where
    F: FnOnce(&mut dyn Stream, &T) -> io::Result<()>,
{
    if (length as usize) < size_of::<T>() {
        (STATUS_INFO_LENGTH_MISMATCH, 0)
    } else {
        let info = unsafe { file_information.cast::<T>().as_ref_unchecked() };
        let mut guard = stream.lock();

        let status = match handler(&mut *guard, info) {
            Ok(_) => STATUS_SUCCESS,
            Err(err) => io_error_to_status(&err),
        };

        (status, size_of::<T>())
    }
}

unsafe fn dispatch_information_mut<T, F>(
    stream: &HeapStream,
    file_information: *mut c_void,
    length: u64,
    handler: F,
) -> (NTSTATUS, usize)
where
    F: FnOnce(&mut dyn Stream, &mut T) -> io::Result<()>,
{
    if (length as usize) < size_of::<T>() {
        (STATUS_INFO_LENGTH_MISMATCH, 0)
    } else {
        let info = unsafe { file_information.cast::<T>().as_mut_unchecked() };
        let mut guard = stream.lock();

        let status = match handler(&mut *guard, info) {
            Ok(_) => STATUS_SUCCESS,
            Err(err) => io_error_to_status(&err),
        };

        (status, size_of::<T>())
    }
}

type NtSetInformationFile = unsafe extern "system" fn(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS;
pub static NT_SET_INFORMATION_FILE_HOOK: LockDetour<NtSetInformationFile> = LazyLock::new(|| {
    SillyHook::new(
        c"ntdll.dll",
        c"NtSetInformationFile",
        set_information_file_detour,
    )
});

unsafe extern "system" fn set_information_file_detour(
    handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtSetInformationFile",
        handle = ?handle, class = ?file_information_class
    );
    let _entered = span.enter();

    let Some((status, information)) = (unsafe {
        set_information_file_handler(handle, file_information, length, file_information_class)
    }) else {
        // forward unmodified call
        unsafe {
            return NT_SET_INFORMATION_FILE_HOOK.unwrap().call(
                handle,
                io_status_block,
                file_information,
                length,
                file_information_class,
            );
        }
    };

    span.record("status", field::debug(status));

    unsafe {
        let r = &mut *io_status_block;
        r.Anonymous.Status = status;
        r.Information = information;
    }

    status
}

unsafe fn set_information_file_handler(
    handle: HANDLE,
    info: *const c_void,
    length: u64,
    class: FILE_INFORMATION_CLASS,
) -> Option<(NTSTATUS, usize)> {
    let stream = FileSilly::platform().get_stream(handle)?;

    unsafe {
        #[allow(non_upper_case_globals)]
        Some(match class {
            FilePositionInformation => dispatch_information::<FILE_POSITION_INFORMATION, _>(
                &stream,
                info,
                length,
                |stream, info| {
                    let offset = info.CurrentByteOffset as u64;
                    stream.seek(SeekFrom::Start(offset))?;
                    Ok(())
                },
            ),

            _ => {
                warn!("requested unknown information class");
                (STATUS_INVALID_INFO_CLASS, 0)
            }
        })
    }
}

type NtQueryInformationFileFn = unsafe extern "system" fn(
    handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS;
pub static NT_QUERY_INFORMATION_FILE_HOOK: LockDetour<NtQueryInformationFileFn> =
    LazyLock::new(|| {
        SillyHook::new(
            c"ntdll.dll",
            c"NtQueryInformationFile",
            query_information_file_detour,
        )
    });

unsafe extern "system" fn query_information_file_detour(
    handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtQueryInformationFile",
        handle = ?handle, class = ?file_information_class
    );
    let _entered = span.enter();

    let Some((status, information)) = (unsafe {
        query_information_file_handler(handle, file_information, length, file_information_class)
    }) else {
        // forward unmodified call
        unsafe {
            return NT_QUERY_INFORMATION_FILE_HOOK.unwrap().call(
                handle,
                io_status_block,
                file_information,
                length,
                file_information_class,
            );
        }
    };

    span.record("status", field::debug(status));

    unsafe {
        let r = &mut *io_status_block;
        r.Anonymous.Status = status;
        r.Information = information;
    }

    status
}

unsafe fn query_information_file_handler(
    handle: HANDLE,
    info: *mut c_void,
    length: u64,
    class: FILE_INFORMATION_CLASS,
) -> Option<(NTSTATUS, usize)> {
    let stream = FileSilly::platform().get_stream(handle)?;

    unsafe {
        #[allow(non_upper_case_globals)]
        Some(match class {
            FileBasicInformation => dispatch_information_mut::<FILE_BASIC_INFORMATION, _>(
                &stream,
                info,
                length,
                |_stream, info| {
                    // TODO
                    info.CreationTime = 0;
                    info.LastAccessTime = 0;
                    info.LastWriteTime = 0;
                    info.ChangeTime = 0;
                    info.FileAttributes = FILE_ATTRIBUTE_NORMAL.0;
                    Ok(())
                },
            ),

            FileStandardInformation => dispatch_information_mut::<FILE_STANDARD_INFORMATION, _>(
                &stream,
                info,
                length,
                |_stream, info| {
                    // TODO
                    info.AllocationSize = 10000;
                    info.EndOfFile = 10000;
                    info.NumberOfLinks = 0;
                    info.DeletePending = false;
                    info.Directory = false;
                    Ok(())
                },
            ),

            FilePositionInformation => dispatch_information_mut::<FILE_POSITION_INFORMATION, _>(
                &stream,
                info,
                length,
                |stream, info| {
                    let position = stream.stream_position()?;
                    info.CurrentByteOffset = position as i64;
                    Ok(())
                },
            ),

            _ => {
                warn!("requested unsupported information class");
                (STATUS_INVALID_INFO_CLASS, 0)
            }
        })
    }
}

type NtQueryVolumeInformationFileFn = unsafe extern "system" fn(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_system_information: *mut c_void,
    length: u64,
    file_system_information_class: FS_INFORMATION_CLASS,
) -> NTSTATUS;
pub static NT_QUERY_VOLUME_INFORMATION_FILE_HOOK: LockDetour<NtQueryVolumeInformationFileFn> =
    LazyLock::new(|| {
        SillyHook::new(
            c"ntdll.dll",
            c"NtQueryVolumeInformationFile",
            query_volume_information_file_detour,
        )
    });

unsafe extern "system" fn query_volume_information_file_detour(
    handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_system_information: *mut c_void,
    length: u64,
    file_system_information_class: FS_INFORMATION_CLASS,
) -> NTSTATUS {
    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "NtQueryVolumeInformationFile",
        handle = ?handle, class = ?file_system_information_class
    );
    let _entered = span.enter();

    let Some((status, information)) = (unsafe {
        query_volume_information_file_handler(
            handle,
            file_system_information,
            length,
            file_system_information_class,
        )
    }) else {
        // forward unmodified call
        unsafe {
            return NT_QUERY_VOLUME_INFORMATION_FILE_HOOK.unwrap().call(
                handle,
                io_status_block,
                file_system_information,
                length,
                file_system_information_class,
            );
        }
    };

    span.record("status", field::debug(status));

    unsafe {
        let r = &mut *io_status_block;
        r.Anonymous.Status = status;
        r.Information = information;
    }

    status
}

unsafe fn query_volume_information_file_handler(
    handle: HANDLE,
    info: *mut c_void,
    length: u64,
    class: FS_INFORMATION_CLASS,
) -> Option<(NTSTATUS, usize)> {
    let stream = FileSilly::platform().get_stream(handle)?;

    unsafe {
        #[allow(non_upper_case_globals)]
        Some(match class {
            FileFsVolumeInformation => dispatch_information_mut::<FILE_FS_VOLUME_INFORMATION, _>(
                &stream,
                info,
                length,
                |_stream, info| {
                    // TODO
                    info.VolumeCreationTime = 0;
                    info.VolumeSerialNumber = 0xc0ffee;
                    info.VolumeLabelLength = 0;
                    info.SupportsObjects = false;
                    info.VolumeLabel = [0; 1];
                    Ok(())
                },
            ),

            FileFsSizeInformation => dispatch_information_mut::<FILE_FS_SIZE_INFORMATION, _>(
                &stream,
                info,
                length,
                |_stream, info| {
                    // TODO
                    info.TotalAllocationUnits = 4096 * 1024;
                    info.AvailableAllocationUnits = 4096 * 512;
                    info.SectorsPerAllocationUnit = 1;
                    info.BytesPerSector = 4096;
                    Ok(())
                },
            ),

            FileFsFullSizeInformationEx => dispatch_information_mut::<
                FILE_FS_FULL_SIZE_INFORMATION_EX,
                _,
            >(&stream, info, length, |_stream, info| {
                // TODO
                info.ActualTotalAllocationUnits = 1024 * 1024;
                info.ActualAvailableAllocationUnits = 1024 * 512;
                info.ActualPoolUnavailableAllocationUnits = 0;
                info.CallerTotalAllocationUnits = 1024 * 1024;
                info.CallerAvailableAllocationUnits = 1024 * 512;
                info.CallerPoolUnavailableAllocationUnits = 0;
                info.UsedAllocationUnits = 1024 * 512;
                info.TotalReservedAllocationUnits = 0;
                info.VolumeStorageReserveAllocationUnits = 0;
                info.AvailableCommittedAllocationUnits = 0;
                info.PoolAvailableAllocationUnits = 1024 * 512;
                info.SectorsPerAllocationUnit = 1024;
                info.BytesPerSector = 4096;
                Ok(())
            }),

            FileFsDeviceInformation => dispatch_information_mut::<FILE_FS_DEVICE_INFORMATION, _>(
                &stream,
                info,
                length,
                |_stream, info| {
                    // TODO
                    info.DeviceType = FILE_DEVICE_DISK.0;
                    info.Characteristics = FILE_VIRTUAL_VOLUME;
                    Ok(())
                },
            ),

            _ => {
                warn!("requested unsupported volume information class");
                (STATUS_INVALID_INFO_CLASS, 0)
            }
        })
    }
}
