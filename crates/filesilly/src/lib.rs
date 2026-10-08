//! Userspace filesystem overlay using function detouring.
use parking_lot::Mutex;
use std::fmt::Debug;
use std::io;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;
use thiserror::Error;

mod hook;
mod os;
mod recursion_guard;

static INSTANCE: OnceLock<Filesilly> = OnceLock::new();

pub type HeapStream = Arc<Mutex<dyn Stream>>;

#[derive(Debug)]
struct Filesilly {
    platform: os::FilesillyPlatform,
    factory: Box<dyn StreamFactory>,
}

impl Filesilly {
    pub(crate) fn setup(platform: os::FilesillyPlatform, factory: Box<dyn StreamFactory>) {
        let instance = Self { platform, factory };

        INSTANCE.set(instance).expect("called init() twice");
    }

    pub fn instance() -> &'static Filesilly {
        INSTANCE
            .get()
            .expect("tried to get filesilly instance before `init`")
    }

    pub fn platform() -> &'static os::FilesillyPlatform {
        let instance = Self::instance();
        &instance.platform
    }

    pub fn factory() -> &'static dyn StreamFactory {
        let instance = Self::instance();
        &*instance.factory
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("system API error {} ({})", .code, .message)]
    System { code: u32, message: String },

    #[error("failed to place function hook")]
    Hook,

    #[error("an i/o error occurred: {}", .0)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Trait returned by [`StreamFactory`] to allow proxying game file reads/writes.
pub trait Stream: Read + Write + Seek + Send + Debug {}
impl Stream for std::fs::File {}

#[derive(Debug, Clone)]
pub struct Stat {
    /// Size of the file in bytes.
    pub size: u64,

    /// The last access time (corresponds to `st_atim` on Unix and `LastAccessTime` on Windows).
    pub access_time: SystemTime,

    /// The last modification time (corresponds to `st_mtim` on Unix and `LastWriteTime` on Windows).
    pub modification_time: SystemTime,

    /// The last time the file was "changed" (corresponds to `st_ctim` on Unix and `ChangeTime` on Windows).
    pub change_time: SystemTime,

    /// The creation time of the file (corresponds to `CreationTime` on Windows, ignored on Unix).
    pub creation_time: SystemTime,
}

/// Result type for [`stat`].
///
/// [`stat`]: StreamFactory::stat
#[derive(Debug, Clone)]
pub enum StatResult {
    /// Passes through the call to the underlying filesystem.
    Passthrough,

    /// Tells the caller that the file doesn't exist.
    DoesntExist,

    /// Tells the caller that the file exists and has the given properties.
    Exists(Stat),
}

/// Factory trait for [`Stream`].
///
/// Paths provided to methods in this trait will always be absolute paths relative to one of the
/// base directories passed to [`filesilly::init`].
///
/// [`filesilly::init`]: init
pub trait StreamFactory: Send + Sync + Debug {
    /// Gets information on a file without opening it.
    ///
    /// # Returns
    /// Information on the provided path if available, or [`Passthrough`] to use the OS result.
    ///
    /// [`Passthrough`]: StatResult::Passthrough
    fn stat(&self, path: &Path) -> io::Result<StatResult>;

    /// Opens a path as a stream.
    ///
    /// # Returns
    /// A stream to use, or `None` to pass the file through to the OS.
    fn open(&self, path: &Path) -> io::Result<Option<HeapStream>>;
}

/// Initializes API hooks.
///
/// # Panics
/// Panics if called multiple times. Use a `Once` if this is a concern.
///
/// # Errors
/// This function errors if any part of initialization fails (e.g. hook placement can fail).
pub fn init(base_paths: &[&Path], factory: Box<dyn StreamFactory>) -> Result<()> {
    os::init(base_paths, factory)
}
