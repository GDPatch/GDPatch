use crate::CustomAttributes;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub trait Converter<'state> {
    type Error;
}

pub trait Provides<'state, T>: Converter<'state> {
    fn provide(&mut self) -> Result<T, Self::Error>;
}

pub trait Consumes<'state, T>: Converter<'state> {
    fn consume(&mut self, value: T) -> Result<(), Self::Error>;
}

/// Dynamic version of the `Fn` trait.
///
/// Allows calling functions using arguments generated at runtime via a [converter]. This trait is
/// implemented for functions with up to 6 arguments, or 12 if the `12-arg` feature is enabled.
///
/// # Type Parameters
/// - `'env` is the lifetime of the function (self) - this will be `'static` unless the function is
///   a closure that borrows from its environment.
/// - `'converter` is the lifetime of the converter.
/// - `ConverterImpl` is the type of the [converter] implementation.
/// - `Dummy` is a dummy type parameter to avoid potential conflicting implementations.
///
/// [converter]: Converter
pub trait ConvertingFn<'env, 'converter, ConverterImpl, Dummy>
where
    Self: 'env,
    ConverterImpl: Converter<'converter>,
{
    /// Calls this function using a converter to generate arguments and handle the return value.
    fn converting_call(&self, converter: &mut ConverterImpl) -> Result<(), ConverterImpl::Error>;
}

macro_rules! impl_function {
    ($(($Arg:ident, $arg:ident)),*) => {
        impl<'env, 'converter, ConverterImpl, FunctionImpl, Output, $($Arg,)*>
            ConvertingFn<'env, 'converter, ConverterImpl, fn($($Arg),*) -> Output> for FunctionImpl
        where
            FunctionImpl: Fn($($Arg),*) -> Output + 'env,
            ConverterImpl: Converter<'converter>,
            ConverterImpl: Consumes<'converter, Output>,
            $(ConverterImpl: Provides<'converter, $Arg>,)*
            $($Arg: 'converter,)*
        {
            fn converting_call(
                &self,
                converter: &mut ConverterImpl,
            ) -> Result<(), ConverterImpl::Error> {
                $(
                let $arg = Provides::<$Arg>::provide(converter)?;
                )*

                let result = (self)($($arg,)*);
                converter.consume(result)
            }
        }
    };
}

#[cfg(not(feature = "12-arg"))]
bevy_utils_proc_macros::all_tuples!(impl_function, 0, 6, Arg, arg);

#[cfg(feature = "12-arg")]
bevy_utils_proc_macros::all_tuples!(impl_callable, 0, 12, Arg, arg);

/// Reflection info for a function.
///
/// Generally instances of this type are obtained through [`walk_functions`] and constructed via
/// derive macro.
///
/// [`new`]: FunctionInfo::new
/// [`walk_functions`]: GobbindFunctions::walk_functions
pub struct FunctionInfo<Imp, Dummy> {
    name: Cow<'static, str>,
    custom_attributes: CustomAttributes,
    pub imp: Imp, // TODO: private this after working out what lifetime bullshit is happening
    marker: PhantomData<Dummy>,
}

impl<Imp, ImpDummy> Debug for FunctionInfo<Imp, ImpDummy> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionInfo")
            .field("name", &self.name)
            .field("custom_attributes", &self.custom_attributes)
            .finish_non_exhaustive()
    }
}

impl<Imp, ImpDummy> FunctionInfo<Imp, ImpDummy> {
    /// The name of the function.
    ///
    /// This will be the Rust name unless overridden with `#[gobbind(rename = "...")]`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Any custom attributes applied to the function.
    pub fn custom_attributes(&self) -> &CustomAttributes {
        &self.custom_attributes
    }
}

impl<'env, 'converter_state, Imp, ConverterImpl, ImpDummy>
    FunctionInfo<Imp, (ConverterImpl, ImpDummy)>
where
    Imp: ConvertingFn<'env, 'converter_state, ConverterImpl, ImpDummy>,
    ConverterImpl: Converter<'converter_state>,
{
    pub const fn const_new(
        imp: Imp,
        name: &'static str,
        custom_attributes: CustomAttributes,
    ) -> Self {
        Self {
            name: Cow::Borrowed(name),
            custom_attributes,
            imp,
            marker: PhantomData,
        }
    }
}

/// Proc-macro based reflection for scripting language bindings.
///
/// **This trait shouldn't be implemented by hand - use [`#[derive(Gobbind)]`](derive@Gobbind).**
///
/// This trait allows you to walk the exposed functions of a type using a provided converter for
/// argument conversion.
pub trait GobbindFunctions<'env, 'converter, Converter> {
    /// Walks the exposed functions of this type.
    fn walk_functions<V>(visitor: &mut V)
    where
        V: FunctionVisitor<'env, 'converter, Converter = Converter>;
}

/// Visitor trait for [`walk_functions`].
///
/// [`walk_functions`]: GobbindFunctions::walk_functions
pub trait FunctionVisitor<'env, 'converter> {
    type Converter: Converter<'converter>;

    /// Called for each exposed function in the type.
    fn visit<Imp, ImpDummy>(&mut self, function: FunctionInfo<Imp, (Self::Converter, ImpDummy)>)
    where
        Imp: ConvertingFn<'env, 'converter, Self::Converter, ImpDummy> + 'static;
}
