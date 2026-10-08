use std::{
    ffi::{c_char, c_void},
    str::FromStr,
    sync::OnceLock,
};

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostfxrDelegateType {
    ComActivation,
    LoadInMemoryAssembly,
    WinrtActivation,
    ComRegister,
    ComUnregister,
    LoadAssemblyAndGetFunctionPointer,
    GetFunctionPointer,
}

pub type HostfxrGetRuntimeDelegateFn = unsafe extern "C" fn(
    host_context_handle: *const c_void,
    r#type: HostfxrDelegateType,
    delegate: *mut *const c_void,
) -> i32;

pub type LoadAssemblyAndGetFunctionPointerFn = unsafe extern "C" fn(
    assembly_path: *const c_char,
    type_name: *const c_char,
    method_name: *const c_char,
    delegate_type_name: *const c_char,
    reserved: *const c_void,
    delegate: *mut *const c_void,
) -> i32;

static LOAD_ASSEMBLY_AND_GET_FUNCTION_POINTER_ORIGINAL: OnceLock<
    LoadAssemblyAndGetFunctionPointerFn,
> = OnceLock::new();

// hostfxr strings are UTF-16 on Windows and UTF-8 on Unix :(
pub struct HostfxrString {
    #[cfg(windows)]
    inner: widestring::U16CString,
    #[cfg(not(windows))]
    inner: std::ffi::CString,
}

impl HostfxrString {
    pub fn from_ptr(ptr: *const c_char) -> crate::Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self {
                inner: unsafe { widestring::U16CString::from_ptr_str(ptr as _) },
            })
        }

        #[cfg(not(windows))]
        {
            todo!()
        }
    }

    pub fn to_string(&self) -> crate::Result<String> {
        #[cfg(windows)]
        {
            self.inner.to_string().map_err(|_| crate::Error::String)
        }

        #[cfg(not(windows))]
        {
            todo!()
        }
    }

    pub fn to_ptr(&self) -> *const c_char {
        #[cfg(windows)]
        {
            self.inner.as_ptr() as *const c_char
        }

        #[cfg(not(windows))]
        {
            todo!()
        }
    }
}

impl FromStr for HostfxrString {
    type Err = crate::Error;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        #[cfg(windows)]
        {
            Ok(Self {
                inner: widestring::U16CString::from_str(str).map_err(|_| crate::Error::String)?,
            })
        }

        #[cfg(not(windows))]
        {
            todo!()
        }
    }
}

fn load_assembly_and_get_function_pointer_detour(
    assembly_path: *const c_char,
    type_name: *const c_char,
    method_name: *const c_char,
    delegate_type_name: *const c_char,
    reserved: *const c_void,
    delegate: *mut *const c_void,
) -> i32 {
    let orig = LOAD_ASSEMBLY_AND_GET_FUNCTION_POINTER_ORIGINAL
        .get()
        .expect("original function to be initialized");

    {
        // FIXME: better error handling here
        let _: crate::Result<()> = try {
            if let Ok(type_name) = HostfxrString::from_ptr(type_name)
                && let Ok(type_name) = type_name.to_string()
                && type_name.starts_with("GodotPlugins.Game.Main")
            {
                let assembly_path = crate::LOADER_ASSEMBLY
                    .get()
                    .ok_or(crate::Error::Unknown)?
                    .clone();
                let assembly_path = assembly_path
                    .into_string()
                    .map_err(|_| crate::Error::Unknown)?;
                let assembly_path = HostfxrString::from_str(&assembly_path)?;

                let type_name =
                    HostfxrString::from_str("GDPatchSharp.Loader.Entrypoint, GDPatchSharp.Loader")?;
                let method_name = HostfxrString::from_str("Main")?;
                let delegate_type_name = HostfxrString::from_str(
                    "GDPatchSharp.Loader.Entrypoint+MainDelegate, GDPatchSharp.Loader",
                )?;
                let mut delegate = std::ptr::null();

                unsafe {
                    let res = orig(
                        assembly_path.to_ptr(),
                        type_name.to_ptr(),
                        method_name.to_ptr(),
                        delegate_type_name.to_ptr(),
                        std::ptr::null(),
                        &raw mut delegate,
                    );

                    if res == 0 && !delegate.is_null() {
                        // FIXME: types
                        #[allow(clippy::missing_transmute_annotations)]
                        let delegate = std::mem::transmute::<_, unsafe extern "C" fn()>(delegate);
                        delegate();
                    }
                };
            }
        };
    }

    unsafe {
        orig(
            assembly_path,
            type_name,
            method_name,
            delegate_type_name,
            reserved,
            delegate,
        )
    }
}

pub(crate) fn hostfxr_get_runtime_delegate_detour(
    host_context_handle: *const c_void,
    r#type: HostfxrDelegateType,
    delegate: *mut *const c_void,
    original: HostfxrGetRuntimeDelegateFn,
) -> i32 {
    let result = unsafe { original(host_context_handle, r#type, delegate) };

    if r#type == HostfxrDelegateType::LoadAssemblyAndGetFunctionPointer {
        unsafe {
            let orig = *delegate;

            // FIXME: types
            #[allow(clippy::missing_transmute_annotations)]
            LOAD_ASSEMBLY_AND_GET_FUNCTION_POINTER_ORIGINAL
                .set(std::mem::transmute::<_, _>(orig))
                .ok();

            *delegate = load_assembly_and_get_function_pointer_detour as _;

            0
        }
    } else {
        result
    }
}
