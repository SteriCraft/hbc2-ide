//! Syntax highlighting of `.has` source code.
//!
//! This is a lenient, line-based scanner meant for display only: it never
//! fails, and the assembler stays the judge of what is valid.

use std::ops::Range;

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, FontId};
use hbc2_core::{Opcode, Register};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Comment,
    Directive,
    Mnemonic,
    Register,
    Number,
    /// `$0x9000`
    Address,
    /// `"text"` or `'c'`
    Text,
    /// `name:`
    LabelDef,
    /// A name used as an operand (label, variable, define).
    Name,
}

/// Splits one line (without its newline) into highlighted spans, covering the whole line.
pub fn spans(line: &str) -> Vec<(Range<usize>, Kind)> {
    let bytes = line.as_bytes();
    let mut out: Vec<(Range<usize>, Kind)> = Vec::new();
    let mut push = |range: Range<usize>, kind: Kind| match out.last_mut() {
        Some((last, k)) if *k == kind && last.end == range.start => last.end = range.end,
        _ => out.push((range, kind)),
    };
    // Only the first word of a statement can be a mnemonic.
    let mut expect_mnemonic = true;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let start = i;
        match c {
            b';' => {
                push(i..bytes.len(), Kind::Comment);
                break;
            }
            b'"' | b'\'' => {
                i += 1;
                while i < bytes.len() && bytes[i] != c {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i = (i + 1).min(bytes.len());
                push(start..i, Kind::Text);
            }
            b'.' if is_name_start(bytes.get(i + 1)) && expect_mnemonic => {
                i = word_end(bytes, i + 1);
                push(start..i, Kind::Directive);
                expect_mnemonic = false;
            }
            b'$' => {
                i = word_end(bytes, i + 1);
                push(start..i, Kind::Address);
            }
            b'0'..=b'9' => {
                i = word_end(bytes, i);
                push(start..i, Kind::Number);
            }
            b'-' if bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                i = word_end(bytes, i + 1);
                push(start..i, Kind::Number);
            }
            _ if is_name_start(Some(&c)) => {
                i = word_end(bytes, i);
                let word = &line[start..i];
                if bytes.get(i) == Some(&b':') {
                    i += 1;
                    push(start..i, Kind::LabelDef);
                    continue; // a label can be followed by an instruction
                }
                let lower = word.to_ascii_lowercase();
                let kind = if expect_mnemonic && Opcode::from_mnemonic(&lower).is_some() {
                    Kind::Mnemonic
                } else if Register::ALL.iter().any(|r| r.name() == lower)
                    // the stack pointer in `[sp + n]`
                    || (lower == "sp" && line[..start].trim_end().ends_with('['))
                {
                    Kind::Register
                } else {
                    Kind::Name
                };
                push(start..i, kind);
                expect_mnemonic = false;
                // `name.msb` / `name.lsb`
                if bytes.get(i) == Some(&b'.') && is_name_start(bytes.get(i + 1)) {
                    let end = word_end(bytes, i + 1);
                    push(i..end, kind);
                    i = end;
                }
            }
            _ => {
                // Whitespace and punctuation; keep UTF-8 characters whole.
                i += line[i..].chars().next().map_or(1, char::len_utf8);
                push(start..i, Kind::Plain);
            }
        }
    }
    out
}

fn is_name_start(c: Option<&u8>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
}

fn word_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    i
}

pub fn color(kind: Kind, dark: bool) -> Color32 {
    let rgb = |d: u32, l: u32| {
        let v = if dark { d } else { l };
        Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    };
    match kind {
        Kind::Plain => rgb(0xD4D4D4, 0x1F1F1F),
        Kind::Comment => rgb(0x6A9955, 0x008000),
        Kind::Directive => rgb(0xC586C0, 0xAF00DB),
        Kind::Mnemonic => rgb(0x569CD6, 0x0000FF),
        Kind::Register => rgb(0x9CDCFE, 0x001080),
        Kind::Number => rgb(0xB5CEA8, 0x098658),
        Kind::Address => rgb(0xD7BA7D, 0x795E26),
        Kind::Text => rgb(0xCE9178, 0xA31515),
        Kind::LabelDef => rgb(0xDCDCAA, 0x795E26),
        Kind::Name => rgb(0x4EC9B0, 0x267F99),
    }
}

/// Lays out a whole file with highlighting.
pub fn layout_job(text: &str, font: FontId, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        for (range, kind) in spans(content) {
            job.append(
                &content[range],
                0.0,
                TextFormat::simple(font.clone(), color(kind, dark)),
            );
        }
        job.append(
            &line[content.len()..],
            0.0,
            TextFormat::simple(font.clone(), color(Kind::Plain, dark)),
        );
        offset += line.len();
    }
    debug_assert_eq!(offset, text.len());
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(&str, Kind)> {
        spans(line)
            .into_iter()
            .filter(|(_, k)| *k != Kind::Plain)
            .map(|(r, k)| (&line[r], k))
            .collect()
    }

    #[test]
    fn instruction_with_comment() {
        assert_eq!(
            kinds("    mov a, 0x2A ; set a"),
            [
                ("mov", Kind::Mnemonic),
                ("a", Kind::Register),
                ("0x2A", Kind::Number),
                ("; set a", Kind::Comment),
            ]
        );
    }

    #[test]
    fn label_then_instruction() {
        assert_eq!(
            kinds("loop: str $0x9000, b"),
            [
                ("loop:", Kind::LabelDef),
                ("str", Kind::Mnemonic),
                ("$0x9000", Kind::Address),
                ("b", Kind::Register),
            ]
        );
    }

    #[test]
    fn directives_and_names() {
        assert_eq!(
            kinds(".data vector $0x0100 handler.msb, 'a', \"x;y\""),
            [
                (".data", Kind::Directive),
                ("vector", Kind::Name),
                ("$0x0100", Kind::Address),
                ("handler.msb", Kind::Name),
                ("'a'", Kind::Text),
                ("\"x;y\"", Kind::Text),
            ]
        );
        // A mnemonic used as an operand is not highlighted as one.
        assert_eq!(kinds("jmp add")[1], ("add", Kind::Name));
    }

    #[test]
    fn stack_and_offsets() {
        assert_eq!(
            kinds("lod a, [sp + 2]"),
            [
                ("lod", Kind::Mnemonic),
                ("a", Kind::Register),
                ("sp", Kind::Register),
                ("2", Kind::Number),
            ]
        );
        assert_eq!(kinds("str [x:y + 4], b")[3], ("4", Kind::Number));
        assert_eq!(kinds("ssp 3")[0], ("ssp", Kind::Mnemonic));
        assert_eq!(
            kinds("jmp sp")[1],
            ("sp", Kind::Name),
            "only inside brackets"
        );
    }

    #[test]
    fn spans_cover_the_line() {
        let line = "  é ; ü";
        let total: usize = spans(line).iter().map(|(r, _)| r.len()).sum();
        assert_eq!(total, line.len());
    }
}
