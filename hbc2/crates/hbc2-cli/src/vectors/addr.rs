//! The address section: PC, SP, the address adder and the address latch
//! (logisim/notes/addr-decisions.md).
//!
//! - `addr`: every BASE / OFFSET / inversion / carry combination, every load
//!   and output enable, the reset, random sequences, against a model of the
//!   section (sum = BASE + (OFFSET xor inv) + cin, loaded on the edge).
//! - `addr_cpu`: instruction-level sequences. The real CPU runs one
//!   instruction (or takes one interrupt); the rows drive the section the
//!   way the control unit will (the step lists of addr-decisions.md) and
//!   check every address it puts on the address bus, every PC byte it
//!   pushes, and PC and SP at the end, against the CPU.
//!
//! One clock step = 2 rows (clk low, then the rising edge).

use super::{MACHINE, Out, Row, Rng, SeqModule};
use hbc2_core::isa::{AddrMode, IVT_START, Instruction, Opcode, PROGRAM_START, Register as Reg, STACK_RESET, encode};
use hbc2_core::trace::Probe;
use hbc2_core::{Cpu, Flags};

pub const ADDR_IN: &[(&str, u32)] = &[
    ("clk", 1),
    ("rst_n", 1),
    ("addr_base_sel", 2),
    ("addr_base_oe_n", 2),
    ("addr_off_sel", 1),
    ("addr_off_oe_n", 2),
    ("addr_inv", 1),
    ("addr_cin", 1),
    ("addr_lat_ld_n", 1),
    ("addr_pc_ld_n", 1),
    ("addr_sp_ld_n", 1),
    ("addr_lat_oe_n", 2),
    ("addr_pc_oe_n", 2),
    ("addr_sp_oe_n", 2),
    ("addr_bus_a", 8),
    ("addr_bus_b", 8),
    ("addr_v1", 8),
    ("addr_v2", 8),
    ("addr_bus_addr_lo_in", 8),
];
/// All three outputs are tri-state buses: the address bus (two bytes, each
/// with its own enable) and the data bus.
const ADDR_BUSES: &[&str] = &["addr_bus_addr_hi", "addr_bus_addr_lo", "addr_bus_out"];
pub const ADDR_OUT: &[(&str, u32)] = &[("addr_bus_addr_hi", 8), ("addr_bus_addr_lo", 8), ("addr_bus_out", 8)];

/// The BASE input of the adder.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Base {
    Pc,
    Sp,
    /// bus A : bus B (R1:R2).
    Regs,
    /// 00 : bus B (IN, OUT: the port number).
    RegsLo,
    /// FE00 + 2p, p = the low byte of the address bus.
    Ivt,
    /// 0000.
    Off,
}

/// The OFFSET input of the adder (before the inversion).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Off {
    Zero,
    /// 00 : V1.
    V1,
    /// 00 : V2.
    V2,
    /// V1 : V2.
    Vx,
}

/// One step's inputs.
#[derive(Clone, Copy, Debug)]
struct Step {
    rst: bool,
    base_sel: u8,
    /// [low byte, high byte], true = enabled.
    base_oe: [bool; 2],
    /// Low byte of the offset: V1 (true) or V2.
    off_v1_lo: bool,
    off_oe: [bool; 2],
    inv: bool,
    cin: bool,
    /// Loads: latch, PC, SP.
    ld: [bool; 3],
    /// The latch onto the address bus, [low, high].
    lat_oe: [bool; 2],
    /// (register 1 = PC / 2 = SP, byte 0 = low / 1 = high) onto the data bus.
    drive: Option<(usize, usize)>,
    a: u8,
    b: u8,
    v1: u8,
    v2: u8,
    /// What the bench puts on addr_bus_addr_lo_in; None = the address bus
    /// as the latch drives it (0 when it doesn't).
    port: Option<u8>,
}

impl Step {
    fn idle() -> Step {
        Step {
            rst: false,
            base_sel: 0,
            base_oe: [false; 2],
            off_v1_lo: false,
            off_oe: [false; 2],
            inv: false,
            cin: false,
            ld: [false; 3],
            lat_oe: [true; 2],
            drive: None,
            a: 0,
            b: 0,
            v1: 0,
            v2: 0,
            port: None,
        }
    }
    /// sum = base + (off xor inv) + cin.
    fn calc(base: Base, off: Off, inv: bool, cin: bool) -> Step {
        let (base_sel, base_oe) = match base {
            Base::Pc => (0, [true, true]),
            Base::Sp => (1, [true, true]),
            Base::Regs => (2, [true, true]),
            Base::RegsLo => (2, [true, false]),
            Base::Ivt => (3, [true, true]),
            Base::Off => (0, [false, false]),
        };
        let (off_oe, off_v1_lo) = match off {
            Off::Zero => ([false, false], false),
            Off::V1 => ([true, false], true),
            Off::V2 => ([true, false], false),
            Off::Vx => ([true, true], false),
        };
        Step { base_sel, base_oe, off_oe, off_v1_lo, inv, cin, ..Step::idle() }
    }
    fn add(base: Base, off: Off) -> Step {
        Step::calc(base, off, false, false)
    }
    fn plus1(base: Base) -> Step {
        Step::calc(base, Off::Zero, false, true)
    }
    fn minus1(base: Base) -> Step {
        Step::calc(base, Off::Zero, true, false)
    }
    /// base − offset.
    fn sub(base: Base, off: Off) -> Step {
        Step::calc(base, off, true, true)
    }
    fn ld(self, lat: bool, pc: bool, sp: bool) -> Step {
        Step { ld: [lat, pc, sp], ..self }
    }
    /// The reset word (control-requirements.md C16): BASE = IVT high byte
    /// only, no offset, PC and SP loaded, every bus driver off.
    fn reset(port: u8) -> Step {
        Step {
            rst: true,
            base_sel: 3,
            base_oe: [false, true],
            ld: [false, true, true],
            lat_oe: [false, false],
            port: Some(port),
            ..Step::idle()
        }
    }
}

fn oe_n(e: [bool; 2]) -> u32 {
    (!e[1] as u32) << 1 | !e[0] as u32
}

fn byte(v: Option<u16>, hi: bool) -> Option<u8> {
    v.map(|v| if hi { (v >> 8) as u8 } else { v as u8 })
}

fn out(v: Option<u8>) -> Out {
    v.map_or(Out::U, |v| Out::V(v as u32))
}

/// The section's state. None = not known (power-up).
#[derive(Clone, Copy)]
struct Model {
    lat: Option<u16>,
    pc: Option<u16>,
    sp: Option<u16>,
}

impl Model {
    fn new() -> Model {
        Model { lat: None, pc: None, sp: None }
    }
    fn port(&self, s: &Step) -> u8 {
        match s.port {
            Some(p) => p,
            None if s.lat_oe[0] => byte(self.lat, false).unwrap_or(0),
            None => 0,
        }
    }
    fn sum(&self, s: &Step) -> Option<u16> {
        let p = self.port(s);
        let src = match s.base_sel {
            0 => self.pc,
            1 => self.sp,
            2 => Some(u16::from_be_bytes([s.a, s.b])),
            // p7 goes through an AND with rst_n
            _ => Some(u16::from_be_bytes([0xFE | (p >> 7 & !s.rst as u8), p << 1])),
        };
        let mask = if s.base_oe[1] { 0xFF00 } else { 0 } | if s.base_oe[0] { 0x00FF } else { 0 };
        let base = if mask == 0 { Some(0) } else { src.map(|v| v & mask) };
        let hi = if s.off_oe[1] { s.v1 } else { 0 };
        let lo = if !s.off_oe[0] {
            0
        } else if s.off_v1_lo {
            s.v1
        } else {
            s.v2
        };
        let off = u16::from_be_bytes([hi, lo]) ^ if s.inv { 0xFFFF } else { 0 };
        base.map(|b| b.wrapping_add(off).wrapping_add(s.cin as u16))
    }
    fn outputs(&self, s: &Step) -> Vec<Out> {
        let hi = if s.lat_oe[1] { out(byte(self.lat, true)) } else { Out::Z };
        let lo = if s.lat_oe[0] { out(byte(self.lat, false)) } else { Out::Z };
        let bus = match s.drive {
            None => Out::Z,
            Some((r, h)) => out(byte(if r == 1 { self.pc } else { self.sp }, h == 1)),
        };
        vec![hi, lo, bus]
    }
    fn inputs(&self, s: &Step, clk: bool) -> Vec<u32> {
        let drv = |r| match s.drive {
            Some((k, h)) if k == r => [h == 0, h == 1],
            _ => [false, false],
        };
        vec![
            clk as u32,
            !s.rst as u32,
            s.base_sel as u32,
            oe_n(s.base_oe),
            s.off_v1_lo as u32,
            oe_n(s.off_oe),
            s.inv as u32,
            s.cin as u32,
            !s.ld[0] as u32,
            !s.ld[1] as u32,
            !s.ld[2] as u32,
            oe_n(s.lat_oe),
            oe_n(drv(1)),
            oe_n(drv(2)),
            s.a as u32,
            s.b as u32,
            s.v1 as u32,
            s.v2 as u32,
            self.port(s) as u32,
        ]
    }
    /// The rows of one step; the model loads on the edge.
    fn step(&mut self, rows: &mut Vec<Row>, s: Step) {
        let ins = self.inputs(&s, false);
        rows.push((ins.clone(), self.outputs(&s)));
        let sum = self.sum(&s);
        if s.ld[0] {
            self.lat = sum;
        }
        if s.ld[1] {
            self.pc = sum.map(|v| if s.rst { v & 0x01FF } else { v });
        }
        if s.ld[2] {
            self.sp = sum;
        }
        let mut ins = ins;
        ins[0] = 1;
        rows.push((ins, self.outputs(&s)));
    }
}

fn rand16(rng: &mut Rng) -> u16 {
    (rng.byte() << 8 | rng.byte()) as u16
}

/// A random valid step: any adder input, any loads, at most one driver on the data bus.
fn random_step(rng: &mut Rng) -> Step {
    let r = rng.byte();
    let q = rng.byte();
    Step {
        rst: false,
        base_sel: (r & 3) as u8,
        base_oe: [r & 4 != 0, r & 8 != 0],
        off_v1_lo: r & 16 != 0,
        off_oe: [r & 32 != 0, r & 64 != 0],
        inv: r & 128 != 0,
        cin: q & 1 != 0,
        ld: [q & 2 != 0, q & 4 != 0, q & 8 != 0],
        lat_oe: [q & 16 != 0, q & 32 != 0],
        drive: match rng.byte() % 5 {
            0 => None,
            k => Some((1 + (k as usize - 1) / 2, (k as usize - 1) % 2)),
        },
        a: rng.byte() as u8,
        b: rng.byte() as u8,
        v1: rng.byte() as u8,
        v2: rng.byte() as u8,
        port: (q & 64 != 0).then(|| rng.byte() as u8),
    }
}

/// Model-based sequences (see the module comment).
pub fn addr_sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0x243F_6A88_85A3_08D3);
    let mut sets = Vec::new();

    // reset from power-up and from random states, p7 = 0 and 1 on the bus
    {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        for k in 0..32 {
            let port = if k < 2 { [0x00, 0x80][k] } else { rng.byte() as u8 };
            m.step(&mut rows, Step::reset(port));
            assert_eq!((m.pc, m.sp), (Some(PROGRAM_START), Some(STACK_RESET)));
            for (r, h) in [(1, 1), (1, 0), (2, 1), (2, 0)] {
                m.step(&mut rows, Step { drive: Some((r, h)), lat_oe: [false; 2], ..Step::idle() });
            }
            for _ in 0..4 {
                let s = random_step(&mut rng);
                m.step(&mut rows, s);
            }
        }
        sets.push(rows);
    }

    // every adder input combination (512), each loaded into the latch and
    // into PC or SP, then PC and SP read back on the data bus
    for pass in 0..4 {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        let (v1, v2) = (rng.byte() as u8, rng.byte() as u8);
        m.step(&mut rows, Step { v1, v2, ..Step::add(Base::Off, Off::Vx).ld(true, true, true) });
        for combo in 0..512u32 {
            let s = Step {
                base_sel: (combo & 3) as u8,
                base_oe: [combo & 4 != 0, combo & 8 != 0],
                off_oe: [combo & 16 != 0, combo & 32 != 0],
                off_v1_lo: combo & 64 != 0,
                inv: combo & 128 != 0,
                cin: combo & 256 != 0,
                a: rng.byte() as u8,
                b: rng.byte() as u8,
                v1: rng.byte() as u8,
                v2: rng.byte() as u8,
                port: (pass & 1 == 0).then(|| rng.byte() as u8),
                ..Step::idle()
            };
            let target = 1 + (combo as usize + pass) % 2;
            m.step(&mut rows, Step { ld: [true, target == 1, target == 2], ..s });
            m.step(&mut rows, Step { drive: Some((target, 1)), ..Step::idle() });
            m.step(&mut rows, Step { drive: Some((target, 0)), ..Step::idle() });
        }
        sets.push(rows);
    }

    // every load and output enable on its own: the others hold
    {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        for k in 0..64 {
            let (v1, v2) = (rng.byte() as u8, rng.byte() as u8);
            let ld = [k % 3 == 0, k % 3 == 1, k % 3 == 2];
            m.step(&mut rows, Step { v1, v2, ld, ..Step::add(Base::Off, Off::Vx) });
            for lat_oe in [[true, true], [false, true], [true, false], [false, false]] {
                for drive in [None, Some((1, 0)), Some((1, 1)), Some((2, 0)), Some((2, 1))] {
                    let s = Step { v1: !v1, v2: !v2, lat_oe, drive, ..Step::add(Base::Off, Off::Vx) };
                    m.step(&mut rows, s);
                }
            }
        }
        sets.push(rows);
    }

    // random sequences, some steps with rst_n low (any word)
    for _ in 0..40 {
        let (mut m, mut rows) = (Model::new(), Vec::new());
        m.step(&mut rows, Step::reset(rng.byte() as u8));
        for _ in 0..200 {
            let mut s = random_step(&mut rng);
            s.rst = rng.byte() % 16 == 0;
            m.step(&mut rows, s);
        }
        sets.push(rows);
    }
    sets
}

// --------------------------------------------------------------------------
// Instruction-level sequences against the real CPU
// --------------------------------------------------------------------------

/// Records the CPU's RAM writes, in order.
#[derive(Default)]
struct Writes(Vec<(u16, u8)>);

impl Probe for Writes {
    fn mem_write(&mut self, address: u16, value: u8) {
        self.0.push((address, value));
    }
}

/// What to run on the CPU.
#[derive(Clone, Copy, Debug)]
enum Event {
    Instr(Instruction),
    /// INT V1, then the software interrupt entry.
    SoftInt(u8),
    /// A hardware interrupt on a port (the IOD handshake, then the entry).
    HardInt(u8),
}

/// The CPU state around one event.
struct Run {
    pc0: u16,
    sp0: u16,
    regs: [u8; 8],
    after: Cpu,
    writes: Vec<(u16, u8)>,
    /// The address bus value an I/O access or INT left (board.address_bus).
    io_addr: u16,
    ram0: Vec<u8>,
    /// RAM after the event (the IVT is read after the pushes).
    ram1: Vec<u8>,
}

fn run(e: Event, pc0: u16, sp0: u16, flags: u8, rng: &mut Rng) -> Run {
    MACHINE.with(|m| {
        let (cpu, board) = &mut *m.borrow_mut();
        board.reset();
        cpu.reset();
        for a in 0..=0xFFFFu16 {
            board.ram.write(a, rng.byte() as u8);
        }
        let i = match e {
            Event::Instr(i) => i,
            Event::SoftInt(p) => encode::imm8(Opcode::Int, p),
            Event::HardInt(_) => encode::none(Opcode::Nop),
        };
        for (k, b) in i.to_bytes().iter().enumerate() {
            board.ram.write(pc0.wrapping_add(k as u16), *b);
        }
        let regs: [u8; 8] = std::array::from_fn(|_| rng.byte() as u8);
        cpu.registers = regs;
        cpu.pc = pc0;
        cpu.sp = sp0;
        // I set (and H clear) so interrupts are taken; the rest random
        cpu.flags = Flags(flags & 0x7B | 0x04);
        let ram0 = board.ram.as_slice().to_vec();
        let mut w = Writes::default();
        match e {
            Event::Instr(_) => {
                cpu.tick_with(board, &mut w);
            }
            Event::SoftInt(_) => {
                assert!(cpu.tick_with(board, &mut w), "INT");
                assert!(cpu.pending_software_interrupt().is_some());
                let io = board.address_bus;
                assert!(cpu.tick_with(board, &mut w), "software interrupt entry");
                board.address_bus = io;
            }
            Event::HardInt(p) => {
                board.iod.trigger_interrupt(p);
                board.tick_iod();
                assert!(!cpu.tick_with(board, &mut w) && board.inr, "INR");
                board.tick_iod();
                assert!(cpu.tick_with(board, &mut w), "hardware interrupt entry");
            }
        }
        Run {
            pc0,
            sp0,
            regs,
            after: cpu.clone(),
            writes: w.0,
            io_addr: board.address_bus,
            ram0,
            ram1: board.ram.as_slice().to_vec(),
        }
    })
}

/// The bench: the model, its rows, and the IR / register values it presents.
struct Bench {
    m: Model,
    rows: Vec<Row>,
    a: u8,
    b: u8,
    v1: u8,
    v2: u8,
    what: String,
}

impl Bench {
    fn go(&mut self, s: Step) {
        let s = Step { a: self.a, b: self.b, v1: self.v1, v2: self.v2, ..s };
        self.m.step(&mut self.rows, s);
    }
    /// The address the latch shows in this step (memory or I/O access).
    fn at(&self, addr: u16, what: &str) {
        assert_eq!(self.m.lat, Some(addr), "{}: {what} address", self.what);
    }
    /// A step that writes a PC byte to memory: the byte on the data bus.
    fn push_pc(&mut self, hi: bool, w: (u16, u8), s: Step) {
        self.at(w.0, "push");
        assert_eq!(byte(self.m.pc, hi), Some(w.1), "{}: pushed PC byte", self.what);
        self.go(Step { drive: Some((1, hi as usize)), ..s });
    }
    /// End of a sequence: latch := PC (the next fetch), PC and SP on the data bus.
    fn finish(&mut self, c: &Run) {
        self.go(Step::add(Base::Pc, Off::Zero).ld(true, false, false));
        self.at(c.after.pc, "next fetch");
        for (r, v) in [(1, c.after.pc), (2, c.after.sp)] {
            for h in [1, 0] {
                self.go(Step { drive: Some((r, h)), ..Step::idle() });
            }
            assert_eq!(if r == 1 { self.m.pc } else { self.m.sp }, Some(v), "{}: PC / SP at the end", self.what);
        }
    }
    /// The reset line (C16), whatever p7 on the bus: PC and SP as the emulator's reset sets them.
    fn reset(&mut self, port: u8) {
        self.go(Step::reset(port));
        let cpu = Cpu::new();
        assert_eq!((self.m.pc, self.m.sp), (Some(cpu.pc), Some(cpu.sp)), "{}: reset", self.what);
        for (r, h) in [(1, 1), (1, 0), (2, 1), (2, 0)] {
            self.go(Step { drive: Some((r, h)), lat_oe: [false; 2], ..Step::idle() });
        }
    }
}

pub fn addr_cpu_sets() -> Vec<Vec<Row>> {
    use Opcode as O;
    let mut rng = Rng(0xB7E1_5162_8AED_2A6B);
    let mut sets = Vec::new();
    let r = |k: u8| Reg::from_bits(k);

    // instructions whose mode the manual doesn't list: the CPU stops on them
    let invalid: Vec<Instruction> = [
        encode::raw(O::Lod, AddrMode::Imm8, 1, 2, 0x12, 0x34),
        encode::raw(O::Jmp, AddrMode::Reg, 3, 4, 0x56, 0x78),
        encode::raw(O::Psh, AddrMode::Imm16, 5, 6, 0x9A, 0xBC),
        encode::raw(O::Ret, AddrMode::Reg, 0, 0, 0, 0),
        encode::raw(O::Asp, AddrMode::Reg16, 1, 1, 1, 1),
    ]
    .into_iter()
    .filter(|i| i.stops())
    .collect();
    assert!(invalid.len() >= 4, "invalid instructions: {invalid:?}");

    let corners: [(u16, u16); 6] =
        [(0x0000, 0xFE00), (0xFFFE, 0x0000), (0xFFFF, 0x0001), (0x7FFE, 0xFFFF), (0x00FC, 0xFE03), (0xFF00, 0x0100)];
    for k in 0..48 {
        let (pc0, sp0) = if k < corners.len() { corners[k] } else { (rand16(&mut rng), rand16(&mut rng)) };
        let (r1, r2, r3) = (rng.byte() as u8 & 7, rng.byte() as u8 & 7, rng.byte() as u8 & 7);
        let (v1, vx) = (rng.byte() as u8, rand16(&mut rng));
        let off = rng.byte() as u8;
        let mut events: Vec<(&str, Event)> = vec![
            ("nop", Event::Instr(encode::none(O::Nop))),
            ("mov reg", Event::Instr(encode::reg(O::Mov, r(r1), r(r2)))),
            ("mov imm", Event::Instr(encode::reg_imm8(O::Mov, r(r1), v1))),
            ("lod ram", Event::Instr(encode::reg_ram(O::Lod, r(r1), vx))),
            ("str ram", Event::Instr(encode::reg_ram(O::Str, r(r1), vx))),
            ("add ram", Event::Instr(encode::reg_ram(O::Add, r(r1), vx))),
            ("lod mode 4", Event::Instr(encode::ramreg_offset(O::Lod, r(r1), r(r2), r(r3), off))),
            ("str mode 4", Event::Instr(encode::ramreg_offset(O::Str, r(r1), r(r2), r(r3), off))),
            ("cmp mode 4", Event::Instr(encode::ramreg_offset(O::Cmp, r(r1), r(r2), r(r3), off))),
            ("lod stack", Event::Instr(encode::stack_offset(O::Lod, r(r1), v1))),
            ("str stack", Event::Instr(encode::stack_offset(O::Str, r(r1), v1))),
            ("sub stack", Event::Instr(encode::stack_offset(O::Sub, r(r1), v1))),
            ("inc imm16", Event::Instr(encode::imm16(O::Inc, vx))),
            ("not imm16", Event::Instr(encode::imm16(O::Not, vx))),
            ("dec mode 4", Event::Instr(encode::ramreg_offset(O::Dec, r(r1), r(r2), r(0), off))),
            ("psh", Event::Instr(encode::reg(O::Psh, r(r1), r(0)))),
            ("pop", Event::Instr(encode::reg(O::Pop, r(r1), r(0)))),
            ("psf", Event::Instr(encode::none(O::Psf))),
            ("pof", Event::Instr(encode::none(O::Pof))),
            ("cal imm16", Event::Instr(encode::imm16(O::Cal, vx))),
            ("cal reg16", Event::Instr(encode::reg16(O::Cal, r(r1), r(r2)))),
            ("ret", Event::Instr(encode::none(O::Ret))),
            ("irt", Event::Instr(encode::none(O::Irt))),
            ("jmp imm16", Event::Instr(encode::imm16(O::Jmp, vx))),
            ("jmp reg16", Event::Instr(encode::reg16(O::Jmp, r(r1), r(r2)))),
            ("jmz imm16", Event::Instr(encode::imm16(O::Jmz, vx))),
            ("jmc reg16", Event::Instr(encode::reg16(O::Jmc, r(r1), r(r2)))),
            ("asp imm8", Event::Instr(encode::imm8(O::Asp, v1))),
            ("ssp imm8", Event::Instr(encode::imm8(O::Ssp, v1))),
            ("asp imm16", Event::Instr(encode::imm16(O::Asp, vx))),
            ("ssp imm16", Event::Instr(encode::imm16(O::Ssp, vx))),
            ("wsp reg16", Event::Instr(encode::reg16(O::Wsp, r(r1), r(r2)))),
            ("wsp imm16", Event::Instr(encode::imm16(O::Wsp, vx))),
            ("rsp", Event::Instr(encode::reg16(O::Rsp, r(r1), r(r2)))),
            ("in", Event::Instr(encode::reg(O::In, r(r1), r(r2)))),
            ("out", Event::Instr(encode::reg(O::Out, r(r1), r(r2)))),
            ("hlt", Event::Instr(encode::none(O::Hlt))),
            ("int + software entry", Event::SoftInt(v1)),
            ("hardware entry", Event::HardInt(rng.byte() as u8)),
        ];
        events.push(("invalid", Event::Instr(invalid[k % invalid.len()])));
        for (what, e) in events {
            let flags = rng.byte() as u8;
            let c = run(e, pc0, sp0, flags, &mut rng);
            let what = format!("{what} pc={pc0:04X} sp={sp0:04X}");
            sets.push(sequence(&what, e, &c, &mut rng));
        }
    }
    sets
}

/// The control unit's steps for one event (addr-decisions.md section 2).
fn sequence(what: &str, e: Event, c: &Run, rng: &mut Rng) -> Vec<Row> {
    let mut b = steps(what, e, c, rng);
    b.reset(rng.byte() as u8 | if rng.byte() & 1 == 0 { 0x80 } else { 0 });
    b.rows
}

fn steps(what: &str, e: Event, c: &Run, rng: &mut Rng) -> Bench {
    use AddrMode as M;
    use Opcode as O;
    let mut b = Bench { m: Model::new(), rows: Vec::new(), a: 0, b: 0, v1: 0, v2: 0, what: what.to_string() };
    let reg = |k: u8| c.regs[k as usize];

    // start: PC and SP from V1:V2
    // (the latch and PC too in the first step, its outputs off: power-up)
    for (v, ld, lat_oe) in [(c.sp0, [true, true, true], [false; 2]), (c.pc0, [true, true, false], [true; 2])] {
        [b.v1, b.v2] = v.to_be_bytes();
        b.go(Step { ld, lat_oe, ..Step::add(Base::Off, Off::Vx) });
    }
    [b.v1, b.v2] = [rng.byte() as u8, rng.byte() as u8];

    // the fetch: n bytes, latch and PC + 1 per byte
    let fetch = |b: &mut Bench, n: u16| {
        for k in 0..n {
            b.at(c.pc0.wrapping_add(k), "fetch");
            b.go(Step::plus1(Base::Pc).ld(true, true, false));
        }
    };
    // the interrupt entry (Q2): push I, PC low, PC high, then the vector
    let entry = |b: &mut Bench, port: u8, software: bool| {
        let w = &c.writes;
        assert_eq!(w.len(), 3, "{what}: entry pushes");
        b.go(Step::minus1(Base::Sp).ld(true, false, true));
        b.at(w[0].0, "push I");
        b.go(Step::minus1(Base::Sp).ld(true, false, true));
        b.push_pc(false, w[1], Step::minus1(Base::Sp).ld(true, false, true));
        b.push_pc(true, w[2], Step::add(Base::Off, Off::Zero).ld(true, false, false)); // latch := 0000
        assert_eq!(b.m.sp, Some(c.after.sp), "{what}: SP after the pushes");
        if !software {
            b.go(Step::idle()); // INR, the IOD answers
        }
        // vector step: the IOD (hardware) or the pending port latch (software) drives p
        let lat_oe = if software { [false, true] } else { [false, false] };
        b.go(Step { lat_oe, port: Some(port), ..Step::add(Base::Ivt, Off::Zero).ld(true, true, false) });
        let ivt = IVT_START.wrapping_add(2 * port as u16);
        b.at(ivt, "vector, high byte");
        b.go(Step::plus1(Base::Pc).ld(true, false, false));
        b.at(ivt.wrapping_add(1), "vector, low byte");
        b.v1 = c.ram1[ivt as usize];
        b.go(Step::idle());
        b.v2 = c.ram1[ivt.wrapping_add(1) as usize];
        b.go(Step::add(Base::Off, Off::Vx).ld(true, true, false));
    };

    let i = match e {
        Event::Instr(i) => i,
        Event::SoftInt(p) => encode::imm8(O::Int, p),
        Event::HardInt(p) => {
            entry(&mut b, p, false);
            b.finish(c);
            return b;
        }
    };
    let n = i.size();
    if i.stops() {
        // fetched, then the control unit undoes the increments (Q5)
        fetch(&mut b, n);
        for _ in 0..n {
            b.go(Step::minus1(Base::Pc).ld(false, true, false));
        }
        assert!(c.after.is_halted());
        b.finish(c);
        return b;
    }
    fetch(&mut b, n);
    let (op, mode) = (i.opcode().unwrap(), i.mode().unwrap());
    [b.v1, b.v2] = [i.v1(), i.v2()];
    let (r1, r2) = (reg(i.r1()), reg(i.r2()));
    let ram = |a: u16| c.ram0[a as usize];
    // one memory access at an address computed in one step, the latch
    // holding it for `steps` steps (read / ALU / write)
    let access = |b: &mut Bench, s: Step, addr: u16, steps: usize| {
        b.go(s.ld(true, false, false));
        b.at(addr, "operand");
        for _ in 0..steps {
            b.go(Step::idle());
        }
    };
    let pointer = u16::from_be_bytes([r1, r2]).wrapping_add(i.v2() as u16);
    let stack = c.sp0.wrapping_add(i.v1() as u16);
    match (op, mode) {
        (O::Nop | O::Mov | O::Hlt, _) => {}
        (O::Rsp, M::Reg16) => {
            b.go(Step { drive: Some((2, 1)), ..Step::idle() });
            b.go(Step { drive: Some((2, 0)), ..Step::idle() });
            let [hi, lo] = c.sp0.to_be_bytes();
            assert_eq!(c.after.registers[i.r2() as usize], lo, "{what}: RSP low");
            if i.r1() != i.r2() {
                assert_eq!(c.after.registers[i.r1() as usize], hi, "{what}: RSP high");
            }
        }
        (O::Psh | O::Psf, _) => {
            b.go(Step::minus1(Base::Sp).ld(true, false, true));
            b.at(c.writes[0].0, "push");
            b.go(Step::idle());
        }
        (O::Pop | O::Pof, _) => {
            b.go(Step::add(Base::Sp, Off::Zero).ld(true, false, false));
            b.at(c.sp0, "pop");
            if op == O::Pop {
                assert_eq!(c.after.registers[i.r1() as usize], ram(c.sp0), "{what}: popped byte");
            }
            b.go(Step::plus1(Base::Sp).ld(false, false, true));
        }
        (O::Cal, _) => {
            let target = if mode == M::Imm16 { Step::add(Base::Off, Off::Vx) } else { Step::add(Base::Regs, Off::Zero) };
            [b.a, b.b] = [r1, r2];
            assert_eq!(c.writes.len(), 2, "{what}: CAL pushes");
            b.go(Step::minus1(Base::Sp).ld(true, false, true));
            b.push_pc(false, c.writes[0], Step::minus1(Base::Sp).ld(true, false, true));
            b.push_pc(true, c.writes[1], target.ld(true, true, false));
        }
        (O::Ret | O::Irt, _) => {
            b.go(Step::add(Base::Sp, Off::Zero).ld(true, false, false));
            b.at(c.sp0, "return address, high");
            b.go(Step::plus1(Base::Sp).ld(true, false, true));
            b.at(c.sp0.wrapping_add(1), "return address, low");
            b.v1 = ram(c.sp0);
            if op == O::Irt {
                b.go(Step::plus1(Base::Sp).ld(true, false, true));
                b.at(c.sp0.wrapping_add(2), "I");
                assert_eq!(c.after.registers[Reg::I as usize], ram(c.sp0.wrapping_add(2)), "{what}: popped I");
            }
            b.go(Step::plus1(Base::Sp).ld(false, false, true));
            b.v2 = ram(c.sp0.wrapping_add(1));
            b.go(Step::add(Base::Off, Off::Vx).ld(true, true, false));
        }
        (O::Jmp | O::Jmc | O::Jme | O::Jmn | O::Jms | O::Jmz | O::Jmf, _) => {
            if c.after.pc != c.pc0.wrapping_add(n) {
                [b.a, b.b] = [r1, r2];
                let target = if mode == M::Imm16 { Step::add(Base::Off, Off::Vx) } else { Step::add(Base::Regs, Off::Zero) };
                b.go(target.ld(true, true, false));
            }
        }
        (O::Asp | O::Ssp, _) => {
            let off = if mode == M::Imm8 { Off::V1 } else { Off::Vx };
            let s = if op == O::Asp { Step::add(Base::Sp, off) } else { Step::sub(Base::Sp, off) };
            b.go(s.ld(false, false, true));
        }
        (O::Wsp, _) => {
            [b.a, b.b] = [r1, r2];
            let s = if mode == M::Imm16 { Step::add(Base::Off, Off::Vx) } else { Step::add(Base::Regs, Off::Zero) };
            b.go(s.ld(false, false, true));
        }
        (O::In | O::Out, _) => {
            b.b = if op == O::In { r2 } else { r1 }; // the port is on bus B
            access(&mut b, Step::add(Base::RegsLo, Off::Zero), c.io_addr, 1);
        }
        (O::Int, _) => {
            access(&mut b, Step::add(Base::Off, Off::V1), c.io_addr, 1);
            let Event::SoftInt(p) = e else { unreachable!() };
            entry(&mut b, p, true);
        }
        (_, _) => {
            // an operand in memory: Vx, R1:R2 + V2 or SP + V1
            [b.a, b.b] = [r1, r2];
            let (s, addr) = match mode {
                M::RegRam | M::Imm16 => (Step::add(Base::Off, Off::Vx), i.vx()),
                M::RamRegImmReg => (Step::add(Base::Regs, Off::V2), pointer),
                M::StackOffset => (Step::add(Base::Sp, Off::V1), stack),
                _ => unreachable!("{what}: {mode:?}"),
            };
            let rmw = matches!(op, O::Inc | O::Dec | O::Not);
            access(&mut b, s, addr, if rmw { 3 } else { 1 });
            // the emulator used that address
            match op {
                O::Str => assert_eq!(c.writes, vec![(addr, reg(if mode == M::RamRegImmReg { i.r3() } else { i.r1() }))]),
                O::Lod => {
                    let d = if mode == M::RamRegImmReg { i.r3() } else { i.r1() };
                    assert_eq!(c.after.registers[d as usize], ram(addr), "{what}: loaded byte");
                }
                _ if rmw => assert_eq!(c.writes.len(), 1) ,
                _ => {}
            }
            if rmw {
                assert_eq!(c.writes[0].0, addr, "{what}: written address");
            }
        }
    }
    b.finish(c);
    b
}

pub const MODULES: &[SeqModule] = &[
    SeqModule { name: "addr", inputs: ADDR_IN, outputs: ADDR_OUT, buses: ADDR_BUSES, sets: addr_sets },
    SeqModule { name: "addr_cpu", inputs: ADDR_IN, outputs: ADDR_OUT, buses: ADDR_BUSES, sets: addr_cpu_sets },
];
