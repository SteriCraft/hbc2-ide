//! The memory section (logisim/notes/ir-mem-decisions.md section 4, M1-M4).
//!
//! - `mem`: the boot copy of the whole 64 KiB ROM into RAM, then random
//!   reads, writes and idle steps, then reads of random addresses. The RAM is
//!   the emulator's own `Ram` (the board's memory); the ROM holds the
//!   at28c256 model's test pattern (designs/at28c256.py `pattern`).
//!
//! One step = 3 rows: the step's inputs with clk_mem_wr_n high, then the
//! strobe low (the RAM writes if mem_we_n is low), then high again. The
//! address only changes in the first row, so it is stable around the strobe.
//! Both outputs are buses: every row checks them (value, Z, or U for a RAM
//! cell never written since power-up).

use super::{Out, Row, Rng, SeqModule};
use hbc2_core::board::Board;

pub const MEM_IN: &[(&str, u32)] = &[
    ("clk_mem_wr_n", 1),
    ("mem_re", 1),
    ("mem_oe_n", 1),
    ("mem_we_n", 1),
    ("mem_wbuf_oe_n", 1),
    ("mem_rom_oe_n", 1),
    ("mem_addr", 16),
    ("mem_bus_in", 8),
];
pub const MEM_OUT: &[(&str, u32)] = &[("mem_bus_out", 8), ("mem_d", 8)];
const MEM_BUSES: &[&str] = &["mem_bus_out", "mem_d"];

/// The at28c256 model's default image: low half, then the high half inverted.
pub fn rom_pattern(a: u16) -> u8 {
    let x = (a & 0x7FFF) as u32;
    let lo = ((x * 37 + (x >> 8) * 11 + 0x5A) & 0xFF) as u8;
    if a & 0x8000 != 0 { !lo } else { lo }
}

/// What a step does (the allowed strobe combinations, C19).
#[derive(Clone, Copy)]
pub enum Kind {
    Idle,
    Read,
    Write,
    Boot,
}

pub struct Model {
    pub board: Board,
    /// RAM cells written since power-up.
    pub known: Vec<bool>,
    pub rom: fn(u16) -> u8,
}

impl Model {
    pub fn new(rom: fn(u16) -> u8) -> Model {
        Model { board: Board::default(), known: vec![false; 1 << 16], rom }
    }

    /// The strobes of a step: re, oe_n, we_n, wbuf_oe_n, rom_oe_n.
    pub fn strobes(k: Kind) -> [u32; 5] {
        match k {
            Kind::Idle => [0, 1, 1, 1, 1],
            Kind::Read => [1, 0, 1, 1, 1],
            Kind::Write => [1, 1, 0, 0, 1],
            Kind::Boot => [1, 1, 0, 1, 0],
        }
    }

    /// (mem_bus_out, mem_d) for a step, `writing` = the strobe is low.
    pub fn outs(&self, k: Kind, a: u16, d: u8) -> [Out; 2] {
        let cell = if self.known[a as usize] { Out::V(self.board.ram.read(a) as u32) } else { Out::U };
        match k {
            Kind::Idle => [Out::Z, Out::Z],
            Kind::Read => [cell, cell],
            Kind::Write => [Out::Z, Out::V(d as u32)],
            Kind::Boot => [Out::Z, Out::V((self.rom)(a) as u32)],
        }
    }

    /// The 3 rows of one step; the RAM writes while the strobe is low.
    pub fn step(&mut self, rows: &mut Vec<Row>, k: Kind, a: u16, d: u8) {
        let s = Model::strobes(k);
        let ins = |strobe: u32| vec![strobe, s[0], s[1], s[2], s[3], s[4], a as u32, d as u32];
        rows.push((ins(1), self.outs(k, a, d).to_vec()));
        match k {
            Kind::Write => {
                self.board.ram.write(a, d);
                self.known[a as usize] = true;
            }
            Kind::Boot => {
                self.board.ram.write(a, (self.rom)(a));
                self.known[a as usize] = true;
            }
            _ => {}
        }
        rows.push((ins(0), self.outs(k, a, d).to_vec()));
        rows.push((ins(1), self.outs(k, a, d).to_vec()));
    }
}

fn mem_sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0x6C10_08AB_28C2_5600);
    let mut m = Model::new(rom_pattern);
    let mut rows = Vec::new();
    let byte = |r: &mut Rng| r.byte() as u8;
    let addr = |r: &mut Rng| (r.byte() << 8 | r.byte()) as u16;
    // power-up: nothing written, a read shows an unknown cell
    m.step(&mut rows, Kind::Idle, 0, byte(&mut rng));
    m.step(&mut rows, Kind::Read, 0x1234, byte(&mut rng));
    // the boot copy (C20): every address, ROM -> RAM
    for a in 0..=0xFFFFu16 {
        m.step(&mut rows, Kind::Boot, a, byte(&mut rng));
    }
    // random steps; writes near the previous address too
    let mut last = 0u16;
    for _ in 0..6000 {
        let a = if rng.byte() & 1 == 0 { addr(&mut rng) } else { last ^ (1 << (rng.byte() % 16)) };
        let k = match rng.byte() % 4 {
            0 => Kind::Idle,
            1 => Kind::Write,
            _ => Kind::Read,
        };
        m.step(&mut rows, k, a, byte(&mut rng));
        last = a;
    }
    for _ in 0..4000 {
        let a = addr(&mut rng);
        m.step(&mut rows, Kind::Read, a, byte(&mut rng));
    }
    vec![rows]
}

pub const MODULES: &[SeqModule] =
    &[SeqModule { name: "mem", inputs: MEM_IN, outputs: MEM_OUT, buses: MEM_BUSES, sets: mem_sets }];
