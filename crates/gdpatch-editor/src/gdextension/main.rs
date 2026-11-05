//! GDExtension entrypoint and global state.

use crate::gdextension::api::{setup_api, API};
use crate::gdextension::sys::{GDExtensionClassLibraryPtr, GDExtensionInitialization, GDExtensionInitializationLevel, GDExtensionInterfaceUnknown, GDExtensionString, GDExtensionStringName};
use std::ffi::c_void;

// #[global_allocator]
// pub static ALLOCATOR: GDExtensionAllocator = GDExtensionAllocator;

#[unsafe(no_mangle)]
unsafe extern "C" fn gdpatch_editor_init(
    unknown_interface: GDExtensionInterfaceUnknown,
    library: GDExtensionClassLibraryPtr,
    initialization: *mut GDExtensionInitialization,
) -> bool {
    unsafe {
        setup_api(unknown_interface, library);
    }

    let initialization = unsafe { initialization.as_mut_unchecked() };
    initialization.minimum_initialization_level = GDExtensionInitializationLevel::Scene;
    initialization.userdata = std::ptr::null_mut();
    initialization.initialize = Some(initialize);
    initialization.deinitialize = Some(deinitialize);

    true
}

unsafe extern "C" fn initialize(_userdata: *mut c_void, level: GDExtensionInitializationLevel) {
    if level != GDExtensionInitializationLevel::Scene {
        return;
    }

    let s = GDExtensionString::from("meow");
    println!("hi: {s}");
    drop(s);
}

unsafe extern "C" fn deinitialize(_userdata: *mut c_void, _level: GDExtensionInitializationLevel) {
    println!("bye meow");
}