use crate::os::FileHandle;
use parking_lot::ReentrantMutex;
use std::cell::RefCell;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::LazyLock;
use std::{collections::HashMap, io, mem};
use thiserror::Error;

mod os;

#[derive(Error, Debug)]
pub enum Error {
    #[error("system API error {} ({})", .code, .message)]
    System { code: u32, message: String },

    #[error("failed to place function hook")]
    Hook,

    #[error("IO error")]
    IO(#[from] io::Error),

    #[error("unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Trait returned by [`StreamFactory`] to allow proxying game file reads/writes.
pub trait Stream: Read + Write + Seek + Send {}

impl Stream for std::fs::File {}

/// Factory trait for [`Stream`].
pub trait StreamFactory: Send {
    /// Creates a stream for a path.
    ///
    /// # Returns
    /// A stream to use, or `None` to pass the file through to the OS.
    fn create_stream(&mut self, path: &Path) -> io::Result<Option<Box<dyn Stream>>>;
}

#[derive(Default)]
struct HandleStore {
    /// Factory for new streams. Will be `None` if unset.
    factory: Option<Box<dyn StreamFactory>>,

    /// Currently open streams.
    streams: HashMap<FileHandle, Box<dyn Stream>>,
}

impl HandleStore {
    /// Creates a stream and returns its handle. [`None`] is returned if the factory didn't
    /// give us a stream.
    pub(crate) fn create(&mut self, path: &Path) -> io::Result<Option<FileHandle>> {
        let Some(factory) = &mut self.factory else {
            return Ok(None);
        };

        let Some(stream) = factory.create_stream(path)? else {
            return Ok(None);
        };

        let file_handle = self.allocate_file_handle();
        self.streams.insert(file_handle, stream);
        Ok(Some(file_handle))
    }

    /// Gets a stream by its handle.
    pub(crate) fn get_stream(&mut self, handle: FileHandle) -> Option<&mut dyn Stream> {
        match self.streams.get_mut(&handle) {
            None => None,
            Some(stream) => Some(&mut **stream),
        }
    }

    /// Closes a handle and returns the stream.
    pub(crate) fn close(&mut self, handle: FileHandle) -> Option<Box<dyn Stream>> {
        self.streams.remove(&handle)
    }
}

static HANDLE_STORE: LazyLock<ReentrantMutex<RefCell<HandleStore>>> =
    LazyLock::new(Default::default);

/// Initializes the API hooks. This is safe to call multiple times.
///
/// # Errors
/// This function errors if any part of initialization fails (e.g. hook placement can fail).
pub fn init() -> Result<()> {
    os::init()
}

/// Sets a new factory, replacing an existing one if set.
pub fn set(factory: Box<dyn StreamFactory>) -> Option<Box<dyn StreamFactory>> {
    let store = HANDLE_STORE.lock();
    let mut store = store.borrow_mut();
    store.factory.replace(factory)
}

/// Unsets the factory. File system operations from this point will be passed through (existing
/// files retain their existing behavior).
pub fn unset() -> Option<Box<dyn StreamFactory>> {
    let store = HANDLE_STORE.lock();
    let mut store = store.borrow_mut();
    mem::take(&mut store.factory)
}
