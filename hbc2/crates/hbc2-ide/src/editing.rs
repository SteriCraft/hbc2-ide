//! Editing helpers of the code editor: indent or unindent the selected
//! lines (Tab, Shift+Tab), closing brackets and quotes added after an
//! opening one, and a new line that keeps the indentation of the line above
//! (Enter).
//!
//! Positions are character indices (not bytes), like the editor's cursor.
//! The selection goes from `anchor` (where it started) to `cursor` (the end
//! that moves); they are equal when nothing is selected.

use std::sync::atomic::{AtomicBool, Ordering};

/// The text and selection after an edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub text: String,
    pub anchor: usize,
    pub cursor: usize,
}

/// Tab puts spaces, not tab characters (Settings > Editor).
static INDENT_WITH_SPACES: AtomicBool = AtomicBool::new(false);

pub fn set_indent_with_spaces(spaces: bool) {
    INDENT_WITH_SPACES.store(spaces, Ordering::Relaxed);
}

/// How the editor indents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indentation {
    /// Columns of one level: the tab width.
    pub width: usize,
    /// Levels are made of spaces, not tabs.
    pub spaces: bool,
}

impl Indentation {
    /// As the settings say.
    pub fn current() -> Indentation {
        Indentation {
            width: crate::highlight::tab_width(),
            spaces: INDENT_WITH_SPACES.load(Ordering::Relaxed),
        }
    }

    /// What Tab types at `column`: a tab, or spaces up to the next tab stop.
    pub fn tab_at(self, column: usize) -> String {
        if self.spaces {
            " ".repeat(self.width - column % self.width)
        } else {
            "\t".into()
        }
    }
}

const PAIRS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];

/// Quotes: the same character opens and closes.
const QUOTES: [char; 2] = ['"', '\''];

/// The closing bracket of `open`.
fn closing_bracket(open: char) -> Option<char> {
    PAIRS.iter().find(|p| p.0 == open).map(|p| p.1)
}

/// The closing bracket or quote of `open`.
pub fn closing(open: char) -> Option<char> {
    closing_bracket(open).or(QUOTES.contains(&open).then_some(open))
}

fn is_closing(c: char) -> bool {
    PAIRS.iter().any(|p| p.1 == c)
}

/// Whether a selection covers more than one line: Tab then indents the
/// lines instead of replacing the text.
pub fn spans_lines(text: &str, anchor: usize, cursor: usize) -> bool {
    let (start, end) = (anchor.min(cursor), anchor.max(cursor));
    text.chars()
        .skip(start)
        .take(end - start)
        .any(|c| c == '\n')
}

/// Start of each line touched by the selection. A selection that ends at
/// the very start of a line does not take that line.
fn line_starts(chars: &[char], start: usize, end: usize) -> Vec<usize> {
    let line_start = |i: usize| {
        chars[..i]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |p| p + 1)
    };
    let last = if end > start && chars[end - 1] == '\n' {
        end - 1
    } else {
        end
    };
    let mut starts = vec![line_start(start)];
    let newlines = (start..last).filter(|&i| chars[i] == '\n');
    starts.extend(newlines.map(|i| i + 1));
    starts
}

/// Tab (`outdent` false) or Shift+Tab (true) on the selected lines, or on
/// the cursor's line. Tab adds a level (a tab, or spaces) at the start of
/// each line that is not empty; Shift+Tab takes away a tab or up to a
/// level of spaces.
pub fn indent(text: &str, anchor: usize, cursor: usize, outdent: bool) -> Edit {
    indent_with(text, anchor, cursor, outdent, Indentation::current())
}

pub fn indent_with(
    text: &str,
    anchor: usize,
    cursor: usize,
    outdent: bool,
    indentation: Indentation,
) -> Edit {
    let mut chars: Vec<char> = text.chars().collect();
    let (mut anchor, mut cursor) = (anchor.min(chars.len()), cursor.min(chars.len()));
    let starts = line_starts(&chars, anchor.min(cursor), anchor.max(cursor));
    // From the last line up: a change does not move the lines above it.
    for &p in starts.iter().rev() {
        if outdent {
            let n = if chars.get(p) == Some(&'\t') {
                1
            } else {
                chars[p..]
                    .iter()
                    .take(indentation.width)
                    .take_while(|&&c| c == ' ')
                    .count()
            };
            chars.drain(p..p + n);
            // A position in the removed indentation goes to the start of the line.
            let shift = |pos: usize| {
                if pos >= p + n { pos - n } else { pos.min(p) }
            };
            anchor = shift(anchor);
            cursor = shift(cursor);
        } else if chars.get(p).is_some_and(|&c| c != '\n') {
            let level: Vec<char> = indentation.tab_at(0).chars().collect();
            let n = level.len();
            chars.splice(p..p, level);
            // A selection starting at the start of the line keeps the new level.
            let shift = |pos: usize| if pos > p { pos + n } else { pos };
            anchor = shift(anchor);
            cursor = shift(cursor);
        }
    }
    Edit {
        text: chars.into_iter().collect(),
        anchor,
        cursor,
    }
}

/// An opening bracket or a quote typed. With a selection, the pair goes
/// around it. Otherwise both are written, with the cursor between them, if
/// the next character is a space, a closing bracket, `;`, `,` or the end of
/// the text (`None`: the character is typed as usual). A quote is not
/// paired before the same quote (it closes the string) or after a letter,
/// a digit, `_` or `\` (`don't`, `'\''`).
pub fn open_bracket(text: &str, anchor: usize, cursor: usize, open: char) -> Option<Edit> {
    let close = closing(open)?;
    let mut chars: Vec<char> = text.chars().collect();
    let (start, end) = (anchor.min(cursor), anchor.max(cursor));
    if start != end {
        chars.insert(end, close);
        chars.insert(start, open);
        return Some(Edit {
            text: chars.into_iter().collect(),
            anchor: anchor + 1,
            cursor: cursor + 1,
        });
    }
    let next = chars.get(start).copied();
    let prev = start.checked_sub(1).and_then(|i| chars.get(i)).copied();
    if open == close
        && (next == Some(close)
            || prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '\\'))
    {
        return None;
    }
    if !next.is_none_or(|c| c.is_whitespace() || is_closing(c) || c == ';' || c == ',') {
        return None;
    }
    chars.splice(start..start, [open, close]);
    Some(Edit {
        text: chars.into_iter().collect(),
        anchor: start + 1,
        cursor: start + 1,
    })
}

/// A closing bracket or quote typed just before the same one: the cursor
/// goes past it instead of adding a second one.
pub fn close_bracket(text: &str, anchor: usize, cursor: usize, close: char) -> Option<Edit> {
    if anchor != cursor
        || !(is_closing(close) || QUOTES.contains(&close))
        || text.chars().nth(cursor) != Some(close)
    {
        return None;
    }
    Some(Edit {
        text: text.to_owned(),
        anchor: cursor + 1,
        cursor: cursor + 1,
    })
}

/// `}` typed with only spaces and tabs before it on the line: the line
/// takes the indentation of the line of the matching `{` (one level less
/// when there is none), so the block closes in line with its start.
pub fn close_brace(text: &str, anchor: usize, cursor: usize, typed: char) -> Option<Edit> {
    if typed != '}' || anchor != cursor {
        return None;
    }
    let mut chars: Vec<char> = text.chars().collect();
    let line_start = |i: usize| {
        chars[..i]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |p| p + 1)
    };
    let start = line_start(cursor);
    if chars[start..cursor].iter().any(|&c| c != ' ' && c != '\t') {
        return None;
    }
    // The matching '{': the first one before, not closed by a '}' after it.
    let mut depth = 0;
    let open = chars[..start].iter().rposition(|&c| match c {
        '}' => {
            depth += 1;
            false
        }
        '{' if depth == 0 => true,
        '{' => {
            depth -= 1;
            false
        }
        _ => false,
    });
    let indent: Vec<char> = match open {
        Some(open) => chars[line_start(open)..]
            .iter()
            .take_while(|&&c| c == ' ' || c == '\t')
            .copied()
            .collect(),
        None => {
            let mut current = chars[start..cursor].to_vec();
            if current.last() == Some(&'\t') {
                current.pop();
            } else {
                let spaces = current
                    .iter()
                    .rev()
                    .take(Indentation::current().width)
                    .take_while(|&&c| c == ' ');
                current.truncate(current.len() - spaces.count());
            }
            current
        }
    };
    let mut insert = indent;
    insert.push('}');
    let new_cursor = start + insert.len();
    chars.splice(start..cursor, insert);
    Some(Edit {
        text: chars.into_iter().collect(),
        anchor: new_cursor,
        cursor: new_cursor,
    })
}

/// Backspace between an opening bracket (or quote) and its closing one, as
/// just typed: both go.
pub fn backspace(text: &str, anchor: usize, cursor: usize) -> Option<Edit> {
    if anchor != cursor || cursor == 0 {
        return None;
    }
    let mut chars: Vec<char> = text.chars().collect();
    let close = closing(*chars.get(cursor - 1)?)?;
    if chars.get(cursor) != Some(&close) {
        return None;
    }
    chars.drain(cursor - 1..=cursor);
    Some(Edit {
        text: chars.into_iter().collect(),
        anchor: cursor - 1,
        cursor: cursor - 1,
    })
}

/// Enter: the new line starts with the spaces and tabs that start the
/// current line, plus a tab after an opening bracket. Between two brackets
/// (`{|}`), the closing one goes on a line of its own below.
pub fn newline(text: &str, anchor: usize, cursor: usize) -> Edit {
    let mut chars: Vec<char> = text.chars().collect();
    let (start, end) = (
        anchor.min(cursor).min(chars.len()),
        anchor.max(cursor).min(chars.len()),
    );
    chars.drain(start..end);
    let line_start = chars[..start]
        .iter()
        .rposition(|&c| c == '\n')
        .map_or(0, |p| p + 1);
    let indent: Vec<char> = chars[line_start..start]
        .iter()
        .take_while(|&&c| c == ' ' || c == '\t')
        .copied()
        .collect();
    let before = chars[line_start..start]
        .iter()
        .rev()
        .find(|c| !c.is_whitespace())
        .copied();
    let close = before.and_then(closing_bracket);

    let mut insert = vec!['\n'];
    insert.extend(&indent);
    if close.is_some() {
        insert.push('\t');
    }
    let new_cursor = start + insert.len();
    if close.is_some() && chars.get(start).copied() == close {
        insert.push('\n');
        insert.extend(&indent);
    }
    chars.splice(start..start, insert);
    Edit {
        text: chars.into_iter().collect(),
        anchor: new_cursor,
        cursor: new_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text with `|` at the cursor, or `[` ... `]` around a selection
    /// whose cursor is at the `]` (`]` ... `[` when the cursor is first).
    fn show(e: &Edit) -> String {
        let mut chars: Vec<char> = e.text.chars().collect();
        if e.anchor == e.cursor {
            chars.insert(e.cursor, '|');
        } else if e.anchor < e.cursor {
            chars.insert(e.cursor, '>');
            chars.insert(e.anchor, '<');
        } else {
            chars.insert(e.anchor, '<');
            chars.insert(e.cursor, '>');
        }
        chars.into_iter().collect()
    }

    #[test]
    fn tab_indents_selected_lines() {
        let text = "a\nb\n\nc\nd";
        // From the start of "a" to after "c": a, b, c (not the empty line).
        assert_eq!(show(&indent(text, 0, 6, false)), "<\ta\n\tb\n\n\tc>\nd");
        // Ending at the start of a line: that line is left alone.
        assert_eq!(show(&indent(text, 2, 7, false)), "a\n<\tb\n\n\tc\n>d");
        // Selected upward: the cursor stays first.
        assert_eq!(show(&indent(text, 4, 1, false)), "\ta>\n\tb\n<\nc\nd");
    }

    #[test]
    fn shift_tab_outdents() {
        let text = "\tx\n      y\n z\nw";
        assert_eq!(show(&indent(text, 1, 13, true)), "<x\n  y\nz>\nw");
        // No selection: the cursor's line, the cursor moves back with its text.
        assert_eq!(show(&indent(text, 9, 9, true)), "\tx\n  |y\n z\nw");
        // Cursor inside the spaces taken away: to the start of the line.
        assert_eq!(show(&indent("    a", 2, 2, true)), "|a");
        // Nothing to take away: unchanged.
        assert_eq!(indent("a\nb", 0, 3, true).text, "a\nb");
        // With spaces, two wide.
        let two = Indentation {
            width: 2,
            spaces: true,
        };
        assert_eq!(show(&indent_with("a\nb", 0, 3, false, two)), "<  a\n  b>");
        assert_eq!(show(&indent_with("   a", 3, 3, true, two)), " |a");
        assert_eq!(two.tab_at(3), " ");
        assert_eq!(two.tab_at(4), "  ");
    }

    #[test]
    fn spans() {
        assert!(spans_lines("a\nb", 0, 3));
        assert!(spans_lines("a\nb", 3, 0));
        assert!(!spans_lines("ab\nc", 0, 2));
    }

    #[test]
    fn brackets() {
        assert_eq!(show(&open_bracket("f", 1, 1, '(').unwrap()), "f(|)");
        assert_eq!(show(&open_bracket("x ;", 1, 1, '[').unwrap()), "x[|] ;");
        assert_eq!(show(&open_bracket("a)", 1, 1, '{').unwrap()), "a{|})");
        // Before a word: just the bracket.
        assert_eq!(open_bracket("ab", 1, 1, '('), None);
        // Around a selection.
        assert_eq!(show(&open_bracket("a+b", 0, 3, '(').unwrap()), "(<a+b>)");
        assert_eq!(open_bracket("a", 0, 0, 'x'), None);

        assert_eq!(show(&close_bracket("f()", 2, 2, ')').unwrap()), "f()|");
        assert_eq!(close_bracket("f(]", 2, 2, ')'), None);
        assert_eq!(close_bracket("f()", 1, 2, ')'), None);

        assert_eq!(show(&backspace("f()", 2, 2).unwrap()), "f|");
        assert_eq!(backspace("f(x)", 2, 2), None);
        assert_eq!(backspace("()", 0, 0), None);
    }

    #[test]
    fn quotes() {
        assert_eq!(show(&open_bracket("f(", 2, 2, '"').unwrap()), "f(\"|\"");
        assert_eq!(
            show(&open_bracket("x = ;", 4, 4, '\'').unwrap()),
            "x = '|';"
        );
        assert_eq!(show(&open_bracket("f()", 2, 2, '"').unwrap()), "f(\"|\")");
        assert_eq!(show(&open_bracket("ab", 0, 2, '"').unwrap()), "\"<ab>\"");
        // Before the same quote: it closes the string, the cursor steps over it.
        assert_eq!(open_bracket("\"ab\"", 3, 3, '"'), None);
        assert_eq!(
            show(&close_bracket("\"ab\"", 3, 3, '"').unwrap()),
            "\"ab\"|"
        );
        // After a word or a backslash, or before a word: just the quote.
        assert_eq!(open_bracket("don", 3, 3, '\''), None);
        assert_eq!(open_bracket("'\\", 2, 2, '\''), None);
        assert_eq!(open_bracket("ab", 0, 0, '"'), None);
        // Backspace between two quotes: both go.
        assert_eq!(show(&backspace("x\"\"", 2, 2).unwrap()), "x|");
        // Enter after a quote: no extra indentation.
        assert_eq!(show(&newline("\"", 1, 1)), "\"\n|");
    }

    #[test]
    fn closing_brace_lines_up_with_its_block() {
        // The case reported: '{' on its own line, '}' typed one tab too far.
        let text = "while (1)\n{\n\t\n\t";
        assert_eq!(
            show(&close_brace(text, 15, 15, '}').unwrap()),
            "while (1)\n{\n\t\n}|"
        );
        // Nested blocks: the '{' that is still open.
        let text = "\tif (a) {\n\t\tif (b) {\n\t\t}\n\t\t";
        let edit = close_brace(text, 27, 27, '}').unwrap();
        assert_eq!(show(&edit), "\tif (a) {\n\t\tif (b) {\n\t\t}\n\t}|");
        // No '{' before: one level less.
        assert_eq!(show(&close_brace("\t\t", 2, 2, '}').unwrap()), "\t}|");
        assert_eq!(show(&close_brace("      ", 6, 6, '}').unwrap()), "  }|");
        // Text before it on the line, or another character: typed as usual.
        assert_eq!(close_brace("{\n\tx", 4, 4, '}'), None);
        assert_eq!(close_brace("{\n\t", 3, 3, ')'), None);
    }

    #[test]
    fn enter_keeps_the_indentation() {
        assert_eq!(show(&newline("\t  a", 4, 4)), "\t  a\n\t  |");
        assert_eq!(show(&newline("x", 1, 1)), "x\n|");
        // Cursor inside the indentation: only what is before it.
        assert_eq!(show(&newline("\t\ta", 1, 1)), "\t\n\t|\ta");
        // After an opening bracket: one more tab; between brackets, the
        // closing one goes below.
        assert_eq!(show(&newline("\tif (a) {", 9, 9)), "\tif (a) {\n\t\t|");
        assert_eq!(show(&newline("\tf() {}", 6, 6)), "\tf() {\n\t\t|\n\t}");
        assert_eq!(show(&newline("{  ", 3, 3)), "{  \n\t|");
        // A selection is replaced.
        assert_eq!(show(&newline("  abc", 3, 5)), "  a\n  |");
    }
}
