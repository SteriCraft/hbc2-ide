//! The emulator thread, seen from the outside: commands in, snapshots out.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use hbc2_asm::{MemoryFiles, Program, Target, assemble};
use hbc2_core::isa::PROGRAM_START;
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
        Target::Ram,
    )
    .unwrap_or_else(|errors| panic!("{errors:?}"))
}

fn no_peripherals() -> MachineConfig {
    MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    }
}

fn machine(source: &str, config: MachineConfig) -> Machine {
    Machine::new(config, Some(&program(source).image)).unwrap()
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
    assert_eq!(s.ticks, 0);
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
    assert_eq!(s.ticks, 3);
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
    let ratio = s.ticks as f64 / expected;
    assert!(
        (0.7..1.1).contains(&ratio),
        "{} cycles, expected about {expected}",
        s.ticks
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
    assert_eq!(s.ticks, 0);
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
