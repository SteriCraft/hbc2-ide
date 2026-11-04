//! `hbc2 vectors`: Logisim test vectors for the CPU circuits.
//!
//! The expected outputs come from the emulator's own code, so a circuit that
//! passes behaves exactly like the emulator:
//! - `hbc2_core::alu` for ADD/ADC/SUB/SBB (functions the CPU calls);
//! - for the other ALU instructions, whose code is inline in `cpu.rs`, the
//!   vectors run the real CPU on that instruction ([`exec`]) and read the
//!   result register and the flags back. The emulator is not changed.
//!
//! A flag the instruction does not update (per `Opcode::info().flags`, and
//! checked against the CPU) is written `x`: the control unit won't store it.
//!
//! Most modules are tested on every input combination (up to 2^21). A
//! module that has too many inputs for that gives its own rows instead.

use std::cell::RefCell;
use std::io::{BufWriter, Write};

use hbc2_core::alu;
use hbc2_core::isa::{AddrMode, Flag, Opcode};
use hbc2_core::{Board, Cpu, Flags};

/// A combinational circuit: its pins (name, width) and its behaviour.
struct Module {
    name: &'static str,
    inputs: &'static [(&'static str, u32)],
    outputs: &'static [(&'static str, u32)],
    /// Input values, in `inputs` order -> output values, in `outputs` order
    /// (None = don't care).
    eval: fn(&[u32]) -> Vec<Option<u32>>,
    /// Input rows to test, when not every combination (None = exhaustive).
    rows: Option<fn() -> Vec<Vec<u32>>>,
}

const MAX_BITS: u32 = 21;

// --------------------------------------------------------------------------
// Running one instruction on the emulator's CPU
// --------------------------------------------------------------------------

/// What the CPU did on one instruction: the destination register and the
/// flags, None for a flag the instruction leaves alone.
#[derive(Clone, Copy, Debug)]
struct Run {
    r: u8,
    c: Option<bool>,
    z: Option<bool>,
    n: Option<bool>,
    s: Option<bool>,
    e: Option<bool>,
    f: Option<bool>,
}

thread_local! {
    static MACHINE: RefCell<(Cpu, Board)> = RefCell::new((Cpu::new(), Board::default()));
}

/// Execute `op` in Reg mode on the CPU: R1 = A (register 0) holds `a`, R2 =
/// B (register 1) holds `b` (binary ops only), the carry flag holds `carry`.
fn exec(op: Opcode, a: u8, b: u8, carry: bool) -> Run {
    let unary = matches!(op, Opcode::Inc | Opcode::Dec | Opcode::Not | Opcode::Shl | Opcode::Asr | Opcode::Shr);
    let word = (op as u32) << 26 | (AddrMode::Reg as u32) << 22 | if unary { 0 } else { 1 << 16 };
    let updated = op.info().flags;
    MACHINE.with(|m| {
        let (cpu, board) = &mut *m.borrow_mut();
        // Two runs: every other flag cleared, then set (but H and I), to
        // check the CPU leaves alone the flags the table says it does.
        let mut out = None;
        for others in [false, true] {
            for (i, byte) in word.to_be_bytes().iter().enumerate() {
                board.ram.write(i as u16, *byte);
            }
            cpu.reset();
            cpu.registers[0] = a;
            cpu.registers[1] = b;
            let mut flags = Flags(0);
            for fl in Flag::ALL {
                if !matches!(fl, Flag::Halt | Flag::Interrupt) {
                    flags.set(fl, others);
                }
            }
            flags.set(Flag::Carry, carry);
            cpu.flags = flags;
            assert!(cpu.tick(board) && !cpu.is_halted(), "{op:?} did not execute");
            for fl in Flag::ALL {
                if !updated.contains(&fl) {
                    assert_eq!(cpu.flags.get(fl), flags.get(fl), "{op:?} changed {fl:?}, not in its flag list");
                }
            }
            let get = |fl: Flag| updated.contains(&fl).then(|| cpu.flags.get(fl));
            let run = Run {
                r: cpu.registers[0],
                c: get(Flag::Carry),
                z: get(Flag::Zero),
                n: get(Flag::Negative),
                s: get(Flag::Superior),
                e: get(Flag::Equal),
                f: get(Flag::Inferior),
            };
            if let Some(first) = out {
                let first: Run = first;
                assert!(
                    first.r == run.r && first.c == run.c && first.z == run.z && first.n == run.n,
                    "{op:?} depends on unrelated flags"
                );
            }
            out = Some(run);
        }
        out.unwrap()
    })
}

fn bit(b: bool) -> Option<u32> {
    Some(b as u32)
}

fn opt(b: Option<bool>) -> Option<u32> {
    b.map(|b| b as u32)
}

// --------------------------------------------------------------------------
// alu_op: the operation code of the ALU (see logisim/notes/alu-report.md)
// --------------------------------------------------------------------------

/// What the ALU computes for one 4-bit `alu_op`.
///
/// op3 = 0: the adder (add_sub8). op0 = sub, op1 = use_c; op2 is ignored,
/// so 01xx are aliases of 00xx. INC and DEC are ADD and SUB with b = 01,
/// which the control unit puts on B (checked against the emulator here).
/// op3 = 1: op2..op0 pick logic or shift through the 74HCT138.
fn alu_reference(op: u32, a: u8, b: u8, cin: bool) -> Run {
    use Opcode as O;
    let op = if op & 0b1000 == 0 { op & 0b0011 } else { op };
    let instr = match op {
        0b0000 => O::Add,
        0b0001 => O::Sub, // and CMP: same ALU operation, other flags kept
        0b0010 => O::Adc,
        0b0011 => O::Sbb,
        0b1000 => O::And,
        0b1001 => O::Or,
        0b1010 => O::Xor,
        0b1011 => O::Not,
        0b1100 => O::Shl,
        0b1101 => O::Asr, // alias of 1111
        0b1110 => O::Shr,
        _ => O::Asr,
    };
    let mut run = exec(instr, a, b, cin);
    if op == 0b0001 {
        let cmp = exec(O::Cmp, a, b, cin);
        (run.s, run.e, run.f) = (cmp.s, cmp.e, cmp.f);
    }
    if op <= 0b0001 && b == 1 {
        // INC / DEC on the emulator = this row (ADD / SUB with b = 01)
        let id = exec(if op == 0 { O::Inc } else { O::Dec }, a, 0, cin);
        assert_eq!((id.r, id.c, id.z, id.n), (run.r, run.c, run.z, run.n), "INC/DEC != ADD/SUB 01, a = {a:02X}");
    }
    run
}

/// Small deterministic generator for the sampled modules (xorshift).
struct Rng(u64);

impl Rng {
    fn byte(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 24) as u32 & 0xFF
    }
}

// --------------------------------------------------------------------------
// Modules
// --------------------------------------------------------------------------

const MODULES: &[Module] = &[
    Module {
        name: "add_sub8",
        inputs: &[
            ("add_sub8_sub", 1),
            ("add_sub8_use_c", 1),
            ("add_sub8_cin", 1),
            ("add_sub8_a", 8),
            ("add_sub8_b", 8),
        ],
        outputs: &[("add_sub8_s", 8), ("add_sub8_c", 1)],
        // ADD/ADC when sub = 0, SUB/SBB when sub = 1; use_c selects ADC/SBB,
        // which use the carry flag (cin). After SUB/SBB, c is the borrow.
        eval: |v| {
            let (sub, use_c, cin) = (v[0] != 0, v[1] != 0, v[2] != 0);
            let (a, b) = (v[3] as u8, v[4] as u8);
            let (s, c) = if sub { alu::sub(a, b, use_c && cin) } else { alu::add(a, b, use_c && cin) };
            vec![Some(s as u32), bit(c)]
        },
        rows: None,
    },
    Module {
        name: "logic8",
        inputs: &[("logic8_a", 8), ("logic8_b", 8)],
        outputs: &[("logic8_and", 8), ("logic8_or", 8), ("logic8_xor", 8)],
        eval: |v| {
            let (a, b) = (v[0] as u8, v[1] as u8);
            [Opcode::And, Opcode::Or, Opcode::Xor]
                .iter()
                .map(|&op| Some(exec(op, a, b, false).r as u32))
                .collect()
        },
        rows: None,
    },
    Module {
        name: "shift8",
        inputs: &[("shift8_sel", 1), ("shift8_asr", 1), ("shift8_c_add", 1), ("shift8_a", 8)],
        outputs: &[("shift8_shl", 8), ("shift8_sr", 8), ("shift8_c", 1)],
        // shl = SHL a; sr = ASR a when sel and asr, else SHR a.
        // c = c_add when sel = 0, else the carry of SHL (asr = 0) or ASR (asr = 1).
        eval: |v| {
            let (sel, asr, c_add, a) = (v[0] != 0, v[1] != 0, v[2] != 0, v[3] as u8);
            let shl = exec(Opcode::Shl, a, 0, false);
            let sr = exec(if sel && asr { Opcode::Asr } else { Opcode::Shr }, a, 0, false);
            let c = match (sel, asr) {
                (false, _) => Some(c_add),
                (true, false) => shl.c,
                (true, true) => exec(Opcode::Asr, a, 0, false).c,
            };
            vec![Some(shl.r as u32), Some(sr.r as u32), opt(c)]
        },
        rows: None,
    },
    Module {
        name: "flags8",
        inputs: &[("flags8_r", 8)],
        outputs: &[("flags8_z", 1), ("flags8_n", 1)],
        // Z and N of a result, as the CPU sets them (ADD r, 0).
        eval: |v| {
            let run = exec(Opcode::Add, v[0] as u8, 0, false);
            vec![opt(run.z), opt(run.n)]
        },
        rows: None,
    },
    Module {
        name: "cmp8",
        inputs: &[("cmp8_a", 8), ("cmp8_b", 8)],
        outputs: &[("cmp8_s", 1), ("cmp8_e", 1), ("cmp8_f", 1)],
        // CMP a, b on the CPU: S = a > b, E = a = b, F = a < b (unsigned).
        eval: |v| {
            let run = exec(Opcode::Cmp, v[0] as u8, v[1] as u8, false);
            vec![opt(run.s), opt(run.e), opt(run.f)]
        },
        rows: None,
    },
    Module {
        name: "rmux8",
        inputs: &[
            ("rmux8_op", 4),
            ("rmux8_sum", 8),
            ("rmux8_and", 8),
            ("rmux8_or", 8),
            ("rmux8_xor", 8),
            ("rmux8_a", 8),
            ("rmux8_shl", 8),
            ("rmux8_sr", 8),
        ],
        outputs: &[("rmux8_r", 8)],
        // The result multiplexer: which source alu_op puts on alu_r. The
        // sources are independent random bytes, so a wrong pick shows.
        eval: |v| {
            let r = match v[0] {
                0..=7 => v[1],
                8 => v[2],
                9 => v[3],
                10 => v[4],
                11 => !v[5] & 0xFF,
                12 => v[6],
                _ => v[7],
            };
            vec![Some(r)]
        },
        rows: Some(|| {
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
            let mut rows = Vec::new();
            for op in 0..16 {
                // every source 00 or FF, the others the opposite
                for k in 1..8 {
                    for hi in [0, 0xFF] {
                        let mut row = vec![op; 8];
                        for (i, cell) in row.iter_mut().enumerate().skip(1) {
                            *cell = if i == k { hi } else { hi ^ 0xFF };
                        }
                        rows.push(row);
                    }
                }
                for _ in 0..4096 {
                    let mut row = vec![op];
                    row.extend((1..8).map(|_| rng.byte()));
                    rows.push(row);
                }
            }
            rows
        }),
    },
    Module {
        name: "alu",
        inputs: &[("alu_op", 4), ("alu_cin", 1), ("alu_a", 8), ("alu_b", 8)],
        outputs: &[
            ("alu_r", 8),
            ("alu_c", 1),
            ("alu_z", 1),
            ("alu_n", 1),
            ("alu_s", 1),
            ("alu_e", 1),
            ("alu_f", 1),
        ],
        // The whole ALU: see alu_reference for what each alu_op does. A flag
        // the instruction doesn't write is don't care.
        eval: |v| {
            let run = alu_reference(v[0], v[2] as u8, v[3] as u8, v[1] != 0);
            vec![Some(run.r as u32), opt(run.c), opt(run.z), opt(run.n), opt(run.s), opt(run.e), opt(run.f)]
        },
        rows: None,
    },
];

fn names() -> String {
    MODULES.iter().map(|m| m.name).collect::<Vec<_>>().join(", ")
}

/// `hbc2 vectors <module> [-o file]`, or `hbc2 vectors --list`.
pub fn vectors(args: &[String]) -> Result<(), String> {
    let Some(name) = args.first() else {
        return Err(format!("usage: hbc2 vectors <module> [-o file]\nmodules: {}", names()));
    };
    if name == "--list" {
        for m in MODULES {
            println!("{}", m.name);
        }
        return Ok(());
    }
    let module = MODULES
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("no module {name:?}; modules: {}", names()))?;
    let out = crate::option(args, "-o");
    match out {
        Some(path) => {
            let file = std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?;
            write(module, &mut BufWriter::new(file))
        }
        None => write(module, &mut BufWriter::new(std::io::stdout().lock())),
    }
    .map_err(|e| format!("writing vectors: {e}"))
}

fn write(m: &Module, w: &mut impl Write) -> std::io::Result<()> {
    let header: Vec<String> = m
        .inputs
        .iter()
        .chain(m.outputs)
        .map(|&(n, width)| if width > 1 { format!("{n}[{width}]") } else { n.to_string() })
        .collect();
    writeln!(w, "{}", header.join(" "))?;

    let line = |values: &[u32], w: &mut dyn Write| -> std::io::Result<()> {
        let outputs = (m.eval)(values);
        let mut cells: Vec<String> = m
            .inputs
            .iter()
            .zip(values)
            .map(|(&(_, width), &v)| format!("{:0width$b}", v, width = width as usize))
            .collect();
        cells.extend(m.outputs.iter().zip(&outputs).map(|(&(_, width), v)| match v {
            Some(v) => format!("{:0width$b}", v, width = width as usize),
            None => "x".repeat(width as usize),
        }));
        writeln!(w, "{}", cells.join(" "))
    };

    if let Some(rows) = m.rows {
        for row in rows() {
            line(&row, w)?;
        }
        return w.flush();
    }
    let bits: u32 = m.inputs.iter().map(|&(_, width)| width).sum();
    if bits > MAX_BITS {
        return Err(std::io::Error::other(format!(
            "{}: {bits} input bits, more than the {MAX_BITS} an exhaustive test allows",
            m.name
        )));
    }
    let mut values = vec![0u32; m.inputs.len()];
    for row in 0u32..(1 << bits) {
        // first input = most significant bits of the row counter
        let mut rest = row;
        for (i, &(_, width)) in m.inputs.iter().enumerate().rev() {
            values[i] = rest & ((1 << width) - 1);
            rest >>= width;
        }
        line(&values, w)?;
    }
    w.flush()
}
