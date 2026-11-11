//! UTF-8 sources, Windows-1252 HBC-2: the characters of the string and
//! character literals are turned into their Windows-1252 codes
//! (`hbc2_core::charset`), between cpp and cc0. Each one becomes a 3-digit
//! octal escape (`"é"` → `"\351"`), so cc0 only ever sees ASCII.

use std::path::PathBuf;

use hbc2_asm::{Diagnostic, Severity};

/// The preprocessed text with its literals in Windows-1252, or the first
/// character that has no code (with its place in the source, from cpp's
/// `# line "file"` markers).
pub(crate) fn literals_to_cp1252(text: &str) -> Result<String, Diagnostic> {
    let mut out = String::with_capacity(text.len());
    let mut file: Option<PathBuf> = None;
    let mut line: u32 = 1;
    for raw in text.split_inclusive('\n') {
        // A line marker: `# 12 "file"` (the next line is line 12 of file).
        if let Some(rest) = raw.trim_start().strip_prefix('#') {
            let mut parts = rest.trim().splitn(2, ' ');
            if let Some(n) = parts.next().and_then(|n| n.parse::<u32>().ok()) {
                if let Some(name) = parts.next() {
                    let name = name.trim().trim_matches('"');
                    file = Some(PathBuf::from(name));
                }
                line = n;
                out.push_str(raw);
                continue;
            }
        }
        convert_line(raw, &mut out).map_err(|(col, c)| Diagnostic {
            severity: Severity::Error,
            file: file.clone(),
            line,
            col,
            message: format!(
                "character '{c}' (U+{:04X}) has no Windows-1252 code",
                c as u32
            ),
        })?;
        line += 1;
    }
    Ok(out)
}

/// One line: the literals' characters converted, the rest copied.
fn convert_line(line: &str, out: &mut String) -> Result<(), (u32, char)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (col, c) in line.chars().enumerate() {
        match quote {
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                }
                out.push(c);
            }
            Some(q) => {
                if escaped {
                    escaped = false;
                    out.push(c);
                } else if c == '\\' {
                    escaped = true;
                    out.push(c);
                } else if c == q {
                    quote = None;
                    out.push(c);
                } else if c.is_ascii() {
                    out.push(c);
                } else {
                    let byte = hbc2_core::charset::from_char(c).ok_or((col as u32 + 1, c))?;
                    out.push_str(&format!("\\{byte:03o}"));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_become_octal_escapes() {
        let text = "# 1 \"t.c\"\nchar *s = \"Noël œ\";\nchar c = 'é', d = '\\'';\n";
        assert_eq!(
            literals_to_cp1252(text).unwrap(),
            "# 1 \"t.c\"\nchar *s = \"No\\353l \\234\";\nchar c = '\\351', d = '\\'';\n"
        );
        // An escaped quote doesn't end the string.
        assert_eq!(
            literals_to_cp1252("\"a\\\"é\"\n").unwrap(),
            "\"a\\\"\\351\"\n"
        );
    }

    #[test]
    fn a_character_without_a_code_is_an_error_at_its_place() {
        let text = "# 1 \"main.c\"\nint a;\n# 7 \"lib.h\"\n  puts(\"→\");\n";
        let e = literals_to_cp1252(text).unwrap_err();
        assert_eq!(e.file, Some(PathBuf::from("lib.h")));
        assert_eq!((e.line, e.col), (7, 9));
        assert!(e.message.contains("'→' (U+2192)"), "{}", e.message);
        // Outside literals nothing is checked (cc0 reports it).
        assert!(literals_to_cp1252("int → ;\n").is_ok());
    }
}
