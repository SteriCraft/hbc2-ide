//! `emutrace random SEED OUT.bin`
//!     a 64 KiB RAM image from hbc2-ucode's random program generator (the
//!     phase 2 generator, `check::random_program`, with the `cu` vectors'
//!     settings), SEED decimal
//! `emutrace trace RAM.bin --max N -o OUT.csv [--int LINE:PORT:DATA]...`
//!     the emulator's trace, as `hbc2 trace` writes it (no peripherals),
//!     except that hardware interrupts are queued in the IOD: PORT with DATA
//!     (hex) just before line LINE runs (the Logisim CPU saw INT while that
//!     line ran). The trace goes on past HLT while an interrupt is queued or
//!     still to come; without --int it is `hbc2 trace`'s.

use hbc2_core::{Machine, MachineConfig, TraceWriter};
use hbc2_ucode::check::{Wild, random_program};
use hbc2_ucode::model::Rng;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.first().map(String::as_str) {
        Some("random") => random(&args[1..]),
        Some("trace") => trace(&args[1..]),
        _ => Err("usage: emutrace random SEED OUT.bin | trace RAM.bin --max N -o OUT.csv [--int LINE:PORT:DATA]...".into()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("emutrace: {e}");
            ExitCode::from(2)
        }
    }
}

fn random(args: &[String]) -> Result<(), String> {
    let seed: u64 = args.first().ok_or("seed")?.parse().map_err(|_| "seed")?;
    let out = args.get(1).ok_or("output file")?;
    let mut r = Rng(0x0C0F_FEE0_0000_0001 ^ (0xC0DE_0000 + seed).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let (ram, _) = random_program(
        &mut r,
        Wild { random_bytes: 5000, jumps: 300, stops: 400, imm16_free: true },
    );
    std::fs::write(out, &ram).map_err(|e| format!("{out}: {e}"))
}

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn trace(args: &[String]) -> Result<(), String> {
    let path = args.first().ok_or("RAM image")?;
    let max: u64 = opt(args, "--max").ok_or("--max N")?.parse().map_err(|_| "--max")?;
    let out = opt(args, "-o").ok_or("-o OUT.csv")?;
    let mut ints: Vec<(u64, u8, u8)> = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if a == "--int" {
            let v: Vec<&str> = args.get(i + 1).ok_or("--int LINE:PORT:DATA")?.split(':').collect();
            let bad = || format!("--int {}", args[i + 1]);
            if v.len() != 3 {
                return Err(bad());
            }
            ints.push((
                v[0].parse().map_err(|_| bad())?,
                u8::from_str_radix(v[1], 16).map_err(|_| bad())?,
                u8::from_str_radix(v[2], 16).map_err(|_| bad())?,
            ));
        }
    }
    ints.sort_by_key(|x| x.0);
    let ram = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let config = MachineConfig { monitor: false, rtc: false, keyboard: false, eeprom: None };
    let mut m = Machine::new(config, Some(&ram)).map_err(|e| e.to_string())?;
    let file = std::fs::File::create(out).map_err(|e| format!("{out}: {e}"))?;
    let mut w = TraceWriter::new(std::io::BufWriter::new(file), Some(max));
    w.start(&m.cpu);
    let mut k = 0;
    while !w.is_full() && w.error().is_none() {
        while k < ints.len() && ints[k].0 <= w.lines() + 1 {
            let (_, port, data) = ints[k];
            let keep = m.board.iod.port(port);
            m.board.iod.set_port(port, data);
            m.board.iod.trigger_interrupt(port);
            m.board.iod.set_port(port, keep);
            k += 1;
        }
        if m.cpu.is_halted() {
            let waiting = m.board.int || m.board.inr || m.board.iod.has_pending_interrupt();
            if m.cpu.is_stopped() || (!waiting && (k == ints.len() || ints[k].0 > w.lines() + 1)) {
                break;
            }
        }
        m.step_with(&mut w);
    }
    let lines = w.lines();
    w.finish().map_err(|e| format!("{out}: {e}"))?;
    println!("{path} -> {out} ({lines} lines)");
    Ok(())
}
