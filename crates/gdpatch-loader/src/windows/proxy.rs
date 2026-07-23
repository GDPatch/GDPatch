use color_eyre::eyre::OptionExt;
use once_cell::sync::OnceCell;
use std::ffi::OsString;
use std::iter::once;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_FLAGS, LOAD_WITH_ALTERED_SEARCH_PATH, LoadLibraryExW,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::core::PCWSTR;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/proxy_generated.rs"));
}

#[derive(Debug)]
struct Module(pub HMODULE);

unsafe impl Sync for Module {}
unsafe impl Send for Module {}

static HANDLE: OnceCell<Module> = OnceCell::new();

fn path_to_windows_buffer(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>()
}

fn find_original_library(current_dll: &Path) -> color_eyre::Result<Option<HMODULE>> {
    let file_name = current_dll
        .file_name()
        .expect("should be able to get filename from current DLL path");

    let file_name = file_name.to_str().ok_or_eyre("non UTF-8 DLL filename")?;

    // check for `<name>.orig.dll` in the current directory
    let local_override_path = {
        let (file_name, ext) = match file_name.rsplit_once(".") {
            None => (file_name, None),
            Some((prefix, suffix)) => (prefix, Some(suffix)),
        };

        let mut expected_file_name = format!("{file_name}.orig");

        if let Some(ext) = ext {
            expected_file_name.push('.');
            expected_file_name.push_str(ext);
        }

        expected_file_name.push('.');
        current_dll.with_file_name(expected_file_name)
    };

    unsafe {
        let as_windows = path_to_windows_buffer(&local_override_path);
        let handle = LoadLibraryExW(
            PCWSTR(as_windows.as_ptr()),
            None,
            LOAD_LIBRARY_FLAGS::default(),
        );

        if let Ok(handle) = handle {
            return Ok(Some(handle));
        }
    };

    // find the original DLL in %SYSTEMROOT%
    let system_directory = unsafe {
        let required_length = GetSystemDirectoryW(None);
        let mut buffer = vec![0u16; required_length as usize];
        let length = GetSystemDirectoryW(Some(&mut buffer));
        OsString::from_wide(&buffer[..length as usize])
    };

    let system_directory = Path::new(&system_directory);
    let system_dll_path = system_directory.join(file_name);

    unsafe {
        let as_windows = path_to_windows_buffer(&system_dll_path);
        let handle = LoadLibraryExW(
            PCWSTR(as_windows.as_ptr()),
            None,
            LOAD_WITH_ALTERED_SEARCH_PATH,
        );

        if let Ok(handle) = handle {
            return Ok(Some(handle));
        }
    };

    Ok(None)
}

/// Loads symbols to forward. Checks for `<name>_orig.dll` in the current directory, and then for
/// `%SYSTEMROOT%/<name>.dll`.
pub fn load(current_dll: &Path) -> color_eyre::Result<()> {
    HANDLE.get_or_try_init(|| {
        let handle =
            find_original_library(current_dll)?.ok_or_eyre("couldn't find original DLL")?;

        unsafe { generated::find_exports(handle) }

        Ok::<Module, color_eyre::Report>(Module(handle))
    })?;

    Ok(())
}

/// Unloads the module handle previously loaded by [`load`].
///
/// # Safety
/// This must only be called once to prevent double-freeing the handle.
pub unsafe fn unload() {
    if let Some(handle) = HANDLE.get() {
        unsafe {
            let _ = FreeLibrary(handle.0);
        }
    }
}
