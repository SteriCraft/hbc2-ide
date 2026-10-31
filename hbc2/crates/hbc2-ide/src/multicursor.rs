//! Several cursors at once: the same key applied at each of them.
//!
//! Cursors here are positions without a selection, as character indices.
//! An edit is applied from the last cursor to the first, so that each one
//! only moves the cursors after it.

use crate::editing;
use crate::highlight;

/// What a key does at every cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Text typed: brackets and `}` are helped as with one cursor.
    Type(String),
    /// Text pasted: with as many lines as cursors, one line at each cursor
    /// (in order); otherwise the whole text at each.
    Paste(String),
    Backspace,
    Delete,
    /// A new line, with the indentation of the cursor's line.
    Enter,
    Tab,
    Left,
    Right,
    Up,
    Down,
    /// To the first character of the line that is not a space, or to the
    /// start of the line when already there.
    Home,
    End,
}

/// The text and cursors after an operation: the cursors sorted, without
/// duplicates (cursors that meet become one), and the main cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub text: String,
    pub cursors: Vec<usize>,
    pub main: usize,
}

/// One change of the text: `start..end` replaced by `text`, and where the
/// cursor goes.
struct Replace {
    start: usize,
    end: usize,
    text: Vec<char>,
    cursor: usize,
}

/// Applies `op` at every cursor. `main` is the cursor egui shows (one of
/// `cursors`): it is followed through the change.
pub fn apply(text: &str, cursors: &[usize], main: usize, op: &Op) -> Outcome {
    let mut chars: Vec<char> = text.chars().collect();
    let mut list: Vec<usize> = cursors.iter().map(|&c| c.min(chars.len())).collect();
    list.sort_unstable();
    list.dedup();
    let main_index = list
        .iter()
        .position(|&c| c == main.min(chars.len()))
        .unwrap_or(0);

    if let Some(pieces) = pieces(op, list.len()) {
        for i in (0..list.len()).rev() {
            let Some(r) = replace(&chars, list[i], op, pieces.get(i).map(String::as_str)) else {
                continue;
            };
            let added = r.text.len();
            chars.splice(r.start..r.end, r.text);
            // The cursors after this one, already moved by their own change.
            for p in &mut list[i + 1..] {
                if *p >= r.end {
                    *p = *p - (r.end - r.start) + added;
                } else if *p > r.start {
                    *p = r.start + added;
                }
            }
            list[i] = r.cursor;
        }
    } else {
        for c in &mut list {
            *c = move_cursor(&chars, *c, op);
        }
    }
    let main = list[main_index];
    let len = chars.len();
    for c in &mut list {
        *c = (*c).min(len);
    }
    list.sort_unstable();
    list.dedup();
    Outcome {
        text: chars.into_iter().collect(),
        cursors: list,
        main: main.min(len),
    }
}

/// For an edit (not a move): the text each cursor gets for a paste, in
/// order (empty for the other edits). `None` for a move.
fn pieces(op: &Op, cursors: usize) -> Option<Vec<String>> {
    match op {
        Op::Paste(text) => {
            let text = text.replace("\r\n", "\n");
            let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
            Some(if cursors > 1 && lines.len() == cursors {
                lines.into_iter().map(str::to_owned).collect()
            } else {
                vec![text.clone(); cursors]
            })
        }
        Op::Type(_) | Op::Backspace | Op::Delete | Op::Enter | Op::Tab => Some(Vec::new()),
        _ => None,
    }
}

fn line_start(chars: &[char], at: usize) -> usize {
    chars[..at]
        .iter()
        .rposition(|&c| c == '\n')
        .map_or(0, |p| p + 1)
}

fn line_end(chars: &[char], at: usize) -> usize {
    chars[at..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(chars.len(), |p| at + p)
}

/// The change one cursor makes.
fn replace(chars: &[char], c: usize, op: &Op, piece: Option<&str>) -> Option<Replace> {
    let insert = |text: &str| {
        let text: Vec<char> = text.chars().collect();
        Some(Replace {
            start: c,
            end: c,
            cursor: c + text.len(),
            text,
        })
    };
    match op {
        Op::Paste(_) => insert(piece.unwrap_or_default()),
        Op::Tab => insert("\t"),
        Op::Type(typed) => {
            let mut typed_chars = typed.chars();
            let (Some(ch), None) = (typed_chars.next(), typed_chars.next()) else {
                return insert(typed);
            };
            let text: String = chars.iter().collect();
            if let Some(close) = editing::closing(ch)
                && editing::open_bracket(&text, c, c, ch).is_some()
            {
                return Some(Replace {
                    start: c,
                    end: c,
                    text: vec![ch, close],
                    cursor: c + 1,
                });
            }
            if editing::close_bracket(&text, c, c, ch).is_some() {
                return Some(Replace {
                    start: c,
                    end: c,
                    text: Vec::new(),
                    cursor: c + 1,
                });
            }
            if let Some(edit) = editing::close_brace(&text, c, c, ch) {
                let start = line_start(chars, c);
                return Some(Replace {
                    start,
                    end: c,
                    text: edit
                        .text
                        .chars()
                        .skip(start)
                        .take(edit.cursor - start)
                        .collect(),
                    cursor: edit.cursor,
                });
            }
            insert(typed)
        }
        Op::Backspace => {
            if c == 0 {
                return None;
            }
            let text: String = chars.iter().collect();
            let end = if editing::backspace(&text, c, c).is_some() {
                c + 1 // between two brackets: both go
            } else {
                c
            };
            Some(Replace {
                start: c - 1,
                end,
                text: Vec::new(),
                cursor: c - 1,
            })
        }
        Op::Delete => (c < chars.len()).then_some(Replace {
            start: c,
            end: c + 1,
            text: Vec::new(),
            cursor: c,
        }),
        Op::Enter => {
            let text: String = chars.iter().collect();
            let edit = editing::newline(&text, c, c);
            let added = edit.text.chars().count() - chars.len();
            Some(Replace {
                start: c,
                end: c,
                text: edit.text.chars().skip(c).take(added).collect(),
                cursor: edit.cursor,
            })
        }
        _ => None,
    }
}

/// Where a cursor goes for a move.
fn move_cursor(chars: &[char], c: usize, op: &Op) -> usize {
    let start = line_start(chars, c);
    // The same column (tabs counted to their tab stop) on another line.
    let at_column = |line: usize, column: usize| {
        let end = line_end(chars, line);
        let mut position = line;
        while position < end {
            let next: String = chars[line..=position].iter().collect();
            if highlight::column(&next) > column {
                break;
            }
            position += 1;
        }
        position
    };
    let column = || highlight::column(&chars[start..c].iter().collect::<String>());
    match op {
        Op::Left => c.saturating_sub(1),
        Op::Right => (c + 1).min(chars.len()),
        Op::Home => {
            let text = chars[start..line_end(chars, c)]
                .iter()
                .position(|c| *c != ' ' && *c != '\t')
                .map_or(start, |p| start + p);
            if c == text { start } else { text }
        }
        Op::End => line_end(chars, c),
        Op::Up if start > 0 => at_column(line_start(chars, start - 1), column()),
        Op::Up => 0,
        Op::Down => {
            let end = line_end(chars, c);
            if end == chars.len() {
                chars.len()
            } else {
                at_column(end + 1, column())
            }
        }
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text with `|` at each cursor and `*` at the main one.
    fn show(o: &Outcome) -> String {
        let mut chars: Vec<char> = o.text.chars().collect();
        for &c in o.cursors.iter().rev() {
            chars.insert(c, if c == o.main { '*' } else { '|' });
        }
        chars.into_iter().collect()
    }

    /// Cursors given as `|` in the text (the first one is the main one).
    fn run(marked: &str, op: Op) -> String {
        let mut cursors = Vec::new();
        let mut text = String::new();
        for c in marked.chars() {
            if c == '|' {
                cursors.push(text.chars().count());
            } else {
                text.push(c);
            }
        }
        show(&apply(&text, &cursors, cursors[0], &op))
    }

    #[test]
    fn typing_everywhere() {
        assert_eq!(run("a|b\nc|d\n|", Op::Type("x".into())), "ax*b\ncx|d\nx|");
        // Brackets are closed, at each cursor.
        assert_eq!(run("f|\ng|", Op::Type("(".into())), "f(*)\ng(|)");
        assert_eq!(run("f(|)\ng(|)", Op::Type(")".into())), "f()*\ng()|");
    }

    #[test]
    fn deleting() {
        assert_eq!(run("ab|c\nde|f", Op::Backspace), "a*c\nd|f");
        // At the start of the text: nothing; at the start of a line: joins it.
        assert_eq!(run("|ab\n|cd", Op::Backspace), "*ab|cd");
        assert_eq!(run("a|b\nc|d", Op::Delete), "a*\nc|");
        // Two cursors side by side: both characters go, the cursors meet.
        assert_eq!(run("ab|c|d", Op::Backspace), "a*d");
        // Between brackets: both.
        assert_eq!(run("f(|)\ng(|)", Op::Backspace), "f*\ng|");
    }

    #[test]
    fn new_lines_keep_the_indentation() {
        assert_eq!(run("\ta|\n  b|", Op::Enter), "\ta\n\t*\n  b\n  |");
        assert_eq!(run("{|}\n{|}", Op::Enter), "{\n\t*\n}\n{\n\t|\n}");
    }

    #[test]
    fn pasting() {
        // One line per cursor.
        assert_eq!(run("|\n|\n|", Op::Paste("a\nb\nc\n".into())), "a*\nb|\nc|");
        // Otherwise the whole text at each.
        assert_eq!(run("|-|", Op::Paste("xy".into())), "xy*-xy|");
        assert_eq!(run("|-|", Op::Paste("1\n2\n3".into())), "1\n2\n3*-1\n2\n3|");
    }

    #[test]
    fn moving() {
        assert_eq!(run("a|b|c", Op::Left), "*a|bc");
        assert_eq!(run("|a|", Op::Left), "*a"); // they meet at the start
        assert_eq!(run("a|b\nc|d", Op::Right), "ab*\ncd|");
        // Up from the first line: to the start; from a longer line: to
        // the end of the shorter one.
        assert_eq!(run("ab|c\nd\nefg|", Op::Up), "*abc\nd|\nefg");
        assert_eq!(run("a|bc\ndef", Op::Down), "abc\nd*ef");
        // Columns count tabs to their tab stop.
        assert_eq!(run("\tx|\nabcdefgh", Op::Down), "\tx\nabcde*fgh");
        assert_eq!(run("  ab|\n|cd", Op::Home), "  *ab\n|cd");
        assert_eq!(run("  |ab", Op::Home), "*  ab");
        assert_eq!(run("a|b\n|c", Op::End), "ab*\nc|");
    }
}
