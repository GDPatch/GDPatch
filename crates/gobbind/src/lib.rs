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
    Consumes, Converter, ConvertingFn, FunctionInfo, FunctionVisitor, GobbindFunctions, Provides,
};

/// TODO
pub use gobbind_macros::Gobbind;

/// TODO
pub use gobbind_macros::gobbind;
