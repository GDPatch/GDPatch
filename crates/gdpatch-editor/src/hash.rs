//! Murmur3 implementation used by Godot.

use std::num::Wrapping;

// Murmurhash3 32-bit version.
// All MurmurHash versions are public domain software, and the author disclaims all copyright to their code.
const HASH_MURMUR3_SEED: u32 = 0x7F07C65;

pub struct MurmurHash3(Wrapping<u32>);

impl MurmurHash3 {
    pub const fn new() -> Self {
        Self(Wrapping(HASH_MURMUR3_SEED))
    }

    pub fn hash(&mut self, value: u32) {
        let mut value = Wrapping(value);
        value *= 0xcc9e2d51;
        value = (value << 15) | (value >> 17);
        value *= 0x1b873593;

        self.0 ^= value;
        self.0 = (self.0 << 13) | (self.0 >> 19);
        self.0 = self.0 * Wrapping(5) + Wrapping(0xe6546b64);
    }

    pub fn finish(&self) -> u32 {
        let mut hash = self.0;
        hash ^= hash >> 16;
        hash *= 0x85ebca6b;
        hash ^= hash >> 13;
        hash *= 0xc2b2ae35;
        hash ^= hash >> 16;

        hash.0
    }
}

impl Default for MurmurHash3 {
    fn default() -> Self {
        Self::new()
    }
}