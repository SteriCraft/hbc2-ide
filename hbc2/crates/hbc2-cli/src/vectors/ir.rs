//! The instruction register (logisim/notes/ir-mem-decisions.md section 2).
//!
//! - `ir`: random sequences of byte loads (any set of the four enables, the
//!   same byte into several registers at once), every field checked with the
//!   emulator's own `Instruction` accessors on the bytes loaded so far.
//! - `ir_cpu`: every opcode x every mode bit pattern, with random operand
//!   bits. The real CPU runs the instruction; the rows load exactly the bytes
//!   it read (`Instruction::size`: 1 for an invalid opcode or a no-operand
//!   opcode, the mode's length, 2 for a mode the opcode doesn't take), the way
//!   the control unit will (C18), and the bytes past the size keep the
//!   previous instruction's values (checked: the hardware doesn't clear them).
//!   Between instructions, RET / IRT / interrupt entries reload V1 and V2
//!   alone (C14).
//!
//! One clock step = 2 rows (clk low, then the rising edge).

use super::{MACHINE, Out, Row, Rng, SeqModule};
use hbc2_core::isa::{Instruction, PROGRAM_START};

pub const IR_IN: &[(&str, u32)] = &[
    ("clk", 1),
    ("ir_b0_ld_n", 1),
    ("ir_b1_ld_n", 1),
    ("ir_v1_ld_n", 1),
    ("ir_v2_ld_n", 1),
    ("ir_bus_in", 8),
];
pub const IR_OUT: &[(&str, u32)] = &[
    ("ir_op", 6),
    ("ir_mode", 4),
    ("ir_r1", 3),
    ("ir_r2", 3),
    ("ir_r3", 3),
    ("ir_r1_2_n", 1),
    ("ir_r2_2_n", 1),
    ("ir_r3_2_n", 1),
    ("ir_v1", 8),
    ("ir_v2", 8),
];
/// The IR drives no bus (its outputs are plain register outputs).
const IR_BUSES: &[&str] = &[];

/// The four IR bytes as last loaded (None: never loaded since power-up).
struct Model {
    b: [Option<u8>; 4],
    rows: Vec<Row>,
}

impl Model {
    fn new() -> Model {
        Model { b: [None; 4], rows: Vec::new() }
    }

    /// Every output, from the emulator's accessors on the bytes held.
    fn outs(&self) -> Vec<Out> {
        let i = Instruction(u32::from_be_bytes(self.b.map(|x| x.unwrap_or(0))));
        let known = |bytes: &[usize], v: u8| {
            if bytes.iter().all(|&k| self.b[k].is_some()) { Out::V(v as u32) } else { Out::U }
        };
        let not2 = |r: u8| (!r >> 2) & 1;
        vec![
            known(&[0], i.opcode_bits()),
            known(&[0, 1], i.mode_bits()),
            known(&[1], i.r1()),
            known(&[1], i.r2()),
            known(&[2], i.r3()),
            known(&[1], not2(i.r1())),
            known(&[1], not2(i.r2())),
            known(&[2], not2(i.r3())),
            known(&[2], i.v1()),
            known(&[3], i.v2()),
        ]
    }

    /// One step: `ld[k]` loads `bus` into byte k on the edge.
    fn step(&mut self, ld: [bool; 4], bus: u8) {
        let ins = |clk: u32| {
            vec![clk, !ld[0] as u32, !ld[1] as u32, !ld[2] as u32, !ld[3] as u32, bus as u32]
        };
        self.rows.push((ins(0), self.outs()));
        for k in 0..4 {
            if ld[k] {
                self.b[k] = Some(bus);
            }
        }
        self.rows.push((ins(1), self.outs()));
    }
}

fn ir_sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0x1A2B_3C4D_5E6F_7081);
    let mut sets = Vec::new();
    for _ in 0..8 {
        let mut m = Model::new();
        m.step([false; 4], rng.byte() as u8); // power-up: nothing known
        for _ in 0..1000 {
            let e = rng.byte();
            let ld = [e & 1 != 0, e & 2 != 0, e & 4 != 0, e & 8 != 0];
            m.step(ld, rng.byte() as u8);
        }
        // every byte value through every register, alone
        for k in 0..4 {
            for v in 0..=255u8 {
                let mut ld = [false; 4];
                ld[k] = true;
                m.step(ld, v);
            }
        }
        sets.push(m.rows);
    }
    sets
}

/// The real CPU runs `word` at PC; returns the IR it holds and the bytes it read.
fn run_cpu(word: u32, rng: &mut Rng) -> (Instruction, usize) {
    MACHINE.with(|m| {
        let (cpu, board) = &mut *m.borrow_mut();
        board.reset();
        cpu.reset();
        let pc = PROGRAM_START.wrapping_add(0x100 + rng.byte() as u16);
        for (k, b) in word.to_be_bytes().iter().enumerate() {
            board.ram.write(pc.wrapping_add(k as u16), *b);
        }
        cpu.pc = pc;
        cpu.tick(board);
        let size = Instruction(word).size() as usize;
        assert_eq!(cpu.ir, Instruction::fetched(word), "CPU IR for {word:08x}");
        (cpu.ir, size)
    })
}

fn ir_cpu_sets() -> Vec<Vec<Row>> {
    let mut rng = Rng(0x0BAD_CAFE_1234_5678);
    let mut sets = Vec::new();
    for op in 0..64u32 {
        let mut m = Model::new();
        for mode in 0..16u32 {
            for _ in 0..4 {
                let rest = (rng.byte() << 14 | rng.byte() << 6 | rng.byte() >> 2) & 0x3F_FFFF;
                let word = op << 26 | mode << 22 | rest;
                let (ir, size) = run_cpu(word, &mut rng);
                let bytes = word.to_be_bytes();
                for (k, &b) in bytes.iter().enumerate().take(size) {
                    let mut ld = [false; 4];
                    ld[k] = true;
                    m.step(ld, b);
                }
                // the fields the instruction owns are the CPU's (the rest are stale)
                let held = Instruction(u32::from_be_bytes(m.b.map(|x| x.unwrap_or(0))));
                let mask = [0, 0xFF00_0000, 0xFFFF_0000, 0xFFFF_FF00, 0xFFFF_FFFF][size];
                assert_eq!(held.0 & mask, ir.0 & mask, "IR bytes for {word:08x}");
                // sometimes a RET / IRT / interrupt entry: V1 then V2 alone
                if rng.byte() & 3 == 0 {
                    m.step([false, false, true, false], rng.byte() as u8);
                    m.step([false, false, false, true], rng.byte() as u8);
                }
            }
        }
        sets.push(m.rows);
    }
    sets
}

pub const MODULES: &[SeqModule] = &[
    SeqModule { name: "ir", inputs: IR_IN, outputs: IR_OUT, buses: IR_BUSES, sets: ir_sets },
    SeqModule { name: "ir_cpu", inputs: IR_IN, outputs: IR_OUT, buses: IR_BUSES, sets: ir_cpu_sets },
];
