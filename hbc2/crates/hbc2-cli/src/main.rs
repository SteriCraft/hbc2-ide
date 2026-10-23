//! `hbc2`: command-line tools for the HBC-2. See [`USAGE`].

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use hbc2_asm::{Program, Target};
use hbc2_core::isa::{PROGRAM_START, encode};
use hbc2_core::peripherals::Eeprom;
use hbc2_core::{Flag, Machine, MachineConfig, Opcode, Register};

const USAGE: &str = "\
usage:
  hbc2 asm <main.has> [-o out.bin] [--eeprom] [--symbols]
        assemble (64 KiB RAM image, or 1 MiB EEPROM image with --eeprom)
  hbc2 run <main.has | ram.bin> [--ticks N] [--eeprom <file.has | eeprom.bin>]
        run headless, then print the CPU state
  hbc2 bench [--seconds N]
        measure emulation speed";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("asm") => asm(&args[1..]),
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

fn has(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

/// The first argument, if it is not an option.
fn input(args: &[String]) -> Result<&str, String> {
    args.first()
        .map(String::as_str)
        .filter(|a| !a.starts_with('-'))
        .ok_or_else(|| USAGE.to_string())
}

/// Assembles a file, printing warnings; errors are returned as one message.
fn assemble(path: &str, target: Target) -> Result<Program, String> {
    match hbc2_asm::assemble_file(path, target) {
        Ok(program) => {
            for warning in &program.warnings {
                eprintln!("{warning}");
            }
            Ok(program)
        }
        Err(errors) => {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            Err(format!(
                "{}\nassembly failed ({} error(s))",
                lines.join("\n"),
                errors.len()
            ))
        }
    }
}

fn is_source(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("has"))
}

fn asm(args: &[String]) -> Result<(), String> {
    let path = input(args)?;
    let target = if has(args, "--eeprom") {
        Target::Eeprom
    } else {
        Target::Ram
    };
    let program = assemble(path, target)?;

    let output = option(args, "-o")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(path).with_extension("bin"));
    std::fs::write(&output, &program.image).map_err(|e| format!("{}: {e}", output.display()))?;
    println!(
        "{} -> {} ({} bytes used, {} files)",
        path,
        output.display(),
        program.bytes_used,
        program.files.len()
    );

    if has(args, "--symbols") {
        for s in &program.symbols {
            let kind = match s.kind {
                hbc2_asm::SymbolKind::Label => "label",
                hbc2_asm::SymbolKind::Data => "data ",
            };
            println!(
                "  ${:#07x}  {kind}  {:>5} bytes  {}",
                s.address, s.size, s.name
            );
        }
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let path = input(args)?;
    let ticks: u64 = parse(args, "--ticks", 1_000_000)?;

    let mut config = MachineConfig::default();
    if let Some(eeprom_path) = option(args, "--eeprom") {
        let image = if is_source(eeprom_path) {
            assemble(eeprom_path, Target::Eeprom)?.image
        } else {
            std::fs::read(eeprom_path).map_err(|e| format!("{eeprom_path}: {e}"))?
        };
        config.eeprom = Some(Eeprom::from_image(image).map_err(|e| e.to_string())?);
    }
    let (ram, program) = if is_source(path) {
        let program = assemble(path, Target::Ram)?;
        (program.image.clone(), Some(program))
    } else {
        (
            std::fs::read(path).map_err(|e| format!("{path}: {e}"))?,
            None,
        )
    };

    let mut machine = Machine::new(config, Some(&ram)).map_err(|e| e.to_string())?;
    let result = machine.run(ticks);

    let cpu = &machine.cpu;
    let name_of = |address: u16| {
        program
            .as_ref()
            .and_then(|p| p.symbol_at(address as u32))
            .map(|s| s.name.clone())
    };
    println!("cycles: {}", result.ticks);
    println!(
        "pc: {:#06x}   sp: {:#04x}   last instruction: {:#06x}  {}",
        cpu.pc,
        cpu.sp,
        cpu.last_pc,
        hbc2_asm::disasm::disassemble_with(cpu.ir.0, name_of)
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
