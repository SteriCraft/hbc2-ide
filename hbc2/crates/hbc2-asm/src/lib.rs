//! Assembler and disassembler for the HBC-2 assembly language (`.has` files).
//!
//! The language is described in `Documentation/HBC-2 Assembly Language.md`.
//!
//! ```no_run
//! use hbc2_asm::{Target, assemble_file};
//! match assemble_file("main.has", Target::Ram) {
//!     Ok(program) => println!("{} bytes used", program.bytes_used),
//!     Err(errors) => errors.iter().for_each(|e| eprintln!("{e}")),
//! }
//! ```

mod assembler;
mod diagnostic;
pub mod disasm;
pub mod encode;
mod lexer;
mod parser;
mod preprocess;
pub mod source;

use std::path::{Path, PathBuf};

pub use diagnostic::{Diagnostic, Loc, Severity};
pub use disasm::disassemble;
pub use source::{FileSystem, MemoryFiles, SourceProvider};

use diagnostic::Report;

/// What the program is assembled for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A 64 KiB RAM image, loaded directly into the emulator's RAM.
    Ram,
    /// A 1 MiB EEPROM image. Only $0x0000-$0x01FF and $0xFE00-$0xFFFF are
    /// copied to RAM at power-on.
    Eeprom,
}

impl Target {
    pub const fn memory_size(self) -> u32 {
        match self {
            Target::Ram => 0x1_0000,
            Target::Eeprom => 0x10_0000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Label,
    Data,
}

/// A label or a `.data` variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub address: u32,
    /// Bytes until the next label (labels) or bytes of data (variables).
    pub size: u32,
    pub kind: SymbolKind,
    /// Index in [`Program::files`].
    pub file: usize,
    pub line: u32,
}

/// Where an instruction comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLine {
    pub address: u32,
    /// Index in [`Program::files`].
    pub file: usize,
    pub line: u32,
}

/// A successfully assembled program.
#[derive(Clone, Debug)]
pub struct Program {
    pub target: Target,
    /// Full memory image (64 KiB for RAM, 1 MiB for EEPROM).
    pub image: Vec<u8>,
    /// Every source file, in the order they were read. Index 0 is the entry file.
    pub files: Vec<PathBuf>,
    /// Sorted by address.
    pub symbols: Vec<Symbol>,
    /// One entry per instruction, sorted by address.
    pub lines: Vec<SourceLine>,
    pub warnings: Vec<Diagnostic>,
    pub bytes_used: usize,
}

impl Program {
    /// Source line of the instruction at `address`.
    pub fn source_of(&self, address: u32) -> Option<&SourceLine> {
        self.lines
            .binary_search_by_key(&address, |l| l.address)
            .ok()
            .map(|i| &self.lines[i])
    }

    /// Address of the instruction written on a line, if any (for breakpoints).
    pub fn address_of(&self, file: usize, line: u32) -> Option<u32> {
        self.lines
            .iter()
            .find(|l| l.file == file && l.line == line)
            .map(|l| l.address)
    }

    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// The label or variable starting exactly at `address`.
    pub fn symbol_at(&self, address: u32) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.address == address)
    }
}

/// Assembles `entry` and the files it includes, read through `provider`.
pub fn assemble(
    entry: &Path,
    provider: &dyn SourceProvider,
    target: Target,
) -> Result<Program, Vec<Diagnostic>> {
    let mut report = Report::default();
    let loaded = preprocess::load(entry, provider, &mut report);

    let mut items = Vec::new();
    for event in loaded.events {
        match event {
            preprocess::Event::BlockBreak => items.push(assembler::Item::BlockBreak),
            preprocess::Event::Line(line) => {
                let stmts = parser::parse_line(line.file, line.line, &line.tokens, &mut report);
                items.extend(stmts.into_iter().map(assembler::Item::Stmt));
            }
        }
    }

    let output = if report.has_errors() {
        None
    } else {
        assembler::assemble(items, &loaded.files, target, &mut report)
    };
    let diagnostics = report.into_diagnostics(&loaded.files);
    match output {
        Some(output) => Ok(Program {
            target,
            image: output.image,
            files: loaded.files,
            symbols: output.symbols,
            lines: output.lines,
            warnings: diagnostics,
            bytes_used: output.bytes_used,
        }),
        None => Err(diagnostics),
    }
}

/// Assembles a file from disk.
pub fn assemble_file(entry: impl AsRef<Path>, target: Target) -> Result<Program, Vec<Diagnostic>> {
    assemble(entry.as_ref(), &FileSystem, target)
}
