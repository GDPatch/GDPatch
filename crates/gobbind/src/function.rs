use crate::__private::FUNCTION_INFOS;
use crate::CustomAttributes;
use std::any::TypeId;
use std::borrow::Cow;
use std::fmt::Debug;

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
/// Allows calling functions using arguments generated at runtime via a [Converter]. This trait is
/// implemented for functions with up to 6 arguments, or 12 if the `12-arg` feature is enabled.
///
/// # Type Parameters
/// - `'env` is the lifetime of the function (self) - this will be `'static` unless the function is
///   a closure that borrows from its environment.
/// - `'converter` is the lifetime of the converter.
/// - `ConverterImpl` is the type of the [Converter] implementation.
/// - `Dummy` is a dummy type parameter to avoid potential conflicting implementations.
pub trait DynamicFn<'env, 'converter, ConverterImpl, Dummy>
where
    Self: 'env,
    ConverterImpl: Converter<'converter>,
{
    /// Calls this function using a converter to generate arguments and handle the return value.
    fn call_dynamic(&self, converter: &mut ConverterImpl) -> Result<(), ConverterImpl::Error>;
}

pub trait Walker<'state> {
    type Error;
}

pub trait WalkArgument<'state, T>: Walker<'state> {
    fn walk(&mut self) -> Result<(), Self::Error>;
}

pub trait WalkReturn<'state, T>: Walker<'state> {
    fn walk(&mut self) -> Result<(), Self::Error>;
}

/// Trait implemented by function pointers to allow their argument and return types to be walked
/// over.
///
/// Allows exploring the arguments of a function at runtime using a [Walker]. This trait is
/// implemented for functions with up to 6 arguments, or 12 if the `12-arg` feature is enabled.
///
/// # Type Parameters
/// - `'env` is the lifetime of the function (self) - this will be `'static` unless the function is
///   a closure that borrows from its environment.
/// - `'walker` is the lifetime of the converter.
/// - `WalkerImpl` is the type of the [Walker] implementation.
/// - `Dummy` is a dummy type parameter to avoid potential conflicting implementations.
pub trait WalkableFn<'env, 'walker, WalkerImpl, Dummy>
where
    Self: 'env,
    WalkerImpl: Walker<'walker>,
{
    /// Calls the `walk` function on the provided walker for each argument as well as the return
    /// type.
    fn walk(&self, walker: &mut WalkerImpl) -> Result<(), WalkerImpl::Error>;
}

macro_rules! impl_function {
    ($(($Arg:ident, $arg:ident)),*) => {
        impl<'env, 'converter, ConverterImpl, FunctionImpl, Output, $($Arg,)*>
            DynamicFn<'env, 'converter, ConverterImpl, fn($($Arg),*) -> Output> for FunctionImpl
        where
            FunctionImpl: Fn($($Arg),*) -> Output + 'env,
            ConverterImpl: Converter<'converter>,
            ConverterImpl: Consumes<'converter, Output>,
            $(ConverterImpl: Provides<'converter, $Arg>,)*
            $($Arg: 'converter,)*
        {
            fn call_dynamic(
                &self,
                converter: &mut ConverterImpl,
            ) -> Result<(), ConverterImpl::Error> {
                $(
                let $arg = Provides::<$Arg>::provide(converter)?;
                )*

                let result = (self)($($arg,)*);
                Consumes::consume(converter, result)
            }
        }

        impl<'env, 'walker, WalkerImpl, FunctionImpl, Output, $($Arg,)*>
            WalkableFn<'env, 'walker, WalkerImpl, fn($($Arg),*) -> Output> for FunctionImpl
        where
            FunctionImpl: Fn($($Arg),*) -> Output + 'env,
            WalkerImpl: Walker<'walker>,
            WalkerImpl: WalkReturn<'walker, Output>,
            $(WalkerImpl: WalkArgument<'walker, $Arg>,)*
            $($Arg: 'walker,)*
        {
            fn walk(
                &self,
                walker: &mut WalkerImpl,
            ) -> Result<(), WalkerImpl::Error> {
                $(
                WalkArgument::<$Arg>::walk(walker)?;
                )*

                WalkReturn::walk(walker)?;
                Ok(())
            }
        }
    };
}

#[cfg(not(feature = "12-arg"))]
bevy_utils_proc_macros::all_tuples!(impl_function, 0, 6, Arg, arg);

#[cfg(feature = "12-arg")]
bevy_utils_proc_macros::all_tuples!(impl_callable, 0, 12, Arg, arg);

/// Reflection info for a function.
#[derive(Debug)]
pub struct FunctionInfo {
    name: Cow<'static, str>,
    custom_attributes: CustomAttributes,
}

impl FunctionInfo {
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

impl FunctionInfo {
    pub(crate) const fn const_new(name: &'static str, custom_attributes: CustomAttributes) -> Self {
        Self {
            name: Cow::Borrowed(name),
            custom_attributes,
        }
    }

    /// Fetches the [`FunctionInfo`] for a function.
    ///
    /// This will only return `Some` if the function was part of a
    /// [`#[gobbind]`][crate::gobbind@attr]-annotated impl block.
    pub fn of<F: 'static>() -> Option<&'static Self> {
        let target_ty = TypeId::of::<F>();
        FUNCTION_INFOS.iter().find_map(
            |(ty, info)| {
                if *ty == target_ty { Some(info) } else { None }
            },
        )
    }

    /// Fetches the [`FunctionInfo`] for a function.
    ///
    /// This will only return `Some` if the function was part of a
    /// [`#[gobbind]`][crate::gobbind@attr]-annotated impl block.
    pub fn of_val<F: 'static>(_f: F) -> Option<&'static Self> {
        Self::of::<F>()
    }
}
