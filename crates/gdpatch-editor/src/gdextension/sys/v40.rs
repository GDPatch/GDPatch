//! Manual bindings for the old 4.0 GDExtension API where a struct is provided instead.

use crate::gdextension::sys::{GDExtensionGodotVersion, GDExtensionInt, GDExtensionInterfaceArrayOperatorIndex, GDExtensionInterfaceArrayOperatorIndexConst, GDExtensionInterfaceArrayRef, GDExtensionInterfaceArraySetTyped, GDExtensionInterfaceClassdbConstructObject, GDExtensionInterfaceClassdbGetClassTag, GDExtensionInterfaceClassdbGetMethodBind, GDExtensionInterfaceClassdbRegisterExtensionClass, GDExtensionInterfaceClassdbRegisterExtensionClassIntegerConstant, GDExtensionInterfaceClassdbRegisterExtensionClassMethod, GDExtensionInterfaceClassdbRegisterExtensionClassProperty, GDExtensionInterfaceClassdbRegisterExtensionClassPropertyGroup, GDExtensionInterfaceClassdbRegisterExtensionClassPropertySubgroup, GDExtensionInterfaceClassdbRegisterExtensionClassSignal, GDExtensionInterfaceClassdbUnregisterExtensionClass, GDExtensionInterfaceDictionaryOperatorIndex, GDExtensionInterfaceDictionaryOperatorIndexConst, GDExtensionInterfaceFileAccessGetBuffer, GDExtensionInterfaceFileAccessStoreBuffer, GDExtensionInterfaceFunctionPtr, GDExtensionInterfaceGetGodotVersion, GDExtensionInterfaceGetLibraryPath, GDExtensionInterfaceGetNativeStructSize, GDExtensionInterfaceGetProcAddress, GDExtensionInterfaceGetVariantFromTypeConstructor, GDExtensionInterfaceGetVariantToTypeConstructor, GDExtensionInterfaceGlobalGetSingleton, GDExtensionInterfaceMemAlloc, GDExtensionInterfaceMemFree, GDExtensionInterfaceMemRealloc, GDExtensionInterfaceObjectCastTo, GDExtensionInterfaceObjectDestroy, GDExtensionInterfaceObjectGetInstanceBinding, GDExtensionInterfaceObjectGetInstanceFromId, GDExtensionInterfaceObjectGetInstanceId, GDExtensionInterfaceObjectMethodBindCall, GDExtensionInterfaceObjectMethodBindPtrcall, GDExtensionInterfaceObjectSetInstance, GDExtensionInterfaceObjectSetInstanceBinding, GDExtensionInterfacePackedByteArrayOperatorIndex, GDExtensionInterfacePackedByteArrayOperatorIndexConst, GDExtensionInterfacePackedColorArrayOperatorIndex, GDExtensionInterfacePackedColorArrayOperatorIndexConst, GDExtensionInterfacePackedFloat32ArrayOperatorIndex, GDExtensionInterfacePackedFloat32ArrayOperatorIndexConst, GDExtensionInterfacePackedFloat64ArrayOperatorIndex, GDExtensionInterfacePackedFloat64ArrayOperatorIndexConst, GDExtensionInterfacePackedInt32ArrayOperatorIndex, GDExtensionInterfacePackedInt32ArrayOperatorIndexConst, GDExtensionInterfacePackedInt64ArrayOperatorIndex, GDExtensionInterfacePackedInt64ArrayOperatorIndexConst, GDExtensionInterfacePackedStringArrayOperatorIndex, GDExtensionInterfacePackedStringArrayOperatorIndexConst, GDExtensionInterfacePackedVector2ArrayOperatorIndex, GDExtensionInterfacePackedVector2ArrayOperatorIndexConst, GDExtensionInterfacePackedVector3ArrayOperatorIndex, GDExtensionInterfacePackedVector3ArrayOperatorIndexConst, GDExtensionInterfacePrintError, GDExtensionInterfacePrintErrorWithMessage, GDExtensionInterfacePrintScriptError, GDExtensionInterfacePrintScriptErrorWithMessage, GDExtensionInterfacePrintWarning, GDExtensionInterfacePrintWarningWithMessage, GDExtensionInterfaceRefGetObject, GDExtensionInterfaceRefSetObject, GDExtensionInterfaceScriptInstanceCreate, GDExtensionInterfaceStringNewWithLatin1Chars, GDExtensionInterfaceStringNewWithLatin1CharsAndLen, GDExtensionInterfaceStringNewWithUtf16Chars, GDExtensionInterfaceStringNewWithUtf16CharsAndLen, GDExtensionInterfaceStringNewWithUtf32Chars, GDExtensionInterfaceStringNewWithUtf32CharsAndLen, GDExtensionInterfaceStringNewWithUtf8Chars, GDExtensionInterfaceStringNewWithUtf8CharsAndLen, GDExtensionInterfaceStringNewWithWideChars, GDExtensionInterfaceStringNewWithWideCharsAndLen, GDExtensionInterfaceStringOperatorIndex, GDExtensionInterfaceStringOperatorIndexConst, GDExtensionInterfaceStringOperatorPlusEqC32str, GDExtensionInterfaceStringOperatorPlusEqChar, GDExtensionInterfaceStringOperatorPlusEqCstr, GDExtensionInterfaceStringOperatorPlusEqString, GDExtensionInterfaceStringOperatorPlusEqWcstr, GDExtensionInterfaceStringToLatin1Chars, GDExtensionInterfaceStringToUtf16Chars, GDExtensionInterfaceStringToUtf32Chars, GDExtensionInterfaceStringToUtf8Chars, GDExtensionInterfaceStringToWideChars, GDExtensionInterfaceVariantBooleanize, GDExtensionInterfaceVariantCall, GDExtensionInterfaceVariantCallStatic, GDExtensionInterfaceVariantCanConvert, GDExtensionInterfaceVariantCanConvertStrict, GDExtensionInterfaceVariantConstruct, GDExtensionInterfaceVariantDestroy, GDExtensionInterfaceVariantDuplicate, GDExtensionInterfaceVariantEvaluate, GDExtensionInterfaceVariantGet, GDExtensionInterfaceVariantGetConstantValue, GDExtensionInterfaceVariantGetIndexed, GDExtensionInterfaceVariantGetKeyed, GDExtensionInterfaceVariantGetNamed, GDExtensionInterfaceVariantGetPtrBuiltinMethod, GDExtensionInterfaceVariantGetPtrConstructor, GDExtensionInterfaceVariantGetPtrDestructor, GDExtensionInterfaceVariantGetPtrGetter, GDExtensionInterfaceVariantGetPtrIndexedGetter, GDExtensionInterfaceVariantGetPtrIndexedSetter, GDExtensionInterfaceVariantGetPtrKeyedChecker, GDExtensionInterfaceVariantGetPtrKeyedGetter, GDExtensionInterfaceVariantGetPtrKeyedSetter, GDExtensionInterfaceVariantGetPtrOperatorEvaluator, GDExtensionInterfaceVariantGetPtrSetter, GDExtensionInterfaceVariantGetPtrUtilityFunction, GDExtensionInterfaceVariantGetType, GDExtensionInterfaceVariantGetTypeName, GDExtensionInterfaceVariantHasKey, GDExtensionInterfaceVariantHasMember, GDExtensionInterfaceVariantHasMethod, GDExtensionInterfaceVariantHash, GDExtensionInterfaceVariantHashCompare, GDExtensionInterfaceVariantIterGet, GDExtensionInterfaceVariantIterInit, GDExtensionInterfaceVariantIterNext, GDExtensionInterfaceVariantNewCopy, GDExtensionInterfaceVariantNewNil, GDExtensionInterfaceVariantRecursiveHash, GDExtensionInterfaceVariantSet, GDExtensionInterfaceVariantSetIndexed, GDExtensionInterfaceVariantSetKeyed, GDExtensionInterfaceVariantSetNamed, GDExtensionInterfaceVariantStringify, GDExtensionInterfaceWorkerThreadPoolAddNativeGroupTask, GDExtensionInterfaceWorkerThreadPoolAddNativeTask, GDExtensionInterfaceXmlParserOpenBuffer, GDExtensionPtrBuiltInMethod, GDExtensionStringName, GDExtensionType, GDExtensionVariantType};
use std::ffi::{c_char, CString};
use std::str::FromStr;
use crate::hash::MurmurHash3;

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

    pub print_error: GDExtensionInterfacePrintError,
    pub print_error_with_message: GDExtensionInterfacePrintErrorWithMessage,
    pub print_warning: GDExtensionInterfacePrintWarning,
    pub print_warning_with_message: GDExtensionInterfacePrintWarningWithMessage,
    pub print_script_error: GDExtensionInterfacePrintScriptError,
    pub print_script_error_with_message: GDExtensionInterfacePrintScriptErrorWithMessage,

    pub get_native_struct_size: GDExtensionInterfaceGetNativeStructSize,

    pub variant_new_copy: GDExtensionInterfaceVariantNewCopy,
    pub variant_new_nil: GDExtensionInterfaceVariantNewNil,
    pub variant_destroy: GDExtensionInterfaceVariantDestroy,
    pub variant_call: GDExtensionInterfaceVariantCall,
    pub variant_call_static: GDExtensionInterfaceVariantCallStatic,
    pub variant_evaluate: GDExtensionInterfaceVariantEvaluate,
    pub variant_set: GDExtensionInterfaceVariantSet,
    pub variant_set_named: GDExtensionInterfaceVariantSetNamed,
    pub variant_set_keyed: GDExtensionInterfaceVariantSetKeyed,
    pub variant_set_indexed: GDExtensionInterfaceVariantSetIndexed,
    pub variant_get: GDExtensionInterfaceVariantGet,
    pub variant_get_named: GDExtensionInterfaceVariantGetNamed,
    pub variant_get_keyed: GDExtensionInterfaceVariantGetKeyed,
    pub variant_get_indexed: GDExtensionInterfaceVariantGetIndexed,
    pub variant_iter_init: GDExtensionInterfaceVariantIterInit,
    pub variant_iter_next: GDExtensionInterfaceVariantIterNext,
    pub variant_iter_get: GDExtensionInterfaceVariantIterGet,
    pub variant_hash: GDExtensionInterfaceVariantHash,
    pub variant_recursive_hash: GDExtensionInterfaceVariantRecursiveHash,
    pub variant_hash_compare: GDExtensionInterfaceVariantHashCompare,
    pub variant_booleanize: GDExtensionInterfaceVariantBooleanize,
    pub variant_duplicate: GDExtensionInterfaceVariantDuplicate,
    pub variant_stringify: GDExtensionInterfaceVariantStringify,
    pub variant_get_type: GDExtensionInterfaceVariantGetType,
    pub variant_has_method: GDExtensionInterfaceVariantHasMethod,
    pub variant_has_member: GDExtensionInterfaceVariantHasMember,
    pub variant_has_key: GDExtensionInterfaceVariantHasKey,
    pub variant_get_type_name: GDExtensionInterfaceVariantGetTypeName,
    pub variant_can_convert: GDExtensionInterfaceVariantCanConvert,
    pub variant_can_convert_strict: GDExtensionInterfaceVariantCanConvertStrict,
    pub get_variant_from_type_constructor: GDExtensionInterfaceGetVariantFromTypeConstructor,
    pub get_variant_to_type_constructor: GDExtensionInterfaceGetVariantToTypeConstructor,
    pub variant_get_ptr_operator_evaluator: GDExtensionInterfaceVariantGetPtrOperatorEvaluator,
    pub variant_get_ptr_builtin_method: GDExtensionInterfaceVariantGetPtrBuiltinMethod,
    pub variant_get_ptr_constructor: GDExtensionInterfaceVariantGetPtrConstructor,
    pub variant_get_ptr_destructor: GDExtensionInterfaceVariantGetPtrDestructor,
    pub variant_construct: GDExtensionInterfaceVariantConstruct,
    pub variant_get_ptr_setter: GDExtensionInterfaceVariantGetPtrSetter,
    pub variant_get_ptr_getter: GDExtensionInterfaceVariantGetPtrGetter,
    pub variant_get_ptr_indexed_setter: GDExtensionInterfaceVariantGetPtrIndexedSetter,
    pub variant_get_ptr_indexed_getter: GDExtensionInterfaceVariantGetPtrIndexedGetter,
    pub variant_get_ptr_keyed_setter: GDExtensionInterfaceVariantGetPtrKeyedSetter,
    pub variant_get_ptr_keyed_getter: GDExtensionInterfaceVariantGetPtrKeyedGetter,
    pub variant_get_ptr_keyed_checker: GDExtensionInterfaceVariantGetPtrKeyedChecker,
    pub variant_get_constant_value: GDExtensionInterfaceVariantGetConstantValue,
    pub variant_get_ptr_utility_function: GDExtensionInterfaceVariantGetPtrUtilityFunction,

    pub string_new_with_latin1_chars: GDExtensionInterfaceStringNewWithLatin1Chars,
    pub string_new_with_utf8_chars: GDExtensionInterfaceStringNewWithUtf8Chars,
    pub string_new_with_utf16_chars: GDExtensionInterfaceStringNewWithUtf16Chars,
    pub string_new_with_utf32_chars: GDExtensionInterfaceStringNewWithUtf32Chars,
    pub string_new_with_wide_chars: GDExtensionInterfaceStringNewWithWideChars,
    pub string_new_with_latin1_chars_and_len: GDExtensionInterfaceStringNewWithLatin1CharsAndLen,
    pub string_new_with_utf8_chars_and_len: GDExtensionInterfaceStringNewWithUtf8CharsAndLen,
    pub string_new_with_utf16_chars_and_len: GDExtensionInterfaceStringNewWithUtf16CharsAndLen,
    pub string_new_with_utf32_chars_and_len: GDExtensionInterfaceStringNewWithUtf32CharsAndLen,
    pub string_new_with_wide_chars_and_len: GDExtensionInterfaceStringNewWithWideCharsAndLen,

    pub string_to_latin1_chars: GDExtensionInterfaceStringToLatin1Chars,
    pub string_to_utf8_chars: GDExtensionInterfaceStringToUtf8Chars,
    pub string_to_utf16_chars: GDExtensionInterfaceStringToUtf16Chars,
    pub string_to_utf32_chars: GDExtensionInterfaceStringToUtf32Chars,
    pub string_to_wide_chars: GDExtensionInterfaceStringToWideChars,

    pub string_operator_index: GDExtensionInterfaceStringOperatorIndex,
    pub string_operator_index_const: GDExtensionInterfaceStringOperatorIndexConst,
    pub string_operator_plus_eq_string: GDExtensionInterfaceStringOperatorPlusEqString,
    pub string_operator_plus_eq_char: GDExtensionInterfaceStringOperatorPlusEqChar,
    pub string_operator_plus_eq_cstr: GDExtensionInterfaceStringOperatorPlusEqCstr,
    pub string_operator_plus_eq_wcstr: GDExtensionInterfaceStringOperatorPlusEqWcstr,
    pub string_operator_plus_eq_c32str: GDExtensionInterfaceStringOperatorPlusEqC32str,

    pub xml_parser_open_buffer: GDExtensionInterfaceXmlParserOpenBuffer,

    pub file_access_store_buffer: GDExtensionInterfaceFileAccessStoreBuffer,
    pub file_access_get_buffer: GDExtensionInterfaceFileAccessGetBuffer,

    pub worker_thread_pool_add_native_group_task:
        GDExtensionInterfaceWorkerThreadPoolAddNativeGroupTask,
    pub worker_thread_pool_add_native_task: GDExtensionInterfaceWorkerThreadPoolAddNativeTask,

    pub packed_byte_array_operator_index: GDExtensionInterfacePackedByteArrayOperatorIndex,
    pub packed_byte_array_operator_index_const:
        GDExtensionInterfacePackedByteArrayOperatorIndexConst,

    pub packed_color_array_operator_index: GDExtensionInterfacePackedColorArrayOperatorIndex,
    pub packed_color_array_operator_index_const:
        GDExtensionInterfacePackedColorArrayOperatorIndexConst,

    pub packed_float32_array_operator_index: GDExtensionInterfacePackedFloat32ArrayOperatorIndex,
    pub packed_float32_array_operator_index_const:
        GDExtensionInterfacePackedFloat32ArrayOperatorIndexConst,

    pub packed_float64_array_operator_index: GDExtensionInterfacePackedFloat64ArrayOperatorIndex,
    pub packed_float64_array_operator_index_const:
        GDExtensionInterfacePackedFloat64ArrayOperatorIndexConst,

    pub packed_int32_array_operator_index: GDExtensionInterfacePackedInt32ArrayOperatorIndex,
    pub packed_int32_array_operator_index_const:
        GDExtensionInterfacePackedInt32ArrayOperatorIndexConst,

    pub packed_int64_array_operator_index: GDExtensionInterfacePackedInt64ArrayOperatorIndex,
    pub packed_int64_array_operator_index_const:
        GDExtensionInterfacePackedInt64ArrayOperatorIndexConst,

    pub packed_string_array_operator_index: GDExtensionInterfacePackedStringArrayOperatorIndex,
    pub packed_string_array_operator_index_const:
        GDExtensionInterfacePackedStringArrayOperatorIndexConst,

    pub packed_vector2_array_operator_index: GDExtensionInterfacePackedVector2ArrayOperatorIndex,
    pub packed_vector2_array_operator_index_const:
        GDExtensionInterfacePackedVector2ArrayOperatorIndexConst,

    pub packed_vector3_array_operator_index: GDExtensionInterfacePackedVector3ArrayOperatorIndex,
    pub packed_vector3_array_operator_index_const:
        GDExtensionInterfacePackedVector3ArrayOperatorIndexConst,

    pub array_operator_index: GDExtensionInterfaceArrayOperatorIndex,
    pub array_operator_index_const: GDExtensionInterfaceArrayOperatorIndexConst,
    pub array_ref: GDExtensionInterfaceArrayRef,
    pub array_set_typed: GDExtensionInterfaceArraySetTyped,

    pub dictionary_operator_index: GDExtensionInterfaceDictionaryOperatorIndex,
    pub dictionary_operator_index_const: GDExtensionInterfaceDictionaryOperatorIndexConst,

    pub object_method_bind_call: GDExtensionInterfaceObjectMethodBindCall,
    pub object_method_bind_ptrcall: GDExtensionInterfaceObjectMethodBindPtrcall,
    pub object_destroy: GDExtensionInterfaceObjectDestroy,
    pub global_get_singleton: GDExtensionInterfaceGlobalGetSingleton,
    pub object_get_instance_binding: GDExtensionInterfaceObjectGetInstanceBinding,
    pub object_set_instance_binding: GDExtensionInterfaceObjectSetInstanceBinding,
    pub object_set_instance: GDExtensionInterfaceObjectSetInstance,
    pub object_cast_to: GDExtensionInterfaceObjectCastTo,
    pub object_get_instance_from_id: GDExtensionInterfaceObjectGetInstanceFromId,
    pub object_get_instance_id: GDExtensionInterfaceObjectGetInstanceId,

    pub ref_get_object: GDExtensionInterfaceRefGetObject,
    pub ref_set_object: GDExtensionInterfaceRefSetObject,

    pub script_instance_create: GDExtensionInterfaceScriptInstanceCreate,

    pub classdb_construct_object: GDExtensionInterfaceClassdbConstructObject,
    pub classdb_get_method_bind: GDExtensionInterfaceClassdbGetMethodBind,
    pub classdb_get_class_tag: GDExtensionInterfaceClassdbGetClassTag,
    pub classdb_register_extension_class: GDExtensionInterfaceClassdbRegisterExtensionClass,
    pub classdb_register_extension_class_method:
        GDExtensionInterfaceClassdbRegisterExtensionClassMethod,
    pub classdb_register_extension_class_integer_constant:
        GDExtensionInterfaceClassdbRegisterExtensionClassIntegerConstant,
    pub classdb_register_extension_class_property:
        GDExtensionInterfaceClassdbRegisterExtensionClassProperty,
    pub classdb_register_extension_class_property_group:
        GDExtensionInterfaceClassdbRegisterExtensionClassPropertyGroup,
    pub classdb_register_extension_class_property_subgroup:
        GDExtensionInterfaceClassdbRegisterExtensionClassPropertySubgroup,
    pub classdb_register_extension_class_signal:
        GDExtensionInterfaceClassdbRegisterExtensionClassSignal,
    pub classdb_unregister_extension_class: GDExtensionInterfaceClassdbUnregisterExtensionClass,

    pub get_library_path: GDExtensionInterfaceGetLibraryPath,
}

#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of GDExtensionInterface"][::std::mem::size_of::<GDExtensionInterface>() - 1080usize];
    ["Alignment of GDExtensionInterface"][::std::mem::align_of::<GDExtensionInterface>() - 8usize];
    ["Offset of field: GDExtensionInterface::version_major"]
        [::std::mem::offset_of!(GDExtensionInterface, version_major) - 0usize];
    ["Offset of field: GDExtensionInterface::version_minor"]
        [::std::mem::offset_of!(GDExtensionInterface, version_minor) - 4usize];
    ["Offset of field: GDExtensionInterface::version_patch"]
        [::std::mem::offset_of!(GDExtensionInterface, version_patch) - 8usize];
    ["Offset of field: GDExtensionInterface::version_string"]
        [::std::mem::offset_of!(GDExtensionInterface, version_string) - 16usize];
    ["Offset of field: GDExtensionInterface::mem_alloc"]
        [::std::mem::offset_of!(GDExtensionInterface, mem_alloc) - 24usize];
    ["Offset of field: GDExtensionInterface::mem_realloc"]
        [::std::mem::offset_of!(GDExtensionInterface, mem_realloc) - 32usize];
    ["Offset of field: GDExtensionInterface::mem_free"]
        [::std::mem::offset_of!(GDExtensionInterface, mem_free) - 40usize];
    ["Offset of field: GDExtensionInterface::print_error"]
        [::std::mem::offset_of!(GDExtensionInterface, print_error) - 48usize];
    ["Offset of field: GDExtensionInterface::print_error_with_message"]
        [::std::mem::offset_of!(GDExtensionInterface, print_error_with_message) - 56usize];
    ["Offset of field: GDExtensionInterface::print_warning"]
        [::std::mem::offset_of!(GDExtensionInterface, print_warning) - 64usize];
    ["Offset of field: GDExtensionInterface::print_warning_with_message"]
        [::std::mem::offset_of!(GDExtensionInterface, print_warning_with_message) - 72usize];
    ["Offset of field: GDExtensionInterface::print_script_error"]
        [::std::mem::offset_of!(GDExtensionInterface, print_script_error) - 80usize];
    ["Offset of field: GDExtensionInterface::print_script_error_with_message"]
        [::std::mem::offset_of!(GDExtensionInterface, print_script_error_with_message) - 88usize];
    ["Offset of field: GDExtensionInterface::get_native_struct_size"]
        [::std::mem::offset_of!(GDExtensionInterface, get_native_struct_size) - 96usize];
    ["Offset of field: GDExtensionInterface::variant_new_copy"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_new_copy) - 104usize];
    ["Offset of field: GDExtensionInterface::variant_new_nil"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_new_nil) - 112usize];
    ["Offset of field: GDExtensionInterface::variant_destroy"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_destroy) - 120usize];
    ["Offset of field: GDExtensionInterface::variant_call"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_call) - 128usize];
    ["Offset of field: GDExtensionInterface::variant_call_static"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_call_static) - 136usize];
    ["Offset of field: GDExtensionInterface::variant_evaluate"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_evaluate) - 144usize];
    ["Offset of field: GDExtensionInterface::variant_set"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_set) - 152usize];
    ["Offset of field: GDExtensionInterface::variant_set_named"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_set_named) - 160usize];
    ["Offset of field: GDExtensionInterface::variant_set_keyed"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_set_keyed) - 168usize];
    ["Offset of field: GDExtensionInterface::variant_set_indexed"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_set_indexed) - 176usize];
    ["Offset of field: GDExtensionInterface::variant_get"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get) - 184usize];
    ["Offset of field: GDExtensionInterface::variant_get_named"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_named) - 192usize];
    ["Offset of field: GDExtensionInterface::variant_get_keyed"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_keyed) - 200usize];
    ["Offset of field: GDExtensionInterface::variant_get_indexed"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_indexed) - 208usize];
    ["Offset of field: GDExtensionInterface::variant_iter_init"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_iter_init) - 216usize];
    ["Offset of field: GDExtensionInterface::variant_iter_next"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_iter_next) - 224usize];
    ["Offset of field: GDExtensionInterface::variant_iter_get"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_iter_get) - 232usize];
    ["Offset of field: GDExtensionInterface::variant_hash"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_hash) - 240usize];
    ["Offset of field: GDExtensionInterface::variant_recursive_hash"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_recursive_hash) - 248usize];
    ["Offset of field: GDExtensionInterface::variant_hash_compare"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_hash_compare) - 256usize];
    ["Offset of field: GDExtensionInterface::variant_booleanize"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_booleanize) - 264usize];
    ["Offset of field: GDExtensionInterface::variant_duplicate"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_duplicate) - 272usize];
    ["Offset of field: GDExtensionInterface::variant_stringify"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_stringify) - 280usize];
    ["Offset of field: GDExtensionInterface::variant_get_type"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_type) - 288usize];
    ["Offset of field: GDExtensionInterface::variant_has_method"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_has_method) - 296usize];
    ["Offset of field: GDExtensionInterface::variant_has_member"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_has_member) - 304usize];
    ["Offset of field: GDExtensionInterface::variant_has_key"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_has_key) - 312usize];
    ["Offset of field: GDExtensionInterface::variant_get_type_name"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_type_name) - 320usize];
    ["Offset of field: GDExtensionInterface::variant_can_convert"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_can_convert) - 328usize];
    ["Offset of field: GDExtensionInterface::variant_can_convert_strict"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_can_convert_strict) - 336usize];
    ["Offset of field: GDExtensionInterface::get_variant_from_type_constructor"][::std::mem::offset_of!(
        GDExtensionInterface,
        get_variant_from_type_constructor
    ) - 344usize];
    ["Offset of field: GDExtensionInterface::get_variant_to_type_constructor"]
        [::std::mem::offset_of!(GDExtensionInterface, get_variant_to_type_constructor) - 352usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_operator_evaluator"][::std::mem::offset_of!(
        GDExtensionInterface,
        variant_get_ptr_operator_evaluator
    ) - 360usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_builtin_method"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_builtin_method) - 368usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_constructor"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_constructor) - 376usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_destructor"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_destructor) - 384usize];
    ["Offset of field: GDExtensionInterface::variant_construct"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_construct) - 392usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_setter"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_setter) - 400usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_getter"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_getter) - 408usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_indexed_setter"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_indexed_setter) - 416usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_indexed_getter"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_indexed_getter) - 424usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_keyed_setter"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_keyed_setter) - 432usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_keyed_getter"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_keyed_getter) - 440usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_keyed_checker"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_keyed_checker) - 448usize];
    ["Offset of field: GDExtensionInterface::variant_get_constant_value"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_constant_value) - 456usize];
    ["Offset of field: GDExtensionInterface::variant_get_ptr_utility_function"]
        [::std::mem::offset_of!(GDExtensionInterface, variant_get_ptr_utility_function) - 464usize];
    ["Offset of field: GDExtensionInterface::string_new_with_latin1_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_new_with_latin1_chars) - 472usize];
    ["Offset of field: GDExtensionInterface::string_new_with_utf8_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_new_with_utf8_chars) - 480usize];
    ["Offset of field: GDExtensionInterface::string_new_with_utf16_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_new_with_utf16_chars) - 488usize];
    ["Offset of field: GDExtensionInterface::string_new_with_utf32_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_new_with_utf32_chars) - 496usize];
    ["Offset of field: GDExtensionInterface::string_new_with_wide_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_new_with_wide_chars) - 504usize];
    ["Offset of field: GDExtensionInterface::string_new_with_latin1_chars_and_len"][::std::mem::offset_of!(
        GDExtensionInterface,
        string_new_with_latin1_chars_and_len
    ) - 512usize];
    ["Offset of field: GDExtensionInterface::string_new_with_utf8_chars_and_len"][::std::mem::offset_of!(
        GDExtensionInterface,
        string_new_with_utf8_chars_and_len
    ) - 520usize];
    ["Offset of field: GDExtensionInterface::string_new_with_utf16_chars_and_len"][::std::mem::offset_of!(
        GDExtensionInterface,
        string_new_with_utf16_chars_and_len
    ) - 528usize];
    ["Offset of field: GDExtensionInterface::string_new_with_utf32_chars_and_len"][::std::mem::offset_of!(
        GDExtensionInterface,
        string_new_with_utf32_chars_and_len
    ) - 536usize];
    ["Offset of field: GDExtensionInterface::string_new_with_wide_chars_and_len"][::std::mem::offset_of!(
        GDExtensionInterface,
        string_new_with_wide_chars_and_len
    ) - 544usize];
    ["Offset of field: GDExtensionInterface::string_to_latin1_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_to_latin1_chars) - 552usize];
    ["Offset of field: GDExtensionInterface::string_to_utf8_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_to_utf8_chars) - 560usize];
    ["Offset of field: GDExtensionInterface::string_to_utf16_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_to_utf16_chars) - 568usize];
    ["Offset of field: GDExtensionInterface::string_to_utf32_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_to_utf32_chars) - 576usize];
    ["Offset of field: GDExtensionInterface::string_to_wide_chars"]
        [::std::mem::offset_of!(GDExtensionInterface, string_to_wide_chars) - 584usize];
    ["Offset of field: GDExtensionInterface::string_operator_index"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_index) - 592usize];
    ["Offset of field: GDExtensionInterface::string_operator_index_const"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_index_const) - 600usize];
    ["Offset of field: GDExtensionInterface::string_operator_plus_eq_string"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_plus_eq_string) - 608usize];
    ["Offset of field: GDExtensionInterface::string_operator_plus_eq_char"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_plus_eq_char) - 616usize];
    ["Offset of field: GDExtensionInterface::string_operator_plus_eq_cstr"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_plus_eq_cstr) - 624usize];
    ["Offset of field: GDExtensionInterface::string_operator_plus_eq_wcstr"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_plus_eq_wcstr) - 632usize];
    ["Offset of field: GDExtensionInterface::string_operator_plus_eq_c32str"]
        [::std::mem::offset_of!(GDExtensionInterface, string_operator_plus_eq_c32str) - 640usize];
    ["Offset of field: GDExtensionInterface::xml_parser_open_buffer"]
        [::std::mem::offset_of!(GDExtensionInterface, xml_parser_open_buffer) - 648usize];
    ["Offset of field: GDExtensionInterface::file_access_store_buffer"]
        [::std::mem::offset_of!(GDExtensionInterface, file_access_store_buffer) - 656usize];
    ["Offset of field: GDExtensionInterface::file_access_get_buffer"]
        [::std::mem::offset_of!(GDExtensionInterface, file_access_get_buffer) - 664usize];
    ["Offset of field: GDExtensionInterface::worker_thread_pool_add_native_group_task"][::std::mem::offset_of!(
        GDExtensionInterface,
        worker_thread_pool_add_native_group_task
    )
        - 672usize];
    ["Offset of field: GDExtensionInterface::worker_thread_pool_add_native_task"][::std::mem::offset_of!(
        GDExtensionInterface,
        worker_thread_pool_add_native_task
    ) - 680usize];
    ["Offset of field: GDExtensionInterface::packed_byte_array_operator_index"]
        [::std::mem::offset_of!(GDExtensionInterface, packed_byte_array_operator_index) - 688usize];
    ["Offset of field: GDExtensionInterface::packed_byte_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_byte_array_operator_index_const
    ) - 696usize];
    ["Offset of field: GDExtensionInterface::packed_color_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_color_array_operator_index
    ) - 704usize];
    ["Offset of field: GDExtensionInterface::packed_color_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_color_array_operator_index_const
    )
        - 712usize];
    ["Offset of field: GDExtensionInterface::packed_float32_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_float32_array_operator_index
    ) - 720usize];
    ["Offset of field: GDExtensionInterface::packed_float32_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_float32_array_operator_index_const
    )
        - 728usize];
    ["Offset of field: GDExtensionInterface::packed_float64_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_float64_array_operator_index
    ) - 736usize];
    ["Offset of field: GDExtensionInterface::packed_float64_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_float64_array_operator_index_const
    )
        - 744usize];
    ["Offset of field: GDExtensionInterface::packed_int32_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_int32_array_operator_index
    ) - 752usize];
    ["Offset of field: GDExtensionInterface::packed_int32_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_int32_array_operator_index_const
    )
        - 760usize];
    ["Offset of field: GDExtensionInterface::packed_int64_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_int64_array_operator_index
    ) - 768usize];
    ["Offset of field: GDExtensionInterface::packed_int64_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_int64_array_operator_index_const
    )
        - 776usize];
    ["Offset of field: GDExtensionInterface::packed_string_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_string_array_operator_index
    ) - 784usize];
    ["Offset of field: GDExtensionInterface::packed_string_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_string_array_operator_index_const
    )
        - 792usize];
    ["Offset of field: GDExtensionInterface::packed_vector2_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_vector2_array_operator_index
    ) - 800usize];
    ["Offset of field: GDExtensionInterface::packed_vector2_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_vector2_array_operator_index_const
    )
        - 808usize];
    ["Offset of field: GDExtensionInterface::packed_vector3_array_operator_index"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_vector3_array_operator_index
    ) - 816usize];
    ["Offset of field: GDExtensionInterface::packed_vector3_array_operator_index_const"][::std::mem::offset_of!(
        GDExtensionInterface,
        packed_vector3_array_operator_index_const
    )
        - 824usize];
    ["Offset of field: GDExtensionInterface::array_operator_index"]
        [::std::mem::offset_of!(GDExtensionInterface, array_operator_index) - 832usize];
    ["Offset of field: GDExtensionInterface::array_operator_index_const"]
        [::std::mem::offset_of!(GDExtensionInterface, array_operator_index_const) - 840usize];
    ["Offset of field: GDExtensionInterface::array_ref"]
        [::std::mem::offset_of!(GDExtensionInterface, array_ref) - 848usize];
    ["Offset of field: GDExtensionInterface::array_set_typed"]
        [::std::mem::offset_of!(GDExtensionInterface, array_set_typed) - 856usize];
    ["Offset of field: GDExtensionInterface::dictionary_operator_index"]
        [::std::mem::offset_of!(GDExtensionInterface, dictionary_operator_index) - 864usize];
    ["Offset of field: GDExtensionInterface::dictionary_operator_index_const"]
        [::std::mem::offset_of!(GDExtensionInterface, dictionary_operator_index_const) - 872usize];
    ["Offset of field: GDExtensionInterface::object_method_bind_call"]
        [::std::mem::offset_of!(GDExtensionInterface, object_method_bind_call) - 880usize];
    ["Offset of field: GDExtensionInterface::object_method_bind_ptrcall"]
        [::std::mem::offset_of!(GDExtensionInterface, object_method_bind_ptrcall) - 888usize];
    ["Offset of field: GDExtensionInterface::object_destroy"]
        [::std::mem::offset_of!(GDExtensionInterface, object_destroy) - 896usize];
    ["Offset of field: GDExtensionInterface::global_get_singleton"]
        [::std::mem::offset_of!(GDExtensionInterface, global_get_singleton) - 904usize];
    ["Offset of field: GDExtensionInterface::object_get_instance_binding"]
        [::std::mem::offset_of!(GDExtensionInterface, object_get_instance_binding) - 912usize];
    ["Offset of field: GDExtensionInterface::object_set_instance_binding"]
        [::std::mem::offset_of!(GDExtensionInterface, object_set_instance_binding) - 920usize];
    ["Offset of field: GDExtensionInterface::object_set_instance"]
        [::std::mem::offset_of!(GDExtensionInterface, object_set_instance) - 928usize];
    ["Offset of field: GDExtensionInterface::object_cast_to"]
        [::std::mem::offset_of!(GDExtensionInterface, object_cast_to) - 936usize];
    ["Offset of field: GDExtensionInterface::object_get_instance_from_id"]
        [::std::mem::offset_of!(GDExtensionInterface, object_get_instance_from_id) - 944usize];
    ["Offset of field: GDExtensionInterface::object_get_instance_id"]
        [::std::mem::offset_of!(GDExtensionInterface, object_get_instance_id) - 952usize];
    ["Offset of field: GDExtensionInterface::ref_get_object"]
        [::std::mem::offset_of!(GDExtensionInterface, ref_get_object) - 960usize];
    ["Offset of field: GDExtensionInterface::ref_set_object"]
        [::std::mem::offset_of!(GDExtensionInterface, ref_set_object) - 968usize];
    ["Offset of field: GDExtensionInterface::script_instance_create"]
        [::std::mem::offset_of!(GDExtensionInterface, script_instance_create) - 976usize];
    ["Offset of field: GDExtensionInterface::classdb_construct_object"]
        [::std::mem::offset_of!(GDExtensionInterface, classdb_construct_object) - 984usize];
    ["Offset of field: GDExtensionInterface::classdb_get_method_bind"]
        [::std::mem::offset_of!(GDExtensionInterface, classdb_get_method_bind) - 992usize];
    ["Offset of field: GDExtensionInterface::classdb_get_class_tag"]
        [::std::mem::offset_of!(GDExtensionInterface, classdb_get_class_tag) - 1000usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class
    ) - 1008usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class_method"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class_method
    )
        - 1016usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class_integer_constant"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class_integer_constant
    )
        - 1024usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class_property"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class_property
    )
        - 1032usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class_property_group"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class_property_group
    )
        - 1040usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class_property_subgroup"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class_property_subgroup
    )
        - 1048usize];
    ["Offset of field: GDExtensionInterface::classdb_register_extension_class_signal"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_register_extension_class_signal
    )
        - 1056usize];
    ["Offset of field: GDExtensionInterface::classdb_unregister_extension_class"][::std::mem::offset_of!(
        GDExtensionInterface,
        classdb_unregister_extension_class
    ) - 1064usize];
    ["Offset of field: GDExtensionInterface::get_library_path"]
        [::std::mem::offset_of!(GDExtensionInterface, get_library_path) - 1072usize];
};

impl GDExtensionInterface {
    pub unsafe fn from_get_proc_address(
        get_proc_address: GDExtensionInterfaceGetProcAddress,
    ) -> Self {
        macro_rules! resolve_one {
            ($get_proc_address:ident $func:ident: $func_ty:ty) => {{
                let c_name = CString::from_str(stringify!($func))
                    .unwrap_or_else(|_| unreachable!("function name contained a null byte"));

                let res = $get_proc_address(c_name.as_ptr()).unwrap_or_else(|| {
                    panic!(concat!("missing GDExtension function: ", stringify!($func)));
                });

                std::mem::transmute::<GDExtensionInterfaceFunctionPtr, $func_ty>(res)
            }};
        }

        macro_rules! resolve {
            (
                $get_proc_address:ident,
                $({ $($rest:tt)* })?
                $(pub $func:ident: $func_ty:ty),*$(,)?
            ) => {
                Self {
                    $($($rest)*)?
                    $(
                        $func : resolve_one!($get_proc_address $func: $func_ty),
                    )*
                }
            };
        }

        let get_godot_version = unsafe {
            resolve_one!(get_proc_address get_godot_version: GDExtensionInterfaceGetGodotVersion)
        };
        let mut godot_version = GDExtensionGodotVersion::default();
        unsafe { get_godot_version(&raw mut godot_version) };

        unsafe {
            resolve! {
                get_proc_address,
                {
                    version_major: godot_version.major,
                    version_minor: godot_version.minor,
                    version_patch: godot_version.patch,
                    version_string: godot_version.string,
                }

                pub mem_alloc: GDExtensionInterfaceMemAlloc,
                pub mem_realloc: GDExtensionInterfaceMemRealloc,
                pub mem_free: GDExtensionInterfaceMemFree,

                pub print_error: GDExtensionInterfacePrintError,
                pub print_error_with_message: GDExtensionInterfacePrintErrorWithMessage,
                pub print_warning: GDExtensionInterfacePrintWarning,
                pub print_warning_with_message: GDExtensionInterfacePrintWarningWithMessage,
                pub print_script_error: GDExtensionInterfacePrintScriptError,
                pub print_script_error_with_message: GDExtensionInterfacePrintScriptErrorWithMessage,

                pub get_native_struct_size: GDExtensionInterfaceGetNativeStructSize,

                pub variant_new_copy: GDExtensionInterfaceVariantNewCopy,
                pub variant_new_nil: GDExtensionInterfaceVariantNewNil,
                pub variant_destroy: GDExtensionInterfaceVariantDestroy,
                pub variant_call: GDExtensionInterfaceVariantCall,
                pub variant_call_static: GDExtensionInterfaceVariantCallStatic,
                pub variant_evaluate: GDExtensionInterfaceVariantEvaluate,
                pub variant_set: GDExtensionInterfaceVariantSet,
                pub variant_set_named: GDExtensionInterfaceVariantSetNamed,
                pub variant_set_keyed: GDExtensionInterfaceVariantSetKeyed,
                pub variant_set_indexed: GDExtensionInterfaceVariantSetIndexed,
                pub variant_get: GDExtensionInterfaceVariantGet,
                pub variant_get_named: GDExtensionInterfaceVariantGetNamed,
                pub variant_get_keyed: GDExtensionInterfaceVariantGetKeyed,
                pub variant_get_indexed: GDExtensionInterfaceVariantGetIndexed,
                pub variant_iter_init: GDExtensionInterfaceVariantIterInit,
                pub variant_iter_next: GDExtensionInterfaceVariantIterNext,
                pub variant_iter_get: GDExtensionInterfaceVariantIterGet,
                pub variant_hash: GDExtensionInterfaceVariantHash,
                pub variant_recursive_hash: GDExtensionInterfaceVariantRecursiveHash,
                pub variant_hash_compare: GDExtensionInterfaceVariantHashCompare,
                pub variant_booleanize: GDExtensionInterfaceVariantBooleanize,
                pub variant_duplicate: GDExtensionInterfaceVariantDuplicate,
                pub variant_stringify: GDExtensionInterfaceVariantStringify,
                pub variant_get_type: GDExtensionInterfaceVariantGetType,
                pub variant_has_method: GDExtensionInterfaceVariantHasMethod,
                pub variant_has_member: GDExtensionInterfaceVariantHasMember,
                pub variant_has_key: GDExtensionInterfaceVariantHasKey,
                pub variant_get_type_name: GDExtensionInterfaceVariantGetTypeName,
                pub variant_can_convert: GDExtensionInterfaceVariantCanConvert,
                pub variant_can_convert_strict: GDExtensionInterfaceVariantCanConvertStrict,
                pub get_variant_from_type_constructor: GDExtensionInterfaceGetVariantFromTypeConstructor,
                pub get_variant_to_type_constructor: GDExtensionInterfaceGetVariantToTypeConstructor,
                pub variant_get_ptr_operator_evaluator: GDExtensionInterfaceVariantGetPtrOperatorEvaluator,
                pub variant_get_ptr_builtin_method: GDExtensionInterfaceVariantGetPtrBuiltinMethod,
                pub variant_get_ptr_constructor: GDExtensionInterfaceVariantGetPtrConstructor,
                pub variant_get_ptr_destructor: GDExtensionInterfaceVariantGetPtrDestructor,
                pub variant_construct: GDExtensionInterfaceVariantConstruct,
                pub variant_get_ptr_setter: GDExtensionInterfaceVariantGetPtrSetter,
                pub variant_get_ptr_getter: GDExtensionInterfaceVariantGetPtrGetter,
                pub variant_get_ptr_indexed_setter: GDExtensionInterfaceVariantGetPtrIndexedSetter,
                pub variant_get_ptr_indexed_getter: GDExtensionInterfaceVariantGetPtrIndexedGetter,
                pub variant_get_ptr_keyed_setter: GDExtensionInterfaceVariantGetPtrKeyedSetter,
                pub variant_get_ptr_keyed_getter: GDExtensionInterfaceVariantGetPtrKeyedGetter,
                pub variant_get_ptr_keyed_checker: GDExtensionInterfaceVariantGetPtrKeyedChecker,
                pub variant_get_constant_value: GDExtensionInterfaceVariantGetConstantValue,
                pub variant_get_ptr_utility_function: GDExtensionInterfaceVariantGetPtrUtilityFunction,

                pub string_new_with_latin1_chars: GDExtensionInterfaceStringNewWithLatin1Chars,
                pub string_new_with_utf8_chars: GDExtensionInterfaceStringNewWithUtf8Chars,
                pub string_new_with_utf16_chars: GDExtensionInterfaceStringNewWithUtf16Chars,
                pub string_new_with_utf32_chars: GDExtensionInterfaceStringNewWithUtf32Chars,
                pub string_new_with_wide_chars: GDExtensionInterfaceStringNewWithWideChars,
                pub string_new_with_latin1_chars_and_len: GDExtensionInterfaceStringNewWithLatin1CharsAndLen,
                pub string_new_with_utf8_chars_and_len: GDExtensionInterfaceStringNewWithUtf8CharsAndLen,
                pub string_new_with_utf16_chars_and_len: GDExtensionInterfaceStringNewWithUtf16CharsAndLen,
                pub string_new_with_utf32_chars_and_len: GDExtensionInterfaceStringNewWithUtf32CharsAndLen,
                pub string_new_with_wide_chars_and_len: GDExtensionInterfaceStringNewWithWideCharsAndLen,

                pub string_to_latin1_chars: GDExtensionInterfaceStringToLatin1Chars,
                pub string_to_utf8_chars: GDExtensionInterfaceStringToUtf8Chars,
                pub string_to_utf16_chars: GDExtensionInterfaceStringToUtf16Chars,
                pub string_to_utf32_chars: GDExtensionInterfaceStringToUtf32Chars,
                pub string_to_wide_chars: GDExtensionInterfaceStringToWideChars,

                pub string_operator_index: GDExtensionInterfaceStringOperatorIndex,
                pub string_operator_index_const: GDExtensionInterfaceStringOperatorIndexConst,
                pub string_operator_plus_eq_string: GDExtensionInterfaceStringOperatorPlusEqString,
                pub string_operator_plus_eq_char: GDExtensionInterfaceStringOperatorPlusEqChar,
                pub string_operator_plus_eq_cstr: GDExtensionInterfaceStringOperatorPlusEqCstr,
                pub string_operator_plus_eq_wcstr: GDExtensionInterfaceStringOperatorPlusEqWcstr,
                pub string_operator_plus_eq_c32str: GDExtensionInterfaceStringOperatorPlusEqC32str,

                pub xml_parser_open_buffer: GDExtensionInterfaceXmlParserOpenBuffer,

                pub file_access_store_buffer: GDExtensionInterfaceFileAccessStoreBuffer,
                pub file_access_get_buffer: GDExtensionInterfaceFileAccessGetBuffer,

                pub worker_thread_pool_add_native_group_task:
                    GDExtensionInterfaceWorkerThreadPoolAddNativeGroupTask,
                pub worker_thread_pool_add_native_task: GDExtensionInterfaceWorkerThreadPoolAddNativeTask,

                pub packed_byte_array_operator_index: GDExtensionInterfacePackedByteArrayOperatorIndex,
                pub packed_byte_array_operator_index_const:
                    GDExtensionInterfacePackedByteArrayOperatorIndexConst,

                pub packed_color_array_operator_index: GDExtensionInterfacePackedColorArrayOperatorIndex,
                pub packed_color_array_operator_index_const:
                    GDExtensionInterfacePackedColorArrayOperatorIndexConst,

                pub packed_float32_array_operator_index: GDExtensionInterfacePackedFloat32ArrayOperatorIndex,
                pub packed_float32_array_operator_index_const:
                    GDExtensionInterfacePackedFloat32ArrayOperatorIndexConst,

                pub packed_float64_array_operator_index: GDExtensionInterfacePackedFloat64ArrayOperatorIndex,
                pub packed_float64_array_operator_index_const:
                    GDExtensionInterfacePackedFloat64ArrayOperatorIndexConst,

                pub packed_int32_array_operator_index: GDExtensionInterfacePackedInt32ArrayOperatorIndex,
                pub packed_int32_array_operator_index_const:
                    GDExtensionInterfacePackedInt32ArrayOperatorIndexConst,

                pub packed_int64_array_operator_index: GDExtensionInterfacePackedInt64ArrayOperatorIndex,
                pub packed_int64_array_operator_index_const:
                    GDExtensionInterfacePackedInt64ArrayOperatorIndexConst,

                pub packed_string_array_operator_index: GDExtensionInterfacePackedStringArrayOperatorIndex,
                pub packed_string_array_operator_index_const:
                    GDExtensionInterfacePackedStringArrayOperatorIndexConst,

                pub packed_vector2_array_operator_index: GDExtensionInterfacePackedVector2ArrayOperatorIndex,
                pub packed_vector2_array_operator_index_const:
                    GDExtensionInterfacePackedVector2ArrayOperatorIndexConst,

                pub packed_vector3_array_operator_index: GDExtensionInterfacePackedVector3ArrayOperatorIndex,
                pub packed_vector3_array_operator_index_const:
                    GDExtensionInterfacePackedVector3ArrayOperatorIndexConst,

                pub array_operator_index: GDExtensionInterfaceArrayOperatorIndex,
                pub array_operator_index_const: GDExtensionInterfaceArrayOperatorIndexConst,
                pub array_ref: GDExtensionInterfaceArrayRef,
                pub array_set_typed: GDExtensionInterfaceArraySetTyped,

                pub dictionary_operator_index: GDExtensionInterfaceDictionaryOperatorIndex,
                pub dictionary_operator_index_const: GDExtensionInterfaceDictionaryOperatorIndexConst,

                pub object_method_bind_call: GDExtensionInterfaceObjectMethodBindCall,
                pub object_method_bind_ptrcall: GDExtensionInterfaceObjectMethodBindPtrcall,
                pub object_destroy: GDExtensionInterfaceObjectDestroy,
                pub global_get_singleton: GDExtensionInterfaceGlobalGetSingleton,
                pub object_get_instance_binding: GDExtensionInterfaceObjectGetInstanceBinding,
                pub object_set_instance_binding: GDExtensionInterfaceObjectSetInstanceBinding,
                pub object_set_instance: GDExtensionInterfaceObjectSetInstance,
                pub object_cast_to: GDExtensionInterfaceObjectCastTo,
                pub object_get_instance_from_id: GDExtensionInterfaceObjectGetInstanceFromId,
                pub object_get_instance_id: GDExtensionInterfaceObjectGetInstanceId,

                pub ref_get_object: GDExtensionInterfaceRefGetObject,
                pub ref_set_object: GDExtensionInterfaceRefSetObject,

                pub script_instance_create: GDExtensionInterfaceScriptInstanceCreate,

                pub classdb_construct_object: GDExtensionInterfaceClassdbConstructObject,
                pub classdb_get_method_bind: GDExtensionInterfaceClassdbGetMethodBind,
                pub classdb_get_class_tag: GDExtensionInterfaceClassdbGetClassTag,
                pub classdb_register_extension_class: GDExtensionInterfaceClassdbRegisterExtensionClass,
                pub classdb_register_extension_class_method:
                    GDExtensionInterfaceClassdbRegisterExtensionClassMethod,
                pub classdb_register_extension_class_integer_constant:
                    GDExtensionInterfaceClassdbRegisterExtensionClassIntegerConstant,
                pub classdb_register_extension_class_property:
                    GDExtensionInterfaceClassdbRegisterExtensionClassProperty,
                pub classdb_register_extension_class_property_group:
                    GDExtensionInterfaceClassdbRegisterExtensionClassPropertyGroup,
                pub classdb_register_extension_class_property_subgroup:
                    GDExtensionInterfaceClassdbRegisterExtensionClassPropertySubgroup,
                pub classdb_register_extension_class_signal:
                    GDExtensionInterfaceClassdbRegisterExtensionClassSignal,
                pub classdb_unregister_extension_class: GDExtensionInterfaceClassdbUnregisterExtensionClass,

                pub get_library_path: GDExtensionInterfaceGetLibraryPath,
            }
        }
    }

    /// Looks up a builtin method using `variant_get_ptr_builtin_method`.
    pub fn lookup_built_in_method<T>(
        &self,
        is_const: bool,
        is_static: bool,
        is_vararg: bool,
        name: impl Into<GDExtensionStringName>,
        return_type: Option<GDExtensionVariantType>,
        arguments: &[GDExtensionVariantType],
    ) -> Option<GDExtensionPtrBuiltInMethod>
    where
        T: GDExtensionType,
    {
        // compute hash of the method - the engine checks to make sure we have the right signature
        // https://github.com/godotengine/godot/blob/master/core/variant/variant_call.cpp#L1308-L1322
        let mut hasher = MurmurHash3::new();
        hasher.hash(if is_const { 1 } else { 0 });
        hasher.hash(if is_static { 1 } else { 0 });
        hasher.hash(if is_vararg { 1 } else { 0 });

        if let Some(return_type) = return_type {
            hasher.hash(1);
            hasher.hash(return_type as u32);
        } else {
            hasher.hash(0);
        }

        hasher.hash(arguments.len() as u32);

        for argument in arguments {
            hasher.hash(*argument as u32);
        }

        let hash = hasher.finish();
        let name = name.into();

        unsafe {
            (self.variant_get_ptr_builtin_method)(
                T::VARIANT_TYPE,
                &name,
                hash as GDExtensionInt
            )
        }
    }
}
