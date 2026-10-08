//! GDExtension types, mostly hand-ported from `gdextension_interface.h`.
#![allow(dead_code)]
use std::{
    ffi::{c_char, c_void},
    fmt::Debug,
};

use crate::gdextension::variant::{GDExtensionString, GDExtensionStringName};

/// The GDExtensionInterface struct used in Godot 4.0.
///
/// [https://github.com/godotengine/godot/blob/4.0-stable/core/extension/gdextension_interface.h#L405-L609]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionInterface {
    pub version_major: u32,
    pub version_minor: u32,
    pub version_patch: u32,
    pub version_string: *const c_char,

    pub mem_alloc: GDExtensionInterfaceMemAlloc,
    pub mem_realloc: GDExtensionInterfaceMemRealloc,
    pub mem_free: GDExtensionInterfaceMemFree,

    pub print_error: *const c_void,
    pub print_error_with_message: *const c_void,
    pub print_warning: *const c_void,
    pub print_warning_with_message: *const c_void,
    pub print_script_error: *const c_void,
    pub print_script_error_with_message: *const c_void,

    pub get_native_struct_size: *const c_void,

    pub variant_new_copy: *const c_void,
    pub variant_new_nil: *const c_void,
    pub variant_destroy: *const c_void,
    pub variant_call: *const c_void,
    pub variant_call_static: *const c_void,
    pub variant_evaluate: *const c_void,
    pub variant_set: *const c_void,
    pub variant_set_named: *const c_void,
    pub variant_set_keyed: *const c_void,
    pub variant_set_indexed: *const c_void,
    pub variant_get: *const c_void,
    pub variant_get_named: *const c_void,
    pub variant_get_keyed: *const c_void,
    pub variant_get_indexed: *const c_void,
    pub variant_iter_init: *const c_void,
    pub variant_iter_next: *const c_void,
    pub variant_iter_get: *const c_void,
    pub variant_hash: *const c_void,
    pub variant_recursive_hash: *const c_void,
    pub variant_hash_compare: *const c_void,
    pub variant_booleanize: *const c_void,
    pub variant_duplicate: *const c_void,
    pub variant_stringify: *const c_void,

    pub variant_get_type: GDExtensionInterfaceVariantGetType,
    pub variant_has_method: *const c_void,
    pub variant_has_member: *const c_void,
    pub variant_has_key: *const c_void,
    pub variant_get_type_name: *const c_void,
    pub variant_can_convert: *const c_void,
    pub variant_can_convert_strict: *const c_void,

    pub get_variant_from_type_constructor: *const c_void,
    pub get_variant_to_type_constructor: GDExtensionInterfaceGetVariantToTypeConstructor,
    pub variant_get_ptr_operator_evaluator: *const c_void,
    pub variant_get_ptr_builtin_method: *const c_void,
    pub variant_get_ptr_constructor: GDExtensionInterfaceVariantGetPtrConstructor,
    pub variant_get_ptr_destructor: GDExtensionInterfaceVariantGetPtrDestructor,
    pub variant_construct: *const c_void,

    pub variant_get_ptr_setter: *const c_void,
    pub variant_get_ptr_getter: *const c_void,
    pub variant_get_ptr_indexed_setter: *const c_void,
    pub variant_get_ptr_indexed_getter: *const c_void,
    pub variant_get_ptr_keyed_setter: *const c_void,
    pub variant_get_ptr_keyed_getter: *const c_void,
    pub variant_get_ptr_keyed_checker: *const c_void,
    pub variant_get_constant_value: *const c_void,
    pub variant_get_ptr_utility_function: *const c_void,

    pub string_new_with_latin1_chars: *const c_void,
    pub string_new_with_utf8_chars: *const c_void,
    pub string_new_with_utf16_chars: *const c_void,
    pub string_new_with_utf32_chars: *const c_void,
    pub string_new_with_wide_chars: *const c_void,
    pub string_new_with_latin1_chars_and_len: *const c_void,
    pub string_new_with_utf8_chars_and_len: GDExtensionInterfaceStringNewWithUtf8CharsAndLen,
    pub string_new_with_utf16_chars_and_len: *const c_void,
    pub string_new_with_utf32_chars_and_len: *const c_void,
    pub string_new_with_wide_chars_and_len: *const c_void,

    pub string_to_latin1_chars: *const c_void,
    pub string_to_utf8_chars: GDExtensionInterfaceStringToUtf8Chars,
    pub string_to_utf16_chars: *const c_void,
    pub string_to_utf32_chars: *const c_void,
    pub string_to_wide_chars: *const c_void,
    pub string_operator_index: *const c_void,
    pub string_operator_index_const: *const c_void,

    pub string_operator_plus_eq_string: *const c_void,
    pub string_operator_plus_eq_char: *const c_void,
    pub string_operator_plus_eq_cstr: *const c_void,
    pub string_operator_plus_eq_wcstr: *const c_void,
    pub string_operator_plus_eq_c32str: *const c_void,

    pub xml_parser_open_buffer: *const c_void,

    pub file_access_store_buffer: *const c_void,
    pub file_access_get_buffer: *const c_void,

    pub worker_thread_pool_add_native_group_task: *const c_void,
    pub worker_thread_pool_add_native_task: *const c_void,

    pub packed_byte_array_operator_index: *const c_void,
    pub packed_byte_array_operator_index_const: *const c_void,

    pub packed_color_array_operator_index: *const c_void,
    pub packed_color_array_operator_index_const: *const c_void,

    pub packed_float32_array_operator_index: *const c_void,
    pub packed_float32_array_operator_index_const: *const c_void,
    pub packed_float64_array_operator_index: *const c_void,
    pub packed_float64_array_operator_index_const: *const c_void,

    pub packed_int32_array_operator_index: *const c_void,
    pub packed_int32_array_operator_index_const: *const c_void,
    pub packed_int64_array_operator_index: *const c_void,
    pub packed_int64_array_operator_index_const: *const c_void,

    pub packed_string_array_operator_index: *const c_void,
    pub packed_string_array_operator_index_const: *const c_void,

    pub packed_vector2_array_operator_index: *const c_void,
    pub packed_vector2_array_operator_index_const: *const c_void,
    pub packed_vector3_array_operator_index: *const c_void,
    pub packed_vector3_array_operator_index_const: *const c_void,

    pub array_operator_index: *const c_void,
    pub array_operator_index_const: *const c_void,
    pub array_ref: *const c_void,
    pub array_set_typed: *const c_void,

    pub dictionary_operator_index: *const c_void,
    pub dictionary_operator_index_const: *const c_void,

    pub object_method_bind_call: *const c_void,
    pub object_method_bind_ptrcall: *const c_void,
    pub object_destroy: *const c_void,
    pub global_get_singleton: *const c_void,

    pub object_get_instance_binding: *const c_void,
    pub object_set_instance_binding: GDExtensionInterfaceObjectSetInstanceBinding,

    pub object_set_instance: GDExtensionInterfaceObjectSetInstance,

    pub object_cast_to: *const c_void,
    pub object_get_instance_from_id: *const c_void,
    pub object_get_instance_id: *const c_void,

    pub ref_get_object: *const c_void,
    pub ref_set_object: *const c_void,

    pub script_instance_create: *const c_void,

    pub classdb_construct_object: GDExtensionInterfaceClassDBConstructObject,
    pub classdb_get_method_bind: *const c_void,
    pub classdb_get_class_tag: *const c_void,

    pub classdb_register_extension_class: GDExtensionInterfaceClassDBRegisterExtensionClass,
    pub classdb_register_extension_class_method:
        GDExtensionInterfaceClassDBRegisterExtensionClassMethod,
    pub classdb_register_extension_class_integer_constant: *const c_void,
    pub classdb_register_extension_class_property: *const c_void,
    pub classdb_register_extension_class_property_group: *const c_void,
    pub classdb_register_extension_class_property_subgroup: *const c_void,
    pub classdb_register_extension_class_signal: *const c_void,
    pub classdb_unregister_extension_class: *const c_void,

    pub get_library_path: *const c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionInitialization {
    pub minimum_initialization_level: GDExtensionInitializationLevel,
    pub userdata: *mut c_void,
    pub initialize:
        Option<unsafe extern "C" fn(userdata: *mut c_void, level: GDExtensionInitializationLevel)>,
    pub deinitialize:
        Option<unsafe extern "C" fn(userdata: *mut c_void, level: GDExtensionInitializationLevel)>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassCreationInfo {
    pub is_virtual: bool,
    pub is_abstract: bool,
    pub set_func: *const c_void,
    pub get_func: *const c_void,
    pub get_property_list_func: *const c_void,
    pub free_property_list_func: *const c_void,
    pub property_can_revert_func: *const c_void,
    pub property_get_revert_func: *const c_void,
    pub notification_func: *const c_void,
    pub to_string_func: *const c_void,
    pub reference_func: *const c_void,
    pub unreference_func: *const c_void,
    pub create_instance_func: GDExtensionClassCreateInstance,
    pub free_instance_func: GDExtensionClassFreeInstance,
    pub get_virtual_func: *const c_void,
    pub get_rid_func: *const c_void,
    pub class_userdata: *mut c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassMethodInfo {
    pub name: *const GDExtensionStringName,
    pub method_userdata: *mut c_void,
    pub call_func: Option<GDExtensionClassMethodCall>,
    pub ptrcall_func: Option<GDExtensionClassMethodPtrCall>,
    pub method_flags: GDExtensionClassMethodFlags,

    pub has_return_value: bool,
    pub return_value_info: *const GDExtensionPropertyInfo,
    pub return_value_metadata: GDExtensionClassMethodArgumentMetadata,

    pub argument_count: u32,
    pub arguments_info: *const *const GDExtensionPropertyInfo,
    pub arguments_metadata: *const GDExtensionClassMethodArgumentMetadata,

    pub default_argument_count: u32,
    pub default_arguments: *const GDExtensionVariantPtr,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionPropertyInfo {
    pub r#type: GDExtensionVariantType,
    pub name: *const GDExtensionStringName,
    pub class_name: *const GDExtensionStringName,

    pub hint: GDExtensionPropertyHint,
    pub hint_string: *const GDExtensionString,
    pub usage: GDExtensionPropertyUsageFlags,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionCallError {
    pub error: GDExtensionCallErrorType,
    pub argument: i32,
    pub expected: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionInstanceBindingCallbacks {
    pub create_callback: *const c_void,
    pub free_callback: *const c_void,
    pub reference_callback: *const c_void,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionInitializationLevel {
    Core = 0,
    Servers = 1,
    Scene = 2,
    Editor = 3,
    Max = 4,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionVariantType {
    Nil,

    // atomic types
    Bool,
    Int,
    Float,
    String,

    // math types
    Vector2,
    Vector2i,
    Rect2,
    Rect2i,
    Vector3,
    Vector3i,
    Transform2d,
    Vector4,
    Vector4i,
    Plane,
    Quaternion,
    Aabb,
    Basis,
    Transform3d,
    Projection,

    // misc types
    Color,
    StringName,
    NodePath,
    Rid,
    Object,
    Callable,
    Signal,
    Dictionary,
    Array,

    // typed arrays
    PackedByteArray,
    PackedInt32Array,
    PackedInt64Array,
    PackedFloat32Array,
    PackedFloat64Array,
    PackedStringArray,
    PackedVector2Array,
    PackedVector3Array,
    PackedColorArray,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionClassMethodFlags {
    Normal = 1,
    Editor = 2,
    Const = 4,
    Virtual = 8,
    Vararg = 16,
    Static = 32,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionCallErrorType {
    OK,
    InvalidMethod,
    InvalidArgument,
    TooManyArguments,
    TooFewArguments,
    InstanceIsNull,
    MethodNotConst,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionClassMethodArgumentMetadata {
    None,
    IntIsInt8,
    IntIsInt16,
    IntIsInt32,
    IntIsInt64,
    IntIsUint8,
    IntIsUint16,
    IntIsUint32,
    IntIsUint64,
    RealIsFloat,
    RealIsDouble,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionPropertyHint {
    None = 0,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GDExtensionPropertyUsageFlags {
    None = 0,
}

/// Either a [`*const GDExtensionInterface`] or [`GDExtensionInterfaceGetProcAddress`].
///
/// [`*const GDExtensionInterface`]: GDExtensionInterface
#[repr(C)]
#[derive(Copy, Clone)]
pub union GDExtensionInterfaceUnknown {
    pub direct_interface: *const GDExtensionInterface,
    pub get_proc_address: GDExtensionInterfaceGetProcAddress,
}

impl Debug for GDExtensionInterfaceUnknown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: Both union variants are pointers.
        write!(f, "GDExtensionInterfaceUnknown({:p})", unsafe {
            self.direct_interface
        })
    }
}

const _: () = {
    if size_of::<GDExtensionInterfaceUnknown>() != size_of::<*const ()>() {
        panic!("GDExtensionInterfaceUnknown size is wrong");
    }

    if align_of::<GDExtensionInterfaceUnknown>() != align_of::<*const ()>() {
        panic!("GDExtensionInterfaceUnknown alignment is wrong");
    }
};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GDExtensionClassLibraryPtr(*const c_void);

#[repr(C)]
#[derive(Debug)]
pub struct GDExtensionObject(c_void);

#[repr(C)]
#[derive(Debug)]
pub struct GDExtensionClassInstance(c_void);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GDExtensionTypePtr(pub *const c_void);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GDExtensionVariantPtr(*const c_void);

pub type GDExtensionInterfaceFunctionPtr = unsafe extern "C" fn();
pub type GDExtensionInterfaceGetProcAddress =
    unsafe extern "C" fn(function_name: *const c_char) -> GDExtensionInterfaceFunctionPtr;

pub type GDExtensionClassCreateInstance =
    unsafe extern "C" fn(userdata: *mut c_void) -> *mut GDExtensionObject;
pub type GDExtensionClassFreeInstance =
    unsafe extern "C" fn(userdata: *mut c_void, instance: *mut GDExtensionClassInstance);

pub type GDExtensionInterfaceMemAlloc = unsafe extern "C" fn(size: usize) -> *mut c_void;
pub type GDExtensionInterfaceMemRealloc =
    unsafe extern "C" fn(ptr: *mut c_void, size: usize) -> *mut c_void;
pub type GDExtensionInterfaceMemFree = unsafe extern "C" fn(ptr: *mut c_void);

pub type GDExtensionInterfaceVariantGetType =
    unsafe extern "C" fn(instance: GDExtensionVariantPtr) -> GDExtensionVariantType;

pub type GDExtensionTypeFromVariantConstructor =
    unsafe extern "C" fn(base: GDExtensionTypePtr, variant: GDExtensionVariantPtr);
pub type GDExtensionInterfaceGetVariantToTypeConstructor =
    unsafe extern "C" fn(r#type: GDExtensionVariantType) -> GDExtensionTypeFromVariantConstructor;

pub type GDExtensionPtrConstructor =
    unsafe extern "C" fn(base: GDExtensionTypePtr, args: *const GDExtensionTypePtr);
pub type GDExtensionInterfaceVariantGetPtrConstructor =
    unsafe extern "C" fn(
        r#type: GDExtensionVariantType,
        constructor: i32,
    ) -> Option<GDExtensionPtrConstructor>;

pub type GDExtensionPtrDestructor = unsafe extern "C" fn(base: GDExtensionTypePtr);
pub type GDExtensionInterfaceVariantGetPtrDestructor =
    unsafe extern "C" fn(r#type: GDExtensionVariantType) -> Option<GDExtensionPtrDestructor>;

pub type GDExtensionInterfaceStringNewWithUtf8CharsAndLen =
    unsafe extern "C" fn(ptr: *mut GDExtensionString, contents: *const c_char, size: i64);

pub type GDExtensionInterfaceStringToUtf8Chars = unsafe extern "C" fn(
    ptr: *const GDExtensionString,
    text: *mut c_char,
    max_write_length: i64,
) -> i64;

pub type GDExtensionInterfaceObjectSetInstanceBinding = unsafe extern "C" fn(
    object: *mut GDExtensionObject,
    library: GDExtensionClassLibraryPtr,
    instance: *const GDExtensionClassInstance,
    callbacks: *const GDExtensionInstanceBindingCallbacks,
);

pub type GDExtensionInterfaceObjectSetInstance = unsafe extern "C" fn(
    object: *mut GDExtensionObject,
    class_name: *const GDExtensionStringName,
    instance: *const GDExtensionClassInstance,
);

pub type GDExtensionInterfaceClassDBConstructObject =
    unsafe extern "C" fn(class_name: *const GDExtensionStringName) -> *mut GDExtensionObject;

pub type GDExtensionInterfaceClassDBRegisterExtensionClass = unsafe extern "C" fn(
    library: GDExtensionClassLibraryPtr,
    class_name: *const GDExtensionStringName,
    parent_class_name: *const GDExtensionStringName,
    extension_funcs: *const GDExtensionClassCreationInfo,
) -> *const c_void;

pub type GDExtensionInterfaceClassDBRegisterExtensionClassMethod = unsafe extern "C" fn(
    library: GDExtensionClassLibraryPtr,
    class_name: *const GDExtensionStringName,
    method_info: *const GDExtensionClassMethodInfo,
);

pub type GDExtensionClassMethodCall = unsafe extern "C" fn(
    method_userdata: *mut c_void,
    instance: *const GDExtensionClassInstance,
    args: *const GDExtensionVariantPtr,
    argument_count: i64,
    ret: GDExtensionVariantPtr,
    error: *mut GDExtensionCallError,
);

pub type GDExtensionClassMethodPtrCall = unsafe extern "C" fn(
    method_userdata: *mut c_void,
    instance: *const GDExtensionClassInstance,
    args: *const GDExtensionTypePtr,
    ret: GDExtensionTypePtr,
);
