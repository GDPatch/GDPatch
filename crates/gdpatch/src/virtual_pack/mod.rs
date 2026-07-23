pub mod builder;

use memmap2::Mmap;
use std::cmp::min;
use std::sync::Arc;
use std::{io, slice};
use tracing::trace;

/// Contents for a [`VirtualPackEntry`].
#[derive(Debug, Clone)]
pub enum FileContents {
    /// An on-disk entry represented by a `mmap`'d file.
    Disk {
        /// The file mapping. Stored as an `Arc` to prevent having to have multiple handles to the
        /// same file.
        mapping: Arc<Mmap>,

        /// Start offset within the mapping. Note that this is not the same as the offset within the
        /// virtual pack file.
        offset: u64,

        /// Length of this entry, in bytes.
        len: u64,
    },

    /// A purely in-memory entry.
    Memory(Vec<u8>),
}

impl FileContents {
    pub fn as_slice(&self) -> &[u8] {
        match &self {
            FileContents::Disk {
                mapping,
                offset,
                len,
            } => unsafe {
                // SAFETY: The mapping is guaranteed to be valid for all of its files by this
                // type's constructors.
                let ptr = mapping.as_ptr().add(*offset as usize);
                slice::from_raw_parts(ptr, *len as usize)
            },
            FileContents::Memory(data) => data,
        }
    }

    pub fn len(&self) -> u64 {
        match &self {
            FileContents::Disk { len, .. } => *len,
            FileContents::Memory(data) => data.len() as u64,
        }
    }
}

#[derive(Debug)]
pub struct VirtualPackEntry {
    /// Offset of this entry from the file base (end of directory).
    offset: u64,

    /// Contents of this entry.
    contents: FileContents,
}

impl VirtualPackEntry {
    /// Returns the length of this entry in bytes.
    pub fn len(&self) -> u64 {
        match &self.contents {
            FileContents::Disk { len, .. } => *len,
            FileContents::Memory(buf) => buf.len() as u64,
        }
    }

    /// Reads some data from this entry. The position is relative to the start of this entry.
    pub fn read(&self, position: usize, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.contents.as_slice();
        let data = &data[position..];
        let to_copy = min(buf.len(), data.len());
        buf[..to_copy].copy_from_slice(&data[..to_copy]);
        Ok(to_copy)
    }
}

/// A virtual pack file. Contains real files (on disk) as well as in-memory ones.
#[derive(Debug)]
pub struct VirtualPack {
    /// The offset of the pack header within the file. The majority of pack files will have this
    /// set to 0. Embedded pack files have this set to their offset within the executable. Reads
    /// are relative to the header, NOT this position.
    header_pos_within_file: u64,

    /// Header and file directory data. This is precomputed at construction time.
    header: Vec<u8>,

    /// Entries within this pack. This list is sorted by the offset of each entry within the pack,
    /// so reads can binary search the
    entries: Vec<VirtualPackEntry>,
}

impl VirtualPack {
    pub fn new(
        header_pos_within_file: u64,
        header: Vec<u8>,
        mut entries: Vec<VirtualPackEntry>,
    ) -> Self {
        entries.sort_by_key(|entry| entry.offset);

        Self {
            header_pos_within_file,
            header,
            entries,
        }
    }

    pub fn read(&self, position: u64, buf: &mut [u8]) -> io::Result<usize> {
        let Some(position_within_files) = position.checked_sub(self.header.len() as u64) else {
            // Position is within the header.
            trace!(%position, "header/directory read");

            let header_slice = &self.header[position as usize..];
            let to_copy = min(buf.len(), header_slice.len());
            buf[..to_copy].copy_from_slice(&header_slice[..to_copy]);
            return Ok(to_copy);
        };

        // Find the entry we're looking for by binary search. This returns the position of the first
        // element that has an offset *greater* than the one we asked for, so in effect it returns
        // the position of the entry we want plus one.
        let index_of_next_entry = self
            .entries
            .partition_point(|entry| entry.offset <= position_within_files);

        if index_of_next_entry == 0 {
            // The read requested a position before the first entry, but after the start of the
            // file base. Fill the buffer with zeros up to the first entry.
            trace!(%position, "before first file");
        } else {
            let entry = &self.entries[index_of_next_entry - 1];
            let position_within_entry = position_within_files - entry.offset;

            if position_within_entry >= entry.len() {
                // Position is past the end of this entry but before the start of the next one,
                // so they asked for data in the gap between this entry and the next one.
                trace!(%position, "in gap");
            } else {
                // Position is within the bounds of the entry. Serve some data!
                trace!(%position, %position_within_entry, "reading an entry");
                return entry.read(position_within_entry as usize, buf);
            }
        }

        let position_to_fill_to = if let Some(next_entry) = self.entries.get(index_of_next_entry) {
            // Fill with zeros up to the position of the next entry.
            trace!(%position, "unmapped (between entries or before first entry)");
            next_entry.offset + self.header.len() as u64
        } else {
            // They've asked for an entry past the end of our file. Tell them we have nothing there.
            trace!(%position, "unmapped (after last entry)");
            return Ok(0);
        };

        // Fill output with zeros until the next data we have.
        let distance_to_next_data = (position_to_fill_to - position) as usize;
        let to_copy = min(distance_to_next_data, buf.len());
        buf[..to_copy].fill(0);
        Ok(to_copy)
    }

    pub fn header_pos_within_file(&self) -> u64 {
        self.header_pos_within_file
    }
}
