//! The global script class cache, which tracks `class_name` statements.
use crate::{
    config_file::ConfigFile,
    variant::{Array, ParseError, ParseResult, Variant},
};

#[derive(Debug, Clone, Default)]
pub struct ClassCache(ConfigFile);

impl ClassCache {
    pub const CLASS_CACHE_PATH: &str = ".godot/global_script_class_cache.cfg";

    pub fn parse(data: &str) -> ParseResult<Self> {
        Ok(Self(ConfigFile::parse(data)?))
    }

    fn get_or_create_list(&mut self) -> ParseResult<&mut Array> {
        let section = self.0.inner.entry(String::default()).or_default();
        let entry = section
            .entry("list".to_string())
            .or_insert_with(|| Array::default().into());
        let Variant::Array(array) = entry else {
            return Err(ParseError("Expected array for class list".into()));
        };

        Ok(array)
    }

    pub fn write(&mut self) -> String {
        self.0.write()
    }

    pub fn merge(&mut self, other: &Self) -> ParseResult<()> {
        let our_array = self.get_or_create_list()?;

        let Some(their_section) = other.0.inner.get("") else {
            return Err(ParseError("Missing top-level section".into()));
        };
        let Some(their_entry) = their_section.get("list") else {
            return Err(ParseError("Missing `list` entry".into()));
        };
        let Variant::Array(their_array) = their_entry else {
            return Err(ParseError("Expected array for class list".into()));
        };

        our_array.inner.extend_from_slice(&their_array.inner);

        Ok(())
    }

    pub fn merge_decode(&mut self, str: &str) -> ParseResult<()> {
        let other = Self::parse(str)?;
        self.merge(&other)?;
        Ok(())
    }
}
