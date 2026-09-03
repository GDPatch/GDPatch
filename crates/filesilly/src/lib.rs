//! Userspace filesystem overlay using function detouring.
use parking_lot::Mutex;
use std::fmt::Debug;
use std::io;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::{Arc, OnceLock};
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
}

pub type Result<T> = std::result::Result<T, Error>;

/// Trait returned by [`StreamFactory`] to allow proxying game file reads/writes.
pub trait Stream: Read + Write + Seek + Send + Debug {}
impl Stream for std::fs::File {}

/// Factory trait for [`Stream`].
pub trait StreamFactory: Send + Sync + Debug {
    /// Creates a stream for a path. The path will always be a path relative to the base directory
    /// passed to [`filesilly::init`].
    ///
    /// [`filesilly::init`]: init
    ///
    /// # Returns
    /// A stream to use, or `None` to pass the file through to the OS.
    fn create_stream(&self, path: &Path) -> io::Result<Option<HeapStream>>;
}

/// Initializes API hooks.
///
/// # Panics
/// Panics if called multiple times. Use a `Once` if this is a concern.
///
/// # Errors
/// This function errors if any part of initialization fails (e.g. hook placement can fail).
pub fn init(base_path: &Path, factory: Box<dyn StreamFactory>) -> Result<()> {
    os::init(base_path, factory)
}
