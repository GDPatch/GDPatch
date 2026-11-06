use std::{ffi::c_void, ptr};

use crate::gdextension::{
    api::API,
    sys::{
        GDExtensionBool, GDExtensionCallError, GDExtensionCallErrorType,
        GDExtensionClassCreationInfo, GDExtensionClassInstancePtr,
        GDExtensionClassMethodArgumentMetadata, GDExtensionClassMethodFlags,
        GDExtensionClassMethodInfo, GDExtensionConstTypePtr, GDExtensionConstVariantPtr,
        GDExtensionInt, GDExtensionObjectPtr, GDExtensionPropertyInfo, GDExtensionString,
        GDExtensionStringName, GDExtensionStringPtr, GDExtensionType, GDExtensionTypePtr,
        GDExtensionVariantPtr, GDExtensionVariantType, PackedArray,
    },
};
use gobbind::{
    Consumes, Converter, FunctionInfo, Provides, WalkArgument, WalkReturn, WalkableFn, Walker,
};

#[derive(Debug)]
pub struct GDExtensionPtrcallConverter {
    instance: GDExtensionClassInstancePtr,
    args: *const GDExtensionConstTypePtr,
    ret: GDExtensionTypePtr,
}

impl<'state> Converter<'state> for GDExtensionPtrcallConverter {
    type Error = ();
}

impl<'state> Provides<'state, String> for GDExtensionPtrcallConverter {
    fn provide(&mut self) -> Result<String, Self::Error> {
        let argument = unsafe { *self.args } as GDExtensionStringPtr;
        unsafe {
            self.args = self.args.add(1);
        }

        let argument = unsafe { argument.as_ref_unchecked() }; // TODO
        Ok(argument.into())
    }
}

impl<'state> Consumes<'state, bool> for GDExtensionPtrcallConverter {
    fn consume(&mut self, value: bool) -> Result<(), Self::Error> {
        unsafe {
            *(self.ret as *mut GDExtensionBool) = value as GDExtensionBool;
        }

        Ok(())
    }
}

impl<'state> Consumes<'state, u64> for GDExtensionPtrcallConverter {
    fn consume(&mut self, value: u64) -> Result<(), Self::Error> {
        unsafe {
            *(self.ret as *mut GDExtensionInt) = value as GDExtensionInt;
        }

        Ok(())
    }
}

impl<'state> Consumes<'state, Vec<u8>> for GDExtensionPtrcallConverter {
    fn consume(&mut self, value: Vec<u8>) -> Result<(), Self::Error> {
        let array = unsafe { (self.ret as *mut PackedArray<u8>).as_mut_unchecked() };
        array.resize(value.len());
        array.copy_from_slice(&value);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GDExtensionVariantConverter {
    instance: GDExtensionClassInstancePtr,
    args: *const GDExtensionConstVariantPtr,
    r#return: GDExtensionVariantPtr,
}

impl GDExtensionVariantConverter {
    pub fn next_arg<T: GDExtensionType>(&mut self) -> Option<T> {
        let argument = unsafe { *self.args };
        unsafe {
            self.args = self.args.add(1);
        }

        let argument = unsafe { argument.as_ref()? };
        Some(T::from_variant(argument))
    }

    pub fn return_arg<T: GDExtensionType>(&mut self, value: T) {
        unsafe {
            *self.r#return = T::to_variant(&value);
        }
    }
}

impl<'state> Converter<'state> for GDExtensionVariantConverter {
    type Error = GDExtensionCallError;
}

impl<'state> Provides<'state, String> for GDExtensionVariantConverter {
    fn provide(&mut self) -> Result<String, Self::Error> {
        let argument = self.next_arg::<GDExtensionString>().unwrap(); // TODO
        Ok((&argument).into())
    }
}

impl<'state> Consumes<'state, String> for GDExtensionVariantConverter {
    fn consume(&mut self, value: String) -> Result<(), Self::Error> {
        self.return_arg(GDExtensionString::from(value.as_str()));
        Ok(())
    }
}

// TODO: other converter stuff

#[derive(Debug, Default)]
struct OwnedArgument {
    pub type_: GDExtensionVariantType,
    pub name: Option<GDExtensionStringName>,
    pub class_name: Option<GDExtensionStringName>,
    pub hint: u32,
    pub hint_string: Option<GDExtensionString>,
    pub usage: u32,
    pub metadata: GDExtensionClassMethodArgumentMetadata,
}

impl OwnedArgument {
    pub fn new(type_: GDExtensionVariantType) -> Self {
        Self {
            type_: type_,
            ..Default::default()
        }
    }

    pub fn as_raw(&self) -> GDExtensionPropertyInfo {
        GDExtensionPropertyInfo {
            type_: self.type_,
            name: if let Some(name) = &self.name {
                name
            } else {
                ptr::null()
            },
            class_name: if let Some(class_name) = &self.class_name {
                class_name
            } else {
                ptr::null()
            },
            hint: self.hint,
            hint_string: if let Some(hint_string) = &self.hint_string {
                hint_string
            } else {
                ptr::null()
            },
            usage: self.usage,
        }
    }
}

#[derive(Debug, Default)]
pub struct GDExtensionWalker {
    return_type: Option<OwnedArgument>,
    arguments: Vec<OwnedArgument>,
}

impl<'state> Walker<'state> for GDExtensionWalker {
    type Error = ();
}

trait ArgumentType {
    fn convert() -> OwnedArgument;
}

impl ArgumentType for bool {
    fn convert() -> OwnedArgument {
        OwnedArgument::new(GDExtensionVariantType::Bool)
    }
}

impl ArgumentType for String {
    fn convert() -> OwnedArgument {
        OwnedArgument::new(GDExtensionVariantType::String)
    }
}

impl ArgumentType for u64 {
    fn convert() -> OwnedArgument {
        let mut argument = OwnedArgument::new(GDExtensionVariantType::Int);
        argument.metadata = GDExtensionClassMethodArgumentMetadata::IntIsUint64;
        argument
    }
}

impl ArgumentType for Vec<u8> {
    fn convert() -> OwnedArgument {
        OwnedArgument::new(GDExtensionVariantType::PackedByteArray)
    }
}

impl<'state, T> WalkArgument<'state, T> for GDExtensionWalker
where
    T: ArgumentType,
{
    fn walk(&mut self) -> Result<(), Self::Error> {
        self.arguments.push(T::convert());
        Ok(())
    }
}

impl<'state, T> WalkReturn<'state, T> for GDExtensionWalker
where
    T: ArgumentType,
{
    fn walk(&mut self) -> Result<(), Self::Error> {
        self.return_type = Some(T::convert());
        Ok(())
    }
}

impl<'state> WalkReturn<'state, ()> for GDExtensionWalker {
    fn walk(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct GDClass {
    class_name: GDExtensionStringName,
}

unsafe extern "C" fn create_instance_impl(p_userdata: *mut c_void) -> GDExtensionObjectPtr {
    todo!()
}

unsafe extern "C" fn free_instance_impl(
    p_userdata: *mut c_void,
    p_instance: GDExtensionClassInstancePtr,
) {
    todo!()
}

unsafe extern "C" fn call_impl(
    method_userdata: *mut c_void,
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstVariantPtr,
    p_argument_count: GDExtensionInt,
    r_return: GDExtensionVariantPtr,
    r_error: *mut GDExtensionCallError,
) {
    todo!()
}

unsafe extern "C" fn ptrcall_impl(
    method_userdata: *mut c_void,
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstTypePtr,
    r_ret: GDExtensionTypePtr,
) {
    todo!()
}

impl GDClass {
    pub fn new(class_name: &str) -> Self {
        let class_info = GDExtensionClassCreationInfo {
            is_virtual: 0,
            is_abstract: 0,
            set_func: None,
            get_func: None,
            get_property_list_func: None,
            free_property_list_func: None,
            property_can_revert_func: None,
            property_get_revert_func: None,
            notification_func: None,
            to_string_func: None,
            reference_func: None,
            unreference_func: None,
            create_instance_func: create_instance_impl,
            free_instance_func: free_instance_impl,
            get_virtual_func: None,
            get_rid_func: None,
            class_userdata: ptr::null_mut(),
        };

        let class_name = GDExtensionStringName::from(class_name);
        let parent_class_name = GDExtensionStringName::from("Node");
        unsafe {
            (API.interface.classdb_register_extension_class)(
                API.library,
                &raw const class_name,
                &raw const parent_class_name,
                &raw const class_info,
            );
        }

        Self { class_name }
    }

    pub fn add_function<'this, F, WalkerDummy>(&'this mut self, function: F)
    where
        F: WalkableFn<'static, 'this, GDExtensionWalker, WalkerDummy>,
        //F: DynamicFn<'static, 'this, GDExtensionConverter<'this>, WalkerDummy>,
    {
        let Some(info) = FunctionInfo::of::<F>() else {
            panic!("cannot bind a function without reflection info")
        };

        let name = GDExtensionStringName::from(info.name());
        let method_flags = GDExtensionClassMethodFlags::NORMAL;

        let mut walker = GDExtensionWalker::default();
        function.walk(&mut walker).unwrap(); // FIXME

        let return_type = walker.return_type.as_ref().map(|t| t.as_raw());
        let arguments = walker
            .arguments
            .iter()
            .map(|t| t.as_raw())
            .collect::<Vec<_>>();
        let arguments_metadata = walker
            .arguments
            .iter()
            .map(|t| t.metadata)
            .collect::<Vec<_>>();

        let method_info = GDExtensionClassMethodInfo {
            name: &name,
            method_userdata: ptr::null_mut(),

            call_func: call_impl,
            ptrcall_func: ptrcall_impl,
            method_flags: method_flags.0 as u32,

            has_return_value: if return_type.is_some() { 1 } else { 0 },
            return_value_info: if let Some(info) = return_type {
                &raw const info
            } else {
                ptr::null()
            },
            return_value_metadata: walker.return_type.map(|t| t.metadata).unwrap_or_default(),

            argument_count: arguments.len() as u32,
            arguments_info: arguments.as_ptr(),
            arguments_metadata: arguments_metadata.as_ptr(),

            default_argument_count: 0,
            default_arguments: ptr::null_mut(),
        };

        unsafe {
            (API.interface.classdb_register_extension_class_method)(
                API.library,
                &self.class_name,
                &raw const method_info,
            );
        }
    }
}
