use crate::{Error, HANDLE_STORE, HandleStore};
use retour::{Function, GenericDetour, HookableWith};
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::io::{ErrorKind, SeekFrom};
use std::os::raw::c_ulong;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{io, mem, os::raw::c_void, slice};
use tracing::{debug, error, trace, trace_span, warn};
use windows::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows::Wdk::Storage::FileSystem::{
    FILE_BASIC_INFORMATION, FILE_INFORMATION_CLASS, FILE_POSITION_INFORMATION,
    FILE_STANDARD_INFORMATION, FS_INFORMATION_CLASS, FileBasicInformation, FileFsDeviceInformation,
    FileFsFullSizeInformationEx, FileFsSizeInformation, FileFsVolumeInformation,
    FilePositionInformation, FileStandardInformation,
};
use windows::Wdk::System::SystemServices::{
    FILE_FS_DEVICE_INFORMATION, FILE_FS_FULL_SIZE_INFORMATION_EX, FILE_FS_SIZE_INFORMATION,
    FILE_FS_VOLUME_INFORMATION, FILE_VIRTUAL_VOLUME,
};
use windows::Win32::Foundation::{
    INVALID_HANDLE_VALUE, NTSTATUS, STATUS_ACCESS_DENIED, STATUS_ADDRESS_ALREADY_ASSOCIATED,
    STATUS_ADDRESS_NOT_ASSOCIATED, STATUS_CONNECTION_ABORTED, STATUS_CONNECTION_REFUSED,
    STATUS_CONNECTION_RESET, STATUS_DIRECTORY_NOT_EMPTY, STATUS_DIRECTORY_NOT_SUPPORTED,
    STATUS_DISK_FULL, STATUS_END_OF_FILE, STATUS_FILE_TOO_LARGE, STATUS_HOST_UNREACHABLE,
    STATUS_INFO_LENGTH_MISMATCH, STATUS_INTERRUPTED, STATUS_INVALID_DEVICE_REQUEST,
    STATUS_INVALID_HANDLE, STATUS_INVALID_INFO_CLASS, STATUS_INVALID_PARAMETER,
    STATUS_MEDIA_WRITE_PROTECTED, STATUS_NETWORK_NAME_DELETED, STATUS_NETWORK_UNREACHABLE,
    STATUS_NO_MEMORY, STATUS_NOT_A_DIRECTORY, STATUS_NOT_FOUND, STATUS_NOT_SAME_DEVICE,
    STATUS_NOT_SUPPORTED, STATUS_OBJECT_NAME_EXISTS, STATUS_OBJECT_PATH_INVALID,
    STATUS_PIPE_BROKEN, STATUS_POSSIBLE_DEADLOCK, STATUS_QUOTA_EXCEEDED, STATUS_RESOURCE_IN_USE,
    STATUS_SUCCESS, STATUS_TIMEOUT, STATUS_TOO_MANY_LINKS, STATUS_UNSUCCESSFUL,
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, FILE_DEVICE_DISK};
use windows::Win32::System::IO::{IO_STATUS_BLOCK, PIO_APC_ROUTINE};
use windows::Win32::System::WindowsProgramming::FILE_OPENED;
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
    core::{PCSTR, PCWSTR, s, w},
};

/// Starting point for fake handles.
const FAKE_HANDLE_START: usize = 0x90D07_00000; // "GODOT" :+1:

/// Ending point for fake handles.
const FAKE_HANDLE_END: usize = 0x10000_00000 + FAKE_HANDLE_START;

static NEXT_FAKE_HANDLE: AtomicUsize = AtomicUsize::new(FAKE_HANDLE_START);

/// Wrapper around a [`HANDLE`] that implements Handle/Send/Sync.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FileHandle(HANDLE);

impl FileHandle {
    /// Checks if a handle is fake (in the fake handle range).
    fn is_fake(&self) -> bool {
        self.0.0 as usize >= FAKE_HANDLE_START && self.0.0 as usize <= FAKE_HANDLE_END
    }

    /// Allocates an unused fake handle.
    pub fn allocate_fake() -> FileHandle {
        let handle = NEXT_FAKE_HANDLE.fetch_add(1, Ordering::Relaxed);
        let handle = FileHandle(HANDLE(handle as *mut c_void));
        assert!(handle.is_fake());
        handle
    }
}

impl Hash for FileHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.0.hash(state);
    }
}

unsafe impl Send for FileHandle {}
unsafe impl Sync for FileHandle {}

fn io_error_to_status(error: &io::Error) -> NTSTATUS {
    match error.kind() {
        ErrorKind::NotFound => STATUS_NOT_FOUND,
        ErrorKind::PermissionDenied => STATUS_ACCESS_DENIED,
        ErrorKind::ConnectionRefused => STATUS_CONNECTION_REFUSED,
        ErrorKind::ConnectionReset => STATUS_CONNECTION_RESET,
        ErrorKind::HostUnreachable => STATUS_HOST_UNREACHABLE,
        ErrorKind::NetworkUnreachable => STATUS_NETWORK_UNREACHABLE,
        ErrorKind::ConnectionAborted => STATUS_CONNECTION_ABORTED,
        ErrorKind::NotConnected => STATUS_UNSUCCESSFUL,
        ErrorKind::AddrInUse => STATUS_ADDRESS_ALREADY_ASSOCIATED,
        ErrorKind::AddrNotAvailable => STATUS_ADDRESS_NOT_ASSOCIATED,
        ErrorKind::NetworkDown => STATUS_NETWORK_UNREACHABLE,
        ErrorKind::BrokenPipe => STATUS_PIPE_BROKEN,
        ErrorKind::AlreadyExists => STATUS_OBJECT_NAME_EXISTS,
        ErrorKind::WouldBlock => panic!("cannot turn WouldBlock into an NTSTATUS"),
        ErrorKind::NotADirectory => STATUS_NOT_A_DIRECTORY,
        ErrorKind::IsADirectory => STATUS_DIRECTORY_NOT_SUPPORTED,
        ErrorKind::DirectoryNotEmpty => STATUS_DIRECTORY_NOT_EMPTY,
        ErrorKind::ReadOnlyFilesystem => STATUS_MEDIA_WRITE_PROTECTED,
        ErrorKind::StaleNetworkFileHandle => STATUS_NETWORK_NAME_DELETED,
        ErrorKind::InvalidInput => STATUS_INVALID_PARAMETER,
        ErrorKind::InvalidData => STATUS_INVALID_PARAMETER,
        ErrorKind::TimedOut => STATUS_TIMEOUT,
        ErrorKind::WriteZero => STATUS_UNSUCCESSFUL,
        ErrorKind::StorageFull => STATUS_DISK_FULL,
        ErrorKind::NotSeekable => STATUS_INVALID_DEVICE_REQUEST,
        ErrorKind::QuotaExceeded => STATUS_QUOTA_EXCEEDED,
        ErrorKind::FileTooLarge => STATUS_FILE_TOO_LARGE,
        ErrorKind::ResourceBusy => STATUS_RESOURCE_IN_USE,
        ErrorKind::ExecutableFileBusy => STATUS_RESOURCE_IN_USE,
        ErrorKind::Deadlock => STATUS_POSSIBLE_DEADLOCK,
        ErrorKind::CrossesDevices => STATUS_NOT_SAME_DEVICE,
        ErrorKind::TooManyLinks => STATUS_TOO_MANY_LINKS,
        ErrorKind::InvalidFilename => STATUS_OBJECT_PATH_INVALID,
        ErrorKind::ArgumentListTooLong => STATUS_INVALID_PARAMETER,
        ErrorKind::Interrupted => STATUS_INTERRUPTED,
        ErrorKind::Unsupported => STATUS_NOT_SUPPORTED,
        ErrorKind::UnexpectedEof => STATUS_END_OF_FILE,
        ErrorKind::OutOfMemory => STATUS_NO_MEMORY,
        _ => STATUS_UNSUCCESSFUL,
    }
}

impl Error {
    // From makes this API visible
    fn from_windows(error: windows::core::Error) -> Self {
        Self::System {
            code: error.code().0 as u32,
            message: error.message(),
        }
    }
}

fn get_export(module: PCWSTR, export: PCSTR) -> crate::Result<*const c_void> {
    unsafe {
        let module = LoadLibraryW(module).map_err(Error::from_windows)?;

        match GetProcAddress(module, export) {
            Some(ptr) => Ok(ptr as *const std::ffi::c_void),
            None => {
                let win_err = windows::core::Error::from_thread();
                Err(Error::from_windows(win_err))
            }
        }
    }
}

pub struct SillyHook<T: HookableWith<D>, D: Function> {
    hook: crate::Result<GenericDetour<T>>,
    #[allow(dead_code)]
    detour: D,
}

impl<T: HookableWith<D>, D: Function> SillyHook<T, D> {
    fn hook(module: PCWSTR, export: PCSTR, detour: D) -> crate::Result<GenericDetour<T>> {
        unsafe {
            let export = get_export(module, export)?;
            let export: T = mem::transmute_copy(&export);
            GenericDetour::new(export, detour).map_err(|_| Error::Hook)
        }
    }

    pub fn new(module: PCWSTR, export: PCSTR, detour: D) -> Self {
        Self {
            hook: SillyHook::hook(module, export, detour),
            detour,
        }
    }

    pub fn get(&self) -> crate::Result<&GenericDetour<T>> {
        match &self.hook {
            Ok(hook) => Ok(hook),
            Err(_) => Err(Error::Hook),
        }
    }

    pub unsafe fn enable(&self) -> crate::Result<()> {
        unsafe { self.get()?.enable().map_err(|_| Error::Hook) }
    }

    pub fn unwrap(&self) -> &GenericDetour<T> {
        self.hook.as_ref().expect("couldn't get hook")
    }
}

type LockDetour<T> = LazyLock<SillyHook<T, T>>;

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

static NT_CREATE_FILE_HOOK: LockDetour<NtCreateFileFn> =
    LazyLock::new(|| SillyHook::new(w!("ntdll.dll"), s!("NtCreateFile"), create_file_detour));

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
    let filename = unsafe {
        let object_attributes = &*object_attributes;
        let object_name = &*object_attributes.ObjectName;
        slice::from_raw_parts(object_name.Buffer.0, object_name.Length as usize / 2)
    };

    let path = OsString::from_wide(filename);
    let path = PathBuf::from(&path);

    // The paths that this API receives are varyingly "normal" paths, UNC paths and NT object
    // manager paths. This attempts to normalize them to make comparisons more reliable.
    let path = if let Ok(path) = path.strip_prefix(r"\??\") {
        // FIXME(katie): This behaviour is definitely wrong.
        Path::new(r"\\?").join(path)
    } else {
        path
    };

    let path = dunce::simplified(&path);
    let _entered = trace_span!("NtCreateFile", ?path).entered();

    let io_status_block = unsafe { &mut *io_status_block };

    let handle_store = HANDLE_STORE.lock();
    let handle = match handle_store.try_borrow_mut() {
        Err(_) => None,
        Ok(mut handle_store) => Some(handle_store.create(path)),
    };
    drop(handle_store);

    let handle = match handle {
        // attempted recursion
        None => None,

        // factory errored
        Some(Err(err)) => {
            warn!(?err);

            let status = io_error_to_status(&err);
            io_status_block.Anonymous.Status = status;
            return status;
        }

        // factory ran successfully
        Some(Ok(handle)) => handle,
    };

    match handle {
        None => unsafe {
            NT_CREATE_FILE_HOOK.unwrap().call(
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
            )
        },

        Some(handle) => {
            trace!(?handle, "created fake handle");

            if !out_handle.is_null() {
                unsafe {
                    *out_handle = handle.0;
                }
            }

            // todo: return if file was opened vs created?
            io_status_block.Anonymous.Status = STATUS_SUCCESS;
            io_status_block.Information = FILE_OPENED as usize;
            STATUS_SUCCESS
        }
    }
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

static NT_READ_FILE_HOOK: LockDetour<NtReadWriteFileFn> =
    LazyLock::new(|| SillyHook::new(w!("ntdll.dll"), s!("NtReadFile"), read_file_detour));

unsafe extern "system" fn read_file_detour(
    raw_handle: HANDLE,
    event_handle: HANDLE,
    apc_routine: PIO_APC_ROUTINE,
    apc_context: *const c_void,
    io_status_block: *mut IO_STATUS_BLOCK,
    buffer: *mut u8,
    length: c_ulong,
    byte_offset: *const i64,
    key: *const c_ulong,
) -> NTSTATUS {
    let handle = FileHandle(raw_handle);
    if handle.is_fake() {
        assert!(
            apc_routine.is_none(),
            "tried to do an async read on a fake handle"
        );
        assert!(
            key.is_null(),
            "unknown key parameter passed for fake handle read"
        );
        assert!(
            event_handle == INVALID_HANDLE_VALUE || event_handle.0.is_null(),
            "passed event to fake handle read"
        );

        let _entered = trace_span!("NtReadFile", ?handle, %length).entered();

        let io_status_block = unsafe { &mut *io_status_block };
        io_status_block.Information = 0;

        let offset = if byte_offset.is_null() {
            -1
        } else {
            unsafe { *byte_offset }
        };

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            trace!("reading from fake handle");

            let slice = unsafe { slice::from_raw_parts_mut(buffer, length as usize) };

            let res = if offset < 0 {
                Ok(0)
            } else {
                stream.seek(SeekFrom::Start(offset as u64))
            };

            let res = res.and_then(|_| stream.read(slice));

            match res {
                Ok(read) => {
                    io_status_block.Anonymous.Status = STATUS_SUCCESS;
                    io_status_block.Information = read;
                    STATUS_SUCCESS
                }

                Err(error) => {
                    error!(?error, "failed to seek/read fake handle");

                    let status = io_error_to_status(&error);
                    io_status_block.Anonymous.Status = status;
                    status
                }
            }
        } else {
            debug!(?handle, "tried to read unknown fake handle");
            io_status_block.Anonymous.Status = STATUS_INVALID_HANDLE;
            STATUS_INVALID_HANDLE
        }
    } else {
        unsafe {
            NT_READ_FILE_HOOK.unwrap().call(
                raw_handle,
                event_handle,
                apc_routine,
                apc_context,
                io_status_block,
                buffer,
                length,
                byte_offset,
                key,
            )
        }
    }
}

static NT_WRITE_FILE_HOOK: LockDetour<NtReadWriteFileFn> =
    LazyLock::new(|| SillyHook::new(w!("ntdll.dll"), s!("NtWriteFile"), write_file_detour));

unsafe extern "system" fn write_file_detour(
    raw_handle: HANDLE,
    event_handle: HANDLE,
    apc_routine: PIO_APC_ROUTINE,
    apc_context: *const c_void,
    io_status_block: *mut IO_STATUS_BLOCK,
    buffer: *mut u8,
    length: c_ulong,
    byte_offset: *const i64,
    key: *const c_ulong,
) -> NTSTATUS {
    let handle = FileHandle(raw_handle);

    if handle.is_fake() {
        assert!(
            apc_routine.is_none(),
            "tried to do an async write on a fake handle"
        );
        assert!(
            key.is_null(),
            "unknown key parameter passed for fake handle write"
        );
        assert!(
            event_handle == INVALID_HANDLE_VALUE || event_handle.0.is_null(),
            "passed event to fake handle write"
        );

        let _entered = trace_span!("WriteFile", ?handle, %length).entered();

        let io_status_block = unsafe { &mut *io_status_block };
        io_status_block.Information = 0;

        let offset = if byte_offset.is_null() {
            -1
        } else {
            unsafe { *byte_offset }
        };

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            let buffer = unsafe { slice::from_raw_parts_mut(buffer, length as usize) };

            let res = if offset < 0 {
                Ok(0)
            } else {
                stream.seek(SeekFrom::Start(offset as u64))
            };

            let res = res.and_then(|_| stream.write(buffer));

            match res {
                Ok(written) => {
                    io_status_block.Anonymous.Status = STATUS_SUCCESS;
                    io_status_block.Information = written;
                    STATUS_SUCCESS
                }

                Err(error) => {
                    error!(?error, "error seeking/writing fake file");

                    let status = io_error_to_status(&error);
                    io_status_block.Anonymous.Status = status;
                    io_status_block.Information = 0;
                    status
                }
            }
        } else {
            debug!("unknown file in custom handle range");
            io_status_block.Anonymous.Status = STATUS_INVALID_HANDLE;
            io_status_block.Information = 0;
            STATUS_INVALID_HANDLE
        }
    } else {
        unsafe {
            NT_WRITE_FILE_HOOK.unwrap().call(
                raw_handle,
                event_handle,
                apc_routine,
                apc_context,
                io_status_block,
                buffer,
                length,
                byte_offset,
                key,
            )
        }
    }
}

type NtCloseFn = unsafe extern "system" fn(object: HANDLE) -> NTSTATUS;
static NT_CLOSE_HOOK: LockDetour<NtCloseFn> =
    LazyLock::new(|| SillyHook::new(w!("ntdll.dll"), s!("NtClose"), close_handle_detour));

unsafe extern "system" fn close_handle_detour(object: HANDLE) -> NTSTATUS {
    let handle = FileHandle(object);

    if handle.is_fake() {
        let _entered = trace_span!("NtClose", ?object).entered();

        let handle_store = HANDLE_STORE.lock();
        if handle_store.borrow_mut().close(handle).is_some() {
            trace!("closing fake handle");
            STATUS_SUCCESS
        } else {
            debug!("tried to closed non-existent fake handle");
            STATUS_INVALID_HANDLE
        }
    } else {
        unsafe { NT_CLOSE_HOOK.unwrap().call(object) }
    }
}

type NtSetInformationFile = unsafe extern "system" fn(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *const c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS;
static NT_SET_INFORMATION_FILE_HOOK: LockDetour<NtSetInformationFile> = LazyLock::new(|| {
    SillyHook::new(
        w!("ntdll.dll"),
        s!("NtSetInformationFile"),
        set_information_file_detour,
    )
});

unsafe extern "system" fn set_information_file_detour(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *const c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS {
    let handle = FileHandle(raw_handle);
    if handle.is_fake() {
        let _entered =
            trace_span!("NtSetInformationFile", ?raw_handle, class = ?file_information_class)
                .entered();
        let io_status_block = unsafe { &mut *io_status_block };

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            #[allow(non_upper_case_globals)]
            let (status, information) = match file_information_class {
                FilePositionInformation => {
                    if (length as usize) < size_of::<FILE_POSITION_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info =
                            unsafe { &*file_information.cast::<FILE_POSITION_INFORMATION>() };
                        let status =
                            match stream.seek(SeekFrom::Start(info.CurrentByteOffset as u64)) {
                                Ok(_) => STATUS_SUCCESS,
                                Err(ref err) => io_error_to_status(err),
                            };

                        (status, size_of::<FILE_POSITION_INFORMATION>())
                    }
                }

                _ => {
                    warn!("requested unknown information class");
                    (STATUS_INVALID_INFO_CLASS, 0)
                }
            };

            io_status_block.Anonymous.Status = status;
            io_status_block.Information = information;
            status
        } else {
            debug!("unknown file in custom handle range");
            io_status_block.Anonymous.Status = STATUS_INVALID_HANDLE;
            io_status_block.Information = 0;
            STATUS_INVALID_HANDLE
        }
    } else {
        unsafe {
            NT_SET_INFORMATION_FILE_HOOK.unwrap().call(
                raw_handle,
                io_status_block,
                file_information,
                length,
                file_information_class,
            )
        }
    }
}

type NtQueryInformationFileFn = unsafe extern "system" fn(
    handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS;
static NT_QUERY_INFORMATION_FILE_HOOK: LockDetour<NtQueryInformationFileFn> = LazyLock::new(|| {
    SillyHook::new(
        w!("ntdll.dll"),
        s!("NtQueryInformationFile"),
        query_information_file_detour,
    )
});

unsafe extern "system" fn query_information_file_detour(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_information: *mut c_void,
    length: u64,
    file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS {
    let handle = FileHandle(raw_handle);
    if handle.is_fake() {
        let _entered =
            trace_span!("NtQueryInformationFile", ?raw_handle, class = ?file_information_class)
                .entered();
        let io_status_block = unsafe { &mut *io_status_block };

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            #[allow(non_upper_case_globals)]
            let (status, information) = match file_information_class {
                FileBasicInformation => {
                    if (length as usize) < size_of::<FILE_BASIC_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info =
                            unsafe { &mut *file_information.cast::<FILE_BASIC_INFORMATION>() };
                        info.CreationTime = 0;
                        info.LastAccessTime = 0;
                        info.LastWriteTime = 0;
                        info.ChangeTime = 0;
                        info.FileAttributes = FILE_ATTRIBUTE_NORMAL.0;
                        (STATUS_SUCCESS, size_of::<FILE_BASIC_INFORMATION>())
                    }
                }

                FileStandardInformation => {
                    if (length as usize) < size_of::<FILE_STANDARD_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info =
                            unsafe { &mut *file_information.cast::<FILE_STANDARD_INFORMATION>() };
                        info.AllocationSize = 10000;
                        info.EndOfFile = 10000;
                        info.NumberOfLinks = 0;
                        info.DeletePending = false;
                        info.Directory = false;
                        (STATUS_SUCCESS, size_of::<FILE_STANDARD_INFORMATION>())
                    }
                }

                FilePositionInformation => {
                    if (length as usize) < size_of::<FILE_POSITION_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info =
                            unsafe { &mut *file_information.cast::<FILE_POSITION_INFORMATION>() };
                        let status = match stream.stream_position() {
                            Ok(offset) => {
                                info.CurrentByteOffset = offset as i64;
                                STATUS_SUCCESS
                            }
                            Err(ref err) => {
                                warn!(?err, "seeking fake file");

                                info.CurrentByteOffset = 0;
                                io_error_to_status(err)
                            }
                        };

                        (status, size_of::<FILE_POSITION_INFORMATION>())
                    }
                }

                class => {
                    debug!(?class, "requested unknown information class");
                    (STATUS_INVALID_INFO_CLASS, 0)
                }
            };

            io_status_block.Anonymous.Status = status;
            io_status_block.Information = information;
            status
        } else {
            debug!("unknown file in custom handle range");
            io_status_block.Anonymous.Status = STATUS_INVALID_HANDLE;
            io_status_block.Information = 0;
            STATUS_INVALID_HANDLE
        }
    } else {
        unsafe {
            NT_QUERY_INFORMATION_FILE_HOOK.unwrap().call(
                raw_handle,
                io_status_block,
                file_information,
                length,
                file_information_class,
            )
        }
    }
}

type NtQueryVolumeInformationFileFn = unsafe extern "system" fn(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_system_information: *mut c_void,
    length: u64,
    file_system_information_class: FS_INFORMATION_CLASS,
) -> NTSTATUS;
static NT_QUERY_VOLUME_INFORMATION_FILE_HOOK: LockDetour<NtQueryVolumeInformationFileFn> =
    LazyLock::new(|| {
        SillyHook::new(
            w!("ntdll.dll"),
            s!("NtQueryVolumeInformationFile"),
            query_volume_information_file_detour,
        )
    });

unsafe extern "system" fn query_volume_information_file_detour(
    raw_handle: HANDLE,
    io_status_block: *mut IO_STATUS_BLOCK,
    file_system_information: *mut c_void,
    length: u64,
    file_system_information_class: FS_INFORMATION_CLASS,
) -> NTSTATUS {
    let handle = FileHandle(raw_handle);
    if handle.is_fake() {
        let _entered = trace_span!("NtQueryVolumeInformationFile", ?raw_handle, class = ?file_system_information_class).entered();
        let io_status_block = unsafe { &mut *io_status_block };

        let handle_store = HANDLE_STORE.lock();
        if let Some(_stream) = handle_store.borrow_mut().get_stream(handle) {
            #[allow(non_upper_case_globals)]
            let (status, information) = match file_system_information_class {
                FileFsVolumeInformation => {
                    if (length as usize) < size_of::<FILE_FS_VOLUME_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info = unsafe {
                            &mut *file_system_information.cast::<FILE_FS_VOLUME_INFORMATION>()
                        };
                        info.VolumeCreationTime = 0;
                        info.VolumeSerialNumber = 0xc0ffee;
                        info.VolumeLabelLength = 0;
                        info.SupportsObjects = false;
                        info.VolumeLabel = [0; 1];
                        (STATUS_SUCCESS, size_of::<FILE_FS_SIZE_INFORMATION>())
                    }
                }

                FileFsSizeInformation => {
                    if (length as usize) < size_of::<FILE_FS_SIZE_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info = unsafe {
                            &mut *file_system_information.cast::<FILE_FS_SIZE_INFORMATION>()
                        };
                        info.TotalAllocationUnits = 4096 * 1024;
                        info.AvailableAllocationUnits = 4096 * 512;
                        info.SectorsPerAllocationUnit = 1;
                        info.BytesPerSector = 4096;
                        (STATUS_SUCCESS, size_of::<FILE_FS_SIZE_INFORMATION>())
                    }
                }

                FileFsFullSizeInformationEx => {
                    if (length as usize) < size_of::<FILE_FS_FULL_SIZE_INFORMATION_EX>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info = unsafe {
                            &mut *file_system_information.cast::<FILE_FS_FULL_SIZE_INFORMATION_EX>()
                        };
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
                        (
                            STATUS_SUCCESS,
                            size_of::<FILE_FS_FULL_SIZE_INFORMATION_EX>(),
                        )
                    }
                }

                FileFsDeviceInformation => {
                    if (length as usize) < size_of::<FILE_FS_DEVICE_INFORMATION>() {
                        (STATUS_INFO_LENGTH_MISMATCH, 0)
                    } else {
                        let info = unsafe {
                            &mut *file_system_information.cast::<FILE_FS_DEVICE_INFORMATION>()
                        };
                        info.DeviceType = FILE_DEVICE_DISK.0;
                        info.Characteristics = FILE_VIRTUAL_VOLUME;
                        (STATUS_SUCCESS, size_of::<FILE_FS_DEVICE_INFORMATION>())
                    }
                }

                class => {
                    debug!(?class, "requested unknown information class");
                    (STATUS_INVALID_INFO_CLASS, 0)
                }
            };

            io_status_block.Anonymous.Status = status;
            io_status_block.Information = information;
            status
        } else {
            debug!("unknown file in custom handle range");
            io_status_block.Anonymous.Status = STATUS_INVALID_HANDLE;
            io_status_block.Information = 0;
            STATUS_INVALID_HANDLE
        }
    } else {
        unsafe {
            NT_QUERY_VOLUME_INFORMATION_FILE_HOOK.unwrap().call(
                raw_handle,
                io_status_block,
                file_system_information,
                length,
                file_system_information_class,
            )
        }
    }
}

pub fn init() -> crate::Result<()> {
    unsafe {
        NT_CREATE_FILE_HOOK.enable()?;
        NT_READ_FILE_HOOK.enable()?;
        NT_WRITE_FILE_HOOK.enable()?;
        NT_CLOSE_HOOK.enable()?;
        NT_SET_INFORMATION_FILE_HOOK.enable()?;
        NT_QUERY_INFORMATION_FILE_HOOK.enable()?;
        NT_QUERY_VOLUME_INFORMATION_FILE_HOOK.enable()?;
    }

    Ok(())
}

impl HandleStore {
    pub(crate) fn allocate_file_handle(&self) -> FileHandle {
        FileHandle::allocate_fake()
    }
}
