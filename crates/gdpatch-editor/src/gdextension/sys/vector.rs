use crate::gdextension::api::API;
use crate::gdextension::sys::{
    GDExtensionConstTypePtr, GDExtensionInt, GDExtensionType, GDExtensionTypePtr,
    GDExtensionUninitializedTypePtr, GDExtensionVariant, GDExtensionVariantType,
};
use std::ffi::c_int;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::{ptr, slice};

#[repr(C, align(8))]
#[derive(Debug)]
pub struct PackedArray<T> {
    inner: [MaybeUninit<u8>; size_of::<*const ()>() * 2],
    phantom: PhantomData<T>,
}

const _: () = {
    if size_of::<PackedArray<u8>>() != size_of::<*const ()>() * 2 {
        panic!("PackedArray size is wrong");
    }
};

impl PackedArray<u8> {
    /// Returns the length of this array.
    pub fn len(&self) -> usize {
        let built_in_methods = API.built_in_methods();
        let mut length: GDExtensionInt = 0;

        unsafe {
            (built_in_methods.packed_byte_array_size)(
                &raw const *self as GDExtensionTypePtr,
                ptr::null(),
                &raw mut length as GDExtensionUninitializedTypePtr,
                0,
            )
        };
        length as usize
    }

    /// Returns whether this array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn resize(&mut self, new_size: usize) {
        let built_in_methods = API.built_in_methods();

        let new_size = new_size as GDExtensionInt;
        let arguments = [&raw const new_size as GDExtensionConstTypePtr];
        let mut error: GDExtensionInt = 0;

        unsafe {
            // TODO: error handling lmfao
            (built_in_methods.resize_packed_byte_array)(
                &raw mut *self as GDExtensionTypePtr,
                arguments.as_ptr(),
                &raw mut error as GDExtensionUninitializedTypePtr,
                arguments.len() as c_int,
            )
        };
    }

    pub fn as_ptr(&self) -> *const u8 {
        let ptr = unsafe {
            (API.interface.packed_byte_array_operator_index_const)(
                &raw const *self as GDExtensionTypePtr,
                0,
            )
        };
        ptr.cast::<u8>()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        let ptr = unsafe {
            (API.interface.packed_byte_array_operator_index)(
                &raw mut *self as GDExtensionTypePtr,
                0,
            )
        };
        ptr.cast::<u8>()
    }
}

impl Deref for PackedArray<u8> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let length = self.len();

        if length == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.as_ptr(), length) }
        }
    }
}

impl DerefMut for PackedArray<u8> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let length = self.len();

        if length == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), length) }
        }
    }
}

impl From<Vec<u8>> for PackedArray<u8> {
    fn from(value: Vec<u8>) -> Self {
        let mut array = Self::new();
        array.resize(value.len());
        array.copy_from_slice(&value);
        array
    }
}

impl GDExtensionType for PackedArray<u8> {
    const VARIANT_TYPE: GDExtensionVariantType = GDExtensionVariantType::PackedByteArray;

    fn new() -> Self {
        let mut result = MaybeUninit::zeroed();

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

impl<T> Drop for PackedArray<T> {
    // TODO
    fn drop(&mut self) {
        unsafe {
            (API.destruct_packed_byte_array)(&raw mut *self as GDExtensionTypePtr);
        }
    }
}
