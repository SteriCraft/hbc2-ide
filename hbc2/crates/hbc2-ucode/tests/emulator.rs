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
