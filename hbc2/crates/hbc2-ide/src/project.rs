//! A project is a folder holding `main.has`, the file the assembler starts from.

use std::io;
use std::path::{Path, PathBuf};

pub const ENTRY_FILE: &str = "main.has";
pub const MAIN_TEMPLATE: &str = include_str!("../../../templates/main.has");
pub const INTERRUPT_HANDLER_FILE: &str = "interruptHandler.has";
pub const INTERRUPT_HANDLER_TEMPLATE: &str =
    include_str!("../../../templates/interruptHandler.has");
/// Content of a file created with "New file".
pub const NEW_FILE_TEMPLATE: &str = "\
; === DEFINES ===


; === VARIABLES ===


; === PROGRAM ===
";

#[derive(Clone, Debug)]
pub struct Project {
    /// Absolute path of the folder.
    pub root: PathBuf,
    /// Every `.has` file in the folder and its subfolders, sorted.
    pub files: Vec<PathBuf>,
}

impl Project {
    /// Opens a project folder. It must contain `main.has`.
    pub fn open(root: &Path) -> io::Result<Project> {
        let root = absolute(root)?;
        if !root.join(ENTRY_FILE).is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} has no {ENTRY_FILE}", root.display()),
            ));
        }
        let mut project = Project {
            root,
            files: Vec::new(),
        };
        project.rescan();
        Ok(project)
    }

    /// Creates `main.has` and `interruptHandler.has` from the templates in
    /// `root` (created if needed), then opens it. Existing files are kept.
    pub fn create(root: &Path) -> io::Result<Project> {
        std::fs::create_dir_all(root)?;
        for (name, content) in [
            (ENTRY_FILE, MAIN_TEMPLATE),
            (INTERRUPT_HANDLER_FILE, INTERRUPT_HANDLER_TEMPLATE),
        ] {
            let path = root.join(name);
            if !path.exists() {
                std::fs::write(path, content)?;
            }
        }
        Project::open(root)
    }

    pub fn name(&self) -> String {
        self.root.file_name().map_or_else(
            || self.root.display().to_string(),
            |n| n.to_string_lossy().into(),
        )
    }

    pub fn entry(&self) -> PathBuf {
        self.root.join(ENTRY_FILE)
    }

    /// Re-reads the file list from disk. Returns true if it changed.
    pub fn rescan(&mut self) -> bool {
        let mut files = Vec::new();
        collect_sources(&self.root, &mut files, 0);
        files.sort();
        let changed = files != self.files;
        self.files = files;
        changed
    }

    /// Path relative to the project folder, for display.
    pub fn relative<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.root).unwrap_or(path)
    }
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    const MAX_DEPTH: usize = 8;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let hidden = entry.file_name().to_string_lossy().starts_with('.');
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() && !hidden && depth < MAX_DEPTH {
            collect_sources(&path, out, depth + 1);
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("has"))
        {
            out.push(path);
        }
    }
}

/// Absolute and normalized, without resolving links (which on Windows would
/// add a `\\?\` prefix the assembler's paths do not have).
pub fn absolute(path: &Path) -> io::Result<PathBuf> {
    Ok(hbc2_asm::source::normalize(&std::path::absolute(path)?))
}

/// Checks a file name typed by the user and adds `.has` if it has no extension.
pub fn source_file_name(name: &str) -> Result<String, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The name is empty.");
    }
    if name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) || name.starts_with('.') {
        return Err("The name contains characters that are not allowed in file names.");
    }
    Ok(if Path::new(name).extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.has")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_open_and_scan() {
        let dir = std::env::temp_dir().join(format!("hbc2-ide-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = Project::create(&dir.join("demo")).unwrap();
        assert_eq!(project.name(), "demo");
        assert_eq!(project.files.len(), 2);
        assert!(project.entry().is_absolute());

        std::fs::create_dir(project.root.join("lib")).unwrap();
        std::fs::write(project.root.join("lib/math.has"), "").unwrap();
        std::fs::write(project.root.join("notes.txt"), "").unwrap();
        let mut project = Project::open(&project.root).unwrap();
        assert_eq!(project.files.len(), 3);
        assert!(!project.rescan(), "nothing changed");

        assert!(Project::open(&dir).is_err(), "no main.has");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_names() {
        assert_eq!(source_file_name(" math "), Ok("math.has".into()));
        assert_eq!(source_file_name("data.inc"), Ok("data.inc".into()));
        assert!(source_file_name("").is_err());
        assert!(source_file_name("a/b").is_err());
    }
}
