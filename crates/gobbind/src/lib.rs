//! # Gobbind
//! Procedural macro-based reflection library for implementing Rust bindings to scripting languages.
extern crate self as gobbind;
#[doc(hidden)] // macro runtime, not public API
#[path = "macro_rt.rs"]
pub mod __private;

mod custom_attributes;
mod function;
mod testing;

pub use crate::custom_attributes::CustomAttributes;
pub use crate::function::{
    Consumes, Converter, DynamicFn, FunctionInfo, Provides, WalkArgument, WalkReturn, WalkableFn,
    Walker,
};

/// TODO
pub use gobbind_macros::gobbind;
