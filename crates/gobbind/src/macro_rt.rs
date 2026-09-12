use crate::{Converter, ConvertingFn, CustomAttributes, FunctionInfo};
use std::any::Any;

pub type ErasedCustomAttribute = &'static (dyn Any + Send + Sync);
pub type CustomAttributesInner = &'static [ErasedCustomAttribute];

pub const fn new_function_info<'env, 'converter_state, Imp, ConverterImpl, ImpDummy>(
    imp: Imp,
    name: &'static str,
    custom_attributes: CustomAttributesInner,
) -> FunctionInfo<Imp, (ConverterImpl, ImpDummy)>
where
    Imp: ConvertingFn<'env, 'converter_state, ConverterImpl, ImpDummy>,
    ConverterImpl: Converter<'converter_state>,
{
    let custom_attributes = CustomAttributes::new(custom_attributes);
    FunctionInfo::const_new(imp, name, custom_attributes)
}

// utility to give pretty error messages if you use #[gobbind] on an impl without the required
// derive
#[diagnostic::on_unimplemented(
    message = "#[gobbind] requires #[derive(Gobbind)] on the type definition"
)]
pub trait GobbindAttributeRequiresDeriveOnType {}

pub const fn assert_derives_gobbind<T>()
where
    T: GobbindAttributeRequiresDeriveOnType,
{
}
