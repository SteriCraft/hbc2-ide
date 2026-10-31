//! From statements to a memory image: blocks, placement, name resolution, encoding.
//!
//! Placement rules:
//! * Code is grouped in blocks. A label with an address (or `_start:`, always
//!   at $0x0000) starts a block at that address.
//! * A label without an address continues the block above it (fall-through),
//!   unless no block is open: then it starts a block placed automatically.
//!   Entering or leaving an included file closes the open block.
//! * Blocks and `.data` without an address are placed automatically, in
//!   source order, in the first free gap large enough (code first, then data).
//! * `.bss` variables are placed together, in source order, in one free gap
//!   (after code and data). Nothing is written for them in the image.
//!   `__bss_start` and `__bss_end` (one past the last byte) are defined for
//!   the startup code that clears the area. `__program_end` is the address
//!   just after the last byte of code or data placed automatically (for a
//!   boot loader that copies the program).
//! * `.common` variables (C's `int n;`, which may be in several files) of
//!   the same name are one `.bss` variable, as large as the largest. A
//!   label, `.data` or `.bss` of that name is used instead of them.
//!
//! A program's own definition of a name replaces the library's (`.library`):
//! the library's definition gets a name nothing can refer to.
//!
//! With [`Options::strip`], what can't be reached is left out before
//! placement (see [`strip`]).

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use hbc2_core::Opcode;
use hbc2_core::isa::{IVT_START, PROGRAM_START};
use hbc2_core::peripherals::eeprom::BOOT_RANGES;

use crate::diagnostic::{Loc, Report};
use crate::encode::{Resolved, encode_instruction, usage};
use crate::parser::{Address, DataValue, Located, Op, Operand, Stmt, Value8};
use crate::{Options, Removed, SourceLine, Symbol, SymbolKind, Target};

pub(crate) enum Item {
    /// A statement, and whether it comes after `.library`.
    Stmt(Located, bool),
    BlockBreak,
}

struct Block {
    address: Option<u32>,
    loc: Loc,
    /// (name, offset in bytes, location)
    labels: Vec<(String, u32, Loc)>,
    instructions: Vec<(Opcode, Vec<Op>, Loc)>,
}

impl Block {
    fn size(&self) -> u32 {
        self.instructions.len() as u32 * 4
    }
}

struct Data {
    name: String,
    address: Option<u32>,
    values: Vec<DataValue>,
    loc: Loc,
}

impl Data {
    fn size(&self) -> u32 {
        self.values.iter().map(DataValue::size).sum()
    }
}

/// A `.bss` variable.
struct Bss {
    name: String,
    size: u32,
    address: u32,
    loc: Loc,
}

/// `.common` variables of one name, as [`group`] collects them.
struct Common {
    name: String,
    /// The largest size, and where it is.
    size: u32,
    loc: Loc,
    /// All of them are in the library.
    library: bool,
}

/// Names defined by the assembler: the `.bss` area, and the end of what
/// automatic placement used.
const BSS_START: &str = "__bss_start";
const BSS_END: &str = "__bss_end";
const PROGRAM_END: &str = "__program_end";

fn assembler_name(name: &str) -> bool {
    name == BSS_START || name == BSS_END || name == PROGRAM_END
}

pub(crate) struct Output {
    pub image: Vec<u8>,
    pub symbols: Vec<Symbol>,
    pub lines: Vec<SourceLine>,
    pub bytes_used: usize,
    pub removed: Vec<Removed>,
}

/// What [`group`] collects.
struct Grouped {
    blocks: Vec<Block>,
    data: Vec<Data>,
    bss: Vec<Bss>,
    /// `.keep` names.
    keeps: Vec<(String, Loc)>,
}

/// `files`: the source files, for messages that name another file.
pub(crate) fn assemble(
    items: Vec<Item>,
    files: &[PathBuf],
    options: Options,
    report: &mut Report,
) -> Option<Output> {
    let target = options.target;
    let Grouped {
        mut blocks,
        mut data,
        mut bss,
        keeps,
    } = group(items, files, report);
    if report.has_errors() {
        return None;
    }
    let removed = if options.strip {
        let removed = strip(&mut blocks, &mut data, &mut bss, &keeps, report);
        if report.has_errors() {
            return None;
        }
        removed
    } else {
        Vec::new()
    };
    let (bss_area, program_end) = place(&mut blocks, &mut data, &mut bss, target, report);
    if report.has_errors() {
        return None;
    }
    let symbols = symbols(&blocks, &data, &bss, bss_area, program_end);
    let mut output = write(&blocks, &data, symbols, target, report);
    output.removed = removed;
    if report.has_errors() {
        None
    } else {
        Some(output)
    }
}

// ----- 1. Group statements into blocks and data

fn group(items: Vec<Item>, files: &[PathBuf], report: &mut Report) -> Grouped {
    let mut out = Grouped {
        blocks: Vec::new(),
        data: Vec::new(),
        bss: Vec::new(),
        keeps: Vec::new(),
    };
    let mut names = Names {
        owners: HashMap::new(),
        files,
    };
    let mut open: Option<usize> = None;
    let mut commons: Vec<Common> = Vec::new();

    for item in items {
        let (Located { loc, stmt }, library) = match item {
            Item::BlockBreak => {
                open = None;
                continue;
            }
            Item::Stmt(s, library) => (s, library),
        };
        match stmt {
            Stmt::Label { name, mut address } => {
                if name == "_start" {
                    if address.is_some_and(|a| a != PROGRAM_START as u32) {
                        report.error(
                            loc,
                            format!(
                                "_start is always at ${PROGRAM_START:#06x}; remove its address"
                            ),
                        );
                    }
                    address = Some(PROGRAM_START as u32);
                }
                // A label without an address continues the open block.
                let continued = open.filter(|_| address.is_none());
                let at = match continued {
                    Some(b) => At::Label(b, out.blocks[b].labels.len()),
                    None => At::Label(out.blocks.len(), 0),
                };
                let Some(name) = names.define(&mut out, at, name, loc, library, report) else {
                    continue;
                };
                match continued {
                    Some(b) => {
                        let offset = out.blocks[b].size();
                        out.blocks[b].labels.push((name, offset, loc));
                    }
                    None => {
                        out.blocks.push(Block {
                            address,
                            loc,
                            labels: vec![(name, 0, loc)],
                            instructions: Vec::new(),
                        });
                        open = Some(out.blocks.len() - 1);
                    }
                }
            }
            Stmt::Instr { opcode, operands } => match open {
                Some(b) => out.blocks[b].instructions.push((opcode, operands, loc)),
                None => report.error(
                    loc,
                    "this instruction has no label above it (add one, e.g. _start:)",
                ),
            },
            Stmt::Data {
                name,
                address,
                values,
            } => {
                let at = At::Data(out.data.len());
                if let Some(name) = names.define(&mut out, at, name, loc, library, report) {
                    out.data.push(Data {
                        name,
                        address,
                        values,
                        loc,
                    });
                }
            }
            Stmt::Bss { name, size } => {
                let at = At::Bss(out.bss.len());
                if let Some(name) = names.define(&mut out, at, name, loc, library, report) {
                    out.bss.push(Bss {
                        name,
                        size,
                        address: 0,
                        loc,
                    });
                }
            }
            Stmt::Common { name, size } => match commons.iter_mut().find(|c| c.name == name) {
                Some(common) => {
                    if common.size < size {
                        (common.size, common.loc) = (size, loc);
                    }
                    common.library &= library;
                }
                None => commons.push(Common {
                    name,
                    size,
                    loc,
                    library,
                }),
            },
            Stmt::Keep { name } => out.keeps.push((name, loc)),
        }
    }
    for common in commons {
        names.common(&mut out, common, report);
    }

    if !names.owners.contains_key("_start") {
        report.global_error("the program has no _start: label");
    }
    for (name, loc) in &out.keeps {
        if !names.owners.contains_key(name) {
            report.error(*loc, format!(".keep: '{name}' is not defined"));
        }
    }
    out
}

/// Where a definition is (or will be) in [`Grouped`].
#[derive(Clone, Copy)]
enum At {
    /// `blocks[.0].labels[.1]`
    Label(usize, usize),
    Data(usize),
    Bss(usize),
}

/// The definition that owns a name.
#[derive(Clone, Copy)]
struct Definition {
    loc: Loc,
    library: bool,
    at: At,
}

/// Every name defined so far.
struct Names<'a> {
    owners: HashMap<String, Definition>,
    /// The source files, for messages that name another file.
    files: &'a [PathBuf],
}

/// A name nothing can refer to (names never hold a space), for a library
/// definition replaced by the program's.
fn hidden(name: &str) -> String {
    format!("{name} (library, replaced)")
}

impl Names<'_> {
    /// Records a definition of `name` that will be put `at` and returns the
    /// name to give it (`None` on an error). A duplicate is an error, unless
    /// exactly one of the two is a library definition: that one gives way
    /// and gets a hidden name (renamed in `out` if it came first).
    fn define(
        &mut self,
        out: &mut Grouped,
        at: At,
        name: String,
        loc: Loc,
        library: bool,
        report: &mut Report,
    ) -> Option<String> {
        if assembler_name(&name) {
            report.error(
                loc,
                format!("'{name}' is defined by the assembler"),
            );
            return None;
        }
        let new = Definition { loc, library, at };
        let Some(first) = self.owners.get(&name).copied() else {
            self.owners.insert(name.clone(), new);
            return Some(name);
        };
        match (first.library, library) {
            (false, true) => Some(hidden(&name)),
            (true, false) => {
                let renamed = hidden(&name);
                match first.at {
                    At::Label(b, l) => out.blocks[b].labels[l].0 = renamed,
                    At::Data(i) => out.data[i].name = renamed,
                    At::Bss(i) => out.bss[i].name = renamed,
                }
                self.owners.insert(name.clone(), new);
                Some(name)
            }
            _ => {
                let message = format!(
                    "'{name}' is already defined ({})",
                    self.place(first.loc, loc)
                );
                report.error(loc, message);
                None
            }
        }
    }

    /// Settles a `.common` variable once every other definition is known:
    /// a `.data` or `.bss` of the same name is used instead (it must be
    /// large enough), else it becomes a `.bss` variable. The library's
    /// definitions give way to the program's `.common`, as to its others.
    fn common(&mut self, out: &mut Grouped, common: Common, report: &mut Report) {
        let Common {
            name,
            size,
            loc,
            library,
        } = common;
        let first = match self.owners.get(&name) {
            Some(first) if !(first.library && !library) => *first,
            _ => {
                let at = At::Bss(out.bss.len());
                if let Some(name) = self.define(out, at, name, loc, library, report) {
                    out.bss.push(Bss {
                        name,
                        size,
                        address: 0,
                        loc,
                    });
                }
                return;
            }
        };
        let place = self.place(first.loc, loc);
        let defined = match first.at {
            At::Label(..) => {
                report.error(
                    loc,
                    format!("'{name}' is a label ({place}), not a variable"),
                );
                return;
            }
            At::Data(i) => out.data[i].size(),
            At::Bss(i) => out.bss[i].size,
        };
        if defined < size {
            report.error(
                loc,
                format!("'{name}' is {defined} bytes ({place}), smaller than this .common ({size} bytes)"),
            );
        }
    }

    /// Where `first` is, said from `loc`: "line 4", or "file, line 4".
    fn place(&self, first: Loc, loc: Loc) -> String {
        match self.files.get(first.file) {
            Some(file) if first.file != loc.file => {
                format!("{}, line {}", file.display(), first.line)
            }
            _ => format!("line {}", first.line),
        }
    }
}

// ----- 1b. Removal of what can't be reached (Options::strip)

/// One piece of the program, kept or left out as a whole.
#[derive(Clone, Copy)]
enum Piece {
    /// From label `label` of block `block` up to the block's next label.
    Code {
        block: usize,
        label: usize,
    },
    Data(usize),
    Bss(usize),
}

/// Instructions after which execution never goes on to the next one.
fn ends_flow(opcode: Opcode) -> bool {
    matches!(opcode, Opcode::Jmp | Opcode::Ret | Opcode::Irt)
}

/// The name an 8-bit value refers to, if any.
fn value8_name(value: &Value8) -> Option<&str> {
    match value {
        Value8::Number(_) => None,
        Value8::Msb(name) | Value8::Lsb(name) => Some(name),
    }
}

/// The name an operand refers to, if any.
fn operand_name(operand: &Operand) -> Option<&str> {
    match operand {
        Operand::Reg(_) | Operand::Wide(_) | Operand::Mem(Address::Absolute(_)) => None,
        Operand::Pair(_, _, v) | Operand::Stack(v) | Operand::Imm(v) => value8_name(v),
        Operand::Mem(Address::Symbol { name, .. }) => Some(name),
    }
}

/// Instructions of the code piece starting at label `label`.
fn piece_range(block: &Block, label: usize) -> Range<usize> {
    let start = block.labels[label].1 as usize / 4;
    let end = block
        .labels
        .get(label + 1)
        .map_or(block.instructions.len(), |l| l.1 as usize / 4);
    start..end
}

/// Leaves out every piece that can't be reached and returns what was left out.
///
/// A piece is the code from one label up to the next label, one `.data` or
/// one `.bss` variable. A piece uses the names it refers to, and a code piece
/// that doesn't end with `jmp`, `ret` or `irt` (an empty one too) also uses
/// the piece after it. The walk starts from `_start`, everything at a fixed
/// address (a block at a fixed address is kept whole, except `_start`'s) and
/// the `.keep` names. Kept pieces of a block stay together, in
/// order.
///
/// Names that aren't defined are reported here, in removed pieces too (a typo
/// in unused code still fails the build).
fn strip(
    blocks: &mut Vec<Block>,
    data: &mut Vec<Data>,
    bss: &mut Vec<Bss>,
    keeps: &[(String, Loc)],
    report: &mut Report,
) -> Vec<Removed> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut by_name: HashMap<&str, usize> = HashMap::new();
    for (b, block) in blocks.iter().enumerate() {
        for (l, (name, _, _)) in block.labels.iter().enumerate() {
            by_name.insert(name, pieces.len());
            pieces.push(Piece::Code { block: b, label: l });
        }
    }
    let data_start = pieces.len();
    for (i, d) in data.iter().enumerate() {
        by_name.insert(&d.name, pieces.len());
        pieces.push(Piece::Data(i));
    }
    let bss_start = pieces.len();
    for (i, v) in bss.iter().enumerate() {
        by_name.insert(&v.name, pieces.len());
        pieces.push(Piece::Bss(i));
    }

    // What each piece uses.
    let mut uses: Vec<Vec<usize>> = vec![Vec::new(); pieces.len()];
    for (p, piece) in pieces.iter().enumerate() {
        let mut use_name = |name: &str, loc: Loc| match by_name.get(name) {
            Some(&to) => uses[p].push(to),
            None if assembler_name(name) => {}
            None => report.error(loc, format!("unknown label or variable '{name}'")),
        };
        match *piece {
            Piece::Code { block, label } => {
                let block = &blocks[block];
                let instructions = &block.instructions[piece_range(block, label)];
                for (_, operands, loc) in instructions {
                    for op in operands {
                        if let Some(name) = operand_name(&op.operand) {
                            use_name(
                                name,
                                Loc {
                                    col: op.col,
                                    ..*loc
                                },
                            );
                        }
                    }
                }
                let falls = instructions.last().is_none_or(|(o, _, _)| !ends_flow(*o));
                if falls && label + 1 < block.labels.len() {
                    uses[p].push(p + 1);
                }
            }
            Piece::Data(i) => {
                let d = &data[i];
                for value in &d.values {
                    let (name, col) = match value {
                        DataValue::Byte(v, col) => (value8_name(v), *col),
                        DataValue::Address(Address::Symbol { name, .. }, _, col) => {
                            (Some(name.as_str()), *col)
                        }
                        DataValue::Bytes(_) | DataValue::Address(Address::Absolute(_), _, _) => {
                            continue;
                        }
                    };
                    if let Some(name) = name {
                        use_name(name, Loc { col, ..d.loc });
                    }
                }
            }
            Piece::Bss(_) => {}
        }
    }
    if report.has_errors() {
        return Vec::new();
    }

    // Walk from the roots.
    let mut todo: Vec<usize> = Vec::new();
    for (p, piece) in pieces.iter().enumerate() {
        let fixed = match *piece {
            // `_start` is always at $0x0000 but its block isn't kept whole:
            // only `_start` itself is a root (below).
            Piece::Code { block, .. } => {
                blocks[block].address.is_some() && blocks[block].labels[0].0 != "_start"
            }
            Piece::Data(i) => data[i].address.is_some(),
            Piece::Bss(_) => false,
        };
        if fixed {
            todo.push(p);
        }
    }
    todo.extend(by_name.get("_start"));
    todo.extend(
        keeps
            .iter()
            .filter_map(|(name, _)| by_name.get(name.as_str())),
    );
    let mut kept = vec![false; pieces.len()];
    while let Some(p) = todo.pop() {
        if !kept[p] {
            kept[p] = true;
            todo.extend(&uses[p]);
        }
    }

    // What is left out, in source order.
    let mut removed: Vec<Removed> = Vec::new();
    for (p, piece) in pieces.iter().enumerate() {
        if kept[p] {
            continue;
        }
        let (name, size, kind, loc) = match *piece {
            Piece::Code { block, label } => {
                let b = &blocks[block];
                let (name, _, loc) = &b.labels[label];
                let size = piece_range(b, label).len() as u32 * 4;
                (name, size, SymbolKind::Label, *loc)
            }
            Piece::Data(i) => (&data[i].name, data[i].size(), SymbolKind::Data, data[i].loc),
            Piece::Bss(i) => (&bss[i].name, bss[i].size, SymbolKind::Data, bss[i].loc),
        };
        removed.push(Removed {
            name: name.clone(),
            size,
            kind,
            file: loc.file,
            line: loc.line,
        });
    }
    removed.sort_by_key(|r| (r.file, r.line));

    // Rebuild with the kept pieces. Pieces of block b are numbered from
    // `first`, in label order.
    let mut first = 0;
    let mut rebuilt = Vec::with_capacity(blocks.len());
    for block in std::mem::take(blocks) {
        let ranges: Vec<Range<usize>> = (0..block.labels.len())
            .map(|l| piece_range(&block, l))
            .collect();
        let mut labels = Vec::new();
        let mut instructions = Vec::new();
        let mut source = block.instructions.into_iter();
        for (l, (name, _, loc)) in block.labels.into_iter().enumerate() {
            let piece: Vec<_> = source.by_ref().take(ranges[l].len()).collect();
            if kept[first + l] {
                labels.push((name, instructions.len() as u32 * 4, loc));
                instructions.extend(piece);
            }
        }
        first += ranges.len();
        if let Some(&(_, _, loc)) = labels.first() {
            rebuilt.push(Block {
                address: block.address,
                loc,
                labels,
                instructions,
            });
        }
    }
    *blocks = rebuilt;
    let mut n = data_start;
    data.retain(|_| {
        n += 1;
        kept[n - 1]
    });
    let mut n = bss_start;
    bss.retain(|_| {
        n += 1;
        kept[n - 1]
    });

    removed
}

// ----- 2. Placement

/// Where automatic placement may put things.
#[allow(clippy::single_range_in_vec_init)] // a list of regions, one of them for RAM
/// Never in the interrupt vector table (from $0xFE00).
fn auto_regions(target: Target) -> Vec<Range<u32>> {
    let ivt = IVT_START as u32;
    match target {
        Target::Ram => vec![PROGRAM_START as u32..ivt],
        // Only these ranges are copied from the EEPROM to RAM at power-on.
        Target::Eeprom => vec![PROGRAM_START as u32..BOOT_RANGES[0].end as u32],
    }
}

/// Places everything; returns the `.bss` area and the address after the
/// last byte placed automatically (code and data).
fn place(
    blocks: &mut [Block],
    data: &mut [Data],
    bss: &mut [Bss],
    target: Target,
    report: &mut Report,
) -> (Range<u32>, u32) {
    // Fixed ranges: (range, name, location)
    let mut fixed: Vec<(Range<u32>, String, Loc)> = Vec::new();
    for b in blocks.iter() {
        if let Some(a) = b.address {
            fixed.push((a..a + b.size(), b.labels[0].0.clone(), b.loc));
        }
    }
    for d in data.iter() {
        if let Some(a) = d.address {
            fixed.push((a..a + d.size(), d.name.clone(), d.loc));
        }
    }

    let end = target.memory_size();
    for (range, name, loc) in &fixed {
        if range.end > end {
            report.error(
                *loc,
                format!(
                    "'{name}' ends at {:#x}, past the end of memory ({end:#x})",
                    range.end
                ),
            );
        }
    }
    fixed.sort_by_key(|(r, _, _)| r.start);
    let non_empty: Vec<_> = fixed.iter().filter(|(r, _, _)| !r.is_empty()).collect();
    for pair in non_empty.windows(2) {
        let (a, a_name, _) = pair[0];
        let (b, b_name, b_loc) = pair[1];
        if b.start < a.end {
            report.error(
                *b_loc,
                format!(
                    "'{b_name}' overlaps '{a_name}' (${:#06x}-${:#06x})",
                    a.start,
                    a.end - 1
                ),
            );
        }
    }
    if report.has_errors() {
        return (0..0, 0);
    }

    let mut free = auto_regions(target);
    for (range, _, _) in &fixed {
        subtract(&mut free, range);
    }

    let mut program_end = 0;
    let mut allocate = |size: u32, name: &str, loc: Loc, report: &mut Report| -> Option<u32> {
        let Some(gap) = free.iter().position(|g| g.end - g.start >= size) else {
            report.error(
                loc,
                format!("not enough free memory to place '{name}' ({size} bytes)"),
            );
            return None;
        };
        let start = free[gap].start;
        subtract(&mut free, &(start..start + size));
        program_end = program_end.max(start + size);
        Some(start)
    };

    for b in blocks.iter_mut().filter(|b| b.address.is_none()) {
        b.address = allocate(b.size(), &b.labels[0].0, b.loc, report);
    }
    for d in data.iter_mut().filter(|d| d.address.is_none()) {
        d.address = allocate(d.size(), &d.name, d.loc, report);
    }
    if report.has_errors() {
        return (0..0, 0);
    }

    // .bss is RAM only (nothing is in the image), so it may go anywhere in
    // RAM that is free, for the EEPROM target too.
    #[allow(clippy::single_range_in_vec_init)] // a list of free ranges
    let mut ram = vec![PROGRAM_START as u32..IVT_START as u32];
    for b in blocks.iter() {
        let a = b.address.expect("placed");
        subtract(&mut ram, &(a..a + b.size()));
    }
    for d in data.iter() {
        let a = d.address.expect("placed");
        subtract(&mut ram, &(a..a + d.size()));
    }
    let total: u32 = bss.iter().map(|b| b.size).sum();
    let Some(gap) = ram.iter().find(|g| g.end - g.start >= total) else {
        if bss.is_empty() {
            return (0..0, 0); // memory full, but nothing to place
        }
        report.error(
            bss[0].loc,
            format!("not enough free memory to place the .bss variables ({total} bytes)"),
        );
        return (0..0, 0);
    };
    let start = gap.start;
    let mut a = start;
    for b in bss.iter_mut() {
        b.address = a;
        a += b.size;
    }
    (start..a, program_end)
}

/// Removes `taken` from the list of free ranges (kept sorted).
fn subtract(free: &mut Vec<Range<u32>>, taken: &Range<u32>) {
    if taken.is_empty() {
        return;
    }
    let mut out = Vec::with_capacity(free.len() + 1);
    for gap in free.drain(..) {
        if taken.end <= gap.start || taken.start >= gap.end {
            out.push(gap);
            continue;
        }
        if gap.start < taken.start {
            out.push(gap.start..taken.start);
        }
        if taken.end < gap.end {
            out.push(taken.end..gap.end);
        }
    }
    *free = out;
}

// ----- 3. Symbols

fn symbols(
    blocks: &[Block],
    data: &[Data],
    bss: &[Bss],
    bss_area: Range<u32>,
    program_end: u32,
) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for b in blocks {
        let start = b.address.expect("placed");
        for (i, (name, offset, loc)) in b.labels.iter().enumerate() {
            let next = b.labels.get(i + 1).map_or(b.size(), |l| l.1);
            symbols.push(Symbol {
                name: name.clone(),
                address: start + offset,
                size: next - offset,
                kind: SymbolKind::Label,
                file: loc.file,
                line: loc.line,
            });
        }
    }
    for d in data {
        symbols.push(Symbol {
            name: d.name.clone(),
            address: d.address.expect("placed"),
            size: d.size(),
            kind: SymbolKind::Data,
            file: d.loc.file,
            line: d.loc.line,
        });
    }
    for b in bss {
        symbols.push(Symbol {
            name: b.name.clone(),
            address: b.address,
            size: b.size,
            kind: SymbolKind::Data,
            file: b.loc.file,
            line: b.loc.line,
        });
    }
    // Last, so that a variable at the same address is found first by address.
    for (name, address) in [
        (BSS_START, bss_area.start),
        (BSS_END, bss_area.end),
        (PROGRAM_END, program_end),
    ] {
        symbols.push(Symbol {
            name: name.to_string(),
            address,
            size: 0,
            kind: SymbolKind::Label,
            file: 0,
            line: 0,
        });
    }
    symbols.sort_by_key(|s| s.address); // stable
    symbols
}

// ----- 4. Resolution and encoding

fn write(
    blocks: &[Block],
    data: &[Data],
    symbols: Vec<Symbol>,
    target: Target,
    report: &mut Report,
) -> Output {
    let by_name: HashMap<&str, u32> = symbols
        .iter()
        .map(|s| (s.name.as_str(), s.address))
        .collect();
    let mut image = vec![0u8; target.memory_size() as usize];
    let mut lines = Vec::new();
    let mut bytes_used = 0usize;

    let lookup = |name: &str, loc: Loc, report: &mut Report| -> Option<u32> {
        let found = by_name.get(name).copied();
        if found.is_none() {
            report.error(loc, format!("unknown label or variable '{name}'"));
        }
        found
    };
    let value8 = |value: &Value8, loc: Loc, report: &mut Report| -> Option<u8> {
        match value {
            Value8::Number(n) => Some(*n as u8), // -128..=255, checked by the parser
            Value8::Msb(name) => lookup(name, loc, report).map(|a| (a >> 8) as u8),
            Value8::Lsb(name) => lookup(name, loc, report).map(|a| a as u8),
        }
    };

    for block in blocks {
        let start = block.address.expect("placed");
        for (n, (opcode, operands, loc)) in block.instructions.iter().enumerate() {
            let address = start + n as u32 * 4;
            let mut resolved = Vec::with_capacity(operands.len());
            for op in operands {
                let op_loc = Loc {
                    col: op.col,
                    ..*loc
                };
                let value = match &op.operand {
                    Operand::Reg(r) => Some(Resolved::Reg(*r)),
                    Operand::Pair(h, l, offset) => {
                        value8(offset, op_loc, report).map(|n| Resolved::Pair(*h, *l, n))
                    }
                    Operand::Stack(offset) => value8(offset, op_loc, report).map(Resolved::Stack),
                    Operand::Imm(v) => value8(v, op_loc, report).map(Resolved::Imm),
                    Operand::Wide(n) => Some(Resolved::Wide(*n)),
                    Operand::Mem(Address::Absolute(a)) => address16(*a as i64, op_loc, report),
                    Operand::Mem(Address::Symbol { name, offset }) => lookup(name, op_loc, report)
                        .and_then(|a| address16(a as i64 + offset, op_loc, report)),
                };
                resolved.extend(value);
            }
            if resolved.len() != operands.len() {
                continue; // an operand failed; already reported
            }
            match encode_instruction(*opcode, &resolved) {
                Some(word) => {
                    let a = address as usize;
                    image[a..a + 4].copy_from_slice(&word.to_bytes());
                    lines.push(SourceLine {
                        address,
                        file: loc.file,
                        line: loc.line,
                    });
                }
                None => {
                    let offset = operands
                        .iter()
                        .find(|op| {
                            matches!(&op.operand, Operand::Pair(_, _, n) if *n != Value8::Number(0))
                        });
                    match offset {
                        Some(op) => report.error(
                            Loc {
                                col: op.col,
                                ..*loc
                            },
                            format!(
                                "{} takes no offset here; expected: {}",
                                opcode.mnemonic(),
                                usage(*opcode)
                            ),
                        ),
                        None => report.error(
                            *loc,
                            format!("invalid operands; expected: {}", usage(*opcode)),
                        ),
                    }
                }
            }
        }
        bytes_used += block.size() as usize;
        warn_location(
            target,
            start..start + block.size(),
            &block.labels[0].0,
            block.loc,
            report,
        );
    }

    for d in data {
        let start = d.address.expect("placed");
        let mut a = start as usize;
        for value in &d.values {
            match value {
                DataValue::Bytes(bytes) => {
                    image[a..a + bytes.len()].copy_from_slice(bytes);
                    a += bytes.len();
                }
                DataValue::Byte(v, col) => {
                    if let Some(byte) = value8(v, Loc { col: *col, ..d.loc }, report) {
                        image[a] = byte;
                    }
                    a += 1;
                }
                DataValue::Address(address, size, col) => {
                    let loc = Loc { col: *col, ..d.loc };
                    let value = match address {
                        Address::Absolute(n) => Some(*n as i64),
                        Address::Symbol { name, offset } => {
                            lookup(name, loc, report).map(|n| n as i64 + offset)
                        }
                    };
                    let size = *size as usize;
                    let bits = size * 8;
                    match value {
                        Some(v) if (0..1i64 << bits).contains(&v) => {
                            let bytes = (v as u64).to_be_bytes();
                            image[a..a + size].copy_from_slice(&bytes[8 - size..]);
                        }
                        Some(v) => {
                            report.error(loc, format!("address {v:#x} does not fit in {bits} bits"))
                        }
                        None => {}
                    }
                    a += size;
                }
            }
        }
        bytes_used += d.size() as usize;
        if d.address.is_some() {
            warn_location(target, start..start + d.size(), &d.name, d.loc, report);
        }
    }

    lines.sort_by_key(|l| l.address);
    Output {
        image,
        symbols,
        lines,
        bytes_used,
        removed: Vec::new(),
    }
}

fn address16(value: i64, loc: Loc, report: &mut Report) -> Option<Resolved> {
    if (0..=0xFFFF).contains(&value) {
        Some(Resolved::Mem(value as u16))
    } else {
        report.error(
            loc,
            format!("address {value:#x} does not fit in 16 bits ($0x0000-$0xFFFF)"),
        );
        None
    }
}

fn warn_location(target: Target, range: Range<u32>, name: &str, loc: Loc, report: &mut Report) {
    if range.is_empty() {
        return;
    }
    match target {
        Target::Ram => {}
        Target::Eeprom => {
            if !BOOT_RANGES
                .iter()
                .any(|b| b.start as u32 <= range.start && range.end <= b.end as u32)
            {
                report.warning(
                    loc,
                    format!("'{name}' is not in a range copied to RAM at power-on ($0x0000-$0x01FF, $0xFE00-$0xFFFF)"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::subtract;

    #[test]
    fn subtract_ranges() {
        let mut free = vec![0..100, 200..300];
        subtract(&mut free, &(50..250));
        assert_eq!(free, [0..50, 250..300]);
        subtract(&mut free, &(10..20));
        assert_eq!(free, [0..10, 20..50, 250..300]);
        subtract(&mut free, &(250..300));
        assert_eq!(free, [0..10, 20..50]);
    }
}
