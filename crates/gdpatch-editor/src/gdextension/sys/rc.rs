use crate::gdextension::{
    api::API,
    sys::{GDExtensionBool, GDExtensionObjectPtr, GDExtensionTypePtr},
};
use std::{marker::PhantomData, ptr};

pub struct GDExtensionRc<T> {
    inner: GDExtensionObjectPtr,
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for GDExtensionRc<T> {}

impl<T> GDExtensionRc<T> {
    // TODO: enforce that this class extends RefCounted first
    pub unsafe fn new(inner: GDExtensionObjectPtr) -> Self {
        let mut success: GDExtensionBool = Default::default();

        unsafe {
            (API.interface.object_method_bind_ptrcall)(
                API.built_in_methods().refcounted_init_ref,
                inner,
                ptr::null(),
                &raw mut success as GDExtensionTypePtr,
            )
        }

        if success == 0 {
            panic!("init_ref failed")
        }

        Self {
            inner,
            phantom: PhantomData,
        }
    }

    pub fn get(&self) -> &T {
        unsafe { (self.inner as *const T).as_ref_unchecked() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { (self.inner as *mut T).as_mut_unchecked() }
    }
}

impl<T> Drop for GDExtensionRc<T> {
    fn drop(&mut self) {
        let mut success: GDExtensionBool = Default::default();

        unsafe {
            (API.interface.object_method_bind_ptrcall)(
                API.built_in_methods().refcounted_unreference,
                self.inner,
                ptr::null(),
                &raw mut success as GDExtensionTypePtr,
            )
        }

        /*if success == 0 {
            panic!("unreference failed")
        }*/
    }
}
