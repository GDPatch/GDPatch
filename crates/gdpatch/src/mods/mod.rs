use crate::mods::builtin::create_builtin_mod;
use crate::mods::config::ModConfig;
use crate::mods::filesystem::{ModLoaderFolderFs, ModLoaderFs, ModLoaderMapFs};
use crate::virtual_pack::FileContents;
use color_eyre::eyre::{Context, Report, bail};
use figment::Figment;
use figment::providers::{Format, Toml};
use gdpatch_godot::build::EngineFlavor;
use gdpatch_godot::pack::Pack;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::path::{Path, PathBuf};

mod builtin;
mod config;
mod filesystem;
pub mod lua;

pub use builtin::BUILTIN_MOD_ID;

/// Optional metadata for the mod.
/// These fields are not used by GDPatch directly, but may be used by other mods (e.g. config UIs).
/// If you publish your mod online, we suggest making sure these values are in sync with your mod page.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ModMeta {
    /// Pretty name for this mod.
    pub name: Option<String>,

    /// Version number. No strict format is imposed on this, but it should be obvious to users.
    pub version: Option<String>,

    /// A list of mod authors.
    #[serde(default)]
    pub authors: Vec<String>,

    /// A short description of what this mod does.
    pub description: Option<String>,
}

/// Optional metadata for a mod's config section.
/// These fields are not used by GDPatch directly, but may be used by other mods (e.g. config UIs).
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModConfigOptionMeta {
    /// Pretty name for this option.
    pub name: Option<String>,

    /// Description for this option.
    pub description: Option<String>,

    /// Type for this option, to be used as a hint for custom config editors.
    /// GDPatch does not perform any type checking, and the value saved in this option may not match the type.
    pub r#type: Option<ModConfigOptionType>,

    /// The default value for this option.
    /// The default value will not be written to the config file directly, but will be returned by the config APIs and displayed in comments.
    pub default: Option<toml::Value>,

    /// Whether to hide all comments for this option.
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModConfigOptionType {
    String,
    Number,
    Boolean,
    Array,
    Table,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModInfo {
    /// A unique ID for this mod. Mod IDs should use snake case (all lowercase, with spaces replaced with underscores).
    /// This is the only required field in the mod info.
    pub id: String,

    /// Human-readable metadata about this mod.
    pub meta: Option<ModMeta>,

    /// Optional metadata for the mod's config options. Config options are referenced by a section ID and option ID.
    /// The "meta" option ID is reserved to represent metadata about the section itself.
    pub config: Option<ModConfigInfo>,
}

pub type ModConfigInfo = IndexMap<String, IndexMap<String, ModConfigOptionMeta>>;

impl ModInfo {
    /// Parse the config.
    pub fn parse(data: &str) -> color_eyre::Result<Self> {
        let toml = Toml::string(data);
        let figment = Figment::from(toml);
        figment.extract().context("failed to extract full config")
    }
}

#[derive(Debug)]
pub enum ModPack {
    /// The mod uses a direct .pck file.
    PackFile { pack: Pack, mapping: FileContents },

    /// The mod uses a loose directory.
    LooseDir {
        files: HashMap<String, FileContents>,
    },
}

impl ModPack {
    // FIXME this shit is ass and would be better served by an iter but I'm lazy
    pub fn files(&self) -> HashMap<String, FileContents> {
        let mut result = HashMap::new();

        match self {
            ModPack::PackFile { pack, mapping } => {
                let mapping = match mapping {
                    FileContents::Disk { mapping, .. } => mapping,
                    _ => unimplemented!(),
                };

                for (path, file) in &pack.files {
                    let contents = FileContents::Disk {
                        mapping: mapping.clone(),
                        offset: file.offset,
                        len: file.size,
                    };

                    result.insert(path.clone(), contents);
                }
            }

            ModPack::LooseDir { files } => {
                for (path, contents) in files {
                    result.insert(path.clone(), contents.clone());
                }
            }
        }

        result
    }
}

/// A loaded mod.
#[derive(Debug)]
pub struct Mod {
    /// Static info on the mod (ID, metadata, etc.).
    pub info: ModInfo,

    /// The mod's patcher script. This will be empty if there is no `patcher.lua` in the mod folder.
    pub patcher: Option<String>,

    /// The mod's pack. This will be empty if there is no `data.pck` in the mod folder.
    pub pack: Option<ModPack>,

    /// The mod's config.
    pub config: ModConfig,
}

#[derive(Debug)]
pub struct Mods(pub HashMap<String, Mod>);

impl Mods {
    /// Reads a mod from a directory. Errors if there is issues with the mod (e.g. invalid or
    /// missing mod info, or patcher with invalid syntax).
    fn read_mod_from_directory(
        fs: &dyn ModLoaderFs,
        flavor: &EngineFlavor,
        configs_directory: &Path,
    ) -> color_eyre::Result<Mod> {
        // Check for a `gdpatch_mod.toml` file.
        let mod_info_path = PathBuf::from("gdpatch_mod.toml");
        if !fs.exists(&mod_info_path)? {
            bail!("Mod is missing a gdpatch_mod.toml");
        }

        let mod_info = fs
            .read(&mod_info_path)
            .wrap_err("reading gdpatch_mod.toml")?;
        let mod_info = mod_info.as_slice();
        let mod_info = std::str::from_utf8(mod_info).wrap_err("parsing gdpatch_mod.toml")?;
        let mod_info = ModInfo::parse(mod_info)?;

        if mod_info
            .id
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '_')
        {
            // mainly to avoid any filesystem shenanigans
            bail!("Mod contains improperly formatted mod ID");
        }

        // Search for patcher.
        let patcher_path = PathBuf::from("patcher.lua");
        let patcher = if fs.exists(&patcher_path)? {
            let patcher = fs.read(&patcher_path).wrap_err("reading patcher.lua")?;
            let patcher = patcher.as_slice();
            let patcher = std::str::from_utf8(patcher).wrap_err("parsing patcher.lua")?;
            Some(patcher.to_string())
        } else {
            None
        };

        // Search for mod data.
        let pck_path = PathBuf::from("data.pck");
        let pack_folder = PathBuf::from("data");
        let pack = if fs.exists(&pck_path)? {
            let file = fs.read(&pck_path).wrap_err("reading data.pck")?;
            let cursor = Cursor::new(file.as_slice());
            let pack = Pack::parse(cursor, flavor).wrap_err("reading pck")?;

            Some(ModPack::PackFile {
                pack,
                mapping: file,
            })
        } else {
            if fs.exists(&pack_folder)? && fs.is_dir(&pack_folder)? {
                let mut files = HashMap::<String, FileContents>::new();
                read_files_recursively(fs, &mut files, &pack_folder, None)?;
                Some(ModPack::LooseDir { files })
            } else {
                None
            }
        };

        let config_path = configs_directory.join(format!("{}.toml", mod_info.id));
        let config = ModConfig::new(config_path, mod_info.config.clone().unwrap_or_default());

        Ok(Mod {
            info: mod_info,
            patcher,
            pack,
            config,
        })
    }

    /// Searches for mod folders in the given directory and loads their metadata/patchers/etc.
    pub fn search_and_load(
        mods_directory: &Path,
        configs_directory: &Path,
        flavor: &EngineFlavor,
    ) -> Result<Self, Vec<Report>> {
        let mut errors = Vec::new();

        if let Err(err) = std::fs::create_dir_all(mods_directory) {
            errors.push(Report::from(err).wrap_err("attempting to create mods folder"));
            return Err(errors);
        }

        if let Err(err) = std::fs::create_dir_all(configs_directory) {
            errors.push(Report::from(err).wrap_err("attempting to create configs folder"));
            return Err(errors);
        }

        let candidate_iter = match std::fs::read_dir(mods_directory) {
            Ok(it) => it,
            Err(err) => {
                errors.push(Report::from(err).wrap_err("listing mods folder"));
                return Err(errors);
            }
        };

        let mut mods = HashMap::new();

        // Load built-in mod.
        {
            let fs = match create_builtin_mod() {
                Ok(fs) => fs,
                Err(err) => {
                    errors.push(err.wrap_err("loading builtin mod"));
                    return Err(errors);
                }
            };
            let fs = ModLoaderMapFs::new(fs);

            match Mods::read_mod_from_directory(&fs, flavor, configs_directory) {
                Ok(r#mod) => {
                    mods.insert(r#mod.info.id.clone(), r#mod);
                }

                Err(err) => {
                    tracing::error!(?err, "failed to load builtin mod");
                }
            }
        }

        for candidate in candidate_iter {
            let candidate = match candidate {
                Ok(v) => v,
                Err(err) => {
                    let msg = "reading mod candidate directory";
                    errors.push(Report::from(err).wrap_err(msg));
                    continue;
                }
            };

            let candidate_path = candidate.path();
            if !candidate_path.is_dir() {
                continue;
            }

            let relative_path = candidate_path
                .strip_prefix(mods_directory)
                .expect("directory doesn't have parent path as prefix?");

            let fs = ModLoaderFolderFs::new(candidate_path.clone());
            match Mods::read_mod_from_directory(&fs, flavor, configs_directory) {
                Ok(r#mod) => {
                    if mods.contains_key(&r#mod.info.id) {
                        tracing::warn!(
                            mod_id = r#mod.info.id,
                            "attempted to load duplicate mod ID"
                        );
                        continue;
                    }

                    mods.insert(r#mod.info.id.clone(), r#mod);
                }
                Err(err) => {
                    let relative_path = relative_path.display();
                    tracing::error!(?err, %relative_path, "failed to load mod");
                }
            }
        }

        Ok(Mods(mods))
    }
}

fn read_files_recursively(
    fs: &dyn ModLoaderFs,
    result: &mut HashMap<String, FileContents>,
    dir: &Path,
    prefix: Option<String>,
) -> color_eyre::Result<()> {
    let files = fs.read_dir(dir)?;

    for file in files {
        let file_path = dir.join(&file);

        let file_name = prefix
            .as_ref()
            .map(|old| format!("{}/{}", old, file))
            .unwrap_or_else(|| file);

        if fs.is_dir(&file_path)? {
            read_files_recursively(fs, result, &file_path, Some(file_name))?;
        } else {
            let contents = fs.read(&file_path)?;
            result.insert(file_name, contents);
        }
    }

    Ok(())
}
