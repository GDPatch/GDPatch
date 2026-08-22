use crate::{
    Config,
    mods::{Mods, lua::PatcherCallbacks},
    virtual_pack::{FileContents, builder::VirtualPackBuilder},
};
use color_eyre::eyre::Context as _;
use gdpatch_godot::{
    ReadableMarshalBuffer, UIDCache, WritableMarshalBuffer,
    build::{EngineBuild, GDScriptBuild, GDScriptV2Build},
    config_file::{class_cache::ClassCache, extension_list::ExtensionList},
    gdscript::{
        parser::parse_to_tokens,
        tokenizer::{
            CompressMode, TokenizerBytecode, TokenizerText, reconstruct_script_binary,
            reconstruct_script_text,
        },
    },
    pack::Pack,
    project_settings::ProjectSettings,
    variant::Variant,
};
use memmap2::Mmap;
use std::sync::Arc;
use tracing::{error, info_span, trace, warn};

pub struct Patcher<'a> {
    old_pack: &'a Pack,
    mapping: Arc<Mmap>,
    engine_build: EngineBuild,

    config: &'a Config,
    callbacks: PatcherCallbacks,
    mods: &'a Mods,

    new_pack: VirtualPackBuilder,

    // files in the game pack
    uid_cache: UIDCache,
    class_cache: ClassCache,
    extension_list: ExtensionList,

    // options in project settings
    real_t_is_double: bool,
    use_hidden_project_data_directory: bool,
}

impl<'a> Patcher<'a> {
    pub fn new(
        old_pack: &'a Pack,
        mapping: Arc<Mmap>,
        engine_build: EngineBuild,

        config: &'a Config,
        callbacks: PatcherCallbacks,
        mods: &'a Mods,
    ) -> Self {
        assert!(old_pack.deltas.is_empty(), "deltas are unimplemented");

        Self {
            old_pack,
            mapping,
            engine_build,

            config,
            callbacks,
            mods,

            new_pack: VirtualPackBuilder::new(old_pack),

            uid_cache: UIDCache::default(),
            class_cache: ClassCache::default(),
            extension_list: ExtensionList::default(),

            real_t_is_double: false,
            use_hidden_project_data_directory: true,
        }
    }

    /// Add the "res://" prefix to a path if required by the engine version.
    fn ensure_path_prefix(&self, path: &str) -> String {
        if self.engine_build.has_prefixless_pck_paths {
            path.strip_prefix("res://").unwrap_or(path).to_string()
        } else {
            if path.starts_with("res://") {
                path.to_string()
            } else {
                format!("res://{}", path)
            }
        }
    }

    /// Strip the "res://" prefix from a path if present.
    fn strip_path_prefix(&self, path: &str) -> String {
        if self.engine_build.has_prefixless_pck_paths {
            path.to_string()
        } else {
            path.strip_prefix("res://").unwrap_or(path).to_string()
        }
    }

    /// Get the path of a file within the project data directory (usually ".godot").
    fn data_dir_path(&self, path: &str) -> String {
        format!(
            "{}/{}",
            if self.use_hidden_project_data_directory {
                ".godot"
            } else {
                "godot"
            },
            path
        )
    }

    fn parse_project_settings(&mut self) -> ProjectSettings {
        // TODO: support using a pre-existing project settings for games with multiple pack files
        let project_settings = {
            let path = self.ensure_path_prefix(ProjectSettings::PROJECT_SETTINGS_PATH);

            let file = self
                .old_pack
                .files
                .get(&path)
                .expect("project settings file is missing");

            let contents = FileContents::Disk {
                mapping: self.mapping.clone(),
                offset: file.offset,
                len: file.size,
            };

            let slice = contents.as_slice();

            let mut buf = ReadableMarshalBuffer::new(slice, true);
            ProjectSettings::parse_binary(&mut buf).expect("failed to parse project settings")
        };

        // Update some flags based off of the project settings.
        self.real_t_is_double = project_settings
            .inner
            .get("application/config/features")
            .map(|p| match p {
                Variant::PackedStringArray(array) => array.iter().any(|f| f == "Double Precision"),
                _ => false,
            })
            .unwrap_or_default();
        self.use_hidden_project_data_directory = project_settings
            .inner
            .get("application/config/use_hidden_project_data_directory")
            .map(|p| match p {
                Variant::Bool(value) => *value,
                _ => true,
            })
            .unwrap_or(true);

        project_settings
    }

    /// Check if this path needs to be specially merged together.
    fn merge_special_file(
        &mut self,
        normalized_path: &str,
        slice: &[u8],
    ) -> color_eyre::Result<bool> {
        if normalized_path == self.data_dir_path(UIDCache::UID_CACHE_FILENAME) {
            let mut buffer = ReadableMarshalBuffer::new(slice, true);

            self.uid_cache
                .merge_decode(&mut buffer)
                .wrap_err("failed to merge UID cache")?;

            return Ok(true);
        }

        if normalized_path == self.data_dir_path(ClassCache::CLASS_CACHE_FILENAME) {
            let str = str::from_utf8(slice).wrap_err("failed to decode class cache string")?;

            self.class_cache
                .merge_decode(str)
                .map_err(|e| color_eyre::eyre::eyre!(e.0))
                .wrap_err("failed to merge class cache")?;

            return Ok(true);
        }

        if normalized_path == self.data_dir_path(ExtensionList::EXTENSION_LIST_FILENAME) {
            let str = str::from_utf8(slice).wrap_err("failed to decode extension list string")?;

            self.extension_list.merge_decode(str);

            return Ok(true);
        }

        Ok(false)
    }

    /// Inject merged files into the built pack.
    fn apply_merged_files(&mut self, project_settings: ProjectSettings) {
        // Patch (and overwrite) project settings.
        // This is done later to enforce a consistent order of operations within the patcher script, making sure all scripts are patched beforehand.
        {
            let result = try {
                let patched_settings = self
                    .callbacks
                    .patch_project_settings(project_settings.clone())
                    .wrap_err("failed to run project settings patchers")?;

                let mut patched_data = WritableMarshalBuffer::new(false);
                patched_settings
                    .encode(&mut patched_data)
                    .wrap_err("failed to encode patched settings")?;

                let patched_data = patched_data.into_inner();
                self.new_pack.add_file(
                    self.ensure_path_prefix(ProjectSettings::PROJECT_SETTINGS_PATH),
                    patched_data.len() as u64,
                    [0u8; 16], // TODO
                    FileContents::Memory(patched_data),
                );
            };

            if let Err(err) = result {
                error!(?err, "failed to write project settings");
            }
        }

        {
            let mut buffer = WritableMarshalBuffer::new(false);
            self.uid_cache.encode(&mut buffer);
            self.new_pack.add_file(
                self.ensure_path_prefix(&self.data_dir_path(UIDCache::UID_CACHE_FILENAME)),
                buffer.len() as u64,
                [0u8; 16], // TODO
                FileContents::Memory(buffer.into_inner()),
            );
        }

        {
            let str = self.class_cache.write();
            let buffer = str.as_bytes().to_vec();
            self.new_pack.add_file(
                self.ensure_path_prefix(&self.data_dir_path(ClassCache::CLASS_CACHE_FILENAME)),
                buffer.len() as u64,
                [0u8; 16], // TODO
                FileContents::Memory(buffer),
            );
        }

        {
            let str = self.extension_list.write();
            let buffer = str.as_bytes().to_vec();
            self.new_pack.add_file(
                self.ensure_path_prefix(
                    &self.data_dir_path(ExtensionList::EXTENSION_LIST_FILENAME),
                ),
                buffer.len() as u64,
                [0u8; 16], // TODO
                FileContents::Memory(buffer),
            );
        }
    }

    /// Patch a script file if a patcher is registered for it.
    fn patch_script(
        &self,
        normalized_path: &str,
        slice: &[u8],
        gdscript_build: &GDScriptV2Build,
    ) -> color_eyre::Result<Vec<u8>> {
        let (tokens, is_binary) = if normalized_path.ends_with(".gd") {
            let source = str::from_utf8(slice).wrap_err("failed to parse utf8")?;
            let mut tokenizer = TokenizerText::new(gdscript_build, source);
            let mut tokens = parse_to_tokens(&mut tokenizer)?;

            if self.config.debug.patch_all_scripts {
                let source = reconstruct_script_text(&tokens);
                let mut tokenizer = TokenizerText::new(gdscript_build, &source);
                tokens = parse_to_tokens(&mut tokenizer)?;
            }

            (tokens, false)
        } else if normalized_path.ends_with(".gdc") {
            let mut tokenizer = TokenizerBytecode::new(gdscript_build, slice)
                .wrap_err("failed to parse bytecode")?;

            let mut tokens = parse_to_tokens(&mut tokenizer)?;

            if self.config.debug.patch_all_scripts {
                let source = reconstruct_script_text(&tokens);

                let mut tokenizer = TokenizerText::new(gdscript_build, &source);
                tokens = parse_to_tokens(&mut tokenizer)?;
            }

            (tokens, true)
        } else {
            unreachable!()
        };

        let mut patched_tokens = self
            .callbacks
            .patch_script(normalized_path, tokens, gdscript_build)
            .wrap_err("failed to run script patchers")?;

        let patched_data = if is_binary {
            if self.config.debug.patch_all_scripts {
                let reconstructed = reconstruct_script_binary(
                    gdscript_build,
                    &patched_tokens,
                    CompressMode::None,
                    self.real_t_is_double,
                )
                .wrap_err("failed to reconstruct script as binary")?;

                let mut tokenizer = TokenizerBytecode::new(gdscript_build, &reconstructed)
                    .wrap_err("failed to reparse bytecode")?;
                patched_tokens = parse_to_tokens(&mut tokenizer)?;
            }

            reconstruct_script_binary(
                gdscript_build,
                &patched_tokens,
                CompressMode::None,
                self.real_t_is_double,
            )
            .wrap_err("failed to reconstruct binary script")?
        } else {
            let content = reconstruct_script_text(&patched_tokens);
            content.into_bytes()
        };

        Ok(patched_data)
    }

    pub fn run(mut self) -> color_eyre::Result<VirtualPackBuilder> {
        let project_settings = self.parse_project_settings();

        let gdscript_build = match self.engine_build.gdscript {
            GDScriptBuild::V2(ref v2) => v2.clone(),
            _ => unimplemented!("GDScript V1"),
        };

        // Rebuild files in the virtual pack.
        for (path, file) in &self.old_pack.files {
            let normalized_path = self.strip_path_prefix(path);
            let _entered = info_span!("pack_entry", path = %normalized_path).entered();

            let contents = FileContents::Disk {
                mapping: self.mapping.clone(),
                offset: file.offset,
                len: file.size,
            };
            let slice = contents.as_slice();

            match self.merge_special_file(&normalized_path, slice) {
                Ok(patched) => {
                    if patched {
                        continue;
                    }
                }

                Err(err) => error!(err = %err, "failed to merge file"),
            };

            // Patch scripts.
            let is_script = path.ends_with(".gd") || path.ends_with(".gdc");
            let should_patch_script = self.callbacks.has_patcher_for_script(&normalized_path)
                || self.config.debug.patch_all_scripts;
            if is_script && should_patch_script {
                match self.patch_script(&normalized_path, slice, &gdscript_build) {
                    Ok(patched_data) => {
                        self.new_pack.add_file(
                            path.clone(),
                            patched_data.len() as u64,
                            file.hash,
                            FileContents::Memory(patched_data),
                        );

                        continue;
                    }

                    Err(err) => {
                        error!(err = %err, "failed to patch script");
                    }
                }
            }

            if self.callbacks.has_patcher_for_file(&normalized_path) {
                match self.callbacks.patch_file(&normalized_path, slice) {
                    Ok(patched_data) => {
                        self.new_pack.add_file(
                            path.clone(),
                            patched_data.len() as u64,
                            file.hash,
                            FileContents::Memory(patched_data),
                        );

                        continue;
                    }

                    Err(err) => {
                        error!(err = %err, "failed to patch file");
                    }
                }
            }

            // Preserve the original file.
            trace!("adding game file");
            self.new_pack
                .add_file(path.clone(), file.size, file.hash, contents);
        }

        // Insert modded pack files.
        // TODO: mod order, maybe?
        for (mod_id, r#mod) in &self.mods.0 {
            let _entered = info_span!("mod_pack", %mod_id).entered();

            let mod_pack = match &r#mod.pack {
                Some(mod_pack) => mod_pack,
                None => continue,
            };

            for (mut path, contents) in mod_pack.files() {
                let _entered = info_span!("mod_pack_entry", path = %path).entered();

                // Add/remove res:// prefix if required.
                path = self.ensure_path_prefix(&path);
                let normalized_path = self.strip_path_prefix(&path);

                let exists_in_old = self.old_pack.files.contains_key(&path);
                let is_script = path.ends_with(".gd") || path.ends_with(".gdc");

                // Disallow replacing scripts directly.
                if exists_in_old && is_script {
                    warn!("cannot replace game script in mod pack");
                    continue;
                }

                // Always skip project settings that were accidentally included by the editor.
                if normalized_path == ProjectSettings::PROJECT_SETTINGS_PATH {
                    warn!("skipping project settings file in mod pack");
                    continue;
                }

                match self.merge_special_file(&normalized_path, contents.as_slice()) {
                    Ok(patched) => {
                        if patched {
                            continue;
                        }
                    }

                    Err(err) => error!(err = %err, "failed to merge file"),
                };

                // Preserve the original file.
                trace!("adding modded file");
                self.new_pack.add_file(
                    path.clone(),
                    contents.len(),
                    [0u8; 16], // TODO
                    contents,
                );
            }
        }

        self.apply_merged_files(project_settings);

        Ok(self.new_pack)
    }
}
