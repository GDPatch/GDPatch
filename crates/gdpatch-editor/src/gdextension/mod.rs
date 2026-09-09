use crate::gdextension::{
    api::{get_api, setup_api},
    types::{
        GDExtensionClassLibraryPtr, GDExtensionClassMethodArgumentMetadata,
        GDExtensionInitialization, GDExtensionInitializationLevel, GDExtensionInterfaceUnknown,
        GDExtensionTypePtr, GDExtensionVariantPtr, GDExtensionVariantType,
    },
    variant::GDExtensionString,
};
use std::ffi::c_void;

pub mod api;
pub mod types;
pub mod variant;

unsafe extern "C" fn initialize(_userdata: *mut c_void, level: GDExtensionInitializationLevel) {
    if level != GDExtensionInitializationLevel::Scene {
        return;
    }

    let api = unsafe { get_api() };
    crate::GDPatchPack::__pissballs_register(api);
}

unsafe extern "C" fn deinitialize(_userdata: *mut c_void, _level: GDExtensionInitializationLevel) {
    println!("bye meow");
}

#[unsafe(export_name = "gdpatch_editor_init")]
unsafe extern "C" fn gdpatch_editor_init(
    interface: GDExtensionInterfaceUnknown,
    library: GDExtensionClassLibraryPtr,
    initialization: *mut GDExtensionInitialization,
) -> bool {
    unsafe {
        setup_api(interface, library);
    }

    let initialization = unsafe { initialization.as_mut_unchecked() };
    initialization.minimum_initialization_level = GDExtensionInitializationLevel::Scene;
    initialization.userdata = std::ptr::null_mut();
    initialization.initialize = Some(initialize);
    initialization.deinitialize = Some(deinitialize);

    true
}

pub trait PropertyConvertInternal
where
    Self: Sized,
{
    fn as_variant_type() -> GDExtensionVariantType;
    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata;

    unsafe fn convert_from_variant(variant: GDExtensionVariantPtr) -> Option<Self>;
    unsafe fn convert_to_variant(self, variant: GDExtensionVariantPtr);

    unsafe fn convert_from_extension_type(value: GDExtensionTypePtr) -> Self;
    unsafe fn convert_to_extension_type(self, value: GDExtensionTypePtr);
}

impl PropertyConvertInternal for String {
    fn as_variant_type() -> GDExtensionVariantType {
        GDExtensionVariantType::String
    }

    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata {
        GDExtensionClassMethodArgumentMetadata::None
    }

    unsafe fn convert_from_variant(variant: GDExtensionVariantPtr) -> Option<Self> {
        let api = unsafe { get_api() };
        let r#type = unsafe { (api.variant_get_type)(variant) };
        if r#type != GDExtensionVariantType::String {
            return None;
        }

        let mut string = GDExtensionString {
            ptr: std::ptr::null(),
        };
        unsafe {
            (api.constructor_string_from_variant)(
                GDExtensionTypePtr(&raw mut string as *const c_void),
                variant,
            )
        };

        Some(string.to_string())
    }

    unsafe fn convert_to_variant(self, _variant: GDExtensionVariantPtr) {
        todo!()
    }

    unsafe fn convert_from_extension_type(value: GDExtensionTypePtr) -> Self {
        unsafe { (*(value.0 as *const GDExtensionString)).to_string() }
    }

    unsafe fn convert_to_extension_type(self, _value: GDExtensionTypePtr) {
        todo!()
    }
}

impl PropertyConvertInternal for u64 {
    fn as_variant_type() -> GDExtensionVariantType {
        GDExtensionVariantType::Int
    }

    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata {
        GDExtensionClassMethodArgumentMetadata::IntIsUint64
    }

    unsafe fn convert_from_variant(_variant: GDExtensionVariantPtr) -> Option<Self> {
        todo!()
    }

    unsafe fn convert_to_variant(self, _variant: GDExtensionVariantPtr) {
        todo!()
    }

    unsafe fn convert_from_extension_type(_value: GDExtensionTypePtr) -> Self {
        todo!()
    }

    unsafe fn convert_to_extension_type(self, _value: GDExtensionTypePtr) {
        todo!()
    }
}

impl PropertyConvertInternal for bool {
    fn as_variant_type() -> GDExtensionVariantType {
        GDExtensionVariantType::Bool
    }

    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata {
        GDExtensionClassMethodArgumentMetadata::None
    }

    unsafe fn convert_from_variant(_variant: GDExtensionVariantPtr) -> Option<Self> {
        todo!()
    }

    unsafe fn convert_to_variant(self, _variant: GDExtensionVariantPtr) {
        todo!()
    }

    unsafe fn convert_from_extension_type(_value: GDExtensionTypePtr) -> Self {
        todo!()
    }

    unsafe fn convert_to_extension_type(self, value: GDExtensionTypePtr) {
        unsafe {
            *(value.0 as *mut bool) = self;
        }
    }
}

impl PropertyConvertInternal for Vec<u8> {
    fn as_variant_type() -> GDExtensionVariantType {
        GDExtensionVariantType::PackedByteArray
    }

    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata {
        GDExtensionClassMethodArgumentMetadata::None
    }

    unsafe fn convert_from_variant(_variant: GDExtensionVariantPtr) -> Option<Self> {
        todo!()
    }

    unsafe fn convert_to_variant(self, _variant: GDExtensionVariantPtr) {
        todo!()
    }

    unsafe fn convert_from_extension_type(_value: GDExtensionTypePtr) -> Self {
        todo!()
    }

    unsafe fn convert_to_extension_type(self, _value: GDExtensionTypePtr) {
        todo!()
    }
}

impl<T> PropertyConvertInternal for Option<T>
where
    T: PropertyConvertInternal,
{
    fn as_variant_type() -> GDExtensionVariantType {
        T::as_variant_type()
    }

    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata {
        T::as_argument_metadata()
    }

    unsafe fn convert_from_variant(variant: GDExtensionVariantPtr) -> Option<Self> {
        unsafe { Some(T::convert_from_variant(variant)) }
    }

    unsafe fn convert_to_variant(self, _variant: GDExtensionVariantPtr) {
        todo!()
    }

    unsafe fn convert_from_extension_type(value: GDExtensionTypePtr) -> Self {
        unsafe { Some(T::convert_from_extension_type(value)) }
    }

    unsafe fn convert_to_extension_type(self, _value: GDExtensionTypePtr) {
        todo!()
    }
}

impl PropertyConvertInternal for () {
    fn as_variant_type() -> GDExtensionVariantType {
        GDExtensionVariantType::Nil
    }

    fn as_argument_metadata() -> GDExtensionClassMethodArgumentMetadata {
        GDExtensionClassMethodArgumentMetadata::None
    }

    unsafe fn convert_from_variant(_variant: GDExtensionVariantPtr) -> Option<Self> {
        Some(())
    }

    unsafe fn convert_to_variant(self, _variant: GDExtensionVariantPtr) {}

    unsafe fn convert_from_extension_type(_value: GDExtensionTypePtr) -> Self {}

    unsafe fn convert_to_extension_type(self, _value: GDExtensionTypePtr) {}
}
