use color_eyre::eyre::{Context, ContextCompat};
use documented::DocumentedFieldsOpt;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use gdpatch_godot::{build::SerializedEngineBuild, pack::PackConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;
use toml_edit::{DocumentMut, RawString};
use tracing::level_filters::LevelFilter;

#[derive(Deserialize, Serialize, Debug, Default, DocumentedFieldsOpt)]
pub struct Config {
    /// Logging configuration.
    pub log: ConfigLog,

    /// Debugging related configuration.
    pub debug: ConfigDebug,

    /// Engine version overrides.
    pub engine: Option<ConfigEngine>,
}

#[derive(Deserialize, Serialize, Debug, DocumentedFieldsOpt)]
pub struct ConfigLog {
    /// The log level to use ("info", "debug", or "trace").
    pub level: LogLevel,

    /// Whether to open a console window containing logs for GDPatch and Godot.
    /// To open the console window as soon as possible, set the GDPATCH_CONSOLE environment variable to 1.
    pub console: bool,

    /// Whether to use colored logs (using ANSI formatting) in the GDPatch console.
    /// This may not work correctly under Wine or Proton.
    pub console_ansi: bool,
}

impl Default for ConfigLog {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            console: false,
            console_ansi: true,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ConfigEngine {
    #[serde(flatten, default)]
    pub engine: SerializedEngineBuild,

    #[serde(flatten, default)]
    pub pack: PackConfig,
}

#[derive(Deserialize, Serialize, Debug, DocumentedFieldsOpt, Default)]
pub struct ConfigDebug {
    /// Whether to patch all game scripts present in the game pack.
    /// This helps verify issues within GDPatch's script parsing code.
    pub patch_all_scripts: bool,
}

impl Config {
    /// Parse the config and write the formatted file to disk.
    pub fn parse(path: &Path) -> color_eyre::Result<Self> {
        let default = Serialized::defaults(Config::default());
        let toml = Toml::file(path);
        let figment = Figment::from(default).merge(toml);

        // Write the config file back to disk, using only the default/toml provider
        let config: Self = figment
            .extract()
            .context("failed to extract partial config")?;
        let config = config.serialize().context("failed to serialize config")?;
        std::fs::write(path, config).context("failed to write config")?;

        // Add env variables, which we consider temporary
        let env = Env::prefixed("GDPATCH_").split("_");
        figment
            .merge(env)
            .extract()
            .context("failed to extract full config")
    }

    /// Serialize into a pretty formatted string, including documentation.
    fn serialize(&self) -> color_eyre::Result<String> {
        // https://github.com/toml-rs/toml/issues/376#issuecomment-1826360347
        // https://github.com/cyqsimon/openvpn-cred-management/blob/e040b32cebb5ecf361a5549feb3e3d5e22741913/src/config.rs#L152-L183
        let mut doc = toml_edit::ser::to_string_pretty(self)?.parse::<DocumentMut>()?;

        annotate_toml_with_docs::<ConfigLog>(&mut doc, "log")?;
        annotate_toml_with_docs::<ConfigDebug>(&mut doc, "debug")?;

        Ok(doc.to_string())
    }
}

fn annotate_toml_with_docs<T: DocumentedFieldsOpt>(
    doc: &mut DocumentMut,
    section_id: &str,
) -> color_eyre::Result<()> {
    for option_id in T::FIELD_NAMES {
        if let Ok(docs) = T::get_field_docs(option_id) {
            let exists = doc
                .get(section_id)
                .and_then(|v| v.as_table())
                .map(|t| t.get(option_id).is_some())
                .unwrap_or_default();

            let str = create_option_comment(option_id, Some(docs), None, None, !exists)?;
            annotate_toml(doc, section_id, Some(*option_id), &str)?;
        }
    }

    Ok(())
}

pub fn create_option_comment(
    option_id: &str,
    description: Option<&str>,
    extra_lines: Option<Vec<&str>>,
    default: Option<&toml::Value>,
    add_placeholder: bool,
) -> color_eyre::Result<String> {
    let mut result = String::new();

    if let Some(description) = description {
        result.push_str(&convert_into_comments(description, 2));
    }

    if let Some(extra_lines) = extra_lines {
        for line in extra_lines {
            result.push_str(&convert_into_comments(line, 2));
        }
    }

    if let Some(default) = r#default {
        let mut default_str = String::new();
        serde::Serialize::serialize(&default, toml::ser::ValueSerializer::new(&mut default_str))?;

        if add_placeholder {
            let str = format!("{} = {}", option_id, default_str);
            result.push_str(&convert_into_comments(&str, 1));
        } else {
            let str = format!("Default value: {}", default_str);
            result.push_str(&convert_into_comments(&str, 2));
        };
    } else if add_placeholder {
        // We want a placeholder even if the value is missing
        let str = format!("{} =", option_id);
        result.push_str(&convert_into_comments(&str, 1));
    }

    Ok(result.trim().to_string())
}

pub fn convert_into_comments(str: &str, num_hashes: usize) -> String {
    let prefix = "#".repeat(num_hashes);
    let mut str = str
        .trim()
        .lines()
        .map(|l| format!("{prefix} {l}").trim().to_string())
        .collect::<Vec<String>>()
        .join("\n");
    str.push('\n');
    str
}

pub fn annotate_toml(
    doc: &mut DocumentMut,
    section_id: &str,
    option_id: Option<&str>,
    comment: &str,
) -> color_eyre::Result<()> {
    if comment.trim().is_empty() {
        return Ok(());
    }

    if !doc.contains_table(section_id) {
        doc.insert(section_id, toml_edit::table());
    }

    let section = doc
        .get_mut(section_id)
        .wrap_err("failed to get section after it was supposed to be created")?;

    let section = section
        .as_table_mut()
        .wrap_err("failed to get section as table")?;

    match option_id {
        Some(option_id) => match section.get_key_value_mut(option_id) {
            Some((mut key, _item)) => {
                // The value exists, so we'll place it as a prefix on the value's decor
                let decor = key.leaf_decor_mut();
                decor.set_prefix(format!("{}\n", comment));
            }

            None => {
                // The key doesn't exist, so we need to place it as a suffix on the section itself
                let decor = section.decor_mut();

                let mut suffix = "\n".to_string();
                if let Some(old_suffix) =
                    decor.suffix().and_then(RawString::as_str).map(|s| s.trim())
                    && !old_suffix.is_empty()
                {
                    suffix.push_str(old_suffix);
                    suffix.push('\n');
                };
                suffix.push_str(comment);

                decor.set_suffix(suffix);
            }
        },

        None => {
            // This applies to the entire section.
            let decor = section.decor_mut();

            let mut prefix = comment.trim_end().to_string();
            if let Some(old_prefix) = decor.prefix().and_then(RawString::as_str) {
                prefix = format!("{}{}", old_prefix.trim_end(), prefix);
            }
            prefix.push('\n');

            decor.set_prefix(prefix);
        }
    }

    Ok(())
}
