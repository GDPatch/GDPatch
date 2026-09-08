use crate::{
    Error,
    hook::{HostfxrDelegateType, HostfxrGetRuntimeDelegateFn},
};
use retour::static_detour;
use std::{ffi::c_void, sync::OnceLock};
use windows::{
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
    core::{PCSTR, PCWSTR, s, w},
};

static_detour! {
    static GetProcAddressHook: fn(HMODULE, PCSTR) -> *const c_void;
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

static GET_RUNTIME_DELEGATE_ORIGINAL: OnceLock<HostfxrGetRuntimeDelegateFn> = OnceLock::new();

fn hostfxr_get_runtime_delegate_detour(
    host_context_handle: *const c_void,
    r#type: HostfxrDelegateType,
    delegate: *mut *const c_void,
) -> i32 {
    crate::hook::hostfxr_get_runtime_delegate_detour(
        host_context_handle,
        r#type,
        delegate,
        *GET_RUNTIME_DELEGATE_ORIGINAL
            .get()
            .expect("original function to be initialized"),
    )
}

fn get_proc_address_detour(module: HMODULE, proc: PCSTR) -> *const c_void {
    let result = GetProcAddressHook.call(module, proc);

    unsafe {
        // If this parameter is an ordinal value, it must be in the low-order word; the high-order word must be zero.
        if !proc.is_null()
            && proc.0 as usize > 0xFFFF
            && let Ok(str) = proc.to_string()
            && str == "hostfxr_get_runtime_delegate"
        {
            // FIXME: types

            #[allow(clippy::missing_transmute_annotations)]
            GET_RUNTIME_DELEGATE_ORIGINAL
                .set(std::mem::transmute::<_, _>(result))
                .ok();

            return hostfxr_get_runtime_delegate_detour as _;
        }
    }

    result
}

pub fn init() -> crate::Result<()> {
    // Amusingly, we must GetProcAddress GetProcAddress to avoid the windows crate's stub
    let get_proc_address = get_export(w!("kernel32.dll"), s!("GetProcAddress"))?;

    unsafe {
        GetProcAddressHook
            .initialize(
                // FIXME: types
                #[allow(clippy::missing_transmute_annotations)]
                std::mem::transmute::<_, _>(get_proc_address),
                get_proc_address_detour,
            )
            .map_err(|_| crate::Error::Hook)?;
        GetProcAddressHook
            .enable()
            .map_err(|_| crate::Error::Hook)?;
    }

    Ok(())
}
