//! Syntax highlighting of `.has` and C source code.
//!
//! These are lenient, line-based scanners meant for display only: they
//! never fail, and the assembler and the compiler stay the judges of what
//! is valid.

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
    /// C: `if`, `return`, …
    Keyword,
    /// C: `int`, `char`, `struct`, …
    Type,
    /// C: a `#` line.
    Preprocessor,
    /// C: a name followed by `(`.
    Function,
}

/// Columns between tab stops: a tab goes to the next multiple of 4, as in
/// most editors.
pub const TAB_WIDTH: usize = 4;

/// The column (from 0) after `line_start`, the start of a line, with each
/// tab going to the next tab stop.
pub fn column(line_start: &str) -> usize {
    line_start.chars().fold(0, |column, c| match c {
        '\t' => column + TAB_WIDTH - column % TAB_WIDTH,
        _ => column + 1,
    })
}

/// Appends `text`, which starts at `column` of its line, to the job and
/// moves `column` past it. egui draws every tab 4 spaces wide wherever it
/// is: each tab gets a smaller font size, so that it ends on the next tab
/// stop (a tab has nothing to show, only its width changes).
fn append_with_tabs(job: &mut LayoutJob, text: &str, format: &TextFormat, column: &mut usize) {
    for (index, piece) in text.split('\t').enumerate() {
        if index > 0 {
            let width = TAB_WIDTH - *column % TAB_WIDTH;
            let mut tab = format.clone();
            tab.font_id.size *= width as f32 / TAB_WIDTH as f32;
            job.append("\t", 0.0, tab);
            *column += width;
        }
        if !piece.is_empty() {
            job.append(piece, 0.0, format.clone());
            *column += piece.chars().count();
        }
    }
}

/// Which highlighter a file gets, by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Syntax {
    Assembly,
    C,
}

impl Syntax {
    pub fn of(path: &std::path::Path) -> Syntax {
        if crate::project::has_extension(path, &["c", "h"]) {
            Syntax::C
        } else {
            Syntax::Assembly
        }
    }
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
        Kind::Keyword | Kind::Preprocessor => color(Kind::Directive, dark),
        Kind::Type => color(Kind::Mnemonic, dark),
        Kind::Function => color(Kind::LabelDef, dark),
    }
}

const C_KEYWORDS: &[&str] = &[
    "break", "case", "continue", "default", "do", "else", "for", "goto", "if", "return", "sizeof",
    "switch", "while",
];

const C_TYPES: &[&str] = &[
    "auto", "char", "const", "double", "enum", "extern", "float", "int", "long", "register",
    "short", "signed", "static", "struct", "typedef", "union", "unsigned", "void", "volatile",
];

/// Splits one line of C into highlighted spans, covering the whole line.
/// `in_comment` says whether the line starts inside a `/* */` comment, and
/// is updated for the next line.
pub fn c_spans(line: &str, in_comment: &mut bool) -> Vec<(Range<usize>, Kind)> {
    let bytes = line.as_bytes();
    let mut out: Vec<(Range<usize>, Kind)> = Vec::new();
    let mut push = |range: Range<usize>, kind: Kind| match out.last_mut() {
        Some((last, k)) if *k == kind && last.end == range.start => last.end = range.end,
        _ => out.push((range, kind)),
    };
    let mut i = 0;
    // A `#` line is one span, but its comments and strings keep their colors.
    let preprocessor = line.trim_start().starts_with('#') && !*in_comment;
    let other = if preprocessor {
        Kind::Preprocessor
    } else {
        Kind::Plain
    };
    while i < bytes.len() {
        let start = i;
        if *in_comment {
            match line[i..].find("*/") {
                Some(end) => {
                    i += end + 2;
                    *in_comment = false;
                }
                None => i = bytes.len(),
            }
            push(start..i, Kind::Comment);
            continue;
        }
        let c = bytes[i];
        match c {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                push(i..bytes.len(), Kind::Comment);
                break;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                *in_comment = true;
                push(start..i, Kind::Comment);
            }
            b'"' | b'\'' => {
                i += 1;
                while i < bytes.len() && bytes[i] != c {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i = (i + 1).min(bytes.len());
                push(start..i, Kind::Text);
            }
            b'<' if preprocessor && line[..i].contains("include") => {
                i = line[i..].find('>').map_or(bytes.len(), |n| i + n + 1);
                push(start..i, Kind::Text);
            }
            b'0'..=b'9' if !preprocessor => {
                i = word_end(bytes, i);
                push(start..i, Kind::Number);
            }
            _ if is_name_start(Some(&c)) && !preprocessor => {
                i = word_end(bytes, i);
                let word = &line[start..i];
                let kind = if C_KEYWORDS.contains(&word) {
                    Kind::Keyword
                } else if C_TYPES.contains(&word) {
                    Kind::Type
                } else if line[i..].trim_start().starts_with('(') {
                    Kind::Function
                } else {
                    Kind::Plain
                };
                push(start..i, kind);
            }
            _ => {
                i += line[i..].chars().next().map_or(1, char::len_utf8);
                push(start..i, other);
            }
        }
    }
    out
}

/// Lays out a whole file with highlighting.
pub fn layout_job(text: &str, syntax: Syntax, font: FontId, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut offset = 0;
    let mut in_comment = false;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let spans = match syntax {
            Syntax::Assembly => spans(content),
            Syntax::C => c_spans(content, &mut in_comment),
        };
        let mut column = 0;
        for (range, kind) in spans {
            let format = TextFormat::simple(font.clone(), color(kind, dark));
            append_with_tabs(&mut job, &content[range], &format, &mut column);
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

    #[test]
    fn columns_with_tab_stops() {
        assert_eq!(column(""), 0);
        assert_eq!(column("ab"), 2);
        assert_eq!(column("\t"), 4);
        assert_eq!(column("ab\t"), 4);
        assert_eq!(column("abcd\t"), 8);
        assert_eq!(column("\tlod c, [sp + 2]\t"), 20);
    }

    /// Tabs end on the tab stops in the laid out text: the comments of
    /// these lines start at the same x.
    #[test]
    fn tabs_line_up() {
        // Each comment starts at column 20.
        let text = "\tmov a, 1\t\t; one\n\tlod c, [sp + 2]\t; two\n\tret\t\t\t\t; three\n";
        let font = FontId::monospace(14.0);
        let job = layout_job(text, Syntax::Assembly, font.clone(), true);
        let ctx = eframe::egui::Context::default();
        let mut galley = None;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            galley = Some(ui.fonts_mut(|f| f.layout_job(job.clone())));
        });
        output.textures_delta.clear();
        let galley = galley.unwrap();
        let x_of = |s: &str| {
            let index = text[..text.find(s).unwrap()].chars().count();
            galley
                .pos_from_cursor(eframe::egui::text::CCursor::new(index))
                .min
                .x
        };
        // Positions are rounded to pixels: the width of a character measured
        // over 13 of them.
        let space = (x_of("2]") - x_of("lod c")) / 13.0;
        let (one, two, three) = (x_of("; one"), x_of("; two"), x_of("; three"));
        for x in [one, two, three] {
            assert!((x - 20.0 * space).abs() < 1.5, "{x} vs {}", 20.0 * space);
        }
        assert_eq!((one, one), (two, three));
    }

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

    fn c_kinds<'a>(line: &'a str, in_comment: &mut bool) -> Vec<(&'a str, Kind)> {
        c_spans(line, in_comment)
            .into_iter()
            .filter(|(_, k)| *k != Kind::Plain)
            .map(|(r, k)| (&line[r], k))
            .collect()
    }

    #[test]
    fn c_lines() {
        let mut comment = false;
        assert_eq!(
            c_kinds(
                "static int f(char c) { return 'x' + 12; } // end",
                &mut comment
            ),
            [
                ("static", Kind::Type),
                ("int", Kind::Type),
                ("f", Kind::Function),
                ("char", Kind::Type),
                ("return", Kind::Keyword),
                ("'x'", Kind::Text),
                ("12", Kind::Number),
                ("// end", Kind::Comment),
            ]
        );
        assert_eq!(
            c_kinds("#include <stdio.h> /* a", &mut comment),
            [
                ("#include ", Kind::Preprocessor),
                ("<stdio.h>", Kind::Text),
                (" ", Kind::Preprocessor),
                ("/* a", Kind::Comment),
            ]
        );
        assert!(comment, "the comment goes on");
        assert_eq!(
            c_kinds("b */ x = \"/*\";", &mut comment),
            [("b */", Kind::Comment), ("\"/*\"", Kind::Text)]
        );
        assert!(!comment);
    }

    #[test]
    fn spans_cover_the_line() {
        let line = "  é ; ü";
        let total: usize = spans(line).iter().map(|(r, _)| r.len()).sum();
        assert_eq!(total, line.len());
    }
}
