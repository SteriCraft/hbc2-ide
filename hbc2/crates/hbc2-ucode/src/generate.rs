//! The generator: lowers each step to a control word (with the per-step
//! rules), places the words in the 15-bit address space (cu-decisions.md
//! section 3), checks the whole microcode, and writes the images, a listing
//! and the cycle-count table.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;

use hbc2_core::isa::{AddrMode, INSTRUCTION_TABLE, Opcode};

use crate::source::{
    self, BlockKind, BusSrc, Cond, Exit, Expr, FlagsOp, IrByte, PortA, PortB, Source, Step, Target,
};
use crate::word::{self, Word, abus, cond, dbus};

/// The boot row: opcode 3F (invalid), mode B (`bus_data` = FE, SP high).
pub const BOOT_ROW: usize = (0x3F << 4) | 0xB;
/// The forced row (00, 0).
pub const FORCED_ROW: usize = 0;

/// What a step does, for the rules that look across steps.
#[derive(Clone, Copy, Debug, Default)]
pub struct Info {
    pub writes_reg: bool,
    pub writes_ram: bool,
    pub alu: Option<u8>,
    pub b_zero: bool,
    pub reads_res: bool,
    /// Byte loaded by a fetch step (bus<-ram, load, lat, pc <- pc + 1).
    pub fetch: Option<IrByte>,
    pub loads: [bool; 4],
    /// IR fields read: R1/R2 (byte 1), R3 and V1, V2.
    pub uses_b1: bool,
    pub uses_v1: bool,
    pub uses_v2: bool,
    /// Flags this step can change (bit numbers C0 .. H7).
    pub flags: u8,
}

impl Info {
    fn alu_step(&self) -> bool {
        // AND with 00 gives 00 whatever port A shows, so a write in that
        // step (the reset sweep) can't disturb the result.
        self.alu.is_some() && (!self.writes_reg || (self.alu == Some(0b1000) && self.b_zero))
    }
}

fn flag_mask(f: FlagsOp) -> u8 {
    match f {
        FlagsOp::Czn => 1 << 0 | 1 << 5 | 1 << 3,
        FlagsOp::Zn => 1 << 5 | 1 << 3,
        FlagsOp::Sef => 1 << 4 | 1 << 1 | 1 << 6,
        FlagsOp::Bus => 0xFF,
        FlagsOp::Single(n, _) => 1 << n,
    }
}

/// Lowers one step to its control word (sequencing fields left idle) and
/// checks the rules that apply inside a step.
pub fn lower(s: &Step) -> Result<(Word, Info), String> {
    let mut w = Word::idle();
    let mut info = Info::default();

    // ----- bus_data: at most one source (C5)
    let mut sources = s.bus.len() + s.hwvec as usize;
    if s.ram_copy {
        // the ROM drives the memory's local bus only, not bus_data
        sources += 0;
    }
    if sources > 1 {
        return Err("two sources on bus_data".into());
    }
    let src = s.bus.first().copied();
    let has_src = sources == 1;
    match src {
        Some(BusSrc::A) => w.set(word::DBUS, dbus::BUS_A),
        Some(BusSrc::Res) => w.set(word::DBUS, dbus::RES),
        Some(BusSrc::Flags) => w.set(word::DBUS, dbus::FLAGS),
        Some(BusSrc::PcHi) => w.set(word::DBUS, dbus::PC_HI),
        Some(BusSrc::PcLo) => w.set(word::DBUS, dbus::PC_LO),
        Some(BusSrc::SpHi) => w.set(word::DBUS, dbus::SP_HI),
        Some(BusSrc::SpLo) => w.set(word::DBUS, dbus::SP_LO),
        Some(BusSrc::Ram) | Some(BusSrc::Io) | None => {}
    }
    info.reads_res = src == Some(BusSrc::Res);

    // ----- memory strobes: only C19's four cases
    let ram_read = src == Some(BusSrc::Ram);
    let mem = [ram_read, s.ram_write, s.ram_copy]
        .iter()
        .filter(|x| **x)
        .count();
    if mem > 1 {
        return Err("two memory operations in one step".into());
    }
    let strobes = if ram_read {
        Some([1, 0, 1, 1, 1])
    } else if s.ram_write {
        if !has_src || src == Some(BusSrc::Ram) {
            return Err("ram<-bus without a CPU source on bus_data".into());
        }
        info.writes_ram = true;
        Some([1, 1, 0, 0, 1])
    } else if s.ram_copy {
        Some([1, 1, 0, 1, 0])
    } else {
        None
    };
    if let Some([re, oe, we, wbuf, rom]) = strobes {
        w.set(word::MEM_RE, re);
        w.set(word::MEM_OE_N, oe);
        w.set(word::MEM_WE_N, we);
        w.set(word::MEM_WBUF_OE_N, wbuf);
        w.set(word::MEM_ROM_OE_N, rom);
    }

    // ----- bus_addr and the IOD
    let io_in = src == Some(BusSrc::Io);
    let abus_uses = [io_in || s.io_out, s.hwvec, s.swvec]
        .iter()
        .filter(|x| **x)
        .count();
    if abus_uses > 1 || (io_in && s.io_out) {
        return Err("two bus_addr modes (io, hwvec, swvec) in one step".into());
    }
    if io_in || s.io_out {
        w.set(word::ABUS, abus::IO);
    }
    if s.hwvec {
        w.set(word::ABUS, abus::HW_VECTOR);
    }
    if s.swvec {
        w.set(word::ABUS, abus::SW_VECTOR);
    }
    if s.io_out && src != Some(BusSrc::A) {
        return Err("io<-bus needs bus<-a (OUT drives R2 from bus A)".into());
    }
    if strobes.is_some() && (abus_uses > 0) {
        return Err("RE and IE (or a vector step) together (C19)".into());
    }

    // ----- registers
    let pa = match (s.a, s.write) {
        (Some(a), Some(wr)) if a != wr => return Err("a= and the written register differ".into()),
        (a, wr) => wr.or(a),
    };
    match pa {
        Some(PortA::R1) => set_a(&mut w, 0b00, 0, 0b10),
        Some(PortA::R2) => set_a(&mut w, 0b01, 0, 0b10),
        Some(PortA::R3) => set_a(&mut w, 0b10, 0, 0b10),
        Some(PortA::Cu(n)) => set_a(&mut w, 0b11, n & 3, if n < 4 { 0b10 } else { 0b01 }),
        None => {} // idle: port A off, the A operand latch on bus A
    }
    if let Some(wr) = s.write {
        if !has_src {
            return Err("register write without a source on bus_data".into());
        }
        if src == Some(BusSrc::A) {
            return Err(
                "regs_a_bus_oe_n with regs_we_n (C5: bus A onto bus_data while writing)".into(),
            );
        }
        w.set(word::REGS_WE_N, 0);
        info.writes_reg = true;
        let _ = wr;
    }
    if let Some(op) = s.alu {
        w.set(word::ALU_OP, op);
        info.alu = Some(op);
    }
    match s.b {
        Some(PortB::R2) => w.set(word::REGS_B_SEL, 0b00),
        Some(PortB::R1) => w.set(word::REGS_B_SEL, 0b01),
        Some(PortB::R3) => w.set(word::REGS_B_SEL, 0b10),
        Some(c @ (PortB::One | PortB::Zero)) => {
            // regs_const_0 = NOT alu_op3 (cu-decisions.md section 2)
            let Some(op) = s.alu else {
                return Err("b=one / b=zero need alu= (regs_const_0 is NOT alu_op bit 3)".into());
            };
            let one = op & 0b1000 == 0;
            if (c == PortB::One) != one {
                return Err(format!(
                    "b={} with alu_op {op:04b}: the constant would be {:02}",
                    if c == PortB::One { "one" } else { "zero" },
                    one as u8
                ));
            }
            w.set(word::REGS_CONST_OE_N, 0);
            info.b_zero = c == PortB::Zero;
        }
        None => {}
    }

    // ----- IR loads, pending latch
    for l in &s.loads {
        let f = match l {
            IrByte::B0 => word::IR_B0_LD_N,
            IrByte::B1 => word::IR_B1_LD_N,
            IrByte::V1 => word::IR_V1_LD_N,
            IrByte::V2 => word::IR_V2_LD_N,
        };
        w.set(f, 0);
        info.loads[*l as usize] = true;
    }
    if !s.loads.is_empty() && !has_src {
        return Err("IR load without a source on bus_data".into());
    }
    if s.swi {
        if !has_src {
            return Err("swi<-bus without a source on bus_data".into());
        }
        w.set(word::CU_SWI_LD_N, 0);
    }
    if s.inr {
        w.set(word::INR, 1);
    }

    // ----- flags (one store per step: the register writes one group)
    if s.flags.len() > 1 {
        return Err(
            "two flag stores in one step (the register stores one group per clock: C2)".into(),
        );
    }
    if let Some(&f) = s.flags.first() {
        match f {
            FlagsOp::Czn | FlagsOp::Zn | FlagsOp::Sef => {
                if s.alu.is_none() {
                    return Err("ALU flags stored without alu=".into());
                }
                if info.writes_reg || info.writes_ram {
                    return Err(
                        "ALU flags stored in a write step (C9: in the ALU step only)".into(),
                    );
                }
            }
            FlagsOp::Bus if !has_src => {
                return Err("flags<-bus without a source on bus_data".into());
            }
            _ => {}
        }
        let (op, v) = match f {
            FlagsOp::Czn => (0b0001, false),
            FlagsOp::Zn => (0b0010, false),
            FlagsOp::Sef => (0b0011, false),
            FlagsOp::Bus => (0b0100, false),
            FlagsOp::Single(n, v) => (0b1000 | n, v),
        };
        w.set(word::FLAGS_OP, op);
        w.set(word::FLAGS_BIT_VAL, v as u8);
        info.flags = flag_mask(f);
    }

    // ----- the address section: one sum per step
    if let Some(&(_, e)) = s.addr.first() {
        if s.addr.iter().any(|(_, x)| *x != e) {
            return Err("two different sums in one step (one adder)".into());
        }
        let mut seen = HashSet::new();
        for (t, _) in &s.addr {
            if !seen.insert(*t) {
                return Err("the same target loaded twice".into());
            }
            w.set(
                match t {
                    Target::Lat => word::ADDR_LAT_LD_N,
                    Target::Pc => word::ADDR_PC_LD_N,
                    Target::Sp => word::ADDR_SP_LD_N,
                },
                0,
            );
        }
        // (base sel, base /E hi lo, off sel, off /G hi lo, inv, cin)
        let (bs, be, os, oe, inv, cin) = match e {
            Expr::Pc => (0b00, 0b00, 0, 0b11, 0, 0),
            Expr::PcPlus1 => (0b00, 0b00, 0, 0b11, 0, 1),
            Expr::PcMinus1 => (0b00, 0b00, 0, 0b11, 1, 0),
            Expr::Sp => (0b01, 0b00, 0, 0b11, 0, 0),
            Expr::SpPlus1 => (0b01, 0b00, 0, 0b11, 0, 1),
            Expr::SpMinus1 => (0b01, 0b00, 0, 0b11, 1, 0),
            Expr::SpPlusV1 => (0b01, 0b00, 1, 0b10, 0, 0),
            Expr::SpMinusV1 => (0b01, 0b00, 1, 0b10, 1, 1),
            Expr::SpPlusVx => (0b01, 0b00, 0, 0b00, 0, 0),
            Expr::SpMinusVx => (0b01, 0b00, 0, 0b00, 1, 1),
            Expr::Ab => (0b10, 0b00, 0, 0b11, 0, 0),
            Expr::AbPlusV2 => (0b10, 0b00, 0, 0b10, 0, 0),
            Expr::ZeroB => (0b10, 0b10, 0, 0b11, 0, 0),
            Expr::Vx => (0b00, 0b11, 0, 0b00, 0, 0),
            Expr::Ivt => (0b11, 0b00, 0, 0b11, 0, 0),
            Expr::IvtHi => (0b11, 0b01, 0, 0b11, 0, 0),
            Expr::Zero => (0b00, 0b11, 0, 0b11, 0, 0),
        };
        w.set(word::ADDR_BASE_SEL, bs);
        w.set(word::ADDR_BASE_OE_N, be);
        w.set(word::ADDR_OFF_SEL, os);
        w.set(word::ADDR_OFF_OE_N, oe);
        w.set(word::ADDR_INV, inv);
        w.set(word::ADDR_CIN, cin);
        if matches!(e, Expr::Ab | Expr::AbPlusV2 | Expr::ZeroB) && info.writes_reg {
            return Err("address load from bus A / B in a register write step (P4)".into());
        }
        info.uses_v1 |= matches!(
            e,
            Expr::SpPlusV1 | Expr::SpMinusV1 | Expr::SpPlusVx | Expr::SpMinusVx | Expr::Vx
        );
        info.uses_v2 |= matches!(
            e,
            Expr::AbPlusV2 | Expr::SpPlusVx | Expr::SpMinusVx | Expr::Vx
        );
        let fetch_sum = e == Expr::PcPlus1
            && seen.contains(&Target::Lat)
            && seen.contains(&Target::Pc)
            && seen.len() == 2;
        if fetch_sum && ram_read && s.loads.len() == 1 {
            info.fetch = Some(s.loads[0]);
        }
    }

    // fields read
    for p in [pa, s.write].into_iter().flatten() {
        match p {
            PortA::R1 | PortA::R2 => info.uses_b1 = true,
            PortA::R3 => info.uses_v1 = true,
            PortA::Cu(_) => {}
        }
    }
    match s.b {
        Some(PortB::R1 | PortB::R2) => info.uses_b1 = true,
        Some(PortB::R3) => info.uses_v1 = true,
        _ => {}
    }
    Ok((w, info))
}

fn set_a(w: &mut Word, sel: u8, idx: u8, bank_n: u8) {
    w.set(word::REGS_A_SEL, sel);
    w.set(word::REGS_CU_IDX, idx);
    w.set(word::REGS_CU_BANK_N, bank_n);
}

// ----- Programs: a block instantiated for one opcode

type WordId = (usize, usize); // (program, step)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    Both(WordId),
    /// (condition, word at cond 1, word at cond 0)
    Pair(Cond, WordId, WordId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rx {
    Local(Key),
    End,
    Dispatch,
    Restart,
}

#[derive(Clone, Debug)]
pub struct Inst {
    pub word: Word,
    pub info: Info,
    pub line: usize,
    pub label: Option<String>,
    rx: Rx,
}

#[derive(Clone, Debug)]
pub struct Prog {
    pub name: String,
    pub insts: Vec<Inst>,
    /// Words of the forced row: their own sequencing stays in that row.
    forced: bool,
}

fn cond_code(c: Cond) -> u8 {
    match c {
        Cond::Jc => cond::JC,
        Cond::NoInt => cond::NOINT,
        Cond::Hwq => cond::HWQ,
        Cond::Xn => cond::XN,
    }
}

/// One placed word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placed {
    id: WordId,
    next: u8,
    force: bool,
    cond_sel: u8,
}

/// The generator's result.
pub struct Microcode {
    pub words: Vec<Word>,
    pub progs: Vec<Prog>,
    /// Which program step each address holds (None: unused, trap word).
    pub origin: Vec<Option<(usize, usize)>>,
    /// Steps per instruction (F0 included): (opcode, mode) → (min, max).
    pub cycles: HashMap<(u8, u8), (u32, u32)>,
    /// Steps of the hardware and software interrupt entries after the
    /// boundary, and from reset to the first fetch.
    pub entry_hw: u32,
    pub entry_sw: u32,
    pub boot_steps: u64,
    /// Every rule checked, with its count, for the report.
    pub checks: Vec<(String, usize)>,
}

const MODE_NAMES: [&str; 9] = [
    "none", "reg", "regimm8", "regram", "mode4", "stack", "reg16", "imm16", "imm8",
];

pub fn mode_from_name(n: &str) -> Option<AddrMode> {
    MODE_NAMES
        .iter()
        .position(|m| *m == n)
        .and_then(|i| AddrMode::from_bits(i as u8))
}

pub fn mode_name(m: u8) -> &'static str {
    MODE_NAMES.get(m as usize).copied().unwrap_or("-")
}

struct Gen {
    progs: Vec<Prog>,
    seqs: HashMap<String, usize>,
    instr: HashMap<(u8, u8), usize>,
    slots: Vec<[[Option<Placed>; 2]; 16]>,
    errors: Vec<String>,
    pending: Vec<(usize, Vec<Option<Exit>>)>,
    forced_map: HashMap<Key, u8>,
    boot_first: u8,
}

fn flags_of(op: Opcode) -> u8 {
    op.info().flags.iter().fold(0, |m, f| m | 1 << *f as u8)
}

/// Compiles the source into the microcode, or the list of errors.
pub fn build(text: &str) -> Result<Microcode, Vec<String>> {
    let src = source::parse(text).map_err(|e| vec![e.to_string()])?;
    let mut g = Gen {
        progs: Vec::new(),
        seqs: HashMap::new(),
        instr: HashMap::new(),
        slots: vec![[[None; 2]; 16]; 1024],
        errors: Vec::new(),
        pending: Vec::new(),
        forced_map: HashMap::new(),
        boot_first: 0,
    };
    g.programs(&src);
    if !g.errors.is_empty() {
        return Err(g.errors);
    }
    let mut checks = Vec::new();
    g.check_programs(&mut checks);
    if !g.errors.is_empty() {
        return Err(g.errors);
    }
    g.place();
    if !g.errors.is_empty() {
        return Err(g.errors);
    }
    g.check_reachable(&mut checks);
    if !g.errors.is_empty() {
        return Err(g.errors);
    }
    Ok(g.finish(checks))
}

impl Gen {
    fn programs(&mut self, src: &Source) {
        for b in &src.blocks {
            match &b.kind {
                BlockKind::Seq(name) => {
                    if self.seqs.contains_key(name) {
                        self.errors
                            .push(format!("line {}: duplicate seq `{name}`", b.line));
                    }
                    let p = self.program(src, b, name.clone(), None, name == "forced");
                    self.seqs.insert(name.clone(), p);
                }
                BlockKind::Instr { ops, modes } => {
                    for op in ops {
                        let Some(opcode) = Opcode::from_mnemonic(op) else {
                            self.errors
                                .push(format!("line {}: unknown opcode `{op}`", b.line));
                            continue;
                        };
                        for m in modes {
                            let Some(mode) = mode_from_name(m) else {
                                self.errors
                                    .push(format!("line {}: unknown mode `{m}`", b.line));
                                continue;
                            };
                            let key = (opcode as u8, mode as u8);
                            if self.instr.contains_key(&key) {
                                self.errors
                                    .push(format!("line {}: {op} {m} defined twice", b.line));
                                continue;
                            }
                            let p = self.program(src, b, format!("{op} {m}"), Some(op), false);
                            self.instr.insert(key, p);
                        }
                    }
                }
            }
        }
        for name in ["forced", "reset", "boot", "stop1", "stop2"] {
            if !self.seqs.contains_key(name) {
                self.errors.push(format!("seq `{name}` missing"));
            }
        }
        if self.errors.is_empty() {
            self.resolve();
        }
    }

    fn program(
        &mut self,
        src: &Source,
        b: &source::Block,
        name: String,
        op: Option<&str>,
        forced: bool,
    ) -> usize {
        let index = self.progs.len();
        let mut insts = Vec::new();
        let mut exits = Vec::new();
        for l in &b.lines {
            match src.step(l, op).and_then(|s| {
                let exit = s.exit.clone();
                lower(&s).map(|wi| (wi, exit)).map_err(|m| source::Error {
                    line: l.line,
                    message: m,
                })
            }) {
                Ok(((word, info), exit)) => {
                    insts.push(Inst {
                        word,
                        info,
                        line: l.line,
                        label: l.label.clone(),
                        rx: Rx::End,
                    });
                    exits.push(exit);
                }
                Err(e) => self.errors.push(format!("{name}: {e}")),
            }
        }
        self.progs.push(Prog {
            name,
            insts,
            forced,
        });
        self.pending.push((index, exits));
        index
    }

    fn resolve(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        let f = self.seqs["forced"];
        let forced_label = |g: &Gen, l: &str| {
            g.progs[f]
                .insts
                .iter()
                .position(|i| i.label.as_deref() == Some(l))
                .map(|i| (f, i))
        };
        let f0 = forced_label(self, "F0");
        let w1 = forced_label(self, "W1");
        let (Some(f0), Some(w1)) = (f0, w1) else {
            self.errors.push("seq forced needs F0 and W1".into());
            return;
        };
        for (p, exits) in pending {
            let n = self.progs[p].insts.len();
            let label = |g: &Gen, l: &str| -> Option<WordId> {
                g.progs[p]
                    .insts
                    .iter()
                    .position(|i| i.label.as_deref() == Some(l))
                    .map(|i| (p, i))
            };
            for (i, exit) in exits.into_iter().enumerate() {
                let line = self.progs[p].insts[i].line;
                let name = self.progs[p].name.clone();
                let rx = match exit {
                    None if i + 1 < n => Rx::Local(Key::Both((p, i + 1))),
                    None => {
                        self.errors.push(format!("{name}: line {line}: the last step has no next step (end, next, if ...)"));
                        continue;
                    }
                    Some(Exit::Next(l)) => match label(self, &l) {
                        Some(id) => Rx::Local(Key::Both(id)),
                        None => {
                            self.errors
                                .push(format!("{name}: line {line}: unknown label `{l}`"));
                            continue;
                        }
                    },
                    Some(Exit::If(c, l1, l0)) => match (label(self, &l1), label(self, &l0)) {
                        (Some(a), Some(b)) => Rx::Local(Key::Pair(c, a, b)),
                        _ => {
                            self.errors
                                .push(format!("{name}: line {line}: unknown label in `if`"));
                            continue;
                        }
                    },
                    Some(Exit::End) => Rx::End,
                    Some(Exit::EndNoCheck) => Rx::Local(Key::Both(f0)),
                    Some(Exit::EndHalt) => Rx::Local(Key::Both(w1)),
                    Some(Exit::Dispatch) => Rx::Dispatch,
                    Some(Exit::Restart) => Rx::Restart,
                };
                self.progs[p].insts[i].rx = rx;
            }
        }
    }
}

include!("place.rs");
