//! A fast step-level model of the HBC-2 datapath, driven by the microcode
//! images exactly as the hardware reads them.
//!
//! Each [`Model::step`] is one `clk` period: the word in the pipeline
//! register (decoded as cu-decisions.md section 2 does, before the 574s)
//! drives the sections; the buses, the adder, the ALU and the next microcode
//! address are evaluated; then everything clocked by `clk` loads at the edge.
//! Values that nothing drives are `None`, and the model fails on any bus with
//! two drivers, any load or write of an undriven value, and any strobe
//! combination outside C19, so the tests also check the bus discipline.

use std::collections::VecDeque;

use crate::word::{self, Word, abus, address, cond};

/// The IOD stub of the test benches (ir-mem-decisions.md M5): port latches
/// and an interrupt FIFO, synchronous to `clk` (iod-requirements.md).
#[derive(Clone, Debug)]
pub struct Iod {
    pub ports: [u8; 256],
    pub queue: VecDeque<(u8, u8)>,
    /// The vector step: INR was seen at the last edge.
    ack: bool,
}

impl Default for Iod {
    fn default() -> Self {
        Iod {
            ports: [0; 256],
            queue: VecDeque::new(),
            ack: false,
        }
    }
}

impl Iod {
    /// INT: high while the queue isn't empty, low in the vector step.
    pub fn int(&self) -> bool {
        !self.ack && !self.queue.is_empty()
    }
}

/// What a step did, for the comparison with the emulator.
#[derive(Clone, Copy, Debug, Default)]
pub struct Event {
    /// The step was F0 (fetch of an instruction's byte 0 from RAM).
    pub fetch: bool,
    /// The step was the first of an interrupt entry (IE1).
    pub entry: bool,
    /// The step was W1 (the halt loop / interrupt decision).
    pub w1: bool,
    /// The step was W1 or W2 (the halt loop).
    pub halt_loop: bool,
    pub ram_write: Option<(u16, u8)>,
    pub io_write: Option<(u8, u8)>,
    /// What the control unit read in the step (for `hbc2 vectors cu`):
    /// `bus_data` (None: undriven), INT, the adder's carry out (None: the
    /// sum is undefined).
    pub bus_data: Option<u8>,
    pub int: bool,
    pub cout: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct ModelError {
    pub step: u64,
    pub address: usize,
    pub message: String,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a = self.address;
        write!(
            f,
            "step {}: word {:02X}/{:X} step {} cond {}: {}",
            self.step,
            a >> 9,
            (a >> 5) & 15,
            (a >> 1) & 15,
            a & 1,
            self.message
        )
    }
}

#[derive(Clone)]
pub struct Model {
    words: Vec<Word>,
    /// The pipeline register (574s + the step register's 175) and the
    /// address it was read from.
    pub w: Word,
    pub addr: usize,
    pub regs: [Option<u8>; 8],
    pub a_lat: Option<u8>,
    pub b_lat: Option<u8>,
    pub res: Option<u8>,
    pub flags: u8,
    pub pc: u16,
    pub sp: u16,
    pub lat: u16,
    /// IR bytes (b0, b1, V1, V2) and whether each was loaded since the start.
    pub ir: [u8; 4],
    pub ir_def: [bool; 4],
    pub hw_q: bool,
    pub cout_q: Option<bool>,
    pub p: bool,
    pub swi: Option<u8>,
    pub ram: Vec<u8>,
    pub rom: Vec<u8>,
    pub iod: Iod,
    pub rst_n: bool,
    pub rst_early_n: bool,
    /// `rst_n` in the step before: the step register (74HCT175, /CLR =
    /// `rst_n`) only holds the word's `cu_next` if it wasn't cleared at the
    /// edge that loaded it.
    prev_rst_n: bool,
    /// Steps left with `rst_n` low.
    reset_left: u32,
    pub steps: u64,
    /// Addresses of IE1 (hardware and software), for [`Event::entry`].
    pub entry_marks: Vec<usize>,
}

const FLAG_C: u8 = 1 << 0;
const FLAG_E: u8 = 1 << 1;
const FLAG_I: u8 = 1 << 2;
const FLAG_N: u8 = 1 << 3;
const FLAG_S: u8 = 1 << 4;
const FLAG_Z: u8 = 1 << 5;
const FLAG_F: u8 = 1 << 6;
const FLAG_H: u8 = 1 << 7;

/// The ALU (alu-report.md, alu_op table): (result, C).
pub fn alu(op: u8, a: u8, b: u8, cin: bool) -> (u8, bool) {
    if op & 0b1000 == 0 {
        let use_c = op & 0b0010 != 0 && cin;
        if op & 1 == 0 {
            let s = a as u16 + b as u16 + use_c as u16;
            (s as u8, s > 0xFF)
        } else {
            let d = a as i16 - b as i16 - use_c as i16;
            (d as u8, d < 0)
        }
    } else {
        match op & 0b0111 {
            0b000 => (a & b, a & 0x80 != 0),
            0b001 => (a | b, a & 1 != 0),
            0b010 => (a ^ b, a & 0x80 != 0),
            0b011 => (!a, a & 1 != 0),
            0b100 => (a << 1, a & 0x80 != 0),
            0b110 => (a >> 1, a & 0x80 != 0),
            _ => ((a >> 1) | (a & 0x80), a & 1 != 0),
        }
    }
}

impl Model {
    /// A model running these 7 images.
    pub fn new(images: &[Vec<u8>]) -> Model {
        let words = word::from_images(images);
        Model {
            w: words[0],
            words,
            addr: 0,
            regs: [None; 8],
            a_lat: None,
            b_lat: None,
            res: None,
            flags: 0,
            pc: 0,
            sp: 0,
            lat: 0,
            ir: [0; 4],
            ir_def: [false; 4],
            hw_q: false,
            cout_q: None,
            p: false,
            swi: None,
            ram: vec![0; 65536],
            rom: vec![0; 65536],
            iod: Iod::default(),
            rst_n: true,
            rst_early_n: true,
            prev_rst_n: true,
            reset_left: 0,
            steps: 0,
            entry_marks: Vec::new(),
        }
    }

    /// Power-up: everything random (from `seed`), the reset held for
    /// `reset_steps` steps (at least 2: C17's two flip-flops), the boot ROM
    /// image in the ROMs. The pipeline starts on the reset word of the IR's
    /// random row (the 574s' power-up contents only last until the first
    /// edge, under 1 µs into the reset).
    pub fn power_on(&mut self, rom: &[u8], seed: u64, reset_steps: u32) {
        let mut r = Rng(seed | 1);
        self.rom = rom.to_vec();
        for b in self.ram.iter_mut() {
            *b = r.byte();
        }
        self.regs = [None; 8];
        self.a_lat = None;
        self.b_lat = None;
        self.res = None;
        self.flags = r.byte();
        self.pc = r.word();
        self.sp = r.word();
        self.lat = r.word();
        self.ir = [r.byte(), r.byte(), r.byte(), r.byte()];
        self.ir_def = [false; 4];
        self.swi = None;
        self.iod = Iod::default();
        self.reset_left = reset_steps.max(2);
        self.rst_n = false;
        self.rst_early_n = false;
        self.prev_rst_n = false;
        let row = ((self.ir[0] as usize) << 2) | (self.ir[1] as usize >> 6);
        self.addr = row << 5;
        self.w = self.words[self.addr];
        self.steps = 0;
    }

    /// Starts at F0 with this architectural state: the previous instruction
    /// just ended (latch = PC, no interrupt taken). Latches the microcode
    /// hasn't loaded yet are undefined.
    pub fn inject(&mut self, regs: [u8; 8], flags: u8, pc: u16, sp: u16, ram: &[u8], ir: [u8; 4]) {
        self.regs = regs.map(Some);
        self.flags = flags;
        self.pc = pc;
        self.sp = sp;
        self.lat = pc;
        self.ram.copy_from_slice(ram);
        self.ir = ir;
        self.ir_def = [false; 4];
        self.a_lat = None;
        self.b_lat = None;
        self.res = None;
        self.hw_q = false;
        self.cout_q = Some(false);
        self.p = false;
        self.swi = None;
        self.rst_n = true;
        self.rst_early_n = true;
        self.prev_rst_n = true;
        self.reset_left = 0;
        self.addr = address(0, 0, 1, true);
        self.w = self.words[self.addr];
        self.steps = 0;
    }

    /// Steps left with `rst_n` low (0 once the reset is over).
    pub fn reset_left(&self) -> u32 {
        self.reset_left
    }

    pub fn reg(&self, i: usize) -> Option<u8> {
        self.regs[i]
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T, ModelError> {
        Err(ModelError {
            step: self.steps,
            address: self.addr,
            message: message.into(),
        })
    }

    /// One `clk` period.
    pub fn step(&mut self) -> Result<Event, ModelError> {
        let w = self.w;
        let g = |f| w.get(f);
        let mut ev = Event::default();
        let rst_n = self.rst_n;
        if !rst_n {
            // asynchronous clears: flags (I = 1), the CU state flip-flops
            self.flags = FLAG_I;
            self.hw_q = false;
            self.cout_q = Some(false);
            self.p = false;
        }
        ev.entry = self.entry_marks.contains(&self.addr);
        ev.w1 = w.bit(word::CU_FORCE) && g(word::CU_COND_SEL) == cond::HWQ;
        ev.halt_loop = ev.w1 || (w.bit(word::CU_FORCE) && g(word::CU_COND_SEL) == cond::XN);

        // ----- IR fields
        let field = |m: &Model, which: u8| -> Result<u8, ModelError> {
            let (byte, v) = match which {
                1 => (1, (m.ir[1] >> 3) & 7),
                2 => (1, m.ir[1] & 7),
                _ => (2, m.ir[2] & 7),
            };
            if !m.ir_def[byte] {
                return m.fail(format!("R{which} read before its IR byte was loaded"));
            }
            Ok(v)
        };

        // ----- bus A
        let a_sel = g(word::REGS_A_SEL);
        let bank_n = g(word::REGS_CU_BANK_N);
        let port_a: Option<usize> = match a_sel {
            0 => Some(field(self, 1)? as usize),
            1 => Some(field(self, 2)? as usize),
            2 => Some(field(self, 3)? as usize),
            _ => match bank_n {
                0b10 => Some(g(word::REGS_CU_IDX) as usize),
                0b01 => Some(4 + g(word::REGS_CU_IDX) as usize),
                0b11 => None,
                _ => return self.fail("regs_cu_bank_n = 00 (both banks on port A)"),
            },
        };
        let a_latch_on = bank_n == 0b11;
        if port_a.is_some() && a_latch_on {
            return self.fail("bus A: port A and the A operand latch both drive it");
        }
        let bus_a = match port_a {
            Some(i) => self.regs[i],
            None => self.a_lat,
        };

        // ----- bus B
        let b_sel = g(word::REGS_B_SEL);
        let const_on = g(word::REGS_CONST_OE_N) == 0;
        let port_b: Option<usize> = match b_sel {
            0 => Some(field(self, 2)? as usize),
            1 => Some(field(self, 1)? as usize),
            2 => Some(field(self, 3)? as usize),
            _ => None,
        };
        let b_latch_on = b_sel == 0b11 && !const_on;
        let drivers = port_b.is_some() as u8 + b_latch_on as u8 + const_on as u8;
        if drivers != 1 {
            return self.fail(format!("bus B: {drivers} drivers"));
        }
        let alu_op = g(word::ALU_OP);
        let bus_b = if let Some(i) = port_b {
            self.regs[i]
        } else if const_on {
            Some((alu_op & 0b1000 == 0) as u8) // regs_const_0 = NOT alu_op3
        } else {
            self.b_lat
        };

        // ----- bus_addr
        let ab = g(word::ABUS);
        let (lat_hi, lat_lo, ie, swi_oe) = match ab {
            abus::LATCH => (true, true, false, false),
            abus::IO => (true, true, true, false),
            abus::HW_VECTOR => (false, false, false, false),
            _ => (true, false, false, true),
        };
        let iod_ack = self.iod.ack;
        let iod_entry = if iod_ack {
            self.iod.queue.front().copied()
        } else {
            None
        };
        if iod_ack && iod_entry.is_none() {
            return self.fail("INR with an empty interrupt queue");
        }
        let hi_drivers = lat_hi as u8 + iod_ack as u8;
        let lo_drivers = lat_lo as u8 + iod_ack as u8 + swi_oe as u8;
        if hi_drivers > 1 || lo_drivers > 1 {
            return self
                .fail("bus_addr: two drivers (the latch, the IOD's vector, the pending latch)");
        }
        let addr_hi: Option<u8> = if lat_hi {
            Some((self.lat >> 8) as u8)
        } else if iod_ack {
            Some(0)
        } else {
            None
        };
        let addr_lo: Option<u8> = if lat_lo {
            Some(self.lat as u8)
        } else if let Some((p, _)) = iod_entry {
            Some(p)
        } else if swi_oe {
            self.swi
        } else {
            None
        };
        let bus_addr: Option<u16> = match (addr_hi, addr_lo) {
            (Some(h), Some(l)) => Some(u16::from_be_bytes([h, l])),
            _ => None,
        };

        // ----- the adder
        let base_sel = g(word::ADDR_BASE_SEL);
        let base_oe = g(word::ADDR_BASE_OE_N);
        let ivt_p = addr_lo;
        let base_hi: Option<u8> = if base_oe & 0b10 != 0 {
            Some(0)
        } else {
            match base_sel {
                0 => Some((self.pc >> 8) as u8),
                1 => Some((self.sp >> 8) as u8),
                2 => bus_a,
                _ if !rst_n => Some(0xFE), // p7 AND rst_n (A1)
                _ => ivt_p.map(|p| 0xFE | (p >> 7)),
            }
        };
        let base_lo: Option<u8> = if base_oe & 0b01 != 0 {
            Some(0)
        } else {
            match base_sel {
                0 => Some(self.pc as u8),
                1 => Some(self.sp as u8),
                2 => bus_b,
                _ => ivt_p.map(|p| p << 1),
            }
        };
        let v1 = self.ir_def[2].then_some(self.ir[2]);
        let v2 = self.ir_def[3].then_some(self.ir[3]);
        let off_oe = g(word::ADDR_OFF_OE_N);
        let off_hi: Option<u8> = if off_oe & 0b10 != 0 { Some(0) } else { v1 };
        let off_lo: Option<u8> = if off_oe & 0b01 != 0 {
            Some(0)
        } else if w.bit(word::ADDR_OFF_SEL) {
            v1
        } else {
            v2
        };
        let sum: Option<u32> = match (base_hi, base_lo, off_hi, off_lo) {
            (Some(bh), Some(bl), Some(oh), Some(ol)) => {
                let base = u16::from_be_bytes([bh, bl]) as u32;
                let mut off = u16::from_be_bytes([oh, ol]) as u32;
                if w.bit(word::ADDR_INV) {
                    off ^= 0xFFFF;
                }
                Some(base + off + w.bit(word::ADDR_CIN) as u32)
            }
            _ => None,
        };
        let cout = sum.map(|s| s > 0xFFFF);

        // ----- memory strobes (C19)
        let strobes = (
            g(word::MEM_RE),
            g(word::MEM_OE_N),
            g(word::MEM_WE_N),
            g(word::MEM_WBUF_OE_N),
            g(word::MEM_ROM_OE_N),
        );
        let (ram_read, ram_write, rom_copy) = match strobes {
            (0, 1, 1, 1, 1) => (false, false, false),
            (1, 0, 1, 1, 1) => (true, false, false),
            (1, 1, 0, 0, 1) => (false, true, false),
            (1, 1, 0, 1, 0) => (false, false, true),
            s => return self.fail(format!("memory strobes {s:?} outside C19's four cases")),
        };
        if strobes.0 == 1 && ie {
            return self.fail("RE and IE both 1");
        }

        // ----- bus_data
        let d = g(word::DBUS);
        let rw = d != word::dbus::BUS_A; // RW = 0 only when bus A drives bus_data
        let mut drivers = 0;
        let mut bus_data: Option<u8> = None;
        let mut drive = |v: Option<u8>| {
            drivers += 1;
            bus_data = v;
        };
        match d {
            0 => {}
            1 => drive(bus_a),
            2 => drive(self.res),
            3 => drive(Some(self.flags)),
            4 => drive(Some((self.pc >> 8) as u8)),
            5 => drive(Some(self.pc as u8)),
            6 => drive(Some((self.sp >> 8) as u8)),
            _ => drive(Some(self.sp as u8)),
        }
        if ram_read {
            let Some(a) = bus_addr else {
                return self.fail("RAM read with bus_addr undriven");
            };
            drive(Some(self.ram[a as usize]));
        }
        if ie && rw {
            let (Some(0), Some(port)) = (addr_hi, addr_lo) else {
                return self.fail("IN: bus_addr is not 00:port");
            };
            drive(Some(self.iod.ports[port as usize]));
        }
        if let Some((_, data)) = iod_entry {
            drive(Some(data));
        }
        if drivers > 1 {
            return self.fail("bus_data: two drivers");
        }
        ev.fetch = ram_read && g(word::IR_B0_LD_N) == 0;

        // ----- register write (transparent 670: port A / B follow D)
        let we = g(word::REGS_WE_N) == 0;
        let (mut alu_a, mut alu_b) = (bus_a, bus_b);
        if we {
            let Some(i) = port_a else {
                return self.fail("register write with port A off");
            };
            if d == word::dbus::BUS_A {
                return self.fail("C5: regs_a_bus_oe_n with regs_we_n");
            }
            if bus_data.is_none() {
                return self.fail("register write from an undriven bus_data");
            }
            alu_a = bus_data;
            if port_b == Some(i) {
                alu_b = bus_data;
            }
        }

        // ----- ALU
        let needs_b = !matches!(alu_op, 0b1011 | 0b1100..=0b1111);
        let alu_out = match (alu_a, alu_b) {
            (Some(a), Some(b)) => Some((a, b, alu(alu_op, a, b, self.flags & FLAG_C != 0))),
            (Some(a), None) if !needs_b => {
                Some((a, 0, alu(alu_op, a, 0, self.flags & FLAG_C != 0)))
            }
            _ => None,
        };

        // ----- flags (computed for the edge)
        let fop = g(word::FLAGS_OP);
        let mut flags = self.flags;
        match fop {
            0b0001..=0b0011 => {
                let Some((a, b, (r, c))) = alu_out else {
                    return self.fail("ALU flags stored from undefined operands");
                };
                let set = |f: &mut u8, bit: u8, v: bool| if v { *f |= bit } else { *f &= !bit };
                match fop {
                    0b0001 => {
                        set(&mut flags, FLAG_C, c);
                        set(&mut flags, FLAG_Z, r == 0);
                        set(&mut flags, FLAG_N, r & 0x80 != 0);
                    }
                    0b0010 => {
                        set(&mut flags, FLAG_Z, r == 0);
                        set(&mut flags, FLAG_N, r & 0x80 != 0);
                    }
                    _ => {
                        set(&mut flags, FLAG_S, a > b);
                        set(&mut flags, FLAG_E, a == b);
                        set(&mut flags, FLAG_F, a < b);
                    }
                }
            }
            0b0100 => match bus_data {
                Some(v) => flags = v,
                None => return self.fail("POF from an undriven bus_data"),
            },
            0b1000..=0b1111 => {
                let bit = 1 << (fop & 7);
                if w.bit(word::FLAGS_BIT_VAL) {
                    flags |= bit
                } else {
                    flags &= !bit
                }
            }
            _ => {}
        }

        // ----- conditions and the next address
        let i_flag = self.flags & FLAG_I != 0;
        let int = self.iod.int();
        let hw = i_flag && int;
        let sw = i_flag && self.p;
        let h = self.flags & FLAG_H != 0;
        ev.bus_data = bus_data;
        ev.int = int;
        ev.cout = cout;
        let cond_sel = g(word::CU_COND_SEL);
        let c = match cond_sel {
            cond::JC => {
                let f = self.flags;
                [
                    f & FLAG_S,
                    f & FLAG_Z,
                    f & FLAG_F,
                    0,
                    f & FLAG_C,
                    f & FLAG_E,
                    f & FLAG_N,
                    1,
                ][(self.ir[0] >> 2) as usize & 7]
                    != 0
            }
            cond::NOINT => !(hw || sw || h),
            cond::HWQ => self.hw_q,
            _ => {
                let Some(cq) = self.cout_q else {
                    return self.fail("x_n read with cout_q undefined");
                };
                !(sw || cq || !self.rst_early_n)
            }
        };
        let (op6, mode) = if w.bit(word::CU_FORCE) {
            (0, 0)
        } else {
            let b0 = if g(word::IR_B0_LD_N) == 0 {
                match bus_data {
                    Some(v) => v,
                    None => return self.fail("fetch bypass: bus_data undriven"),
                }
            } else {
                self.ir[0]
            };
            let b1 = if g(word::IR_B1_LD_N) == 0 {
                match bus_data {
                    Some(v) => v,
                    None => return self.fail("fetch bypass: bus_data undriven"),
                }
            } else {
                self.ir[1]
            };
            (b0 >> 2, ((b0 & 3) << 2) | (b1 >> 6))
        };
        // the step register: cleared while rst_n is low, and still 0 in the
        // first step after the release (it was cleared at the edge that
        // loaded this word)
        let step = if rst_n && self.prev_rst_n {
            g(word::CU_NEXT)
        } else {
            0
        };
        let next_addr = address(op6, mode, step, c);

        // ----- the clock edge
        if we {
            self.regs[port_a.unwrap()] = bus_data;
        }
        // AND with 00 is 00 whatever A holds (the reset word and S1 clear the
        // result latch that way, even when the A latch was never loaded)
        self.res = match alu_out {
            Some((_, _, (r, _))) => Some(r),
            None if alu_op == 0b1000 && alu_b == Some(0) => Some(0),
            None => None,
        };
        self.a_lat = bus_data;
        self.b_lat = bus_data;
        self.flags = if rst_n { flags } else { FLAG_I };
        let loads = [
            (word::ADDR_LAT_LD_N, 0),
            (word::ADDR_PC_LD_N, 1),
            (word::ADDR_SP_LD_N, 2),
        ];
        for (f, which) in loads {
            if g(f) != 0 {
                continue;
            }
            let Some(s) = sum else {
                return self.fail("address load from an undefined sum");
            };
            let s = s as u16;
            match which {
                0 => self.lat = s,
                1 => self.pc = if rst_n { s } else { s & 0x01FF }, // bits 15..9 AND rst_n
                _ => self.sp = s,
            }
        }
        for (k, f) in [
            word::IR_B0_LD_N,
            word::IR_B1_LD_N,
            word::IR_V1_LD_N,
            word::IR_V2_LD_N,
        ]
        .into_iter()
        .enumerate()
        {
            if g(f) == 0 {
                let Some(v) = bus_data else {
                    return self.fail("IR load from an undriven bus_data");
                };
                self.ir[k] = v;
                self.ir_def[k] = true;
            }
        }
        if ram_write || rom_copy {
            let Some(a) = bus_addr else {
                return self.fail("RAM write with bus_addr undriven");
            };
            let v = if ram_write {
                match bus_data {
                    Some(v) => v,
                    None => return self.fail("RAM write from an undriven bus_data"),
                }
            } else {
                self.rom[a as usize]
            };
            self.ram[a as usize] = v;
            if ram_write {
                ev.ram_write = Some((a, v));
            }
        }
        if ie && !rw {
            let (Some(0), Some(port)) = (addr_hi, addr_lo) else {
                return self.fail("OUT: bus_addr is not 00:port");
            };
            let Some(v) = bus_data else {
                return self.fail("OUT from an undriven bus_data");
            };
            self.iod.ports[port as usize] = v;
            ev.io_write = Some((port, v));
        }
        let swi_ld = g(word::CU_SWI_LD_N) == 0;
        if swi_ld {
            let Some(v) = bus_data else {
                return self.fail("pending latch load from an undriven bus_data");
            };
            self.swi = Some(v);
        }
        self.p = rst_n && ((self.p && !swi_oe) || swi_ld);
        self.hw_q = rst_n && hw;
        self.cout_q = if rst_n { cout } else { Some(false) };
        if iod_ack {
            self.iod.queue.pop_front();
            self.iod.ack = false;
        }
        if w.bit(word::INR) {
            if self.iod.queue.is_empty() {
                return self.fail("INR with no interrupt pending");
            }
            self.iod.ack = true;
        }
        self.prev_rst_n = rst_n;
        if self.reset_left > 0 {
            self.reset_left -= 1;
        }
        self.rst_n = self.reset_left == 0;
        self.rst_early_n = self.reset_left <= 1;
        self.addr = next_addr;
        self.w = self.words[next_addr];
        self.steps += 1;
        Ok(ev)
    }
}

/// xorshift64, for power-up states.
#[derive(Clone, Copy)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
    pub fn word(&mut self) -> u16 {
        (self.next() >> 24) as u16
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}
