use crate::virtual_pack::{FileContents, VirtualPack, VirtualPackEntry};
use gdpatch_godot::pack::{Pack, PackBuilder, PackedFile};
use indexmap::IndexMap;

/// Arbitrary alignment for files in packs. At least on Windows the userspace API tends to fetch
/// 4KB blocks at a time so this value seems appropriate.
const PACK_ALIGNMENT: u64 = 4096 * 2;

pub struct VirtualPackBuilder {
    builder: PackBuilder,
    entries: IndexMap<u64, FileContents>,
    pack_offset: u64,
}

impl VirtualPackBuilder {
    /// Create a new [`VirtualPackBuilder`] using the same format as the [`old_pack`].
    pub fn new(old_pack: &Pack) -> Self {
        Self {
            builder: PackBuilder::new(old_pack.format, old_pack.engine_version),
            entries: Default::default(),
            pack_offset: 0,
        }
    }

    // TODO: recalculate hash probably, all of the hashes we pass are wrong
    pub fn add_file(&mut self, path: String, size: u64, hash: [u8; 16], contents: FileContents) {
        // If we're overwriting a file, remove the old backing entry.
        if let Some(old_contents) = self.builder.files.get(&path) {
            self.entries.shift_remove(&old_contents.offset);
        }

        self.builder.files.insert(
            path,
            PackedFile {
                encrypted: false,
                offset: self.pack_offset,
                size,
                hash,
            },
        );

        self.entries.insert(self.pack_offset, contents);

        self.pack_offset = (self.pack_offset + size).next_multiple_of(PACK_ALIGNMENT);
    }

    /// Consume this instance and return the built [`VirtualPack`].
    pub fn build(self, header_pos_within_file: u64) -> VirtualPack {
        let header = self.builder.encode(header_pos_within_file);

        let entries = self
            .entries
            .into_iter()
            .map(|(offset, contents)| VirtualPackEntry { offset, contents })
            .collect();

        VirtualPack::new(header_pos_within_file, header, entries)
    }
}
