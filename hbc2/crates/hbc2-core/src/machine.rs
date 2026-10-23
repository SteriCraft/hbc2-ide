//! A complete HBC-2: CPU, board and peripherals, driven one clock cycle at a time.

use crate::board::Board;
use crate::cpu::Cpu;
use crate::isa::RAM_SIZE;
use crate::peripherals::{Eeprom, Keyboard, Monitor, Peripherals, Rtc};

/// Which peripherals are plugged in.
#[derive(Clone, Debug)]
pub struct MachineConfig {
    pub monitor: bool,
    pub rtc: bool,
    pub keyboard: bool,
    /// With an EEPROM, RAM starts empty and is filled by the EEPROM at power-on.
    pub eeprom: Option<Eeprom>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        MachineConfig {
            monitor: true,
            rtc: true,
            keyboard: true,
            eeprom: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineError {
    /// The RAM image must be exactly 65,536 bytes.
    WrongRamImageSize(usize),
}

impl std::fmt::Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineError::WrongRamImageSize(size) => {
                write!(f, "RAM image is {size} bytes, expected {RAM_SIZE} bytes")
            }
        }
    }
}

impl std::error::Error for MachineError {}

/// One bit per RAM address.
#[derive(Clone)]
pub struct Breakpoints(Box<[u64; RAM_SIZE / 64]>);

impl Default for Breakpoints {
    fn default() -> Self {
        Breakpoints(Box::new([0; RAM_SIZE / 64]))
    }
}

impl std::fmt::Debug for Breakpoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Breakpoints {
    #[inline(always)]
    pub fn contains(&self, address: u16) -> bool {
        self.0[address as usize >> 6] & (1 << (address & 63)) != 0
    }

    pub fn set(&mut self, address: u16, enabled: bool) {
        let mask = 1 << (address & 63);
        let word = &mut self.0[address as usize >> 6];
        if enabled {
            *word |= mask;
        } else {
            *word &= !mask;
        }
    }

    pub fn clear(&mut self) {
        self.0.fill(0);
    }

    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0..=u16::MAX).filter(|&a| self.contains(a))
    }
}

/// Why [`Machine::run`] returned before using its whole budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// The PC reached a breakpoint; the instruction there has not run yet.
    Breakpoint(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunResult {
    /// Clock cycles actually run.
    pub ticks: u64,
    pub stop: Option<StopReason>,
}

#[derive(Clone, Debug)]
pub struct Machine {
    pub cpu: Cpu,
    pub board: Board,
    pub peripherals: Peripherals,
    pub breakpoints: Breakpoints,
    /// RAM content loaded at each power-on (when there is no EEPROM).
    ram_image: Option<Box<[u8]>>,
    ticks: u64,
}

impl Machine {
    /// Builds and powers on a machine. `ram_image`, if given, must be 64 KiB;
    /// it is ignored when an EEPROM is plugged (the EEPROM fills RAM instead).
    pub fn new(config: MachineConfig, ram_image: Option<&[u8]>) -> Result<Machine, MachineError> {
        if let Some(image) = ram_image
            && image.len() != RAM_SIZE
        {
            return Err(MachineError::WrongRamImageSize(image.len()));
        }
        let peripherals = Peripherals {
            monitor: config.monitor.then(Monitor::new),
            rtc: config.rtc.then(Rtc::new),
            keyboard: config.keyboard.then(Keyboard::new),
            eeprom: config.eeprom,
        };
        let mut machine = Machine {
            cpu: Cpu::new(),
            board: Board::default(),
            peripherals,
            breakpoints: Breakpoints::default(),
            ram_image: ram_image.map(Box::from),
            ticks: 0,
        };
        machine.power_on();
        Ok(machine)
    }

    /// Power cycle: RAM, CPU, IOD and peripherals go back to their initial
    /// state. EEPROM content and breakpoints are kept.
    pub fn power_on(&mut self) {
        self.ticks = 0;
        self.cpu.reset();
        self.board.reset();
        match (&self.ram_image, self.peripherals.eeprom.is_some()) {
            (Some(image), false) => self.board.ram.as_mut_slice().copy_from_slice(image),
            _ => self.board.ram.fill(0),
        }
        self.peripherals.power_on(&mut self.board);
    }

    /// Clock cycles since power-on.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// One clock cycle. Returns true when the PC moved to a new instruction.
    #[inline(always)]
    pub fn tick(&mut self) -> bool {
        let advanced = self.cpu.tick(&mut self.board);
        if let Some(port) = self.board.iod.take_pending_write() {
            self.peripherals.port_written(port, &mut self.board);
        }
        self.board.tick_iod();
        self.ticks += 1;
        advanced
    }

    /// Runs until the next instruction boundary: one instruction, or the
    /// jump into an interrupt handler. When halted with nothing pending,
    /// runs a single cycle. Returns the number of cycles run.
    pub fn step(&mut self) -> u64 {
        // The longest path is a hardware interrupt handshake (2 cycles).
        for n in 1..=4 {
            if self.tick() || self.cpu.is_halted() {
                return n;
            }
        }
        4
    }

    /// Runs at most `max_ticks` cycles, stopping early when the PC lands on
    /// a breakpoint. The instruction at the current PC always runs first, so
    /// calling `run` again resumes from a breakpoint.
    pub fn run(&mut self, max_ticks: u64) -> RunResult {
        for n in 1..=max_ticks {
            if self.tick() && self.breakpoints.contains(self.cpu.pc) {
                return RunResult {
                    ticks: n,
                    stop: Some(StopReason::Breakpoint(self.cpu.pc)),
                };
            }
        }
        RunResult {
            ticks: max_ticks,
            stop: None,
        }
    }

    /// Tells time-based peripherals (the RTC) that `nanos` of host time went by.
    pub fn advance_time(&mut self, nanos: u64) {
        self.peripherals.advance_time(nanos, &mut self.board);
    }

    /// Sends a key event through the keyboard. Returns false if it was dropped.
    pub fn key_event(&mut self, scan_code: u8, pressed: bool) -> bool {
        match &mut self.peripherals.keyboard {
            Some(keyboard) => keyboard.key_event(scan_code, pressed, &mut self.board),
            None => false,
        }
    }
}
