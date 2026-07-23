//! Godot binary marshaling functions.

use crate::Error::TooShort;
use crate::variant::Real;
use std::ops::{Deref, DerefMut};

pub const ENCODED_OBJECT_AS_ID_CLASS_NAME: &str = "EncodedObjectAsID";

/// Buffer for reading Godot's marshaled types.
///
/// Maintains an internal current offset and a marked offset. The marked offset is updated to the
/// current offset when [`update_marked_offset`] is called. The current offset is reset to the
/// marked offset when a decoder function returns an error.
#[derive(Debug)]
pub struct ReadableMarshalBuffer<'buf> {
    buffer: &'buf [u8],

    disable_marking: bool,

    /// The real read offset within the buffer.
    offset: usize,

    /// The marked offset, mimicking the Godot behavior for error handling reasons.
    marked_offset: usize,
}

impl<'buf> ReadableMarshalBuffer<'buf> {
    /// Creates a new [`ReadableMarshalBuffer`].
    ///
    /// # Parameters
    /// - `buffer`: the underlying buffer to read from
    /// - `disable_marking`: disables the marking functionality
    pub fn new(buffer: &'buf [u8], disable_marking: bool) -> Self {
        Self {
            buffer,
            disable_marking,
            offset: 0,
            marked_offset: 0,
        }
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.offset
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the contents of the buffer after the current offset.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer[self.offset..]
    }

    /// Returns the full contents of the buffer.
    pub fn full_buffer(&self) -> &[u8] {
        self.buffer
    }

    /// Ensures the buffer has enough space.
    ///
    /// Resets the current offset to the mark and returns [`TooShort`] if there's not enough data.
    pub fn ensure_remaining(&mut self, minimum: usize) -> crate::Result<()> {
        if minimum > self.remaining() {
            self.reset_to_mark();
            Err(TooShort)
        } else {
            Ok(())
        }
    }

    /// Adds `length` to the current offset.
    ///
    /// Note that this does not change the marked offset.
    pub fn advance(&mut self, length: usize) {
        self.offset += length;
    }

    /// Updates the marked offset to be equal to the current offset.
    pub fn mark(&mut self) {
        if !self.disable_marking {
            self.marked_offset = self.offset;
        }
    }

    /// Resets the current offset to the last marked offset.
    pub fn reset_to_mark(&mut self) {
        if !self.disable_marking {
            self.offset = self.marked_offset;
        }
    }

    pub fn decode_uint32(&mut self) -> crate::Result<u32> {
        self.ensure_remaining(4)?;
        let arr: [u8; 4] = self.buffer[self.offset..][..4].try_into().unwrap();
        self.offset += 4;

        Ok(u32::from_le_bytes(arr))
    }

    pub fn decode_uint64(&mut self) -> crate::Result<u64> {
        self.ensure_remaining(8)?;
        let arr: [u8; 8] = self.buffer[self.offset..][..8].try_into().unwrap();
        self.offset += 8;

        Ok(u64::from_le_bytes(arr))
    }
    pub fn decode_float(&mut self) -> crate::Result<f32> {
        self.ensure_remaining(4)?;
        let arr: [u8; 4] = self.buffer[self.offset..][..4].try_into().unwrap();
        self.offset += 4;

        Ok(f32::from_le_bytes(arr))
    }

    pub fn decode_double(&mut self) -> crate::Result<f64> {
        self.ensure_remaining(8)?;
        let arr: [u8; 8] = self.buffer[self.offset..][..8].try_into().unwrap();
        self.offset += 8;

        Ok(f64::from_le_bytes(arr))
    }

    pub fn decode_real(&mut self, as_64: bool) -> crate::Result<f64> {
        if as_64 {
            self.decode_double()
        } else {
            self.decode_float().map(|f| f as f64)
        }
    }

    pub fn decode_slice(&mut self, length: usize) -> crate::Result<&[u8]> {
        self.ensure_remaining(length)?;
        let buf = &self.buffer[self.offset..][..length];
        self.offset += length;
        Ok(buf)
    }
}

/// Buffer for writing Godot's marshaled types.
///
/// Works the same internally as `ReadableMarshalBuffer`.
#[derive(Debug)]
pub struct WritableMarshalBuffer {
    buffer: Vec<u8>,
    write_reals_as_64_bit: bool,
}

impl WritableMarshalBuffer {
    /// Creates a new [`WritableMarshalBuffer`].
    ///
    /// # Parameters
    /// - `write_reals_as_64_bit`: whether to write `real_t` types as floats or doubles.
    pub fn new(write_reals_as_64_bit: bool) -> Self {
        Self {
            buffer: Vec::new(),
            write_reals_as_64_bit,
        }
    }

    /// Creates a new [`WritableMarshalBuffer`] from another.
    ///
    /// Copies the `write_reals_as_64_bit` flag from the passed buffer.
    pub fn new_from(other: &WritableMarshalBuffer) -> Self {
        Self::new(other.write_reals_as_64_bit)
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn write_reals_as_64_bit(&self) -> bool {
        self.write_reals_as_64_bit
    }

    /// Returns the contents of the buffer.
    pub fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buffer
    }

    pub fn push(&mut self, v: u8) {
        self.buffer.push(v)
    }

    pub fn encode_uint32(&mut self, v: u32) {
        self.buffer.extend_from_slice(&v.to_le_bytes())
    }

    pub fn encode_uint64(&mut self, v: u64) {
        self.buffer.extend_from_slice(&v.to_le_bytes())
    }

    pub fn encode_float(&mut self, v: f32) {
        self.buffer.extend_from_slice(&v.to_le_bytes())
    }

    pub fn encode_double(&mut self, v: f64) {
        self.buffer.extend_from_slice(&v.to_le_bytes())
    }

    pub fn encode_real(&mut self, v: Real) {
        if self.write_reals_as_64_bit {
            self.encode_double(v.into());
        } else {
            self.encode_float(f64::from(v) as f32);
        }
    }
}

impl Deref for WritableMarshalBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for WritableMarshalBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
