//! # `gdpatch-godot`
//!
//! Godot types and file format parsers.
extern crate core;
use thiserror::Error;

pub mod build;
pub mod gdscript;
mod marshalling;
pub mod pack;
pub mod project_settings;
mod string;
mod util;
pub mod variant;

pub use self::marshalling::{ReadableMarshalBuffer, WritableMarshalBuffer};

mod private {
    pub trait Sealed {}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("provided buffer shorter than expected")]
    TooShort,

    #[error("malformed data")]
    BadData,

    #[error("unsupported/unknown version {}", .0)]
    UnknownVersion(u32),

    #[error("an i/o error occurred: {}", .0)]
    Io(#[from] std::io::Error),
}
