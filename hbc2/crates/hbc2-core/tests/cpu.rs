//! Instruction set behaviour, checked against the HBC-2 Instruction Manual.

mod common;

use common::*;
use hbc2_core::isa::{PROGRAM_START, encode::*};
use hbc2_core::{AddrMode, Flag};

fn flags(m: &hbc2_core::Machine) -> String {
    Flag::ALL
        .iter()
        .map(|&f| if m.cpu.flag(f) { f.letter() } else { '-' })
        .collect()
}

#[test]
fn power_on_state() {
    let m = machine(&[]);
    assert_eq!(m.cpu.pc, PROGRAM_START);
    assert_eq!(m.cpu.sp, 0);
    assert_eq!(m.cpu.registers, [0; 8]);
    assert_eq!(flags(&m), "--I-----");
}

#[test]
fn mov() {
    let mut m = machine(&[reg_imm8(O::Mov, A, 0x42), reg(O::Mov, B, A)]);
    steps(&mut m, 2);
    assert_eq!((m.cpu.reg(A), m.cpu.reg(B)), (0x42, 0x42));
    assert_eq!(m.cpu.pc, at(2));
}

#[test]
fn add_sets_carry_zero_negative() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0xFF),
        reg_imm8(O::Add, A, 1),    // 0x00, C Z
        reg_imm8(O::Add, A, 0x80), // 0x80, N
    ]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0);
    assert!(m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Zero) && !m.cpu.flag(Flag::Negative));
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0x80);
    assert!(!m.cpu.flag(Flag::Carry) && !m.cpu.flag(Flag::Zero) && m.cpu.flag(Flag::Negative));
}

#[test]
fn add_operand_modes() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, A, 1),
            reg_imm8(O::Mov, B, 2),
            reg(O::Add, A, B),          // 3
            reg_ram(O::Add, A, 0x9000), // 3 + 10
        ])
        .byte(0x9000, 10)
        .build(bare());
    steps(&mut m, 4);
    assert_eq!(m.cpu.reg(A), 13);
}

#[test]
fn adc_adds_the_carry_flag() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 1),
        reg_imm8(O::Adc, A, 1), // no carry: 2
        none(O::Stc),
        reg_imm8(O::Adc, A, 1), // carry: 4
        reg_imm8(O::Mov, B, 0xFF),
        none(O::Stc),
        reg_imm8(O::Adc, B, 0), // 0xFF + 0 + 1 = 0x00, carry out
    ]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 2);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 4);
    assert!(!m.cpu.flag(Flag::Carry));
    steps(&mut m, 3);
    assert_eq!(m.cpu.reg(B), 0);
    assert!(m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Zero));
}

#[test]
fn sub_borrows() {
    let mut m = machine(&[reg_imm8(O::Mov, A, 1), reg_imm8(O::Sub, A, 2)]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0xFF);
    assert_eq!(flags(&m), "C-IN----");
}

#[test]
fn sbb_subtracts_the_carry_flag() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 5),
        reg_imm8(O::Sbb, A, 2), // no carry: 3
        none(O::Stc),
        reg_imm8(O::Sbb, A, 2), // carry: 0
        none(O::Stc),
        reg_imm8(O::Sbb, A, 0), // 0 - 0 - 1 = 0xFF, borrow
    ]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 3);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0);
    assert!(m.cpu.flag(Flag::Zero) && !m.cpu.flag(Flag::Carry));
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0xFF);
    assert!(m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Negative));
}

#[test]
fn logic_operations() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, A, 0b1100_1100),
            reg_imm8(O::And, A, 0b1010_1010), // 1000_1000
            reg_imm8(O::Mov, B, 0b0000_0011),
            reg(O::Or, A, B),           // 1000_1011
            reg_ram(O::Xor, A, 0x9000), // ^ 1000_1011 = 0
        ])
        .byte(0x9000, 0b1000_1011)
        .build(bare());
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0b1000_1000);
    assert!(m.cpu.flag(Flag::Negative));
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0b1000_1011);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0);
    assert!(m.cpu.flag(Flag::Zero) && !m.cpu.flag(Flag::Negative));
}

#[test]
fn cmp_modes() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, A, 5),
            reg_imm8(O::Mov, B, 7),
            reg(O::Cmp, A, B),      // A < B
            reg_imm8(O::Cmp, B, 5), // B > 5
            reg_imm8(O::Mov, C, 0x90),
            reg_imm8(O::Mov, D, 0x00),
            ramreg_immreg(O::Cmp, C, D, A), // A ? $(0x9000) = 5 ? 5
        ])
        .byte(0x9000, 5)
        .build(bare());
    steps(&mut m, 3);
    assert_eq!(flags(&m), "--I---F-");
    steps(&mut m, 1);
    assert_eq!(flags(&m), "--I-S---");
    steps(&mut m, 3);
    assert_eq!(flags(&m), "-EI-----");
}

#[test]
fn inc_dec_registers_and_memory() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, A, 0xFF),
            raw(O::Inc, AddrMode::Reg, A as u8, 0, 0, 0), // 0x00, C Z
            raw(O::Dec, AddrMode::Reg, A as u8, 0, 0, 0), // 0xFF, C N
            imm16(O::Inc, 0x9000),                        // $(0x9000) = 0x11
            reg_imm8(O::Mov, C, 0x90),
            reg_imm8(O::Mov, D, 0x01),
            reg16(O::Dec, C, D), // $(0x9001) = 0x1F
        ])
        .byte(0x9000, 0x10)
        .byte(0x9001, 0x20)
        .build(bare());
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0);
    assert_eq!(flags(&m), "C-I--Z--");
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0xFF);
    assert_eq!(flags(&m), "C-IN----");
    steps(&mut m, 4);
    assert_eq!((ram(&m, 0x9000), ram(&m, 0x9001)), (0x11, 0x1F));
    assert!(!m.cpu.flag(Flag::Carry));
}

#[test]
fn not_register_and_memory() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, A, 0x0F),
            raw(O::Not, AddrMode::Reg, A as u8, 0, 0, 0),
            imm16(O::Not, 0x9000),
        ])
        .byte(0x9000, 0xFF)
        .build(bare());
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0xF0);
    assert!(m.cpu.flag(Flag::Negative));
    steps(&mut m, 1);
    assert_eq!(ram(&m, 0x9000), 0);
    assert!(m.cpu.flag(Flag::Zero));
}

#[test]
fn shifts() {
    let shift = |op| raw(op, AddrMode::Reg, A as u8, 0, 0, 0);
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0b1000_0001),
        shift(O::Shl), // 0000_0010, C
        reg_imm8(O::Mov, A, 0b1000_0011),
        shift(O::Asr), // 1100_0001, C N (sign kept)
        none(O::Clc),
        shift(O::Shr), // 0110_0000, carry untouched
    ]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0b0000_0010);
    assert!(m.cpu.flag(Flag::Carry));
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0b1100_0001);
    assert!(m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Negative));
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0b0110_0000);
    assert!(!m.cpu.flag(Flag::Carry) && !m.cpu.flag(Flag::Negative));
}

#[test]
fn shr_does_not_raise_carry() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 1),
        raw(O::Shr, AddrMode::Reg, A as u8, 0, 0, 0),
    ]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0);
    assert!(!m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Zero));
}

#[test]
fn load_and_store() {
    let mut m = Image::new()
        .program(&[
            reg_ram(O::Lod, A, 0x9000),
            reg_ram(O::Str, A, 0x9001),
            reg_imm8(O::Mov, C, 0x90),
            reg_imm8(O::Mov, D, 0x02),
            reg_imm8(O::Mov, B, 0x77),
            ramreg_immreg(O::Str, C, D, B), // $(0x9002) = B
            ramreg_immreg(O::Lod, C, D, J), // J = $(0x9002)
        ])
        .byte(0x9000, 0x5A)
        .build(bare());
    steps(&mut m, 7);
    assert_eq!(m.cpu.reg(A), 0x5A);
    assert_eq!(ram(&m, 0x9001), 0x5A);
    assert_eq!(ram(&m, 0x9002), 0x77);
    assert_eq!(m.cpu.reg(J), 0x77);
    assert_eq!(flags(&m), "--I-----", "LOD / STR / MOV do not touch flags");
}

#[test]
fn push_and_pop() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0x11),
        reg_imm8(O::Mov, B, 0x22),
        raw(O::Psh, AddrMode::Reg, A as u8, 0, 0, 0),
        raw(O::Psh, AddrMode::Reg, B as u8, 0, 0, 0),
        raw(O::Pop, AddrMode::Reg, C as u8, 0, 0, 0),
        raw(O::Pop, AddrMode::Reg, D as u8, 0, 0, 0),
    ]);
    steps(&mut m, 4);
    // PSH: STK = STK + 1, $(STK) = R1
    assert_eq!(m.cpu.sp, 2);
    assert_eq!((ram(&m, 0x0001), ram(&m, 0x0002)), (0x11, 0x22));
    steps(&mut m, 2);
    assert_eq!((m.cpu.reg(C), m.cpu.reg(D)), (0x22, 0x11));
    assert_eq!(m.cpu.sp, 0);
}

#[test]
fn conditional_jumps() {
    // For each jump: set its flag, jump over a "mov a, 1"; then clear it, and fall through.
    let cases = [
        (O::Jmc, O::Stc, O::Clc),
        (O::Jme, O::Ste, O::Cle),
        (O::Jmn, O::Stn, O::Cln),
        (O::Jms, O::Sts, O::Cls),
        (O::Jmz, O::Stz, O::Clz),
        (O::Jmf, O::Stf, O::Clf),
    ];
    for (jump, set, clear) in cases {
        let mut m = machine(&[none(set), imm16(jump, at(3)), reg_imm8(O::Mov, A, 1)]);
        steps(&mut m, 2);
        assert_eq!(m.cpu.pc, at(3), "{jump:?} taken");

        let mut m = machine(&[none(clear), imm16(jump, at(3)), reg_imm8(O::Mov, A, 1)]);
        steps(&mut m, 3);
        assert_eq!(m.cpu.reg(A), 1, "{jump:?} not taken");
    }
}

#[test]
fn jmp_reg16() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0x12),
        reg_imm8(O::Mov, B, 0x34),
        reg16(O::Jmp, A, B),
    ]);
    steps(&mut m, 3);
    assert_eq!(m.cpu.pc, 0x1234);
}

#[test]
fn call_and_return() {
    let mut m = Image::new()
        .program(&[imm16(O::Cal, 0x2000), reg_imm8(O::Mov, B, 2)])
        .code(0x2000, &[reg_imm8(O::Mov, A, 1), none(O::Ret)])
        .build(bare());
    steps(&mut m, 1);
    assert_eq!(m.cpu.pc, 0x2000);
    // Return address pushed LSB first, so that RET pops the MSB first.
    assert_eq!(m.cpu.sp, 2);
    assert_eq!((ram(&m, 0x0001), ram(&m, 0x0002)), (0x04, 0x03));
    steps(&mut m, 3);
    assert_eq!((m.cpu.reg(A), m.cpu.reg(B)), (1, 2));
    assert_eq!(m.cpu.sp, 0);
}

#[test]
fn cal_reg16() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0x20),
        reg_imm8(O::Mov, B, 0x00),
        reg16(O::Cal, A, B),
    ]);
    steps(&mut m, 3);
    assert_eq!(m.cpu.pc, 0x2000);
}

#[test]
fn flag_instructions() {
    let pairs = [
        (O::Stc, O::Clc, Flag::Carry),
        (O::Ste, O::Cle, Flag::Equal),
        (O::Sti, O::Cli, Flag::Interrupt),
        (O::Stn, O::Cln, Flag::Negative),
        (O::Sts, O::Cls, Flag::Superior),
        (O::Stz, O::Clz, Flag::Zero),
        (O::Stf, O::Clf, Flag::Inferior),
    ];
    for (set, clear, flag) in pairs {
        let mut m = machine(&[none(clear), none(set), none(clear)]);
        steps(&mut m, 1);
        assert!(!m.cpu.flag(flag), "{clear:?}");
        steps(&mut m, 1);
        assert!(m.cpu.flag(flag), "{set:?}");
        steps(&mut m, 1);
        assert!(!m.cpu.flag(flag), "{clear:?}");
    }
}

#[test]
fn in_and_out_use_port_latches() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0x30),
        reg_imm8(O::Mov, B, 0x99),
        reg(O::Out, A, B), // port(A) = B
        reg(O::In, C, A),  // C = port(A)
    ]);
    steps(&mut m, 4);
    assert_eq!(m.board.iod.port(0x30), 0x99);
    assert_eq!(m.cpu.reg(C), 0x99);
}

#[test]
fn hlt_stops_execution() {
    let mut m = machine(&[none(O::Hlt), reg_imm8(O::Mov, A, 1)]);
    m.run(100);
    assert!(m.cpu.is_halted());
    assert_eq!(m.cpu.pc, at(1));
    assert_eq!(m.cpu.reg(A), 0);
    assert_eq!(m.ticks(), 100, "clock keeps ticking while halted");
}

#[test]
fn invalid_mode_and_opcode_are_nop() {
    let mut m = machine(&[
        imm16(O::Mov, 0x1234),                    // MOV has no Imm16 mode
        hbc2_core::isa::Instruction(0xFFFF_FFFF), // opcode 0x3F, mode 0xF
        reg_imm8(O::Mov, A, 1),
    ]);
    steps(&mut m, 2);
    assert_eq!(m.cpu.registers, [0; 8]);
    assert_eq!(m.cpu.pc, at(2));
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 1);
}

#[test]
fn stack_pointer_wraps() {
    let mut m = machine(&[raw(O::Pop, AddrMode::Reg, A as u8, 0, 0, 0)]);
    steps(&mut m, 1);
    assert_eq!(m.cpu.sp, 0xFF, "no underflow flag: the 8-bit pointer wraps");
}
