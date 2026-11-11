//! `mem_cpu`: the address section, the IR and the memory section together
//! (logisim/circuits/mem/test_mem_cpu.circ, notes/ir-mem-decisions.md M5, M6).
//!
//! The real CPU runs a random program (every opcode but HLT and INT, random
//! modes and operands, invalid opcodes included, with hardware interrupts
//! queued at random times). The rows drive the three sections the way the
//! control unit will:
//!
//! - reset (C16), then the boot loop (C20): latch := PC, then 65,536 steps
//!   latch, PC := PC + 1 copying ROM -> RAM, until addr_cout;
//! - each instruction: its bytes fetched from the Logisim RAM into the IR
//!   (C18), every RAM write the CPU made mirrored (address through bus A:B,
//!   the data from `cpu_data`, the register section's stand-in), IN / OUT
//!   through the IOD stub (C22), then latch, PC := the CPU's next PC (bus A:B:
//!   the jump logic is the control unit's, not tested here);
//! - each hardware interrupt: the three pushes, the vector step (the stub
//!   drives the port and the data, C21), V1 and V2 read from the IVT in RAM
//!   into the IR, then latch, PC := V1:V2 through the IR's outputs, checked
//!   against the CPU's handler address.
//!
//! The ROM image is the program's power-on RAM (`Machine::power_on` copies
//! it as is); the bench writes it into the at28c256 model, so
//! the generator writes it to $HBC2_ROM_OUT (designs/test_mem_cpu.py does).
//! One step = 4 rows: clk_mem_wr_n high, low (the RAM writes), high, then
//! the rising clk edge.

use super::mem::Kind;
use super::{Out, Row, Rng, SeqModule};
use hbc2_core::isa::{IVT_START, Instruction, Opcode, encode};
use hbc2_core::trace::Probe;
use hbc2_core::{Board, Cpu};

pub const IN: &[(&str, u32)] = &[
    ("clk", 1),
    ("rst_n", 1),
    ("clk_mem_wr_n", 1),
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
    ("ir_b0_ld_n", 1),
    ("ir_b1_ld_n", 1),
    ("ir_v1_ld_n", 1),
    ("ir_v2_ld_n", 1),
    ("mem_re", 1),
    ("mem_oe_n", 1),
    ("mem_we_n", 1),
    ("mem_wbuf_oe_n", 1),
    ("mem_rom_oe_n", 1),
    ("iod_port", 8),
    ("iod_port_oe_n", 1),
    ("iod_data", 8),
    ("iod_data_oe_n", 1),
    ("cpu_data", 8),
    ("cpu_data_oe_n", 1),
];
pub const OUT: &[(&str, u32)] = &[
    ("bus_data", 8),
    ("bus_addr_hi", 8),
    ("bus_addr_lo", 8),
    ("mem_d", 8),
    ("addr_cout", 1),
    ("ir_op", 6),
    ("ir_mode", 4),
    ("ir_v1", 8),
    ("ir_v2", 8),
];
const BUSES: &[&str] = &["bus_data", "bus_addr_hi", "bus_addr_lo", "mem_d"];

/// BASE selects (addr_base_sel).
const PC: u8 = 0;
const REGS: u8 = 2;
const IVT: u8 = 3;

/// One step's control word.
#[derive(Clone, Copy)]
struct St {
    rst: bool,
    base: u8,
    /// [low, high] byte enabled.
    base_oe: [bool; 2],
    off_v1_lo: bool,
    off_oe: [bool; 2],
    cin: bool,
    ld_lat: bool,
    ld_pc: bool,
    lat_oe: [bool; 2],
    a: u8,
    b: u8,
    ir_ld: [bool; 4],
    mem: Kind,
    iod_port: Option<u8>,
    iod_data: Option<u8>,
    cpu_data: Option<u8>,
}

impl St {
    fn idle() -> St {
        St {
            rst: false,
            base: PC,
            base_oe: [true, true],
            off_v1_lo: false,
            off_oe: [false, false],
            cin: false,
            ld_lat: false,
            ld_pc: false,
            lat_oe: [true, true],
            a: 0,
            b: 0,
            ir_ld: [false; 4],
            mem: Kind::Idle,
            iod_port: None,
            iod_data: None,
            cpu_data: None,
        }
    }
    /// latch, PC := PC + 1 (the fetch, the boot copy).
    fn inc(ld_lat: bool, ld_pc: bool) -> St {
        St { cin: true, ld_lat, ld_pc, ..St::idle() }
    }
    /// latch (and PC) := a:b through bus A:B.
    fn set(v: u16, ld_pc: bool) -> St {
        St { base: REGS, a: (v >> 8) as u8, b: v as u8, ld_lat: true, ld_pc, ..St::idle() }
    }
}

fn v(x: Option<u8>) -> Out {
    x.map_or(Out::U, |b| Out::V(b as u32))
}

struct Model {
    lat: Option<u16>,
    pc: Option<u16>,
    ram: Vec<Option<u8>>,
    ir: [Option<u8>; 4],
    rom: Vec<u8>,
    rows: Vec<Row>,
}

impl Model {
    fn bus_addr(&self, s: &St) -> (Out, Out, Option<u8>) {
        let hi = if s.lat_oe[1] { v(self.lat.map(|l| (l >> 8) as u8)) } else { Out::Z };
        let lo_v = if s.lat_oe[0] { self.lat.map(|l| l as u8) } else { s.iod_port };
        let lo = if s.lat_oe[0] || s.iod_port.is_some() { v(lo_v) } else { Out::Z };
        (hi, lo, lo_v)
    }
    fn cell(&self) -> Option<u8> {
        self.lat.and_then(|a| self.ram[a as usize])
    }
    fn bus_data(&self, s: &St) -> (Out, Option<u8>) {
        match (s.mem, s.cpu_data, s.iod_data) {
            (Kind::Read, None, None) => (v(self.cell()), self.cell()),
            (_, Some(d), None) | (_, None, Some(d)) => (Out::V(d as u32), Some(d)),
            (_, None, None) => (Out::Z, None),
            _ => panic!("two drivers on bus_data"),
        }
    }
    fn mem_d(&self, s: &St) -> Out {
        match s.mem {
            Kind::Idle => Out::Z,
            Kind::Read => v(self.cell()),
            Kind::Write => v(s.cpu_data),
            Kind::Boot => v(self.lat.map(|a| self.rom[a as usize])),
        }
    }
    fn sum17(&self, s: &St, p: Option<u8>) -> Option<u32> {
        let src = match s.base {
            PC => self.pc,
            REGS => Some(u16::from_be_bytes([s.a, s.b])),
            IVT if s.rst => Some(0xFE00),
            IVT => p.map(|p| u16::from_be_bytes([0xFE | p >> 7, p << 1])),
            _ => unreachable!(),
        };
        let mask = if s.base_oe[1] { 0xFF00 } else { 0 } | if s.base_oe[0] { 0x00FF } else { 0 };
        let base = if mask == 0 { Some(0) } else { src.map(|x| x & mask) };
        let hi = if s.off_oe[1] { self.ir[2] } else { Some(0) };
        let lo = if !s.off_oe[0] {
            Some(0)
        } else if s.off_v1_lo {
            self.ir[2]
        } else {
            self.ir[3]
        };
        let off = hi.zip(lo).map(|(h, l)| u16::from_be_bytes([h, l]));
        base.zip(off).map(|(b, o)| b as u32 + o as u32 + s.cin as u32)
    }
    fn outs(&self, s: &St) -> Vec<Out> {
        let (hi, lo, p) = self.bus_addr(s);
        let (data, _) = self.bus_data(s);
        let i = Instruction(u32::from_be_bytes(self.ir.map(|x| x.unwrap_or(0))));
        let known = |b: &[usize], x: u8| if b.iter().all(|&k| self.ir[k].is_some()) { Out::V(x as u32) } else { Out::U };
        vec![
            data,
            hi,
            lo,
            self.mem_d(s),
            self.sum17(s, p).map_or(Out::U, |x| Out::V(x >> 16)),
            known(&[0], i.opcode_bits()),
            known(&[0, 1], i.mode_bits()),
            known(&[2], i.v1()),
            known(&[3], i.v2()),
        ]
    }
    fn ins(s: &St, clk: u32, strobe: u32) -> Vec<u32> {
        let oe = |e: [bool; 2]| (!e[1] as u32) << 1 | !e[0] as u32;
        let m = super::mem::Model::strobes(s.mem);
        vec![
            clk,
            !s.rst as u32,
            strobe,
            s.base as u32,
            oe(s.base_oe),
            s.off_v1_lo as u32,
            oe(s.off_oe),
            0,
            s.cin as u32,
            !s.ld_lat as u32,
            !s.ld_pc as u32,
            1,
            oe(s.lat_oe),
            3,
            3,
            s.a as u32,
            s.b as u32,
            !s.ir_ld[0] as u32,
            !s.ir_ld[1] as u32,
            !s.ir_ld[2] as u32,
            !s.ir_ld[3] as u32,
            m[0],
            m[1],
            m[2],
            m[3],
            m[4],
            s.iod_port.unwrap_or(0) as u32,
            s.iod_port.is_none() as u32,
            s.iod_data.unwrap_or(0) as u32,
            s.iod_data.is_none() as u32,
            s.cpu_data.unwrap_or(0) as u32,
            s.cpu_data.is_none() as u32,
        ]
    }
    fn step(&mut self, s: St) {
        self.rows.push((Model::ins(&s, 0, 1), self.outs(&s)));
        if let Some(a) = self.lat {
            match s.mem {
                Kind::Write => self.ram[a as usize] = s.cpu_data,
                Kind::Boot => self.ram[a as usize] = Some(self.rom[a as usize]),
                _ => {}
            }
        }
        self.rows.push((Model::ins(&s, 0, 0), self.outs(&s)));
        self.rows.push((Model::ins(&s, 0, 1), self.outs(&s)));
        // the edge
        let (_, _, p) = self.bus_addr(&s);
        let sum = self.sum17(&s, p).map(|x| x as u16);
        let (_, data) = self.bus_data(&s);
        for k in 0..4 {
            if s.ir_ld[k] {
                self.ir[k] = data;
            }
        }
        if s.ld_lat {
            self.lat = sum;
        }
        if s.ld_pc {
            self.pc = sum.map(|x| if s.rst { x & 0x01FF } else { x });
        }
        self.rows.push((Model::ins(&s, 1, 1), self.outs(&s)));
    }
    /// A RAM write of the CPU at `a`: latch := a, then the write step.
    fn write(&mut self, a: u16, d: u8, pc: u16) {
        self.step(St::set(a, false));
        self.step(St { mem: Kind::Write, cpu_data: Some(d), ..St::idle() });
        let _ = pc;
    }
}

/// What the CPU did in one tick.
#[derive(Default)]
struct Rec {
    writes: Vec<(u16, u8)>,
    out: Option<(u8, u8)>,
    instr: Option<(u16, Instruction)>,
    interrupt: bool,
}

impl Probe for Rec {
    fn mem_write(&mut self, address: u16, value: u8) {
        self.writes.push((address, value));
    }
    fn io_write(&mut self, port: u8, value: u8) {
        self.out = Some((port, value));
    }
    fn instruction(&mut self, _cpu: &Cpu, pc: u16, ir: Instruction) {
        self.instr = Some((pc, ir));
    }
    fn interrupt(&mut self, _cpu: &Cpu, _interrupted: u16) {
        self.interrupt = true;
    }
}

/// A random straight-line program at 0000 (STI first): every opcode but the
/// ones that would make it run off into data (jumps, CAL, RET, IRT), stop it
/// (HLT, a mode the opcode doesn't take, POF: random flags, H included), take a software interrupt (INT:
/// the pending port latch isn't built), move SP (ASP, SSP, WSP) or write
/// RAM at a random address (STR, INC / DEC / NOT on memory): PSH, PSF and the
/// interrupt pushes still write RAM. Invalid opcodes (1-byte NOPs) included.
/// Handlers for ports 1-8 start on instruction boundaries; the IVT points there.
fn program(rng: &mut Rng) -> Vec<u8> {
    use hbc2_core::isa::AddrMode;
    let mut image = vec![0u8; 1 << 16];
    let mut at = 0usize;
    let mut starts = Vec::new();
    let mut put = |image: &mut Vec<u8>, at: &mut usize, i: Instruction| {
        starts.push(*at as u16);
        for &b in i.to_bytes().iter().take(i.size() as usize) {
            image[*at] = b;
            *at += 1;
        }
    };
    put(&mut image, &mut at, encode::none(Opcode::Sti));
    while at < 0x0600 {
        let word = rng.byte() << 24 | rng.byte() << 16 | rng.byte() << 8 | rng.byte();
        let i = Instruction(word);
        use Opcode as O;
        let bad = match i.opcode() {
            Some(O::Jmc | O::Jme | O::Jmn | O::Jmp | O::Jms | O::Jmz | O::Jmf | O::Cal | O::Ret | O::Irt) => true,
            Some(O::Hlt | O::Int | O::Asp | O::Ssp | O::Wsp | O::Str | O::Pof) => true,
            Some(O::Inc | O::Dec | O::Not) => i.mode() != Some(AddrMode::Reg),
            _ => false,
        };
        if i.stops() || bad {
            continue;
        }
        put(&mut image, &mut at, i);
    }
    for p in 1..=8u16 {
        let h = starts[(rng.byte() as usize * starts.len()) >> 8];
        image[(IVT_START + 2 * p) as usize] = (h >> 8) as u8;
        image[(IVT_START + 2 * p + 1) as usize] = h as u8;
    }
    image
}

fn sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0x0C0F_FEE0_1234_ABCD);
    let image = program(&mut rng);
    if let Ok(path) = std::env::var("HBC2_ROM_OUT") {
        std::fs::write(&path, &image).expect("HBC2_ROM_OUT");
    }
    let mut m = Model { lat: None, pc: None, ram: vec![None; 1 << 16], ir: [None; 4], rom: image.clone(), rows: vec![] };

    // reset (C16): two steps of the reset word, every bus undriven
    let reset = St {
        rst: true,
        base: IVT,
        base_oe: [false, true],
        ld_pc: true,
        lat_oe: [false, false],
        ..St::idle()
    };
    m.step(reset);
    m.step(reset);
    assert_eq!(m.pc, Some(0));
    // the boot loop (C20)
    m.step(St { ld_lat: true, ..St::idle() });
    loop {
        let end = m.sum17(&St::inc(true, true), None).map(|x| x >> 16) == Some(1);
        m.step(St { mem: Kind::Boot, ..St::inc(true, true) });
        if end {
            break;
        }
    }
    assert_eq!((m.lat, m.pc), (Some(0), Some(0)));
    assert!(m.ram.iter().zip(&image).all(|(r, i)| *r == Some(*i)), "boot copy");

    // the emulator: Machine::power_on, then the program
    let mut cpu = Cpu::new();
    let mut board = Board::default();
    cpu.reset();
    board.reset();
    board.ram.as_mut_slice().copy_from_slice(&image);
    let mut taken = 0;
    for _ in 0..4000 {
        if cpu.is_stopped() {
            break;
        }
        if rng.byte() < 8 {
            let p = 1 + (rng.byte() % 8) as u8;
            board.iod.set_port(p, rng.byte() as u8);
            board.iod.trigger_interrupt(p);
        }
        let regs = cpu.registers;
        let buses = (board.address_bus as u8, board.data_bus);
        let mut rec = Rec::default();
        cpu.tick_with(&mut board, &mut rec);
        board.tick_iod();
        if rec.interrupt {
            for &(a, d) in &rec.writes {
                m.write(a, d, cpu.pc);
            }
            let (p, d) = buses;
            m.step(St {
                base: IVT,
                base_oe: [true, true],
                ld_lat: true,
                ld_pc: true,
                lat_oe: [false, false],
                iod_port: Some(p),
                iod_data: Some(d),
                ..St::idle()
            });
            m.step(St { mem: Kind::Read, ir_ld: [false, false, true, false], ..St::inc(true, false) });
            m.step(St { mem: Kind::Read, ir_ld: [false, false, false, true], ..St::idle() });
            m.step(St { base_oe: [false, false], off_oe: [true, true], ld_lat: true, ld_pc: true, ..St::idle() });
            assert_eq!(m.pc, Some(cpu.pc), "handler address, port {p}");
            taken += 1;
            continue;
        }
        let Some((pc, ir)) = rec.instr else { continue };
        assert_eq!(m.pc, Some(pc), "PC before the fetch");
        for k in 0..ir.size() as usize {
            let mut ld = [false; 4];
            ld[k] = true;
            m.step(St { mem: Kind::Read, ir_ld: ld, ..St::inc(true, true) });
        }
        let mask = [0, 0xFF00_0000, 0xFFFF_0000, 0xFFFF_FF00, 0xFFFF_FFFF][ir.size() as usize];
        let held = u32::from_be_bytes(m.ir.map(|x| x.unwrap_or(0)));
        assert_eq!(held & mask, ir.0 & mask, "IR at {pc:04x}");
        match ir.opcode() {
            Some(Opcode::In) => {
                let port = regs[ir.r2() as usize];
                m.step(St::set(port as u16, false));
                m.step(St { iod_data: Some(cpu.registers[ir.r1() as usize]), ..St::idle() });
            }
            Some(Opcode::Out) => {
                let (port, val) = rec.out.expect("OUT without a port write");
                m.step(St::set(port as u16, false));
                m.step(St { cpu_data: Some(val), ..St::idle() });
            }
            _ => {}
        }
        for &(a, d) in &rec.writes {
            m.write(a, d, cpu.pc);
        }
        m.step(St::set(cpu.pc, true));
    }
    eprintln!("mem_cpu: stopped {} at pc {:04x}, ir {:08x}", cpu.is_stopped(), cpu.pc, cpu.ir.0);
    assert!(taken > 0, "no hardware interrupt was taken");
    eprintln!("mem_cpu: {taken} hardware interrupts taken");
    vec![m.rows]
}

pub const MODULES: &[SeqModule] = &[SeqModule { name: "mem_cpu", inputs: IN, outputs: OUT, buses: BUSES, sets }];
