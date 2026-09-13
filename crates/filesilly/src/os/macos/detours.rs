use crate::{
    Filesilly, StatResult, Stream,
    os::macos::{
        WrappedFd,
        util::{mark_errno, resolve_path, system_time_to_timespec},
    },
    recursion_guard::RecursionGuard,
};
use libc::{FILE, c_char, c_int, mode_t, off_t, size_t, ssize_t, stat};
use std::{
    ffi::{CStr, OsStr, OsString},
    io::SeekFrom,
    mem::MaybeUninit,
    os::{raw::c_void, unix::ffi::OsStrExt},
    path::PathBuf,
    slice,
};
use tracing::{error, field, trace, trace_span};

unsafe extern "C" {
    // In macOS/iOS, fopen uses open$NOCANCEL/__open_nocancel internally.
    #[link_name = "open$NOCANCEL"]
    unsafe fn libsystem_kernel_open_nocancel(
        filename: *const c_char,
        flags: c_int,
        mode: mode_t,
    ) -> c_int;
    #[link_name = "fclose"]
    unsafe fn libc_fclose(stream: *mut FILE) -> c_int;
    #[link_name = "fread"]
    unsafe fn libc_fread(
        ptr: *mut c_void,
        size: size_t,
        nobj: size_t,
        stream: *mut FILE,
    ) -> ssize_t;
    #[link_name = "fwrite"]
    unsafe fn libc_fwrite(
        ptr: *const c_void,
        size: size_t,
        nobj: size_t,
        stream: *mut FILE,
    ) -> ssize_t;
    #[link_name = "fseeko"]
    unsafe fn libc_fseeko(stream: *mut FILE, offset: off_t, whence: c_int) -> off_t;
    #[link_name = "ftello"]
    unsafe fn libc_ftello(stream: *mut FILE) -> off_t;
    #[link_name = "stat"]
    pub fn libc_stat(filename: *const c_char, buf: *mut stat) -> c_int;
    #[link_name = "access"]
    pub fn libc_access(filename: *const c_char, typ: c_int) -> c_int;

}

thread_local! {
    static RECURSION_GUARD: RecursionGuard = const { RecursionGuard::new() };
}

unsafe extern "system" fn open_detour(
    filename: *const c_char,
    flags: c_int,
    mode: mode_t,
) -> c_int {
    let span = trace_span!(target: "filesilly::hooks", parent: None, "open");
    let _entered = span.enter();

    let Some(result) = RECURSION_GUARD
        .try_with(|r| match r.acquire() {
            Some(_guard) => unsafe { open_handler(filename) },
            None => {
                trace!("re-entrant call to open");
                None
            }
        })
        .ok()
        .flatten()
    else {
        // forward unmodified call
        unsafe { return libsystem_kernel_open_nocancel(filename, flags, mode) }
    };

    match result {
        Ok(fd) => {
            span.record("fd", field::debug(fd));
            fd
        }

        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn open_handler(filename: *const c_char) -> Option<Result<c_int, c_int>> {
    let path = unsafe {
        let str = CStr::from_ptr(filename);
        OsStr::from_bytes(str.to_bytes())
    };
    let path: PathBuf = OsString::from(path).into();
    let path = resolve_path(&path)?;
    let result = Filesilly::factory_opt()?.open(&path);

    match result {
        Ok(None) => None,
        Ok(Some(stream)) => Some(
            match Filesilly::platform_opt()?.allocate_fd_for_stream(stream) {
                Ok(fd) => Ok(fd.0),
                Err(err) => {
                    error!(?err, "failed to generate fake file descriptor");
                    return Some(Err(libc::EINVAL));
                }
            },
        ),
        Err(err) => {
            error!(?err, "stream factory returned an error");
            let status = err.raw_os_error().unwrap_or(libc::EINVAL);
            Some(Err(status))
        }
    }
}

unsafe extern "system" fn close_detour(stream: *mut FILE) -> c_int {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "close",
        fd = ?fd
    );
    let _entered = span.enter();

    if let Some(platform) = Filesilly::platform_opt()
        && let Some((_, stream)) = platform.file_descriptors.remove(&WrappedFd(fd))
    {
        trace!("closed handle");
        drop(stream);
    }

    unsafe { libc_fclose(stream) }
}

unsafe extern "system" fn read_detour(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> ssize_t {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "read",
        fd = ?fd, size = %size
    );
    let _entered = span.enter();

    let Some(result) = (unsafe {
        read_write_file_handler(ptr, size, nobj, fd, |stream, buffer| stream.read(buffer))
    }) else {
        // forward unmodified call
        unsafe { return libc_fread(ptr, size, nobj, stream) }
    };

    match result {
        Ok(read) => read as ssize_t,
        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe extern "system" fn write_detour(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> ssize_t {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "write",
        fd = ?fd, size = %size
    );
    let _entered = span.enter();

    let Some(result) = (unsafe {
        read_write_file_handler(ptr, size, nobj, fd, |stream, buffer| stream.write(buffer))
    }) else {
        // forward unmodified call
        unsafe { return libc_fwrite(ptr, size, nobj, stream) }
    };

    match result {
        Ok(wrote) => wrote as ssize_t,
        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn read_write_file_handler<F>(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    fd: c_int,
    callback: F,
) -> Option<Result<usize, c_int>>
where
    F: FnOnce(&mut dyn Stream, &mut [u8]) -> std::io::Result<usize>,
{
    let stream = Filesilly::platform_opt()?.get_stream(fd)?;

    if ptr.is_null() {
        return Some(Ok(0));
    }

    let res = {
        let mut guard = stream.lock();
        // FIXME(jules): this is subtly wrong, it should attempt to read *up to* size elements of nobj size
        let length = size * nobj;
        let slice = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, length) };
        callback(&mut *guard, slice)
    };

    Some(res.map_err(|error| {
        error!(?error, "failed to read/write fake fd");
        error.raw_os_error().unwrap_or(libc::EINVAL)
    }))
}

unsafe extern "system" fn seek_detour(stream: *mut FILE, offset: off_t, whence: c_int) -> off_t {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "seek",
        fd = ?fd, offset = %offset
    );
    let _entered = span.enter();

    let Some(result) = (unsafe { seek_handler(fd, offset, whence) }) else {
        // forward unmodified call
        unsafe { return libc_fseeko(stream, offset, whence) }
    };

    match result {
        Ok(_) => 0,
        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn seek_handler(fd: c_int, offset: off_t, whence: c_int) -> Option<Result<u64, c_int>> {
    let stream = Filesilly::platform_opt()?.get_stream(fd)?;

    let from = match whence {
        libc::SEEK_SET => SeekFrom::Start(offset as u64),
        libc::SEEK_CUR => SeekFrom::Current(offset),
        libc::SEEK_END => SeekFrom::End(offset),
        _ => {
            return Some(Err(libc::EINVAL));
        }
    };

    let res = {
        let mut guard = stream.lock();
        guard.seek(from)
    };

    Some(res.map_err(|error| {
        error!(?error, "failed to seek fake fd");
        error.raw_os_error().unwrap_or(libc::EINVAL)
    }))
}

unsafe extern "system" fn tell_detour(stream: *mut FILE) -> off_t {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "tell",
        fd = ?fd
    );
    let _entered = span.enter();

    let Some(result) = (unsafe { tell_handler(fd) }) else {
        // forward unmodified call
        unsafe { return libc_ftello(stream) }
    };

    match result {
        Ok(pos) => pos as off_t,
        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn tell_handler(fd: c_int) -> Option<Result<u64, c_int>> {
    let stream = Filesilly::platform_opt()?.get_stream(fd)?;

    let res = {
        let mut guard = stream.lock();
        guard.stream_position()
    };

    Some(res.map_err(|error| {
        error!(?error, "failed to tell fake fd");
        error.raw_os_error().unwrap_or(libc::EINVAL)
    }))
}

unsafe extern "system" fn stat_detour(filename: *const c_char, buf: *mut stat) -> c_int {
    let span = trace_span!(target: "filesilly::hooks", parent: None, "stat");
    let _entered = span.enter();

    let Some(result) = RECURSION_GUARD
        .try_with(|r| match r.acquire() {
            Some(_guard) => unsafe { stat_handler(filename) },
            None => {
                trace!("re-entrant call to stat");
                None
            }
        })
        .ok()
        .flatten()
    else {
        // forward unmodified call
        unsafe { return libc_stat(filename, buf) }
    };

    match result {
        Ok(stat) => {
            unsafe {
                *buf = stat;
            }

            0
        }

        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn stat_handler(filename: *const c_char) -> Option<Result<stat, c_int>> {
    let path = unsafe {
        let str = CStr::from_ptr(filename);
        OsStr::from_bytes(str.to_bytes())
    };
    let path: PathBuf = OsString::from(path).into();
    let path = resolve_path(&path)?;

    let result = Filesilly::factory_opt()?.stat(&path);
    match result {
        Ok(StatResult::Passthrough) => None,
        Ok(StatResult::DoesntExist) => Some(Err(libc::ENOENT)),
        Ok(StatResult::Exists(info)) => {
            let stat = MaybeUninit::<stat>::zeroed();
            let mut stat = unsafe { stat.assume_init() };

            stat.st_mode = libc::S_IFREG;
            stat.st_size = info.size as i64;

            let access_time = system_time_to_timespec(info.access_time);
            stat.st_atime = access_time.0;
            stat.st_atime_nsec = access_time.1;

            let modification_time = system_time_to_timespec(info.modification_time);
            stat.st_mtime = modification_time.0;
            stat.st_mtime_nsec = modification_time.1;

            let change_time = system_time_to_timespec(info.change_time);
            stat.st_ctime = change_time.0;
            stat.st_ctime_nsec = change_time.1;

            Some(Ok(stat))
        }
        Err(err) => {
            error!(?err, "stream factory returned an error");
            let status = err.raw_os_error().unwrap_or(libc::EINVAL);
            Some(Err(status))
        }
    }
}

unsafe extern "system" fn access_detour(filename: *const c_char, typ: c_int) -> c_int {
    let span = trace_span!(target: "filesilly::hooks", parent: None, "access");
    let _entered = span.enter();

    let Some(result) = RECURSION_GUARD
        .try_with(|r| match r.acquire() {
            Some(_guard) => unsafe { access_handler(filename) },
            None => {
                trace!("re-entrant call to access");
                None
            }
        })
        .ok()
        .flatten()
    else {
        // forward unmodified call
        unsafe { return libc_access(filename, typ) }
    };

    match result {
        Ok(()) => 0,

        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn access_handler(filename: *const c_char) -> Option<Result<(), c_int>> {
    let path = unsafe {
        let str = CStr::from_ptr(filename);
        OsStr::from_bytes(str.to_bytes())
    };
    let path: PathBuf = OsString::from(path).into();
    let path = resolve_path(&path)?;

    let result = Filesilly::factory_opt()?.stat(&path);
    match result {
        Ok(StatResult::Passthrough) => None,
        Ok(StatResult::DoesntExist) => Some(Err(libc::ENOENT)),
        Ok(StatResult::Exists(_)) => Some(Ok(())),
        Err(err) => {
            error!(?err, "stream factory returned an error");
            let status = err.raw_os_error().unwrap_or(libc::EINVAL);
            Some(Err(status))
        }
    }
}

#[repr(C)]
struct InterposeEntry {
    replacement: *const c_void,
    original: *const c_void,
}

unsafe impl Sync for InterposeEntry {}

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_OPEN_NOCANCEL: InterposeEntry = {
    InterposeEntry {
        replacement: open_detour as *const c_void,
        original: libsystem_kernel_open_nocancel as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_CLOSE: InterposeEntry = {
    InterposeEntry {
        replacement: close_detour as *const c_void,
        original: libc_fclose as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_READ: InterposeEntry = {
    InterposeEntry {
        replacement: read_detour as *const c_void,
        original: libc_fread as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_WRITE: InterposeEntry = {
    InterposeEntry {
        replacement: write_detour as *const c_void,
        original: libc_fwrite as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_SEEK: InterposeEntry = {
    InterposeEntry {
        replacement: seek_detour as *const c_void,
        original: libc_fseeko as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_TELL: InterposeEntry = {
    InterposeEntry {
        replacement: tell_detour as *const c_void,
        original: libc_ftello as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_STAT: InterposeEntry = {
    InterposeEntry {
        replacement: stat_detour as *const c_void,
        original: libc_stat as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_ACCESS: InterposeEntry = {
    InterposeEntry {
        replacement: access_detour as *const c_void,
        original: libc_access as *const c_void,
    }
};
