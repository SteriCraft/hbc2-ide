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
//!
//! Circuits with state (the flag register) are written as sequences, with
//! Logisim's `<set>` and `<seq>` columns: `flags` drives every control from
//! every state against a model built from the emulator's flag lists,
//! `flags_cpu` replays every flag change of the real CPU.

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

// --------------------------------------------------------------------------
// Sequential modules: Logisim's <set> / <seq> columns
// --------------------------------------------------------------------------

/// Expected value of one output in a sequential row.
#[derive(Clone, Copy)]
enum Out {
    /// Don't care. Never on a bus output (SeqModule::buses).
    X,
    /// Not known to the model (a register never written since power-up).
    /// Written as don't care, like X, but allowed on a bus: something drives
    /// it, the model just doesn't know the value.
    U,
    V(u32),
    /// High impedance (tri-state output off).
    Z,
}

type Row = (Vec<u32>, Vec<Out>);

/// A circuit with state: sequences of rows, each sequence run in order
/// without resetting the circuit in between.
struct SeqModule {
    name: &'static str,
    inputs: &'static [(&'static str, u32)],
    outputs: &'static [(&'static str, u32)],
    /// Outputs that are (or show) a tri-state bus. Logisim shows two drivers
    /// that disagree as E, which fails any expected value or <FLOAT>, but
    /// passes a don't care. So a bus output is always checked: a value, Z
    /// (nothing drives it) or U (driven, value unknown); X is refused.
    buses: &'static [&'static str],
    sets: fn() -> Vec<Vec<Row>>,
}

fn write_seq(m: &SeqModule, w: &mut impl Write) -> std::io::Result<()> {
    let mut header = vec!["<set>".to_string(), "<seq>".to_string()];
    header.extend(
        m.inputs
            .iter()
            .chain(m.outputs)
            .map(|&(n, width)| if width > 1 { format!("{n}[{width}]") } else { n.to_string() }),
    );
    writeln!(w, "{}", header.join(" "))?;
    for b in m.buses {
        assert!(m.outputs.iter().any(|&(n, _)| n == *b), "{}: bus {b} is not an output", m.name);
    }
    let is_bus: Vec<bool> = m.outputs.iter().map(|&(n, _)| m.buses.contains(&n)).collect();
    let mut unknown = 0;
    for (k, set) in (m.sets)().iter().enumerate() {
        for (j, (ins, outs)) in set.iter().enumerate() {
            assert_eq!((ins.len(), outs.len()), (m.inputs.len(), m.outputs.len()), "{}: bad row", m.name);
            for ((&(n, _), v), &bus) in m.outputs.iter().zip(outs).zip(&is_bus) {
                match v {
                    Out::X if bus => panic!("{}: set {} row {}: bus {n} is don't care", m.name, k + 1, j + 1),
                    Out::U if bus => unknown += 1,
                    _ => {}
                }
            }
            let mut cells = vec![(k + 1).to_string(), (j + 1).to_string()];
            cells.extend(m.inputs.iter().zip(ins).map(|(&(_, wd), &v)| format!("{:0wd$b}", v, wd = wd as usize)));
            cells.extend(m.outputs.iter().zip(outs).map(|(&(_, wd), v)| match v {
                Out::V(v) => format!("{:0wd$b}", v, wd = wd as usize),
                Out::X | Out::U => "x".repeat(wd as usize),
                Out::Z => "<FLOAT>".to_string(),
            }));
            writeln!(w, "{}", cells.join(" "))?;
        }
    }
    if unknown > 0 {
        eprintln!("{}: {unknown} bus values unknown to the model (power-up): contention not checked there", m.name);
    }
    w.flush()
}

// --------------------------------------------------------------------------
// The flag register (logisim/notes/flags-decisions.md)
// --------------------------------------------------------------------------

/// flags_op: what the flag register stores on the next clock edge.
///   0000 hold      0001 C Z N <- ALU   0010 Z N <- ALU   0011 S E F <- ALU
///   0100 all 8 <- data bus (POF)       0101..0111 hold
///   1fff flag fff (bit number, C0 E1 I2 N3 S4 Z5 F6 H7) <- flags_bit_val
const FLAGS_IN: &[(&str, u32)] = &[
    ("clk", 1),
    ("rst_n", 1),
    ("flags_op", 4),
    ("flags_bit_val", 1),
    ("flags_alu_c", 1),
    ("flags_alu_z", 1),
    ("flags_alu_n", 1),
    ("flags_alu_s", 1),
    ("flags_alu_e", 1),
    ("flags_alu_f", 1),
    ("flags_bus_in", 8),
    ("flags_oe_n", 1),
];
const FLAGS_BUSES: &[&str] = &["flags_bus_out"];
const FLAGS_OUT: &[(&str, u32)] = &[
    ("flag_c", 1),
    ("flag_e", 1),
    ("flag_i", 1),
    ("flag_n", 1),
    ("flag_s", 1),
    ("flag_z", 1),
    ("flag_f", 1),
    ("flag_h", 1),
    ("flags_bus_out", 8),
];
const OP_HOLD: u32 = 0b0000;
const OP_CZN: u32 = 0b0001;
const OP_ZN: u32 = 0b0010;
const OP_SEF: u32 = 0b0011;
const OP_BUS: u32 = 0b0100;
const fn op_bit(flag: Flag) -> u32 {
    0b1000 | flag as u32
}

/// The ALU flag outputs that reach the flag register.
#[derive(Clone, Copy, Default)]
struct AluFlags {
    c: bool,
    z: bool,
    n: bool,
    s: bool,
    e: bool,
    f: bool,
}

impl AluFlags {
    fn get(self, flag: Flag) -> bool {
        match flag {
            Flag::Carry => self.c,
            Flag::Zero => self.z,
            Flag::Negative => self.n,
            Flag::Superior => self.s,
            Flag::Equal => self.e,
            Flag::Inferior => self.f,
            _ => panic!("the ALU has no {flag:?} output"),
        }
    }
    fn from_bits(b: u32) -> AluFlags {
        AluFlags { c: b & 1 != 0, z: b & 2 != 0, n: b & 4 != 0, s: b & 8 != 0, e: b & 16 != 0, f: b & 32 != 0 }
    }
}

/// The flags each ALU load writes, from the emulator's instruction table:
/// every instruction of a group must list exactly these flags.
fn flag_group(op: u32) -> &'static [Flag] {
    use Opcode as O;
    let (first, members): (Opcode, &[Opcode]) = match op {
        OP_CZN => (O::Add, &[O::Adc, O::Sub, O::Sbb, O::Inc, O::Dec, O::Shl, O::Asr]),
        OP_ZN => (O::And, &[O::Or, O::Xor, O::Not, O::Shr]),
        OP_SEF => (O::Cmp, &[]),
        OP_BUS => (O::Pof, &[]),
        _ => unreachable!(),
    };
    let set = first.info().flags;
    for m in members {
        let other = m.info().flags;
        assert!(
            other.len() == set.len() && other.iter().all(|f| set.contains(f)),
            "{m:?} writes {other:?}, not the {first:?} group {set:?}"
        );
    }
    set
}

/// The flag register's next state, built from the emulator's flag lists.
fn flags_model(state: u8, op: u32, val: bool, alu: AluFlags, bus: u8) -> u8 {
    let mut f = Flags(state);
    match op {
        OP_CZN | OP_ZN | OP_SEF => {
            for &fl in flag_group(op) {
                f.set(fl, alu.get(fl));
            }
        }
        OP_BUS => {
            assert_eq!(flag_group(op).len(), 8, "POF restores every flag");
            f = Flags(bus);
        }
        0b1000..=0b1111 => {
            let fl = Flag::ALL[(op & 7) as usize];
            assert_eq!(fl as u32, op & 7, "Flag::ALL is in bit order");
            f.set(fl, val);
        }
        _ => {}
    }
    f.0
}

/// Inputs of one flag register row.
#[derive(Clone, Copy)]
struct FlagsIn {
    clk: bool,
    rst_n: bool,
    op: u32,
    val: bool,
    alu: AluFlags,
    bus: u8,
    oe_n: bool,
}

impl FlagsIn {
    fn idle() -> FlagsIn {
        FlagsIn { clk: false, rst_n: true, op: OP_HOLD, val: false, alu: AluFlags::default(), bus: 0, oe_n: true }
    }
    fn row(self, flags: Option<u8>, bus_out: Out) -> Row {
        let a = self.alu;
        let ins = vec![
            self.clk as u32,
            self.rst_n as u32,
            self.op,
            self.val as u32,
            a.c as u32,
            a.z as u32,
            a.n as u32,
            a.s as u32,
            a.e as u32,
            a.f as u32,
            self.bus as u32,
            self.oe_n as u32,
        ];
        let mut outs: Vec<Out> = (0..8).map(|b| flags.map_or(Out::X, |f| Out::V((f >> b) as u32 & 1))).collect();
        outs.push(bus_out);
        (ins, outs)
    }
}

/// One clocked step: inputs with clk low (flags still `before`), then clk
/// high (flags = `after`). The bus output shows the flags when oe_n is low.
fn step(rows: &mut Vec<Row>, i: FlagsIn, before: u8, after: u8) {
    let out = |f: u8| if i.oe_n { Out::Z } else { Out::V(f as u32) };
    let low = if i.rst_n { before } else { after }; // reset doesn't wait for the clock
    rows.push(FlagsIn { clk: false, ..i }.row(Some(low), out(low)));
    rows.push(FlagsIn { clk: true, ..i }.row(Some(after), out(after)));
}

/// Start a sequence in `state`, loaded from the bus (the first row's flags
/// are unknown).
fn load(state: u8) -> Vec<Row> {
    let i = FlagsIn { op: OP_BUS, bus: state, ..FlagsIn::idle() };
    vec![i.row(None, Out::Z), FlagsIn { clk: true, ..i }.row(Some(state), Out::Z)]
}

/// Every flags_op and bit value from every state of the 8 flags, with the
/// data inputs the opposite of the state (so a wrong load shows), then
/// random; and a reset from every state.
fn flags_sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut sets = Vec::new();
    for state in 0..=255u8 {
        for op in 0..16 {
            for val in [false, true] {
                for random in [false, true] {
                    let (alu, bus) = if random {
                        (AluFlags::from_bits(rng.byte()), rng.byte() as u8)
                    } else {
                        let inv = !state;
                        let bit = |fl: Flag| inv >> fl as u8 & 1 != 0;
                        let alu = AluFlags {
                            c: bit(Flag::Carry),
                            z: bit(Flag::Zero),
                            n: bit(Flag::Negative),
                            s: bit(Flag::Superior),
                            e: bit(Flag::Equal),
                            f: bit(Flag::Inferior),
                        };
                        (alu, inv)
                    };
                    let mut rows = load(state);
                    let next = flags_model(state, op, val, alu, bus);
                    let i = FlagsIn { op, val, alu, bus, oe_n: random, ..FlagsIn::idle() };
                    step(&mut rows, i, state, next);
                    sets.push(rows);
                }
            }
        }
        // reset: I = 1, the rest 0, at once (asynchronous), whatever the clock
        let mut rows = load(state);
        let reset = FlagsIn { rst_n: false, op: OP_BUS, bus: state, ..FlagsIn::idle() };
        rows.push(reset.row(Some(0x04), Out::Z));
        rows.push(FlagsIn { clk: true, ..reset }.row(Some(0x04), Out::Z));
        rows.push(FlagsIn { rst_n: true, ..FlagsIn::idle() }.row(Some(0x04), Out::Z));
        sets.push(rows);
    }
    sets
}

/// After `cpu.reset()`: runs one NOP (four zero bytes at the reset PC, so
/// the mode bits are 0 too), then puts the PC back. `Cpu::reset` delays
/// interrupts until the first instruction has run, as the hardware's first
/// fetch after the boot has no interrupt check; the generators that test
/// interrupt entries start from a CPU past that instruction. The caller
/// writes its RAM and state afterwards.
pub(crate) fn past_first_instruction(cpu: &mut Cpu, board: &mut Board) {
    let pc = cpu.pc;
    for k in 0..4 {
        board.ram.write(pc.wrapping_add(k), 0);
    }
    assert!(cpu.tick(board) && !cpu.interrupt_delayed(), "NOP after reset");
    cpu.pc = pc;
}

/// Run the CPU from flags `flags`: `program` at address 0, `prep` sets up the
/// rest, then `ticks` clock cycles. Returns the CPU flags and the byte at
/// SP (for PSF).
fn run_cpu(flags: u8, program: u32, prep: impl Fn(&mut Cpu, &mut Board), ticks: usize) -> (u8, u8) {
    MACHINE.with(|m| {
        let (cpu, board) = &mut *m.borrow_mut();
        board.reset();
        cpu.reset();
        past_first_instruction(cpu, board);
        for (i, byte) in program.to_be_bytes().iter().enumerate() {
            board.ram.write(i as u16, *byte);
        }
        cpu.flags = Flags(flags);
        prep(cpu, board);
        for _ in 0..ticks {
            cpu.tick(board);
        }
        (cpu.flags.0, board.ram.read(cpu.sp))
    })
}

fn word(op: Opcode, mode: AddrMode) -> u32 {
    (op as u32) << 26 | (mode as u32) << 22
}

/// Every way the emulator changes the flags, from every state it can run
/// in: the real CPU gives the flags after; the rows drive the flag register
/// the way the control unit will (flags-decisions.md) and check it ends
/// with the same flags.
fn flags_cpu_sets() -> Vec<Vec<Row>> {
    use Opcode as O;
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut sets = Vec::new();
    let running = (0..=255u8).filter(|s| s & 0x80 == 0); // H = 0: the CPU executes
    // run the steps from `state`; the last one must give `cpu_after`
    let mut seq = |state: u8, steps: &[FlagsIn], cpu_after: u8, what: &str| {
        let mut rows = load(state);
        let mut f = state;
        for &i in steps {
            let next = if i.rst_n { flags_model(f, i.op, i.val, i.alu, i.bus) } else { 0x04 };
            step(&mut rows, i, f, next);
            f = next;
        }
        assert_eq!(f, cpu_after, "{what} from {state:08b}: register {f:08b}, CPU {cpu_after:08b}");
        sets.push(rows);
    };

    // ALU instructions (Reg mode): alu_op, b forced to 01 (INC/DEC), flags_op
    let alu_instrs: &[(Opcode, u32, bool, u32)] = &[
        (O::Add, 0b0000, false, OP_CZN),
        (O::Adc, 0b0010, false, OP_CZN),
        (O::Sub, 0b0001, false, OP_CZN),
        (O::Sbb, 0b0011, false, OP_CZN),
        (O::Inc, 0b0000, true, OP_CZN),
        (O::Dec, 0b0001, true, OP_CZN),
        (O::Shl, 0b1100, false, OP_CZN),
        (O::Asr, 0b1111, false, OP_CZN),
        (O::And, 0b1000, false, OP_ZN),
        (O::Or, 0b1001, false, OP_ZN),
        (O::Xor, 0b1010, false, OP_ZN),
        (O::Not, 0b1011, false, OP_ZN),
        (O::Shr, 0b1110, false, OP_ZN),
        (O::Cmp, 0b0001, false, OP_SEF),
    ];
    for state in running.clone() {
        for &(op, alu_op, one, flags_op) in alu_instrs {
            let unary = matches!(op, O::Inc | O::Dec | O::Not | O::Shl | O::Asr | O::Shr);
            for k in 0..8 {
                let (a, b) = match k {
                    0 => (0x00, 0x00),
                    1 => (0xFF, 0x01),
                    2 => (0x80, 0x80),
                    3 => (0x7F, 0xFF),
                    _ => (rng.byte() as u8, rng.byte() as u8),
                };
                let program = word(op, AddrMode::Reg) | if unary { 0 } else { 1 << 16 };
                let (after, _) = run_cpu(state, program, |cpu, _| cpu.registers[..2].copy_from_slice(&[a, b]), 1);
                // what the ALU outputs for this alu_op; a flag the ALU leaves
                // undefined gets the opposite of the stored one
                let run = alu_reference(alu_op, a, if one { 1 } else { b }, state & 1 != 0);
                let or_not = |v: Option<bool>, fl: Flag| v.unwrap_or(state >> fl as u8 & 1 == 0);
                let alu = AluFlags {
                    c: or_not(run.c, Flag::Carry),
                    z: or_not(run.z, Flag::Zero),
                    n: or_not(run.n, Flag::Negative),
                    s: or_not(run.s, Flag::Superior),
                    e: or_not(run.e, Flag::Equal),
                    f: or_not(run.f, Flag::Inferior),
                };
                let i = FlagsIn { op: flags_op, alu, bus: !state, ..FlagsIn::idle() };
                seq(state, &[i], after, &format!("{op:?} {a:02X} {b:02X}"));
            }
        }
        // POF: the popped byte, every flag
        for _ in 0..4 {
            let byte = rng.byte() as u8;
            let (after, _) = run_cpu(state, word(O::Pof, AddrMode::None), |cpu, b| b.ram.write(cpu.sp, byte), 1);
            seq(state, &[FlagsIn { op: OP_BUS, bus: byte, ..FlagsIn::idle() }], after, "POF");
        }
        // PSF: the flags onto the data bus, nothing changes
        let (after, pushed) = run_cpu(state, word(O::Psf, AddrMode::None), |_, _| {}, 1);
        assert_eq!(pushed, state, "PSF pushes the flags");
        seq(state, &[FlagsIn { oe_n: false, ..FlagsIn::idle() }], after, "PSF");
        // one flag at a time
        let single: &[(Opcode, Flag, bool)] = &[
            (O::Clc, Flag::Carry, false),
            (O::Cle, Flag::Equal, false),
            (O::Cli, Flag::Interrupt, false),
            (O::Cln, Flag::Negative, false),
            (O::Cls, Flag::Superior, false),
            (O::Clz, Flag::Zero, false),
            (O::Clf, Flag::Inferior, false),
            (O::Stc, Flag::Carry, true),
            (O::Ste, Flag::Equal, true),
            (O::Sti, Flag::Interrupt, true),
            (O::Stn, Flag::Negative, true),
            (O::Sts, Flag::Superior, true),
            (O::Stz, Flag::Zero, true),
            (O::Stf, Flag::Inferior, true),
            (O::Hlt, Flag::Halt, true),
            (O::Irt, Flag::Interrupt, true), // IRT: I = 1 (the popped byte goes to register I)
        ];
        for &(op, fl, val) in single {
            let (after, _) = run_cpu(state, word(op, AddrMode::None), |cpu, _| cpu.sp = 0xFD00, 1);
            seq(state, &[FlagsIn { op: op_bit(fl), val, ..FlagsIn::idle() }], after, &format!("{op:?}"));
        }
        // invalid instruction (CLC with mode bits 01xx): H = 1, I = 0, in two steps
        let (after, _) = run_cpu(state, 0b0100 << 22 | (O::Clc as u32) << 26, |_, _| {}, 1);
        let steps = [
            FlagsIn { op: op_bit(Flag::Halt), val: true, ..FlagsIn::idle() },
            FlagsIn { op: op_bit(Flag::Interrupt), val: false, ..FlagsIn::idle() },
        ];
        seq(state, &steps, after, "invalid instruction");
    }
    // interrupt entry, from every state that accepts one (I = 1, halted or
    // not): I = 0, H = 0, in two steps
    for state in (0..=255u8).filter(|s| s & 0x04 != 0) {
        let (after, _) = run_cpu(
            state,
            word(O::Nop, AddrMode::None),
            |cpu, b| {
                b.int = true;
                cpu.tick(b); // INT seen: INR, acknowledge
                b.int = false; // the IOD lowers INT: the CPU enters the handler on the next tick
            },
            1,
        );
        let steps = [
            FlagsIn { op: op_bit(Flag::Interrupt), val: false, ..FlagsIn::idle() },
            FlagsIn { op: op_bit(Flag::Halt), val: false, ..FlagsIn::idle() },
        ];
        seq(state, &steps, after, "interrupt entry");
    }
    // reset, from every state
    for state in 0..=255u8 {
        let after = MACHINE.with(|m| {
            let (cpu, _) = &mut *m.borrow_mut();
            cpu.flags = Flags(state);
            cpu.reset();
            cpu.flags.0
        });
        seq(state, &[FlagsIn { rst_n: false, ..FlagsIn::idle() }], after, "reset");
    }
    sets
}

const SEQ_MODULES: &[SeqModule] = &[
    SeqModule { name: "flags", inputs: FLAGS_IN, outputs: FLAGS_OUT, buses: FLAGS_BUSES, sets: flags_sets },
    SeqModule { name: "flags_cpu", inputs: FLAGS_IN, outputs: FLAGS_OUT, buses: FLAGS_BUSES, sets: flags_cpu_sets },
];

mod addr;
mod cu;
mod ir;
mod mem;
mod mem_cpu;
mod regs;

/// Every sequential module: the flag register, the register section
/// (vectors/regs.rs), the address section (vectors/addr.rs) and the
/// instruction register (vectors/ir.rs), memory (vectors/mem.rs,
/// mem_cpu.rs) and the control unit (vectors/cu.rs).
fn seq_modules() -> impl Iterator<Item = &'static SeqModule> {
    SEQ_MODULES.iter().chain(regs::MODULES).chain(addr::MODULES).chain(ir::MODULES).chain(mem::MODULES).chain(mem_cpu::MODULES).chain(cu::MODULES)
}

fn names() -> String {
    MODULES.iter().map(|m| m.name).chain(seq_modules().map(|m| m.name)).collect::<Vec<_>>().join(", ")
}

/// `hbc2 vectors <module> [-o file]`, or `hbc2 vectors --list`.
pub fn vectors(args: &[String]) -> Result<(), String> {
    let Some(name) = args.first() else {
        return Err(format!("usage: hbc2 vectors <module> [-o file]\nmodules: {}", names()));
    };
    if name == "--list" {
        println!("{}", names().replace(", ", "
"));
        return Ok(());
    }
    let out = crate::option(args, "-o");
    if let Some(seq) = seq_modules().find(|m| m.name == name) {
        return match out {
            Some(path) => {
                let file = std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?;
                write_seq(seq, &mut BufWriter::new(file))
            }
            None => write_seq(seq, &mut BufWriter::new(std::io::stdout().lock())),
        }
        .map_err(|e| format!("writing vectors: {e}"));
    }
    let module = MODULES
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("no module {name:?}; modules: {}", names()))?;
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
