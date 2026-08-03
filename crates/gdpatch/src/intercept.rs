//! Lower-level filesystem interception implementation.
use crate::GDPatch;
use crate::virtual_pack::VirtualPack;
use color_eyre::eyre::eyre;
use filesilly::{Stream, StreamFactory};
use gdpatch_godot::pack::{Pack, PackConfig};
use std::env::{current_dir, current_exe};
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{io, mem};
use tracing::{debug, error, trace, warn};

/// An open handle to a virtual pack. Stores a seek position and forwards reads to the inner pack.
#[derive(Debug)]
struct OpenVirtualPack {
    /// Current seek position within the virtual pack.
    pos: u64,

    /// The virtual pack to read from.
    inner: Arc<VirtualPack>,
}

impl OpenVirtualPack {
    pub fn new(inner: Arc<VirtualPack>) -> Self {
        Self { pos: 0, inner }
    }
}

impl Seek for OpenVirtualPack {
    // NOTE: The positions expected by the caller to the seek APIs on this file are absolute within
    // the pack, but for consistency we want to always work with positions relative to the pack
    // header. To facilitate this we offset all reads by the position of the header within the file,
    // hence the pile of match statements below.
    fn seek(&mut self, style: SeekFrom) -> io::Result<u64> {
        let header_pos = self.inner.header_pos_within_file();
        trace!(?style, %header_pos, "seeking to position in pack");

        match style {
            SeekFrom::Start(n) => match n.checked_sub(header_pos) {
                Some(n) => {
                    self.pos = n;
                    Ok(self.pos + header_pos)
                }
                None => {
                    error!("game tried to seek before PCK file header");
                    Err(io::Error::new(
                        ErrorKind::InvalidInput,
                        "tried to seek before PCK header",
                    ))
                }
            },
            SeekFrom::End(_) => {
                error!("game tried to seek from end of PCK");
                Err(io::Error::new(
                    ErrorKind::Unsupported,
                    "seeking from end is unsupported",
                ))
            }
            SeekFrom::Current(n) => match self
                .pos
                .checked_add_signed(n)
                .and_then(|pos| pos.checked_add(header_pos))
            {
                Some(n) => {
                    self.pos = n - header_pos;
                    Ok(n)
                }
                None => Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "invalid seek to a negative or overflowing position",
                )),
            },
        }
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        Ok(self
            .pos
            .checked_add(self.inner.header_pos_within_file())
            .expect("position overflowed"))
    }
}

impl Read for OpenVirtualPack {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(self.pos, buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

#[derive(Debug)]
enum PackStreamInner {
    /// Invalid state caused by a panic.
    Invalid,

    /// No operations have been performed yet, can't conclude.
    Unknown {
        /// Path of the file being operated on.
        path: PathBuf,

        /// The file being operated on.
        file: File,

        /// The config to parse the pack file with.
        config: PackConfig,

        // This should probably be changed to a more robust detection method, but we assume that
        // the file we're reading isn't a pack after a certain amount of reads without a pack
        // header. Ccurrently this threshold is set to 16 reads.
        read_count: usize,
    },

    /// Some other non-pack file.
    SomethingElse(File),

    /// A virtual pack file.
    Virtual(OpenVirtualPack),
}

/// [`Stream`] implementation used for files that might be packs.
#[derive(Debug)]
struct PackStream(PackStreamInner);

impl PackStream {
    /// Creates a new [`PackStream`] in an unknown state.
    pub fn new(path: PathBuf, file: File, config: PackConfig) -> PackStream {
        PackStream(PackStreamInner::Unknown {
            path,
            file,
            config,
            read_count: 0,
        })
    }

    /// Creates a new [`PackStream`] that emulates pack reads from the given virtual pack.
    pub fn new_virtual(pack: Arc<VirtualPack>) -> PackStream {
        let inner = OpenVirtualPack::new(pack);
        PackStream(PackStreamInner::Virtual(inner))
    }
}

impl Read for PackStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match mem::replace(&mut self.0, PackStreamInner::Invalid) {
            PackStreamInner::Invalid => panic!("invalid state set in pck intercept stream"),
            PackStreamInner::Unknown {
                path,
                mut file,
                config,
                mut read_count,
            } => {
                let offset = file.stream_position()?;
                let file_size = file.stream_len()?;
                match file.read(buf) {
                    Err(err) => Err(err),
                    Ok(n) => {
                        read_count += 1;

                        if read_count > 64 {
                            self.0 = PackStreamInner::SomethingElse(file);
                            return Ok(n);
                        }

                        // On some platforms (Linux w/ self-contained .pcks), the magic is read one byte at a time.
                        let possible_magic: Option<u32> = if n >= 4 {
                            buf[..4].try_into().ok().map(u32::from_le_bytes)
                        } else if offset + 4 < file_size {
                            trace!("possible short read on magic");
                            let offset_after_read = file.stream_position()?;

                            file.seek(SeekFrom::Start(offset))?;
                            let mut buf = [0u8; 4];
                            let result = match file.read_exact(&mut buf[..]) {
                                Ok(()) => Some(u32::from_le_bytes(buf)),
                                Err(_) => None,
                            };

                            file.seek(SeekFrom::Start(offset_after_read))?;

                            result
                        } else {
                            None
                        };

                        if let Some(possible_magic) = possible_magic
                            && possible_magic == config.header_magic()
                        {
                            file.seek(SeekFrom::Start(offset))?;
                            match Pack::parse(&mut file, config.clone()) {
                                Ok(pack) => {
                                    debug!(file_count = %pack.files.len(), "found PCK file!");

                                    let gdpatch = GDPatch::instance();
                                    let virtual_pack =
                                        gdpatch.create_virtual_pack(path, file, pack, offset);

                                    self.0 = PackStreamInner::Virtual(OpenVirtualPack::new(
                                        virtual_pack,
                                    ));

                                    // rerun read on PCK file
                                    return self.read(buf);
                                }
                                Err(err) => {
                                    file.seek(SeekFrom::Start(offset + n as u64))?;
                                    warn!(
                                        ?err,
                                        path = %path.display(),
                                        "failed to parse likely PCK file"
                                    );
                                }
                            }
                        }

                        self.0 = PackStreamInner::Unknown {
                            path,
                            file,
                            config,
                            read_count,
                        };
                        Ok(n)
                    }
                }
            }

            // Unknown file type, we have to keep proxying operations to it.
            PackStreamInner::SomethingElse(mut file) => {
                let res = file.read(buf);
                self.0 = PackStreamInner::SomethingElse(file);
                res
            }

            // Virtual pack, pass the read on.
            PackStreamInner::Virtual(mut pack) => {
                let res = pack.read(buf);
                self.0 = PackStreamInner::Virtual(pack);
                res
            }
        }
    }
}

impl Write for PackStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.0 {
            PackStreamInner::Invalid => panic!("invalid state set in pck intercept stream"),
            PackStreamInner::Unknown { file, .. } | PackStreamInner::SomethingElse(file) => {
                file.write(buf)
            }

            // Godot never writes to its pack file in official builds.
            PackStreamInner::Virtual(_) => {
                warn!("game tried to write to its pack file");
                let error = eyre!("can't write to PCK files");
                Err(io::Error::other(error))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.0 {
            PackStreamInner::Invalid => panic!("invalid state set in pck intercept stream"),
            PackStreamInner::Unknown { file, .. } | PackStreamInner::SomethingElse(file) => {
                file.flush()
            }

            // Godot never writes to its pack file in official builds.
            PackStreamInner::Virtual(_) => {
                warn!("game tried to flush its pack file");
                let error = eyre!("can't flush PCK files");
                Err(io::Error::other(error))
            }
        }
    }
}

impl Seek for PackStream {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match &mut self.0 {
            PackStreamInner::Invalid => panic!("invalid state set in pck intercept stream"),
            PackStreamInner::Unknown { file, .. } | PackStreamInner::SomethingElse(file) => {
                file.seek(pos)
            }
            PackStreamInner::Virtual(pack) => pack.seek(pos),
        }
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        match &mut self.0 {
            PackStreamInner::Invalid => panic!("invalid state set in pck intercept stream"),
            PackStreamInner::Unknown { file, .. } | PackStreamInner::SomethingElse(file) => {
                file.stream_position()
            }
            PackStreamInner::Virtual(pack) => pack.stream_position(),
        }
    }
}

impl Stream for PackStream {}

/// Main entrypoint for GDPatch patching functionality. Handles patching Godot packs in memory, as
/// well as direct GDScript communication via special paths.
#[derive(Debug)]
pub struct GDPatchStreamFactory(pub PackConfig);

impl StreamFactory for GDPatchStreamFactory {
    fn create_stream(&mut self, path: &Path) -> io::Result<Option<Box<dyn Stream>>> {
        // Redirect to the IPC stream if needed.
        if path == crate::ipc::IPC_FILENAME
            || current_dir()
                .map(|d| path == d.join(crate::ipc::IPC_FILENAME))
                .unwrap_or_default()
        {
            return Ok(Some(Box::new(crate::ipc::IpcStream::new())));
        }

        let gdpatch = GDPatch::instance();

        // Prevent proxying anything in GDPatch's root directory.
        // If this wasn't here, mod .pck files would fail to load.
        if path.starts_with(&gdpatch.root_directory) {
            return Ok(None);
        }

        // Check for paths we already know to be pack files.
        if let Some(pack) = gdpatch.get_virtual_pack(path) {
            // Path is a known pack file, just return a reference to its virtual pack.
            let stream = PackStream::new_virtual(pack);
            return Ok(Some(Box::new(stream)));
        }

        // Godot will try and read a pack from either the executable itself, or from a separate
        // .pck file.
        let asked_for_pck = path.to_string_lossy().ends_with(".pck");
        let asked_for_exe = current_exe().map(|exe| path == exe).unwrap_or(false);

        if asked_for_pck || asked_for_exe {
            let file = File::open(path)?;
            let path = path.to_owned();
            return Ok(Some(Box::new(PackStream::new(path, file, self.0.clone()))));
        }

        Ok(None)
    }
}
