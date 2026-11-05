//! The control unit (logisim/notes/cu-decisions.md, section 14).
//!
//! The reference is the microcode as built (`hbc2_ucode::build`, the images
//! written into the at28c256 model) run by the datapath model, which is equal
//! to the emulator on every instruction, mode, interrupt entry and the boot
//! (`hbc2 ucode --check`). The rows give `cu` what the model's sections give
//! it in each step (`bus_data`, the IR, the flags, INT, the adder's carry out,
//! the reset lines) and expect the pipeline register's outputs: the word the
//! model holds, decoded as section 2 does (`word::outputs`), or the pulled
//! idle levels while `rst_rc` holds the 574s off (C24).
//!
//! Each set starts at Logisim's power-up with the reset held (`rst_rc` high
//! for the first steps: the 574s' power-up contents never show), runs the
//! whole boot (register sweep, 65,536-byte ROM copy), then a random program
//! with hardware interrupts from the IOD stub and halts woken by an
//! interrupt; when the CPU stops for good (an invalid mode, or HLT with
//! I = 0), the reset button and a new boot, up to 3 boots per set.
//!
//! One clock step = 2 rows (clk low, then the rising edge).

use super::{Out, Row, SeqModule};
use hbc2_ucode::check::{Wild, random_program};
use hbc2_ucode::model::{Model, Rng};
use hbc2_ucode::word::{self, Word};

pub const CU_IN: &[(&str, u32)] = &[
    ("clk", 2), // two clock branches, always together
    ("rst_n", 1),
    ("rst_early", 1),
    ("rst_rc", 1),
    ("cu_bus_in", 8),
    ("cu_ir_op", 6),
    ("cu_ir_mode", 4),
    ("cu_flag_c", 1),
    ("cu_flag_e", 1),
    ("cu_flag_i", 1),
    ("cu_flag_n", 1),
    ("cu_flag_s", 1),
    ("cu_flag_z", 1),
    ("cu_flag_f", 1),
    ("cu_flag_h", 1),
    ("cu_int", 1),
    ("cu_addr_cout", 1),
];

/// `cu_` + every name of `word::outputs`, in its order, then the pending
/// port latch's driver on the low byte of `bus_addr`.
pub const CU_OUT: &[(&str, u32)] = &[
    ("cu_alu_op", 4),
    ("cu_flags_op", 4),
    ("cu_flags_bit_val", 1),
    ("cu_flags_oe_n", 1),
    ("cu_regs_we_n", 1),
    ("cu_regs_a_sel", 2),
    ("cu_regs_cu_idx", 2),
    ("cu_regs_cu_bank_n", 2),
    ("cu_regs_b_sel", 2),
    ("cu_regs_a_lat_oe_n", 1),
    ("cu_regs_b_lat_oe_n", 1),
    ("cu_regs_const_oe_n", 1),
    ("cu_regs_const_0", 1),
    ("cu_regs_a_bus_oe_n", 1),
    ("cu_regs_res_oe_n", 1),
    ("cu_addr_base_sel", 2),
    ("cu_addr_base_oe_n", 2),
    ("cu_addr_off_sel", 1),
    ("cu_addr_off_oe_n", 2),
    ("cu_addr_inv", 1),
    ("cu_addr_cin", 1),
    ("cu_addr_lat_ld_n", 1),
    ("cu_addr_pc_ld_n", 1),
    ("cu_addr_sp_ld_n", 1),
    ("cu_addr_lat_oe_n", 2),
    ("cu_addr_pc_oe_n", 2),
    ("cu_addr_sp_oe_n", 2),
    ("cu_ir_b0_ld_n", 1),
    ("cu_ir_b1_ld_n", 1),
    ("cu_ir_v1_ld_n", 1),
    ("cu_ir_v2_ld_n", 1),
    ("cu_mem_re", 1),
    ("cu_mem_oe_n", 1),
    ("cu_mem_we_n", 1),
    ("cu_mem_wbuf_oe_n", 1),
    ("cu_mem_rom_oe_n", 1),
    ("cu_ie", 1),
    ("cu_rw", 1),
    ("cu_inr", 1),
    ("cu_bus_addr_lo", 8),
];
const CU_BUSES: &[&str] = &["cu_bus_addr_lo"];

/// Sets (each from Logisim's power-up) and boots per set at most (the
/// power-up, then a button reset each time the CPU stops for good). A set
/// stays under logitest's 524,288-row chunk. `HBC2_CU_SETS` overrides
/// SETS (shorter runs, e.g. to try a change).
const SETS: u64 = 10;
const BOOTS: u64 = 3;
/// Steps per set at most (rows = 2 x steps).
const SET_STEPS: u64 = 260_000;
/// A boot, with margin (65,548 steps after the reset).
const BOOT_STEPS: u64 = 65_600;
/// A halt loop this long gets an interrupt (I = 1) or a button reset (I = 0).
const HALT_WAIT: u32 = 24;

/// The expected outputs for the word in the pipeline register, the pending
/// port latch holding `swi`, with the 574s off (`off`: the pulled levels).
fn outs(w: Word, swi: Option<u8>, off: bool) -> Vec<Out> {
    let shown = if off { word::pulled() } else { w };
    let list = word::outputs(shown);
    debug_assert_eq!(list.len() + 1, CU_OUT.len());
    let mut v: Vec<Out> = list.iter().map(|&(_, _, x)| Out::V(x)).collect();
    v.push(if word::swi_oe(shown) {
        match swi {
            Some(x) => Out::V(x as u32),
            None => Out::U,
        }
    } else {
        Out::Z
    });
    v
}

struct Run {
    m: Model,
    r: Rng,
    rows: Vec<Row>,
    reached: Vec<bool>,
}

impl Run {
    /// One step of the model, as two rows.
    fn step(&mut self) -> Result<hbc2_ucode::model::Event, String> {
        let m = &mut self.m;
        let w = m.w;
        let swi = m.swi;
        let off = m.reset_left() > 2; // rst_rc falls a step before rst_early_n rises (C24)
        let (rst_n, rst_early) = (m.rst_n as u32, !m.rst_early_n as u32);
        let (flags, ir) = (m.flags, m.ir);
        self.reached[m.addr >> 5] |= m.rst_n && m.addr & 0x1E == 2; // step 1 of a row ran
        let ev = m.step().map_err(|e| format!("model: {e}"))?;
        let bus = ev.bus_data.unwrap_or_else(|| self.r.byte());
        let cout = ev.cout.unwrap_or_else(|| self.r.below(2) == 1);
        let ins = |clk: u32| {
            let mut v = vec![
                clk * 3,
                rst_n,
                rst_early,
                off as u32,
                bus as u32,
                (ir[0] >> 2) as u32,
            ];
            v.push((((ir[0] & 3) << 2) | (ir[1] >> 6)) as u32);
            v.extend((0..8).map(|k| ((flags >> k) & 1) as u32));
            v.push(ev.int as u32);
            v.push(cout as u32);
            v
        };
        self.rows.push((ins(0), outs(w, swi, off)));
        self.rows.push((ins(1), outs(m.w, m.swi, off)));
        Ok(ev)
    }

    /// Reset (power-up or the button) and the whole boot, up to the first fetch.
    fn boot(&mut self, rom: &[u8], seed: u64, reset_steps: u32) -> Result<(), String> {
        self.m.power_on(rom, seed, reset_steps);
        if self.r.below(2) == 0 {
            // opcode bits 2..0 = 111 in the power-up IR: jc = 1 during the
            // reset, so a cu_cond_sel pulled to 00 would pick S1 (C24)
            self.m.ir[0] |= 0x1C;
        }
        loop {
            self.step()?;
            let w = self.m.w;
            let fetch = w.get(word::IR_B0_LD_N) == 0 && w.get(word::MEM_RE) == 1;
            if self.m.rst_n && fetch {
                return Ok(());
            }
            if self.m.steps > 70_000 {
                return Err("boot: no first fetch".into());
            }
        }
    }

    /// The program, with interrupts, for up to `steps` steps; Ok(true) when
    /// the CPU stopped for good (halted with I = 0: only a reset gets out).
    fn program(&mut self, steps: u64) -> Result<bool, String> {
        let mut halted = 0u32;
        for _ in 0..steps {
            if self.m.iod.queue.len() < 2 && self.r.below(150) == 0 {
                let x = (self.r.byte(), self.r.byte());
                self.m.iod.queue.push_back(x);
            }
            let ev = self.step()?;
            halted = if ev.halt_loop { halted + 1 } else { 0 };
            if halted > HALT_WAIT {
                if self.m.flags & 0x04 == 0 {
                    return Ok(true);
                }
                let x = (self.r.byte(), self.r.byte());
                self.m.iod.queue.push_back(x);
                halted = 0;
            }
        }
        Ok(false)
    }
}

fn cu_sets() -> Vec<Vec<Row>> {
    let mc = hbc2_ucode::build().unwrap_or_else(|e| {
        panic!(
            "microcode: {}",
            e.join(
                "
"
            )
        )
    });
    for (&(name, width), (out, w, _)) in CU_OUT.iter().zip(word::outputs(Word::idle())) {
        assert_eq!(
            (name.strip_prefix("cu_"), width),
            (Some(out), w),
            "CU_OUT order"
        );
    }
    let mut sets = Vec::new();
    let mut reached = vec![false; 1024];
    let (mut steps, mut boots) = (0usize, 0u32);
    let sets_n: u64 = std::env::var("HBC2_CU_SETS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(SETS);
    for k in 0..sets_n {
        let seed = 0xC0DE_0000 + k;
        let mut r = Rng(0x0C0F_FEE0_0000_0001 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (rom, _) = random_program(
            &mut r,
            Wild {
                random_bytes: 5000,
                jumps: 300,
                stops: 400,
                imm16_free: true,
            },
        );
        let mut run = Run {
            m: mc.model(),
            r,
            rows: Vec::new(),
            reached: std::mem::take(&mut reached),
        };
        let result = (|| -> Result<(), String> {
            for b in 0..BOOTS {
                if b > 0 && (run.rows.len() as u64) / 2 + BOOT_STEPS + 1_000 > SET_STEPS {
                    break; // no room for another boot
                }
                // power-up, then the button: the reset held 3 to 8 steps
                let held = 3 + run.r.below(6) as u32;
                run.boot(&rom, seed + 100 * b, held)?;
                boots += 1;
                let ports: Vec<u8> = (0..256).map(|_| run.r.byte()).collect();
                run.m.iod.ports.copy_from_slice(&ports);
                let left = SET_STEPS - (run.rows.len() as u64) / 2;
                if !run.program(left)? {
                    break; // the set is full
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            panic!("cu set {}: {e}", k + 1);
        }
        assert!(
            run.rows.len() <= 524_288,
            "cu set {}: {} rows, more than logitest's chunk",
            k + 1,
            run.rows.len()
        );
        steps += run.rows.len() / 2;
        reached = run.reached;
        sets.push(run.rows);
    }
    let rows = reached.iter().filter(|&&x| x).count();
    let program = steps as u64 - u64::from(boots) * BOOT_STEPS;
    eprintln!(
        "cu: {sets_n} sets, {steps} steps, {boots} boots (power-up and button), about {program} program steps; {rows} of 1024 opcode x mode rows ran their step 1"
    );
    sets
}

pub const MODULES: &[SeqModule] = &[SeqModule {
    name: "cu",
    inputs: CU_IN,
    outputs: CU_OUT,
    buses: CU_BUSES,
    sets: cu_sets,
}];
