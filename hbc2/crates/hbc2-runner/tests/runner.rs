//! The emulator thread, seen from the outside: commands in, snapshots out.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use hbc2_asm::{MemoryFiles, Program, assemble};
use hbc2_core::isa::{HARDWARE_BOOT_CYCLES, PROGRAM_START};
use hbc2_core::{Machine, MachineConfig, Register, StopReason};
use hbc2_runner::{RunState, Runner, Snapshot, Speed};

/// Counts in A forever, interrupts disabled.
const COUNTER: &str = "
_start:
    cli
loop:
    add a, 1
    jmp loop
";

/// Sleeps with HLT; the keyboard (alone, so on port 1) handler copies the
/// scan code into A.
const KEYBOARD: &str = "
.data vector0 $0xFE00 ignore.msb, ignore.lsb
.data vector1 $0xFE02 pressed.msb, pressed.lsb
_start:
    hlt
    jmp _start
ignore:
    irt
pressed:
    mov a, i
    irt
";

fn program(source: &str) -> Program {
    assemble(
        Path::new("main.has"),
        &MemoryFiles::new().with("main.has", source),
    )
    .unwrap_or_else(|errors| panic!("{errors:?}"))
}

fn no_peripherals() -> MachineConfig {
    MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
        mailbox: None,
    }
}

fn machine(source: &str, config: MachineConfig) -> Machine {
    let mut m = Machine::new(config, Some(&program(source).image)).unwrap();
    m.finish_boot();
    m
}

fn counter() -> Runner {
    Runner::new(machine(COUNTER, no_peripherals()))
}

/// Polls the runner until `done` holds, or fails after a few seconds.
fn wait_until(runner: &Runner, done: impl Fn(&Snapshot) -> bool) -> Arc<Snapshot> {
    let start = Instant::now();
    loop {
        let snapshot = runner.sync();
        if done(&snapshot) {
            return snapshot;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "timed out; last snapshot: {snapshot:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn starts_paused_at_the_program_start() {
    let runner = counter();
    let s = runner.snapshot();
    assert_eq!(s.state, RunState::Paused);
    assert_eq!(s.cpu.pc, PROGRAM_START);
    assert_eq!(s.ticks, HARDWARE_BOOT_CYCLES, "booted");
}

#[test]
fn step_runs_instructions_then_stays_paused() {
    let runner = counter();
    runner.step(3); // cli, add, jmp
    let s = runner.sync();
    assert_eq!(s.state, RunState::Paused);
    assert_eq!(s.cpu.reg(Register::A), 1);
    assert_eq!(
        s.cpu.pc,
        PROGRAM_START + 1,
        "back to loop, after cli (1 byte)"
    );
    assert_eq!(
        s.ticks,
        HARDWARE_BOOT_CYCLES + 2 + 5 + 5,
        "cli, add Reg / Imm8, jmp Imm16"
    );
}

#[test]
fn runs_fastest_until_paused() {
    let runner = counter();
    runner.set_speed(Speed::Fastest);
    runner.run();
    let running = wait_until(&runner, |s| s.measured_hz > 0.0);
    assert_eq!(running.state, RunState::Running);

    runner.pause();
    let paused = runner.sync();
    assert_eq!(paused.state, RunState::Paused);
    assert_eq!(paused.measured_hz, 0.0);
    assert!(paused.ticks > 100_000, "only {} cycles", paused.ticks);

    // Paused means paused.
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(runner.sync().ticks, paused.ticks);
}

#[test]
fn keeps_the_chosen_clock_speed() {
    let runner = counter();
    runner.set_speed(Speed::Hz(20_000));
    let start = Instant::now();
    runner.run();
    std::thread::sleep(Duration::from_millis(500));
    runner.pause();
    let s = runner.sync();
    let expected = 20_000.0 * start.elapsed().as_secs_f64();
    let ratio = (s.ticks - HARDWARE_BOOT_CYCLES) as f64 / expected;
    assert!(
        (0.7..1.1).contains(&ratio),
        "{} cycles, expected about {expected}",
        s.ticks
    );
}

#[test]
fn the_boot_runs_at_hardware_speed_at_least() {
    let booting = Machine::new(no_peripherals(), Some(&program(COUNTER).image)).unwrap();
    assert!(booting.booting());
    let runner = Runner::new(booting);
    runner.set_speed(Speed::Hz(1_000)); // the boot would last 66 s
    let start = Instant::now();
    runner.run();
    wait_until(&runner, |s| s.ticks > HARDWARE_BOOT_CYCLES);
    let boot = start.elapsed();
    assert!(boot < Duration::from_millis(500), "boot took {boot:?}");

    // The program itself runs at the selected speed.
    std::thread::sleep(Duration::from_millis(300));
    runner.pause();
    let after = runner.sync().ticks - HARDWARE_BOOT_CYCLES;
    let expected = 1_000.0 * (start.elapsed() - boot).as_secs_f64();
    assert!(
        (after as f64) < expected * 1.5 + 20.0,
        "{after} cycles after the boot, expected about {expected}"
    );
}

#[test]
fn breakpoint_pauses_before_the_instruction() {
    let address = program(COUNTER).symbol("loop").unwrap().address as u16;
    let runner = counter();
    runner.set_breakpoint(address, true);
    runner.run();
    let s = wait_until(&runner, |s| s.state == RunState::Paused && s.ticks > 0);
    assert_eq!(s.stop, Some(StopReason::Breakpoint(address)));
    assert_eq!(s.cpu.pc, address);
    assert_eq!(s.cpu.reg(Register::A), 0);
    assert!(s.breakpoints.contains(address));

    // Running again goes once around the loop.
    runner.run();
    let s = wait_until(&runner, |s| s.state == RunState::Paused && s.ticks > 1);
    assert_eq!(s.cpu.reg(Register::A), 1);

    runner.clear_breakpoints();
    runner.step(1);
    let s = runner.sync();
    assert_eq!(s.stop, None, "step clears the stop reason");
    assert!(!s.breakpoints.contains(address));
}

#[test]
fn key_press_wakes_the_halted_cpu() {
    for speed in [Speed::Fastest, Speed::Hz(1_000)] {
        let config = MachineConfig {
            keyboard: true,
            ..no_peripherals()
        };
        let runner = Runner::new(machine(KEYBOARD, config));
        runner.set_speed(speed);
        runner.run();
        wait_until(&runner, |s| {
            s.cpu.is_halted() && s.board.iod.queue().len() == 0
        });

        runner.key(0x2A, true);
        let s = wait_until(&runner, |s| s.cpu.reg(Register::A) == 0x2A);
        assert_eq!(s.state, RunState::Running, "{speed:?}");
    }
}

#[test]
fn notifies_each_new_snapshot() {
    let count = Arc::new(AtomicUsize::new(0));
    let runner = Runner::with_notify(machine(COUNTER, no_peripherals()), {
        let count = count.clone();
        move || {
            count.fetch_add(1, Ordering::Relaxed);
        }
    });
    runner.run();
    std::thread::sleep(Duration::from_millis(200));
    runner.pause();
    runner.sync();
    // About 60 per second while running.
    assert!(count.load(Ordering::Relaxed) >= 5);
}

#[test]
fn load_exec_and_power_cycle() {
    let runner = counter();
    runner.step(10);

    runner.load(machine(KEYBOARD, no_peripherals()));
    let s = runner.sync();
    assert_eq!(s.ticks, HARDWARE_BOOT_CYCLES);
    assert_eq!(s.cpu.pc, PROGRAM_START);

    runner.exec(|m| m.board.ram.write(0x8000, 0x42));
    assert_eq!(runner.sync().board.ram.read(0x8000), 0x42);

    runner.run();
    runner.power_cycle();
    let s = runner.sync();
    assert_eq!(s.ticks, 0);
    assert_eq!(s.state, RunState::Paused, "a restart pauses");
    assert_eq!(s.board.ram.read(0x8000), 0, "RAM image reloaded");
}

#[test]
fn monitor_copy_is_shared_while_unchanged() {
    let config = MachineConfig {
        monitor: true,
        ..no_peripherals()
    };
    let runner = Runner::new(machine(COUNTER, config));
    let a = runner.sync();
    runner.step(5);
    let b = runner.sync();
    assert!(Arc::ptr_eq(
        a.monitor.as_ref().unwrap(),
        b.monitor.as_ref().unwrap()
    ));
}

#[test]
fn dropping_a_running_runner_stops_its_thread() {
    let runner = counter();
    runner.set_speed(Speed::Fastest);
    runner.run();
    std::thread::sleep(Duration::from_millis(20));
    drop(runner); // joins the thread: the test would hang if it did not stop
}

/// Runs a short loop 5 times, then sits in an endless one.
const LINES: &str = "
_start:
    cli
    mov b, 5
again:
    add a, 1
    sub b, 1
    jmz done
    jmp again
done:
    nop
forever:
    jmp forever
";

#[test]
fn step_within_runs_until_the_pc_leaves_the_addresses() {
    let p = program(LINES);
    let at = |name: &str| p.symbol(name).unwrap().address as u16;
    let runner = Runner::new(machine(LINES, no_peripherals()));
    runner.set_speed(Speed::Fastest);
    runner.step(2); // cli, mov b, 5

    // The four instructions of the loop: it goes round 5 times.
    let lop: Vec<u16> = p
        .lines
        .iter()
        .map(|l| l.address as u16)
        .filter(|a| (at("again")..at("done")).contains(a))
        .collect();
    assert_eq!(lop.len(), 4);
    runner.step_within(lop);
    let s = wait_until(&runner, |s| s.state == RunState::Paused && s.ticks > 2);
    assert_eq!(s.cpu.pc, at("done"));
    assert_eq!(s.cpu.reg(Register::A), 5);
    assert_eq!(s.stop, None);

    // Already out after one instruction: a plain step.
    runner.step_within(vec![at("again")]);
    let s = runner.sync();
    assert_eq!((s.state, s.cpu.pc), (RunState::Paused, at("forever")));

    // A line that never ends: running until paused.
    runner.step_within(vec![at("forever")]);
    let s = wait_until(&runner, |s| s.ticks > 10_000);
    assert_eq!(s.state, RunState::Running);
    runner.pause();
    assert_eq!(runner.sync().state, RunState::Paused);
    // Run afterwards is a plain run.
    runner.run();
    assert_eq!(runner.sync().state, RunState::Running);
}

/// Nested calls, a push, and a breakpoint in a routine.
const CALLS: &str = "
_start:
    cli
    mov a, 0
    cal outer
after:
    psh a
    pop a
    cal deep
back:
    nop
forever:
    jmp forever
outer:
    cal inner
    cal inner
    ret
inner:
    add a, 1
    ret
deep:
    cal inner
marked:
    nop
    ret
";

#[test]
fn step_over_runs_the_routines_called() {
    let p = program(CALLS);
    let at = |name: &str| p.symbol(name).unwrap().address as u16;
    let runner = Runner::new(machine(CALLS, no_peripherals()));
    runner.set_speed(Speed::Fastest);
    runner.step(2); // cli, mov a, 0

    // Over a CAL: the whole routine (and the ones it calls) runs.
    runner.step_over(vec![]);
    let s = wait_until(&runner, |s| {
        s.state == RunState::Paused && s.cpu.pc != at("after") - 4 // the cal
    });
    assert_eq!(s.cpu.pc, at("after"));
    assert_eq!(s.cpu.reg(Register::A), 2);
    assert_eq!(s.stop, None);

    // Over a PSH: a plain step, though the stack grew.
    runner.step_over(vec![]);
    let s = runner.sync();
    assert_eq!((s.state, s.cpu.pc), (RunState::Paused, at("after") + 2));
    runner.step(1); // pop a

    // A breakpoint in a routine called stops the step there.
    runner.set_breakpoint(at("marked"), true);
    runner.step_over(vec![]);
    let s = wait_until(&runner, |s| {
        s.state == RunState::Paused && s.cpu.pc != at("back") - 4
    });
    assert_eq!(s.cpu.pc, at("marked"));
    assert_eq!(s.stop, Some(StopReason::Breakpoint(at("marked"))));
    assert_eq!(s.cpu.reg(Register::A), 3);

    // Over the routine's RET: back in the caller.
    runner.step_over(vec![]);
    runner.step_over(vec![]);
    let s = wait_until(&runner, |s| {
        s.state == RunState::Paused && s.cpu.pc == at("back")
    });
    assert_eq!(s.stop, None);

    // A line jumping to itself: over it, one instruction at a time.
    runner.step(1); // nop
    runner.step_over(vec![]);
    let s = runner.sync();
    assert_eq!((s.state, s.cpu.pc), (RunState::Paused, at("forever")));

    // The addresses of a line: it runs while the PC stays on them.
    runner.step_over(vec![at("forever")]);
    let s = wait_until(&runner, |s| s.ticks > 10_000);
    assert_eq!(s.state, RunState::Running);
    runner.pause();
}

#[test]
fn step_out_runs_until_the_routine_returns() {
    let p = program(CALLS);
    let at = |name: &str| p.symbol(name).unwrap().address as u16;
    let runner = Runner::new(machine(CALLS, no_peripherals()));
    runner.set_speed(Speed::Fastest);
    runner.step(4); // cli, mov a, 0, cal outer, cal inner
    assert_eq!(runner.sync().cpu.pc, at("inner"));

    // Out of inner: back in outer, after its first CAL.
    runner.step_out();
    let s = wait_until(&runner, |s| {
        s.state == RunState::Paused && s.cpu.pc != at("inner")
    });
    assert_eq!(s.cpu.pc, at("outer") + 4);
    assert_eq!(s.cpu.reg(Register::A), 1);

    // Out of outer: its second call to inner runs on the way.
    runner.step_out();
    let s = wait_until(&runner, |s| {
        s.state == RunState::Paused && s.cpu.pc == at("after")
    });
    assert_eq!(s.cpu.reg(Register::A), 2);
    assert_eq!(s.stop, None);
}

fn trace_file(name: &str) -> (std::fs::File, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("hbc2-runner-{}-{name}.csv", std::process::id()));
    (std::fs::File::create(&path).unwrap(), path)
}

/// The lines of a trace file after the header.
fn trace_lines(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut lines = text.lines().map(str::to_string);
    assert_eq!(lines.next().as_deref(), Some(hbc2_core::TRACE_HEADER));
    lines.collect()
}

#[test]
fn trace_records_steps_until_stopped() {
    let runner = counter();
    let (file, path) = trace_file("steps");
    runner.trace_start(file, path.clone(), 1000);
    assert!(!runner.sync().trace.as_ref().unwrap().started);
    runner.step(3);
    assert_eq!(runner.sync().trace.as_ref().unwrap().lines, 3);
    runner.trace_stop();
    let snapshot = runner.sync();
    assert!(snapshot.trace.is_none());
    assert_eq!(snapshot.trace_ends, 1);
    let end = snapshot.trace_end.as_ref().unwrap();
    assert_eq!(
        (end.lines, &end.reason),
        (3, &hbc2_runner::TraceEndReason::Stopped)
    );
    let lines = trace_lines(&path);
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "0,0000,,00,00,00,00,00,00,00,00,FE00,04,,");
    assert!(lines[2].starts_with("2,0001,"), "add a, 1: {}", lines[2]);
    assert!(lines[2].contains(",01,00,"), "A = 1: {}", lines[2]);
    let _ = std::fs::remove_file(path);
}

#[test]
fn trace_stops_at_its_limit_while_running() {
    let runner = counter();
    runner.set_speed(Speed::Fastest);
    let (file, path) = trace_file("limit");
    runner.trace_start(file, path.clone(), 50);
    runner.run();
    let snapshot = wait_until(&runner, |s| s.trace_ends == 1);
    let end = snapshot.trace_end.as_ref().unwrap();
    assert_eq!(
        (end.lines, &end.reason),
        (50, &hbc2_runner::TraceEndReason::Limit)
    );
    assert_eq!(snapshot.state, RunState::Running, "only the trace stops");
    let lines = trace_lines(&path);
    assert_eq!(lines.len(), 51);
    assert!(lines[50].starts_with("50,"));
    runner.pause();
    let _ = std::fs::remove_file(path);
}

#[test]
fn trace_waits_through_a_reload_and_ends_on_reset() {
    let runner = Runner::new(machine("_start:\n    hlt\n", no_peripherals()));
    let (file, path) = trace_file("reset");
    runner.trace_start(file, path.clone(), 1000);
    // Not started yet: loading the program (as Run does) keeps it.
    runner.load(machine(COUNTER, no_peripherals()));
    runner.step(1);
    let snapshot = runner.sync();
    assert_eq!(snapshot.trace_ends, 0);
    assert_eq!(snapshot.trace.as_ref().unwrap().lines, 1);
    runner.power_cycle();
    let snapshot = runner.sync();
    assert!(snapshot.trace.is_none());
    let end = snapshot.trace_end.as_ref().unwrap();
    assert_eq!(
        (end.lines, &end.reason),
        (1, &hbc2_runner::TraceEndReason::Reset)
    );
    let lines = trace_lines(&path);
    assert_eq!(lines.len(), 2);
    assert!(
        lines[1].starts_with("1,0000,"),
        "cli of the loaded program: {}",
        lines[1]
    );
    let _ = std::fs::remove_file(path);
}
