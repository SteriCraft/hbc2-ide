//! The emulator thread: owns the machine, handles commands, keeps the pace.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike};
use hbc2_core::Machine;
use hbc2_core::peripherals::rtc::DateTime;
use hbc2_core::peripherals::{Eeprom, Monitor};

use crate::{Command, RunState, Shared, Snapshot, Speed};

/// Most cycles run between two looks at the command channel
/// (about 1-2 ms of work in a release build).
const SLICE: u64 = 200_000;
/// Time between two snapshots while running (about 60 per second).
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(16);
/// Window over which the reached frequency is measured.
const METER_WINDOW: Duration = Duration::from_millis(250);
/// Longest sleep while running, so commands are never left waiting long.
const MAX_SLEEP: Duration = Duration::from_millis(10);
/// Shortest sleep: shorter ones cost more than they are worth.
const MIN_SLEEP: Duration = Duration::from_millis(1);
/// When the host cannot keep up with the target speed, the delay is
/// forgiven beyond this, instead of being caught up in a long burst.
const MAX_LATE: Duration = Duration::from_millis(100);
/// Time between two looks at the host clock for the RTC while running.
const RTC_SYNC_INTERVAL: Duration = Duration::from_millis(10);
/// Time between two looks at the host clock for the RTC while paused.
const PAUSED_RTC_SYNC_INTERVAL: Duration = Duration::from_millis(200);

pub(crate) struct Worker {
    machine: Machine,
    commands: Receiver<Command>,
    state: RunState,
    stop: Option<hbc2_core::StopReason>,
    speed: Speed,
    /// While running a `StepWithin`: the addresses it stays on.
    within: Option<Vec<u16>>,

    /// Pace keeping: cycles run since `epoch`.
    epoch: Instant,
    paced_ticks: u64,
    /// Last time the RTC was synced with the host clock.
    rtc_synced: Instant,

    /// Frequency meter.
    meter_start: Instant,
    meter_ticks: u64,
    measured_hz: f64,

    seq: u64,
    last_publish: Instant,
    /// Something changed since the last snapshot.
    dirty: bool,
    /// Sleep before the next slice.
    sleep: Option<Duration>,
    /// Copies reused while the peripheral content does not change.
    monitor_copy: Option<(u64, Arc<Monitor>)>,
    eeprom_copy: Option<(u64, Arc<Eeprom>)>,
}

enum Flow {
    Continue,
    Exit,
}

impl Worker {
    pub(crate) fn new(machine: Machine, commands: Receiver<Command>) -> Worker {
        let now = Instant::now();
        let mut worker = Worker {
            machine,
            commands,
            state: RunState::Paused,
            stop: None,
            speed: Speed::default(),
            within: None,
            epoch: now,
            paced_ticks: 0,
            rtc_synced: now,
            meter_start: now,
            meter_ticks: 0,
            measured_hz: 0.0,
            seq: 0,
            last_publish: now,
            dirty: false,
            sleep: None,
            monitor_copy: None,
            eeprom_copy: None,
        };
        worker.sync_rtc(false);
        worker
    }

    pub(crate) fn main_loop(mut self, shared: &Shared) {
        loop {
            let due = match self.state {
                RunState::Paused => self.dirty,
                RunState::Running => self.last_publish.elapsed() >= SNAPSHOT_INTERVAL,
            };
            if due {
                shared.publish(self.snapshot());
            }

            let command = match self.state {
                // Nothing to run until told otherwise: wait, only waking up
                // to keep the RTC on time.
                RunState::Paused => match self.commands.recv_timeout(PAUSED_RTC_SYNC_INTERVAL) {
                    Ok(command) => Some(command),
                    Err(RecvTimeoutError::Timeout) => {
                        if self.sync_rtc(false) {
                            self.dirty = true;
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return, // the Runner is gone
                },
                RunState::Running => {
                    if let Some(duration) = self.sleep.take() {
                        std::thread::sleep(duration);
                    }
                    match self.commands.try_recv() {
                        Ok(command) => Some(command),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => return,
                    }
                }
            };

            match command {
                // Handle every waiting command before running again.
                Some(command) => {
                    if let Flow::Exit = self.handle(command, shared) {
                        return;
                    }
                }
                None => self.run_slice(),
            }
        }
    }

    fn handle(&mut self, command: Command, shared: &Shared) -> Flow {
        self.dirty = true;
        match command {
            Command::Run => {
                self.stop = None;
                self.within = None;
                self.resume();
            }
            Command::Pause => self.pause(),
            Command::Step(instructions) => {
                self.pause();
                self.stop = None;
                for _ in 0..instructions {
                    self.machine.step();
                }
            }
            Command::StepWithin(addresses) => {
                self.pause();
                self.stop = None;
                self.machine.step();
                if addresses.binary_search(&self.machine.cpu.pc).is_ok() {
                    self.within = Some(addresses);
                    self.resume();
                }
            }
            Command::PowerCycle => {
                self.pause();
                self.stop = None;
                self.machine.power_on();
                self.sync_rtc(false);
                self.forget_copies();
                self.restart_pacing();
            }
            Command::Load(machine) => {
                self.pause();
                self.stop = None;
                self.machine = *machine;
                self.sync_rtc(false);
                self.forget_copies();
            }
            Command::SetSpeed(speed) => {
                self.speed = match speed {
                    Speed::Hz(hz) => Speed::Hz(hz.max(1)),
                    fastest => fastest,
                };
                self.restart_pacing();
            }
            Command::SetBreakpoint(address, enabled) => {
                self.machine.breakpoints.set(address, enabled)
            }
            Command::ClearBreakpoints => self.machine.breakpoints.clear(),
            Command::Key { scan_code, pressed } => {
                self.machine.key_event(scan_code, pressed);
            }
            Command::Exec(f) => {
                f(&mut self.machine);
                self.forget_copies(); // `f` may have replaced a peripheral
            }
            Command::Sync(reply) => {
                let snapshot = self.snapshot();
                shared.publish(snapshot.clone());
                let _ = reply.send(snapshot);
            }
            Command::Shutdown => return Flow::Exit,
        }
        Flow::Continue
    }

    fn resume(&mut self) {
        if self.state == RunState::Paused {
            self.state = RunState::Running;
            self.restart_pacing();
        }
    }

    fn pause(&mut self) {
        self.within = None;
        if self.state == RunState::Running {
            self.report_host_time();
            self.state = RunState::Paused;
            self.measured_hz = 0.0;
        }
    }

    fn restart_pacing(&mut self) {
        let now = Instant::now();
        self.epoch = now;
        self.paced_ticks = 0;
        self.meter_start = now;
        self.meter_ticks = 0;
    }

    fn forget_copies(&mut self) {
        self.monitor_copy = None;
        self.eeprom_copy = None;
    }

    /// Runs the machine for a short while, or decides how long to sleep.
    fn run_slice(&mut self) {
        let budget = match self.speed {
            Speed::Fastest => {
                if self.machine.is_idle() {
                    // Halted, waiting for a key or the RTC: no need to spin.
                    self.sleep = Some(MIN_SLEEP);
                    self.report_host_time();
                    return;
                }
                SLICE
            }
            Speed::Hz(hz) => {
                let elapsed = self.epoch.elapsed();
                let due = (elapsed.as_nanos() * hz as u128 / 1_000_000_000) as u64;
                if due <= self.paced_ticks {
                    // Ahead of time: sleep until the next cycle is due.
                    let next = Duration::from_nanos(
                        ((self.paced_ticks + 1) as u128 * 1_000_000_000 / hz as u128) as u64,
                    );
                    self.sleep = Some(next.saturating_sub(elapsed).clamp(MIN_SLEEP, MAX_SLEEP));
                    self.report_host_time();
                    return;
                }
                let late = due - self.paced_ticks;
                if Duration::from_nanos((late as u128 * 1_000_000_000 / hz as u128) as u64)
                    > MAX_LATE
                {
                    // The host cannot keep up: start counting again from now.
                    self.epoch = Instant::now();
                    self.paced_ticks = 0;
                }
                late.min(SLICE)
            }
        };

        let result = match &self.within {
            Some(addresses) => run_within(&mut self.machine, budget, addresses),
            None => self.machine.run(budget),
        };
        let left = self
            .within
            .as_ref()
            .is_some_and(|a| a.binary_search(&self.machine.cpu.pc).is_err());
        self.paced_ticks += result.ticks;
        self.meter_ticks += result.ticks;
        self.report_host_time();
        self.update_meter();
        if let Some(stop) = result.stop {
            self.pause();
            self.stop = Some(stop);
            self.dirty = true;
        } else if left {
            self.pause();
            self.dirty = true;
        }
    }

    /// While running: keeps the RTC on the host's local time, which sends
    /// its once-per-second interrupt (unless the speed is too low for it).
    /// Cheap enough to call after every slice.
    fn report_host_time(&mut self) {
        if self.rtc_synced.elapsed() >= RTC_SYNC_INTERVAL {
            self.sync_rtc(self.speed.sends_rtc_interrupts());
        }
    }

    /// Sets the RTC to the host's local time. Returns true if it changed.
    fn sync_rtc(&mut self, interrupt: bool) -> bool {
        self.rtc_synced = Instant::now();
        self.machine.sync_rtc(host_time(), interrupt)
    }

    fn update_meter(&mut self) {
        let elapsed = self.meter_start.elapsed();
        if elapsed >= METER_WINDOW {
            self.measured_hz = self.meter_ticks as f64 / elapsed.as_secs_f64();
            self.meter_start = Instant::now();
            self.meter_ticks = 0;
        }
    }

    pub(crate) fn snapshot(&mut self) -> Arc<Snapshot> {
        self.seq += 1;
        self.dirty = false;
        self.last_publish = Instant::now();

        let peripherals = &self.machine.peripherals;
        let monitor = peripherals
            .monitor
            .as_ref()
            .map(|m| reuse(&mut self.monitor_copy, m, m.generation()));
        let eeprom = peripherals
            .eeprom
            .as_ref()
            .map(|e| reuse(&mut self.eeprom_copy, e, e.generation()));

        Arc::new(Snapshot {
            seq: self.seq,
            state: self.state,
            stop: self.stop,
            speed: self.speed,
            measured_hz: self.measured_hz,
            ticks: self.machine.ticks(),
            cpu: self.machine.cpu.clone(),
            board: self.machine.board.clone(),
            breakpoints: self.machine.breakpoints.clone(),
            monitor,
            rtc: peripherals.rtc.clone(),
            keyboard: peripherals.keyboard.clone(),
            eeprom,
        })
    }
}

/// Runs about `budget` cycles, one instruction at a time, stopping as soon
/// as the PC is not one of `addresses` (sorted).
fn run_within(machine: &mut Machine, budget: u64, addresses: &[u16]) -> hbc2_core::RunResult {
    let mut ticks = 0;
    while ticks < budget && addresses.binary_search(&machine.cpu.pc).is_ok() {
        ticks += machine.step();
    }
    hbc2_core::RunResult { ticks, stop: None }
}

/// The host's local date and time (its time zone, daylight saving included).
fn host_time() -> DateTime {
    let now = chrono::Local::now();
    DateTime {
        year: now.year().clamp(1900, 2155) as u16,
        month: now.month() as u8,
        day: now.day() as u8,
        hour: now.hour() as u8,
        minute: now.minute() as u8,
        // A leap second shows as 60: keep it in range.
        second: now.second().min(59) as u8,
    }
}

/// Returns the kept copy if `generation` did not change, else a new copy.
fn reuse<T: Clone>(copy: &mut Option<(u64, Arc<T>)>, value: &T, generation: u64) -> Arc<T> {
    match copy {
        Some((kept, arc)) if *kept == generation => arc.clone(),
        _ => {
            let arc = Arc::new(value.clone());
            *copy = Some((generation, arc.clone()));
            arc
        }
    }
}
