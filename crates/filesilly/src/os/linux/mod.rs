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
    fs::File,
    hash::{Hash, Hasher},
    os::{fd::AsRawFd, raw::c_void},
    path::{Path, PathBuf},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
struct WrappedFd(pub c_int);

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
    source_file: File,

    /// Map of currently open streams.
    file_descriptors: DashMap<WrappedFd, HeapStream, BuildIdentityHasher<WrappedFd>>,

    /// The paths for the base directories we're modifying.
    base_paths: Vec<PathBuf>,
}

impl FilesillyPlatform {
    pub fn new(base_paths: &[&Path]) -> crate::Result<Self> {
        let source_file = File::open("/dev/null")?;

        Ok(Self {
            source_file,
            file_descriptors: DashMap::default(),
            base_paths: base_paths.iter().map(|p| p.to_path_buf()).collect(),
        })
    }

    fn allocate_fd_for_stream(&self, stream: HeapStream) -> crate::Result<WrappedFd> {
        let source_fd = self.source_file.as_raw_fd();

        let fd = unsafe { libc::dup(source_fd) };
        if fd == -1 {
            return Err(std::io::Error::last_os_error().into());
        }

        let fd = WrappedFd(fd);
        self.file_descriptors.insert(fd, stream);
        Ok(fd)
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
