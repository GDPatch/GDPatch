use crate::{
    GDPatchPack,
    gdextension::{bindings::GDClassShaped, sys::GDExtensionRc},
};
use gobbind::gobbind;
use parking_lot::{Mutex, ReentrantMutex};
use std::{
    cell::RefCell,
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::SystemTime,
};

static CURRENT_PACK: LazyLock<ReentrantMutex<RefCell<Option<GDExtensionRc<GDPatchPack>>>>> =
    LazyLock::new(Default::default);

#[derive(Default)]
pub struct Filesilly;

impl GDClassShaped for Filesilly {
    fn new() -> Self {
        Self
    }
}

#[gobbind]
impl Filesilly {
    pub fn init(project_path: String, pack: GDExtensionRc<GDPatchPack>) {
        let current_pack = CURRENT_PACK.lock();
        let mut current_pack = current_pack.borrow_mut();
        current_pack.replace(pack);

        let project_path = PathBuf::from(project_path);

        filesilly::init(
            &[&project_path],
            Box::new(GDPatchEditorStreamFactory(project_path.to_owned())),
        )
        .expect("failed to init filesilly");
    }
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
            return Ok(filesilly::StatResult::Passthrough);
        }

        let Ok(stripped_path) = path.strip_prefix(&self.0) else {
            return Ok(filesilly::StatResult::Passthrough);
        };
        let stripped_path = normalize_path(stripped_path);

        let current_pack = CURRENT_PACK.lock();
        let current_pack = current_pack.borrow();
        if let Some(current_pack) = current_pack.as_ref()
            && let Some(info) = current_pack.get().get_file_info(&stripped_path)
        {
            return Ok(filesilly::StatResult::Exists(filesilly::Stat {
                size: info.size,
                // TODO
                access_time: SystemTime::now(),
                modification_time: SystemTime::now(),
                change_time: SystemTime::now(),
                creation_time: SystemTime::now(),
            }));
        }

        Ok(filesilly::StatResult::Passthrough)
    }

    fn open(&self, path: &Path) -> std::io::Result<Option<filesilly::HeapStream>> {
        let Ok(stripped_path) = path.strip_prefix(&self.0) else {
            return Ok(None);
        };
        let stripped_path = normalize_path(stripped_path);

        // Never proxy the original project file.
        if stripped_path == "project.godot" {
            return Ok(None);
        }

        // Never proxy the shader cache.
        // TODO: why does this crash?
        if stripped_path.starts_with(".godot/shader_cache/") {
            return Ok(None);
        }

        let current_pack = CURRENT_PACK.lock();
        let current_pack = current_pack.borrow();
        if let Some(current_pack) = current_pack.as_ref()
            && let Some(file) = current_pack.get().get_file(&stripped_path)
        {
            let file = Cursor::new(file);
            let file = FilesillyBytesWrapper(file);
            let file = Arc::new(Mutex::new(file));
            return Ok(Some(file));
        }

        Ok(None)
    }
}

fn normalize_path(path: &Path) -> String {
    path.display().to_string().replace("\\", "/")
}
