use crate::{
    Filesilly, Stream,
    hook::{LockDetour, SillyHook},
    os::linux::{WrappedFd, util::mark_errno},
    recursion_guard::RecursionGuard,
};
use libc::{FILE, c_char, c_int, mode_t, off_t, off64_t, size_t, ssize_t};
use std::{
    ffi::{CStr, OsStr, OsString},
    io::SeekFrom,
    os::{raw::c_void, unix::ffi::OsStrExt},
    path::PathBuf,
    slice,
    sync::LazyLock,
};
use tracing::{error, field, trace, trace_span};

type OpenFn =
    unsafe extern "system" fn(filename: *const c_char, flags: c_int, mode: mode_t) -> c_int;
pub static OPEN_HOOK: LockDetour<OpenFn> =
    LazyLock::new(|| SillyHook::new(c"GLIBC_2.2.5", c"open64", open_detour));

thread_local! {
    static OPEN_RECURSION_GUARD: RecursionGuard = const { RecursionGuard::new() };
}

unsafe extern "system" fn open_detour(
    filename: *const c_char,
    flags: c_int,
    mode: mode_t,
) -> c_int {
    let span = trace_span!(target: "filesilly::hooks", parent: None, "open");
    let _entered = span.enter();

    let Some(result) = OPEN_RECURSION_GUARD
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
        unsafe { return OPEN_HOOK.unwrap().call(filename, flags, mode) }
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
    let path = if !path.is_absolute()
        && let Ok(current_dir) = std::env::current_dir()
    {
        current_dir.join(path)
    } else {
        path
    };

    let base_path = &Filesilly::platform().base_path;
    let relative_path = path.strip_prefix(base_path).ok()?;
    let absolute_path = Filesilly::platform().base_path.join(relative_path);
    let result = Filesilly::factory().open(&absolute_path);

    let stream = match result {
        Ok(None) => return None,
        Ok(Some(stream)) => stream,
        Err(err) => {
            error!(?err, "stream factory returned an error");
            let status = err.raw_os_error().unwrap_or(libc::EINVAL);
            return Some(Err(status));
        }
    };

    let fd = Filesilly::platform().allocate_fd_for_stream(stream);
    Some(Ok(fd.0))
}

type CloseFn = unsafe extern "system" fn(stream: *mut FILE) -> c_int;
pub static CLOSE_HOOK: LockDetour<CloseFn> =
    LazyLock::new(|| SillyHook::new(c"GLIBC_2.2.5", c"fclose", close_detour));

unsafe extern "system" fn close_detour(stream: *mut FILE) -> c_int {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "close",
        fd = ?fd
    );
    let _entered = span.enter();

    let platform = Filesilly::platform();

    if let Some((_, stream)) = platform.file_descriptors.remove(&WrappedFd(fd)) {
        trace!("closed handle");
        drop(stream);
        return 0;
    }

    unsafe { CLOSE_HOOK.unwrap().call(stream) }
}

type ReadFn = unsafe extern "system" fn(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> ssize_t;
pub static READ_HOOK: LockDetour<ReadFn> =
    LazyLock::new(|| SillyHook::new(c"GLIBC_2.2.5", c"fread", read_detour));

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
        unsafe { return READ_HOOK.unwrap().call(ptr, size, nobj, stream) }
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

type WriteFn = unsafe extern "system" fn(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> ssize_t;
pub static WRITE_HOOK: LockDetour<WriteFn> =
    LazyLock::new(|| SillyHook::new(c"GLIBC_2.2.5", c"fwrite", write_detour));

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
        unsafe { return WRITE_HOOK.unwrap().call(ptr, size, nobj, stream) }
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
    let stream = Filesilly::platform().get_stream(fd)?;

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

type SeekFn =
    unsafe extern "system" fn(stream: *mut FILE, offset: off64_t, whence: c_int) -> off64_t;
pub static SEEK_HOOK: LockDetour<SeekFn> =
    LazyLock::new(|| SillyHook::new(c"GLIBC_2.2.5", c"fseeko64", seek_detour));

unsafe extern "system" fn seek_detour(stream: *mut FILE, offset: off_t, whence: c_int) -> off64_t {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "seek",
        fd = ?fd, offset = %offset
    );
    let _entered = span.enter();

    let Some(result) = (unsafe { seek_handler(fd, offset, whence) }) else {
        // forward unmodified call
        unsafe { return SEEK_HOOK.unwrap().call(stream, offset, whence) }
    };

    match result {
        Ok(offset) => offset as off64_t,
        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn seek_handler(fd: c_int, offset: off_t, whence: c_int) -> Option<Result<u64, c_int>> {
    let stream = Filesilly::platform().get_stream(fd)?;

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

type TellFn = unsafe extern "system" fn(stream: *mut FILE) -> off64_t;
pub static TELL_HOOK: LockDetour<TellFn> =
    LazyLock::new(|| SillyHook::new(c"GLIBC_2.2.5", c"ftello64", tell_detour));

unsafe extern "system" fn tell_detour(stream: *mut FILE) -> off64_t {
    let fd = unsafe { libc::fileno(stream) };

    let span = trace_span!(
        target: "filesilly::hooks", parent: None, "tell",
        fd = ?fd
    );
    let _entered = span.enter();

    let Some(result) = (unsafe { tell_handler(fd) }) else {
        // forward unmodified call
        unsafe { return TELL_HOOK.unwrap().call(stream) }
    };

    match result {
        Ok(pos) => pos as off64_t,
        Err(errno) => {
            span.record("errno", field::debug(errno));
            mark_errno(errno);
            -1
        }
    }
}

unsafe fn tell_handler(fd: c_int) -> Option<Result<u64, c_int>> {
    let stream = Filesilly::platform().get_stream(fd)?;

    let res = {
        let mut guard = stream.lock();
        guard.stream_position()
    };

    Some(res.map_err(|error| {
        error!(?error, "failed to tell fake fd");
        error.raw_os_error().unwrap_or(libc::EINVAL)
    }))
}
