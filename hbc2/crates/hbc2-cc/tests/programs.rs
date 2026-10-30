//! Every C program in compiler/tests and compiler/tests/ops, and every
//! program of several files in compiler/tests/multi (one folder each: its
//! `.c` and `.has` files in name order, `main.c` says what is expected):
//! built with the startup code, the runtime and the C library, and run
//! until `__exit` with a monitor, an RTC and a keyboard. The first line of
//! each program says what main returns: "Expected: main returns N". Next to
//! `name.c` (`main.c` in a folder):
//! - `name.screen`: the text the monitor must show at the end (rows
//!   without their trailing spaces, empty rows at the end left out);
//! - `name.keys`: text typed on the keyboard once the program reads keys
//!   (letters as on the keys, without Shift; a new line is Enter).
//!
//! Needs the compiler tools (compiler/build.cmd).

use std::path::{Path, PathBuf};

use hbc2_cc::{Library, build_library, build_program, compiler_dir};
use hbc2_core::peripherals::monitor::{TEXT_COLUMNS, TEXT_ROWS};
use hbc2_core::{Machine, MachineConfig, Register, StopReason};

/// Clock cycles a program may run.
const MAX_TICKS: u64 = 50_000_000;
/// Clock cycles between two keys typed.
const KEY_TICKS: u64 = 20_000;

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

/// The text of the monitor, as in a `.screen` file.
fn screen_text(m: &Machine) -> String {
    let Some(monitor) = &m.peripherals.monitor else {
        return String::new();
    };
    let cells = monitor.text_buffer();
    let mut rows: Vec<String> = (0..TEXT_ROWS)
        .map(|row| {
            let line: String = cells[row * TEXT_COLUMNS..(row + 1) * TEXT_COLUMNS]
                .iter()
                .map(|c| match c.ascii {
                    0x20..=0x7E => c.ascii as char,
                    _ => ' ',
                })
                .collect();
            line.trim_end().to_string()
        })
        .collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows.join("\n")
}

/// Scan code of a character typed (see the Keyboard reference).
fn scan_code(c: char) -> u8 {
    match c {
        '\n' => 0x0D,
        'a'..='z' => c.to_ascii_uppercase() as u8,
        _ => c as u8,
    }
}

/// Runs one program: `sources` (C and assembly files); `source` is the file
/// that says what is expected, `name.keys` and `name.screen` are next to it.
fn run(sources: &[PathBuf], source: &Path, library: &Library) -> Result<Run, String> {
    let stem = source.file_stem().unwrap().to_string_lossy().to_string();
    let parent = source
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy();
    let dir = std::env::temp_dir()
        .join("hbc2-cc-programs")
        .join(format!("{parent}_{stem}"));
    let build = build_program(sources, library, &dir)?;
    let program = &build.program;
    let exit = program.symbol("__exit").expect("__exit").address as u16;
    let kbd_ready = program
        .symbol("term_kbd_ready")
        .expect("term_kbd_ready")
        .address as u16;
    let mut keys: Vec<char> = std::fs::read_to_string(source.with_extension("keys"))
        .map(|t| t.replace('\r', "").chars().collect())
        .unwrap_or_default();
    keys.reverse();
    let mut m = Machine::new(MachineConfig::default(), Some(&program.image)).unwrap();
    m.breakpoints.set(exit, true);
    let mut ticks = 0;
    loop {
        let result = m.run(KEY_TICKS);
        ticks += result.ticks;
        if let Some(StopReason::Breakpoint(_)) = result.stop {
            break;
        }
        if ticks >= MAX_TICKS {
            return Err("main did not return".into());
        }
        // Keys typed once the terminal takes them, one per wait
        if m.board.ram.read(kbd_ready) == 1
            && let Some(c) = keys.pop()
        {
            m.key_event(scan_code(c), true);
            m.key_event(scan_code(c), false);
        }
    }
    if m.cpu.sp != 0xFE00 {
        return Err(format!("stack not balanced: sp = {:#06x}", m.cpu.sp));
    }
    if let Ok(want) = std::fs::read_to_string(source.with_extension("screen")) {
        let want = want.replace('\r', "");
        let want = want.trim_end_matches('\n');
        let got = screen_text(&m);
        if got != want {
            return Err(format!(
                "the screen shows\n{got}\n--- instead of\n{want}\n---"
            ));
        }
    }
    Ok(Run {
        result: u16::from_be_bytes([m.cpu.reg(Register::A), m.cpu.reg(Register::B)]),
        code_bytes: build
            .compiled
            .iter()
            .map(|f| code_bytes(&std::fs::read_to_string(f).unwrap()))
            .sum(),
        ticks,
    })
}

/// The `.c` and `.has` files of a folder, in name order.
fn sources_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "c" || e == "has"))
        .collect();
    files.sort();
    files
}

#[test]
fn programs() {
    let dir = compiler_dir().join("tests");
    // (sources, the file that says what is expected)
    let mut programs: Vec<(Vec<PathBuf>, PathBuf)> = [dir.clone(), dir.join("ops")]
        .iter()
        .flat_map(|d| sources_in(d))
        .filter(|p| p.extension().is_some_and(|e| e == "c"))
        .map(|p| (vec![p.clone()], p))
        .collect();
    let mut folders: Vec<_> = std::fs::read_dir(dir.join("multi"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    for folder in folders {
        programs.push((sources_in(&folder), folder.join("main.c")));
    }
    assert!(!programs.is_empty(), "no C test in {}", dir.display());
    let library = build_library(&std::env::temp_dir().join("hbc2-cc-library"))
        .unwrap_or_else(|e| panic!("library: {e}"));
    let mut failures = Vec::new();
    for (sources, file) in &programs {
        let source = std::fs::read_to_string(file).unwrap();
        let want = expected(&source);
        let name = file
            .strip_prefix(&dir)
            .unwrap_or(file)
            .display()
            .to_string()
            .replace('\\', "/");
        match run(sources, file, &library) {
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
