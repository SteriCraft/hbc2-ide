//! The register section (logisim/notes/registers-decisions.md).
//!
//! - `regs`: the register file, the operand buses and the data bus drivers
//!   against a model of the section: writes and reads on both ports, every
//!   index and every index source, reads of the register being written,
//!   every operand source, blocked writes, the reset sweep, random sequences.
//! - `regs_cpu`: instruction-level sequences. The real CPU runs one
//!   instruction; the rows drive the section the way the control unit will
//!   (the step lists below) and check the operands it presents, the bytes
//!   it puts on the data bus, and the 8 registers at the end, against the
//!   CPU.
//!
//! One clock step = 2 rows (clk low, then the rising edge), or 4 rows when
//! it writes (clk low, clk_wr_n low, clk_wr_n high, edge).

use super::{MACHINE, Out, Row, Rng, SeqModule};
use hbc2_core::isa::{AddrMode, Opcode, encode};
use hbc2_core::isa::Register as Reg;

pub const REGS_IN: &[(&str, u32)] = &[
    ("clk", 1),
    ("clk_wr_n", 1),
    ("regs_we_n", 1),
    ("regs_a_sel", 2),
    ("regs_cu_idx", 2),
    ("regs_cu_bank_n", 2),
    ("regs_b_sel", 2),
    ("regs_a_lat_oe_n", 1),
    ("regs_b_lat_oe_n", 1),
    ("regs_const_oe_n", 1),
    ("regs_const_0", 1),
    ("regs_a_bus_oe_n", 1),
    ("regs_res_oe_n", 1),
    ("regs_r1", 3),
    ("regs_r2", 3),
    ("regs_r3", 3),
    ("regs_alu_r", 8),
    ("regs_bus_in", 8),
];
pub const REGS_OUT: &[(&str, u32)] = &[("regs_a", 8), ("regs_b", 8), ("regs_bus_out", 8)];

/// What drives bus A.
#[derive(Clone, Copy, PartialEq, Debug)]
enum A {
    R1,
    R2,
    R3,
    /// The control unit's index (0..7).
    Cu(u8),
    /// The A operand latch (port A off).
    Latch,
    /// Nothing (port A off, latch off): only for blocked-write checks.
    Off,
}

/// What drives bus B.
#[derive(Clone, Copy, PartialEq, Debug)]
enum B {
    R2,
    R1,
    R3,
    Latch,
    /// The constant 0000000c.
    Const(u8),
}

/// One step's inputs.
#[derive(Clone, Copy, Debug)]
struct Step {
    we: bool,
    a: A,
    b: B,
    a_bus: bool,
    res: bool,
    f: [u8; 3],
    alu_r: u8,
    bus: u8,
}

impl Step {
    fn new(f: [u8; 3]) -> Step {
        Step { we: false, a: A::R1, b: B::R2, a_bus: false, res: false, f, alu_r: 0, bus: 0 }
    }
    fn a_index(&self) -> Option<u8> {
        match self.a {
            A::R1 => Some(self.f[0]),
            A::R2 => Some(self.f[1]),
            A::R3 => Some(self.f[2]),
            A::Cu(k) => Some(k),
            A::Latch | A::Off => None,
        }
    }
    fn b_index(&self) -> Option<u8> {
        match self.b {
            B::R2 => Some(self.f[1]),
            B::R1 => Some(self.f[0]),
            B::R3 => Some(self.f[2]),
            _ => None,
        }
    }
    fn inputs(&self, clk: bool, wr: bool) -> Vec<u32> {
        let (a_sel, cu_idx, bank) = match self.a {
            A::R1 => (0, 0, 0b11),
            A::R2 => (1, 0, 0b11),
            A::R3 => (2, 0, 0b11),
            A::Cu(k) => (3, k as u32 & 3, if k < 4 { 0b10 } else { 0b01 }),
            A::Latch | A::Off => (3, 0, 0b11),
        };
        let b_sel = match self.b {
            B::R2 => 0,
            B::R1 => 1,
            B::R3 => 2,
            _ => 3,
        };
        let konst = match self.b {
            B::Const(c) => c as u32,
            _ => 0,
        };
        vec![
            clk as u32,
            !wr as u32,
            !self.we as u32,
            a_sel,
            cu_idx,
            bank,
            b_sel,
            (self.a != A::Latch) as u32,
            (self.b != B::Latch) as u32,
            !matches!(self.b, B::Const(_)) as u32,
            konst,
            !self.a_bus as u32,
            !self.res as u32,
            self.f[0] as u32,
            self.f[1] as u32,
            self.f[2] as u32,
            self.alu_r as u32,
            self.bus as u32,
        ]
    }
}

/// The section's state. None = not known (power-up).
#[derive(Clone, Copy)]
struct Model {
    regs: [Option<u8>; 8],
    res: Option<u8>,
    /// Both operand latches hold the same byte.
    lat: Option<u8>,
}

fn out(v: Option<u8>) -> Out {
    v.map_or(Out::X, |v| Out::V(v as u32))
}

impl Model {
    fn new() -> Model {
        Model { regs: [None; 8], res: None, lat: None }
    }
    fn bus_a(&self, s: &Step) -> Option<u8> {
        match s.a {
            A::Latch => self.lat,
            _ => s.a_index().and_then(|i| self.regs[i as usize]),
        }
    }
    fn bus_b(&self, s: &Step) -> Option<u8> {
        match s.b {
            B::Latch => self.lat,
            B::Const(c) => Some(c),
            _ => self.regs[s.b_index().unwrap() as usize],
        }
    }
    fn outputs(&self, s: &Step) -> Vec<Out> {
        let a = if s.a == A::Off { Out::Z } else { out(self.bus_a(s)) };
        let bus = if s.a_bus {
            a
        } else if s.res {
            out(self.res)
        } else {
            Out::Z
        };
        vec![a, out(self.bus_b(s)), bus]
    }
    /// The rows of one step; the model is updated (write, then the edge).
    fn step(&mut self, rows: &mut Vec<Row>, s: Step) {
        assert!(!(s.a_bus && s.res), "two drivers on the data bus");
        assert!(!(s.we && s.a_bus), "the bus-A driver is never on during a write");
        rows.push((s.inputs(false, false), self.outputs(&s)));
        if s.we {
            if let Some(i) = s.a_index() {
                self.regs[i as usize] = Some(s.bus); // transparent: the ports show it at once
            }
            rows.push((s.inputs(false, true), self.outputs(&s)));
            rows.push((s.inputs(false, false), self.outputs(&s)));
        }
        self.res = Some(s.alu_r);
        self.lat = Some(s.bus);
        rows.push((s.inputs(true, false), self.outputs(&s)));
    }
    /// Writes `values` into the 8 registers through the control unit's index.
    fn load(&mut self, rows: &mut Vec<Row>, values: [u8; 8], f: [u8; 3]) {
        for (k, &v) in values.iter().enumerate() {
            self.step(rows, Step { we: true, a: A::Cu(k as u8), bus: v, ..Step::new(f) });
        }
    }
}

fn bytes8(rng: &mut Rng) -> [u8; 8] {
    std::array::from_fn(|_| rng.byte() as u8)
}

/// Model-based sequences (see the module comment).
pub fn regs_sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let mut sets = Vec::new();

    // reset sweep, as the control unit will run it (control-requirements.md C7):
    // B = 00, the ALU (AND) gives 00, the result latch drives the bus
    {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        let f = [5, 2, 7];
        m.step(&mut rows, Step { b: B::Const(0), alu_r: 0, ..Step::new(f) });
        for k in 0..8 {
            m.step(&mut rows, Step { we: true, a: A::Cu(k), b: B::Const(0), res: true, ..Step::new(f) });
        }
        for k in 0..8 {
            m.step(&mut rows, Step { a: A::Cu(k), ..Step::new(f) });
            assert_eq!(m.regs[k as usize], Some(0));
        }
        sets.push(rows);
    }

    // every pair of indexes on both ports, from every field source
    for pass in 0..2 {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        let vals = if pass == 0 { [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80] } else { bytes8(&mut rng) };
        m.load(&mut rows, vals, [0, 0, 0]);
        for i in 0..8u8 {
            for j in 0..8u8 {
                let k = rng.byte() as u8 & 7;
                for (a, b, f) in [
                    (A::R1, B::R2, [i, j, k]),
                    (A::R2, B::R1, [j, i, k]),
                    (A::R3, B::R3, [k, k ^ 7, i]),
                    (A::Cu(i), B::R3, [k, k, j]),
                ] {
                    m.step(&mut rows, Step { a, b, ..Step::new(f) });
                }
            }
        }
        sets.push(rows);
    }

    // a write to every index from every A source, port B reading every index
    // (the written one included: it follows D while /WE is low)
    for src in 0..4 {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        m.load(&mut rows, bytes8(&mut rng), [0, 0, 0]);
        for w in 0..8u8 {
            for j in 0..8u8 {
                let other = (w + 1 + rng.byte() as u8 % 7) & 7;
                let (a, f) = match src {
                    0 => (A::R1, [w, j, other]),
                    1 => (A::R2, [j, w, other]),
                    2 => (A::R3, [other, j, w]),
                    _ => (A::Cu(w), [other, j, other]),
                };
                let b = if src == 1 { B::R1 } else { B::R2 };
                let f = if src == 1 { [j, w, other] } else { f };
                let bus = rng.byte() as u8;
                m.step(&mut rows, Step { we: true, a, b, bus, alu_r: rng.byte() as u8, ..Step::new(f) });
                m.step(&mut rows, Step { a: A::Cu(w), b: B::R2, ..Step::new([0, j, 0]) });
            }
        }
        sets.push(rows);
    }

    // operand sources and data bus drivers, with values that show a wrong source
    {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        m.load(&mut rows, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88], [0, 0, 0]);
        for _ in 0..64 {
            let f = [rng.byte() as u8 & 7, rng.byte() as u8 & 7, rng.byte() as u8 & 7];
            for a in [A::R1, A::Latch] {
                for b in [B::R2, B::R1, B::R3, B::Latch, B::Const(0), B::Const(1)] {
                    for (a_bus, res) in [(false, false), (true, false), (false, true)] {
                        let (alu_r, bus) = (rng.byte() as u8, rng.byte() as u8);
                        m.step(&mut rows, Step { a, b, a_bus, res, alu_r, bus, ..Step::new(f) });
                    }
                }
            }
        }
        // port A off: no write can happen, bus A floats
        for k in 0..8 {
            m.step(&mut rows, Step { we: true, a: A::Off, bus: 0xA5, ..Step::new([k, k, k]) });
            m.step(&mut rows, Step { we: true, a: A::Latch, bus: 0x5A, ..Step::new([k, k, k]) });
        }
        sets.push(rows);
    }

    // random sequences of valid steps
    for _ in 0..40 {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        m.load(&mut rows, bytes8(&mut rng), [0, 0, 0]);
        for _ in 0..200 {
            let r = rng.byte();
            let f = [rng.byte() as u8 & 7, rng.byte() as u8 & 7, rng.byte() as u8 & 7];
            let a = match r & 7 {
                0 | 1 => A::R1,
                2 => A::R2,
                3 => A::R3,
                4 | 5 => A::Cu(rng.byte() as u8 & 7),
                _ => A::Latch,
            };
            let b = match (r >> 3) & 7 {
                0 | 1 => B::R2,
                2 => B::R1,
                3 => B::R3,
                4 | 5 => B::Latch,
                k => B::Const(k as u8 & 1),
            };
            let we = a != A::Latch && r & 0x40 != 0;
            let (a_bus, res) = match rng.byte() % 3 {
                0 => (false, false),
                1 => (!we, false),
                _ => (false, true),
            };
            m.step(&mut rows, Step { we, a, b, a_bus, res, f, alu_r: rng.byte() as u8, bus: rng.byte() as u8 });
        }
        sets.push(rows);
    }
    sets
}

// --------------------------------------------------------------------------
// Instruction-level sequences against the real CPU
// --------------------------------------------------------------------------

/// The CPU state around one instruction.
struct Run {
    before: [u8; 8],
    after: [u8; 8],
    sp: u16,
    /// RAM byte at an address, before and after.
    ram: Box<dyn Fn(u16) -> (u8, u8)>,
    /// IOD port value before and after.
    port: Box<dyn Fn(u8) -> (u8, u8)>,
}

/// Runs `i` from registers `regs`, with random bytes at the addresses the
/// instruction can touch and on every I/O port.
fn run(i: hbc2_core::isa::Instruction, regs: [u8; 8], sp: u16, rng: &mut Rng) -> Run {
    MACHINE.with(|m| {
        let (cpu, board) = &mut *m.borrow_mut();
        board.reset();
        cpu.reset();
        board.ram.fill(0);
        for a in 4..=0xFFFFu32 {
            if a & 0xFF == 0 || a % 7 == 0 {
                continue;
            }
            board.ram.write(a as u16, rng.byte() as u8);
        }
        for p in 0..=255u8 {
            board.iod.set_port(p, rng.byte() as u8);
        }
        for (k, b) in i.to_bytes().iter().enumerate() {
            board.ram.write(k as u16, *b);
        }
        cpu.registers = regs;
        cpu.sp = sp;
        let ram0 = board.ram.as_slice().to_vec();
        let ports0: Vec<u8> = (0..=255u8).map(|p| board.iod.port(p)).collect();
        cpu.tick(board);
        assert!(!cpu.is_halted(), "{i:?} stopped the CPU");
        let ram1 = board.ram.as_slice().to_vec();
        let ports1: Vec<u8> = (0..=255u8).map(|p| board.iod.port(p)).collect();
        Run {
            before: regs,
            after: cpu.registers,
            sp: cpu.sp,
            ram: Box::new(move |a| (ram0[a as usize], ram1[a as usize])),
            port: Box::new(move |p| (ports0[p as usize], ports1[p as usize])),
        }
    })
}

/// Expected values for steps that check the operands or the address: the
/// rows' outputs come from the model, and these asserts make sure the model
/// agrees with the CPU (so a wrong step list can't pass).
fn expect(m: &Model, s: &Step, a: Option<u8>, b: Option<u8>, what: &str) {
    if let Some(a) = a {
        assert_eq!(m.bus_a(s), Some(a), "{what}: bus A");
    }
    if let Some(b) = b {
        assert_eq!(m.bus_b(s), Some(b), "{what}: bus B");
    }
}

/// The ALU ops tested in every operand mode, and what the bench's ALU does.
fn alu(op: Opcode, a: u8, b: u8) -> u8 {
    match op {
        Opcode::Add => a.wrapping_add(b),
        Opcode::Sub => a.wrapping_sub(b),
        Opcode::Xor => a ^ b,
        Opcode::And => a & b,
        Opcode::Inc => a.wrapping_add(b),
        Opcode::Not => !a,
        _ => unreachable!(),
    }
}

pub fn regs_cpu_sets() -> Vec<Vec<Row>> {
    use Opcode as O;
    let mut rng = Rng(0x6A09_E667_F3BC_C909);
    let mut sets = Vec::new();
    let r = |k: u8| Reg::from_bits(k);

    for r1 in 0..8u8 {
        for r2 in 0..8u8 {
            for r3 in 0..8u8 {
                // instructions that use R3 are run for every R3, the others once
                let mut list: Vec<(&str, hbc2_core::isa::Instruction)> = vec![
                    ("lod mode 4", encode::ramreg_offset(O::Lod, r(r1), r(r2), r(r3), rng.byte() as u8)),
                    ("str mode 4", encode::ramreg_offset(O::Str, r(r1), r(r2), r(r3), rng.byte() as u8)),
                    ("add mode 4", encode::ramreg_offset(O::Add, r(r1), r(r2), r(r3), rng.byte() as u8)),
                ];
                if r3 == 0 {
                    let vx = 0x0100 | rng.byte() as u16 * 0x0101;
                    let v1 = rng.byte() as u8;
                    list.extend([
                        ("mov reg", encode::reg(O::Mov, r(r1), r(r2))),
                        ("mov imm", encode::reg_imm8(O::Mov, r(r1), v1)),
                        ("lod ram", encode::reg_ram(O::Lod, r(r1), vx)),
                        ("lod stack", encode::stack_offset(O::Lod, r(r1), v1)),
                        ("str ram", encode::reg_ram(O::Str, r(r1), vx)),
                        ("str stack", encode::stack_offset(O::Str, r(r1), v1)),
                        ("psh", encode::reg(O::Psh, r(r1), r(0))),
                        ("pop", encode::reg(O::Pop, r(r1), r(0))),
                        ("in", encode::reg(O::In, r(r1), r(r2))),
                        ("out", encode::reg(O::Out, r(r1), r(r2))),
                        ("rsp", encode::reg16(O::Rsp, r(r1), r(r2))),
                        ("add reg", encode::reg(O::Add, r(r1), r(r2))),
                        ("sub imm", encode::reg_imm8(O::Sub, r(r1), v1)),
                        ("xor ram", encode::reg_ram(O::Xor, r(r1), vx)),
                        ("and stack", encode::stack_offset(O::And, r(r1), v1)),
                        ("inc reg", encode::reg(O::Inc, r(r1), r(0))),
                        ("inc imm16", encode::imm16(O::Inc, vx)),
                        ("inc mode 4", encode::ramreg_offset(O::Inc, r(r1), r(r2), r(0), rng.byte() as u8)),
                        ("not imm16", encode::imm16(O::Not, vx)),
                    ]);
                }
                for (what, i) in list {
                    let regs = std::array::from_fn(|_| rng.byte() as u8);
                    let sp = 0x8000 | (rng.byte() as u16) << 4 | 0x0F;
                    let c = run(i, regs, sp, &mut rng);
                    sets.push(sequence(what, i, &c));
                }
            }
        }
    }
    sets
}

/// The control unit's steps for one instruction (registers-decisions.md).
fn sequence(what: &str, i: hbc2_core::isa::Instruction, c: &Run) -> Vec<Row> {
    use AddrMode as M;
    use Opcode as O;
    let (op, mode) = (i.opcode().unwrap(), i.mode().unwrap());
    let f = [i.r1(), i.r2(), i.r3()];
    let reg = |k: u8| c.before[k as usize];
    let pointer = u16::from_be_bytes([reg(f[0]), reg(f[1])]).wrapping_add(i.v2() as u16);
    let stack = c.sp.wrapping_add(i.v1() as u16); // LOD/STR stack: SP unchanged by them
    let what = format!("{what} r1={} r2={} r3={}", f[0], f[1], f[2]);

    let (mut m, mut rows) = (Model::new(), Vec::new());
    m.load(&mut rows, c.before, f);
    let s0 = Step::new(f);
    let go = |m: &mut Model, rows: &mut Vec<Row>, s: Step, a: Option<u8>, b: Option<u8>| {
        expect(m, &s, a, b, &what);
        m.step(rows, s);
    };
    // the result latch drives the bus: the bench's data bus shows it
    let write_res = |res: u8, a: A| Step { we: true, a, res: true, bus: res, ..s0 };

    match (op, mode) {
        (O::Mov, M::Reg) => {
            // AND R2, R2 into the result latch, then write R1
            let v = alu(O::And, reg(f[1]), reg(f[1]));
            go(&mut m, &mut rows, Step { a: A::R2, b: B::R2, alu_r: v, ..s0 }, Some(reg(f[1])), Some(reg(f[1])));
            go(&mut m, &mut rows, write_res(v, A::R1), None, None);
        }
        (O::Mov, M::RegImm8) => {
            go(&mut m, &mut rows, Step { we: true, bus: i.v1(), ..s0 }, None, None); // while V1 is fetched
        }
        (O::Lod, M::RegRam) | (O::Lod, M::StackOffset) | (O::Pop, M::Reg) => {
            let addr = match (op, mode) {
                (O::Lod, M::RegRam) => i.vx(),
                (O::Lod, _) => stack,
                _ => c.sp.wrapping_sub(1),
            };
            go(&mut m, &mut rows, Step { we: true, bus: (c.ram)(addr).0, ..s0 }, None, None);
        }
        (O::Lod, M::RamRegImmReg) => {
            // R1:R2 to the address latch, then memory -> R3
            go(&mut m, &mut rows, s0, Some(reg(f[0])), Some(reg(f[1])));
            go(&mut m, &mut rows, Step { we: true, a: A::R3, bus: (c.ram)(pointer).0, ..s0 }, None, None);
        }
        (O::Str, _) | (O::Psh, M::Reg) => {
            let (addr, a) = match mode {
                M::RegRam => (i.vx(), A::R1),
                M::StackOffset => (stack, A::R1),
                M::RamRegImmReg => (pointer, A::R3),
                _ => (c.sp, A::R1), // PSH: SP after = the byte written
            };
            if mode == M::RamRegImmReg {
                go(&mut m, &mut rows, s0, Some(reg(f[0])), Some(reg(f[1])));
            }
            let written = (c.ram)(addr).1;
            let s = Step { a, a_bus: true, ..s0 };
            assert_eq!(m.bus_a(&s), Some(written), "{what}: stored byte");
            go(&mut m, &mut rows, Step { bus: written, ..s }, None, None);
        }
        (O::In, M::Reg) => {
            // B = R2 (the port) to the address latch, then the IOD -> R1
            go(&mut m, &mut rows, s0, None, Some(reg(f[1])));
            go(&mut m, &mut rows, Step { we: true, bus: (c.port)(reg(f[1])).0, ..s0 }, None, None);
        }
        (O::Out, M::Reg) => {
            // B = R1 (the port) to the address latch, then R2 -> data bus
            go(&mut m, &mut rows, Step { b: B::R1, ..s0 }, None, Some(reg(f[0])));
            let s = Step { a: A::R2, a_bus: true, bus: reg(f[1]), ..s0 };
            assert_eq!(Some((c.port)(reg(f[0])).1), m.bus_a(&s), "{what}: port value");
            go(&mut m, &mut rows, s, None, None);
        }
        (O::Rsp, M::Reg16) => {
            let [hi, lo] = c.sp.to_be_bytes();
            go(&mut m, &mut rows, Step { we: true, a: A::R1, bus: hi, ..s0 }, None, None);
            go(&mut m, &mut rows, Step { we: true, a: A::R2, bus: lo, ..s0 }, None, None);
        }
        (O::Inc, M::Reg) => {
            let v = alu(O::Inc, reg(f[0]), 1);
            go(&mut m, &mut rows, Step { b: B::Const(1), alu_r: v, ..s0 }, Some(reg(f[0])), Some(1));
            go(&mut m, &mut rows, write_res(v, A::R1), None, None);
        }
        (O::Inc, _) | (O::Not, M::Imm16) => {
            // memory -> operand latch; ALU with A = the latch; result -> memory
            let addr = if mode == M::Imm16 { i.vx() } else { pointer };
            if mode == M::RamRegImmReg {
                go(&mut m, &mut rows, s0, Some(reg(f[0])), Some(reg(f[1])));
            }
            let (mem, written) = (c.ram)(addr);
            go(&mut m, &mut rows, Step { bus: mem, ..s0 }, None, None);
            let (b, bv) = if op == O::Inc { (B::Const(1), 1) } else { (B::R2, 0) };
            let v = alu(op, mem, bv);
            go(&mut m, &mut rows, Step { a: A::Latch, b, alu_r: v, ..s0 }, Some(mem), (op == O::Inc).then_some(1));
            let s = Step { a: A::Off, res: true, bus: v, ..s0 };
            assert_eq!(m.res, Some(written), "{what}: written byte");
            go(&mut m, &mut rows, s, None, None);
        }
        (_, _) => {
            // ALU op R1 (or R3) with R2, V1 or a memory operand
            let (a, dest, b, bv) = match mode {
                M::Reg => (A::R1, f[0], B::R2, reg(f[1])),
                M::RegImm8 => (A::R1, f[0], B::Latch, i.v1()),
                M::RegRam => (A::R1, f[0], B::Latch, (c.ram)(i.vx()).0),
                M::StackOffset => (A::R1, f[0], B::Latch, (c.ram)(stack).0),
                _ => (A::R3, f[2], B::Latch, (c.ram)(pointer).0),
            };
            if mode == M::RamRegImmReg {
                go(&mut m, &mut rows, s0, Some(reg(f[0])), Some(reg(f[1])));
            }
            if b == B::Latch {
                go(&mut m, &mut rows, Step { bus: bv, ..s0 }, None, None); // V1 fetch or memory read
            }
            let v = alu(op, reg(dest), bv);
            assert_eq!(v, c.after[dest as usize], "{what}: ALU result");
            go(&mut m, &mut rows, Step { a, b, alu_r: v, ..s0 }, Some(reg(dest)), Some(bv));
            go(&mut m, &mut rows, write_res(v, a), None, None);
        }
    }
    // read the 8 registers back: they must be the CPU's
    for k in 0..8u8 {
        let s = Step { a: A::Cu(k), ..s0 };
        assert_eq!(m.bus_a(&s), Some(c.after[k as usize]), "{what}: register {k} at the end");
        m.step(&mut rows, s);
    }
    rows
}

pub const MODULES: &[SeqModule] = &[
    SeqModule { name: "regs", inputs: REGS_IN, outputs: REGS_OUT, sets: regs_sets },
    SeqModule { name: "regs_cpu", inputs: REGS_IN, outputs: REGS_OUT, sets: regs_cpu_sets },
];
