//! `hbc2`: command-line tools for the HBC-2.
//!
//!   hbc2 bench [--seconds N]                      measure emulation speed
//!   hbc2 run <ram.bin> [--ticks N] [--eeprom F]   run headless, print the CPU state

use std::process::ExitCode;
use std::time::{Duration, Instant};

use hbc2_core::isa::{PROGRAM_START, encode};
use hbc2_core::peripherals::Eeprom;
use hbc2_core::{Flag, Machine, MachineConfig, Opcode, Register};

const USAGE: &str = "\
usage:
  hbc2 bench [--seconds N]                      measure emulation speed
  hbc2 run <ram.bin> [--ticks N] [--eeprom F]   run headless, print the CPU state";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("bench") => bench(&args[1..]),
        Some("run") => run(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Returns the value following `--name`, if present.
fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn parse<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> Result<T, String> {
    match option(args, name) {
        Some(value) => value
            .parse()
            .map_err(|_| format!("invalid value for {name}: {value}")),
        None => Ok(default),
    }
}

/// A loop mixing ALU, memory, compare/jump and one monitor write per iteration.
fn bench_program() -> Vec<u8> {
    use Opcode as O;
    use Register::*;
    let loop_start = PROGRAM_START + 4 * 4;
    let program = [
        encode::none(O::Cli),           // ignore the plug-in interrupts
        encode::reg_imm8(O::Mov, D, 5), // D = monitor CMD port (monitor is plugged first, ports 1-5)
        encode::reg_imm8(O::Mov, J, 1), // J = monitor WRITE command
        encode::reg_imm8(O::Mov, A, 0),
        // loop:
        encode::reg_imm8(O::Add, A, 1),
        encode::reg_ram(O::Str, A, 0x8000),
        encode::reg_ram(O::Lod, B, 0x8000),
        encode::reg(O::Xor, C, A),
        encode::reg(O::Out, D, J),
        encode::reg(O::Cmp, A, B),
        encode::imm16(O::Jme, loop_start),
        encode::none(O::Hlt),
    ];
    let mut ram = vec![0u8; hbc2_core::isa::RAM_SIZE];
    for (n, instruction) in program.iter().enumerate() {
        let address = PROGRAM_START as usize + n * 4;
        ram[address..address + 4].copy_from_slice(&instruction.to_bytes());
    }
    ram
}

fn bench(args: &[String]) -> Result<(), String> {
    let seconds: f64 = parse(args, "--seconds", 3.0)?;
    let ram = bench_program();
    let mut machine =
        Machine::new(MachineConfig::default(), Some(&ram)).map_err(|e| e.to_string())?;

    const BATCH: u64 = 1_000_000;
    let budget = Duration::from_secs_f64(seconds);
    let start = Instant::now();
    let mut ticks = 0u64;
    while start.elapsed() < budget {
        ticks += machine.run(BATCH).ticks;
    }
    let elapsed = start.elapsed().as_secs_f64();

    if machine.cpu.is_halted() {
        return Err("benchmark program halted unexpectedly".into());
    }
    println!(
        "{ticks} cycles in {elapsed:.2} s -> {:.1} MHz",
        ticks as f64 / elapsed / 1e6
    );
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let path = args.first().filter(|a| !a.starts_with("--")).ok_or(USAGE)?;
    let ticks: u64 = parse(args, "--ticks", 1_000_000)?;
    let ram = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;

    let mut config = MachineConfig::default();
    if let Some(eeprom_path) = option(args, "--eeprom") {
        let image = std::fs::read(eeprom_path).map_err(|e| format!("{eeprom_path}: {e}"))?;
        config.eeprom = Some(Eeprom::from_image(image).map_err(|e| e.to_string())?);
    }
    let mut machine = Machine::new(config, Some(&ram)).map_err(|e| e.to_string())?;
    let result = machine.run(ticks);

    let cpu = &machine.cpu;
    println!("cycles: {}", result.ticks);
    println!(
        "pc: {:#06x}   sp: {:#04x}   last instruction: {:#06x}",
        cpu.pc, cpu.sp, cpu.last_pc
    );
    for r in Register::ALL {
        print!("{}={:#04x} ", r.name(), cpu.reg(r));
    }
    println!();
    let flags: String = Flag::ALL
        .iter()
        .map(|&f| if cpu.flag(f) { f.letter() } else { '-' })
        .collect();
    println!("flags: {flags}");
    Ok(())
}
