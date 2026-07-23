use color_eyre::eyre::OptionExt;
use include_dir::{Dir, DirEntry, include_dir};
use std::collections::HashMap;

use crate::mods::{ModInfo, ModMeta};

pub const BUILTIN_MOD_ID: &str = "gdpatch";

static BUILTIN_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/mods/builtin");

fn extract(result: &mut HashMap<String, Vec<u8>>, dir: &Dir) -> color_eyre::Result<()> {
    for entry in dir.entries() {
        let path = entry
            .path()
            .to_str()
            .ok_or_eyre("failed to stringify path")?;

        match entry {
            DirEntry::Dir(d) => extract(result, d)?,
            DirEntry::File(f) => {
                result.insert(path.to_string(), f.contents().to_vec());
            }
        }
    }

    Ok(())
}

pub fn create_builtin_mod() -> color_eyre::Result<HashMap<String, Vec<u8>>> {
    let mut result = HashMap::new();
    extract(&mut result, &BUILTIN_DIR)?;

    let mod_info = ModInfo {
        id: BUILTIN_MOD_ID.to_string(),
        meta: Some(ModMeta {
            name: Some("GDPatch".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            description: Some(env!("CARGO_PKG_DESCRIPTION").to_string()),
            ..Default::default()
        }),
        config: None, // TODO
    };
    let mod_info = toml::to_string(&mod_info)?;
    result.insert("gdpatch_mod.toml".to_string(), mod_info.as_bytes().to_vec());

    Ok(result)
}
