#![feature(try_blocks)]

use include_dir::{Dir, include_dir};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};
use thiserror::Error;

pub mod hook;
mod os;

#[cfg(debug_assertions)]
static LOADER_DIR: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../../dotnet/GDPatchSharp.Loader/bin/Debug/net6.0");

#[cfg(not(debug_assertions))]
static LOADER_DIR: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../../dotnet/GDPatchSharp.Loader/bin/Release/net6.0");

pub(crate) static LOADER_ASSEMBLY: OnceLock<PathBuf> = OnceLock::new();

#[derive(Error, Debug)]
pub enum Error {
    #[error("system API error {} ({})", .code, .message)]
    System { code: u32, message: String },

    #[error("failed to place function hook")]
    Hook,

    #[error("IO error")]
    IO(#[from] std::io::Error),

    #[error("failed to parse string")]
    String,

    #[error("unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn init(assembly_dir: &Path) -> crate::Result<()> {
    LOADER_DIR.extract(assembly_dir)?;
    LOADER_ASSEMBLY
        .set(assembly_dir.join("GDPatchSharp.Loader.dll"))
        .expect("init was called twice");

    os::init()?;

    Ok(())
}
