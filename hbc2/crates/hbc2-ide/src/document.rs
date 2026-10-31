//! A source file open in an editor tab.

use std::collections::BTreeSet;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use eframe::egui::text::LayoutJob;

pub struct Document {
    pub path: PathBuf,
    pub text: String,
    /// Text as last read from or written to disk.
    saved: String,
    /// Modification time of the file when last read or written.
    disk_time: Option<SystemTime>,
    /// Set when the file changed on disk while it had unsaved changes.
    pub conflict: bool,
    /// Line (1-based) to bring into view on the next frame.
    pub scroll_to: Option<u32>,
    /// Bring `scroll_to` into view without centering it (following a
    /// running program: the view moves only when the line leaves it).
    pub scroll_gently: bool,
    /// Characters to select and bring into view on the next frame.
    pub select: Option<Range<usize>>,
    /// Gives the editor the keyboard when it is next drawn. (Focusing a
    /// widget that is not drawn in the frame stops the program when
    /// Windows' accessibility is active.)
    pub focus: bool,
    /// Highlighted layout of `text`, keyed by a hash of the text and the style.
    pub layout_cache: Option<(u64, LayoutJob)>,
    /// Characters taken out by the last save (blank lines emptied), as
    /// (position, count) in the text before: the editor moves its cursor
    /// accordingly when it is next drawn.
    pub emptied: Option<Vec<(usize, usize)>>,
}

impl Document {
    pub fn open(path: &Path) -> io::Result<Document> {
        let text = std::fs::read_to_string(path)?;
        Ok(Document {
            path: path.to_path_buf(),
            saved: text.clone(),
            text,
            disk_time: modified(path),
            conflict: false,
            scroll_to: None,
            scroll_gently: false,
            select: None,
            focus: false,
            layout_cache: None,
            emptied: None,
        })
    }

    pub fn name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into(),
        )
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved
    }

    /// Writes the file, after emptying its lines that hold only spaces and
    /// tabs (the lines stay).
    pub fn save(&mut self) -> io::Result<()> {
        let (text, removed) = empty_blank_lines(&self.text);
        if !removed.is_empty() {
            self.text = text;
            self.emptied = Some(removed);
        }
        std::fs::write(&self.path, &self.text)?;
        self.saved = self.text.clone();
        self.disk_time = modified(&self.path);
        self.conflict = false;
        Ok(())
    }

    /// Drops unsaved changes.
    pub fn revert(&mut self) {
        self.text = self.saved.clone();
        self.conflict = false;
    }

    /// Checks the file on disk. Reloads it if it changed and there are no
    /// unsaved changes; flags a conflict otherwise. Returns true if reloaded.
    pub fn check_disk(&mut self) -> bool {
        let time = modified(&self.path);
        if time.is_none() || time == self.disk_time {
            return false;
        }
        self.disk_time = time;
        if self.is_dirty() {
            self.conflict = true;
            return false;
        }
        match std::fs::read_to_string(&self.path) {
            Ok(text) if text != self.text => {
                self.saved = text.clone();
                self.text = text;
                true
            }
            _ => false,
        }
    }

    /// The document was renamed on disk.
    pub fn set_path(&mut self, path: PathBuf) {
        self.disk_time = modified(&path);
        self.path = path;
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Number of lines, as an editor shows them.
pub fn line_count(text: &str) -> u32 {
    text.bytes().filter(|&b| b == b'\n').count() as u32 + 1
}

/// After an edit from `old` to `new`, moves breakpoints (1-based lines) so
/// they stay on the same code: lines below the edit shift by the number of
/// lines added or removed. Breakpoints on removed lines go away.
pub fn shift_breakpoints(breakpoints: &mut BTreeSet<u32>, old: &str, new: &str) {
    let delta = line_count(new) as i64 - line_count(old) as i64;
    if delta == 0 || breakpoints.is_empty() {
        return;
    }
    let common = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    // Line where the edit starts. A breakpoint on it stays put, unless the
    // edit starts at the very beginning of that line (a line inserted above it).
    let edit_line = line_count(&old[..common]);
    let at_line_start = common == 0 || old.as_bytes()[common - 1] == b'\n';
    let first_moved = if at_line_start {
        edit_line
    } else {
        edit_line + 1
    };
    let removed_end = first_moved as i64 - delta; // exclusive, when delta < 0
    *breakpoints = breakpoints
        .iter()
        .filter_map(|&line| {
            if line < first_moved {
                Some(line)
            } else if delta < 0 && (line as i64) < removed_end {
                None
            } else {
                Some((line as i64 + delta) as u32)
            }
        })
        .collect();
}

/// The text with its lines that hold only spaces and tabs emptied (a `\r`
/// before the line break stays), and what was taken out: (position,
/// count) in characters, in order.
pub fn empty_blank_lines(text: &str) -> (String, Vec<(usize, usize)>) {
    let mut out = String::with_capacity(text.len());
    let mut removed = Vec::new();
    let mut position = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let count = content.chars().count();
        if count > 0 && content.chars().all(|c| c == ' ' || c == '\t') {
            removed.push((position, count));
            out.push_str(&line[content.len()..]);
        } else {
            out.push_str(line);
        }
        position += line.chars().count();
    }
    (out, removed)
}

/// Where a position goes once `removed` (from [`empty_blank_lines`]) is
/// taken out: one inside a removed part goes to its start.
pub fn position_after(position: usize, removed: &[(usize, usize)]) -> usize {
    let mut taken = 0;
    for &(start, count) in removed {
        if position >= start + count {
            taken += count;
        } else if position > start {
            taken += position - start;
        }
    }
    position - taken
}

#[cfg(test)]
mod tests {
    #[test]
    fn blank_lines_are_emptied() {
        use super::{empty_blank_lines, position_after};
        let text = "a\n  \t\nb \n\t\r\n\n  ";
        let (out, removed) = empty_blank_lines(text);
        // Lines with text keep their spaces; "\r" and empty lines stay.
        assert_eq!(out, "a\n\nb \n\r\n\n");
        assert_eq!(removed, [(2, 3), (9, 1), (13, 2)]);
        assert_eq!(position_after(1, &removed), 1); // before
        assert_eq!(position_after(4, &removed), 2); // inside: line start
        assert_eq!(position_after(6, &removed), 3); // after: 3 fewer
        assert_eq!(position_after(15, &removed), 9); // the end
        assert_eq!(empty_blank_lines("x\ny").1, []);
    }

    use super::*;

    fn shifted(lines: &[u32], old: &str, new: &str) -> Vec<u32> {
        let mut set: BTreeSet<u32> = lines.iter().copied().collect();
        shift_breakpoints(&mut set, old, new);
        set.into_iter().collect()
    }

    #[test]
    fn inserting_lines_moves_breakpoints_below() {
        let old = "a\nb\nc\n";
        // Newline typed at the end of line 1: line 2 moves to 3.
        assert_eq!(shifted(&[1, 2, 3], old, "a\n\nb\nc\n"), [1, 3, 4]);
        // Line inserted at the very top: everything moves.
        assert_eq!(shifted(&[1, 3], old, "x\na\nb\nc\n"), [2, 4]);
    }

    #[test]
    fn removing_lines_drops_their_breakpoints() {
        let old = "a\nb\nc\nd\n";
        assert_eq!(shifted(&[1, 2, 3, 4], old, "a\nd\n"), [1, 2]);
        // Joining line 2 into line 1 (backspace at the start of line 2).
        assert_eq!(shifted(&[1, 3], "a\nb\nc", "ab\nc"), [1, 2]);
    }

    #[test]
    fn same_line_count_changes_nothing() {
        assert_eq!(shifted(&[2], "a\nb", "a\nbb"), [2]);
    }
}
