//! A project is a folder holding its entry file: `main.has`, where the
//! assembler starts, or `main.c` for a C project, which builds every `.c`
//! and `.has` file of the folder (see [`Project::c_sources`]).

use std::io;
use std::path::{Path, PathBuf};

pub const ENTRY_FILE: &str = "main.has";
pub const C_ENTRY_FILE: &str = "main.c";
pub const C_MAIN_TEMPLATE: &str = include_str!("../../../templates/main.c");
pub const MAIN_TEMPLATE: &str = include_str!("../../../templates/main.has");
pub const INTERRUPT_HANDLER_FILE: &str = "interruptHandler.has";
pub const INTERRUPT_HANDLER_TEMPLATE: &str =
    include_str!("../../../templates/interruptHandler.has");
/// Content of a `.has` file created with "New file".
pub const NEW_FILE_TEMPLATE: &str = "\
; === DEFINES ===


; === VARIABLES ===


; === PROGRAM ===
";

/// The content of a file created with "New file", by its name.
pub fn new_file_content(name: &str) -> &'static str {
    if has_extension(Path::new(name), &["has"]) {
        NEW_FILE_TEMPLATE
    } else {
        ""
    }
}

/// The language a project is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Assembly,
    C,
}

#[derive(Clone, Debug)]
pub struct Project {
    /// Absolute path of the folder.
    pub root: PathBuf,
    /// C when the folder holds `main.c`.
    pub language: Language,
    /// Every source file (`.has`, `.c`, `.h`) in the folder and its
    /// subfolders, sorted.
    pub files: Vec<PathBuf>,
    /// Every subfolder, empty ones included, sorted.
    pub folders: Vec<PathBuf>,
}

impl Project {
    /// Opens a project folder. It must contain `main.c` (a C project) or
    /// `main.has`.
    pub fn open(root: &Path) -> io::Result<Project> {
        let root = absolute(root)?;
        let language = if root.join(C_ENTRY_FILE).is_file() {
            Language::C
        } else if root.join(ENTRY_FILE).is_file() {
            Language::Assembly
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "{} has no {ENTRY_FILE} (or {C_ENTRY_FILE} for a C project)",
                    root.display()
                ),
            ));
        };
        let mut project = Project {
            root,
            language,
            files: Vec::new(),
            folders: Vec::new(),
        };
        project.rescan();
        Ok(project)
    }

    /// Creates the entry file from its template in `root` (created if
    /// needed): `main.c` for C, otherwise `main.has`, with
    /// `interruptHandler.has` when `interrupt_handler` is set. Then opens
    /// it. Existing files are kept.
    pub fn create(root: &Path, language: Language, interrupt_handler: bool) -> io::Result<Project> {
        std::fs::create_dir_all(root)?;
        let main = main_source(interrupt_handler);
        let mut files = match language {
            Language::C => vec![(C_ENTRY_FILE, C_MAIN_TEMPLATE)],
            Language::Assembly => vec![(ENTRY_FILE, main.as_str())],
        };
        if interrupt_handler && language == Language::Assembly {
            files.push((INTERRUPT_HANDLER_FILE, INTERRUPT_HANDLER_TEMPLATE));
        }
        for (name, content) in files {
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
        self.root.join(match self.language {
            Language::C => C_ENTRY_FILE,
            Language::Assembly => ENTRY_FILE,
        })
    }

    /// The files a C project builds: every `.c` and `.has` file, `main.c`
    /// first (unit 1), then in name order.
    pub fn c_sources(&self) -> Vec<PathBuf> {
        let entry = self.entry();
        let mut sources = vec![entry.clone()];
        sources.extend(
            self.files
                .iter()
                .filter(|p| **p != entry && has_extension(p, &["c", "has"]))
                .cloned(),
        );
        sources
    }

    /// Where a C project's compiled files go (hidden: not in `files`).
    pub fn build_dir(&self) -> PathBuf {
        self.root.join(hbc2_cc::BUILD_DIR)
    }

    /// Re-reads the file and folder lists from disk. Returns true if they changed.
    pub fn rescan(&mut self) -> bool {
        let mut files = Vec::new();
        let mut folders = Vec::new();
        collect_sources(&self.root, &mut files, &mut folders, 0);
        files.sort();
        folders.sort();
        let changed = files != self.files || folders != self.folders;
        self.files = files;
        self.folders = folders;
        changed
    }

    /// How `main.has` includes `path`: relative to the project folder, with `/`.
    pub fn include_path(&self, path: &Path) -> String {
        self.relative(path)
            .iter()
            .map(|part| part.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Path relative to the project folder, for display.
    pub fn relative<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.root).unwrap_or(path)
    }
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>, folders: &mut Vec<PathBuf>, depth: usize) {
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
            collect_sources(&path, out, folders, depth + 1);
            folders.push(path);
        } else if kind.is_file() && has_extension(&path, &["has", "c", "h"]) {
            out.push(path);
        }
    }
}

/// The file's extension is one of `extensions` (any case).
pub fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// The `main.has` template, without its `.include` of the interrupt handler
/// when the project has none.
fn main_source(interrupt_handler: bool) -> String {
    if interrupt_handler {
        return MAIN_TEMPLATE.to_string();
    }
    let include = format!(".include \"{INTERRUPT_HANDLER_FILE}\"");
    MAIN_TEMPLATE
        .split_inclusive('\n')
        .filter(|line| line.trim() != include)
        .collect()
}

/// Name of the folder, inside the user's Documents, where project dialogs open.
pub const PROJECTS_DIR_NAME: &str = "hbc2_projects";

/// `Documents/hbc2_projects`, or `None` when the system has no Documents folder.
pub fn projects_dir() -> Option<PathBuf> {
    dirs::document_dir().map(|docs| docs.join(PROJECTS_DIR_NAME))
}

/// Where the "New project" dialog creates projects unless another folder is chosen.
pub fn default_projects_parent() -> PathBuf {
    projects_dir()
        .filter(|dir| dir.is_dir())
        .or_else(dirs::document_dir)
        .or_else(dirs::home_dir)
        .unwrap_or_default()
}

/// Creates `Documents/hbc2_projects` if it is missing.
pub fn create_projects_dir() -> io::Result<()> {
    match projects_dir() {
        Some(dir) => std::fs::create_dir_all(dir),
        None => Ok(()),
    }
}

/// Absolute and normalized, without resolving links (which on Windows would
/// add a `\\?\` prefix the assembler's paths do not have).
pub fn absolute(path: &Path) -> io::Result<PathBuf> {
    Ok(hbc2_asm::source::normalize(&std::path::absolute(path)?))
}

/// Trims a file or folder name typed by the user and checks it.
fn checked_name(name: &str) -> Result<&str, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The name is empty.");
    }
    if name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) || name.starts_with('.') {
        return Err("The name contains characters that are not allowed in file names.");
    }
    Ok(name)
}

/// Checks a project name typed by the user: it becomes the project's folder.
pub fn project_folder_name(name: &str) -> Result<&str, &'static str> {
    checked_name(name)
}

/// Checks a folder name typed by the user.
pub fn folder_name(name: &str) -> Result<&str, &'static str> {
    checked_name(name)
}

/// Checks a file name typed by the user and adds `.has` (`.c` in a C
/// project) if it has no extension.
pub fn source_file_name(name: &str, language: Language) -> Result<String, &'static str> {
    let name = checked_name(name)?;
    Ok(if Path::new(name).extension().is_some() {
        name.to_string()
    } else {
        match language {
            Language::C => format!("{name}.c"),
            Language::Assembly => format!("{name}.has"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_open_and_scan() {
        let dir = std::env::temp_dir().join(format!("hbc2-ide-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = Project::create(&dir.join("demo"), Language::Assembly, true).unwrap();
        assert_eq!(project.name(), "demo");
        assert_eq!(project.files.len(), 2);
        assert!(project.entry().is_absolute());

        std::fs::create_dir(project.root.join("lib")).unwrap();
        std::fs::write(project.root.join("lib/math.has"), "").unwrap();
        std::fs::write(project.root.join("notes.txt"), "").unwrap();
        let mut project = Project::open(&project.root).unwrap();
        assert_eq!(project.files.len(), 3);
        assert_eq!(project.folders, vec![project.root.join("lib")]);
        assert_eq!(
            project.include_path(&project.root.join("lib/math.has")),
            "lib/math.has"
        );
        assert!(!project.rescan(), "nothing changed");

        std::fs::create_dir(project.root.join("empty")).unwrap();
        assert!(project.rescan(), "empty folders count");
        assert_eq!(project.folders.len(), 2);

        assert!(Project::open(&dir).is_err(), "no main.has");

        let bare = Project::create(&dir.join("bare"), Language::Assembly, false).unwrap();
        assert_eq!(bare.files, vec![bare.entry()]);
        let main = std::fs::read_to_string(bare.entry()).unwrap();
        assert!(!main.contains(INTERRUPT_HANDLER_FILE));
        assert!(main.contains("_start:"));

        let c = Project::create(&dir.join("c"), Language::C, true).unwrap();
        assert_eq!(c.language, Language::C);
        assert_eq!(
            c.files,
            vec![c.root.join(C_ENTRY_FILE)],
            "no interrupt handler"
        );
        std::fs::write(c.root.join("a.c"), "").unwrap();
        std::fs::write(c.root.join("a.h"), "").unwrap();
        std::fs::write(c.root.join("b.has"), "").unwrap();
        std::fs::create_dir(c.build_dir()).unwrap();
        std::fs::write(c.build_dir().join("main_1.has"), "").unwrap();
        let c = Project::open(&c.root).unwrap();
        assert_eq!(c.files.len(), 4, "the build folder is hidden");
        assert_eq!(
            c.c_sources(),
            ["main.c", "a.c", "b.has"].map(|n| c.root.join(n))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_names() {
        let asm = Language::Assembly;
        assert_eq!(source_file_name(" math ", asm), Ok("math.has".into()));
        assert_eq!(source_file_name("math", Language::C), Ok("math.c".into()));
        assert_eq!(source_file_name("data.inc", asm), Ok("data.inc".into()));
        assert!(source_file_name("", asm).is_err());
        assert!(source_file_name("a/b", asm).is_err());
        assert_eq!(project_folder_name(" demo "), Ok("demo"));
        assert!(project_folder_name("a:b").is_err());
        assert_eq!(folder_name(" lib "), Ok("lib"));
        assert!(folder_name("a/b").is_err());
    }
}
