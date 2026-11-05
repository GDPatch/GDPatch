use crate::gdextension::api::API;
use crate::gdextension::sys::string::GDExtensionStringName;
use crate::gdextension::sys::{GDExtensionCallError, GDExtensionCallErrorType, GDExtensionConstVariantPtr, GDExtensionInt, GDExtensionVariantType};
use std::mem::MaybeUninit;

#[repr(C, align(8))] // data field is alignas(8)
#[derive(Debug)]
// If the engine is compiled with real_t as a double then Variant is 40 bytes. Most of the
// time this will not be the case, but there is no harm in allocating too large of a variant;
// it is only passed across the ABI boundary as a pointer.
pub struct GDExtensionVariant([MaybeUninit<u8>; 40]);

impl GDExtensionVariant {
    /// Creates a new nil variant.
    pub fn new() -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.interface.variant_new_nil)(&mut result);
            result.assume_init()
        }
    }

    pub fn call(
        &mut self,
        method: &GDExtensionStringName,
        arguments: &[&GDExtensionVariant],
    ) -> Result<GDExtensionVariant, GDExtensionCallError> {
        let mut result = MaybeUninit::uninit();
        let mut error = GDExtensionCallError::new();

        unsafe {
            (API.interface.variant_call)(
                self,
                method,
                arguments.as_ptr().cast::<GDExtensionConstVariantPtr>(),
                arguments.len() as GDExtensionInt,
                &mut result,
                &mut error
            );
        }

        if error.error == GDExtensionCallErrorType::Ok {
            Ok(unsafe { result.assume_init() })
        } else {
            Err(error)
        }
    }

    pub fn call_static(
        r#type: GDExtensionVariantType,
        method: &GDExtensionStringName,
        arguments: &[&GDExtensionVariant],
    ) -> Result<GDExtensionVariant, GDExtensionCallError> {
        let mut result = MaybeUninit::uninit();
        let mut error = GDExtensionCallError::new();

        unsafe {
            (API.interface.variant_call_static)(
                r#type,
                method,
                arguments.as_ptr().cast::<GDExtensionConstVariantPtr>(),
                arguments.len() as GDExtensionInt,
                &mut result,
                &mut error
            );
        }

        if error.error == GDExtensionCallErrorType::Ok {
            Ok(unsafe { result.assume_init() })
        } else {
            Err(error)
        }
    }
}

impl Default for GDExtensionVariant {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GDExtensionVariant {
    fn clone(&self) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.interface.variant_new_copy)(&mut result, self);
            result.assume_init()
        }
    }
}

impl Drop for GDExtensionVariant {
    fn drop(&mut self) {
        unsafe {
            (API.interface.variant_destroy)(self);
        }
    }
}