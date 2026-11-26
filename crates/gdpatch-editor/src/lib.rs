#![deny(unsafe_op_in_unsafe_fn)]
use crate::gdextension::bindings::GDClassShaped;
use gdpatch_godot::{
    ReadableMarshalBuffer, UIDCache,
    build::{EngineBuild, VersionSpecifier, bundled_builds},
    pack::{Pack, PackConfig, PackedFile},
    project_settings::ProjectSettings,
};
use gobbind::gobbind;
use memmap2::Mmap;
use std::fs::File;

mod gdextension;
mod hash;
mod intercept;

#[derive(Default)]
pub struct GDPatchPack {
    pack: Option<(Pack, Mmap)>,
    build: Option<EngineBuild>,
    uid_cache: UIDCache,
}

impl GDClassShaped for GDPatchPack {
    fn new() -> Self {
        Self::default()
    }
}

#[gobbind]
impl GDPatchPack {
    pub fn init(&mut self, pack_path: String) {
        let file = match File::open(pack_path) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("failed to open pack: {}", err);
                return;
            }
        };

        let pack = match Pack::parse(&file, PackConfig::default()) {
            Ok(pack) => pack,
            Err(err) => {
                eprintln!("failed to parse pack: {}", err);
                return;
            }
        };

        let version = VersionSpecifier::new(
            pack.engine_version.0,
            pack.engine_version.1,
            pack.engine_version.2,
            0,
            "stable",
        );
        let build = match bundled_builds().find_approximate_build(&version) {
            Some(build) => build.clone(),
            None => {
                eprintln!("failed to find engine version {}", version);
                return;
            }
        };
        self.build = Some(build);

        let mapping = unsafe { Mmap::map(&file).expect("failed to mmap pack") };
        self.pack = Some((pack, mapping));

        let project_settings = self
            .get_file(ProjectSettings::PROJECT_SETTINGS_PATH)
            .and_then(|bytes| {
                let mut buffer = ReadableMarshalBuffer::new(&bytes, true);
                ProjectSettings::parse_binary(&mut buffer).ok()
            });
        let use_hidden_project_data_directory = project_settings
            .map(|p| p.use_hidden_project_data_directory())
            .unwrap_or(true);

        let uid_cache = self
            .get_file(&format!(
                "{}/{}",
                if use_hidden_project_data_directory {
                    ".godot"
                } else {
                    "godot"
                },
                UIDCache::UID_CACHE_FILENAME
            ))
            .and_then(|bytes| {
                let mut buffer = ReadableMarshalBuffer::new(&bytes, true);
                UIDCache::decode(&mut buffer).ok()
            })
            .unwrap_or_default();
        self.uid_cache = uid_cache;
    }

    fn format_pack_path(&self, path: &str) -> String {
        let build = self.build.as_ref().unwrap();
        if build.has_prefixless_pck_paths {
            path.to_string()
        } else {
            format!("res://{}", path)
        }
    }

    pub fn get_file(&self, path: &str) -> Option<Vec<u8>> {
        let (pack, mapping) = self.pack.as_ref().unwrap();

        let path = self.format_pack_path(path);
        let entry = pack.files.get(&path)?;

        let data = unsafe {
            let ptr = mapping.as_ptr().add(entry.offset as usize);
            std::slice::from_raw_parts(ptr, entry.size as usize)
        };

        Some(data.to_vec())
    }

    pub fn get_file_info(&self, path: &str) -> Option<&PackedFile> {
        let (pack, _) = self.pack.as_ref().unwrap();
        let path = self.format_pack_path(path);
        pack.files.get(&path)
    }

    pub fn file_exists(&self, path: &str) -> bool {
        self.get_file_info(path).is_some()
    }

    pub fn get_uid(&self, path: &str) -> Option<u64> {
        let path = format!("res://{}", path);
        self.uid_cache.0.iter().find(|e| e.1 == path).map(|e| e.0.0)
    }
}
