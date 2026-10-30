//! The C line of each line of a compiled file, from the `;#line n` lines
//! that cc2 writes before the code of each statement (and copt keeps).
//! With them, the program's instructions point at their C lines: the
//! debugger then shows, and stops on, the C source.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hbc2_asm::Program;
use hbc2_asm::source::normalize;

/// The C lines of the compiled files of a build.
#[derive(Default)]
pub(crate) struct LineMaps {
    /// compiled file (normalized) → (C file, C line of each compiled line)
    files: HashMap<PathBuf, (PathBuf, Vec<Option<u32>>)>,
}

/// The C line of each line of `asm` (index 0 = line 1): the line of the
/// last marker above it, none before the first marker or after a
/// `;#line 0` (code from a header).
pub(crate) fn c_lines(asm: &str) -> Vec<Option<u32>> {
    let mut current = None;
    asm.lines()
        .map(|line| {
            if let Some(n) = line.strip_prefix(";#line ") {
                current = n.trim().parse().ok().filter(|&n| n > 0);
            }
            current
        })
        .collect()
}

impl LineMaps {
    /// Reads the markers of each compiled file (`c_files`: compiled, C file).
    pub(crate) fn read(c_files: &[(PathBuf, PathBuf)]) -> LineMaps {
        let mut maps = LineMaps::default();
        for (compiled, source) in c_files {
            if let Ok(asm) = std::fs::read_to_string(compiled) {
                maps.files
                    .insert(normalize(compiled), (normalize(source), c_lines(&asm)));
            }
        }
        maps
    }

    /// The C file and line of a line of a compiled file, if it has one.
    pub(crate) fn c_line(&self, compiled: &Path, line: u32) -> Option<(&Path, u32)> {
        let (source, lines) = self.files.get(&normalize(compiled))?;
        let n = (*lines.get((line as usize).checked_sub(1)?)?)?;
        Some((source, n))
    }

    /// Points the program's instructions and labels at their C lines; the
    /// C files are added to `program.files`. The others stay where they are.
    pub(crate) fn apply(&self, program: &mut Program) {
        let mut index: HashMap<PathBuf, usize> = HashMap::new();
        let mut files = program.files.clone();
        let mut remap = |file: &mut usize, line: &mut u32| {
            let Some(path) = files.get(*file).cloned() else {
                return;
            };
            if let Some((source, n)) = self.c_line(&path, *line) {
                let i = *index.entry(source.to_path_buf()).or_insert_with(|| {
                    files.push(source.to_path_buf());
                    files.len() - 1
                });
                *file = i;
                *line = n;
            }
        };
        for l in &mut program.lines {
            remap(&mut l.file, &mut l.line);
        }
        for s in &mut program.symbols {
            remap(&mut s.file, &mut s.line);
        }
        program.files = files;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_from_markers() {
        let asm =
            "; C file 1\n_f:\n;#line 3\n\tmov a, 0\n__L1_2:\n;#line 0\n\tret\n;#line 5\n\tret\n";
        assert_eq!(
            c_lines(asm),
            [
                None,
                None,
                Some(3),
                Some(3),
                Some(3),
                None,
                None,
                Some(5),
                Some(5)
            ]
        );
    }
}
