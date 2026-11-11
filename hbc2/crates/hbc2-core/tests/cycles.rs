//! Instruction timing: each instruction, interrupt entry and halt loop turn
//! takes as many clock cycles as on the hardware (`logisim/notes/cu-cycles.md`;
//! the table itself is checked against the microcode by `cargo test -p hbc2-ucode`).

mod common;

use common::*;
use hbc2_core::isa::{
    HALT_LOOP_CYCLES, HARDWARE_BOOT_CYCLES, HARDWARE_INTERRUPT_CYCLES, Instruction,
    SOFTWARE_INTERRUPT_CYCLES, encode::*,
};
use hbc2_core::{Machine, StopReason};

#[test]
fn the_first_instruction_starts_after_the_boot() {
    let image = Image::new().program(&[reg_imm8(O::Mov, A, 7)]).0;
    let mut m = Machine::new(bare(), Some(&image)).unwrap();
    assert!(m.booting());
    assert_eq!(m.cpu.pc, 0x0000);
    m.run(HARDWARE_BOOT_CYCLES);
    assert_eq!(m.cpu.reg(A), 0, "nothing run during the boot");
    assert!(!m.booting());
    assert!(!m.tick(), "the first instruction's first cycle");
    assert_eq!(m.cpu.reg(A), 7);

    m.power_on();
    assert!(m.booting(), "every power-on boots");
    assert_eq!(m.step(), HARDWARE_BOOT_CYCLES + 3, "boot, then MOV");
    assert_eq!(m.cpu.reg(A), 7);
    assert_eq!(m.finish_boot(), 0, "already booted");
}

#[test]
fn no_interrupt_before_the_first_instruction() {
    // An interrupt waits from power-on: INT is up when the boot ends, but
    // the first fetch has no interrupt check (as on the hardware).
    let image = Image::new().program(&[none(O::Cli), none(O::Hlt)]).0;
    let mut m = Machine::new(bare(), Some(&image)).unwrap();
    assert!(m.board.iod.trigger_interrupt(0x20));
    m.run(HARDWARE_BOOT_CYCLES + 100);
    assert!(m.cpu.is_stopped(), "CLI ran first, then HLT");
    assert_eq!(m.cpu.pc, 2);
}

#[test]
fn each_step_takes_the_instruction_cycles() {
    let program = [
        none(O::Nop),               // 1
        reg_imm8(O::Mov, A, 7),     // 3
        reg(O::Add, A, A),          // 4
        reg_ram(O::Str, A, 0x9000), // 6
        reg_ram(O::Add, A, 0x9000), // 8
        imm16(O::Cal, 0x0100),      // 7
    ];
    let mut m = Image::new()
        .program(&program)
        .code(0x0100, &[none(O::Ret)]) // 5
        .build(bare());
    for (expected, what) in [
        (1, "NOP"),
        (3, "MOV Reg / Imm8"),
        (4, "ADD Reg"),
        (6, "STR Reg / Ram"),
        (8, "ADD Reg / Ram"),
        (7, "CAL Imm16"),
        (5, "RET"),
    ] {
        assert_eq!(m.step(), expected, "{what}");
    }
    assert_eq!(m.ticks(), HARDWARE_BOOT_CYCLES + 1 + 3 + 4 + 6 + 8 + 7 + 5);
    assert_eq!(m.cpu.reg(A), 28);
}

#[test]
fn invalid_opcodes_take_one_cycle_and_stops_three_or_four() {
    assert_eq!(Instruction(0xFC00_0000).cycles(), 1, "opcode 3F: a NOP");
    // CLC with mode bits 01xx: stops before reading a second byte.
    assert_eq!(
        Instruction(((O::Clc as u32) << 26) | (0b0100 << 22)).cycles(),
        3
    );
    // MOV with mode Imm16: stops after 2 bytes.
    let mov_imm16 = Instruction(((O::Mov as u32) << 26) | (0x7 << 22));
    assert!(mov_imm16.stops());
    assert_eq!(mov_imm16.cycles(), 4);
}

#[test]
fn the_work_is_done_in_the_first_cycle_and_the_boundary_is_the_last() {
    let mut m = machine(&[reg_imm8(O::Mov, A, 7), none(O::Nop)]);
    assert!(!m.tick(), "first of 3 cycles");
    assert_eq!(m.cpu.reg(A), 7, "result already there");
    assert!(m.mid_instruction());
    assert!(!m.tick());
    assert!(m.tick(), "boundary on the last cycle");
    assert!(!m.mid_instruction());
    assert!(m.tick(), "NOP: 1 cycle");
}

#[test]
fn a_run_budget_can_end_inside_an_instruction() {
    let program = [
        reg_ram(O::Add, A, 0x9000), // 8 cycles
        none(O::Nop),
        imm16(O::Jmp, 0x0000),
    ];
    let mut m = machine(&program);
    m.breakpoints.set(at(&program, 1), true);
    assert_eq!(m.run(5).stop, None, "5 of ADD's 8 cycles");
    assert!(m.mid_instruction());
    let result = m.run(100);
    assert_eq!(result.stop, Some(StopReason::Breakpoint(at(&program, 1))));
    assert_eq!(result.ticks, 3, "the rest of ADD");
    assert_eq!(m.ticks(), HARDWARE_BOOT_CYCLES + 8);
}

#[test]
fn software_interrupt_entry() {
    let mut m = Image::new()
        .program(&[imm8(O::Int, 0x20), imm16(O::Jmp, 0x0003)])
        .vector(0x20, 0xF000)
        .build(bare());
    assert_eq!(m.step(), 4, "INT");
    assert_eq!(
        m.step(),
        SOFTWARE_INTERRUPT_CYCLES as u64,
        "W1, W2, IE1-IE8"
    );
    assert_eq!(m.cpu.pc, 0xF000);
}

#[test]
fn hardware_interrupt_entry() {
    let mut m = Image::new()
        .program(&[imm16(O::Jmp, 0x0000)])
        .vector(0x20, 0xF000)
        .build(bare());
    m.board.iod.trigger_interrupt(0x20);
    assert_eq!(m.step(), 5, "JMP Imm16; INT raised meanwhile");
    assert_eq!(m.step(), HARDWARE_INTERRUPT_CYCLES as u64);
    assert_eq!(m.cpu.pc, 0xF000);
}

#[test]
fn halted_cpu_turns_in_the_halt_loop() {
    let mut m = Image::new()
        .program(&[none(O::Hlt), none(O::Nop)])
        .vector(0x20, 0xF000)
        .build(bare());
    assert_eq!(m.step(), 2, "HLT");
    assert_eq!(
        m.step(),
        HALT_LOOP_CYCLES as u64,
        "one turn, nothing pending"
    );
    m.board.iod.trigger_interrupt(0x20);
    let mut cycles = 0;
    while m.cpu.pc != 0xF000 {
        cycles += m.step();
        assert!(cycles < 20);
    }
    // The IOD raises INT during the loop turn; the entry follows.
    assert_eq!(
        cycles,
        HALT_LOOP_CYCLES as u64 + HARDWARE_INTERRUPT_CYCLES as u64
    );
}
