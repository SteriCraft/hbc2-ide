//! The microcode, run by the datapath model, against the emulator (src/check.rs).

use std::sync::OnceLock;

use hbc2_ucode::check;
use hbc2_ucode::generate::Microcode;

fn microcode() -> &'static Microcode {
    static M: OnceLock<Microcode> = OnceLock::new();
    M.get_or_init(|| hbc2_ucode::build().unwrap_or_else(|e| panic!("{}", e.join("\n"))))
}

fn ok(r: Result<String, String>) {
    match r {
        Ok(s) => println!("{s}"),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn every_instruction_and_mode() {
    ok(check::every_instruction_and_mode(
        microcode(),
        if cfg!(debug_assertions) { 12 } else { 64 },
    ));
}

#[test]
fn random_programs_with_interrupts() {
    ok(check::random_programs(
        microcode(),
        if cfg!(debug_assertions) { 400 } else { 4000 },
    ));
}

#[test]
fn boot() {
    ok(check::boot(microcode()));
}

/// The emulator's cycle counts (hbc2-core isa.rs) are the microcode's.
#[test]
fn emulator_cycle_counts() {
    use hbc2_core::isa::{self, Instruction};
    let mc = microcode();
    let mut wrong = Vec::new();
    for op in 0..64u8 {
        for mode in 0..16u8 {
            let (lo, hi) = mc.cycles[&(op, mode)];
            // Opcode bits 31-26, mode bits 25-22.
            let i = Instruction::fetched(((op as u32) << 26) | ((mode as u32) << 22));
            let emu = i.cycles() as u32;
            if lo != hi || emu != lo {
                wrong.push(format!(
                    "{op:02X} mode {mode:X}: microcode {lo} / {hi}, emulator {emu}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(
        mc.entry_hw,
        isa::HARDWARE_INTERRUPT_CYCLES as u32,
        "hardware interrupt entry"
    );
    assert_eq!(
        mc.entry_sw,
        isa::SOFTWARE_INTERRUPT_CYCLES as u32,
        "software interrupt entry"
    );
    assert_eq!(mc.boot_steps, isa::HARDWARE_BOOT_CYCLES, "boot");
}
