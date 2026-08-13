//! Godot project settings parser.
use crate::marshalling::{ReadableMarshalBuffer, WritableMarshalBuffer};
use crate::variant::Variant;
use indexmap::IndexMap;

/// Godot's binary project settings magic ("ECFG" in ASCII).
const BINARY_MAGIC: u32 = 0x47464345;

#[derive(Debug, Clone)]
pub struct ProjectSettings {
    pub inner: IndexMap<String, Variant>,
}

impl ProjectSettings {
    pub const PROJECT_SETTINGS_PATH: &str = "project.binary";

    pub fn parse_binary(contents: &mut ReadableMarshalBuffer<'_>) -> crate::Result<Self> {
        let magic = contents.decode_uint32()?;

        if magic != BINARY_MAGIC {
            return Err(crate::Error::BadData);
        }

        let count = contents.decode_uint32()?;

        let mut inner = IndexMap::with_capacity(count as usize);

        for _ in 0..count {
            let key_len = contents.decode_uint32()? as usize;
            contents.ensure_remaining(key_len)?;

            let key = std::str::from_utf8(&contents.buffer()[..key_len])
                .map_err(|_| crate::Error::BadData)?
                .to_owned();

            contents.advance(key_len);

            let value_len = contents.decode_uint32()? as usize;
            contents.ensure_remaining(value_len)?;

            let value_bytes = &contents.decode_slice(value_len)?;
            let mut buf = ReadableMarshalBuffer::new(value_bytes, false);
            let value = Variant::decode(&mut buf, true)?;

            inner.insert(key, value);
        }

        Ok(Self { inner })
    }

    pub fn encode(&self, buf: &mut WritableMarshalBuffer) -> crate::Result<()> {
        buf.encode_uint32(BINARY_MAGIC);
        buf.encode_uint32(self.inner.len() as u32);

        for (key, value) in &self.inner {
            let key_bytes = key.as_bytes();

            buf.encode_uint32(key_bytes.len() as u32);
            buf.buffer().extend_from_slice(key_bytes);

            let mut value_buf = WritableMarshalBuffer::new_from(buf);
            value.encode(&mut value_buf, true)?;

            buf.encode_uint32(value_buf.len() as u32);
            buf.buffer().extend_from_slice(&value_buf);
        }

        Ok(())
    }
}
