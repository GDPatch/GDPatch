mod detours;
mod util;

use crate::os::windows::detours::{
    NT_CLOSE_HOOK, NT_QUERY_INFORMATION_FILE_HOOK, NT_QUERY_VOLUME_INFORMATION_FILE_HOOK,
    NT_READ_FILE_HOOK, NT_SET_INFORMATION_FILE_HOOK, NT_WRITE_FILE_HOOK,
};
use crate::os::windows::util::normalize_path;
use crate::{Error, Filesilly, HeapStream, StreamFactory};
use dashmap::DashMap;
use detours::NT_CREATE_FILE_HOOK;
use identity_hash::{BuildIdentityHasher, IdentityHashable};
use std::ffi::CStr;
use std::hash::{Hash, Hasher};
use std::iter::once;
use std::os::raw::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle};
use windows::Win32::System::Threading::{CreateEventW, GetCurrentProcess};
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
    core::{PCSTR, PCWSTR},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
struct WrappedHandle(pub HANDLE);

unsafe impl Send for WrappedHandle {}
unsafe impl Sync for WrappedHandle {}

impl IdentityHashable for WrappedHandle {}
impl Hash for WrappedHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.0 as usize);
    }
}

impl From<HANDLE> for WrappedHandle {
    fn from(h: HANDLE) -> Self {
        WrappedHandle(h)
    }
}

#[derive(Debug)]
struct BasePath {
    /// NT object manager format path.
    nt: Vec<u16>,

    /// Base directory as passed to the initializer.
    rust: PathBuf,
}

#[derive(Debug)]
pub struct FilesillyPlatform {
    /// Event handle we duplicate to provide proxy handles.
    source_handle: WrappedHandle,

    /// Map of currently open streams.
    handles: DashMap<WrappedHandle, HeapStream, BuildIdentityHasher<WrappedHandle>>,

    base_paths: Vec<BasePath>,
}

impl FilesillyPlatform {
    pub fn new(base_paths: &[&Path]) -> windows::core::Result<Self> {
        // Make an unnamed event to get a handle to a kernel object we can use.
        let source_handle = unsafe { CreateEventW(None, true, false, None)? };

        let base_paths = base_paths
            .iter()
            .map(|path| {
                let nt = path
                    .as_os_str()
                    .encode_wide()
                    .chain(once(0u16))
                    .collect::<Vec<_>>();

                let mut nt = normalize_path(&nt)?;

                if !nt.ends_with(&['\\' as u16]) {
                    nt.push('\\' as u16);
                }

                let rust = PathBuf::from(path);

                Ok(BasePath { nt, rust })
            })
            .collect::<windows::core::Result<Vec<BasePath>>>()?;

        Ok(Self {
            source_handle: source_handle.into(),
            handles: DashMap::default(),
            base_paths,
        })
    }

    fn allocate_handle_for_stream(
        &self,
        stream: HeapStream,
    ) -> windows::core::Result<WrappedHandle> {
        let mut handle = HANDLE::default();

        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                self.source_handle.0,
                GetCurrentProcess(),
                &raw mut handle,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )?;
        }

        debug_assert!(!handle.is_invalid());

        let handle = WrappedHandle(handle);
        self.handles.insert(handle, stream);
        Ok(handle)
    }

    fn get_stream(&self, handle: HANDLE) -> Option<HeapStream> {
        let r = self.handles.get(&WrappedHandle(handle))?;
        Some(r.clone())
    }
}

impl Drop for FilesillyPlatform {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.source_handle.0);
        }
    }
}

pub fn init(base_paths: &[&Path], factory: Box<dyn StreamFactory>) -> crate::Result<()> {
    let platform = FilesillyPlatform::new(base_paths).map_err(Error::from_windows)?;
    Filesilly::setup(platform, factory);

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

impl Error {
    // impl From<...> makes this publicly visible
    fn from_windows(error: windows::core::Error) -> Self {
        Self::System {
            code: error.code().0 as u32,
            message: error.message(),
        }
    }
}

pub fn get_export(module: &CStr, export: &CStr) -> crate::Result<*const c_void> {
    unsafe {
        let wide_module = module
            .to_bytes_with_nul()
            .iter()
            .map(|c| *c as u16)
            .collect::<Vec<_>>();

        let module = LoadLibraryW(PCWSTR(wide_module.as_ptr())).map_err(Error::from_windows)?;

        match GetProcAddress(module, PCSTR(export.as_ptr().cast::<u8>())) {
            Some(ptr) => Ok(ptr as *const std::ffi::c_void),
            None => {
                let win_err = windows::core::Error::from_thread();
                Err(Error::from_windows(win_err))
            }
        }
    }
}
