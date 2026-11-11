//! Cycles between the calls of `mark()` in a C program (a folder of .c and
//! .has files): `cargo run --release -p hbc2-cc --example screen_bench <dir>`.

use std::path::PathBuf;

use hbc2_cc::{build_library, build_program};
use hbc2_core::machine::StopReason;
use hbc2_core::{Machine, MachineConfig};

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("a folder"));
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "c" || e == "has"))
        .collect();
    sources.sort();
    let temp = std::env::temp_dir().join("hbc2-screen-bench");
    let library = build_library(&temp.join("library")).unwrap_or_else(|e| panic!("{e:?}"));
    let build = build_program(&sources, &library, &temp.join("program"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let program = &build.program;
    let mark = program.symbol("_mark").expect("_mark").address as u16;
    let exit = program.symbol("__exit").expect("__exit").address as u16;
    let mut m = Machine::new(MachineConfig::default(), Some(&program.image)).unwrap();
    m.breakpoints.set(mark, true);
    m.breakpoints.set(exit, true);
    let mut last = None;
    loop {
        let result = m.run(100_000_000);
        let Some(StopReason::Breakpoint(pc)) = result.stop else {
            panic!("no mark after 100,000,000 cycles");
        };
        if pc == exit {
            break;
        }
        if let Some(start) = last {
            let cycles = m.ticks() - start;
            println!("{cycles:>10} cycles  {:>9.3} ms", cycles as f64 / 2000.0);
        }
        last = Some(m.ticks());
        m.step(); // past the breakpoint
    }
}
