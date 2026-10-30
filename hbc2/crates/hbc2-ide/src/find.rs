//! Find in the current file and in the project's files.

use std::ops::Range;
use std::path::{Path, PathBuf};

/// Which search the find bar does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindMode {
    /// In the file of the editor tab (Ctrl+F).
    File,
    /// In every source file of the project (Ctrl+Shift+F).
    Files,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub case_sensitive: bool,
    pub whole_word: bool,
}

/// One match of a "find in files" search.
#[derive(Clone, Debug)]
pub struct Hit {
    pub path: PathBuf,
    /// 1-based.
    pub line: u32,
    /// Characters of the match in its line.
    pub columns: Range<usize>,
    /// The line, for display.
    pub text: String,
}

/// Result of a "find in files" search.
#[derive(Default)]
pub struct FilesResult {
    pub query: String,
    pub hits: Vec<Hit>,
    pub files: usize,
}

/// The find bar at the bottom of the editor.
#[derive(Default)]
pub struct Find {
    /// `None`: the bar is closed.
    pub mode: Option<FindMode>,
    pub query: String,
    pub options: Options,
    /// Gives the query field the keyboard focus (and selects its text) on
    /// the next frame.
    pub focus_query: bool,
    /// The match last gone to, in this file.
    pub current: Option<(PathBuf, Range<usize>)>,
    pub results: Option<FilesResult>,
}

/// Characters that make up a word, for "whole word" searches.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every match of `query` in `text`, as character ranges, without overlaps.
/// Case-insensitive searches fold ASCII letters only.
pub fn find_all(text: &str, query: &str, options: Options) -> Vec<Range<usize>> {
    let (hay, needle) = (text.as_bytes(), query.as_bytes());
    let mut found = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return found;
    }
    let same = |a: &u8, b: &u8| {
        if options.case_sensitive {
            a == b
        } else {
            a.eq_ignore_ascii_case(b)
        }
    };
    let query_chars = query.chars().count();
    // Character index of byte `counted`, to turn byte offsets into characters.
    let (mut counted, mut chars) = (0, 0);
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        let end = i + needle.len();
        // A match starts and ends on character boundaries: it starts with the
        // query's first byte, which is never inside a character, and folding
        // only changes ASCII bytes.
        let found_here = hay[i..end].iter().zip(needle).all(|(a, b)| same(a, b))
            && (!options.whole_word
                || (!text[..i].chars().next_back().is_some_and(is_word)
                    && !text[end..].chars().next().is_some_and(is_word)));
        if found_here {
            chars += text[counted..i].chars().count();
            counted = i;
            found.push(chars..chars + query_chars);
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

/// Index of the first match starting at or after `from`, wrapping around.
pub fn next_match(matches: &[Range<usize>], from: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().position(|m| m.start >= from).unwrap_or(0))
}

/// Index of the last match starting before `before`, wrapping around.
pub fn previous_match(matches: &[Range<usize>], before: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(
        matches
            .iter()
            .rposition(|m| m.start < before)
            .unwrap_or(matches.len() - 1),
    )
}

/// Matches in one file, line by line.
pub fn find_in_text(path: &Path, text: &str, query: &str, options: Options) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        for columns in find_all(line, query, options) {
            hits.push(Hit {
                path: path.to_path_buf(),
                line: index as u32 + 1,
                columns,
                text: line.to_string(),
            });
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANY: Options = Options {
        case_sensitive: false,
        whole_word: false,
    };

    #[test]
    fn finds_every_match_as_characters() {
        assert_eq!(find_all("mov r1, R1", "r1", ANY), [4..6, 8..10]);
        let case = Options {
            case_sensitive: true,
            ..ANY
        };
        assert_eq!(find_all("mov r1, R1", "r1", case), vec![4..6]);
        // Character indices, not bytes.
        assert_eq!(find_all("é r1", "r1", ANY), vec![2..4]);
        assert_eq!(find_all("aaaa", "aa", ANY), [0..2, 2..4]);
        assert!(find_all("abc", "", ANY).is_empty());
    }

    #[test]
    fn whole_words() {
        let word = Options {
            whole_word: true,
            ..ANY
        };
        assert_eq!(
            find_all("loop loop_2 loop2 (loop)", "loop", word),
            [0..4, 19..23]
        );
    }

    #[test]
    fn next_and_previous_wrap_around() {
        let matches = [2..4, 10..12];
        assert_eq!(next_match(&matches, 0), Some(0));
        assert_eq!(next_match(&matches, 3), Some(1));
        assert_eq!(next_match(&matches, 11), Some(0));
        assert_eq!(previous_match(&matches, 10), Some(0));
        assert_eq!(previous_match(&matches, 2), Some(1));
        assert_eq!(next_match(&[], 0), None);
    }

    #[test]
    fn lines_of_a_file() {
        let path = Path::new("a.has");
        let hits = find_in_text(path, "nop\r\n  call f\r\ncall g", "call", ANY);
        let places: Vec<(u32, Range<usize>)> =
            hits.iter().map(|h| (h.line, h.columns.clone())).collect();
        assert_eq!(places, [(2, 2..6), (3, 0..4)]);
        assert_eq!(hits[0].text, "  call f");
    }
}
