//! The I/O processor (the ESP32 behind the mailbox, io-redesign-proposal.md
//! 9.3, 9.9): a model of its firmware.
//!
//! The firmware runs today's IOD as its protocol. Its state is the board's
//! [`Iod`](crate::iod::Iod) (port latches, owners, interrupt queue) and the
//! [`Peripherals`]:
//! - `0x01 p v` (OUT): `Iod::cpu_write(p, v)`, then the owning peripheral
//!   reacts, exactly as an `out` did with the old IOD;
//! - `0x02 p` (IN): replies `Iod::port(p)` on RX;
//! - `0x00` (NOP); any other byte is ignored, as a NOP;
//! - `0x03` (INFO): replies the protocol version, the number of devices N,
//!   then N descriptors of 6 bytes ([`Peripherals::descriptors`]: class,
//!   product ID, first port, number of ports);
//! - every queued interrupt `(port, data)` becomes an event on EVT, in order.
//!   Plugging a device sends none: the CPU asks with INFO.
//!
//! After power-on the firmware boots for `boot_cycles` before it looks at
//! the mailbox: a command written meanwhile waits in TX. `rst_n` resets the
//! CPU and the card only: the MCU drops the command it was receiving and
//! its replies, and keeps the rest (ports, queue, devices).
//!
//! The MCU polls the mailbox once per CPU clock cycle, after the `clk` edge.
//! It keeps the firmware rule (I15): RX or EVT is written only when its flag
//! reads 0 and at least `rule_cycles` cycles after it went to 0. Each
//! mailbox action (a TX byte read and handled, an RX byte or an event
//! written) keeps it busy for `service_cycles`.

use std::collections::VecDeque;

use crate::board::Board;
use crate::peripherals::Peripherals;

pub const PROTOCOL_VERSION: u8 = 1;

/// Command bytes (9.3, compat + NOP and INFO).
pub mod command {
    pub const NOP: u8 = 0x00;
    pub const OUT: u8 = 0x01;
    pub const IN: u8 = 0x02;
    pub const INFO: u8 = 0x03;
}

/// What the I/O processor handled, for the debugger ([`Iop::log`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogEntry {
    /// `0x01 p v`.
    Out { port: u8, value: u8 },
    /// `0x02 p`, and the value replied.
    In { port: u8, value: u8 },
    /// `0x03`.
    Info,
    /// `0x00`, or a byte that is no command (ignored).
    Nop(u8),
    /// An interrupt sent on EVT.
    Event { port: u8, data: u8 },
}

/// Entries kept in [`Iop::log`].
pub const LOG_SIZE: usize = 100;

/// How fast the emulated MCU is, in CPU clock cycles (500 ns each).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IopConfig {
    /// Cycles one mailbox action takes (Q-C4: about 4 us, 8 cycles).
    pub service_cycles: u32,
    /// Cycles to wait after RX or EVT went to 0 before writing it again
    /// (I15: 1 us, 2 cycles). 0 breaks the rule, for tests: a write in the
    /// CPU's clear step is lost.
    pub rule_cycles: u32,
    /// Cycles from power-on until the firmware first looks at the mailbox
    /// (the ESP32's boot: about 1 s, 2,000,000 cycles, on the board). 0 by
    /// default, so programs start at once in the emulator.
    pub boot_cycles: u32,
}

impl Default for IopConfig {
    fn default() -> Self {
        IopConfig {
            service_cycles: 8,
            rule_cycles: 2,
            boot_cycles: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Iop {
    pub config: IopConfig,
    /// Cycles left before the firmware has booted.
    booting: u32,
    /// Cycles left in the action now running.
    busy: u32,
    /// The command being received.
    command: Vec<u8>,
    /// Reply bytes waiting for RX.
    replies: VecDeque<u8>,
    /// Cycles since RX full / EVT pending was seen going to 0 (None = 1).
    rx_age: Option<u32>,
    evt_age: Option<u32>,
    /// Bytes moved since power-on, for the debugger: TX bytes read, RX
    /// bytes written, events written.
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub events: u64,
    /// The last commands and events, oldest first (at most [`LOG_SIZE`]).
    pub log: VecDeque<LogEntry>,
    /// Entries logged since power-on (the first kept is number
    /// `logged - log.len()`).
    pub logged: u64,
}

fn age(a: &mut Option<u32>, flag: bool, cycles: u32) {
    *a = if flag {
        None
    } else {
        Some(a.map_or(cycles - 1, |a| a.saturating_add(cycles)))
    };
}

impl Iop {
    pub fn new(config: IopConfig) -> Iop {
        Iop {
            config,
            booting: config.boot_cycles,
            ..Iop::default()
        }
    }

    /// Power-on: the firmware boots again, from nothing.
    pub fn power_on(&mut self) {
        *self = Iop::new(self.config);
    }

    /// `rst_n` (I16): the firmware keeps running (and booting, if it was),
    /// but drops any partial command and the replies not read yet.
    pub fn rst_n(&mut self) {
        self.command.clear();
        self.replies.clear();
        self.busy = 0;
        self.rx_age = None;
        self.evt_age = None;
    }

    /// Cycles left before the firmware has booted (0 once it runs).
    pub fn boot_cycles_left(&self) -> u32 {
        self.booting
    }

    /// The command bytes received so far (an incomplete command).
    pub fn partial_command(&self) -> &[u8] {
        &self.command
    }

    /// Reply bytes waiting for RX.
    pub fn replies(&self) -> impl ExactSizeIterator<Item = &u8> {
        self.replies.iter()
    }

    fn record(&mut self, entry: LogEntry) {
        if self.log.len() == LOG_SIZE {
            self.log.pop_front();
        }
        self.log.push_back(entry);
        self.logged += 1;
    }

    /// Nothing to do until the CPU or a peripheral acts: every cycle until
    /// then leaves the MCU as it is (but for the ages, see [`Iop::skip`]).
    pub fn idle(&self, board: &Board) -> bool {
        let Some(m) = &board.mailbox else { return true };
        self.booting == 0
            && self.busy == 0
            && !m.tx_full
            && (self.replies.is_empty() || m.rx_full)
            && (!board.iod.has_pending_interrupt() || m.evt_pend)
    }

    /// `cycles` idle cycles at once (see [`Iop::idle`]).
    pub fn skip(&mut self, cycles: u32, board: &Board) {
        if let Some(m) = &board.mailbox
            && cycles > 0
        {
            age(&mut self.rx_age, m.rx_full, cycles);
            age(&mut self.evt_age, m.evt_pend, cycles);
        }
    }

    /// One polling pass, after a `clk` edge.
    pub fn cycle(&mut self, board: &mut Board, peripherals: &mut Peripherals) {
        let Some(m) = &mut board.mailbox else { return };
        age(&mut self.rx_age, m.rx_full, 1);
        age(&mut self.evt_age, m.evt_pend, 1);
        if self.booting > 0 {
            self.booting -= 1;
            return;
        }
        if self.busy > 0 {
            self.busy -= 1;
            return;
        }
        let rule = self.config.rule_cycles;
        let ready = |a: Option<u32>| a.is_some_and(|a| a >= rule);
        let service = self.config.service_cycles.saturating_sub(1);
        if !m.rx_full
            && ready(self.rx_age)
            && let Some(byte) = self.replies.pop_front()
        {
            m.mcu_write_rx(byte);
            self.rx_age = None;
            self.rx_bytes += 1;
            self.busy = service;
        } else if !m.evt_pend
            && ready(self.evt_age)
            && let Some(event) = board.iod.pop_interrupt()
        {
            m.mcu_write_evt(event.port, event.data);
            self.evt_age = None;
            self.events += 1;
            self.record(LogEntry::Event {
                port: event.port,
                data: event.data,
            });
            self.busy = service;
        } else if m.tx_full {
            let byte = m.mcu_read_tx();
            self.tx_bytes += 1;
            self.busy = service;
            self.receive(byte, board, peripherals);
        }
    }

    /// One command byte from TX.
    fn receive(&mut self, byte: u8, board: &mut Board, peripherals: &mut Peripherals) {
        self.command.push(byte);
        let length = match self.command[0] {
            command::OUT => 3,
            command::IN => 2,
            _ => 1,
        };
        if self.command.len() < length {
            return;
        }
        let c = std::mem::take(&mut self.command);
        match c[0] {
            command::OUT => {
                board.iod.cpu_write(c[1], c[2]);
                if let Some(port) = board.iod.take_pending_write() {
                    peripherals.port_written(port, board);
                }
                self.record(LogEntry::Out {
                    port: c[1],
                    value: c[2],
                });
            }
            command::IN => {
                let value = board.iod.port(c[1]);
                self.replies.push_back(value);
                self.record(LogEntry::In { port: c[1], value });
            }
            command::INFO => {
                self.record(LogEntry::Info);
                let devices = peripherals.descriptors();
                self.replies.push_back(PROTOCOL_VERSION);
                self.replies.push_back(devices.len() as u8);
                self.replies.extend(devices.iter().flatten());
            }
            other => self.record(LogEntry::Nop(other)),
        }
    }
}
