//! Every C program in compiler/tests and compiler/tests/ops: compiled, assembled with the startup
//! code and the runtime, and run without peripherals until `__exit`. The
//! first line of each file says what main returns: "Expected: main returns N".
//! Needs the compiler tools (compiler/build.cmd).

use std::path::Path;

use hbc2_asm::{Target, assemble_file};
use hbc2_cc::{compile, compiler_dir, runtime_dir};
use hbc2_core::{Machine, MachineConfig, Register, StopReason};

fn slashes(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn expected(source: &str) -> u16 {
    let line = source.lines().next().unwrap_or_default();
    let n = line
        .split("main returns ")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit() && c != '-').next())
        .unwrap_or_else(|| panic!("no 'Expected: main returns N' on the first line"));
    n.parse::<i32>().unwrap() as u16
}

/// What a run gives: main's result, the size of the compiled code, the time.
struct Run {
    result: u16,
    code_bytes: usize,
    ticks: u64,
}

/// Instructions in a compiled file (4 bytes each): indented lines that are
/// not directives or comments. `psh`/`pop` with a register list is one
/// instruction per register.
fn code_bytes(asm: &str) -> usize {
    asm.lines()
        .filter(|l| l.starts_with('\t') && !l.trim_start().starts_with(['.', ';']))
        .map(|l| {
            let l = l.trim_start();
            if l.starts_with("psh ") || l.starts_with("pop ") {
                l.matches(',').count() + 1
            } else {
                1
            }
        })
        .sum::<usize>()
        * 4
}

/// Runs one program.
fn run(source: &Path) -> Result<Run, String> {
    let dir = std::env::temp_dir().join("hbc2-cc-programs");
    std::fs::create_dir_all(&dir).unwrap();
    let stem = source.file_stem().unwrap().to_string_lossy().to_string();
    let compiled = dir.join(format!("{stem}.has"));
    compile(source, &compiled, 1)?;
    let main = dir.join(format!("{stem}_main.has"));
    let rt = runtime_dir();
    std::fs::write(
        &main,
        format!(
            ".include \"{}\"\n.include \"{}\"\n.include \"{}\"\n",
            slashes(&rt.join("crt0.has")),
            slashes(&rt.join("runtime.has")),
            slashes(&compiled)
        ),
    )
    .unwrap();
    let program = assemble_file(&main, Target::Ram).map_err(|e| {
        e.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let exit = program.symbol("__exit").expect("__exit").address as u16;
    let config = MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    };
    let mut m = Machine::new(config, Some(&program.image)).unwrap();
    m.breakpoints.set(exit, true);
    let result = m.run(50_000_000);
    match result.stop {
        Some(StopReason::Breakpoint(_)) => {}
        None => return Err("main did not return".into()),
    }
    if m.cpu.sp != 0xFE00 {
        return Err(format!("stack not balanced: sp = {:#06x}", m.cpu.sp));
    }
    Ok(Run {
        result: u16::from_be_bytes([m.cpu.reg(Register::A), m.cpu.reg(Register::B)]),
        code_bytes: code_bytes(&std::fs::read_to_string(&compiled).unwrap()),
        ticks: result.ticks,
    })
}

#[test]
fn programs() {
    let dir = compiler_dir().join("tests");
    let mut files: Vec<_> = [dir.clone(), dir.join("ops")]
        .iter()
        .flat_map(|d| std::fs::read_dir(d).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "c"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no C test in {}", dir.display());
    let mut failures = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).unwrap();
        let want = expected(&source);
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        match run(file) {
            Ok(run) if run.result == want => println!(
                "{name}: ok ({} bytes of code, {} ticks)",
                run.code_bytes, run.ticks
            ),
            Ok(Run { result: got, .. }) => failures.push(format!(
                "{name}: main returned {got} ({:#06x}), expected {want}",
                got
            )),
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
