//! Where source files come from: the file system, or memory (tests, unsaved editor buffers).

use std::collections::HashMap;
use std::io;
use std::path::{Component, Path, PathBuf};

pub trait SourceProvider {
    fn read(&self, path: &Path) -> io::Result<String>;
}

/// Reads files from disk.
pub struct FileSystem;

impl SourceProvider for FileSystem {
    fn read(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
}

/// Files held in memory, keyed by normalized path.
#[derive(Default, Clone, Debug)]
pub struct MemoryFiles(pub HashMap<PathBuf, String>);

impl MemoryFiles {
    pub fn new() -> MemoryFiles {
        MemoryFiles::default()
    }

    pub fn with(mut self, path: impl AsRef<Path>, content: impl Into<String>) -> MemoryFiles {
        self.0.insert(normalize(path.as_ref()), content.into());
        self
    }
}

impl SourceProvider for MemoryFiles {
    fn read(&self, path: &Path) -> io::Result<String> {
        self.0
            .get(&normalize(path))
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))
    }
}

/// Two normalized paths name the same file when their keys are equal.
/// Windows file names ignore letter case, so `Lib/Math.has` and
/// `lib/math.has` are one file there.
pub fn same_file_key(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    } else {
        path.to_path_buf()
    }
}

/// Removes `.` and resolves `..` without touching the file system.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
