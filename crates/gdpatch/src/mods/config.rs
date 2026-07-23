use crate::{
    config::{annotate_toml, convert_into_comments, create_option_comment},
    mods::ModConfigInfo,
};
use indexmap::IndexMap;
use std::path::PathBuf;
use toml_edit::DocumentMut;

pub type ModConfigData = IndexMap<String, IndexMap<String, toml::Value>>;

#[derive(Debug)]
pub struct ModConfig {
    /// Path to the config file.
    path: PathBuf,

    /// Config metadata from the mod's manifest.
    meta: ModConfigInfo,

    /// Config data.
    data: ModConfigData,
}

impl ModConfig {
    pub fn new(path: PathBuf, meta: ModConfigInfo) -> Self {
        let data: ModConfigData = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str::<ModConfigData>(&s).ok())
            .unwrap_or_default();
        Self { path, meta, data }
    }

    pub fn get_option(&self, section: &str, option: &str) -> Option<&toml::Value> {
        self.data
            .get(section)
            .and_then(|s| s.get(option))
            .or_else(|| {
                self.meta
                    .get(section)
                    .and_then(|s| s.get(option))
                    .and_then(|o| o.default.as_ref())
            })
    }

    pub fn set_option(
        &mut self,
        section: &str,
        option: &str,
        value: Option<toml::Value>,
    ) -> color_eyre::Result<()> {
        let section = self.data.entry(section.to_string()).or_default();

        match value {
            Some(value) => section.insert(option.to_string(), value),
            None => section.shift_remove(option),
        };

        self.write()?; // TODO: should we write on every set?
        Ok(())
    }

    pub fn write(&self) -> color_eyre::Result<()> {
        let mut doc = toml_edit::ser::to_string_pretty(&self.data)?.parse::<DocumentMut>()?;

        for (section_id, options) in &self.meta {
            for (option_id, meta) in options {
                if option_id == "meta" {
                    let mut comment = String::new();

                    if let Some(desc) = &meta.description {
                        comment.push_str(&convert_into_comments(desc, 2));
                    }

                    annotate_toml(&mut doc, section_id, None, &comment)?;
                } else if !meta.hidden {
                    let exists = doc
                        .get(section_id)
                        .and_then(|v| v.as_table())
                        .map(|t| t.get(option_id).is_some())
                        .unwrap_or_default();

                    let str = create_option_comment(
                        option_id,
                        meta.description.as_deref(),
                        Some(vec![]),
                        meta.default.as_ref(),
                        !exists,
                    )?;
                    annotate_toml(&mut doc, section_id, Some(option_id), &str)?;
                }
            }
        }

        std::fs::write(&self.path, doc.to_string())?;
        Ok(())
    }
}
