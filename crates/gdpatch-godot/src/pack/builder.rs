use crate::WritableMarshalBuffer;
use crate::pack::{PackConfig, PackFormat, PackedFile};
use indexmap::IndexMap;

#[derive(Debug)]
pub struct PackBuilder {
    format: PackFormat,
    engine_version: (u32, u32, u32),
    pub files: IndexMap<String, PackedFile>,
}

impl PackBuilder {
    pub fn new(format: PackFormat, engine_version: (u32, u32, u32)) -> PackBuilder {
        PackBuilder {
            format,
            engine_version,
            files: Default::default(),
        }
    }

    /// Encodes this pack's file directory into a buffer.
    fn encode_directory(&self) -> Vec<u8> {
        // Write out directory.
        let mut directory = WritableMarshalBuffer::new(false);
        directory.encode_uint32(self.files.len() as u32);

        for (path, file) in &self.files {
            directory.encode_uint32(path.len() as u32);
            directory.buffer().extend_from_slice(path.as_bytes());
            directory.encode_uint64(file.offset);
            directory.encode_uint64(file.size);
            directory.buffer().extend_from_slice(&file.hash);

            // no flags
            directory.encode_uint32(0);
        }

        directory.into_inner()
    }

    /// Encodes this pack's header and directory into a buffer.
    ///
    /// The `header_pos_within_file` parameter is only used for encoding V2 format pack files,
    /// where the encoded header file base is absolute instead of relative. This method assumes
    /// all files will be written directly after the end of the directory.
    pub fn encode(&self, config: impl Into<PackConfig>, header_pos_within_file: u64) -> Vec<u8> {
        let config = config.into();
        let directory = self.encode_directory();

        let mut header = WritableMarshalBuffer::new(false);
        header.encode_uint32(config.header_magic());

        header.encode_uint32(self.format as u32);
        header.encode_uint32(self.engine_version.0);
        header.encode_uint32(self.engine_version.1);
        header.encode_uint32(self.engine_version.2);

        // flags
        header.encode_uint32(0);

        let file_base_pos = header.len();
        header.encode_uint64(0);

        match self.format {
            PackFormat::V3 | PackFormat::V4 => {
                // our directory is always written directly after our header, so the directory offset is
                // set to the size of the header (40 bytes + the 32 byte salt, if present)
                header.encode_uint64(40);

                // TODO: write salt
            }
            PackFormat::V2 => {
                // V2: Directory directly after the header.
                header.buffer().extend_from_slice(&[0; 16 * 4]);
            }
        };

        // rewrite file base
        let mut file_base = header.len() as u64 + directory.len() as u64;

        // For V2 format packs, the file base is absolute. For later versions, the file base is
        // relative to the start of the header. Godot uses a flag for V2 packs, PACK_REL_FILEBASE,
        // but that doesn't exist in all V2 versions, so we cannot use it.
        if self.format == PackFormat::V2 {
            file_base += header_pos_within_file;
        }

        header[file_base_pos..file_base_pos + 8].copy_from_slice(&file_base.to_le_bytes());

        header.buffer().extend_from_slice(&directory);
        header.into_inner()
    }
}
