//! The global script class cache, which tracks `class_name` statements.
use crate::{
    config_file::ConfigFile,
    variant::{Array, Variant},
};

#[derive(Debug, Clone, Default)]
pub struct ClassCache(ConfigFile);

impl ClassCache {
    pub const CLASS_CACHE_PATH: &str = ".godot/global_script_class_cache.cfg";

    pub fn parse(data: &str) -> crate::Result<Self> {
        Ok(Self(ConfigFile::parse(data)?))
    }

    fn get_or_create_list(&mut self) -> crate::Result<&mut Array> {
        let section = self.0.inner.entry(String::default()).or_default();
        let entry = section
            .entry("list".to_string())
            .or_insert_with(|| Array::default().into());
        let Variant::Array(array) = entry else {
            return Err(crate::Error::BadData);
        };

        Ok(array)
    }

    pub fn write(&mut self) -> String {
        self.0.write()
    }

    pub fn merge(&mut self, other: &Self) -> crate::Result<()> {
        let our_array = self.get_or_create_list()?;

        let their_section = other.0.inner.get("").ok_or(crate::Error::BadData)?;
        let their_entry = their_section.get("list").ok_or(crate::Error::BadData)?;
        let Variant::Array(their_array) = their_entry else {
            return Err(crate::Error::BadData);
        };

        our_array.inner.extend_from_slice(&their_array.inner);

        Ok(())
    }

    pub fn merge_decode(&mut self, str: &str) -> crate::Result<()> {
        let other = Self::parse(str)?;
        self.merge(&other)?;
        Ok(())
    }
}
