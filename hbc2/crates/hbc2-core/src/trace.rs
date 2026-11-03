//! Execution trace: one CSV line per executed instruction, to compare the
//! emulator with the TTL circuit drawn in Logisim, line by line.
//!
//! The CPU reports what it does to a [`Probe`]. [`NoProbe`] ignores
//! everything and compiles to nothing: [`crate::Machine::run`] and
//! [`crate::Machine::step`] use it, so tracing costs nothing when it is off.
//! [`TraceWriter`] is the probe that writes the CSV file.
//!
//! # Format
//!
//! ASCII, `\n` line endings, `,` separator, no spaces, upper-case hex with
//! leading zeros, no `0x`. The header is [`TRACE_HEADER`], then:
//!
//! | column | content |
//! |---|---|
//! | `n` | line number, decimal; 0 = state when the trace started |
//! | `pc` | address of the instruction (4 hex digits) |
//! | `ir` | the 1 to 4 bytes of the instruction (2 to 8 hex digits); empty on line 0; `INT` when entering an interrupt handler |
//! | `a`..`y` | registers after the instruction (2 hex digits each) |
//! | `sp` | stack pointer after (4 hex digits) |
//! | `flags` | flag register after (2 hex digits, bit 0 = C … bit 7 = H) |
//! | `mem_w` | RAM writes, in order: `AAAA=VV` separated by `;` |
//! | `io_w` | port writes (`OUT`): `PP=VV` separated by `;` |
//!
//! An `INT` line is written when the CPU enters an interrupt handler
//! (hardware or software): `pc` is the interrupted address (the one
//! pushed), `mem_w` holds the three pushes (I, then PC LSB, then PC MSB),
//! and the registers and flags are those at the handler's entry.
//!
//! An invalid opcode is a NOP and gets a normal line. A valid opcode with an
//! invalid addressing mode gets a normal line with H set and I cleared (the
//! CPU stops there).
//!
//! [`compare`] finds the first line where two traces differ (`hbc2 compare`).

use std::fmt;
use std::io::{self, Write};

use crate::cpu::Cpu;
use crate::isa::Instruction;

/// First line of a trace file.
pub const TRACE_HEADER: &str = "n,pc,ir,a,b,c,d,i,j,x,y,sp,flags,mem_w,io_w";

/// What the CPU tells an observer. Every method does nothing by default.
pub trait Probe {
    /// The CPU wrote `value` at `address` in RAM.
    #[inline(always)]
    fn mem_write(&mut self, _address: u16, _value: u8) {}

    /// The CPU wrote `value` on `port` (`OUT`).
    #[inline(always)]
    fn io_write(&mut self, _port: u8, _value: u8) {}

    /// An instruction is done (or stopped the CPU, if its addressing mode
    /// is invalid). `pc` is its address, `ir` its bytes; `cpu` is the state
    /// after it.
    #[inline(always)]
    fn instruction(&mut self, _cpu: &Cpu, _pc: u16, _ir: Instruction) {}

    /// The CPU entered an interrupt handler. `interrupted` is the address
    /// it will return to; `cpu` is the state at the handler's entry.
    #[inline(always)]
    fn interrupt(&mut self, _cpu: &Cpu, _interrupted: u16) {}
}

/// Ignores everything.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProbe;

impl Probe for NoProbe {}

/// Writes the trace CSV to `W` (wrap a file in a `BufWriter`).
///
/// Call [`TraceWriter::start`] with the state before the first instruction:
/// it writes the header and line 0. Events before that are ignored. With a
/// limit, lines past it are dropped and [`TraceWriter::is_full`] turns true.
pub struct TraceWriter<W: Write> {
    out: W,
    /// Number of the last line written (0 = line 0).
    n: u64,
    started: bool,
    /// Highest line number written.
    limit: Option<u64>,
    /// Pending `mem_w` and `io_w` columns of the line being built.
    mem_w: Vec<u8>,
    io_w: Vec<u8>,
    line: Vec<u8>,
    /// First write error; nothing is written after it.
    error: Option<io::Error>,
}

impl<W: Write> TraceWriter<W> {
    /// `limit`: stop after line number `limit` (`None`: no limit).
    pub fn new(out: W, limit: Option<u64>) -> TraceWriter<W> {
        TraceWriter {
            out,
            n: 0,
            started: false,
            limit,
            mem_w: Vec::with_capacity(32),
            io_w: Vec::with_capacity(16),
            line: Vec::with_capacity(96),
            error: None,
        }
    }

    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Writes the header and line 0 (`cpu`'s current state, empty `ir`).
    /// Does nothing if already started.
    pub fn start(&mut self, cpu: &Cpu) {
        if self.started {
            return;
        }
        self.started = true;
        self.mem_w.clear();
        self.io_w.clear();
        self.emit_raw(TRACE_HEADER.as_bytes());
        self.emit_raw(b"\n");
        self.write_line(cpu, cpu.pc, b"");
    }

    /// Number of the last line written (0 after `start`).
    pub fn lines(&self) -> u64 {
        self.n
    }

    /// The limit was reached: further lines are dropped.
    pub fn is_full(&self) -> bool {
        self.limit.is_some_and(|limit| self.n >= limit)
    }

    /// A write failed; the trace stopped there.
    pub fn error(&self) -> Option<&io::Error> {
        self.error.as_ref()
    }

    /// Flushes and returns the writer, or the first write error.
    pub fn finish(mut self) -> io::Result<W> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        self.out.flush()?;
        Ok(self.out)
    }

    #[inline(always)]
    fn recording(&self) -> bool {
        self.started && !self.is_full() && self.error.is_none()
    }

    fn emit_raw(&mut self, bytes: &[u8]) {
        if self.error.is_none()
            && let Err(e) = self.out.write_all(bytes)
        {
            self.error = Some(e);
        }
    }

    fn write_line(&mut self, cpu: &Cpu, pc: u16, ir: &[u8]) {
        let mut line = std::mem::take(&mut self.line);
        line.clear();
        line.extend_from_slice(self.n.to_string().as_bytes());
        line.push(b',');
        hex16(&mut line, pc);
        line.push(b',');
        line.extend_from_slice(ir);
        for &r in &cpu.registers {
            line.push(b',');
            hex8(&mut line, r);
        }
        line.push(b',');
        hex16(&mut line, cpu.sp);
        line.push(b',');
        hex8(&mut line, cpu.flags.0);
        line.push(b',');
        line.extend_from_slice(&self.mem_w);
        line.push(b',');
        line.extend_from_slice(&self.io_w);
        line.push(b'\n');
        self.emit_raw(&line);
        self.line = line;
        self.mem_w.clear();
        self.io_w.clear();
    }
}

impl<W: Write> Probe for TraceWriter<W> {
    fn mem_write(&mut self, address: u16, value: u8) {
        if self.recording() {
            if !self.mem_w.is_empty() {
                self.mem_w.push(b';');
            }
            hex16(&mut self.mem_w, address);
            self.mem_w.push(b'=');
            hex8(&mut self.mem_w, value);
        }
    }

    fn io_write(&mut self, port: u8, value: u8) {
        if self.recording() {
            if !self.io_w.is_empty() {
                self.io_w.push(b';');
            }
            hex8(&mut self.io_w, port);
            self.io_w.push(b'=');
            hex8(&mut self.io_w, value);
        }
    }

    fn instruction(&mut self, cpu: &Cpu, pc: u16, ir: Instruction) {
        if self.recording() {
            let mut bytes = [0u8; 8];
            let size = ir.size() as usize;
            for (k, &byte) in ir.to_bytes()[..size].iter().enumerate() {
                bytes[2 * k] = HEX[(byte >> 4) as usize];
                bytes[2 * k + 1] = HEX[(byte & 0xF) as usize];
            }
            self.n += 1;
            self.write_line(cpu, pc, &bytes[..2 * size]);
        }
    }

    fn interrupt(&mut self, cpu: &Cpu, interrupted: u16) {
        if self.recording() {
            self.n += 1;
            self.write_line(cpu, interrupted, b"INT");
        }
    }
}

const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[inline(always)]
fn hex8(out: &mut Vec<u8>, value: u8) {
    out.push(HEX[(value >> 4) as usize]);
    out.push(HEX[(value & 0xF) as usize]);
}

#[inline(always)]
fn hex16(out: &mut Vec<u8>, value: u16) {
    hex8(out, (value >> 8) as u8);
    hex8(out, value as u8);
}

/// Number of identical lines shown before a difference.
pub const CONTEXT_LINES: usize = 3;

/// The first difference between two traces (see [`compare`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceDiff {
    /// Line number in the files, from 1 (1 = header).
    pub line: usize,
    /// The `n` column of the line (`None` for the header).
    pub n: Option<String>,
    pub kind: DiffKind,
    /// Up to [`CONTEXT_LINES`] lines before it (the same in both traces).
    pub before: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffKind {
    /// Both traces have the line; these columns differ.
    Columns(Vec<ColumnDiff>),
    /// The first trace ends before this line; `other` is the second's.
    FirstEnds { other: String },
    /// The second trace ends before this line; `other` is the first's.
    SecondEnds { other: String },
}

/// A column that differs. `None`: the line has no such column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnDiff {
    pub name: String,
    pub first: Option<String>,
    pub second: Option<String>,
}

/// Compares two traces (file contents) line by line; `None`: identical.
/// Columns are named after the first trace's header (`#k` past its end).
/// `\r\n` line endings are accepted.
pub fn compare(first: &str, second: &str) -> Option<TraceDiff> {
    let first: Vec<&str> = first.lines().collect();
    let second: Vec<&str> = second.lines().collect();
    let names: Vec<&str> = first.first().map_or(Vec::new(), |h| h.split(',').collect());
    let n_of = |k: usize, line: &str| (k > 0).then(|| line.split(',').next().unwrap().to_string());
    for k in 0..first.len().max(second.len()) {
        let (kind, n) = match (first.get(k), second.get(k)) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(a), Some(b)) => (DiffKind::Columns(columns(&names, a, b)), n_of(k, a)),
            (None, Some(b)) => (
                DiffKind::FirstEnds {
                    other: b.to_string(),
                },
                n_of(k, b),
            ),
            (Some(a), None) => (
                DiffKind::SecondEnds {
                    other: a.to_string(),
                },
                n_of(k, a),
            ),
            (None, None) => unreachable!(),
        };
        let before = first[k.saturating_sub(CONTEXT_LINES)..k.min(first.len())]
            .iter()
            .map(|l| l.to_string())
            .collect();
        return Some(TraceDiff {
            line: k + 1,
            n,
            kind,
            before,
        });
    }
    None
}

fn columns(names: &[&str], a: &str, b: &str) -> Vec<ColumnDiff> {
    let a: Vec<&str> = a.split(',').collect();
    let b: Vec<&str> = b.split(',').collect();
    (0..a.len().max(b.len()))
        .filter(|&i| a.get(i) != b.get(i))
        .map(|i| ColumnDiff {
            name: names
                .get(i)
                .map_or_else(|| format!("#{}", i + 1), |s| s.to_string()),
            first: a.get(i).map(|s| s.to_string()),
            second: b.get(i).map(|s| s.to_string()),
        })
        .collect()
}

/// A difference with the names of the two traces, for display.
pub struct DiffReport<'a> {
    pub diff: &'a TraceDiff,
    pub first: &'a str,
    pub second: &'a str,
}

impl fmt::Display for DiffReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let d = self.diff;
        match &d.n {
            Some(n) => writeln!(f, "traces differ at n={n} (line {} of the files)", d.line)?,
            None => writeln!(f, "traces differ in the header (line {})", d.line)?,
        }
        match &d.kind {
            DiffKind::Columns(columns) => {
                let width = columns
                    .iter()
                    .map(|c| c.name.len())
                    .max()
                    .unwrap_or(0)
                    .max(6);
                let show = |v: &Option<String>| match v.as_deref() {
                    Some("") => "(empty)".to_string(),
                    Some(v) => v.to_string(),
                    None => "(missing)".to_string(),
                };
                let rows: Vec<(&str, String, String)> = columns
                    .iter()
                    .map(|c| (c.name.as_str(), show(&c.first), show(&c.second)))
                    .collect();
                let values = rows
                    .iter()
                    .map(|r| r.1.len())
                    .chain([self.first.len()])
                    .max()
                    .unwrap_or(0);
                writeln!(
                    f,
                    "  {:width$}  {:values$}  |  {}",
                    "column", self.first, self.second
                )?;
                for (name, first, second) in rows {
                    writeln!(f, "  {name:width$}  {first:values$}  |  {second}")?;
                }
            }
            DiffKind::FirstEnds { other } => {
                writeln!(
                    f,
                    "  {} ends here; {} goes on with:",
                    self.first, self.second
                )?;
                writeln!(f, "  {other}")?;
            }
            DiffKind::SecondEnds { other } => {
                writeln!(
                    f,
                    "  {} ends here; {} goes on with:",
                    self.second, self.first
                )?;
                writeln!(f, "  {other}")?;
            }
        }
        if !d.before.is_empty() {
            writeln!(f, "previous lines:")?;
            for line in &d.before {
                writeln!(f, "  {line}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACE: &str = "\
n,pc,ir,a,b,c,d,i,j,x,y,sp,flags,mem_w,io_w
0,0000,,00,00,00,00,00,00,00,00,FE00,04,,
1,0000,748003,03,00,00,00,00,00,00,00,FE00,04,,
2,0003,748800,03,00,00,00,00,00,00,00,FE00,04,,
3,0006,74985A,03,00,00,5A,00,00,00,00,FE00,04,,
4,0009,749010,03,00,10,5A,00,00,00,00,FE00,04,,
5,000C,8440,03,00,10,5A,00,00,00,00,FDFF,04,FDFF=03,
";

    fn report(diff: &TraceDiff) -> String {
        DiffReport {
            diff,
            first: "a.csv",
            second: "b.csv",
        }
        .to_string()
    }

    #[test]
    fn identical_traces() {
        assert_eq!(compare(TRACE, TRACE), None);
        assert_eq!(compare(TRACE, &TRACE.replace('\n', "\r\n")), None);
        assert_eq!(compare("", ""), None);
    }

    #[test]
    fn one_register_differs() {
        let other = TRACE.replace("4,0009,749010,03,00,10,", "4,0009,749010,03,00,11,");
        let diff = compare(TRACE, &other).unwrap();
        assert_eq!(diff.line, 6);
        assert_eq!(diff.n.as_deref(), Some("4"));
        assert_eq!(
            diff.kind,
            DiffKind::Columns(vec![ColumnDiff {
                name: "c".into(),
                first: Some("10".into()),
                second: Some("11".into()),
            }])
        );
        let lines: Vec<&str> = TRACE.lines().collect();
        assert_eq!(diff.before, &lines[2..5]);

        let text = report(&diff);
        assert!(
            text.starts_with("traces differ at n=4 (line 6 of the files)\n"),
            "{text}"
        );
        let table = "\n  column  a.csv  |  b.csv\n  c       10     |  11\n";
        assert!(text.contains(table), "{text}");
        assert!(text.contains("previous lines:\n  1,0000,"), "{text}");
    }

    #[test]
    fn one_trace_shorter() {
        let lines: Vec<&str> = TRACE.lines().collect();
        let short = lines[..5].join("\n") + "\n";

        let diff = compare(&short, TRACE).unwrap();
        assert_eq!(diff.line, 6);
        assert_eq!(diff.n.as_deref(), Some("4"));
        assert_eq!(
            diff.kind,
            DiffKind::FirstEnds {
                other: lines[5].into()
            }
        );
        assert_eq!(diff.before, &lines[2..5]);
        assert!(report(&diff).contains("a.csv ends here; b.csv goes on with:"));

        let diff = compare(TRACE, &short).unwrap();
        assert_eq!(
            diff.kind,
            DiffKind::SecondEnds {
                other: lines[5].into()
            }
        );
        assert!(report(&diff).contains("b.csv ends here; a.csv goes on with:"));
    }

    #[test]
    fn missing_column_and_header() {
        let other = TRACE.replace("FDFF=03,", "FDFF=03");
        let diff = compare(TRACE, &other).unwrap();
        assert_eq!(
            diff.kind,
            DiffKind::Columns(vec![ColumnDiff {
                name: "io_w".into(),
                first: Some("".into()),
                second: None,
            }])
        );
        assert!(report(&diff).contains("\n  io_w    (empty)  |  (missing)\n"));

        let diff = compare(TRACE, &TRACE.replacen("flags", "f", 1)).unwrap();
        assert_eq!((diff.line, diff.n.clone()), (1, None));
        assert!(diff.before.is_empty());
    }
}
