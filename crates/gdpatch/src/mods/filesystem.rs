use crate::virtual_pack::FileContents;
use color_eyre::eyre::{Context as _, OptionExt, eyre};
use memmap2::Mmap;
use std::{
    collections::{HashMap, HashSet},
    io::Seek,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Filesystem operations for loading mods.
/// This is abstracted so mods can be loaded from in-memory.
pub trait ModLoaderFs {
    /// Check if a path exists.
    fn exists(&self, path: &Path) -> color_eyre::Result<bool>;

    /// Check if a path is a directory.
    fn is_dir(&self, path: &Path) -> color_eyre::Result<bool>;

    /// Read a path, returning a [`FileContents`] mapping.
    fn read(&self, path: &Path) -> color_eyre::Result<FileContents>;

    /// List a directory and returns a list of filenames.
    fn read_dir(&self, path: &Path) -> color_eyre::Result<HashSet<String>>;
}

/// Filesystem operations based off of a root folder.
pub struct ModLoaderFolderFs {
    root: PathBuf,
}

impl ModLoaderFolderFs {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl ModLoaderFs for ModLoaderFolderFs {
    fn exists(&self, path: &Path) -> color_eyre::Result<bool> {
        Ok(self.root.join(path).exists())
    }

    fn is_dir(&self, path: &Path) -> color_eyre::Result<bool> {
        Ok(self.root.join(path).is_dir())
    }

    fn read(&self, path: &Path) -> color_eyre::Result<FileContents> {
        let path = self.root.join(path);

        let mut file = std::fs::File::open(path).wrap_err("reading file")?;
        let len = file.stream_len().wrap_err("getting file size")?;

        let mapping = unsafe { Mmap::map(&file).expect("failed to mmap file") };
        let mapping = Arc::new(mapping);

        Ok(FileContents::Disk {
            mapping,
            offset: 0,
            len,
        })
    }

    fn read_dir(&self, path: &Path) -> color_eyre::Result<HashSet<String>> {
        let path = self.root.join(path);
        let mut result = HashSet::new();

        for file in std::fs::read_dir(path)? {
            let file = file?;
            let name = file
                .file_name()
                .into_string()
                .map_err(|_| eyre!("failed to parse filename"))?;

            result.insert(name);
        }

        Ok(result)
    }
}

/// Filesystem operations based off of a HashMap.
pub struct ModLoaderMapFs {
    inner: HashMap<String, Vec<u8>>,
}

impl ModLoaderMapFs {
    pub fn new(inner: HashMap<String, Vec<u8>>) -> Self {
        Self { inner }
    }
}

impl ModLoaderFs for ModLoaderMapFs {
    fn exists(&self, path: &Path) -> color_eyre::Result<bool> {
        let key = path
            .to_str()
            .ok_or_eyre("failed to stringify path")?
            .replace("\\", "/");

        Ok(self.is_dir(path).unwrap_or_default() || self.inner.contains_key(&key))
    }

    fn is_dir(&self, path: &Path) -> color_eyre::Result<bool> {
        let key = path
            .to_str()
            .ok_or_eyre("failed to stringify path")?
            .replace("\\", "/");

        let dir = format!("{}/", key);
        Ok(self.inner.keys().any(|f| f.starts_with(&dir)))
    }

    fn read(&self, path: &Path) -> color_eyre::Result<FileContents> {
        let key = path
            .to_str()
            .ok_or_eyre("failed to stringify path")?
            .replace("\\", "/");

        let data = self.inner.get(&key).ok_or_eyre("failed to read path")?;
        let contents = FileContents::Memory(data.clone());
        Ok(contents)
    }

    fn read_dir(&self, path: &Path) -> color_eyre::Result<HashSet<String>> {
        let key = path
            .to_str()
            .ok_or_eyre("failed to stringify path")?
            .replace("\\", "/");
        let parts = key.split('/').collect::<Vec<_>>();
        let dir = format!("{}/", key);

        // the things I do instead of a tree structure
        Ok(self
            .inner
            .keys()
            .filter(|p| p.starts_with(&dir))
            .map(|p| p.split('/').collect::<Vec<_>>())
            .map(|p| p[parts.len()].to_string())
            .collect())
    }
}
