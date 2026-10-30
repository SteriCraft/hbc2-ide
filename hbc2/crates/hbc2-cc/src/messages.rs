//! Messages of the compiler stages and of the assembler, turned into
//! [`Diagnostic`]s about the C files.

use std::path::{Path, PathBuf};

use hbc2_asm::source::normalize;
use hbc2_asm::{Diagnostic, Severity};

use crate::lines::LineMaps;

/// A message about no file in particular.
pub(crate) fn plain(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        file: None,
        line: 0,
        col: 0,
        message: message.into(),
    }
}

/// The messages a stage wrote on stderr, one per line. `source` is the C
/// file being compiled; `include` the folders searched for a file that a
/// message names without its folder (cc0 and cc1 do that). Lines in none of
/// the known forms are about `source`, without a line.
pub(crate) fn tool_messages(
    text: &str,
    severity: Severity,
    source: &Path,
    include: &[PathBuf],
) -> Vec<Diagnostic> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut d = Diagnostic {
                severity,
                file: Some(source.to_path_buf()),
                line: 0,
                col: 0,
                message: line.to_string(),
            };
            if let Some((file, n, message)) = cpp_form(line) {
                // cpp joins its -I folders and the name with '/'
                d.file = Some(normalize(Path::new(file)));
                // cpp has already counted the line of the directive
                d.line = n.saturating_sub(1).max(1);
                d.message = message;
                if message_is_warning(line) {
                    d.severity = Severity::Warning;
                }
            } else if let Some((name, n, message)) = cc_form(line) {
                match find_file(name, source, include) {
                    Some(file) => {
                        d.file = Some(file);
                        d.line = n;
                        d.message = message.to_string();
                    }
                    None => d.message = format!("{name}, line {n}: {message}"),
                }
            }
            d
        })
        .collect()
}

/// `name:12 - message` (cc0, cc1): name, line, message.
fn cc_form(line: &str) -> Option<(&str, u32, &str)> {
    for (i, _) in line.match_indices(':') {
        let rest = &line[i + 1..];
        let digits = rest.find(|c: char| !c.is_ascii_digit())?;
        if digits > 0
            && let Some(message) = rest[digits..].strip_prefix(" - ")
        {
            return Some((&line[..i], rest[..digits].parse().ok()?, message));
        }
    }
    None
}

/// `[ 'what' - ]file:12:error: message` (cpp): file, line, message (with
/// `what` at its end).
fn cpp_form(line: &str) -> Option<(&str, u32, String)> {
    let (what, line) = match line.strip_prefix('\'').and_then(|l| l.split_once("' - ")) {
        Some((what, rest)) => (Some(what), rest),
        None => (None, line),
    };
    for kind in [":error: ", ":warning: "] {
        let Some(at) = line.find(kind) else {
            continue;
        };
        let (place, message) = (&line[..at], &line[at + kind.len()..]);
        let (file, n) = place.rsplit_once(':')?;
        let n = n.parse().ok()?;
        let message = match what {
            Some(what) => format!("{message} '{what}'"),
            None => message.to_string(),
        };
        return Some((file, n, message));
    }
    None
}

fn message_is_warning(line: &str) -> bool {
    line.contains(":warning: ")
}

/// The file a message names by its file name only: the C file, or a header
/// in one of the `include` folders.
fn find_file(name: &str, source: &Path, include: &[PathBuf]) -> Option<PathBuf> {
    if source.file_name().is_some_and(|n| n == name) {
        return Some(source.to_path_buf());
    }
    include
        .iter()
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// An assembler message, told in C terms when it is about a compiled file
/// (`c_files`: compiled file, C file): the C file and line instead of the
/// compiled ones (no line when the compiled line has no C line), C names
/// without their `_`.
pub(crate) fn c_diagnostic(
    mut d: Diagnostic,
    c_files: &[(PathBuf, PathBuf)],
    lines: &LineMaps,
) -> Diagnostic {
    if let Some(file) = &d.file
        && let Some((_, source)) = c_files.iter().find(|(compiled, _)| compiled == file)
    {
        let line = lines.c_line(file, d.line).map_or(0, |(_, n)| n);
        d.file = Some(source.clone());
        d.line = line;
        d.col = 0;
    }
    let mut message = d.message;
    for (compiled, source) in c_files {
        // "(compiled, line n)" → "(source, line m)", or "(source)"
        let start_text = format!("({}, line ", compiled.display());
        while let Some(start) = message.find(&start_text) {
            let end = start + message[start..].find(')').unwrap_or(0);
            let n = message[start + start_text.len()..end].parse().unwrap_or(0);
            let place = match lines.c_line(compiled, n) {
                Some((_, m)) => format!("({}, line {m})", source.display()),
                None => format!("({})", source.display()),
            };
            message.replace_range(start..=end, &place);
        }
    }
    // names are between quotes: every second part
    let mut out = String::new();
    for (i, part) in message.split('\'').enumerate() {
        if i > 0 {
            out.push('\'');
        }
        out += if i % 2 == 1 { c_name(part) } else { part };
    }
    d.message = out.replace("unknown label or variable", "not defined:");
    d
}

/// The C name of an assembly name (`_name`, `__S<unit>_name`), or the name.
pub(crate) fn c_name(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix("__S")
        && let Some(i) = rest.find('_')
        && i > 0
        && rest[..i].chars().all(|c| c.is_ascii_digit())
    {
        return &rest[i + 1..];
    }
    match name.strip_prefix('_') {
        Some(rest) if !rest.is_empty() && !rest.starts_with('_') => rest,
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(file: &str, line: u32, message: &str) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            file: Some(PathBuf::from(file)),
            line,
            col: if line > 0 { 1 } else { 0 },
            message: message.into(),
        }
    }

    #[test]
    fn names() {
        assert_eq!(c_name("_count"), "count");
        assert_eq!(c_name("__S12_count"), "count");
        assert_eq!(c_name("__mul"), "__mul");
        assert_eq!(c_name("__Sx_a"), "__Sx_a");
        assert_eq!(c_name("label"), "label");
    }

    #[test]
    fn assembler_messages() {
        let files = [(PathBuf::from("/t/a_1.has"), PathBuf::from("/s/a.c"))];
        let none = LineMaps::default();
        let d = c_diagnostic(
            error("/t/a_1.has", 12, "unknown label or variable '_foo'"),
            &files,
            &none,
        );
        assert_eq!(d.to_string(), "/s/a.c: error: not defined: 'foo'");
        let d = c_diagnostic(
            error(
                "/t/b_2.has",
                4,
                "'_g' is already defined (/t/a_1.has, line 7)",
            ),
            &files,
            &none,
        );
        assert_eq!(
            d.to_string(),
            "/t/b_2.has:4:1: error: 'g' is already defined (/s/a.c)"
        );

        // with the C lines of the compiled file
        let dir = std::env::temp_dir().join(format!("hbc2-cc-asm-messages-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let compiled = dir.join("a_1.has");
        std::fs::write(&compiled, "_g:\n;#line 4\n\tnop\n;#line 9\n\tcal _foo\n").unwrap();
        let source = dir.join("a.c");
        let files = [(compiled.clone(), source.clone())];
        let lines = LineMaps::read(&files);
        let message = format!("'_g' is already defined ({}, line 3)", compiled.display());
        let d = c_diagnostic(
            error(&compiled.display().to_string(), 5, &message),
            &files,
            &lines,
        );
        assert_eq!((d.file.as_deref(), d.line), (Some(&*source), 9));
        assert_eq!(
            d.message,
            format!("'g' is already defined ({}, line 4)", source.display())
        );
        let d = c_diagnostic(
            error(&compiled.display().to_string(), 1, "x"),
            &files,
            &lines,
        );
        assert_eq!(d.line, 0, "no C line before the first marker");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stage_messages() {
        let dir = std::env::temp_dir().join(format!("hbc2-cc-messages-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("inc")).unwrap();
        std::fs::write(dir.join("inc/defs.h"), "").unwrap();
        let source = dir.join("main.c");
        let include = [dir.clone(), dir.join("inc")];
        let text = "main.c:4 - missing semicolon\n\
                    defs.h:2 - expected \";\"\n\
                    other.h:9 - invalid value\n\
                    cc2: something odd\n";
        let d = tool_messages(text, Severity::Error, &source, &include);
        assert_eq!(d.len(), 4);
        assert_eq!((d[0].file.as_deref(), d[0].line), (Some(&*source), 4));
        assert_eq!(d[0].message, "missing semicolon");
        assert_eq!(
            (d[1].file.clone(), d[1].line),
            (Some(dir.join("inc/defs.h")), 2)
        );
        assert_eq!((d[2].file.as_deref(), d[2].line), (Some(&*source), 0));
        assert_eq!(d[2].message, "other.h, line 9: invalid value");
        assert_eq!(
            (d[3].line, d[3].message.as_str()),
            (0, "cc2: something odd")
        );

        let text = "'nothere.h' - C:\\p\\b.c:2:error: Cannot open include file\n\
                    C:\\p\\c.c:3:error: #error stop\n\
                    C:\\p\\c.c:5:warning: odd\n";
        let d = tool_messages(text, Severity::Error, &source, &include);
        assert_eq!(
            (d[0].file.clone(), d[0].line),
            (Some(PathBuf::from("C:\\p\\b.c")), 1)
        );
        assert_eq!(d[0].message, "Cannot open include file 'nothere.h'");
        assert_eq!((d[1].line, d[1].message.as_str()), (2, "#error stop"));
        assert_eq!(d[2].severity, Severity::Warning);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
