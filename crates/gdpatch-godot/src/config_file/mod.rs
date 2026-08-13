//! Godot config file parser.
pub mod class_cache;
pub mod extension_list;

use crate::{
    util::escape_string_multiline,
    variant::{TagAssign, Variant, VariantParser, write_variant},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ConfigFile {
    pub inner: HashMap<String, HashMap<String, Variant>>,
}

fn property_name_encode(str: &str) -> String {
    for ch in str.chars() {
        if ch == '='
            || ch == '"'
            || ch == ';'
            || ch == '['
            || ch == ']'
            || (ch as u32) < 33
            || (ch as u32) > 126
        {
            return escape_string_multiline(str);
        }
    }

    // Keep as is
    str.to_string()
}

impl ConfigFile {
    pub fn parse(data: &str) -> crate::Result<Self> {
        let mut parser = VariantParser::new(data);
        let mut section = String::new();
        let mut inner: HashMap<String, HashMap<String, Variant>> = HashMap::new();

        loop {
            match parser
                .parse_tag_assign_eof(true)
                .map_err(|e| crate::Error::from(e))?
            {
                Some(TagAssign::Tag(tag)) => {
                    section = tag.name.replace("\\]", "]");
                }
                Some(TagAssign::Variant { assign, value }) => {
                    let section = inner.entry(section.clone()).or_default();

                    if matches!(value, Variant::Nil(_)) {
                        section.remove(&assign);
                    } else {
                        section.insert(assign, value);
                    }
                }
                None => break,
            }
        }

        Ok(Self { inner })
    }

    pub fn write(&self) -> String {
        let mut result = String::new();

        let mut first = true;
        for (section, values) in &self.inner {
            if first {
                first = false;
            } else {
                result.push('\n');
            }

            if !section.is_empty() {
                result.push_str(&format!("[{}]\n\n", section.replace("]", "\\]")));
            }

            for (key, value) in values {
                let value_str = write_variant(value, true);
                result.push_str(&format!("{}={}\n", property_name_encode(key), value_str));
            }
        }

        result
    }
}
