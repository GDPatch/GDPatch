//! Macro runtime, not public API
pub use crate::gdextension::PropertyConvertInternal;
pub use crate::gdextension::api::*;
pub use crate::gdextension::types::*;
pub use crate::gdextension::variant::*;
use std::ffi::c_void;
use std::str::FromStr;

pub unsafe extern "C" fn create_instance<T: Default>(
    _userdata: *mut c_void,
) -> *mut GDExtensionObject {
    let api = unsafe { get_api() };

    let parent_class_name = GDExtensionString::from_str("Node").unwrap();
    let parent_class_name = GDExtensionStringName::from(&parent_class_name);
    let parent = unsafe { (api.classdb_construct_object)(&raw const parent_class_name) };

    let instance = T::default();
    let boxed = Box::new(instance);
    let ptr = Box::into_raw(boxed) as *mut GDExtensionClassInstance;

    // FIXME
    let class_name = GDExtensionString::from_str("GDPatchPack").unwrap();
    let class_name = GDExtensionStringName::from(&class_name);
    unsafe { (api.object_set_instance)(parent, &raw const class_name, ptr) };

    let callbacks = GDExtensionInstanceBindingCallbacks {
        create_callback: std::ptr::null(),
        free_callback: std::ptr::null(),
        reference_callback: std::ptr::null(),
    };
    unsafe { (api.object_set_instance_binding)(parent, api.library, ptr, &raw const callbacks) };

    parent
}

pub unsafe extern "C" fn free_instance<T>(
    _userdata: *mut c_void,
    instance: *mut GDExtensionClassInstance,
) {
    let r#box = unsafe { Box::from_raw(instance as *mut T) };
    drop(r#box);
}
