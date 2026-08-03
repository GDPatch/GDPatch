#![feature(try_blocks)]
#![feature(seek_stream_len)]

use color_eyre::eyre::{Context, ContextCompat, OptionExt, bail};
use gdpatch_godot::gdscript::parser::parse_to_tokens;
use gdpatch_godot::gdscript::tokenizer::{
    CompressMode, TokenizerBytecode, TokenizerText, reconstruct_script_binary,
    reconstruct_script_text,
};
use gdpatch_godot::project_settings::ProjectSettings;
use gdpatch_godot::variant::Variant;
use memmap2::Mmap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tracing::{debug, debug_span, error, info, info_span, level_filters::LevelFilter, trace, warn};
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

mod bindings;
mod config;
mod intercept;
mod ipc;
mod mods;
mod virtual_pack;

use crate::intercept::GDPatchStreamFactory;
use crate::mods::lua::{ModLua, PatcherCallbacks};
use crate::mods::{BUILTIN_MOD_ID, Mods};
use crate::virtual_pack::builder::VirtualPackBuilder;
use crate::virtual_pack::{FileContents, VirtualPack};
pub use config::Config;
use gdpatch_godot::build::{
    EngineBuilds, GDScriptBuild, SerializedBuildsFile, VersionSpecifier, resolve_bundled_builds,
};
use gdpatch_godot::pack::Pack;
use gdpatch_godot::{ReadableMarshalBuffer, UIDCache, WritableMarshalBuffer};

static INSTANCE: OnceLock<GDPatch> = OnceLock::new();

#[derive(Debug)]
pub struct GDPatch {
    pub config: Config,
    root_directory: PathBuf,
    virtual_packs: RwLock<HashMap<PathBuf, Arc<VirtualPack>>>,
    engine_builds: OnceLock<EngineBuilds>,
    mods: RwLock<Option<Mods>>,
}

impl GDPatch {
    fn new(config: Config, root_directory: PathBuf) -> Self {
        Self {
            config,
            root_directory,
            virtual_packs: Default::default(),
            engine_builds: Default::default(),
            mods: RwLock::new(None),
        }
    }
    /// Configures the global instance.
    pub fn setup_instance_logging_etc() -> color_eyre::Result<()> {
        let game_directory = std::env::current_exe()
            .context("couldn't get current exe")?
            .parent()
            .context("couldn't get parent directory")?
            .to_path_buf();

        // This is one of the few environment variables that we don't use via figment
        let root_directory = if let Ok(dir) = std::env::var("GDPATCH_ROOT_DIRECTORY") {
            PathBuf::from(dir)
        } else {
            game_directory.join("GDPatch")
        };
        std::fs::create_dir_all(&root_directory).context("failed to create root directory")?;

        let config = &root_directory.join("config.toml");
        let config = Config::parse(config).context("failed to read config")?;

        let instance = GDPatch::new(config, root_directory);

        // Set up logger and global instance.
        instance.setup_logger()?;

        if INSTANCE.set(instance).is_err() {
            bail!("called GDPatch::setup multiple times");
        }

        Ok(())
    }

    /// Finishes the global setup.
    pub fn finish_setup(&self) -> color_eyre::Result<()> {
        info!("This is GDPatch {}, heya!", env!("CARGO_PKG_VERSION"));

        // Setup file hooks.
        filesilly::init()?;
        filesilly::set(Box::new(GDPatchStreamFactory));

        // Search for mods.
        let mods_directory = self.root_directory.join("mods");
        let configs_directory = self.root_directory.join("configs");

        let mods = match Mods::search_and_load(&mods_directory, &configs_directory) {
            Ok(mods) => mods,
            Err(errs) => {
                let pretty_errs = errs
                    .into_iter()
                    .map(|report| format!("- {}\n", report))
                    .collect::<String>();

                bail!("Mod loading has failed:\n{pretty_errs}");
            }
        };

        info!(count = %mods.0.len(), "Mods loaded!");

        for r#mod in mods.0.values() {
            debug!(
                id = %r#mod.info.id,
                //version = %r#mod.info.meta.version,
                //authors = ?r#mod.info.meta.authors,
                has_patcher = %r#mod.patcher.is_some(),
                has_pck = %r#mod.patcher.is_some()
            );

            if r#mod.info.id != BUILTIN_MOD_ID {
                r#mod.config.write()?;
            }
        }

        let existing = self.mods.write().replace(mods);
        assert!(existing.is_none(), "ran mod initialization twice!");

        // Load custom engine builds if present.
        let custom_builds_path = self.root_directory.join("builds.toml");
        let custom_builds = if custom_builds_path.exists() {
            let result = try {
                let str = std::fs::read_to_string(custom_builds_path)
                    .context("failed to read custom builds.toml")?;

                toml::from_str::<SerializedBuildsFile>(&str)
                    .context("failed to parse custom builds.toml")?
            };

            if let Err(err) = &result {
                error!(err = %err, "failed to read custom build metadata");
            }

            result.ok()
        } else {
            None
        };

        let engine_builds = resolve_bundled_builds(custom_builds)?;
        self.engine_builds
            .set(engine_builds)
            .expect("loaded engine builds twice!");

        Ok(())
    }

    /// Returns the global instance.
    ///
    /// # Panics
    /// This will panic if [`setup_instance`] hasn't been called.
    pub fn instance() -> &'static GDPatch {
        INSTANCE
            .get()
            .expect("tried to get GDPatch before initialization")
    }

    /// Sets up the logging configuration.
    fn setup_logger(&self) -> color_eyre::Result<()> {
        let level_layer: LevelFilter = self.config.log.level.into();

        let log_directory = self.root_directory.clone();
        let log_file: PathBuf = "output.log".into();

        // Since tracing_appender is built around rolling, just delete the log file ourselves on a clean boot
        let full_log_file = log_directory.join(log_file.clone());
        if full_log_file.exists() {
            std::fs::remove_file(full_log_file).ok();
        }

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(tracing_appender::rolling::never(log_directory, log_file))
            .with_ansi(false);

        let stdout_layer = tracing_subscriber::fmt::layer().with_ansi(self.config.log.console_ansi);

        tracing_subscriber::registry()
            .with(ErrorLayer::default())
            .with(level_layer)
            .with(file_layer)
            .with(stdout_layer)
            .try_init()
            .context("failed to setup tracing")?;

        Ok(())
    }

    pub fn get_config_option(
        &self,
        mod_id: &str,
        section: &str,
        option: &str,
    ) -> Option<toml::Value> {
        if mod_id == BUILTIN_MOD_ID {
            // jank hack to read our config like this
            let str = toml::to_string(&self.config).ok()?;
            let data: HashMap<String, HashMap<String, toml::Value>> = toml::from_str(&str).ok()?;
            data.get(section).and_then(|s| s.get(option)).cloned()
        } else {
            let mods = self.mods.read();
            let mods = mods.as_ref()?;
            let r#mod = mods.0.get(mod_id)?;
            r#mod.config.get_option(section, option).cloned()
        }
    }

    pub fn set_config_option(
        &self,
        mod_id: &str,
        section: &str,
        option: &str,
        value: Option<toml::Value>,
    ) -> color_eyre::Result<()> {
        if mod_id == BUILTIN_MOD_ID {
            // No.
            Ok(())
        } else {
            let mut mods = self.mods.write();
            let mods = mods
                .as_mut()
                .ok_or_eyre("mods should have been initialized")?;
            let r#mod = mods.0.get_mut(mod_id).wrap_err("mod ID does not exist")?;

            r#mod.config.set_option(section, option, value)?;
            Ok(())
        }
    }

    /// Gets a pre-existing virtual pack by path.
    pub fn get_virtual_pack(&self, path: &Path) -> Option<Arc<VirtualPack>> {
        let guard = self.virtual_packs.read();
        let pack = guard.get(path).cloned();
        drop(guard);

        pack
    }

    /// Converts a pack into a virtual pack. This runs Lua hooks, etc.
    pub fn create_virtual_pack(
        &self,
        path: PathBuf,
        file: File,
        old_pack: Pack,
        header_pos_within_file: u64,
    ) -> Arc<VirtualPack> {
        let _entered = debug_span!("create_pack", original = %path.display()).entered();
        debug!("creating virtual pack");

        let mapping = unsafe { Mmap::map(&file).expect("failed to mmap pack") };
        let mapping = Arc::new(mapping);

        assert!(old_pack.deltas.is_empty());
        let mut builder = VirtualPackBuilder::new(&old_pack);

        let mut patchers = Vec::new();
        let mut callbacks = PatcherCallbacks::default();
        let old_pack = Arc::new(old_pack);

        {
            let mods = self.mods.read();
            let mods = mods.as_ref().expect("mods should have been initialized");

            // Initialize mod patcher callbacks.
            for r#mod in mods.0.values() {
                let _entered = info_span!("patcher_setup", mod = %r#mod.info.id).entered();

                if let Some(patcher) = &r#mod.patcher {
                    match ModLua::new(patcher, r#mod.info.id.clone()) {
                        Ok(patcher) => patchers.push(patcher),
                        Err(error) => {
                            error!(?error, "failed to create patcher")
                        }
                    }
                }
            }
        }

        for patcher in &patchers {
            let old_pack = Arc::downgrade(&old_pack);
            let _entered = info_span!("patcher", mod = %patcher.mod_id).entered();

            match patcher.run(old_pack, path.clone()) {
                Ok(mod_callbacks) => callbacks.merge(mod_callbacks),
                Err(err) => error!(?err, mod_id = patcher.mod_id, "failed to run patcher"),
            }
        }

        let engine_build = {
            let version = match &self.config.engine.version {
                // Overwrite the flavor, which is specified separately
                Some(version) => VersionSpecifier::new(
                    version.major,
                    version.minor,
                    version.patch,
                    version.sub_patch,
                    &self.config.engine.flavor,
                ),

                None => VersionSpecifier::new(
                    old_pack.engine_version.0,
                    old_pack.engine_version.1,
                    old_pack.engine_version.2,
                    0,
                    &self.config.engine.flavor,
                ),
            };

            let engine_builds = self
                .engine_builds
                .get()
                .expect("engine builds should have been initialized");
            let engine_build = engine_builds
                .find_approximate_build(&version)
                .expect("failed to resolve engine build");

            info!(version = %engine_build.version, "using engine build");

            if engine_build.version.minor != old_pack.engine_version.1
                || engine_build.version.patch != old_pack.engine_version.2
            {
                let pack_version = format!(
                    "{}.{}.{}",
                    old_pack.engine_version.0, old_pack.engine_version.1, old_pack.engine_version.2
                );
                warn!(pack = pack_version, resolved = %engine_build.version, "unsupported engine minor/patch version - you may encounter issues");
            }

            engine_build.clone()
        };
        let strip_path_prefix = |path: &str| {
            if engine_build.has_prefixless_pck_paths {
                path.to_string()
            } else {
                path.strip_prefix("res://").unwrap_or(path).to_string()
            }
        };
        let ensure_path_prefix = |path: &str| {
            if engine_build.has_prefixless_pck_paths {
                path.strip_prefix("res://").unwrap_or(path).to_string()
            } else {
                if path.starts_with("res://") {
                    path.to_string()
                } else {
                    format!("res://{}", path)
                }
            }
        };

        let gdscript_build = match &engine_build.gdscript {
            GDScriptBuild::V2(v2) => v2,
            _ => unimplemented!("GDScript V1"),
        };

        let project_settings = {
            let path = ensure_path_prefix(ProjectSettings::PROJECT_SETTINGS_FILENAME);

            let file = old_pack
                .files
                .get(&path)
                .expect("project settings file is missing");

            let contents = FileContents::Disk {
                mapping: mapping.clone(),
                offset: file.offset,
                len: file.size,
            };

            let slice = contents.as_slice();

            let mut buf = ReadableMarshalBuffer::new(slice, true);
            ProjectSettings::parse_binary(&mut buf).expect("failed to parse project settings")
        };

        let real_t_is_double = project_settings
            .inner
            .get("application/config/features")
            .map(|p| match p {
                Variant::PackedStringArray(array) => array.iter().any(|f| f == "Double Precision"),
                _ => false,
            })
            .unwrap_or_default();

        let mut uid_cache = UIDCache::default();

        // Rebuild files in the virtual pack.
        for (path, file) in &old_pack.files {
            let normalized_path = strip_path_prefix(path);
            let _entered = debug_span!("pack_entry", path = %normalized_path).entered();

            let contents = FileContents::Disk {
                mapping: mapping.clone(),
                offset: file.offset,
                len: file.size,
            };
            let slice = contents.as_slice();

            // Patch scripts.
            let result = try {
                if self.config.debug.patch_all_scripts
                    || callbacks.has_patcher_for_script(&normalized_path)
                {
                    let script: Option<(Vec<_>, bool)> = if path.ends_with(".gd") {
                        let source = str::from_utf8(slice).wrap_err("failed to parse utf8")?;
                        let mut tokenizer = TokenizerText::new(gdscript_build, source);
                        let mut tokens = parse_to_tokens(&mut tokenizer)?;

                        if self.config.debug.patch_all_scripts {
                            let source = reconstruct_script_text(&tokens);
                            let mut tokenizer = TokenizerText::new(gdscript_build, &source);
                            tokens = parse_to_tokens(&mut tokenizer)?;
                        }

                        Some((tokens, false))
                    } else if path.ends_with(".gdc") {
                        let mut tokenizer = TokenizerBytecode::new(gdscript_build, slice)
                            .wrap_err("failed to parse bytecode")?;

                        let mut tokens = parse_to_tokens(&mut tokenizer)?;

                        if self.config.debug.patch_all_scripts {
                            let source = reconstruct_script_text(&tokens);

                            let mut tokenizer = TokenizerText::new(gdscript_build, &source);
                            tokens = parse_to_tokens(&mut tokenizer)?;
                        }

                        Some((tokens, true))
                    } else {
                        None
                    };

                    if let Some((tokens, is_binary)) = script {
                        match callbacks.patch_script(&normalized_path, tokens, gdscript_build) {
                            Ok(mut patched_tokens) => {
                                let patched_data = if is_binary {
                                    if self.config.debug.patch_all_scripts {
                                        let reconstructed = reconstruct_script_binary(
                                            gdscript_build,
                                            &patched_tokens,
                                            CompressMode::None,
                                            real_t_is_double,
                                        )
                                        .wrap_err("failed to reconstruct script as binary")?;

                                        let mut tokenizer =
                                            TokenizerBytecode::new(gdscript_build, &reconstructed)
                                                .wrap_err("failed to reparse bytecode")?;
                                        patched_tokens = parse_to_tokens(&mut tokenizer)?;
                                    }

                                    reconstruct_script_binary(
                                        gdscript_build,
                                        &patched_tokens,
                                        CompressMode::None,
                                        real_t_is_double,
                                    )
                                    .wrap_err("failed to reconstruct binary script")?
                                } else {
                                    let content = reconstruct_script_text(&patched_tokens);
                                    content.into_bytes()
                                };

                                builder.add_file(
                                    path.clone(),
                                    patched_data.len() as u64,
                                    file.hash,
                                    FileContents::Memory(patched_data),
                                );

                                continue;
                            }
                            Err(err) => error!(?err, "failed to patch script"),
                        };
                    }
                }
            };

            if let Err(err) = result {
                error!(?err, "failed to patch script");
            }

            if normalized_path == UIDCache::UID_CACHE_PATH {
                let mut buffer = ReadableMarshalBuffer::new(slice, true);
                if let Err(err) = uid_cache.merge_decode(&mut buffer) {
                    error!(?err, "failed to decode UID cache");
                }

                // Skip adding UID cache, as it's done later.
                continue;
            }

            // Preserve the original file.
            builder.add_file(path.clone(), file.size, file.hash, contents);
        }

        // Insert modded .pck files.
        // TODO: mod order, maybe?
        {
            let mods = self.mods.read();
            let mods = mods.as_ref().expect("mods should have been initialized");
            for (mod_id, r#mod) in &mods.0 {
                let _entered = info_span!("mod_pack", %mod_id).entered();

                let mod_pack = match &r#mod.pack {
                    Some(mod_pack) => mod_pack,
                    None => continue,
                };

                for (mut path, contents) in mod_pack.files() {
                    let _entered = info_span!("pack_entry", path = %path).entered();

                    // Add/remove res:// prefix if required.
                    path = ensure_path_prefix(&path);
                    let normalized_path = strip_path_prefix(&path);

                    let exists_in_old = old_pack.files.contains_key(&path);
                    let is_script = path.ends_with(".gd") || path.ends_with(".gdc");

                    // Disallow replacing scripts directly.
                    if exists_in_old && is_script {
                        warn!("cannot replace game script in mod pack");
                        continue;
                    }

                    if normalized_path == UIDCache::UID_CACHE_PATH {
                        let mut buffer = ReadableMarshalBuffer::new(contents.as_slice(), true);
                        if let Err(err) = uid_cache.merge_decode(&mut buffer) {
                            error!(?err, "failed to decode UID cache");
                        }
                    }

                    if normalized_path == ProjectSettings::PROJECT_SETTINGS_FILENAME {
                        warn!("skipping project settings file in mod pack");
                        continue;
                    }

                    // TODO: Merge the global script class cache.
                    if normalized_path == ".godot/global_script_class_cache.cfg" {
                        warn!("skipping global script class cache in mod pack");
                        continue;
                    }

                    trace!("adding modded file");
                    builder.add_file(
                        path.clone(),
                        contents.len(),
                        [0u8; 16], // TODO
                        contents,
                    );
                }
            }
        }

        // Patch (and overwrite) project settings.
        // This is done later to enforce a consistent order of operations within the patcher script, making sure all scripts are patched beforehand.
        {
            let result = try {
                match callbacks.patch_project_settings(project_settings.clone()) {
                    Ok(patched_settings) => {
                        let mut patched_data = WritableMarshalBuffer::new(false);
                        patched_settings.encode(&mut patched_data)?;

                        let patched_data = patched_data.into_inner();
                        builder.add_file(
                            ensure_path_prefix(ProjectSettings::PROJECT_SETTINGS_FILENAME),
                            patched_data.len() as u64,
                            [0u8; 16], // TODO
                            FileContents::Memory(patched_data),
                        );
                    }
                    Err(err) => error!(?err, "failed to patch project settings"),
                }
            };

            if let Err(err) = result {
                error!(?err, "failed to patch project settings");
            }
        }

        // Save UID cache.
        {
            let mut buffer = WritableMarshalBuffer::new(false);
            uid_cache.encode(&mut buffer);
            builder.add_file(
                ensure_path_prefix(UIDCache::UID_CACHE_PATH),
                buffer.len() as u64,
                [0u8; 16], // TODO
                FileContents::Memory(buffer.into_inner()),
            );
        }

        let virtual_pack = Arc::new(builder.build(header_pos_within_file));
        self.virtual_packs
            .write()
            .insert(path, virtual_pack.clone());
        virtual_pack
    }
}
