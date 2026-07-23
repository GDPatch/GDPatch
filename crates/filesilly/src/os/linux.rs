use crate::{Error, HANDLE_STORE, HandleStore};
use libc::{FILE, c_char, c_int, mode_t, off_t, off64_t, size_t};
use retour::{Function, GenericDetour, HookableWith};
use std::{
    ffi::{CStr, OsStr, OsString},
    io::SeekFrom,
    mem,
    os::{raw::c_void, unix::ffi::OsStrExt},
    path::PathBuf,
    slice,
    sync::{
        LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};
use tracing::{error, trace, trace_span, warn};

/// Starting point for fake handles.
const FAKE_HANDLE_START: usize = 0x90D07; // "GODOT" :+1:

static NEXT_FAKE_HANDLE: AtomicUsize = AtomicUsize::new(FAKE_HANDLE_START);

/// Wrapper around a [`HANDLE`] that implements Handle/Send/Sync.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileHandle(c_int);

impl FileHandle {
    pub fn is_fake(&self) -> bool {
        self.0 > 0 && self.0 as usize >= FAKE_HANDLE_START
    }

    /// Allocates an unused fake handle.
    pub fn allocate_fake() -> FileHandle {
        let handle = NEXT_FAKE_HANDLE.fetch_add(1, Ordering::Relaxed);
        FileHandle(handle as c_int)
    }
}

unsafe impl Send for FileHandle {}
unsafe impl Sync for FileHandle {}

fn get_export(export: &str, version: &str) -> crate::Result<*mut std::ffi::c_void> {
    unsafe {
        let libc = libc::dlopen(c"libc.so".as_ptr() as *const c_char, libc::RTLD_LAZY);
        let result = libc::dlvsym(
            libc,
            export.as_ptr() as *const c_char,
            version.as_ptr() as *const c_char,
        );

        if result.is_null() {
            Err(Error::Hook)
        } else {
            Ok(result)
        }
    }
}

fn mark_errno(value: c_int) {
    unsafe {
        *libc::__errno_location() = value;
    }
}

struct SillyHook<T: HookableWith<D>, D: Function> {
    hook: crate::Result<GenericDetour<T>>,
    #[allow(dead_code)]
    detour: D,
}

impl<T: HookableWith<D>, D: Function> SillyHook<T, D> {
    fn hook(export: &str, version: &str, detour: D) -> crate::Result<GenericDetour<T>> {
        unsafe {
            let export = get_export(export, version)?;
            let export: T = mem::transmute_copy(&export);
            GenericDetour::new(export, detour).map_err(|_| Error::Hook)
        }
    }

    pub fn new(export: &str, version: &str, detour: D) -> Self {
        Self {
            hook: SillyHook::hook(export, version, detour),
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

type OpenFn =
    unsafe extern "system" fn(filename: *const c_char, flags: c_int, mode: mode_t) -> c_int;
static OPEN_HOOK: LockDetour<OpenFn> =
    LazyLock::new(|| SillyHook::new("open64\0", "GLIBC_2.2.5\0", open_detour));

unsafe extern "system" fn open_detour(
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
            warn!(?err, path = %path.display(), "factory errored");
            mark_errno(libc::EINVAL);
            return -1;
        }

        // factory ran successfully
        Some(Ok(handle)) => handle,
    };

    match handle {
        None => unsafe { OPEN_HOOK.unwrap().call(filename, flags, mode) },

        Some(handle) => {
            trace!(?handle, "created fake handle");
            handle.0
        }
    }
}

type CloseFn = unsafe extern "system" fn(stream: *mut FILE) -> c_int;
static CLOSE_HOOK: LockDetour<CloseFn> =
    LazyLock::new(|| SillyHook::new("fclose\0", "GLIBC_2.2.5\0", close_detour));

unsafe extern "system" fn close_detour(stream: *mut FILE) -> c_int {
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
        unsafe { CLOSE_HOOK.unwrap().call(stream) }
    }
}

type TellFn = unsafe extern "system" fn(stream: *mut FILE) -> off64_t;
static TELL_HOOK: LockDetour<TellFn> =
    LazyLock::new(|| SillyHook::new("ftello64\0", "GLIBC_2.2.5\0", tell_detour));

unsafe extern "system" fn tell_detour(stream: *mut FILE) -> off64_t {
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
        unsafe { TELL_HOOK.unwrap().call(stream) }
    }
}

type SeekFn =
    unsafe extern "system" fn(stream: *mut FILE, offset: off64_t, whence: c_int) -> off64_t;
static SEEK_HOOK: LockDetour<SeekFn> =
    LazyLock::new(|| SillyHook::new("fseeko64\0", "GLIBC_2.2.5\0", seek_detour));

unsafe extern "system" fn seek_detour(stream: *mut FILE, offset: off_t, whence: c_int) -> off64_t {
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
                    offset as off64_t
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
        unsafe { SEEK_HOOK.unwrap().call(stream, offset, whence) }
    }
}

type ReadFn = unsafe extern "system" fn(
    ptr: *mut c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> size_t;
static READ_HOOK: LockDetour<ReadFn> =
    LazyLock::new(|| SillyHook::new("fread\0", "GLIBC_2.2.5\0", read_detour));

unsafe extern "system" fn read_detour(
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
        unsafe { READ_HOOK.unwrap().call(ptr, size, nobj, stream) }
    }
}

type WriteFn = unsafe extern "system" fn(
    ptr: *const c_void,
    size: size_t,
    nobj: size_t,
    stream: *mut FILE,
) -> size_t;
static WRITE_HOOK: LockDetour<WriteFn> =
    LazyLock::new(|| SillyHook::new("fwrite\0", "GLIBC_2.2.5\0", write_detour));

unsafe extern "system" fn write_detour(
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
        unsafe { WRITE_HOOK.unwrap().call(ptr, size, nobj, stream) }
    }
}

pub fn init() -> crate::Result<()> {
    unsafe {
        OPEN_HOOK.enable()?;
        CLOSE_HOOK.enable()?;
        TELL_HOOK.enable()?;
        SEEK_HOOK.enable()?;
        READ_HOOK.enable()?;
        WRITE_HOOK.enable()?;
    }

    Ok(())
}

impl HandleStore {
    pub(crate) fn allocate_file_handle(&self) -> FileHandle {
        FileHandle::allocate_fake()
    }
}
