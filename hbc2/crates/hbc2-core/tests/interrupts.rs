//! Hardware and software interrupts ("Hardware interrupts" and "Software
//! interrupts" in the manual), and the run loop.

mod common;

use common::*;
use hbc2_core::isa::encode::*;
use hbc2_core::{Flag, MachineConfig, StopReason};

/// Interrupt handler that logs I into 0x9010 + n, where n is counted at 0x9000.
fn logging_handler() -> Vec<hbc2_core::isa::Instruction> {
    vec![
        reg_ram(O::Lod, A, 0x9000),
        reg_imm8(O::Mov, X, 0x90),
        reg(O::Mov, Y, A),
        reg_imm8(O::Add, Y, 0x10),
        ramreg_immreg(O::Str, X, Y, I),
        imm16(O::Inc, 0x9000),
        none(O::Irt),
    ]
}

fn monitor_only() -> MachineConfig {
    MachineConfig {
        monitor: true,
        ..bare()
    }
}

#[test]
fn plugging_a_peripheral_sends_port_then_id_on_port_0() {
    let mut m = Image::new()
        .program(&[
            none(O::Cli),
            reg_imm8(O::Mov, I, 0x77),
            none(O::Sti),
            imm16(O::Jmp, at(3)),
        ])
        .vector(0, 0xF000)
        .code(0xF000, &logging_handler())
        .build(monitor_only());
    m.run(200);
    assert_eq!(ram(&m, 0x9000), 2, "two interrupts");
    assert_eq!(ram(&m, 0x9010), 1, "first: monitor's first port");
    assert_eq!(ram(&m, 0x9011), 0x4A, "second: monitor's ID");
    assert_eq!(m.cpu.reg(I), 0x77, "IRT restores I");
    assert_eq!(m.cpu.sp, 0);
    assert!(m.cpu.flag(Flag::Interrupt));
}

#[test]
fn interrupt_entry_pushes_i_then_pc() {
    let mut m = Image::new()
        .program(&[
            none(O::Cli),
            reg_imm8(O::Mov, I, 0x77),
            none(O::Sti),
            imm16(O::Jmp, at(3)),
        ])
        .vector(0, 0xF000)
        .code(0xF000, &[none(O::Irt)])
        .build(monitor_only());
    m.breakpoints.set(0xF000, true);
    let result = m.run(100);
    assert_eq!(result.stop, Some(StopReason::Breakpoint(0xF000)));
    assert_eq!(m.cpu.sp, 3);
    assert_eq!(ram(&m, 0x0001), 0x77, "I");
    let return_pc = u16::from_be_bytes([ram(&m, 0x0003), ram(&m, 0x0002)]);
    assert_eq!(return_pc, at(3), "PC, MSB on top");
    assert_eq!(m.cpu.reg(I), 1, "I holds the interrupt data");
    assert!(
        !m.cpu.flag(Flag::Interrupt),
        "interrupts disabled in the handler"
    );
}

#[test]
fn hardware_interrupt_handshake_takes_two_cycles() {
    let mut m = Image::new()
        .program(&[imm16(O::Jmp, at(0))])
        .vector(0, 0xF000)
        .build(monitor_only());
    m.tick(); // JMP runs; the IOD raises INT at the end of the cycle
    assert!(m.board.int);
    assert_eq!(m.step(), 2, "INR raised, then handler entered");
    assert_eq!(m.cpu.pc, 0xF000);
    assert!(!m.board.inr);
}

#[test]
fn interrupts_wait_while_the_flag_is_clear() {
    let mut m = Image::new()
        .program(&[none(O::Cli), imm16(O::Jmp, at(1))])
        .vector(0, 0xF000)
        .build(monitor_only());
    m.run(1000);
    assert_ne!(m.cpu.pc, 0xF000);
    assert_eq!(
        m.board.iod.queue().len(),
        2,
        "both plug-in interrupts still queued"
    );
}

#[test]
fn software_interrupt() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, I, 0x42),
            imm8(O::Int, 0x10),
            reg_imm8(O::Mov, B, 1),
            none(O::Hlt),
        ])
        .vector(0x10, 0xF000)
        .code(0xF000, &[reg(O::Mov, A, I), none(O::Irt)])
        .build(bare());
    m.run(100);
    assert_eq!(m.cpu.reg(A), 0x42, "handler sees I as data");
    assert_eq!(m.cpu.reg(B), 1, "execution resumes after INT");
    assert!(m.cpu.is_halted());
}

#[test]
fn software_interrupt_waits_for_sti() {
    let mut m = Image::new()
        .program(&[
            none(O::Cli),
            imm8(O::Int, 0x10),
            none(O::Nop),
            none(O::Sti),
            none(O::Hlt),
        ])
        .vector(0x10, 0xF000)
        .code(0xF000, &[reg_imm8(O::Mov, A, 1), none(O::Irt)])
        .build(bare());
    steps(&mut m, 3);
    assert_eq!(m.cpu.reg(A), 0);
    assert!(m.cpu.pending_software_interrupt().is_some());
    m.run(100);
    assert_eq!(m.cpu.reg(A), 1);
}

#[test]
fn hlt_wakes_up_on_interrupt() {
    let config = MachineConfig {
        keyboard: true,
        ..bare()
    };
    let mut m = Image::new()
        .program(&[
            none(O::Hlt),
            reg_imm8(O::Mov, B, 1),
            none(O::Hlt),
            imm16(O::Jmp, at(2)),
        ])
        .vector(0, 0xF100) // plug-in interrupts: ignored
        .code(0xF100, &[none(O::Irt)])
        .vector(1, 0xF000) // keyboard "pressed" port
        .code(0xF000, &[reg(O::Mov, A, I), none(O::Irt)])
        .build(config);
    m.run(100);
    assert!(m.cpu.is_halted());
    assert_eq!(m.cpu.reg(B), 1, "plug-in interrupts woke it once already");

    m.key_event(0x2A, true);
    m.run(100);
    assert_eq!(m.cpu.reg(A), 0x2A);
    assert!(m.cpu.is_halted());
}

#[test]
fn iod_queue_drops_interrupts_when_full() {
    let mut m = machine(&[none(O::Cli), imm16(O::Jmp, at(1))]);
    let accepted = (0..300)
        .filter(|_| m.board.iod.trigger_interrupt(0x20))
        .count();
    assert_eq!(accepted, 256);
    assert_eq!(m.board.iod.queue().len(), 256);
}

#[test]
fn breakpoint_stops_before_the_instruction_and_resumes() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 1),
        reg_imm8(O::Mov, A, 2),
        reg_imm8(O::Mov, A, 3),
        imm16(O::Jmp, at(0)),
    ]);
    m.breakpoints.set(at(2), true);
    let result = m.run(1000);
    assert_eq!(result.stop, Some(StopReason::Breakpoint(at(2))));
    assert_eq!(result.ticks, 2);
    assert_eq!(m.cpu.reg(A), 2);

    let result = m.run(1000);
    assert_eq!(
        result.stop,
        Some(StopReason::Breakpoint(at(2))),
        "next loop iteration"
    );
    assert_eq!(result.ticks, 4);
}

#[test]
fn halted_breakpoint_does_not_retrigger() {
    let mut m = machine(&[none(O::Hlt), none(O::Nop)]);
    m.breakpoints.set(at(1), true);
    assert_eq!(m.run(100).stop, Some(StopReason::Breakpoint(at(1))));
    assert_eq!(m.run(100).stop, None);
}

#[test]
fn power_on_restores_initial_state() {
    let mut m = machine(&[reg_imm8(O::Mov, A, 1), reg_ram(O::Str, A, 0x0300)]);
    m.breakpoints.set(0x1234, true);
    steps(&mut m, 2);
    m.power_on();
    assert_eq!(m.cpu.reg(A), 0);
    assert_eq!(m.ticks(), 0);
    assert_ne!(ram(&m, 0x0300), 1, "program reloaded from the image");
    assert!(m.breakpoints.contains(0x1234), "breakpoints kept");
}
