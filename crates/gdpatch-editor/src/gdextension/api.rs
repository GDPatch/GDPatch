#![allow(clippy::missing_transmute_annotations)]

use crate::gdextension::types::*;
use std::{ffi::CString, str::FromStr, sync::OnceLock};

static API: OnceLock<GDExtensionAPI> = OnceLock::new();

#[derive(Debug, Copy, Clone)]
pub struct GDExtensionAPI {
    pub library: GDExtensionClassLibraryPtr,

    pub variant_get_type: GDExtensionInterfaceVariantGetType,

    pub string_new_with_utf8_chars_and_len: GDExtensionInterfaceStringNewWithUtf8CharsAndLen,

    pub string_to_utf8_chars: GDExtensionInterfaceStringToUtf8Chars,

    pub object_set_instance_binding: GDExtensionInterfaceObjectSetInstanceBinding,

    pub object_set_instance: GDExtensionInterfaceObjectSetInstance,

    pub classdb_construct_object: GDExtensionInterfaceClassDBConstructObject,

    pub classdb_register_extension_class: GDExtensionInterfaceClassDBRegisterExtensionClass,
    pub classdb_register_extension_class_method:
        GDExtensionInterfaceClassDBRegisterExtensionClassMethod,

    pub constructor_string_name: GDExtensionPtrConstructor,
    pub constructor_string_from_variant: GDExtensionTypeFromVariantConstructor,

    pub destructor_string: GDExtensionPtrDestructor,
    pub destructor_string_name: GDExtensionPtrDestructor,
}

unsafe impl Send for GDExtensionAPI {}
unsafe impl Sync for GDExtensionAPI {}

impl GDExtensionAPI {
    unsafe fn resolve(
        interface: GDExtensionInterfaceUnknown,
        library: GDExtensionClassLibraryPtr,
    ) -> Self {
        // Godot 4.0's interface is a direct pointer to GDExtensionInterface,
        // but 4.1 and onwards use GDExtensionInterfaceGetProcAddress. Since
        // GDExtensionInterface starts with a version number, we can dereference
        // this pointer and check if it equals the major version (4) to determine
        // what engine version we're running on. Yes, this hack is as stupid as it
        // sounds. In theory, GDExtensionInterfaceGetProcAddress could also start
        // with a `4` but it doesn't make sense as an instruction to start a function
        // on any major ISA.

        let maybe_major_version = unsafe { *interface.direct_interface.cast::<u32>() };
        let is_4_0 = maybe_major_version == 4;

        macro_rules! resolve {
            ($func:ident) => {
                if is_4_0 {
                    unsafe {
                        let interface = interface.direct_interface.as_ref_unchecked();
                        interface.$func
                    }
                } else {
                    unsafe {
                        let name = CString::from_str(stringify!($func)).unwrap();
                        let res = (interface.get_proc_address)(name.as_ptr());

                        std::mem::transmute::<_, _>(res)
                    }
                }
            };
        }

        macro_rules! resolve_multiple {
            ({ $($func:ident),*$(,)? }, {$($body:tt),*$(,)? }) => {
                Self {
                    $(
                        $func: resolve!($func),
                    )*
                    $(
                        $body,
                    )*
                }
            }
        }

        let variant_get_ptr_constructor = resolve!(variant_get_ptr_constructor);
        let variant_get_ptr_destructor = resolve!(variant_get_ptr_destructor);
        let get_variant_to_type_constructor = resolve!(get_variant_to_type_constructor);

        let constructor_string_name = unsafe {
            (variant_get_ptr_constructor)(GDExtensionVariantType::StringName, 2)
                .expect("StringName constructor should exist")
        };
        let constructor_string_from_variant =
            unsafe { (get_variant_to_type_constructor)(GDExtensionVariantType::String) };
        let destructor_string = unsafe {
            (variant_get_ptr_destructor)(GDExtensionVariantType::String)
                .expect("String destructor should exist")
        };
        let destructor_string_name = unsafe {
            (variant_get_ptr_destructor)(GDExtensionVariantType::StringName)
                .expect("StringName destructor should exist")
        };

        resolve_multiple!({
            variant_get_type,

            string_new_with_utf8_chars_and_len,

            string_to_utf8_chars,

            object_set_instance_binding,

            object_set_instance,

            classdb_construct_object,

            classdb_register_extension_class,
            classdb_register_extension_class_method
        }, {
            library,

            constructor_string_name,
            constructor_string_from_variant,
            destructor_string,
            destructor_string_name
        })
    }
}

pub unsafe fn setup_api(
    interface: GDExtensionInterfaceUnknown,
    library: GDExtensionClassLibraryPtr,
) {
    let api = unsafe { GDExtensionAPI::resolve(interface, library) };

    API.set(api).expect("API should only be initialized once")
}

pub unsafe fn get_api() -> &'static GDExtensionAPI {
    API.get().expect("API should have been initialized")
}
