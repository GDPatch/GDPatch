use color_eyre::eyre::WrapErr;
use gdpatch_godot::{
    build::GDScriptV2Build,
    gdscript::{
        Spanned, Token,
        parser::parse_to_tokens,
        tokenizer::{TokenizerText, reconstruct_script_text},
    },
    pack::Pack,
    project_settings::ProjectSettings,
};
use indexmap::IndexMap;
use mlua::{
    BString, FromLua, Function, IntoLua, Lua, LuaSerdeExt, MultiValue, ObjectLike, Table, UserData,
    UserDataMethods, Value, WeakLua,
};
use serde::Deserialize;
use std::{cmp::Ordering, collections::HashMap, ops::DerefMut, path::PathBuf, sync::Weak};
use tracing::{error, info, info_span, warn};

use crate::{GDPatch, bindings::LuaVariant};

/// Holds a specific mod's Lua runtime.
#[derive(Debug)]
pub struct ModLua {
    pub mod_id: String,
    lua: Lua,
    chunk: Function,
}

impl ModLua {
    /// Creates a new [`ModLua`] with the given source code and chunk name. This will parse the
    /// source into a chunk without running it.
    pub fn new(source: &str, mod_id: String) -> color_eyre::Result<Self> {
        let lua = Lua::new();

        lua.globals()
            .set("GDPatch", GDPatchLuaGlobal)
            .expect("failed to set global on runtime");
        crate::bindings::register_module(&lua)?;

        {
            let utils = include_str!("utils.lua");
            let utils = lua
                .load(utils)
                .eval::<mlua::Table>()
                .expect("utils should never fail to load");

            lua.register_module("gdpatch.utils", utils)?;
        }

        // TODO(jules): make this an actual logger perhaps
        lua.globals().set(
            "print",
            lua.create_function(|lua, vals: MultiValue| {
                let app_data = lua
                    .app_data_ref::<ModLuaState>()
                    .expect("app data should be available");

                let message = vals
                    .iter()
                    .map(|v| {
                        lua.globals()
                            .call_function::<String>("tostring", v)
                            .unwrap()
                    })
                    .collect::<Vec<_>>()
                    .join("\t");

                info!(target: "lua", parent: None, mod_id = app_data.mod_id, "{message}");
                Ok(())
            })?,
        )?;

        let chunk = lua
            .load(source)
            .set_name(&mod_id)
            .into_function()
            .wrap_err("parsing chunk")?;

        Ok(Self { lua, chunk, mod_id })
    }

    /// Resets the runtime state and runs the patcher.
    pub fn run(&self, pack: Weak<Pack>, pack_path: PathBuf) -> mlua::Result<PatcherCallbacks> {
        self.lua.set_app_data(ModLuaState {
            pack,
            pack_path,
            callbacks: Default::default(),
            mod_id: self.mod_id.clone(),
        });

        self.chunk.call::<()>(())?;

        let app_data = self
            .lua
            .app_data_ref::<ModLuaState>()
            .expect("app data should be available");
        let callbacks = app_data.callbacks.clone();
        drop(app_data);

        Ok(callbacks)
    }
}

/// Lua app data for a [`ModLua`] instance.
#[derive(Clone)]
struct ModLuaState {
    pack: Weak<Pack>,
    pack_path: PathBuf,
    callbacks: PatcherCallbacks,
    mod_id: String,
}

#[derive(Deserialize, Default, Clone)]
struct PatcherCallbackOptions {
    #[serde(default)]
    before: Vec<String>,
    #[serde(default)]
    after: Vec<String>,
}

#[derive(Clone)]
pub struct PatcherCallback {
    lua: WeakLua,
    function: Function,
    options: PatcherCallbackOptions,
    mod_id: String,
}

pub type PatcherCallbackMap = HashMap<String, Vec<PatcherCallback>>;

#[derive(Clone, Default)]
pub struct PatcherCallbacks {
    patch_script_as_text: PatcherCallbackMap,
    patch_script_as_ast: PatcherCallbackMap,
    patch_file: PatcherCallbackMap,
    patch_project_settings: Vec<PatcherCallback>,
}

impl PatcherCallbacks {
    /// Merge another [`PatcherCallbacks`] into this one.
    pub fn merge(&mut self, other: Self) {
        let sort_callbacks = |callback: &mut Vec<PatcherCallback>| {
            callback.sort_by(|one, two| {
                if one.options.before.contains(&two.mod_id) {
                    Ordering::Less
                } else if one.options.after.contains(&two.mod_id) {
                    Ordering::Greater
                } else if two.options.after.contains(&one.mod_id) {
                    Ordering::Less
                } else if two.options.before.contains(&one.mod_id) {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            });
        };

        let merge_one = |ours: &mut PatcherCallbackMap, theirs: &PatcherCallbackMap| {
            theirs.iter().for_each(|(path, funcs)| {
                ours.entry(path.clone())
                    .or_default()
                    .extend(funcs.iter().cloned());
            });

            for callback in ours.values_mut() {
                sort_callbacks(callback);
            }
        };

        merge_one(&mut self.patch_script_as_text, &other.patch_script_as_text);
        merge_one(&mut self.patch_script_as_ast, &other.patch_script_as_ast);
        merge_one(&mut self.patch_file, &other.patch_file);

        self.patch_project_settings
            .extend(other.patch_project_settings);
        sort_callbacks(&mut self.patch_project_settings);
    }

    pub fn patch_script(
        &self,
        path: &str,
        mut tokens: Vec<Spanned<Token>>,
        version: &GDScriptV2Build,
    ) -> color_eyre::Result<Vec<Spanned<Token>>> {
        if self.patch_script_as_text.contains_key(path) {
            tokens = self.patch_script_as_text(path, tokens, version)?;
        }

        if self.patch_script_as_ast.contains_key(path) {
            tokens = self.patch_script_as_ast(path, tokens)?;
        }

        Ok(tokens)
    }

    pub fn has_patcher_for_script(&self, path: &str) -> bool {
        self.patch_script_as_text.contains_key(path) || self.patch_script_as_ast.contains_key(path)
    }

    fn patch_script_as_text(
        &self,
        path: &str,
        mut tokens: Vec<Spanned<Token>>,
        version: &GDScriptV2Build,
    ) -> color_eyre::Result<Vec<Spanned<Token>>> {
        if let Some(funcs) = self.patch_script_as_text.get(path) {
            for PatcherCallback {
                lua,
                function,
                mod_id,
                ..
            } in funcs
            {
                let _entered = info_span!("patcher", mod = %mod_id, type = "script_text").entered();
                let lua = lua.upgrade();

                let context = lua.create_table()?;
                context.set("path", path)?;

                // Since we pass the script in as raw tokens, this also serves as a handy normalization step for mods
                let source = reconstruct_script_text(&tokens);

                let new_source = match function.call::<String>((context, source.clone())) {
                    Ok(new_source) => {
                        if source == new_source {
                            warn!("patcher returned identical script output");
                        }
                        new_source
                    }
                    Err(err) => {
                        error!(%err, "failed to run patcher callback");
                        source
                    }
                };

                let mut tokenizer = TokenizerText::new(version, &new_source);
                tokens = parse_to_tokens(&mut tokenizer)?;
            }
        }

        Ok(tokens)
    }

    #[allow(unused)]
    #[allow(unused_assignments)]
    fn patch_script_as_ast(
        &self,
        path: &str,
        mut tokens: Vec<Spanned<Token>>,
    ) -> color_eyre::Result<Vec<Spanned<Token>>> {
        if let Some(funcs) = self.patch_script_as_ast.get(path) {
            for PatcherCallback {
                lua,
                function,
                mod_id,
                ..
            } in funcs
            {
                let _entered = info_span!("patcher", mod = %mod_id, type = "script_ast").entered();
                let lua = lua.upgrade();

                let context = lua.create_table()?;
                context.set("path", path)?;

                // TODO: implement AST representation
                let mut ast = ();

                match function.call::<()>((context, ast)) {
                    Ok(new_ast) => {
                        ast = new_ast;
                    }
                    Err(err) => error!(%err, "failed to run patcher callback"),
                }
            }
        }

        Ok(tokens)
    }

    pub fn patch_project_settings(
        &self,
        mut settings: ProjectSettings,
    ) -> color_eyre::Result<ProjectSettings> {
        for PatcherCallback {
            lua,
            function,
            mod_id,
            ..
        } in &self.patch_project_settings
        {
            let _entered =
                info_span!("patcher", mod = %mod_id, type = "project_settings").entered();
            let lua = lua.upgrade();

            let entries = lua.create_table()?;
            for (key, value) in &settings.inner {
                let entry = lua.create_table()?;

                entry.push(key.clone())?;
                entry.push(LuaVariant(value.clone()).into_lua(&lua)?)?;

                entries.push(entry)?;
            }

            match function.call::<Table>(entries) {
                Ok(new_entries) => {
                    let mut entries = IndexMap::new();

                    for entry in new_entries.sequence_values::<Table>() {
                        let entry = entry?;

                        let key = entry.get::<String>(1)?;
                        let value = LuaVariant::from_lua(entry.get::<Value>(2)?, &lua)?;

                        entries.insert(key, value.0);
                    }

                    settings.inner = entries;
                }
                Err(err) => error!(%err, "failed to run patcher callback"),
            }
        }

        Ok(settings)
    }

    pub fn has_patcher_for_file(&self, path: &str) -> bool {
        self.patch_file.contains_key(path)
    }

    pub fn patch_file(&self, path: &str, input: &[u8]) -> color_eyre::Result<Vec<u8>> {
        let mut data = BString::from(input);
        if let Some(funcs) = self.patch_file.get(path) {
            for PatcherCallback {
                lua,
                function,
                mod_id,
                ..
            } in funcs
            {
                let _entered = info_span!("patcher", mod = %mod_id, type = "file_text").entered();
                let lua = lua.upgrade();

                let context = lua.create_table()?;
                context.set("path", path)?;

                data = match function.call::<BString>((context, data.clone())) {
                    Ok(new_data) => {
                        if data == new_data {
                            warn!("patcher returned identical output");
                        }
                        new_data
                    }
                    Err(err) => {
                        error!(%err, "failed to run patcher callback");
                        data
                    }
                };
            }
        }

        Ok(data.to_vec())
    }
}

/// Wrapper for Lua paths to either accept a string or table of strings.
struct PatcherPaths(Vec<String>);

impl mlua::FromLua for PatcherPaths {
    fn from_lua(value: Value, lua: &Lua) -> mlua::Result<Self> {
        let table = if value.is_string() {
            let str = String::from_lua(value, lua)?;
            vec![str.to_string()]
        } else if value.is_table() {
            Vec::<String>::from_lua(value.clone(), lua)?
        } else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "PatcherPaths".to_string(),
                message: Some("expected string or table".to_string()),
            });
        };

        let table = table
            .iter()
            .map(|path| {
                if let Some(stripped) = path.strip_prefix("res://") {
                    stripped.to_string()
                } else {
                    path.clone()
                }
            })
            .collect();

        Ok(Self(table))
    }
}

struct GDPatchLuaGlobal;

impl UserData for GDPatchLuaGlobal {
    fn add_methods<M>(methods: &mut M)
    where
        M: UserDataMethods<Self>,
    {
        methods.add_function("get_root_directory", |_, (): ()| {
            let gdpatch = GDPatch::instance();
            Ok(gdpatch.get_root_directory())
        });

        methods.add_function("get_mod_directory", |lua, mod_id: Option<String>| {
            let mod_id = mod_id.unwrap_or_else(|| {
                let state = lua
                    .app_data_ref::<ModLuaState>()
                    .expect("runtime state should be available");

                state.mod_id.clone()
            });

            let gdpatch = GDPatch::instance();
            Ok(gdpatch.get_mod_directory(&mod_id))
        });

        methods.add_function(
            "get_config_option",
            |lua, (mod_id, section, option): (Option<String>, String, String)| {
                let mod_id = mod_id.unwrap_or_else(|| {
                    let state = lua
                        .app_data_ref::<ModLuaState>()
                        .expect("runtime state should be available");

                    state.mod_id.clone()
                });

                let gdpatch = GDPatch::instance();
                let value = gdpatch.get_config_option(&mod_id, &section, &option);
                let value = value.map(|v| lua.to_value(&v)).transpose()?;

                Ok(value)
            },
        );

        methods.add_function(
            "set_config_option",
            |lua, (mod_id, section, option, value): (Option<String>, String, String, mlua::Value)| {
                let mod_id = mod_id.unwrap_or_else(|| {
                    let state = lua
                        .app_data_ref::<ModLuaState>()
                        .expect("runtime state should be available");

                    state.mod_id.clone()
                });

                let value: Option<toml::Value> = lua.from_value(value)?;

                let gdpatch = GDPatch::instance();
                gdpatch.set_config_option(&mod_id, &section, &option, value).map_err(mlua::Error::external)?;

                Ok(())
            },
        );

        methods.add_function("get_mods", |lua, (): ()| {
            let gdpatch = GDPatch::instance();
            let mods = gdpatch.mods.read();
            let mods = mods.as_ref().expect("mods should be initialized");
            let mod_infos = mods
                .0
                .values()
                .map(|m| lua.to_value(&m.info))
                .filter_map(|m| m.ok())
                .collect::<Vec<Value>>();
            Ok(mod_infos)
        });

        methods.add_function("get_pack", |lua, (): ()| {
            let state = lua
                .app_data_ref::<ModLuaState>()
                .expect("runtime state should be available");
            let pack = state.pack.upgrade().expect("pack should be available");

            let path = state.pack_path.to_string_lossy();
            let files: Vec<_> = pack.files.keys().cloned().collect();

            let table = lua.create_table()?;
            table.set("path", path)?;
            table.set("files", files)?;

            Ok(table)
        });

        let mut add_patcher =
            |name: &'static str, get_callbacks: fn(&mut ModLuaState) -> &mut PatcherCallbackMap| {
                methods.add_function(
                    name,
                    move |lua, (paths, function, options): (PatcherPaths, Function, Value)| {
                        let options = if options.is_nil() {
                            PatcherCallbackOptions::default()
                        } else {
                            lua.from_value::<PatcherCallbackOptions>(options)?
                        };

                        let mut state = lua
                            .app_data_mut::<ModLuaState>()
                            .expect("runtime state should be available");
                        let mod_id = state.mod_id.clone();

                        for path in paths.0.iter() {
                            get_callbacks(state.deref_mut())
                                .entry(path.clone())
                                .or_default()
                                .push(PatcherCallback {
                                    lua: lua.weak(),
                                    function: function.clone(),
                                    options: options.clone(),
                                    mod_id: mod_id.clone(),
                                });
                        }

                        Ok(())
                    },
                )
            };

        add_patcher("patch_script_as_text", |s| {
            &mut s.callbacks.patch_script_as_text
        });
        add_patcher("patch_script_as_ast", |s| {
            &mut s.callbacks.patch_script_as_ast
        });
        add_patcher("patch_file", |s| &mut s.callbacks.patch_file);

        methods.add_function(
            "patch_project_settings",
            |lua, (function, options): (Function, Value)| {
                let options = if options.is_nil() {
                    PatcherCallbackOptions::default()
                } else {
                    lua.from_value::<PatcherCallbackOptions>(options)?
                };

                let mut state = lua
                    .app_data_mut::<ModLuaState>()
                    .expect("runtime state should be available");
                let mod_id = state.mod_id.clone();

                state
                    .callbacks
                    .patch_project_settings
                    .push(PatcherCallback {
                        lua: lua.weak(),
                        function: function.clone(),
                        options: options.clone(),
                        mod_id: mod_id.clone(),
                    });

                Ok(())
            },
        );
    }
}
