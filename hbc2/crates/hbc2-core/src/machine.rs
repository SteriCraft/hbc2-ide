//! A complete HBC-2: CPU, board and peripherals, driven one clock cycle at a time.
//!
//! Instructions take as many clock cycles as on the hardware (see
//! [`Instruction::cycles`](crate::isa::Instruction::cycles)). An instruction
//! does all its work in its first cycle; its other cycles only wait, and the
//! machine reaches the next instruction boundary at its last one. Interrupts
//! are taken at boundaries only. After a power-on, the first instruction
//! starts after the hardware's boot time ([`HARDWARE_BOOT_CYCLES`]).

use crate::board::Board;
use crate::cpu::{Cpu, CpuState};
use crate::iop::{Iop, IopConfig};
use crate::mailbox::Mailbox;
use crate::isa::{Flag, HARDWARE_BOOT_CYCLES, RAM_SIZE};
use crate::peripherals::rtc::DateTime;
use crate::peripherals::{Eeprom, Keyboard, Monitor, Peripherals, Rtc};
use crate::trace::{NoProbe, Probe};

/// Which peripherals are plugged in.
#[derive(Clone, Debug)]
pub struct MachineConfig {
    pub monitor: bool,
    pub rtc: bool,
    pub keyboard: bool,
    /// With an EEPROM, RAM starts empty and is filled by the EEPROM at power-on.
    pub eeprom: Option<Eeprom>,
    /// The I/O: None = the IOD on the CPU's bus (the old design, and the CPU
    /// bench's stub); Some = the mailbox card and the I/O processor (the
    /// ESP32) running the IOD as its protocol.
    pub mailbox: Option<IopConfig>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        MachineConfig {
            monitor: true,
            rtc: true,
            keyboard: true,
            eeprom: None,
            mailbox: None,
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
    /// The I/O processor, when the board has a mailbox.
    pub iop: Iop,
    pub breakpoints: Breakpoints,
    /// RAM content loaded at each power-on (when there is no EEPROM).
    ram_image: Option<Box<[u8]>>,
    ticks: u64,
    /// Cycles left in the instruction (or interrupt entry, or halt loop turn)
    /// now running, its first cycle excluded; or in the boot.
    busy: u32,
    /// The work now running ends on an instruction boundary.
    boundary: bool,
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
        let board = Board { mailbox: config.mailbox.map(|_| Mailbox::new()), ..Board::default() };
        let mut machine = Machine {
            cpu: Cpu::new(),
            board,
            peripherals,
            iop: Iop::new(config.mailbox.unwrap_or_default()),
            breakpoints: Breakpoints::default(),
            ram_image: ram_image.map(Box::from),
            ticks: 0,
            busy: 0,
            boundary: false,
        };
        machine.power_on();
        Ok(machine)
    }

    /// Power cycle: RAM, CPU, IOD and peripherals go back to their initial
    /// state. EEPROM content and breakpoints are kept.
    pub fn power_on(&mut self) {
        self.ticks = 0;
        // The boot ROM copy. Its end is not a boundary: like a run from a
        // breakpoint, the first instruction runs before any breakpoint stops.
        self.busy = HARDWARE_BOOT_CYCLES as u32;
        self.boundary = false;
        self.cpu.reset();
        self.board.reset();
        self.iop.reset();
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

    /// True between the first and the last cycle of an instruction (or of
    /// an interrupt entry, or of a halt loop turn): the CPU state already
    /// shows the instruction's result. Also true during the boot.
    pub fn mid_instruction(&self) -> bool {
        self.busy > 0
    }

    /// True during the boot: from power-on until the first instruction.
    pub fn booting(&self) -> bool {
        self.ticks < HARDWARE_BOOT_CYCLES && self.busy > 0
    }

    /// Cycles left before the first instruction (0 once booted).
    pub fn boot_cycles_left(&self) -> u64 {
        if self.booting() { self.busy as u64 } else { 0 }
    }

    /// Spends the rest of the boot at once (nothing happens during it).
    /// Returns the number of cycles spent.
    pub fn finish_boot(&mut self) -> u64 {
        if !self.booting() {
            return 0;
        }
        let cycles = self.busy;
        self.wait(cycles);
        cycles as u64
    }

    /// One clock cycle. Returns true when the machine reached a new
    /// instruction boundary: the last cycle of an instruction, or of the jump
    /// into an interrupt handler.
    #[inline(always)]
    pub fn tick(&mut self) -> bool {
        self.tick_with(&mut NoProbe)
    }

    /// [`Machine::tick`], reporting what the CPU does to `probe`.
    #[inline(always)]
    pub fn tick_with<P: Probe>(&mut self, probe: &mut P) -> bool {
        if self.busy > 0 {
            return self.wait(1);
        }
        let advanced = self.cpu.tick_with(&mut self.board, probe);
        if self.board.mailbox.is_some() {
            self.io_cycle();
        } else {
            if let Some(port) = self.board.iod.take_pending_write() {
                self.peripherals.port_written(port, &mut self.board);
            }
            self.board.tick_iod();
        }
        self.ticks += 1;
        let busy = self.cpu.cycles as u32 - 1;
        if busy > 0 {
            self.busy = busy;
            self.boundary = advanced;
            return false;
        }
        advanced
    }

    /// Spends `cycles` (at most the cycles left) of the work now running.
    /// Returns true if that reached its end and the end is a boundary.
    #[inline(always)]
    fn wait(&mut self, cycles: u32) -> bool {
        self.busy -= cycles;
        self.ticks += cycles as u64;
        // The CPU drives nothing while it waits: the IOD only follows its
        // queue; the mailbox and the I/O processor run cycle by cycle.
        if self.board.mailbox.is_some() {
            self.io_cycles(cycles);
        } else {
            self.board.tick_iod();
        }
        self.busy == 0 && std::mem::take(&mut self.boundary)
    }

    /// The end of a step with a mailbox: the `clk` edge, then the I/O
    /// processor's polling pass.
    #[inline(always)]
    fn io_cycle(&mut self) {
        if let Some(m) = &mut self.board.mailbox {
            m.clock(self.board.inr);
        }
        self.iop.cycle(&mut self.board, &mut self.peripherals);
        self.board.mailbox_outputs();
    }

    /// True when more steps without I/O would change nothing on the mailbox
    /// side (but the I/O processor's ages, see [`Iop::skip`]).
    fn io_quiet(&self) -> bool {
        self.board.mailbox.as_ref().is_none_or(|m| m.settled()) && !self.board.inr && self.iop.idle(&self.board)
    }

    /// `cycles` steps without I/O from the CPU.
    fn io_cycles(&mut self, mut cycles: u32) {
        while cycles > 0 {
            if self.io_quiet() {
                self.iop.skip(cycles, &self.board);
                return;
            }
            self.io_cycle();
            cycles -= 1;
        }
    }

    /// The board has the mailbox card and the I/O processor.
    pub fn has_mailbox(&self) -> bool {
        self.board.mailbox.is_some()
    }

    /// Runs until the next instruction boundary: the end of one instruction,
    /// or of the jump into an interrupt handler. When halted with nothing
    /// pending, runs one turn of the halt loop. Right after a power-on, the
    /// boot and the first instruction. Returns the number of cycles run.
    pub fn step(&mut self) -> u64 {
        self.step_with(&mut NoProbe)
    }

    /// [`Machine::step`], reporting what the CPU does to `probe`.
    pub fn step_with<P: Probe>(&mut self, probe: &mut P) -> u64 {
        let start = self.ticks;
        // The longest path is a hardware interrupt entry after the handshake.
        for _ in 0..64 {
            let advanced = if self.busy > 0 {
                self.wait(self.busy)
            } else {
                self.tick_with(probe)
            };
            if advanced {
                break;
            }
            if self.busy == 0 && self.cpu.is_halted() && self.cpu.state == CpuState::Executing {
                break;
            }
        }
        self.ticks - start
    }

    /// Runs at most `max_ticks` cycles, stopping early when the PC lands on
    /// a breakpoint. The instruction at the current PC always runs first, so
    /// calling `run` again resumes from a breakpoint.
    ///
    /// When the machine becomes idle (see [`Machine::is_idle`]), the rest of
    /// the budget is counted without being simulated: those cycles would
    /// change nothing.
    pub fn run(&mut self, max_ticks: u64) -> RunResult {
        self.run_with(max_ticks, &mut NoProbe)
    }

    /// [`Machine::run`], reporting what the CPU does to `probe`.
    pub fn run_with<P: Probe>(&mut self, max_ticks: u64, probe: &mut P) -> RunResult {
        let mut n = 0;
        while n < max_ticks {
            // Wait cycles change nothing: spend them all at once.
            let advanced = if self.busy > 0 {
                let cycles = (self.busy as u64).min(max_ticks - n) as u32;
                n += cycles as u64;
                self.wait(cycles)
            } else {
                n += 1;
                self.tick_with(probe)
            };
            if advanced {
                if self.breakpoints.contains(self.cpu.pc) {
                    return RunResult {
                        ticks: n,
                        stop: Some(StopReason::Breakpoint(self.cpu.pc)),
                    };
                }
            } else if self.is_idle() {
                self.ticks += max_ticks - n;
                break;
            }
        }
        RunResult {
            ticks: max_ticks,
            stop: None,
        }
    }

    /// True when the CPU is halted and nothing can wake it up without an
    /// outside event (a key press, the RTC, a debugger write): every clock
    /// cycle until then would leave the machine unchanged.
    pub fn is_idle(&self) -> bool {
        let cpu = &self.cpu;
        let board = &self.board;
        if self.busy > 0 || !cpu.is_halted() || cpu.state != CpuState::Executing || board.inr {
            return false;
        }
        if board.mailbox.is_some() {
            // The mailbox and the I/O processor have nothing left to do; a
            // pending event (INT high) only matters when interrupts are on.
            return self.io_quiet()
                && (cpu.is_stopped()
                    || (!board.int
                        && (cpu.pending_software_interrupt().is_none() || !cpu.flag(Flag::Interrupt))));
        }
        if cpu.is_stopped() {
            // Interrupts are ignored; only the INT line may still follow the queue.
            return board.int == board.iod.has_pending_interrupt();
        }
        !board.int
            && !board.iod.has_pending_interrupt()
            && (cpu.pending_software_interrupt().is_none() || !cpu.flag(Flag::Interrupt))
    }

    /// Tells time-based peripherals (the RTC) that `nanos` of host time went by.
    pub fn advance_time(&mut self, nanos: u64) {
        self.peripherals.advance_time(nanos, &mut self.board);
    }

    /// Sets the RTC to the host's local time (see `Rtc::sync`).
    /// Returns true if the clock changed.
    pub fn sync_rtc(&mut self, host: DateTime, interrupt: bool) -> bool {
        match &mut self.peripherals.rtc {
            Some(rtc) => rtc.sync(host, interrupt, &mut self.board),
            None => false,
        }
    }

    /// Sends the RTC's interrupt now (see `Rtc::interrupt`).
    pub fn rtc_interrupt(&mut self) -> bool {
        match &self.peripherals.rtc {
            Some(rtc) => rtc.interrupt(&mut self.board),
            None => false,
        }
    }

    /// Sends a key event through the keyboard. Returns false if it was dropped.
    pub fn key_event(&mut self, scan_code: u8, pressed: bool) -> bool {
        match &mut self.peripherals.keyboard {
            Some(keyboard) => keyboard.key_event(scan_code, pressed, &mut self.board),
            None => false,
        }
    }
}
