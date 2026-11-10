//! The microcode, run by the datapath model, against the emulator (cpu.rs).
//! Used by this crate's tests and by `hbc2 ucode --check`.
//!
//! - every opcode (64) × every mode bit pattern (16), from random states;
//! - random programs with hardware and software interrupts, STI / CLI, halts;
//! - the boot: reset, register sweep, ROM copy, first fetch.

use crate::generate::Microcode;
use crate::model::{Model, Rng};
use hbc2_core::isa::{AddrMode, INSTRUCTION_TABLE, Opcode, encode};
use hbc2_core::{Board, Cpu, CpuState, Flag, Flags, Probe};

macro_rules! ensure {
    ($c:expr, $($t:tt)*) => {
        if !$c {
            return Err(format!($($t)*));
        }
    };
}

macro_rules! ensure_eq {
    ($a:expr, $b:expr, $($t:tt)*) => {{
        let (a, b) = (&$a, &$b);
        if a != b {
            return Err(format!("{}:
  model    {:02X?}
  emulator {:02X?}", format!($($t)*), a, b));
        }
    }};
}

#[derive(Default)]
struct Rec {
    writes: Vec<(u16, u8)>,
    io: Vec<(u8, u8)>,
}

impl Probe for Rec {
    fn mem_write(&mut self, address: u16, value: u8) {
        self.writes.push((address, value));
    }
    fn io_write(&mut self, port: u8, value: u8) {
        self.io.push((port, value));
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Snap {
    regs: [u8; 8],
    flags: u8,
    pc: u16,
    sp: u16,
}

fn snap_cpu(c: &Cpu) -> Snap {
    Snap {
        regs: c.registers,
        flags: c.flags.0,
        pc: c.pc,
        sp: c.sp,
    }
}

fn snap_model(m: &Model) -> Result<Snap, String> {
    let mut regs = [0; 8];
    for (i, r) in regs.iter_mut().enumerate() {
        *r = m.regs[i].ok_or(format!("register {i} undefined"))?;
    }
    Ok(Snap {
        regs,
        flags: m.flags,
        pc: m.pc,
        sp: m.sp,
    })
}

/// What the model did up to its next instruction start (F0) or interrupt
/// entry (IE1).
struct Item {
    steps: u64,
    first_w1: Option<u64>,
    writes: Vec<(u16, u8)>,
    io: Vec<(u8, u8)>,
    /// None: no F0 / IE1 within the limit (halted).
    end: Option<&'static str>,
}

fn run_item(m: &mut Model, limit: u64) -> Result<Item, String> {
    let start = m.steps;
    let mut it = Item {
        steps: 0,
        first_w1: None,
        writes: Vec::new(),
        io: Vec::new(),
        end: None,
    };
    let mut only_loop = true;
    loop {
        let ev = m.step().map_err(|e| e.to_string())?;
        if ev.w1 && it.first_w1.is_none() {
            it.first_w1 = Some(m.steps - 1 - start);
        }
        it.writes.extend(ev.ram_write);
        it.io.extend(ev.io_write);
        only_loop &= ev.halt_loop;
        // the word now in the pipeline starts the next item? (steps of the
        // halt loop alone belong to what comes after them)
        if next_starts_item(m) && !only_loop {
            it.steps = m.steps - start;
            it.end = Some(if is_entry(m) { "entry" } else { "fetch" });
            return Ok(it);
        }
        if m.steps - start >= limit {
            it.steps = m.steps - start;
            return Ok(it);
        }
    }
}

fn is_entry(m: &Model) -> bool {
    m.entry_marks.contains(&m.addr)
}

fn next_starts_item(m: &Model) -> bool {
    use crate::word;
    let w = m.w;
    let fetch =
        w.get(word::IR_B0_LD_N) == 0 && w.get(word::MEM_RE) == 1 && w.get(word::MEM_OE_N) == 0;
    fetch || is_entry(m)
}

fn random_ram(r: &mut Rng) -> Vec<u8> {
    (0..65536).map(|_| r.byte()).collect()
}

/// Every opcode × mode bits, from random states: one instruction (plus the
/// software interrupt entry when INT runs with I set), compared with cpu.rs.
pub fn every_instruction_and_mode(mc: &Microcode, cases: u32) -> Result<String, String> {
    let mut m = mc.model();
    let mut r = Rng(0x5EED_1234_5678_9ABC);
    let base = random_ram(&mut r);
    let ports: Vec<u8> = (0..256).map(|_| r.byte()).collect();
    m.iod.ports.copy_from_slice(&ports);
    let mut n = 0;
    let mut failures = Vec::new();
    for op in 0..64u8 {
        for mode in 0..16u8 {
            for case in 0..cases {
                let regs: [u8; 8] = std::array::from_fn(|_| r.byte());
                let flags = r.byte() & 0x7F; // H = 0: the CPU is running
                let pc = if case % 4 == 0 { 0xFFFE } else { r.word() };
                let sp = match case % 5 {
                    0 => 0xFE00,
                    1 => 0x0001,
                    _ => r.word(),
                };
                let r1 = r.byte() & 7;
                let r2 = r.byte() & 7;
                let bytes = [
                    (op << 2) | (mode >> 2),
                    ((mode & 3) << 6) | (r1 << 3) | r2,
                    r.byte(),
                    r.byte(),
                ];
                let mut ram = base.clone();
                for (k, b) in bytes.iter().enumerate() {
                    ram[pc.wrapping_add(k as u16) as usize] = *b;
                }
                // emulator
                let mut board = Board::default();
                board.ram.as_mut_slice().copy_from_slice(&ram);
                for (p, v) in ports.iter().enumerate() {
                    board.iod.set_port(p as u8, *v);
                }
                let mut cpu = Cpu::new();
                cpu.registers = regs;
                cpu.flags = Flags(flags);
                cpu.pc = pc;
                cpu.sp = sp;
                let mut rec = Rec::default();
                cpu.tick_with(&mut board, &mut rec);
                let emulator_cycles = cpu.cycles;
                let mut entered = false;
                if cpu.flag(Flag::Interrupt)
                    && !cpu.interrupt_delayed()
                    && cpu.pending_software_interrupt().is_some()
                {
                    cpu.tick_with(&mut board, &mut rec);
                    entered = true;
                }
                // model
                let stale = [r.byte(), r.byte(), r.byte(), r.byte()];
                m.inject(regs, flags, pc, sp, &ram, stale);
                m.iod.ports.copy_from_slice(&ports);
                m.iod.queue.clear();
                let name = format!(
                    "{:02X} {} mode {:X} ({:02X} {:02X} {:02X} {:02X}) case {case}",
                    op,
                    Opcode::from_bits(op)
                        .map(|o| o.mnemonic())
                        .unwrap_or("invalid"),
                    mode,
                    bytes[0],
                    bytes[1],
                    bytes[2],
                    bytes[3]
                );
                let it = match run_item(&mut m, 40) {
                    Ok(it) if entered && it.end == Some("entry") => {
                        // INT then the entry: run the entry too
                        match run_item(&mut m, 40) {
                            Ok(mut it2) => {
                                it2.writes = [it.writes, it2.writes].concat();
                                it2
                            }
                            Err(e) => {
                                failures.push(format!("{name}: {e}"));
                                continue;
                            }
                        }
                    }
                    Ok(it) => it,
                    Err(e) => {
                        failures.push(format!("{name}: {e}"));
                        continue;
                    }
                };
                let halted = cpu.is_halted() && cpu.state == CpuState::Executing;
                let check = (|| -> Result<(), String> {
                    if halted != it.end.is_none() {
                        return Err(format!(
                            "emulator halted {halted}, model ended with {:?}",
                            it.end
                        ));
                    }
                    let ms = snap_model(&m)?;
                    let es = snap_cpu(&cpu);
                    if ms != es {
                        return Err(format!(
                            "state differs:\n  model    {ms:02X?}\n  emulator {es:02X?}"
                        ));
                    }
                    if it.writes != rec.writes {
                        return Err(format!(
                            "RAM writes: model {:02X?}, emulator {:02X?}",
                            it.writes, rec.writes
                        ));
                    }
                    if it.io != rec.io {
                        return Err(format!(
                            "port writes: model {:02X?}, emulator {:02X?}",
                            it.io, rec.io
                        ));
                    }
                    let (lo, hi) = mc.cycles[&(op, mode)];
                    let measured = if halted {
                        it.first_w1.unwrap_or(0)
                    } else {
                        it.steps
                    };
                    if !entered && measured != lo as u64 && measured != hi as u64 {
                        return Err(format!("{measured} steps, the table says {lo} / {hi}"));
                    }
                    if !entered && measured != emulator_cycles as u64 {
                        return Err(format!(
                            "{measured} steps, the emulator counts {emulator_cycles} cycles"
                        ));
                    }
                    Ok(())
                })();
                if let Err(e) = check {
                    failures.push(format!("{name}: {e}"));
                }
                n += 1;
            }
        }
    }
    ensure!(
        failures.is_empty(),
        "{} / {n} cases fail; first ones:\n{}",
        failures.len(),
        failures[..failures.len().min(12)].join("\n")
    );
    Ok(format!(
        "{n} cases (64 opcodes × 16 mode patterns × {cases} random states), all equal to the emulator"
    ))
}

/// How wild [`random_program`] is.
#[derive(Clone, Copy)]
pub struct Wild {
    /// One instruction in `random_bytes` is a random byte (0: never).
    pub random_bytes: u64,
    /// Of the jumps, calls and returns, keep 1 in `jumps` (they land
    /// anywhere, often inside an instruction).
    pub jumps: u64,
    /// Of HLT, CLI and POF, keep 1 in `stops`.
    pub stops: u64,
    /// Jumps and calls with a 16-bit target always kept (their target is
    /// set to an instruction start): only the other modes and RET / IRT
    /// go through the `jumps` filter.
    pub imm16_free: bool,
}

/// What `random_programs` uses.
pub const WILD: Wild = Wild {
    random_bytes: 200,
    jumps: 3,
    stops: 4,
    imm16_free: false,
};

/// A random valid instruction (mostly), as bytes.
fn random_instruction(r: &mut Rng, wild: Wild) -> Vec<u8> {
    if wild.random_bytes != 0 && r.below(wild.random_bytes) == 0 {
        return vec![r.byte()];
    }
    loop {
        let info = &INSTRUCTION_TABLE[r.below(INSTRUCTION_TABLE.len() as u64) as usize];
        // fewer ways to stop the run for good
        if matches!(info.opcode, Opcode::Hlt | Opcode::Cli | Opcode::Pof)
            && r.below(wild.stops) != 0
        {
            continue;
        }
        // jumps and returns land anywhere, often inside an instruction
        let jump = matches!(
            info.opcode,
            Opcode::Jmp
                | Opcode::Jmc
                | Opcode::Jme
                | Opcode::Jmn
                | Opcode::Jms
                | Opcode::Jmz
                | Opcode::Jmf
                | Opcode::Cal
                | Opcode::Ret
                | Opcode::Irt
        );
        if jump && !wild.imm16_free && r.below(wild.jumps) != 0 {
            continue;
        }
        let mode = info.modes[r.below(info.modes.len() as u64) as usize];
        let safe = mode == AddrMode::Imm16 && !matches!(info.opcode, Opcode::Ret | Opcode::Irt);
        if jump && wild.imm16_free && !safe && r.below(wild.jumps) != 0 {
            continue;
        }
        let i = encode::raw(info.opcode, mode, r.byte(), r.byte(), r.byte(), r.byte());
        let len = if info.opcode.takes_operands() {
            mode.length()
        } else {
            1
        } as usize;
        return i.to_bytes()[..len].to_vec();
    }
}

/// Queues (port, data) in the emulator's IOD without changing the port's
/// latch (trigger_interrupt takes the data from it).
fn raise_emu(board: &mut Board, (port, data): (u8, u8)) {
    let old = board.iod.port(port);
    board.iod.set_port(port, data);
    board.iod.trigger_interrupt(port);
    board.iod.set_port(port, old);
}

fn emu_item(cpu: &mut Cpu, board: &mut Board, rec: &mut Rec) -> bool {
    for _ in 0..4 {
        let advanced = cpu.tick_with(board, rec);
        board.tick_iod();
        if advanced {
            return true;
        }
        if cpu.state == CpuState::Executing && cpu.is_halted() {
            return false;
        }
    }
    false
}

/// A 64 KiB RAM image of random instructions (mostly valid; jump and call
/// targets with a 16-bit address land on instruction starts, and so do the
/// IVT entries at FE00-FFFF), with the instruction starts.
pub fn random_program(r: &mut Rng, wild: Wild) -> (Vec<u8>, Vec<u16>) {
    let mut ram = Vec::with_capacity(65536 + 4);
    let mut starts = Vec::new();
    let mut imm16 = Vec::new(); // jumps / calls with a 16-bit target
    while ram.len() < 0xFE00 {
        let i = random_instruction(r, wild);
        starts.push(ram.len() as u16);
        let op = i[0] >> 2;
        let mode = ((i[0] & 3) << 2) | (i.get(1).copied().unwrap_or(0) >> 6);
        if (0x14..=0x1A).contains(&op) || op == 0x04 {
            if mode == AddrMode::Imm16 as u8 {
                imm16.push(ram.len() + 2);
            }
        }
        ram.extend(i);
    }
    ram.truncate(0xFE00);
    // targets and handlers at instruction starts, so runs last longer
    for at in imm16 {
        if at + 1 < ram.len() {
            let t = starts[r.below(starts.len() as u64) as usize].to_be_bytes();
            ram[at] = t[0];
            ram[at + 1] = t[1];
        }
    }
    while ram.len() < 65536 {
        let t = starts[r.below(starts.len() as u64) as usize].to_be_bytes();
        ram.extend(t);
    }
    (ram, starts)
}

/// Random programs, with hardware interrupts raised at random boundaries:
/// every instruction and interrupt entry compared with cpu.rs.
pub fn random_programs(mc: &Microcode, runs: u32) -> Result<String, String> {
    let mut m = mc.model();
    let mut items = 0u64;
    let mut entries = 0u64;
    for seed in 0..runs {
        let mut r = Rng(0xA5A5_0000_0000_0001 ^ (seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (ram, starts) = random_program(&mut r, WILD);
        let ports: Vec<u8> = (0..256).map(|_| r.byte()).collect();
        let regs: [u8; 8] = std::array::from_fn(|_| r.byte());
        let flags = (r.byte() & 0x7F) | 0x04;
        let pc = starts[r.below(starts.len() as u64) as usize];
        let sp = r.word();

        let mut board = Board::default();
        board.ram.as_mut_slice().copy_from_slice(&ram);
        for (p, v) in ports.iter().enumerate() {
            board.iod.set_port(p as u8, *v);
        }
        let mut cpu = Cpu::new();
        cpu.registers = regs;
        cpu.flags = Flags(flags);
        cpu.pc = pc;
        cpu.sp = sp;
        m.inject(regs, flags, pc, sp, &ram, [0; 4]);
        m.iod.ports.copy_from_slice(&ports);
        m.iod.queue.clear();

        // interrupt k is raised after item k (emulator), i.e. while item k
        // runs (model: INT is sampled at the item's last step)
        let schedule: Vec<bool> = (0..400).map(|_| r.below(5) == 0).collect();
        let raise = |r: &mut Rng| (r.byte(), r.byte());
        let mut j = 0usize;
        let mut prev = None;
        let mut irq = Rng(seed as u64 + 77);
        // what the model must raise at the start of item j + 1
        let mut planned: Vec<Option<(u8, u8)>> = schedule
            .iter()
            .map(|s| if *s { Some(raise(&mut irq)) } else { None })
            .collect();
        planned[0] = None;
        if let Some(x) = planned[1] {
            m.iod.queue.push_back(x);
        }
        loop {
            if j + 2 >= planned.len() {
                break;
            }
            let mut rec = Rec::default();
            let advanced = emu_item(&mut cpu, &mut board, &mut rec);
            let it = match run_item(&mut m, 400) {
                Ok(it) => it,
                Err(e) => return Err(format!("seed {seed} item {}: model error: {e}", j + 1)),
            };
            if !advanced {
                // halted (or stopped): the model must loop without a fetch
                ensure!(
                    it.end.is_none(),
                    "seed {seed} item {}: emulator halted, the model went on ({:?})",
                    j + 1,
                    it.end
                );
                ensure_eq!(
                    snap_model(&m)?,
                    snap_cpu(&cpu),
                    "seed {seed}: state when halted"
                );
                if !cpu.flag(Flag::Interrupt) {
                    break; // stopped for good
                }
                // wake both with a hardware interrupt
                let x = raise(&mut irq);
                raise_emu(&mut board, x);
                board.int = true;
                m.iod.queue.push_back(x);
                continue;
            }
            j += 1;
            items += 1;
            ensure!(
                it.end.is_some() || cpu.is_halted(),
                "seed {seed} item {j}: the model halted, the emulator didn't: emulator after {:08X} at {:04X}: {:02X?}; model {:02X?} P {} queue {:?} word at {:04X}",
                cpu.ir.0,
                cpu.last_pc,
                snap_cpu(&cpu),
                snap_model(&m),
                m.p,
                m.iod.queue,
                m.addr
            );
            if it.end == Some("entry") {
                entries += 1;
            }
            let ms = snap_model(&m)?;
            let es = snap_cpu(&cpu);
            ensure_eq!(
                ms,
                es,
                "seed {seed} item {j} ({:?}): state; emulator last {:08X} at {:04X}, queue {:?}, pending swi {:?}; model queue {:?}, P {} swi {:?}, prev {:?}",
                it.end,
                cpu.ir.0,
                cpu.last_pc,
                board.iod.queue().collect::<Vec<_>>(),
                cpu.pending_software_interrupt(),
                m.iod.queue,
                m.p,
                m.swi,
                prev
            );
            prev = Some((es, cpu.ir.0, cpu.last_pc));
            ensure_eq!(it.writes, rec.writes, "seed {seed} item {j}: RAM writes");
            ensure_eq!(it.io, rec.io, "seed {seed} item {j}: port writes");
            // raise interrupt j after item j (emulator), j + 1 now (model)
            if let Some(x) = planned[j] {
                raise_emu(&mut board, x);
                if !board.inr {
                    board.int = true;
                }
            }
            if it.end.is_none() {
                // the instruction halted the CPU: item j + 1 won't start,
                // so the interrupt planned for it isn't raised (the run wakes
                // the CPU itself, above)
                planned[j + 1] = None;
            }
            if let Some(x) = planned[j + 1] {
                m.iod.queue.push_back(x);
            }
        }
    }
    ensure!(
        entries > runs as u64,
        "too few interrupt entries tested: {entries}"
    );
    Ok(format!(
        "{runs} random programs: {items} instructions and interrupt entries ({entries} entries), all equal to the emulator"
    ))
}

/// Reset → register sweep → ROM copy → first fetch, as the emulator's
/// power-on (`Cpu::new`, the RAM = the boot image).
pub fn boot(mc: &Microcode) -> Result<String, String> {
    let mut m = mc.model();
    for (seed, reset_steps) in [(1u64, 2u32), (2, 3), (3, 7), (4, 3)] {
        let mut r = Rng(seed * 0x1234_5678);
        let rom: Vec<u8> = (0..65536).map(|_| r.byte()).collect();
        m.power_on(&rom, seed, reset_steps);
        if seed == 4 {
            // the power-up IR on the boot row (3F, B), whose S1 goes on to
            // S2: the step register, cleared at the reset's last edge, still
            // gives S1 twice (found by the cu circuit test)
            m.ir[0] = 0xFE;
            m.ir[1] |= 0xC0;
        }
        let mut steps_after_reset = 0u64;
        loop {
            let released = m.rst_n;
            m.step().map_err(|e| format!("boot: {e}"))?;
            if released {
                steps_after_reset += 1;
            }
            if next_starts_item(&m) && m.rst_n {
                break;
            }
            ensure!(m.steps < 70_000, "no first fetch");
        }
        let cpu = Cpu::new();
        ensure_eq!(snap_model(&m)?, snap_cpu(&cpu), "state at the first fetch");
        ensure_eq!(m.lat, 0, "latch at the first fetch");
        ensure!(m.ram == rom, "RAM = the boot ROM image");
        ensure_eq!(
            steps_after_reset,
            mc.boot_steps,
            "steps from the reset release to the first fetch"
        );
    }
    Ok(format!(
        "boot from 4 power-up states (one with the IR on the boot row): RAM = ROM image, state = Cpu::new(), {} steps",
        mc.boot_steps
    ))
}
