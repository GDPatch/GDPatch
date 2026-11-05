use crate::gdextension::sys::{
    GDExtensionClassMethodArgumentMetadata, GDExtensionPropertyInfo, GDExtensionString,
    GDExtensionStringName, GDExtensionVariantType,
};
use gobbind::{Converter, DynamicFn, FunctionInfo, WalkArgument, WalkReturn, WalkableFn, Walker};
use std::ptr;

#[derive(Debug)]
pub struct GDExtensionConverter<'builder> {
    builder: &'builder mut GDClassBuilder,
}

impl<'state> Converter<'state> for GDExtensionConverter<'state> {
    type Error = ();
}

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
}

#[derive(Debug)]
pub struct GDExtensionWalker<'builder> {
    builder: &'builder mut GDClassBuilder,
    return_type: Option<OwnedArgument>,
    arguments: Vec<OwnedArgument>,
}

impl<'state> Walker<'state> for GDExtensionWalker<'state> {
    type Error = ();
}

impl<'state> WalkReturn<'state, ()> for GDExtensionWalker<'state> {
    fn walk(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'state> WalkReturn<'state, bool> for GDExtensionWalker<'state> {
    fn walk(&mut self) -> Result<(), Self::Error> {
        let info = OwnedArgument::new(GDExtensionVariantType::Bool);
        self.return_type = Some(info);

        Ok(())
    }
}

impl<'state> WalkReturn<'state, Option<Vec<u8>>> for GDExtensionWalker<'state> {
    fn walk(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

impl<'state> WalkArgument<'state, String> for GDExtensionWalker<'state> {
    fn walk(&mut self) -> Result<(), Self::Error> {
        let info = GDExtensionPropertyInfo {
            type_: GDExtensionVariantType::String,
            name: ptr::null_mut(),
            class_name: ptr::null_mut(),
            hint: Default::default(),
            hint_string: ptr::null_mut(),
            usage: Default::default(),
        };

        // self.arguments
        //     .push((info, GDExtensionClassMethodArgumentMetadata::None));

        Ok(())
    }
}

#[derive(Debug)]
struct GDClassBuilder {}

impl GDClassBuilder {
    pub fn add_function<'this, F, WalkerDummy>(&mut self, _f: F)
    where
        F: WalkableFn<'static, 'this, GDExtensionWalker<'this>, WalkerDummy>,
        F: DynamicFn<'static, 'this, GDExtensionConverter<'this>, WalkerDummy>,
    {
        let Some(info) = FunctionInfo::of::<F>() else {
            panic!("cannot bind a function without reflection info")
        };

        info.name();
    }
}
