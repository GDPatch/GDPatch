use crate::{HANDLE_STORE, HandleStore};
use libc::{FILE, c_char, c_int, mode_t, off_t, size_t};
use std::{
    ffi::{CStr, OsStr, OsString},
    io::SeekFrom,
    os::{raw::c_void, unix::ffi::OsStrExt},
    path::PathBuf,
    slice,
    sync::atomic::{AtomicI32, Ordering},
};
use tracing::{error, trace, trace_span, warn};

/// Starting point for fake handles.
const FAKE_HANDLE_START: i32 = 0x90D; // "GODOT" :+1:

static NEXT_FAKE_HANDLE: AtomicI32 = AtomicI32::new(FAKE_HANDLE_START);

/// Wrapper around a [`HANDLE`] that implements Handle/Send/Sync.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileHandle(i32);

impl FileHandle {
    pub fn is_fake(&self) -> bool {
        self.0 >= FAKE_HANDLE_START
    }

    /// Allocates an unused fake handle.
    pub fn allocate_fake() -> FileHandle {
        let handle = NEXT_FAKE_HANDLE.fetch_add(1, Ordering::Relaxed);
        FileHandle(handle)
    }
}

unsafe impl Send for FileHandle {}
unsafe impl Sync for FileHandle {}

fn mark_errno(value: c_int) {
    unsafe {
        *libc::__error() = value;
    }
}

unsafe extern "system" fn interposed_open_nocancel(
    filename: *const c_char,
    flags: c_int,
    mode: mode_t,
) -> c_int {
    let path = unsafe {
        let str = CStr::from_ptr(filename);
        OsStr::from_bytes(str.to_bytes())
    };
    let path: PathBuf = OsString::from(path).into();
    let path = path.canonicalize().unwrap_or(path);

    let _entered = trace_span!("open", ?path).entered();

    let handle_store = HANDLE_STORE.lock();
    let handle = match handle_store.try_borrow_mut() {
        Err(_) => None,
        Ok(mut handle_store) => Some(handle_store.create(&path)),
    };
    drop(handle_store);

    let handle = match handle {
        // attempted recursion
        None => None,

        // factory errored
        Some(Err(err)) => {
            warn!(?err);
            mark_errno(libc::EINVAL);
            return -1;
        }

        // factory ran successfully
        Some(Ok(handle)) => handle,
    };

    match handle {
        None => unsafe { libsystem_kernel_open_nocancel(filename, flags, mode) },

        Some(handle) => {
            trace!(?handle, "created fake handle");
            handle.0
        }
    }
}

unsafe extern "system" fn interposed_close(stream: *mut FILE) -> c_int {
    let fd = unsafe { libc::fileno(stream) };
    let handle = FileHandle(fd);

    if handle.is_fake() {
        let _entered = trace_span!("close", ?fd).entered();

        let handle_store = HANDLE_STORE.lock();
        if handle_store.borrow_mut().close(handle).is_some() {
            trace!("closing fake handle");
            0
        } else {
            warn!("tried to closed non-existent fake handle");
            mark_errno(libc::EBADF);
            -1
        }
    } else {
        unsafe { libc_fclose(stream) }
    }
}

unsafe extern "system" fn interposed_tell(stream: *mut FILE) -> off_t {
    let fd = unsafe { libc::fileno(stream) };
    let handle = FileHandle(fd);

    if handle.is_fake() {
        let _entered = trace_span!("tell", ?fd).entered();

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            let res = stream.stream_position();

            match res {
                Ok(pos) => {
                    trace!(?pos, "tell for fake handle");
                    pos as i64
                }
                Err(error) => {
                    error!(?error, "failed to tell fake handle");
                    mark_errno(libc::EINVAL);
                    -1
                }
            }
        } else {
            warn!("unknown file in custom handle range");
            mark_errno(libc::EINVAL);
            -1
        }
    } else {
        unsafe { libc_ftello(stream) }
    }
}

unsafe extern "system" fn interposed_seek(
    stream: *mut FILE,
    offset: off_t,
    whence: c_int,
) -> off_t {
    let fd = unsafe { libc::fileno(stream) };
    let handle = FileHandle(fd);

    if handle.is_fake() {
        let from = match whence {
            libc::SEEK_SET => SeekFrom::Start(offset as u64),
            libc::SEEK_CUR => SeekFrom::Current(offset),
            libc::SEEK_END => SeekFrom::End(offset),
            _ => {
                mark_errno(libc::EINVAL);
                return -1;
            }
        };
        let _entered = trace_span!("seek", ?fd, ?from).entered();

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            let res = stream.seek(from);
            match res {
                Ok(offset) => {
                    trace!(?offset, "seeked fake handle");
                    offset as off_t
                }
                Err(error) => {
                    error!(?error, "failed to seek fake handle");
                    mark_errno(libc::EINVAL);
                    -1
                }
            }
        } else {
            warn!("unknown file in custom handle range");
            mark_errno(libc::EINVAL);
            -1
        }
    } else {
        unsafe { libc_fseeko(stream, offset, whence) }
    }
}

unsafe extern "system" fn interposed_read(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> size_t {
    let fd = unsafe { libc::fileno(stream) };
    let handle = FileHandle(fd);

    if handle.is_fake() {
        let _entered = trace_span!("read", ?fd, ?size, ?nobj).entered();

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            // FIXME(jules): this is subtly wrong, it should attempt to read *up to* size elements of nobj size
            let slice = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, size * nobj) };
            let res = stream.read(&mut slice[..]);

            match res {
                Ok(read) => {
                    trace!(?read, "read from fake handle");
                    read
                }
                Err(error) => {
                    error!(?error, "failed to read fake handle");
                    0
                }
            }
        } else {
            warn!("tried to read unknown fake handle");
            0
        }
    } else {
        unsafe { libc_fread(ptr, size, nobj, stream) }
    }
}

unsafe extern "system" fn interposed_write(
    ptr: *const c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> size_t {
    let fd = unsafe { libc::fileno(stream) };
    let handle = FileHandle(fd);

    if handle.is_fake() {
        let _entered = trace_span!("write", ?fd, ?size, ?nobj).entered();

        let handle_store = HANDLE_STORE.lock();
        if let Some(stream) = handle_store.borrow_mut().get_stream(handle) {
            // FIXME(jules): this is subtly wrong, it should attempt to read *up to* size elements of nobj size
            let slice = unsafe { slice::from_raw_parts_mut(ptr as *mut u8, size * nobj) };
            let res = stream.write(&slice[..]);

            match res {
                Ok(wrote) => {
                    trace!(?wrote, "wrote to fake handle");
                    wrote
                }
                Err(error) => {
                    error!(?error, "failed to write to fake handle");
                    0
                }
            }
        } else {
            warn!("tried to read unknown fake handle");
            0
        }
    } else {
        unsafe { libc_fwrite(ptr, size, nobj, stream) }
    }
}

pub fn init() -> crate::Result<()> {
    // not needed for macos
    Ok(())
}

impl HandleStore {
    pub(crate) fn allocate_file_handle(&self) -> FileHandle {
        FileHandle::allocate_fake()
    }
}

unsafe extern "C" {
    #[link_name = "open$NOCANCEL"]
    unsafe fn libsystem_kernel_open_nocancel(
        filename: *const c_char,
        flags: c_int,
        mode: mode_t,
    ) -> c_int;
    #[link_name = "fclose"]
    unsafe fn libc_fclose(stream: *mut FILE) -> c_int;
    #[link_name = "ftello"]
    unsafe fn libc_ftello(stream: *mut FILE) -> off_t;
    #[link_name = "fseeko"]
    unsafe fn libc_fseeko(stream: *mut FILE, offset: off_t, whence: c_int) -> off_t;
    #[link_name = "fread"]
    unsafe fn libc_fread(ptr: *mut c_void, size: size_t, nobj: size_t, stream: *mut FILE)
    -> size_t;
    #[link_name = "fwrite"]
    unsafe fn libc_fwrite(
        ptr: *const c_void,
        size: size_t,
        nobj: size_t,
        stream: *mut FILE,
    ) -> size_t;
}

#[repr(C)]
struct InterposeEntry {
    replacement: *const c_void,
    original: *const c_void,
}

unsafe impl Sync for InterposeEntry {}

// In macOS/iOS, fopen uses open$NOCANCEL/__open_nocancel internally.
#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_OPEN_NOCANCEL: InterposeEntry = {
    InterposeEntry {
        replacement: interposed_open_nocancel as *const c_void,
        original: libsystem_kernel_open_nocancel as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_CLOSE: InterposeEntry = {
    InterposeEntry {
        replacement: interposed_close as *const c_void,
        original: libc_fclose as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_TELL: InterposeEntry = {
    InterposeEntry {
        replacement: interposed_tell as *const c_void,
        original: libc_ftello as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_SEEK: InterposeEntry = {
    InterposeEntry {
        replacement: interposed_seek as *const c_void,
        original: libc_fseeko as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_READ: InterposeEntry = {
    InterposeEntry {
        replacement: interposed_read as *const c_void,
        original: libc_fread as *const c_void,
    }
};

#[used]
#[unsafe(link_section = "__DATA,__interpose")]
static INTERPOSE_WRITE: InterposeEntry = {
    InterposeEntry {
        replacement: interposed_write as *const c_void,
        original: libc_fwrite as *const c_void,
    }
};
