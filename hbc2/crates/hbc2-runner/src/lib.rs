//! Runs an HBC-2 [`Machine`] on its own thread.
//!
//! The emulator thread is the only one that touches the machine. Other
//! threads (the GUI) talk to it in two directions only:
//!
//! * **commands in**: [`Runner`] methods (`run`, `pause`, `step`, `key`…)
//!   send messages through a channel. They never block.
//! * **snapshots out**: about 60 times per second while running, and after
//!   every command while paused, the thread publishes a [`Snapshot`]: a copy
//!   of the machine's state. [`Runner::snapshot`] returns the latest one.
//!
//! There is no shared mutable state and no lock held for more than swapping
//! a pointer, so the GUI can never freeze the emulation, and the emulation
//! can never freeze the GUI.
//!
//! Speed: the thread runs the machine in short slices and sleeps between
//! them to match the chosen clock frequency ([`Speed::Hz`]), or never sleeps
//! ([`Speed::Fastest`]). A halted CPU waiting for an interrupt costs nothing.

mod worker;

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use hbc2_core::peripherals::{Eeprom, Keyboard, Monitor, Rtc};
use hbc2_core::{Board, Breakpoints, Cpu, Machine, StopReason};

/// Target clock frequency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    /// Cycles per second (at least 1).
    Hz(u64),
    /// As fast as the host can go.
    Fastest,
}

impl Speed {
    pub const MHZ: u64 = 1_000_000;
    /// Below this speed, the RTC sends no interrupts (as when paused or
    /// stepping): the CPU could not handle one per second, and they would
    /// only pile up.
    pub const RTC_INTERRUPT_MIN_HZ: u64 = 100;

    /// Whether the RTC sends its once-per-second interrupt while running.
    pub fn sends_rtc_interrupts(self) -> bool {
        match self {
            Speed::Hz(hz) => hz >= Speed::RTC_INTERRUPT_MIN_HZ,
            Speed::Fastest => true,
        }
    }
}

impl Default for Speed {
    fn default() -> Self {
        Speed::Hz(Speed::MHZ)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Paused,
    Running,
}

/// A copy of the machine's state at one moment, plus the runner's status.
///
/// Monitor and EEPROM content is shared between snapshots while it does not
/// change, so publishing a snapshot only copies about 75 KiB.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// Increases with every published snapshot.
    pub seq: u64,
    pub state: RunState,
    /// Why the runner paused by itself, if it did (cleared by `run` and `step`).
    pub stop: Option<StopReason>,
    pub speed: Speed,
    /// Clock frequency actually reached recently (0 while paused).
    pub measured_hz: f64,
    /// Clock cycles since power-on.
    pub ticks: u64,
    pub cpu: Cpu,
    /// RAM, IOD, buses and interrupt lines.
    pub board: Board,
    pub breakpoints: Breakpoints,
    pub monitor: Option<Arc<Monitor>>,
    pub rtc: Option<Rtc>,
    pub keyboard: Option<Keyboard>,
    pub eeprom: Option<Arc<Eeprom>>,
}

/// A function run on the emulator thread with full access to the machine.
pub type MachineFn = Box<dyn FnOnce(&mut Machine) + Send>;

enum Command {
    Run,
    Pause,
    /// Executes this many instructions, then pauses.
    Step(u32),
    /// Executes one instruction, then runs while the PC is one of these
    /// addresses (sorted), then pauses.
    StepWithin(Vec<u16>),
    PowerCycle,
    Load(Box<Machine>),
    SetSpeed(Speed),
    SetBreakpoint(u16, bool),
    ClearBreakpoints,
    Key {
        scan_code: u8,
        pressed: bool,
    },
    Exec(MachineFn),
    /// Publishes a snapshot right away and sends it back.
    Sync(Sender<Arc<Snapshot>>),
    Shutdown,
}

struct Shared {
    latest: Mutex<Arc<Snapshot>>,
    notify: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Shared {
    fn latest(&self) -> Arc<Snapshot> {
        // A poisoned lock only means the emulator thread panicked; the
        // snapshot inside is still a complete value.
        self.latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn publish(&self, snapshot: Arc<Snapshot>) {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner()) = snapshot;
        if let Some(notify) = &self.notify {
            notify();
        }
    }
}

/// Handle to the emulator thread. Dropping it stops the thread.
pub struct Runner {
    commands: Sender<Command>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Runner {
    /// Starts the emulator thread, paused, with this machine.
    pub fn new(machine: Machine) -> Runner {
        Runner::start(machine, None)
    }

    /// Like [`Runner::new`], and calls `notify` (from the emulator thread)
    /// each time a new snapshot is published. A GUI passes its "request
    /// repaint" function here.
    pub fn with_notify(machine: Machine, notify: impl Fn() + Send + Sync + 'static) -> Runner {
        Runner::start(machine, Some(Box::new(notify)))
    }

    fn start(machine: Machine, notify: Option<Box<dyn Fn() + Send + Sync>>) -> Runner {
        let (commands, receiver) = mpsc::channel();
        let mut worker = worker::Worker::new(machine, receiver);
        let shared = Arc::new(Shared {
            latest: Mutex::new(worker.snapshot()),
            notify,
        });
        let thread = std::thread::Builder::new()
            .name("hbc2-emulator".into())
            .spawn({
                let shared = shared.clone();
                move || worker.main_loop(&shared)
            })
            .expect("cannot start the emulator thread");
        Runner {
            commands,
            shared,
            thread: Some(thread),
        }
    }

    /// The latest published snapshot. Cheap: no copy, no waiting.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.shared.latest()
    }

    /// Waits until the emulator thread has handled every command sent so
    /// far, and returns a snapshot taken right then.
    pub fn sync(&self) -> Arc<Snapshot> {
        let (reply, answer) = mpsc::channel();
        if self.commands.send(Command::Sync(reply)).is_ok()
            && let Ok(snapshot) = answer.recv()
        {
            return snapshot;
        }
        self.snapshot() // the thread is gone (it panicked)
    }

    fn send(&self, command: Command) {
        // Fails only if the emulator thread panicked; there is nothing to do then.
        let _ = self.commands.send(command);
    }

    /// Runs until `pause`, or until a breakpoint is reached.
    pub fn run(&self) {
        self.send(Command::Run);
    }

    pub fn pause(&self) {
        self.send(Command::Pause);
    }

    /// Pauses, then executes `instructions` instructions (an interrupt
    /// entering its handler counts as one).
    pub fn step(&self, instructions: u32) {
        self.send(Command::Step(instructions));
    }

    /// Power cycle (see [`Machine::power_on`]). Keeps running if it was.
    /// Runs one instruction, then goes on while the PC stays on one of
    /// `addresses` (the instructions of one source line, for example), at
    /// the current speed, then pauses. The state is Running meanwhile:
    /// `pause` stops it like a run. Breakpoints are not checked.
    pub fn step_within(&self, mut addresses: Vec<u16>) {
        addresses.sort_unstable();
        addresses.dedup();
        self.send(Command::StepWithin(addresses));
    }

    /// Restarts the machine from its image. The runner pauses.
    pub fn power_cycle(&self) {
        self.send(Command::PowerCycle);
    }

    /// Replaces the machine (for instance after assembling a new program).
    /// The runner pauses.
    pub fn load(&self, machine: Machine) {
        self.send(Command::Load(Box::new(machine)));
    }

    pub fn set_speed(&self, speed: Speed) {
        self.send(Command::SetSpeed(speed));
    }

    pub fn set_breakpoint(&self, address: u16, enabled: bool) {
        self.send(Command::SetBreakpoint(address, enabled));
    }

    pub fn clear_breakpoints(&self) {
        self.send(Command::ClearBreakpoints);
    }

    /// A key of the HBC-2 keyboard was pressed or released.
    pub fn key(&self, scan_code: u8, pressed: bool) {
        self.send(Command::Key { scan_code, pressed });
    }

    /// Runs `f` on the emulator thread with full access to the machine, for
    /// anything the other methods do not cover (editing RAM, reading the
    /// whole EEPROM…). Results can be sent back through a channel.
    pub fn exec(&self, f: impl FnOnce(&mut Machine) + Send + 'static) {
        self.send(Command::Exec(Box::new(f)));
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
