//! Folding: hiding the body of functions and blocks in the editor.
//!
//! A region is a header line, which stays shown, and the lines after it
//! that a fold hides:
//! - C: a `{ … }` block over several lines. The line of the `}` stays
//!   shown when the `}` starts it (`} else {` is the header of the next one).
//! - Assembly: a function, a label at the start of a line that comes after
//!   a comment, a directive, a blank line or nothing (a label right after
//!   code is a label inside the function). It runs up to its last line of
//!   code before the next function.
//!
//! The text is never changed: the editor lays the hidden lines out with no
//! height (see [`hide`]).

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::ops::RangeInclusive;

use eframe::egui::Color32;
use eframe::egui::text::{ByteIndex, LayoutJob, LayoutSection};

use crate::highlight::{self, Kind, Syntax};

/// Lines (1-based) of a region: `header` stays shown, `header + 1..=last`
/// are hidden when it is folded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub header: u32,
    pub last: u32,
}

/// The regions of a text, sorted by header (one per header: the largest).
pub fn regions(text: &str, syntax: Syntax) -> Vec<Region> {
    let mut regions = match syntax {
        Syntax::C => c_regions(text),
        Syntax::Assembly => asm_regions(text),
    };
    regions.sort_by_key(|r| (r.header, std::cmp::Reverse(r.last)));
    regions.dedup_by_key(|r| r.header);
    regions
}

fn c_regions(text: &str) -> Vec<Region> {
    let lines: Vec<&str> = text.split('\n').collect();
    // Char index where each line starts.
    let mut starts = Vec::with_capacity(lines.len());
    let mut index = 0;
    for line in &lines {
        starts.push(index);
        index += line.chars().count() + 1;
    }
    let line_of = |at: usize| starts.partition_point(|&s| s <= at) as u32; // 1-based
    let mut open = Vec::new();
    let mut regions = Vec::new();
    for (at, bracket) in highlight::brackets(text, Syntax::C) {
        match bracket {
            '{' => open.push(line_of(at)),
            '}' => {
                let Some(header) = open.pop() else { continue };
                let close = line_of(at);
                let starts_line = lines[close as usize - 1].trim_start().starts_with('}');
                let last = if starts_line { close - 1 } else { close };
                if last > header {
                    regions.push(Region { header, last });
                }
            }
            _ => {}
        }
    }
    regions
}

fn asm_regions(text: &str) -> Vec<Region> {
    #[derive(PartialEq)]
    enum Line {
        Blank,
        Comment,
        Directive,
        /// A label at the start of the line (maybe with an instruction).
        Label,
        Code,
    }
    let kinds: Vec<Line> = text
        .split('\n')
        .map(|line| {
            let line = line.trim_end_matches('\r');
            let code = line.trim_start();
            if code.is_empty() {
                Line::Blank
            } else if code.starts_with(';') {
                Line::Comment
            } else if code.starts_with('.') {
                Line::Directive
            } else if matches!(highlight::spans(line).first(), Some((r, Kind::LabelDef)) if r.start == 0)
            {
                Line::Label
            } else {
                Line::Code
            }
        })
        .collect();
    let functions: Vec<usize> = (0..kinds.len())
        .filter(|&i| {
            kinds[i] == Line::Label
                && kinds[..i]
                    .iter()
                    .rev()
                    .find(|k| **k != Line::Blank)
                    .is_none_or(|k| matches!(k, Line::Comment | Line::Directive))
        })
        .collect();
    let mut regions = Vec::new();
    for (n, &start) in functions.iter().enumerate() {
        let end = functions.get(n + 1).copied().unwrap_or(kinds.len());
        let last = (start + 1..end)
            .rev()
            .find(|&i| matches!(kinds[i], Line::Label | Line::Code));
        if let Some(last) = last {
            regions.push(Region {
                header: start as u32 + 1,
                last: last as u32 + 1,
            });
        }
    }
    regions
}

/// The folds of a document: the header lines of its folded regions.
#[derive(Default)]
pub struct Folds {
    folded: BTreeSet<u32>,
    /// Regions of the text, keyed by a hash of the text.
    cache: Option<(u64, Vec<Region>)>,
}

impl Folds {
    /// The regions of `text` (computed again when it changed).
    pub fn regions(&mut self, text: &str, syntax: Syntax) -> &[Region] {
        let key = text_key(text, syntax);
        if self.cache.as_ref().is_none_or(|(k, _)| *k != key) {
            self.cache = Some((key, regions(text, syntax)));
        }
        &self.cache.as_ref().expect("filled above").1
    }

    /// The region whose header is `line`.
    pub fn region_at(&mut self, text: &str, syntax: Syntax, line: u32) -> Option<Region> {
        let regions = self.regions(text, syntax);
        regions
            .binary_search_by_key(&line, |r| r.header)
            .ok()
            .map(|i| regions[i])
    }

    pub fn is_folded(&self, header: u32) -> bool {
        self.folded.contains(&header)
    }

    /// The hidden lines, as sorted ranges that do not touch. Folds whose
    /// header no longer starts a region (the text changed) are dropped.
    pub fn hidden(&mut self, text: &str, syntax: Syntax) -> Vec<RangeInclusive<u32>> {
        if self.folded.is_empty() {
            return Vec::new();
        }
        let regions = self.regions(text, syntax).to_vec();
        self.folded
            .retain(|h| regions.binary_search_by_key(h, |r| r.header).is_ok());
        let mut hidden: Vec<RangeInclusive<u32>> = Vec::new();
        for r in regions.iter().filter(|r| self.folded.contains(&r.header)) {
            let (start, end) = (r.header + 1, r.last);
            match hidden.last_mut() {
                // Regions are sorted by header: a nested one is inside the last.
                Some(last) if start <= *last.end() + 1 => {
                    *last = *last.start()..=end.max(*last.end());
                }
                _ => hidden.push(start..=end),
            }
        }
        hidden
    }

    pub fn toggle(&mut self, header: u32) {
        if !self.folded.remove(&header) {
            self.folded.insert(header);
        }
    }

    /// Folds the regions no other region holds (functions, in C).
    pub fn fold_all(&mut self, text: &str, syntax: Syntax) {
        let mut end = 0;
        for r in self.regions(text, syntax).to_vec() {
            if r.header > end {
                self.folded.insert(r.header);
                end = r.last;
            }
        }
    }

    pub fn unfold_all(&mut self) {
        self.folded.clear();
    }

    /// Unfolds the regions that hide `line`.
    pub fn reveal(&mut self, text: &str, syntax: Syntax, line: u32) {
        if self.folded.is_empty() {
            return;
        }
        let hiding: Vec<u32> = self
            .regions(text, syntax)
            .iter()
            .filter(|r| r.header < line && line <= r.last)
            .map(|r| r.header)
            .collect();
        for header in hiding {
            self.folded.remove(&header);
        }
    }

    /// The text changed from `old` to `new`: the folds follow their lines.
    pub fn shift(&mut self, old: &str, new: &str) {
        crate::document::shift_breakpoints(&mut self.folded, old, new);
    }
}

fn text_key(text: &str, syntax: Syntax) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (text, syntax).hash(&mut hasher);
    hasher.finish()
}

/// Whether `line` is in one of the `hidden` ranges.
pub fn is_hidden(hidden: &[RangeInclusive<u32>], line: u32) -> Option<RangeInclusive<u32>> {
    hidden.iter().find(|r| r.contains(&line)).cloned()
}

/// Lays the `hidden` lines of `job` (the layout of `text`) out with no
/// height and no color.
pub fn hide(job: &mut LayoutJob, text: &str, hidden: &[RangeInclusive<u32>]) {
    if hidden.is_empty() {
        return;
    }
    // Byte ranges of the hidden lines, from the start of the first one to
    // the start of the line after the last one.
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let byte = |line: u32| starts.get(line as usize - 1).copied().unwrap_or(text.len());
    let ranges: Vec<(usize, usize)> = hidden
        .iter()
        .map(|r| (byte(*r.start()), byte(*r.end() + 1)))
        .collect();
    let mut sections = Vec::with_capacity(job.sections.len() + 2 * ranges.len());
    for section in job.sections.drain(..) {
        let (start, end) = (section.byte_range.start.0, section.byte_range.end.0);
        let mut cuts = vec![start];
        for &(a, b) in &ranges {
            for at in [a, b] {
                if start < at && at < end {
                    cuts.push(at);
                }
            }
        }
        cuts.push(end);
        cuts.sort_unstable();
        for (i, pair) in cuts.windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let mut piece = LayoutSection {
                leading_space: if i == 0 { section.leading_space } else { 0.0 },
                byte_range: ByteIndex(a)..ByteIndex(b),
                format: section.format.clone(),
            };
            if ranges.iter().any(|&(x, y)| x <= a && b <= y) {
                piece.format.line_height = Some(0.0);
                piece.format.color = Color32::TRANSPARENT;
                piece.format.background = Color32::TRANSPARENT;
                piece.leading_space = 0.0;
            }
            sections.push(piece);
        }
    }
    job.sections = sections;
}

/// Where the cursor goes when it moved from `old` to `new`, a char index
/// on a hidden line: past the hidden lines when it moved forward, on their
/// header when it moved back (at the end of it for one character), at the
/// column it had.
pub fn snap(text: &str, hidden: &[RangeInclusive<u32>], old: usize, new: usize) -> Option<usize> {
    let line = line_of(text, new);
    let range = is_hidden(hidden, line)?;
    let lines: Vec<&str> = text.split('\n').collect();
    let start_of = |line: u32| -> usize {
        lines[..line as usize - 1]
            .iter()
            .map(|l| l.chars().count() + 1)
            .sum()
    };
    let length = |line: u32| {
        lines[line as usize - 1]
            .trim_end_matches('\r')
            .chars()
            .count()
    };
    let old_line = line_of(text, old);
    let column = old - start_of(old_line);
    let header = *range.start() - 1;
    let after = *range.end() + 1;
    if new > old && (after as usize) <= lines.len() {
        let column = if new == old + 1 { 0 } else { column };
        Some(start_of(after) + column.min(length(after)))
    } else {
        let column = if new + 1 == old || new == old || header == 0 {
            usize::MAX
        } else {
            column
        };
        let header = header.max(1);
        Some(start_of(header) + column.min(length(header)))
    }
}

/// Line (1-based) of the char index `at`.
pub fn line_of(text: &str, at: usize) -> u32 {
    text.chars().take(at).filter(|&c| c == '\n').count() as u32 + 1
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn r(header: u32, last: u32) -> Region {
        Region { header, last }
    }

    #[test]
    fn c_blocks() {
        let text = "int f(void) {\n    if (x) {\n        y();\n    } else {\n        z();\n    }\n    return 0;\n}\nint g(void) { return 1; }\n";
        assert_eq!(regions(text, Syntax::C), [r(1, 7), r(2, 3), r(4, 5)],);
        // `}` after code: its line is hidden too.
        assert_eq!(regions("s = {\n 1,\n 2 };\nx;", Syntax::C), [r(1, 3)]);
        // Braces in comments and strings do not count.
        assert_eq!(regions("// {\nx;\n\"}\"\n", Syntax::C), []);
    }

    #[test]
    fn asm_functions() {
        let text = "\
; header
.data8 x 0

; f: does things
f:
    mov a, 1
loop:
    dec a

done:
    ret

; g
g:
    ret
; trailing comment
";
        assert_eq!(regions(text, Syntax::Assembly), [r(5, 11), r(14, 15)]);
        // A label with nothing after it has nothing to fold.
        assert_eq!(regions("start:\n; end\n", Syntax::Assembly), []);
    }

    #[test]
    fn hidden_ranges_and_fold_all() {
        let text =
            "int f(void) {\n    if (x) {\n        y();\n    }\n}\nint g(void) {\n    h();\n}\n";
        let mut folds = Folds::default();
        folds.fold_all(text, Syntax::C);
        assert!(folds.is_folded(1) && folds.is_folded(6) && !folds.is_folded(2));
        assert_eq!(folds.hidden(text, Syntax::C), [2..=4, 7..=7]);
        folds.reveal(text, Syntax::C, 3);
        assert_eq!(folds.hidden(text, Syntax::C), [7..=7]);
        // The header line changed: the fold goes.
        let changed = text.replacen("int g(void) {", "int g(void);", 1);
        assert_eq!(folds.hidden(&changed, Syntax::C), []);
    }

    #[test]
    fn snapping() {
        let text = "ab {\n  x\n  y\n}\nlast";
        let hidden = [2..=3];
        // Down from the header: the line after the hidden ones, same column.
        assert_eq!(snap(text, &hidden, 1, 6), Some(13 + 1));
        // Right at the end of the header: start of the line after.
        assert_eq!(snap(text, &hidden, 4, 5), Some(13));
        // Left at the start of the line after: end of the header.
        assert_eq!(snap(text, &hidden, 13, 12), Some(4));
        // Up from the line after: the header, same column.
        assert_eq!(snap(text, &hidden, 14, 10), Some(1));
        // Not hidden: nothing to do.
        assert_eq!(snap(text, &hidden, 0, 2), None);
    }

    #[test]
    fn hidden_lines_have_no_height() {
        use eframe::egui::{self, FontId};
        let text = "int f(void) {
    a();

    b();
}
end
";
        let tabs = highlight::Tabs {
            width: 4,
            char_width: 8.0,
        };
        let mut job = highlight::layout_job(text, Syntax::C, FontId::monospace(14.0), true, tabs);
        hide(&mut job, text, &[2..=4]);
        let ctx = egui::Context::default();
        let mut heights = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let galley = ui.fonts_mut(|f| f.layout_job(job.clone()));
            heights = galley.rows.iter().map(|r| r.rect().height()).collect();
        });
        output.textures_delta.clear(); // no GPU here
        assert_eq!(heights.len(), 7);
        let shown = heights[0];
        assert!(shown > 10.0);
        assert_eq!(&heights[1..4], [0.0; 3]);
        assert!(heights[4..].iter().all(|&h| h == shown), "{heights:?}");
    }
}
