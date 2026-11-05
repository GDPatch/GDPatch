use crate::gdextension::sys::{GDExtensionClassLibraryPtr, GDExtensionConstTypePtr, GDExtensionInterface, GDExtensionInterfaceUnknown, GDExtensionPtrBuiltInMethod, GDExtensionPtrConstructor, GDExtensionPtrDestructor, GDExtensionTypeFromVariantConstructorFunc, GDExtensionVariantFromTypeConstructorFunc, GDExtensionVariantType, PackedArray};
use std::ops::Deref;
use std::sync::OnceLock;

pub(crate) struct GDExtensionApiHolder(OnceLock<GDExtensionApi>);
pub(crate) static API: GDExtensionApiHolder = GDExtensionApiHolder(OnceLock::new());

impl Deref for GDExtensionApiHolder {
    type Target = GDExtensionApi;

    fn deref(&self) -> &Self::Target {
        self.0.get().expect("API should have been initialized")
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GDExtensionApi {
    pub library: GDExtensionClassLibraryPtr,
    pub interface: GDExtensionInterface,

    pub construct_string_name: GDExtensionPtrConstructor,
    pub construct_string_name_from_string: GDExtensionPtrConstructor,
    pub construct_string_name_from_variant: GDExtensionTypeFromVariantConstructorFunc,
    pub construct_variant_from_string_name: GDExtensionVariantFromTypeConstructorFunc,
    pub destruct_string_name: GDExtensionPtrDestructor,

    pub construct_string: GDExtensionPtrConstructor,
    pub construct_string_from_string_name: GDExtensionPtrConstructor,
    pub construct_string_from_variant: GDExtensionTypeFromVariantConstructorFunc,
    pub construct_variant_from_string: GDExtensionVariantFromTypeConstructorFunc,
    pub destruct_string: GDExtensionPtrDestructor,

    pub construct_packed_byte_array: GDExtensionPtrConstructor,
    pub destruct_packed_byte_array: GDExtensionPtrDestructor,
    pub construct_packed_byte_array_from_variant: GDExtensionTypeFromVariantConstructorFunc,
    pub construct_variant_from_packed_byte_array: GDExtensionVariantFromTypeConstructorFunc,
    pub resize_packed_byte_array: GDExtensionPtrBuiltInMethod,
    pub packed_byte_array_size: GDExtensionPtrBuiltInMethod,
}

unsafe impl Send for GDExtensionApi {}
unsafe impl Sync for GDExtensionApi {}

// https://github.com/godotengine/godot/blob/4.0-stable/core/variant/variant_construct.cpp#L58
const CONSTRUCT_STRING_NAME_IDX: i32 = 0;
const CONSTRUCT_STRING_NAME_FROM_STRING_IDX: i32 = 2;

const CONSTRUCT_STRING_IDX: i32 = 0;
const CONSTRUCT_STRING_FROM_STRING_NAME_IDX: i32 = 2;

impl GDExtensionApi {
    pub fn try_instance() -> Option<&'static GDExtensionApi> {
        API.0.get()
    }

    unsafe fn resolve(
        interface: GDExtensionInterface,
        library: GDExtensionClassLibraryPtr,
    ) -> Self {
        let construct_string_name = unsafe {
            (interface.variant_get_ptr_constructor)(
                GDExtensionVariantType::StringName,
                CONSTRUCT_STRING_NAME_IDX,
            )
            .expect("missing constructor")
        };

        let construct_string_name_from_string = unsafe {
            (interface.variant_get_ptr_constructor)(
                GDExtensionVariantType::StringName,
                CONSTRUCT_STRING_NAME_FROM_STRING_IDX,
            )
            .expect("missing constructor")
        };

        let construct_string_name_from_variant = unsafe {
            (interface.get_variant_to_type_constructor)(GDExtensionVariantType::StringName)
                .expect("missing variant to type converter")
        };

        let construct_variant_from_string_name = unsafe {
            (interface.get_variant_from_type_constructor)(GDExtensionVariantType::StringName)
                .expect("missing type to variant converter")
        };

        let destruct_string_name = unsafe {
            (interface.variant_get_ptr_destructor)(GDExtensionVariantType::StringName)
                .expect("missing destructor")
        };

        let construct_string = unsafe {
            (interface.variant_get_ptr_constructor)(
                GDExtensionVariantType::String,
                CONSTRUCT_STRING_IDX,
            )
            .expect("missing constructor")
        };

        let construct_string_from_string_name = unsafe {
            (interface.variant_get_ptr_constructor)(
                GDExtensionVariantType::String,
                CONSTRUCT_STRING_FROM_STRING_NAME_IDX,
            )
            .expect("missing constructor")
        };

        let construct_string_from_variant = unsafe {
            (interface.get_variant_to_type_constructor)(GDExtensionVariantType::String)
                .expect("missing variant to type converter")
        };

        let construct_variant_from_string = unsafe {
            (interface.get_variant_from_type_constructor)(GDExtensionVariantType::String)
                .expect("missing type to variant converter")
        };

        let destruct_string = unsafe {
            (interface.variant_get_ptr_destructor)(GDExtensionVariantType::String)
                .expect("missing destructor")
        };

        let construct_packed_byte_array = unsafe {
            (interface.variant_get_ptr_constructor)(
                GDExtensionVariantType::PackedByteArray,
                0,
            )
            .expect("missing constructor")
        };

        let destruct_packed_byte_array = unsafe {
            (interface.variant_get_ptr_destructor)(GDExtensionVariantType::PackedByteArray)
                .expect("missing destructor")
        };

        let construct_packed_byte_array_from_variant = unsafe {
            (interface.get_variant_to_type_constructor)(GDExtensionVariantType::PackedByteArray)
                .expect("missing variant to type constructor")
        };

        let construct_variant_from_packed_byte_array = unsafe {
            (interface.get_variant_from_type_constructor)(GDExtensionVariantType::PackedByteArray)
                .expect("missing variant from type constructor")
        };

        let resize_packed_byte_array = unsafe {
            interface.lookup_built_in_method::<PackedArray<u8>>(
                false,
                false,
                false,
                "resize",
                None,
                &[GDExtensionVariantType::Int]
            )
                .expect("missing PackedByteArray::resize")
        };

        let packed_byte_array_size = unsafe {
            interface.lookup_built_in_method::<PackedArray<u8>>(
                false,
                false,
                false,
                "size",
                None,
                &[]
            )
                .expect("missing PackedByteArray::resize")
        };

        Self {
            library,
            interface,

            construct_string_name,
            construct_string_name_from_string,
            construct_string_name_from_variant,
            construct_variant_from_string_name,
            destruct_string_name,

            construct_string,
            construct_string_from_string_name,
            construct_string_from_variant,
            construct_variant_from_string,
            destruct_string,

            construct_packed_byte_array,
            destruct_packed_byte_array,
            construct_packed_byte_array_from_variant,
            construct_variant_from_packed_byte_array,
            resize_packed_byte_array,
            packed_byte_array_size,
        }
    }
}

pub unsafe fn setup_api(
    unknown_interface: GDExtensionInterfaceUnknown,
    library: GDExtensionClassLibraryPtr,
) {
    let interface = unsafe { unknown_interface.resolve() };
    let api = unsafe { GDExtensionApi::resolve(interface, library) };

    API.0.set(api).expect("API should only be initialized once")
}
