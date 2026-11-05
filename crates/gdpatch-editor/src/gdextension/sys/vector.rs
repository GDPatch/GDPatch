use std::marker::PhantomData;
use crate::gdextension::api::API;
use crate::gdextension::sys::{
    GDExtensionConstTypePtr, GDExtensionType, GDExtensionUninitializedTypePtr
    , GDExtensionVariant, GDExtensionVariantType,
};
use std::mem::MaybeUninit;
use std::ptr;

#[repr(C)]
#[derive(Debug)]
pub struct PackedArray<T> {
    ptr: *const (),
    phantom: PhantomData<T>,
}

impl GDExtensionType for PackedArray<u8> {
    const VARIANT_TYPE: GDExtensionVariantType = GDExtensionVariantType::PackedByteArray;

    fn new() -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_packed_byte_array)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                ptr::null(),
            );

            result.assume_init()
        }
    }

    fn from_variant(variant: &GDExtensionVariant) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_packed_byte_array_from_variant)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                variant,
            );

            result.assume_init()
        }
    }

    fn to_variant(&self) -> GDExtensionVariant {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_variant_from_packed_byte_array)(
                &mut result,
                &raw const *self as GDExtensionConstTypePtr,
            );

            result.assume_init()
        }
    }
}
