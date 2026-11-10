use crate::gdextension::api::API;
use crate::gdextension::sys::{
    GDExtensionBool, GDExtensionCallError, GDExtensionClassCreationInfo,
    GDExtensionClassInstancePtr, GDExtensionClassMethodArgumentMetadata,
    GDExtensionClassMethodFlags, GDExtensionClassMethodInfo, GDExtensionConstTypePtr,
    GDExtensionConstVariantPtr, GDExtensionInstanceBindingCallbacks, GDExtensionInt,
    GDExtensionObjectPtr, GDExtensionPropertyInfo, GDExtensionString, GDExtensionStringName,
    GDExtensionStringPtr, GDExtensionType, GDExtensionTypePtr, GDExtensionVariant,
    GDExtensionVariantPtr, GDExtensionVariantType, PackedArray,
};
use gdpatch_godot::variant::Variant;
use gobbind::{
    Consumes, Converter, DynamicFn, FunctionInfo, Provides, WalkArgument, WalkReturn, WalkableFn,
    Walker,
};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::{ffi::c_void, mem, ptr};

#[derive(Debug)]
pub struct GDExtensionPtrcallConverter<Class> {
    instance: GDExtensionClassInstancePtr,
    args: *const GDExtensionConstTypePtr,
    ret: GDExtensionTypePtr,
    phantom: PhantomData<Class>,
}

impl<'state, Class> Converter<'state> for GDExtensionPtrcallConverter<Class> {
    type Error = ();
}

impl<'state, Class> Provides<'state, String> for GDExtensionPtrcallConverter<Class> {
    fn provide(&mut self) -> Result<String, Self::Error> {
        let argument = unsafe { *self.args } as GDExtensionStringPtr;
        unsafe {
            self.args = self.args.add(1);
        }

        let argument = unsafe { argument.as_ref_unchecked() }; // TODO
        Ok(argument.into())
    }
}

impl<'state, Class> Consumes<'state, bool> for GDExtensionPtrcallConverter<Class> {
    fn consume(&mut self, value: bool) -> Result<(), Self::Error> {
        unsafe {
            *(self.ret as *mut GDExtensionBool) = value as GDExtensionBool;
        }

        Ok(())
    }
}

impl<'state, Class> Consumes<'state, u64> for GDExtensionPtrcallConverter<Class> {
    fn consume(&mut self, value: u64) -> Result<(), Self::Error> {
        unsafe {
            *(self.ret as *mut GDExtensionInt) = value as GDExtensionInt;
        }

        Ok(())
    }
}

impl<'state, Class> Consumes<'state, Vec<u8>> for GDExtensionPtrcallConverter<Class> {
    fn consume(&mut self, value: Vec<u8>) -> Result<(), Self::Error> {
        let array = unsafe { (self.ret as *mut PackedArray<u8>).as_mut_unchecked() };
        array.resize(value.len());
        array.copy_from_slice(&value);
        Ok(())
    }
}

impl<'state, Class> Consumes<'state, ()> for GDExtensionPtrcallConverter<Class> {
    fn consume(&mut self, _: ()) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct GDExtensionVariantConverter<Class> {
    instance: GDExtensionClassInstancePtr,
    args: *const GDExtensionConstVariantPtr,
    r#return: GDExtensionVariantPtr,
    marker: PhantomData<Class>,
}

impl<Class> GDExtensionVariantConverter<Class> {
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

impl<'state, Class> Converter<'state> for GDExtensionVariantConverter<Class> {
    type Error = GDExtensionCallError;
}

impl<'state, Class> Provides<'state, String> for GDExtensionVariantConverter<Class> {
    fn provide(&mut self) -> Result<String, Self::Error> {
        let argument = self.next_arg::<GDExtensionString>().unwrap(); // TODO
        Ok((&argument).into())
    }
}

impl<'state, Class> Consumes<'state, String> for GDExtensionVariantConverter<Class> {
    fn consume(&mut self, value: String) -> Result<(), Self::Error> {
        self.return_arg(GDExtensionString::from(value.as_str()));
        Ok(())
    }
}

impl<'state, Class> Consumes<'state, ()> for GDExtensionVariantConverter<Class> {
    fn consume(&mut self, _: ()) -> Result<(), Self::Error> {
        unsafe {
            *self.r#return = GDExtensionVariant::new();
        }

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
            type_,
            ..Default::default()
        }
    }

    pub fn as_raw(&self) -> GDExtensionPropertyInfo {
        GDExtensionPropertyInfo {
            type_: self.type_,
            name: self.name.as_ref().unwrap_or(&API.strings().empty_name),
            class_name: self
                .class_name
                .as_ref()
                .unwrap_or(&API.strings().empty_name),
            hint: self.hint,
            hint_string: self.hint_string.as_ref().unwrap_or(&API.strings().empty),
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

pub trait GDClassShaped {
    fn new() -> Self;
}

#[derive(Debug)]
pub struct GDClass<T> {
    userdata: ManuallyDrop<Box<GDClassUserData>>,
    marker: PhantomData<T>,
}

#[derive(Debug)]
struct GDClassUserData {
    pub parent_name: GDExtensionStringName,
    pub class_name: GDExtensionStringName,
}

unsafe extern "C" fn create_instance_impl<T>(userdata_ptr: *mut c_void) -> GDExtensionObjectPtr
where
    T: GDClassShaped,
{
    let instance = T::new();
    let ptr = Box::into_raw(Box::new(instance)) as GDExtensionClassInstancePtr;

    let callbacks = GDExtensionInstanceBindingCallbacks {
        create_callback: None,
        free_callback: None,
        reference_callback: None,
    };

    unsafe {
        let userdata = &*userdata_ptr.cast::<GDClassUserData>();
        let object = (API.interface.classdb_construct_object)(&raw const userdata.parent_name);
        (API.interface.object_set_instance)(object, &raw const userdata.class_name, ptr);
        (API.interface.object_set_instance_binding)(object, API.library, ptr, &raw const callbacks);

        object
    }
}

unsafe extern "C" fn free_instance_impl<T>(
    _userdata_ptr: *mut c_void,
    instance: GDExtensionClassInstancePtr,
) where
    T: GDClassShaped,
{
    let boxed = unsafe { Box::from_raw(instance.cast::<T>()) };
    drop(boxed);
}

unsafe extern "C" fn call_impl<Class, Fn, Dummy>(
    method_userdata_ptr: *mut c_void,
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstVariantPtr,
    p_argument_count: GDExtensionInt,
    r_return: GDExtensionVariantPtr,
    r_error: *mut GDExtensionCallError,
) where
    Fn: DynamicFn<'static, 'static, GDExtensionVariantConverter<Class>, Dummy>,
{
    let method_userdata = unsafe { &*method_userdata_ptr.cast::<Fn>() };

    let mut converter = GDExtensionVariantConverter {
        instance: p_instance,
        args: p_args,
        r#return: r_return,
        marker: PhantomData,
    };

    let result = method_userdata.call_dynamic(&mut converter);

    if let Err(error) = result {
        unsafe {
            *r_error = error;
        }
    }
}

unsafe extern "C" fn ptrcall_impl<Class, Fn, Dummy>(
    method_userdata_ptr: *mut c_void,
    p_instance: GDExtensionClassInstancePtr,
    p_args: *const GDExtensionConstTypePtr,
    r_ret: GDExtensionTypePtr,
) where
    Fn: DynamicFn<'static, 'static, GDExtensionPtrcallConverter<Class>, Dummy>,
{
    let method_userdata = unsafe { &*method_userdata_ptr.cast::<Fn>() };

    let mut converter = GDExtensionPtrcallConverter {
        instance: p_instance,
        args: p_args,
        ret: r_ret,
        phantom: PhantomData,
    };

    let _ = method_userdata.call_dynamic(&mut converter);
}

impl<Class> GDClass<Class>
where
    Class: GDClassShaped,
{
    pub fn new(class_name: &str) -> Self {
        let class_userdata = ManuallyDrop::new(Box::new(GDClassUserData {
            parent_name: API.strings().object.clone(),
            class_name: class_name.into(),
        }));

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
            create_instance_func: create_instance_impl::<Class>,
            free_instance_func: free_instance_impl::<Class>,
            get_virtual_func: None,
            get_rid_func: None,
            class_userdata: Box::as_ptr(&class_userdata) as *mut c_void,
        };

        unsafe {
            (API.interface.classdb_register_extension_class)(
                API.library,
                &raw const class_userdata.class_name,
                &raw const class_userdata.parent_name,
                &raw const class_info,
            );
        }

        Self {
            userdata: class_userdata,
            marker: PhantomData,
        }
    }

    pub fn add_function<'this, Fn, Dummy>(&'this mut self, function: Fn)
    where
        Fn: WalkableFn<'static, 'this, GDExtensionWalker, Dummy>,
        Fn: DynamicFn<'static, 'static, GDExtensionPtrcallConverter<Class>, Dummy>,
        Fn: DynamicFn<'static, 'static, GDExtensionVariantConverter<Class>, Dummy>,
    {
        let Some(info) = FunctionInfo::of::<Fn>() else {
            panic!("cannot bind a function without reflection info")
        };

        let name = GDExtensionStringName::from(info.name());
        let method_flags = GDExtensionClassMethodFlags::STATIC;

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
            method_userdata: unsafe { mem::transmute::<&Fn, *mut c_void>(&function) },

            call_func: call_impl::<Class, Fn, _>,
            ptrcall_func: ptrcall_impl::<Class, Fn, _>,
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
                &self.userdata.class_name,
                &raw const method_info,
            );
        }
    }
}

impl<T> Drop for GDClass<T> {
    fn drop(&mut self) {
        unsafe {
            (API.interface.classdb_unregister_extension_class)(
                API.library,
                &self.userdata.class_name,
            );

            ManuallyDrop::drop(&mut self.userdata);
        }
    }
}
