//! Safe wrappers for GDExtension string types.

use crate::gdextension::api::API;
use crate::gdextension::sys::variant::GDExtensionVariant;
use crate::gdextension::sys::{
    GDExtensionConstTypePtr, GDExtensionInt, GDExtensionStringPtr, GDExtensionType,
    GDExtensionTypePtr, GDExtensionUninitializedTypePtr, GDExtensionVariantType, char32_t,
};
use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::{ptr, slice};

#[repr(C)]
#[derive(Debug)]
pub struct GDExtensionString(*const ());

impl GDExtensionString {
    /// Returns the length of this string in `char`s.
    pub fn len(&self) -> usize {
        let length = unsafe { (API.interface.string_to_utf32_chars)(self, ptr::null_mut(), 0) };
        length as usize
    }

    /// Returns whether this string is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends a character to this string.
    pub fn push(&mut self, c: char) {
        unsafe { (API.interface.string_operator_plus_eq_char)(self, c as char32_t) }
    }

    /// Appends another string to the end of this string.
    pub fn push_str(&mut self, other: &GDExtensionString) {
        unsafe { (API.interface.string_operator_plus_eq_string)(self, other) }
    }

    pub fn as_ptr(&self) -> *const char {
        let ptr = unsafe { (API.interface.string_operator_index_const)(self, 0) };
        ptr.cast::<char>()
    }

    pub fn as_mut_ptr(&mut self) -> *mut char {
        let ptr = unsafe { (API.interface.string_operator_index)(self, 0) };
        ptr.cast::<char>()
    }
}

impl Default for GDExtensionString {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for GDExtensionString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let converted = String::from(self);
        f.write_str(&converted)
    }
}

impl Drop for GDExtensionString {
    fn drop(&mut self) {
        unsafe { (API.destruct_string)(&raw mut *self as GDExtensionTypePtr) }
    }
}

impl Clone for GDExtensionString {
    fn clone(&self) -> Self {
        let mut result = MaybeUninit::uninit();
        let args = [&raw const *self as GDExtensionTypePtr];

        unsafe {
            (API.construct_string_from_string_name)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                &raw const args as _,
            );

            result.assume_init()
        }
    }
}

impl Deref for GDExtensionString {
    type Target = [char];

    fn deref(&self) -> &Self::Target {
        let length = self.len();

        if length == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.as_ptr(), length) }
        }
    }
}

impl DerefMut for GDExtensionString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let length = self.len();

        if length == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), length) }
        }
    }
}

impl From<&CStr> for GDExtensionString {
    fn from(value: &CStr) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.interface.string_new_with_latin1_chars)(&mut result, value.as_ptr() as *const _);
            result.assume_init()
        }
    }
}

impl From<&str> for GDExtensionString {
    fn from(value: &str) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.interface.string_new_with_utf8_chars_and_len)(
                &mut result,
                value.as_ptr() as *const _,
                value.len() as i64,
            );

            result.assume_init()
        }
    }
}

impl From<&GDExtensionString> for String {
    fn from(value: &GDExtensionString) -> Self {
        let mut buf = Vec::with_capacity(64);

        loop {
            let capacity = buf.capacity();

            let len = unsafe {
                (API.interface.string_to_utf8_chars)(
                    &raw const *value as GDExtensionStringPtr,
                    buf.as_mut_ptr() as _,
                    capacity as GDExtensionInt,
                )
            };

            if capacity >= (len as usize) {
                // SAFETY: Buffer was initialized to this length.
                unsafe { buf.set_len(len as usize) };

                // SAFETY: `string_to_latin1_chars` never writes null bytes.
                break unsafe { String::from_utf8_unchecked(buf) };
            } else {
                buf.reserve(len as usize);
            }
        }
    }
}

impl GDExtensionType for GDExtensionString {
    const VARIANT_TYPE: GDExtensionVariantType = GDExtensionVariantType::String;

    fn new() -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_string)(&raw mut result as _, ptr::null());
            result.assume_init()
        }
    }

    fn from_variant(variant: &GDExtensionVariant) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_string_from_variant)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                variant,
            );

            result.assume_init()
        }
    }

    fn to_variant(&self) -> GDExtensionVariant {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_variant_from_string)(
                &mut result,
                &raw const *self as GDExtensionConstTypePtr,
            );

            result.assume_init()
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct GDExtensionStringName(*const ());

impl Default for GDExtensionStringName {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for GDExtensionStringName {
    fn from(value: &str) -> Self {
        (&GDExtensionString::from(value)).into()
    }
}

impl From<&GDExtensionString> for GDExtensionStringName {
    fn from(value: &GDExtensionString) -> Self {
        let mut result = MaybeUninit::uninit();
        let args = [&raw const *value];

        unsafe {
            (API.construct_string_name_from_string)(&raw mut result as _, &raw const args as _);
            result.assume_init()
        }
    }
}

impl From<&GDExtensionStringName> for GDExtensionVariant {
    fn from(value: &GDExtensionStringName) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_variant_from_string_name)(
                &mut result,
                &raw const value as GDExtensionTypePtr,
            );
            result.assume_init()
        }
    }
}

impl Drop for GDExtensionStringName {
    fn drop(&mut self) {
        unsafe { (API.destruct_string_name)(&raw mut *self as GDExtensionTypePtr) }
    }
}

impl Clone for GDExtensionStringName {
    fn clone(&self) -> Self {
        let mut result = MaybeUninit::uninit();
        let args = [&raw const *self as GDExtensionTypePtr];

        unsafe {
            (API.construct_string_name_from_string_name)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                &raw const args as _,
            );

            result.assume_init()
        }
    }
}

impl GDExtensionType for GDExtensionStringName {
    const VARIANT_TYPE: GDExtensionVariantType = GDExtensionVariantType::StringName;

    fn new() -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_string_name)(&raw mut result as _, ptr::null());
            result.assume_init()
        }
    }

    fn from_variant(variant: &GDExtensionVariant) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_string_name_from_variant)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                variant,
            );

            result.assume_init()
        }
    }

    fn to_variant(&self) -> GDExtensionVariant {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_variant_from_string_name)(
                &mut result,
                &raw const *self as GDExtensionConstTypePtr,
            );

            result.assume_init()
        }
    }
}
