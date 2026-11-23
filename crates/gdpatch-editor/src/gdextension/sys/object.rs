use crate::gdextension::{
    api::API,
    sys::{
        GDExtensionType, GDExtensionUninitializedTypePtr, GDExtensionVariant,
        GDExtensionVariantType,
    },
};
use std::{ffi::c_void, mem::MaybeUninit};

pub struct GDExtensionObject {
    id: u64,
    obj: *const c_void,
}

impl GDExtensionType for GDExtensionObject {
    const VARIANT_TYPE: GDExtensionVariantType = GDExtensionVariantType::Object;

    fn new() -> Self {
        unimplemented!()
    }

    fn from_variant(variant: &GDExtensionVariant) -> Self {
        let mut result = MaybeUninit::uninit();

        unsafe {
            (API.construct_object_from_variant)(
                &raw mut result as GDExtensionUninitializedTypePtr,
                variant,
            );

            result.assume_init()
        }
    }

    fn to_variant(&self) -> GDExtensionVariant {
        todo!()
    }
}
