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
//!   the startup code that clears the area.

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use hbc2_core::Opcode;
use hbc2_core::isa::{IVT_START, PROGRAM_START};
use hbc2_core::peripherals::eeprom::BOOT_RANGES;

use crate::diagnostic::{Loc, Report};
use crate::encode::{Resolved, encode_instruction, usage};
use crate::parser::{Address, DataValue, Located, Op, Operand, Stmt, Value8};
use crate::{SourceLine, Symbol, SymbolKind, Target};

pub(crate) enum Item {
    Stmt(Located),
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

/// Names defined by the assembler for the `.bss` area.
const BSS_START: &str = "__bss_start";
const BSS_END: &str = "__bss_end";

pub(crate) struct Output {
    pub image: Vec<u8>,
    pub symbols: Vec<Symbol>,
    pub lines: Vec<SourceLine>,
    pub bytes_used: usize,
}

/// `files`: the source files, for messages that name another file.
pub(crate) fn assemble(
    items: Vec<Item>,
    files: &[PathBuf],
    target: Target,
    report: &mut Report,
) -> Option<Output> {
    let (mut blocks, mut data, mut bss) = group(items, files, report);
    if report.has_errors() {
        return None;
    }
    let bss_area = place(&mut blocks, &mut data, &mut bss, target, report);
    if report.has_errors() {
        return None;
    }
    let symbols = symbols(&blocks, &data, &bss, bss_area);
    let output = write(&blocks, &data, symbols, target, report);
    if report.has_errors() {
        None
    } else {
        Some(output)
    }
}

// ----- 1. Group statements into blocks and data

fn group(
    items: Vec<Item>,
    files: &[PathBuf],
    report: &mut Report,
) -> (Vec<Block>, Vec<Data>, Vec<Bss>) {
    let mut blocks: Vec<Block> = Vec::new();
    let mut data: Vec<Data> = Vec::new();
    let mut bss: Vec<Bss> = Vec::new();
    let mut names: HashMap<String, Loc> = HashMap::new();
    let mut open: Option<usize> = None;

    let mut claim = |name: &str, loc: Loc, report: &mut Report| -> bool {
        if name == BSS_START || name == BSS_END {
            report.error(
                loc,
                format!("'{name}' is defined by the assembler (bounds of the .bss area)"),
            );
            return false;
        }
        if let Some(first) = names.get(name) {
            let message = match files.get(first.file) {
                Some(file) if first.file != loc.file => format!(
                    "'{name}' is already defined ({}, line {})",
                    file.display(),
                    first.line
                ),
                _ => format!("'{name}' is already defined (line {})", first.line),
            };
            report.error(loc, message);
            return false;
        }
        names.insert(name.to_string(), loc);
        true
    };

    for item in items {
        let Located { loc, stmt } = match item {
            Item::BlockBreak => {
                open = None;
                continue;
            }
            Item::Stmt(s) => s,
        };
        match stmt {
            Stmt::Label { name, mut address } => {
                if !claim(&name, loc, report) {
                    continue;
                }
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
                match (address, open) {
                    (None, Some(b)) => {
                        let offset = blocks[b].size();
                        blocks[b].labels.push((name, offset, loc));
                    }
                    _ => {
                        blocks.push(Block {
                            address,
                            loc,
                            labels: vec![(name, 0, loc)],
                            instructions: Vec::new(),
                        });
                        open = Some(blocks.len() - 1);
                    }
                }
            }
            Stmt::Instr { opcode, operands } => match open {
                Some(b) => blocks[b].instructions.push((opcode, operands, loc)),
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
                if claim(&name, loc, report) {
                    data.push(Data {
                        name,
                        address,
                        values,
                        loc,
                    });
                }
            }
            Stmt::Bss { name, size } => {
                if claim(&name, loc, report) {
                    bss.push(Bss {
                        name,
                        size,
                        address: 0,
                        loc,
                    });
                }
            }
        }
    }

    if !names.contains_key("_start") {
        report.global_error("the program has no _start: label");
    }
    (blocks, data, bss)
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

/// Places everything; returns the `.bss` area.
fn place(
    blocks: &mut [Block],
    data: &mut [Data],
    bss: &mut [Bss],
    target: Target,
    report: &mut Report,
) -> Range<u32> {
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
        return 0..0;
    }

    let mut free = auto_regions(target);
    for (range, _, _) in &fixed {
        subtract(&mut free, range);
    }

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
        Some(start)
    };

    for b in blocks.iter_mut().filter(|b| b.address.is_none()) {
        b.address = allocate(b.size(), &b.labels[0].0, b.loc, report);
    }
    for d in data.iter_mut().filter(|d| d.address.is_none()) {
        d.address = allocate(d.size(), &d.name, d.loc, report);
    }
    if report.has_errors() {
        return 0..0;
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
            return 0..0; // memory full, but nothing to place
        }
        report.error(
            bss[0].loc,
            format!("not enough free memory to place the .bss variables ({total} bytes)"),
        );
        return 0..0;
    };
    let start = gap.start;
    let mut a = start;
    for b in bss.iter_mut() {
        b.address = a;
        a += b.size;
    }
    start..a
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

fn symbols(blocks: &[Block], data: &[Data], bss: &[Bss], bss_area: Range<u32>) -> Vec<Symbol> {
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
    for (name, address) in [(BSS_START, bss_area.start), (BSS_END, bss_area.end)] {
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
