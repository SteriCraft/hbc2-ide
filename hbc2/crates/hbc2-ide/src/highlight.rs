//! Syntax highlighting of `.has` and C source code.
//!
//! These are lenient, line-based scanners meant for display only: they
//! never fail, and the assembler and the compiler stay the judges of what
//! is valid.

use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// Columns between tab stops (Settings > Editor): a tab goes to the next
/// multiple of it.
static TAB_WIDTH: AtomicUsize = AtomicUsize::new(4);

pub fn tab_width() -> usize {
    TAB_WIDTH.load(Ordering::Relaxed).max(1)
}

pub fn set_tab_width(width: usize) {
    TAB_WIDTH.store(width.max(1), Ordering::Relaxed);
}

/// Columns egui gives a tab, wherever it is.
const EGUI_TAB_COLUMNS: usize = 4;

/// The column (from 0) after `line_start`, the start of a line, with each
/// tab going to the next tab stop.
pub fn column(line_start: &str) -> usize {
    column_with(line_start, tab_width())
}

pub fn column_with(line_start: &str, tab_width: usize) -> usize {
    line_start.chars().fold(0, |column, c| match c {
        '\t' => column + tab_width - column % tab_width,
        _ => column + 1,
    })
}

/// How tabs are laid out: columns between tab stops, and the width of a
/// character of the (monospace) font.
#[derive(Clone, Copy, Debug)]
pub struct Tabs {
    pub width: usize,
    pub char_width: f32,
}

/// Appends `text`, which starts at `column` of its line, to the job and
/// moves `column` past it. egui draws every tab 4 characters wide: each tab
/// is made narrower (smaller font) or wider (space before it), so
/// that it ends on the next tab stop (a tab has nothing to show).
fn append_with_tabs(
    job: &mut LayoutJob,
    text: &str,
    format: &TextFormat,
    column: &mut usize,
    tabs: Tabs,
) {
    for (index, piece) in text.split('\t').enumerate() {
        if index > 0 {
            let width = tabs.width - *column % tabs.width;
            let mut tab = format.clone();
            let mut space_before = 0.0;
            if width <= EGUI_TAB_COLUMNS {
                tab.font_id.size *= width as f32 / EGUI_TAB_COLUMNS as f32;
            } else {
                space_before = (width - EGUI_TAB_COLUMNS) as f32 * tabs.char_width;
            }
            job.append("\t", space_before, tab);
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

/// Text color of a bracket that has no partner.
fn lonely_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0xF4, 0x47, 0x47)
    } else {
        Color32::from_rgb(0xCD, 0x31, 0x31)
    }
}

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
pub fn layout_job(text: &str, syntax: Syntax, font: FontId, dark: bool, tabs: Tabs) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut offset = 0;
    let mut in_comment = false;
    // Brackets without a partner stand out in red.
    let mut lonely = lonely_brackets(text, syntax).into_iter().peekable();
    let lonely_format = TextFormat {
        background: if dark {
            Color32::from_rgba_unmultiplied(255, 70, 70, 70)
        } else {
            Color32::from_rgba_unmultiplied(255, 60, 60, 50)
        },
        ..TextFormat::simple(font.clone(), lonely_color(dark))
    };
    let mut index = 0; // char index of the start of the current span
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let spans = match syntax {
            Syntax::Assembly => spans(content),
            Syntax::C => c_spans(content, &mut in_comment),
        };
        let mut column = 0;
        for (range, kind) in spans {
            let format = TextFormat::simple(font.clone(), color(kind, dark));
            let piece = &content[range];
            let mut from = 0;
            for (i, (byte, _)) in piece.char_indices().enumerate() {
                if lonely.next_if_eq(&(index + i)).is_some() {
                    append_with_tabs(&mut job, &piece[from..byte], &format, &mut column, tabs);
                    // Brackets are one byte long.
                    let bracket = &piece[byte..byte + 1];
                    append_with_tabs(&mut job, bracket, &lonely_format, &mut column, tabs);
                    from = byte + 1;
                }
            }
            append_with_tabs(&mut job, &piece[from..], &format, &mut column, tabs);
            index += piece.chars().count();
        }
        let end = &line[content.len()..];
        job.append(
            end,
            0.0,
            TextFormat::simple(font.clone(), color(Kind::Plain, dark)),
        );
        index += end.chars().count();
        offset += line.len();
    }
    debug_assert_eq!(offset, text.len());
    job
}

/// The brackets of the code (not in comments or strings), as
/// (char index, bracket).
fn brackets(text: &str, syntax: Syntax) -> Vec<(usize, char)> {
    let mut brackets = Vec::new();
    let mut index = 0;
    let mut in_comment = false;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let spans = match syntax {
            Syntax::Assembly => spans(content),
            Syntax::C => c_spans(content, &mut in_comment),
        };
        let mut chars = content.char_indices().peekable();
        for (range, kind) in spans {
            let code = !matches!(kind, Kind::Comment | Kind::Text);
            while let Some(&(at, c)) = chars.peek() {
                if at >= range.end {
                    break;
                }
                if code && "()[]{}".contains(c) {
                    brackets.push((index, c));
                }
                chars.next();
                index += 1;
            }
        }
        index += line[content.len()..].chars().count();
    }
    brackets
}

/// The brackets that have no partner: closing brackets that no opening
/// bracket matches, and opening brackets that are never closed. Sorted char
/// indices. Each kind of bracket is counted on its own, like
/// [`matching_bracket`] does.
pub fn lonely_brackets(text: &str, syntax: Syntax) -> Vec<usize> {
    if !text.contains(['(', ')', '[', ']', '{', '}']) {
        return Vec::new();
    }
    // Char indices of the opening brackets not closed yet, per kind.
    let mut open: [Vec<usize>; 3] = Default::default();
    let mut lonely = Vec::new();
    for (at, c) in brackets(text, syntax) {
        let kind = match c {
            '(' | ')' => 0,
            '[' | ']' => 1,
            _ => 2,
        };
        if "([{".contains(c) {
            open[kind].push(at);
        } else if open[kind].pop().is_none() {
            lonely.push(at);
        }
    }
    lonely.extend(open.into_iter().flatten());
    lonely.sort_unstable();
    lonely
}

/// The bracket next to the cursor (`cursor` is a char index) and the one
/// that matches it, as char indices. The bracket left of the cursor wins
/// over the one right of it. Brackets in comments and strings are ignored.
pub fn matching_bracket(text: &str, syntax: Syntax, cursor: usize) -> Option<(usize, usize)> {
    let is_bracket = |c: char| "()[]{}".contains(c);
    // Cheap check first: most of the time no bracket is next to the cursor.
    let mut around = text.chars().skip(cursor.saturating_sub(1));
    let left = if cursor > 0 { around.next() } else { None };
    let right = around.next();
    if !left.is_some_and(is_bracket) && !right.is_some_and(is_bracket) {
        return None;
    }

    let brackets = brackets(text, syntax);
    let find = |at: usize| brackets.iter().position(|&(i, _)| i == at);
    let start = cursor
        .checked_sub(1)
        .and_then(find)
        .or_else(|| find(cursor))?;
    let (at, bracket) = brackets[start];
    let (partner, forward) = match bracket {
        '(' => (')', true),
        '[' => (']', true),
        '{' => ('}', true),
        ')' => ('(', false),
        ']' => ('[', false),
        _ => ('{', false),
    };
    // Walking away from the bracket, its kind opens a level, its partner
    // closes one.
    let mut depth = 0;
    let mut visit = |&(i, c): &(usize, char)| {
        if c == bracket {
            depth += 1;
        } else if c == partner {
            depth -= 1;
        }
        (depth == 0).then_some(i)
    };
    let other = if forward {
        brackets[start..].iter().find_map(&mut visit)
    } else {
        brackets[..=start].iter().rev().find_map(&mut visit)
    }?;
    Some((at, other))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brackets() {
        let c = |text: &str, at| matching_bracket(text, Syntax::C, at);
        let text = "f(a[1], {b})";
        assert_eq!(c(text, 2), Some((1, 11))); // after `(`
        assert_eq!(c(text, 1), Some((1, 11))); // before `(`
        assert_eq!(c(text, 12), Some((11, 1))); // after `)`
        assert_eq!(c(text, 4), Some((3, 5))); // `[` left wins over `1`
        assert_eq!(c(text, 9), Some((8, 10))); // `{`
        assert_eq!(c(text, 0), None);
        assert_eq!(c("(()", 1), None); // unmatched
        // Comments and strings are skipped, also across lines.
        assert_eq!(c("(\")\" // )\n/* ) */ )", 1), Some((0, 18)));
        let asm = |text: &str, at| matching_bracket(text, Syntax::Assembly, at);
        assert_eq!(asm("mov a, [b] ; ]", 7), Some((7, 9)));
        assert_eq!(asm("x ')' (", 4), None);
    }

    #[test]
    fn lonely() {
        let c = |text: &str| lonely_brackets(text, Syntax::C);
        assert_eq!(c("f(a[1], {b})"), Vec::<usize>::new());
        assert_eq!(c("a) b] c}"), vec![1, 4, 7]);
        assert_eq!(c("(])"), vec![1]);
        assert_eq!(c("{\n}\n}"), vec![4]);
        assert_eq!(c("f(a[1, {b}"), vec![1, 3]);
        assert_eq!(c("{ ( } ) {"), vec![8]);
        assert_eq!(c("(("), vec![0, 1]);
        assert_eq!(c("x = ')'; // )\n/* ] */"), Vec::<usize>::new());
        assert_eq!(lonely_brackets("mov a, b] ; ]", Syntax::Assembly), vec![8]);
    }

    #[test]
    fn columns_with_tab_stops() {
        assert_eq!(column(""), 0);
        assert_eq!(column("ab"), 2);
        assert_eq!(column("\t"), 4);
        assert_eq!(column("ab\t"), 4);
        assert_eq!(column("abcd\t"), 8);
        assert_eq!(column("\tlod c, [sp + 2]\t"), 20);
        assert_eq!(column_with("ab\t", 8), 8);
        assert_eq!(column_with("ab\t", 2), 4);
    }

    /// Tabs end on the tab stops in the laid out text, whatever the tab
    /// width: each comment starts at the column its line puts it at.
    #[test]
    fn tabs_line_up() {
        let text = "\tmov a, 1\t\t; one\n\tlod c, [sp + 2]\t; two\n\tret\t\t\t\t; three\n";
        let font = FontId::monospace(14.0);
        let ctx = eframe::egui::Context::default();
        for width in [2, 4, 8] {
            let mut galley = None;
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let char_width = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
                let tabs = Tabs { width, char_width };
                let job = layout_job(text, Syntax::Assembly, font.clone(), true, tabs);
                galley = Some(ui.fonts_mut(|f| f.layout_job(job)));
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
            // Positions are rounded to pixels: the width of a character
            // measured over 13 of them.
            let space = (x_of("2]") - x_of("lod c")) / 13.0;
            for comment in ["; one", "; two", "; three"] {
                let line_start = text[..text.find(comment).unwrap()]
                    .rfind('\n')
                    .map_or(0, |n| n + 1);
                let before = &text[line_start..text.find(comment).unwrap()];
                let expected = column_with(before, width) as f32 * space;
                let x = x_of(comment);
                // Within a third of a character (the measure of `space` drifts).
                assert!(
                    (x - expected).abs() < space / 3.0,
                    "width {width}: {x} vs {expected}"
                );
            }
        }
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
