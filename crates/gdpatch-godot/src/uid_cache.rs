use crate::{ReadableMarshalBuffer, WritableMarshalBuffer};
use crate::Error::BadData;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct UID(pub u64);

#[derive(Debug, Default, Clone)]
pub struct UIDCache(pub Vec<(UID, String)>);

impl UIDCache {
    pub const UID_CACHE_PATH: &str = ".godot/uid_cache.bin";

    pub fn decode(buffer: &mut ReadableMarshalBuffer) -> crate::Result<Self> {
        let count = buffer.decode_uint32()?;
        let mut entries = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let uid = buffer.decode_uint64().map(UID)?;
            let path_len = buffer.decode_uint32()? as usize;
            let path = &buffer.buffer()[..path_len];
            let path = str::from_utf8(path).map_err(|_| BadData)?.to_owned();
            buffer.advance(path_len);

            entries.push((uid, path));
        }

        Ok(Self(entries))
    }

    pub fn encode(&self, buffer: &mut WritableMarshalBuffer) {
        buffer.encode_uint32(self.0.len() as u32);

        for (uid, path) in &self.0 {
            buffer.encode_uint64(uid.0);
            buffer.encode_uint32(path.len() as u32);
            buffer.buffer().extend_from_slice(path.as_bytes());
        }
    }

    pub fn merge(&mut self, other: &Self) {
        self.0.extend_from_slice(&other.0);
    }

    pub fn merge_decode(&mut self, buffer: &mut ReadableMarshalBuffer) -> crate::Result<()> {
        let other = Self::decode(buffer)?;
        self.merge(&other);
        Ok(())
    }
}