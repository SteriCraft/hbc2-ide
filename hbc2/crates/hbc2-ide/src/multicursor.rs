//! Several cursors at once, each with its own selection: the same key
//! applied at each of them.
//!
//! Positions are character indices. An edit is applied from the last
//! cursor to the first, so that each one only moves the cursors after it.

use crate::editing;
use crate::highlight;

/// A cursor: the selection goes from `anchor` (where it started) to `head`
/// (the end that moves, where the cursor shows); equal when nothing is
/// selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cursor {
    pub anchor: usize,
    pub head: usize,
}

impl Cursor {
    pub fn at(position: usize) -> Cursor {
        Cursor {
            anchor: position,
            head: position,
        }
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    fn map(self, f: impl Fn(usize) -> usize) -> Cursor {
        Cursor {
            anchor: f(self.anchor),
            head: f(self.head),
        }
    }
}

/// Where a move goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    /// To the first character of the line that is not a space, or to the
    /// start of the line when already there.
    Home,
    End,
}

/// What a key does at every cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Text typed (it replaces the selection): brackets and `}` are helped
    /// as with one cursor, an opening bracket goes around a selection.
    Type(String),
    /// Text pasted: with as many lines as cursors, one line at each cursor
    /// (in order); otherwise the whole text at each.
    Paste(String),
    Backspace,
    Delete,
    /// The selections go (the cursors without one do nothing).
    Cut,
    /// A new line, with the indentation of the cursor's line.
    Enter,
    Tab,
    /// A move; `extend` (Shift) moves only the head, selecting.
    Move {
        motion: Motion,
        extend: bool,
    },
}

/// The text and cursors after an operation: the cursors in text order,
/// overlapping ones merged, and which one is the main cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub text: String,
    pub cursors: Vec<Cursor>,
    pub main: usize,
}

/// The cursors in text order, those that overlap (or are at the same
/// place) merged into one; and where `main` (an index into `cursors`)
/// ends up.
pub fn normalize(cursors: &[Cursor], main: usize, len: usize) -> (Vec<Cursor>, usize) {
    let mut order: Vec<usize> = (0..cursors.len()).collect();
    order.sort_by_key(|&i| (cursors[i].start(), cursors[i].end()));
    let mut out: Vec<Cursor> = Vec::new();
    let mut main_out = 0;
    for i in order {
        let c = cursors[i].map(|p| p.min(len));
        match out.last_mut() {
            Some(last)
                if c.start() < last.end()
                    || (c.start() == last.start() && c.end() == last.end()) =>
            {
                // Merged: the selection covers both, it keeps the direction of
                // the first one.
                let (start, end) = (last.start(), last.end().max(c.end()));
                *last = if last.anchor <= last.head {
                    Cursor {
                        anchor: start,
                        head: end,
                    }
                } else {
                    Cursor {
                        anchor: end,
                        head: start,
                    }
                };
            }
            _ => out.push(c),
        }
        if i == main {
            main_out = out.len() - 1;
        }
    }
    (out, main_out)
}

/// The selected texts, in order (for copy).
pub fn selected(text: &str, cursors: &[Cursor]) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    cursors
        .iter()
        .map(|c| {
            chars[c.start().min(chars.len())..c.end().min(chars.len())]
                .iter()
                .collect()
        })
        .collect()
}

/// One change of the text: `start..end` replaced by `text`, and the
/// cursor's new selection.
struct Replace {
    start: usize,
    end: usize,
    text: Vec<char>,
    cursor: Cursor,
}

/// Applies `op` at every cursor. `main` is the index of the cursor egui
/// shows: it is followed through the change.
pub fn apply(text: &str, cursors: &[Cursor], main: usize, op: &Op) -> Outcome {
    let mut chars: Vec<char> = text.chars().collect();
    let (mut list, main) = normalize(cursors, main, chars.len());

    if let Op::Move { motion, extend } = op {
        for c in &mut list {
            *c = move_cursor(&chars, *c, *motion, *extend);
        }
    } else {
        let pieces = pieces(op, list.len());
        for i in (0..list.len()).rev() {
            let Some(r) = replace(&chars, list[i], op, &pieces[i]) else {
                continue;
            };
            let added = r.text.len();
            chars.splice(r.start..r.end, r.text);
            // The cursors after this one, already moved by their own change.
            let shift = |p: usize| {
                if p >= r.end {
                    p - (r.end - r.start) + added
                } else if p > r.start {
                    r.start + added
                } else {
                    p
                }
            };
            for c in &mut list[i + 1..] {
                *c = c.map(shift);
            }
            list[i] = r.cursor;
        }
    }
    let (cursors, main) = normalize(&list, main, chars.len());
    Outcome {
        text: chars.into_iter().collect(),
        cursors,
        main,
    }
}

/// The text each cursor gets for a paste, in order (empty strings for the
/// other edits).
fn pieces(op: &Op, cursors: usize) -> Vec<String> {
    match op {
        Op::Paste(text) => {
            let text = text.replace("\r\n", "\n");
            let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
            if cursors > 1 && lines.len() == cursors {
                lines.into_iter().map(str::to_owned).collect()
            } else {
                vec![text.clone(); cursors]
            }
        }
        _ => vec![String::new(); cursors],
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
fn replace(chars: &[char], cursor: Cursor, op: &Op, piece: &str) -> Option<Replace> {
    let (start, end) = (cursor.start(), cursor.end());
    // The selection replaced by `text`, the cursor after it.
    let put = |text: &str| {
        let text: Vec<char> = text.chars().collect();
        Some(Replace {
            start,
            end,
            cursor: Cursor::at(start + text.len()),
            text,
        })
    };
    let c = cursor.head;
    match op {
        Op::Paste(_) => put(piece),
        Op::Tab => put("\t"),
        Op::Delete | Op::Cut if !cursor.is_empty() => put(""),
        Op::Cut => None,
        Op::Backspace if !cursor.is_empty() => put(""),
        Op::Type(typed) => {
            let mut typed_chars = typed.chars();
            let (Some(ch), None) = (typed_chars.next(), typed_chars.next()) else {
                return put(typed);
            };
            let text: String = chars.iter().collect();
            if let Some(close) = editing::closing(ch)
                && editing::open_bracket(&text, cursor.anchor, cursor.head, ch).is_some()
            {
                // Around the selection (which stays selected), or a pair.
                let mut new = vec![ch];
                new.extend(&chars[start..end]);
                new.push(close);
                return Some(Replace {
                    start,
                    end,
                    text: new,
                    cursor: cursor.map(|p| p + 1),
                });
            }
            if !cursor.is_empty() {
                return put(typed);
            }
            if editing::close_bracket(&text, c, c, ch).is_some() {
                return Some(Replace {
                    start: c,
                    end: c,
                    text: Vec::new(),
                    cursor: Cursor::at(c + 1),
                });
            }
            if let Some(edit) = editing::close_brace(&text, c, c, ch) {
                let line = line_start(chars, c);
                return Some(Replace {
                    start: line,
                    end: c,
                    text: edit
                        .text
                        .chars()
                        .skip(line)
                        .take(edit.cursor - line)
                        .collect(),
                    cursor: Cursor::at(edit.cursor),
                });
            }
            put(typed)
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
                cursor: Cursor::at(c - 1),
            })
        }
        Op::Delete => (c < chars.len()).then_some(Replace {
            start: c,
            end: c + 1,
            text: Vec::new(),
            cursor: Cursor::at(c),
        }),
        Op::Enter => {
            let text: String = chars.iter().collect();
            let edit = editing::newline(&text, cursor.anchor, cursor.head);
            let kept = chars.len() - (end - start);
            let added = edit.text.chars().count() - kept;
            Some(Replace {
                start,
                end,
                text: edit.text.chars().skip(start).take(added).collect(),
                cursor: Cursor::at(edit.cursor),
            })
        }
        Op::Move { .. } => None,
    }
}

/// Where a cursor goes for a move. Without Shift, Left and Right on a
/// selection go to its start and end.
fn move_cursor(chars: &[char], cursor: Cursor, motion: Motion, extend: bool) -> Cursor {
    if !extend && !cursor.is_empty() {
        match motion {
            Motion::Left => return Cursor::at(cursor.start()),
            Motion::Right => return Cursor::at(cursor.end()),
            _ => {}
        }
    }
    let c = cursor.head;
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
    let head = match motion {
        Motion::Left => c.saturating_sub(1),
        Motion::Right => (c + 1).min(chars.len()),
        Motion::Home => {
            let text = chars[start..line_end(chars, c)]
                .iter()
                .position(|c| *c != ' ' && *c != '\t')
                .map_or(start, |p| start + p);
            if c == text { start } else { text }
        }
        Motion::End => line_end(chars, c),
        Motion::Up if start > 0 => at_column(line_start(chars, start - 1), column()),
        Motion::Up => 0,
        Motion::Down => {
            let end = line_end(chars, c);
            if end == chars.len() {
                chars.len()
            } else {
                at_column(end + 1, column())
            }
        }
    };
    if extend {
        Cursor {
            anchor: cursor.anchor,
            head,
        }
    } else {
        Cursor::at(head)
    }
}

/// Ctrl+D: each line holding a cursor (every line a selection covers) is
/// copied below itself; the cursors go to the copy, at the same place.
/// Cursors on the same lines copy them once.
pub fn duplicate_lines(text: &str, cursors: &[Cursor], main: usize) -> Outcome {
    let chars: Vec<char> = text.chars().collect();
    let (list, main) = normalize(cursors, main, chars.len());
    // The lines of each cursor (start, end without the line break), merged
    // when they overlap. A selection ending at the start of a line does not
    // take that line.
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    for c in &list {
        let last = if !c.is_empty() && chars[c.end() - 1] == '\n' {
            c.end() - 1
        } else {
            c.end()
        };
        let block = (line_start(&chars, c.start()), line_end(&chars, last));
        match blocks.last_mut() {
            Some(previous) if block.0 <= previous.1 => previous.1 = previous.1.max(block.1),
            _ => blocks.push(block),
        }
    }
    let mut out = Vec::with_capacity(chars.len());
    let mut copied = 0;
    for &(start, end) in &blocks {
        out.extend(&chars[copied..end]);
        out.push('\n');
        out.extend(&chars[start..end]);
        copied = end;
    }
    out.extend(&chars[copied..]);
    // A position moves by the length of the copies made at or before it.
    let shift = |p: usize| {
        p + blocks
            .iter()
            .filter(|(start, _)| *start <= p)
            .map(|(start, end)| end - start + 1)
            .sum::<usize>()
    };
    let moved: Vec<Cursor> = list.iter().map(|c| c.map(shift)).collect();
    let (cursors, main) = normalize(&moved, main, out.len());
    Outcome {
        text: out.into_iter().collect(),
        cursors,
        main,
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The word (letters, digits, `_`) at `at` or just before it; an empty
/// range elsewhere.
pub fn word_at(text: &str, at: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let word = |i: usize| chars.get(i).is_some_and(|&c| is_word(c));
    let at = if word(at) || at == 0 || !word(at - 1) {
        at
    } else {
        at - 1
    };
    if !word(at) {
        return (at, at);
    }
    let start = (0..at).rev().take_while(|&i| word(i)).last().unwrap_or(at);
    let end = (at..chars.len()).find(|&i| !word(i)).unwrap_or(chars.len());
    (start, end)
}

/// Whether the selection is a whole word: only letters, digits and `_`,
/// with none just before or after it. Ctrl+L then only finds whole words.
pub fn is_whole_word(text: &str, cursor: Cursor) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = (cursor.start(), cursor.end().min(chars.len()));
    start < end
        && chars[start..end].iter().all(|&c| is_word(c))
        && (start == 0 || !is_word(chars[start - 1]))
        && chars.get(end).is_none_or(|&c| !is_word(c))
}

/// Where `needle` is in `chars`, from `from` on, going round to the
/// start; whole words only when `whole_word`.
fn find(chars: &[char], needle: &[char], from: usize, whole_word: bool) -> Vec<usize> {
    let mut found = Vec::new();
    if needle.is_empty() || needle.len() > chars.len() {
        return found;
    }
    let fits = |i: usize| {
        chars[i..i + needle.len()] == *needle
            && (!whole_word
                || ((i == 0 || !is_word(chars[i - 1]))
                    && chars.get(i + needle.len()).is_none_or(|&c| !is_word(c))))
    };
    let last = chars.len() - needle.len();
    let from = from.min(last + 1);
    found.extend((from..=last).filter(|&i| fits(i)));
    found.extend((0..from).filter(|&i| fits(i)));
    found
}

/// Ctrl+L: the main cursor without a selection selects its word; with one,
/// the next place with the same text (after it, then from the start) is
/// selected too and becomes the main cursor. `whole_word`: only whole
/// words match (when the first selection was a word taken by Ctrl+L).
/// `None` when there is nothing more to add.
pub fn select_next(
    text: &str,
    cursors: &[Cursor],
    main: usize,
    whole_word: bool,
) -> Option<(Vec<Cursor>, usize)> {
    let current = *cursors.get(main)?;
    let mut list = cursors.to_vec();
    if current.is_empty() {
        let (start, end) = word_at(text, current.head);
        if start == end {
            return None;
        }
        list[main] = Cursor {
            anchor: start,
            head: end,
        };
        return Some(normalize(&list, main, text.chars().count()));
    }
    let chars: Vec<char> = text.chars().collect();
    let needle = &chars[current.start()..current.end()];
    let taken = |i: usize| list.iter().any(|c| c.start() == i);
    let next = find(&chars, needle, current.end(), whole_word)
        .into_iter()
        .find(|&i| !taken(i))?;
    list.push(Cursor {
        anchor: next,
        head: next + needle.len(),
    });
    Some(normalize(&list, list.len() - 1, chars.len()))
}

/// Ctrl+Shift+L: every place with the text of the main selection selected;
/// without a selection, every place with the word at the cursor (whole
/// words).
pub fn select_all(
    text: &str,
    cursors: &[Cursor],
    main: usize,
    whole_word: bool,
) -> Option<(Vec<Cursor>, usize)> {
    let mut current = *cursors.get(main)?;
    let mut whole_word = whole_word;
    if current.is_empty() {
        let (start, end) = word_at(text, current.head);
        if start == end {
            return None;
        }
        current = Cursor {
            anchor: start,
            head: end,
        };
        whole_word = true;
    }
    let chars: Vec<char> = text.chars().collect();
    let needle = &chars[current.start()..current.end()];
    let mut list: Vec<Cursor> = find(&chars, needle, 0, whole_word)
        .into_iter()
        .map(|i| Cursor {
            anchor: i,
            head: i + needle.len(),
        })
        .collect();
    let main = list.iter().position(|c| c.start() == current.start())?;
    list.swap(0, main);
    Some(normalize(&list, 0, chars.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text with the cursors marked: `|` for a cursor, `[`...`]` for a
    /// selection; `*` and `{`...`}` for the main one.
    fn show(text: &str, cursors: &[Cursor], main: usize) -> String {
        let mut chars: Vec<char> = text.chars().collect();
        for (i, c) in cursors.iter().enumerate().rev() {
            let m = i == main;
            if c.is_empty() {
                chars.insert(c.head, if m { '*' } else { '|' });
            } else {
                chars.insert(c.end(), if m { '}' } else { ']' });
                chars.insert(c.start(), if m { '{' } else { '[' });
            }
        }
        chars.into_iter().collect()
    }

    /// Cursors marked with `|` and `[`...`]` (selections going forward);
    /// the first one is the main one.
    fn parse(marked: &str) -> (String, Vec<Cursor>) {
        let mut cursors = Vec::new();
        let mut text = String::new();
        let mut open = None;
        for c in marked.chars() {
            let at = text.chars().count();
            match c {
                '|' => cursors.push(Cursor::at(at)),
                '[' => open = Some(at),
                ']' => cursors.push(Cursor {
                    anchor: open.take().unwrap(),
                    head: at,
                }),
                _ => text.push(c),
            }
        }
        (text, cursors)
    }

    fn run(marked: &str, op: Op) -> String {
        let (text, cursors) = parse(marked);
        let o = apply(&text, &cursors, 0, &op);
        show(&o.text, &o.cursors, o.main)
    }

    fn mv(motion: Motion) -> Op {
        Op::Move {
            motion,
            extend: false,
        }
    }

    #[test]
    fn typing_everywhere() {
        assert_eq!(run("a|b\nc|d\n|", Op::Type("x".into())), "ax*b\ncx|d\nx|");
        assert_eq!(run("f|\ng|", Op::Type("(".into())), "f(*)\ng(|)");
        assert_eq!(run("f(|)\ng(|)", Op::Type(")".into())), "f()*\ng()|");
    }

    #[test]
    fn typing_over_selections() {
        // The rename case: each selected name replaced.
        assert_eq!(
            run("[foo](1);\nx = [foo];", Op::Type("b".into())),
            "b*(1);\nx = b|;"
        );
        // A bracket goes around each selection, which stays selected.
        assert_eq!(run("[a] + [b]", Op::Type("(".into())), "({a}) + ([b])");
        assert_eq!(run("[ab]c[de]", Op::Backspace), "*c|");
        assert_eq!(run("[ab]c[de]", Op::Delete), "*c|");
        assert_eq!(run("x[ab]\ny[cd]", Op::Enter), "x\n*\ny\n|");
    }

    #[test]
    fn deleting() {
        assert_eq!(run("ab|c\nde|f", Op::Backspace), "a*c\nd|f");
        // At the start of the text: nothing; at the start of a line: joins it.
        assert_eq!(run("|ab\n|cd", Op::Backspace), "*ab|cd");
        assert_eq!(run("a|b\nc|d", Op::Delete), "a*\nc|");
        // Two cursors side by side: both characters go, the cursors meet.
        assert_eq!(run("ab|c|d", Op::Backspace), "a*d");
        assert_eq!(run("f(|)\ng(|)", Op::Backspace), "f*\ng|");
    }

    #[test]
    fn new_lines_keep_the_indentation() {
        assert_eq!(run("\ta|\n  b|", Op::Enter), "\ta\n\t*\n  b\n  |");
        assert_eq!(run("{|}\n{|}", Op::Enter), "{\n\t*\n}\n{\n\t|\n}");
    }

    #[test]
    fn pasting() {
        assert_eq!(run("|\n|\n|", Op::Paste("a\nb\nc\n".into())), "a*\nb|\nc|");
        assert_eq!(run("|-|", Op::Paste("xy".into())), "xy*-xy|");
        assert_eq!(run("|-|", Op::Paste("1\n2\n3".into())), "1\n2\n3*-1\n2\n3|");
        // Over selections.
        assert_eq!(run("[a]-[b]", Op::Paste("x\ny".into())), "x*-y|");
    }

    #[test]
    fn moving() {
        assert_eq!(run("a|b|c", mv(Motion::Left)), "*a|bc");
        assert_eq!(run("|a|", mv(Motion::Left)), "*a");
        assert_eq!(run("a|b\nc|d", mv(Motion::Right)), "ab*\ncd|");
        assert_eq!(run("ab|c\nd\nefg|", mv(Motion::Up)), "*abc\nd|\nefg");
        assert_eq!(run("a|bc\ndef", mv(Motion::Down)), "abc\nd*ef");
        assert_eq!(run("\tx|\nabcdefgh", mv(Motion::Down)), "\tx\nabcde*fgh");
        assert_eq!(run("  ab|\n|cd", mv(Motion::Home)), "  *ab\n|cd");
        assert_eq!(run("  |ab", mv(Motion::Home)), "*  ab");
        assert_eq!(run("a|b\n|c", mv(Motion::End)), "ab*\nc|");
        // A selection: Left / Right go to its ends.
        assert_eq!(run("a[bc]d e[f]", mv(Motion::Left)), "a*bcd e|f");
        assert_eq!(run("a[bc]d e[f]", mv(Motion::Right)), "abc*d ef|");
    }

    #[test]
    fn shift_selects() {
        let extend = |motion| Op::Move {
            motion,
            extend: true,
        };
        assert_eq!(run("a|bc d|ef", extend(Motion::Right)), "a{b}c d[e]f");
        assert_eq!(run("ab|c\nde|f", extend(Motion::End)), "ab{c}\nde[f]");
        // Selections that touch stay apart; one that grows into another
        // becomes one with it.
        assert_eq!(run("a|b|c", extend(Motion::Right)), "a{b}[c]");
        assert_eq!(run("[a|b]c", extend(Motion::Right)), "{abc}");
    }

    #[test]
    fn ctrl_l_selects_the_next_occurrences() {
        let text = "foo(1); foobar; foo; xfoo";
        let mut cursors = vec![Cursor::at(1)];
        let mut main = 0;
        let step = |cursors: &mut Vec<Cursor>, main: &mut usize| {
            let (c, m) = select_next(text, cursors, *main, true).unwrap();
            *cursors = c;
            *main = m;
            show(text, cursors, *main)
        };
        // First the word at the cursor, then the next whole words.
        assert_eq!(step(&mut cursors, &mut main), "{foo}(1); foobar; foo; xfoo");
        assert_eq!(
            step(&mut cursors, &mut main),
            "[foo](1); foobar; {foo}; xfoo"
        );
        // None left.
        assert_eq!(select_next(text, &cursors, main, true), None);
        // Not whole words (a selection made with the mouse): every match.
        let (c, m) = select_next(text, &cursors, main, false).unwrap();
        assert_eq!(show(text, &c, m), "[foo](1); foobar; [foo]; x{foo}");
    }

    #[test]
    fn ctrl_l_goes_round() {
        let text = "ab ab ab";
        let cursors = vec![Cursor { anchor: 3, head: 5 }];
        let (c, m) = select_next(text, &cursors, 0, true).unwrap();
        assert_eq!(show(text, &c, m), "ab [ab] {ab}");
        let (c, m) = select_next(text, &c, m, true).unwrap();
        assert_eq!(show(text, &c, m), "{ab} [ab] [ab]");
    }

    #[test]
    fn ctrl_shift_l_selects_them_all() {
        let text = "foo(1); foobar; foo";
        let (c, m) = select_all(text, &[Cursor::at(17)], 0, false).unwrap();
        assert_eq!(show(text, &c, m), "[foo](1); foobar; {foo}");
        // A selection (not whole words): every match.
        let selection = [Cursor { anchor: 0, head: 3 }];
        let (c, m) = select_all(text, &selection, 0, false).unwrap();
        assert_eq!(show(text, &c, m), "{foo}(1); [foo]bar; [foo]");
    }

    #[test]
    fn cut_and_whole_words() {
        assert_eq!(run("[ab]c|d[e]", Op::Cut), "*c|d|");
        let word = |a, h| is_whole_word("foo foobar", Cursor { anchor: a, head: h });
        assert!(word(0, 3));
        assert!(word(3, 0));
        assert!(!word(4, 7)); // "foo" in "foobar"
        assert!(!word(2, 5)); // "o f"
        assert!(!word(1, 1));
    }

    #[test]
    fn ctrl_d_duplicates_lines() {
        let dup = |marked: &str| {
            let (text, cursors) = parse(marked);
            let o = duplicate_lines(&text, &cursors, 0);
            show(&o.text, &o.cursors, o.main)
        };
        assert_eq!(
            dup("ab|c
d"),
            "abc
ab*c
d"
        );
        // The last line, without a line break after it.
        assert_eq!(
            dup("a
b|c"),
            "a
bc
b*c"
        );
        // Each line with a cursor; two cursors on a line copy it once.
        assert_eq!(
            dup("|a
b
|c|d"),
            "a
*a
b
cd
|c|d"
        );
        // A selection copies its lines, and stays selected in the copy.
        assert_eq!(
            dup("x[a
b]c
y"),
            "xa
bc
x{a
b}c
y"
        );
        // Ending at the start of a line: that line is not copied.
        assert_eq!(
            dup("[ab
]cd"),
            "ab
{ab
}cd"
        );
    }

    #[test]
    fn merging() {
        let cursors = [
            Cursor { anchor: 5, head: 8 },
            Cursor::at(2),
            Cursor {
                anchor: 6,
                head: 10,
            },
            Cursor::at(2),
        ];
        let (list, main) = normalize(&cursors, 2, 20);
        assert_eq!(
            list,
            [
                Cursor::at(2),
                Cursor {
                    anchor: 5,
                    head: 10
                }
            ]
        );
        assert_eq!(main, 1);
    }
}
