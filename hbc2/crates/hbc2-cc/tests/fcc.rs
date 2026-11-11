//! FCC's own test programs (upstream `test/`, copied into
//! compiler/tests/fcc, see compiler/fcc/UPSTREAM.md), run as upstream's
//! scripts run them:
//! - `tests/*.c`: they check themselves; main returns 0 when they pass;
//! - `wtests/input*.c` (from Warren Toomey's acwj compiler), built with
//!   `cprintf.c`: they print with `printint` / `printchar` (here
//!   compiler/tests/fcc/support.c, which keeps the text in `fcc_out`) and
//!   the text must be `out.inputNNN.c`.
//!
//! Each program is built with the startup code, the runtime and the C
//! library, and runs until `__exit`. The programs in `SKIPPED` are not run;
//! the reason is given (a program in that list that passes is an error, so
//! the list stays true).
//!
//! Needs the compiler tools (compiler/build.cmd).

use std::path::{Path, PathBuf};

use hbc2_cc::{Library, build_library, build_program, compiler_dir, report};
use hbc2_core::{Machine, MachineConfig, Register, StopReason};

/// Clock cycles a program may run.
const MAX_TICKS: u64 = 50_000_000;

/// Programs not run, and why (the path under compiler/tests/fcc).
const SKIPPED: &[(&str, &str)] = &[];

/// What a run gives: main's result and the text printed.
struct Run {
    result: u16,
    output: String,
}

fn run(sources: &[PathBuf], name: &str, library: &Library) -> Result<Run, String> {
    let dir = std::env::temp_dir()
        .join("hbc2-cc-fcc")
        .join(name.replace(['/', '.'], "_"));
    let build = build_program(sources, library, &dir).map_err(|e| report(&e))?;
    let program = &build.program;
    let exit = program.symbol("__exit").expect("__exit").address as u16;
    let mut m = Machine::new(MachineConfig::default(), Some(&program.image)).unwrap();
    m.breakpoints.set(exit, true);
    let result = m.run(MAX_TICKS);
    if !matches!(result.stop, Some(StopReason::Breakpoint(_))) {
        return Err("main did not return".into());
    }
    if m.cpu.sp != 0xFE00 {
        return Err(format!("stack not balanced: sp = {:#06x}", m.cpu.sp));
    }
    let mut output = String::new();
    if let Some(out) = program.symbol("_fcc_out") {
        let len = program.symbol("_fcc_len").expect("_fcc_len").address as u16;
        let len = u16::from_be_bytes([m.board.ram.read(len), m.board.ram.read(len + 1)]);
        output = (0..len)
            .map(|i| m.board.ram.read(out.address as u16 + i) as char)
            .collect();
    }
    Ok(Run {
        result: u16::from_be_bytes([m.cpu.reg(Register::A), m.cpu.reg(Register::B)]),
        output,
    })
}

fn c_files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "c"))
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix))
        .collect();
    files.sort();
    files
}

#[test]
fn fcc_tests() {
    let dir = compiler_dir().join("tests").join("fcc");
    let support = dir.join("support.c");
    let cprintf = dir.join("wtests").join("cprintf.c");
    let library = build_library(&std::env::temp_dir().join("hbc2-cc-library"))
        .unwrap_or_else(|e| panic!("library: {}", report(&e)));
    let (mut ok, mut skipped) = (0, 0);
    let mut failures = Vec::new();
    // (name, sources, expected output: None = main returns 0)
    let mut programs: Vec<(String, Vec<PathBuf>, Option<PathBuf>)> = Vec::new();
    for file in c_files(&dir.join("tests"), "") {
        let name = format!("tests/{}", file.file_name().unwrap().to_string_lossy());
        programs.push((name, vec![file], None));
    }
    for file in c_files(&dir.join("wtests"), "input") {
        let base = file.file_name().unwrap().to_string_lossy().to_string();
        let out = file.with_file_name(format!("out.{base}"));
        programs.push((
            format!("wtests/{base}"),
            vec![support.clone(), cprintf.clone(), file],
            Some(out),
        ));
    }
    for (name, sources, out) in &programs {
        let skip = SKIPPED.iter().find(|(n, _)| n == name).map(|(_, why)| *why);
        let outcome = match run(sources, name, &library) {
            Err(e) => Err(e),
            Ok(run) => match out {
                None if run.result == 0 => Ok(()),
                None => Err(format!("main returned {} ({:#06x})", run.result, run.result)),
                Some(out) => {
                    let want = std::fs::read_to_string(out).unwrap().replace('\r', "");
                    if run.output == want {
                        Ok(())
                    } else {
                        Err(format!("printed\n{}--- instead of\n{want}---", run.output))
                    }
                }
            },
        };
        match (outcome, skip) {
            (Ok(()), None) => {
                ok += 1;
                println!("{name}: ok");
            }
            (Err(e), None) => failures.push(format!("{name}: {e}")),
            (Err(_), Some(why)) => {
                skipped += 1;
                println!("{name}: skipped ({why})");
            }
            (Ok(()), Some(_)) => failures.push(format!("{name}: passes, take it out of SKIPPED")),
        }
    }
    println!("{ok} ok, {skipped} skipped, {} failed", failures.len());
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
