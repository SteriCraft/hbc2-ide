//! Emulation core of the HBC-2 (HomeBrew Computer 2).
//!
//! The reference for everything in this crate is the *HBC-2 Instruction Manual*
//! and the peripheral manuals in `Documentation/`.
//!
//! This crate has no GUI and no threads. A [`Machine`] is a plain value: the
//! caller drives it with [`Machine::tick`], [`Machine::step`] or [`Machine::run`],
//! and reads its state directly.

pub mod alu;
pub mod board;
pub mod boot_rom;
pub mod charset;
pub mod cpu;
pub mod iod;
pub mod iop;
pub mod isa;
pub mod machine;
pub mod mailbox;
pub mod peripherals;
pub mod trace;

pub use board::{Board, Ram};
pub use cpu::{Cpu, CpuState, Flags};
pub use iod::{Interrupt, Iod};
pub use iop::{Iop, IopConfig};
pub use isa::{AddrMode, Flag, Opcode, Register};
pub use machine::{Breakpoints, Machine, MachineConfig, MachineError, RunResult, StopReason};
pub use mailbox::Mailbox;
pub use trace::{
    ColumnDiff, DiffKind, DiffReport, NoProbe, Probe, TRACE_HEADER, TraceDiff, TraceWriter,
};
