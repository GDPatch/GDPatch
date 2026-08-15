mod detours;
mod util;

use crate::{
    Error, Filesilly, HeapStream, StreamFactory,
    os::linux::detours::{
        ACCESS_HOOK, CLOSE_HOOK, OPEN_HOOK, READ_HOOK, SEEK_HOOK, STAT_HOOK, TELL_HOOK, WRITE_HOOK,
    },
};
use dashmap::DashMap;
use identity_hash::{BuildIdentityHasher, IdentityHashable};
use libc::c_int;
use std::{
    ffi::CStr,
    hash::{Hash, Hasher},
    os::raw::c_void,
    path::{Path, PathBuf},
    sync::atomic::{AtomicI32, Ordering},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
struct WrappedFd(pub c_int);

unsafe impl Send for WrappedFd {}
unsafe impl Sync for WrappedFd {}

impl IdentityHashable for WrappedFd {}
impl Hash for WrappedFd {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.0);
    }
}

impl From<c_int> for WrappedFd {
    fn from(h: c_int) -> Self {
        WrappedFd(h)
    }
}

#[derive(Debug)]
pub struct FilesillyPlatform {
    next_handle: AtomicI32,

    /// Map of currently open streams.
    file_descriptors: DashMap<WrappedFd, HeapStream, BuildIdentityHasher<WrappedFd>>,

    /// The paths for the base directories we're modifying.
    base_paths: Vec<PathBuf>,
}

impl FilesillyPlatform {
    const FAKE_HANDLE_START: c_int = 0x90D07; // "GODOT" :+1:

    pub fn new(base_paths: &[&Path]) -> crate::Result<Self> {
        let next_handle = AtomicI32::new(Self::FAKE_HANDLE_START);

        Ok(Self {
            next_handle,
            file_descriptors: DashMap::default(),
            base_paths: base_paths.iter().map(|p| p.to_path_buf()).collect(),
        })
    }

    fn allocate_fd_for_stream(&self, stream: HeapStream) -> WrappedFd {
        let fd = self.next_handle.fetch_add(1, Ordering::Relaxed);
        let fd = WrappedFd(fd);
        self.file_descriptors.insert(fd, stream);
        fd
    }

    fn get_stream(&self, fd: c_int) -> Option<HeapStream> {
        let r = self.file_descriptors.get(&WrappedFd(fd))?;
        Some(r.clone())
    }
}

pub fn init(base_path: &[&Path], factory: Box<dyn StreamFactory>) -> crate::Result<()> {
    let platform = FilesillyPlatform::new(base_path)?;
    Filesilly::setup(platform, factory);

    unsafe {
        OPEN_HOOK.enable()?;
        CLOSE_HOOK.enable()?;
        READ_HOOK.enable()?;
        WRITE_HOOK.enable()?;
        SEEK_HOOK.enable()?;
        TELL_HOOK.enable()?;
        STAT_HOOK.enable()?;
        ACCESS_HOOK.enable()?;
    }

    Ok(())
}

pub fn get_export(version: &CStr, export: &CStr) -> crate::Result<*const c_void> {
    unsafe {
        let libc = libc::dlopen(c"libc.so".as_ptr(), libc::RTLD_LAZY);
        let result = libc::dlvsym(libc, export.as_ptr(), version.as_ptr());

        if result.is_null() {
            Err(Error::Hook)
        } else {
            Ok(result)
        }
    }
}
