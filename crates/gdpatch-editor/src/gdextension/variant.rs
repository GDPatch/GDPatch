use crate::gdextension::{GDExtensionTypePtr, api::get_api};
use std::{
    ffi::{CString, NulError, c_void},
    fmt::Display,
    str::FromStr,
};

#[repr(C)]
#[derive(Debug)]
pub struct GDExtensionString {
    pub ptr: *const c_void,
}

impl Display for GDExtensionString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let api = get_api();

            let len = (api.string_to_utf8_chars)(&raw const *self, std::ptr::null_mut(), 0);
            let mut buf = vec![0u8; len as usize];
            (api.string_to_utf8_chars)(&raw const *self, buf.as_mut_ptr() as _, len);

            let str = std::str::from_utf8_unchecked(&buf);

            f.write_str(str)
        }
    }
}

impl FromStr for GDExtensionString {
    type Err = NulError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unsafe {
            let api = get_api();
            let cstring = CString::new(s)?;

            let mut string = GDExtensionString {
                ptr: std::ptr::null(),
            };

            (api.string_new_with_utf8_chars_and_len)(
                &raw mut string,
                cstring.as_ptr(),
                cstring.count_bytes() as i64,
            );

            Ok(string)
        }
    }
}

impl Drop for GDExtensionString {
    fn drop(&mut self) {
        unsafe {
            let api = get_api();
            (api.destructor_string)(GDExtensionTypePtr(&raw mut *self as _))
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct GDExtensionStringName {
    ptr: *const c_void,
}

impl From<&GDExtensionString> for GDExtensionStringName {
    fn from(value: &GDExtensionString) -> Self {
        unsafe {
            let api = get_api();

            let mut string_name = GDExtensionStringName {
                ptr: std::ptr::null(),
            };

            let args = [&raw const *value];
            (api.constructor_string_name)(
                GDExtensionTypePtr(&raw mut string_name as _),
                &raw const args as _,
            );

            string_name
        }
    }
}

impl Drop for GDExtensionStringName {
    fn drop(&mut self) {
        unsafe {
            let api = get_api();
            (api.destructor_string_name)(GDExtensionTypePtr(&raw mut *self as _))
        }
    }
}
