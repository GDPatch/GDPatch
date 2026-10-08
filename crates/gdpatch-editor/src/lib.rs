use filesilly::{Stat, StatResult};
use gdpatch_godot::{
    ReadableMarshalBuffer, UIDCache,
    build::{EngineBuild, VersionSpecifier, bundled_builds},
    pack::{Pack, PackConfig, PackedFile},
    project_settings::ProjectSettings,
};
use memmap2::Mmap;
use parking_lot::{Mutex, ReentrantMutex};
use std::{
    cell::RefCell,
    fs::File,
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::SystemTime,
};

mod gdextension;

#[path = "macro_rt.rs"]
#[doc(hidden)]
mod __rt;

static CURRENT_PACK: LazyLock<ReentrantMutex<RefCell<Option<LoadedPack>>>> =
    LazyLock::new(Default::default);

pub struct LoadedPack {
    pack: Pack,
    mapping: Mmap,
    build: EngineBuild,
    uid_cache: UIDCache,
}

fn normalize_path(path: &Path) -> String {
    // FIXME this kinda sucks
    path.display().to_string().replace("\\", "/")
}

fn format_pack_path(path: &str, build: &EngineBuild) -> String {
    if build.has_prefixless_pck_paths {
        path.to_string()
    } else {
        format!("res://{}", path)
    }
}

pub fn get_file(pack: &Pack, mapping: &Mmap, build: &EngineBuild, path: &str) -> Option<Vec<u8>> {
    let path = format_pack_path(path, build);
    let entry = pack.files.get(&path)?;

    let data = unsafe {
        let ptr = mapping.as_ptr().add(entry.offset as usize);
        std::slice::from_raw_parts(ptr, entry.size as usize)
    };

    Some(data.to_vec())
}

impl LoadedPack {
    pub fn new(pack: Pack, mapping: Mmap, build: EngineBuild) -> Self {
        let project_settings = get_file(
            &pack,
            &mapping,
            &build,
            ProjectSettings::PROJECT_SETTINGS_PATH,
        )
        .and_then(|bytes| {
            let mut buffer = ReadableMarshalBuffer::new(&bytes, true);
            ProjectSettings::parse_binary(&mut buffer).ok()
        });

        let use_hidden_project_data_directory = project_settings
            .map(|p| p.use_hidden_project_data_directory())
            .unwrap_or(true);

        let uid_cache = get_file(
            &pack,
            &mapping,
            &build,
            &format!(
                "{}/{}",
                if use_hidden_project_data_directory {
                    ".godot"
                } else {
                    "godot"
                },
                UIDCache::UID_CACHE_FILENAME
            ),
        )
        .and_then(|bytes| {
            let mut buffer = ReadableMarshalBuffer::new(&bytes, true);
            UIDCache::decode(&mut buffer).ok()
        })
        .unwrap_or_default();

        Self {
            pack,
            mapping,
            build,
            uid_cache,
        }
    }

    pub fn file_exists(&self, path: &str) -> bool {
        let path = format_pack_path(path, &self.build);
        self.pack.files.get(&path).is_some()
    }

    pub fn get_file_info(&self, path: &str) -> Option<&PackedFile> {
        let path = format_pack_path(path, &self.build);
        self.pack.files.get(&path)
    }

    pub fn get_file(&self, path: &str) -> Option<Vec<u8>> {
        get_file(&self.pack, &self.mapping, &self.build, path)
    }

    pub fn get_uid(&self, path: &str) -> Option<u64> {
        let path = format!("res://{}", path);
        self.uid_cache.0.iter().find(|e| e.1 == path).map(|e| e.0.0)
    }
}

#[derive(Default)]
pub struct GDPatchPack {}

#[gobbind_macros::expose(GDPatchPack)]
impl GDPatchPack {
    pub fn set_pack(project_path: String, pack_path: String) {
        let project_path = PathBuf::from(project_path);
        init_filesilly(&project_path);

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

        let mapping = unsafe { Mmap::map(&file).expect("failed to mmap pack") };
        let loaded_pack = LoadedPack::new(pack, mapping, build);

        let current_pack = CURRENT_PACK.lock();
        let mut current_pack = current_pack.borrow_mut();
        current_pack.replace(loaded_pack);
    }

    pub fn file_exists(path: String) -> bool {
        let current_pack = CURRENT_PACK.lock();
        let current_pack = current_pack.borrow();
        if let Some(current_pack) = current_pack.as_ref() {
            current_pack.file_exists(&path)
        } else {
            false
        }
    }

    pub fn get_file(path: String) -> Option<Vec<u8>> {
        let current_pack = CURRENT_PACK.lock();
        let current_pack = current_pack.borrow();
        if let Some(current_pack) = current_pack.as_ref() {
            current_pack.get_file(&path)
        } else {
            None
        }
    }

    pub fn get_uid(path: String) -> Option<u64> {
        let current_pack = CURRENT_PACK.lock();
        let current_pack = current_pack.borrow();
        if let Some(current_pack) = current_pack.as_ref() {
            current_pack.get_uid(&path)
        } else {
            None
        }
    }
}

pub fn init_filesilly(base_path: &Path) {
    filesilly::init(
        &[base_path],
        Box::new(GDPatchEditorStreamFactory(base_path.to_owned())),
    )
    .expect("failed to init filesilly");
}

#[derive(Debug)]
struct FilesillyBytesWrapper(Cursor<Vec<u8>>);

impl Read for FilesillyBytesWrapper {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for FilesillyBytesWrapper {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for FilesillyBytesWrapper {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

impl filesilly::Stream for FilesillyBytesWrapper {}

#[derive(Debug)]
pub struct GDPatchEditorStreamFactory(PathBuf);

impl filesilly::StreamFactory for GDPatchEditorStreamFactory {
    fn stat(&self, path: &Path) -> std::io::Result<filesilly::StatResult> {
        // Never replace files on disk.
        if path.exists() {
            return Ok(StatResult::Passthrough);
        }

        let Ok(stripped_path) = path.strip_prefix(&self.0) else {
            return Ok(StatResult::Passthrough);
        };
        let stripped_path = normalize_path(stripped_path);

        let current_pack = CURRENT_PACK.lock();
        let current_pack = current_pack.borrow();
        if let Some(current_pack) = current_pack.as_ref()
            && let Some(info) = current_pack.get_file_info(&stripped_path)
        {
            return Ok(StatResult::Exists(Stat {
                size: info.size,
                // TODO
                access_time: SystemTime::now(),
                modification_time: SystemTime::now(),
                change_time: SystemTime::now(),
                creation_time: SystemTime::now(),
            }));
        }

        Ok(StatResult::Passthrough)
    }

    fn open(&self, path: &Path) -> std::io::Result<Option<filesilly::HeapStream>> {
        let Ok(stripped_path) = path.strip_prefix(&self.0) else {
            return Ok(None);
        };
        let stripped_path = normalize_path(stripped_path);

        if stripped_path != "project.godot" && !stripped_path.starts_with(".godot/shader_cache") {
            let current_pack = CURRENT_PACK.lock();
            let current_pack = current_pack.borrow();
            if let Some(current_pack) = current_pack.as_ref()
                && let Some(file) = current_pack.get_file(&stripped_path)
            {
                let file = Cursor::new(file);
                let file = FilesillyBytesWrapper(file);
                let file = Arc::new(Mutex::new(file));
                return Ok(Some(file));
            }
        }

        Ok(None)
    }
}
