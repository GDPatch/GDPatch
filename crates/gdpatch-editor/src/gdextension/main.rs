//! GDExtension entrypoint and global state.

use crate::GDPatchPack;
use crate::gdextension::GDClass;
use crate::gdextension::api::setup_api;
use crate::gdextension::sys::{
    GDExtensionClassLibraryPtr, GDExtensionInitialization, GDExtensionInitializationLevel,
    GDExtensionInterfaceUnknown,
};
use std::ffi::c_void;
use std::mem;
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

    let mut class = GDClass::<GDPatchPack>::new("GDPatchPack");
    class.add_function(GDPatchPack::set_pack);
    mem::forget(class); // TODO: drop unregisters
}

unsafe extern "C" fn deinitialize(_userdata: *mut c_void, level: GDExtensionInitializationLevel) {
    if level == GDExtensionInitializationLevel::Scene {
        println!("bye meow");
    }
}
