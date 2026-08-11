//! Godot PCK file format parsers etc.

mod builder;

use crate::Error;
use byteorder::{LittleEndian, ReadBytesExt};
use core::str;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use tracing::error;

pub use self::builder::PackBuilder;

const PACK_DIR_ENCRYPTED: u32 = 1 << 0;
const PACK_REL_FILEBASE: u32 = 1 << 1;
const PACK_SPARSE_BUNDLE: u32 = 1 << 2;

const PACK_FILE_ENCRYPTED: u32 = 1 << 0;
const PACK_FILE_REMOVAL: u32 = 1 << 1;
const PACK_FILE_DELTA: u32 = 1 << 2;

/// Extra unconventional settings for parsing a pack file.
///
/// Some engine builds modify the pack format in a way that requires special workarounds. This is a problem for our
/// build catalog system, as it uses the pack header to determine the engine version automatically. To avoid requiring
/// the user to specify the game version directly, we store some settings about how to parse the pack file independently
/// from the build configs, so we can parse the pack before resolving an engine version.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct PackConfig {
    /// The magic constant present in the header of pack files.
    pck_header_magic: Option<String>,
}

impl PackConfig {
    // defaults are implemented here so we don't have to look them up in the build catalog; it's a little weird but it works
    pub fn header_magic(&self) -> u32 {
        const PACK_HEADER_MAGIC: u32 = 0x43504447; // "GDPC" in ASCII
        self.pck_header_magic
            .clone()
            .map(|s| {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&s.as_bytes()[..4]);
                u32::from_le_bytes(bytes)
            })
            .unwrap_or(PACK_HEADER_MAGIC)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum PackFormat {
    V2 = 2,
    V3 = 3,
    V4 = 4,
}

#[derive(Debug, Clone)]
pub struct PackedFile {
    /// Whether this file is encrypted.
    pub encrypted: bool,

    /// Offset within the PCK file. This offset is relative to the pack's file base (which may
    /// be relative to the header depending on the pack format).
    pub offset: u64,

    /// Size of this file in bytes.
    pub size: u64,

    /// Hash of the contents of this file. Godot uses MD5.
    pub hash: [u8; 16],
}

#[derive(Debug)]
pub struct Pack {
    /// The version of the pack format this pack is using.
    pub format: PackFormat,

    /// The engine version. This has to match the target engine version for the pack to load.
    pub engine_version: (u32, u32, u32),

    /// A salt used for pack encryption.
    pub salt: Option<[u8; 32]>,

    /// The offset of files within the pack relative to the header.
    pub file_base: u64,

    /// A list of files within this pack.
    pub files: IndexMap<String, PackedFile>,

    /// A list of deltas to apply to files within the pack, keyed by the file hash.
    pub deltas: IndexMap<[u8; 16], Vec<PackedFile>>,
}

impl Pack {
    // TODO: rewrite this to use marshalling
    pub fn parse<R>(mut f: R, config: impl Into<PackConfig>) -> crate::Result<Self>
    where
        R: Read + Seek,
    {
        let config = config.into();
        let pck_start_pos = f.stream_position()?;

        let magic = f.read_u32::<LittleEndian>()?;

        if magic != config.header_magic() {
            return Err(Error::BadData);
        }

        let format = f.read_u32::<LittleEndian>()?;
        let engine_major = f.read_u32::<LittleEndian>()?;
        let engine_minor = f.read_u32::<LittleEndian>()?;
        let engine_patch = f.read_u32::<LittleEndian>()?;

        let format = match format {
            2 => PackFormat::V2,
            3 => PackFormat::V3,
            4 => PackFormat::V4,
            _ => return Err(Error::UnknownVersion(format)),
        };

        let pack_flags = f.read_u32::<LittleEndian>()?;
        let enc_directory = (pack_flags & PACK_DIR_ENCRYPTED) != 0;
        let rel_filebase = (pack_flags & PACK_REL_FILEBASE) != 0; // Note: Always enabled for V3.
        let sparse_bundle = (pack_flags & PACK_SPARSE_BUNDLE) != 0;

        let mut file_base = f.read_u64::<LittleEndian>()?;
        if format == PackFormat::V4
            || format == PackFormat::V3
            || (format == PackFormat::V2 && rel_filebase)
        {
            file_base += pck_start_pos;
        }

        let mut salt = None;

        if format == PackFormat::V3 || format == PackFormat::V4 {
            // V3/V4: Read directory offset and skip reserved part of the header.
            let dir_offset = f.read_u64::<LittleEndian>()?;

            if sparse_bundle && enc_directory && format == PackFormat::V4 {
                // V4: Read encrypted directory salt.
                let mut salt_data = [0u8; 32];
                f.read_exact(&mut salt_data)?;
                salt = Some(salt_data);
            }

            f.seek(SeekFrom::Start(dir_offset + pck_start_pos))?;
        } else if format == PackFormat::V2 {
            // V2: Directory directly after the header.
            f.seek_relative(16 * 4)?;
        }

        // Read directory.
        let file_count = f.read_u32::<LittleEndian>()?;

        if enc_directory {
            panic!("Games using encryption are currently unsupported");
        }

        let mut files = IndexMap::new();
        let mut deltas = IndexMap::new();

        for _ in 0..file_count {
            let path_length = f.read_u32::<LittleEndian>()? as usize;
            let mut path_bytes = vec![0u8; path_length];
            f.read_exact(&mut path_bytes)?;
            let path = str::from_utf8(&path_bytes)
                .map_err(|err| {
                    error!(?err, "invalid utf-8 in PCK file path");
                    Error::BadData
                })?
                .trim_end_matches('\0')
                .to_string();

            let offset = f.read_u64::<LittleEndian>()?;
            let size = f.read_u64::<LittleEndian>()?;

            let mut hash = [0u8; 16];
            f.read_exact(&mut hash)?;

            let flags = f.read_u32::<LittleEndian>()?;
            let encrypted = (flags & PACK_FILE_ENCRYPTED) != 0;
            let removal = (flags & PACK_FILE_REMOVAL) != 0;
            let delta = (flags & PACK_FILE_DELTA) != 0;

            if removal {
                unimplemented!("implement removals")
            } else {
                let file = PackedFile {
                    encrypted,
                    offset: file_base + offset,
                    size,
                    hash,
                };

                if delta {
                    deltas.entry(hash).or_insert_with(Vec::new).push(file);
                } else {
                    files.insert(path.clone(), file);
                }
            }
        }

        Ok(Self {
            format,
            engine_version: (engine_major, engine_minor, engine_patch),
            salt,
            file_base,
            files,
            deltas,
        })
    }
}
