use std::fmt;
use std::path::PathBuf;

/// A position in the sources. `file` indexes [`crate::Program::files`]
/// (or the file list of the failed assembly).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loc {
    pub file: usize,
    /// 1-based. 0 = whole file / no line.
    pub line: u32,
    /// 1-based. 0 = whole line.
    pub col: u32,
}

impl Loc {
    pub const fn new(file: usize, line: u32, col: u32) -> Loc {
        Loc { file, line, col }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// An error or warning, with its location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// `None` for errors that concern the whole project.
    pub file: Option<PathBuf>,
    pub line: u32,
    pub col: u32,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match &self.file {
            Some(file) if self.line > 0 => {
                write!(
                    f,
                    "{}:{}:{}: {kind}: {}",
                    file.display(),
                    self.line,
                    self.col.max(1),
                    self.message
                )
            }
            Some(file) => write!(f, "{}: {kind}: {}", file.display(), self.message),
            None => write!(f, "{kind}: {}", self.message),
        }
    }
}

/// Collects diagnostics during assembly, before file paths are attached.
#[derive(Default, Debug)]
pub(crate) struct Report {
    pub entries: Vec<(Severity, Option<Loc>, String)>,
}

impl Report {
    pub fn error(&mut self, loc: Loc, message: impl Into<String>) {
        self.entries
            .push((Severity::Error, Some(loc), message.into()));
    }

    pub fn global_error(&mut self, message: impl Into<String>) {
        self.entries.push((Severity::Error, None, message.into()));
    }

    pub fn warning(&mut self, loc: Loc, message: impl Into<String>) {
        self.entries
            .push((Severity::Warning, Some(loc), message.into()));
    }

    pub fn has_errors(&self) -> bool {
        self.entries.iter().any(|(s, _, _)| *s == Severity::Error)
    }

    pub fn into_diagnostics(self, files: &[PathBuf]) -> Vec<Diagnostic> {
        let mut out: Vec<Diagnostic> = self
            .entries
            .into_iter()
            .map(|(severity, loc, message)| Diagnostic {
                severity,
                file: loc.and_then(|l| files.get(l.file).cloned()),
                line: loc.map_or(0, |l| l.line),
                col: loc.map_or(0, |l| l.col),
                message,
            })
            .collect();
        out.sort_by(|a, b| (&a.file, a.line, a.col).cmp(&(&b.file, b.line, b.col)));
        out
    }
}
