//! Instruction set behaviour, checked against the HBC-2 Instruction Manual.

mod common;

use common::*;
use hbc2_core::isa::{PROGRAM_START, STACK_RESET, encode::*};
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
    assert_eq!(m.cpu.pc, 0x0000);
    assert_eq!(m.cpu.pc, PROGRAM_START);
    assert_eq!(m.cpu.sp, 0xFE00);
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
            ramreg_immreg(O::Dec, C, D, A), // $(0x9001) = 0x1F
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
    steps(&mut m, 3);
    // PSH: STK = STK - 1, $(STK) = R1: the first push writes at 0xFDFF.
    assert_eq!(m.cpu.sp, 0xFDFF);
    assert_eq!(ram(&m, 0xFDFF), 0x11);
    steps(&mut m, 1);
    assert_eq!(m.cpu.sp, 0xFDFE);
    assert_eq!(ram(&m, 0xFDFE), 0x22);
    // POP: R1 = $(STK), STK = STK + 1
    steps(&mut m, 1);
    assert_eq!((m.cpu.reg(C), m.cpu.sp), (0x22, 0xFDFF));
    steps(&mut m, 1);
    assert_eq!((m.cpu.reg(D), m.cpu.sp), (0x11, STACK_RESET));
}

#[test]
fn push_and_pop_flags() {
    let mut m = machine(&[
        none(O::Stc),
        none(O::Stz),
        none(O::Psf), // C I Z pushed
        none(O::Clc),
        none(O::Clz),
        none(O::Cli),
        none(O::Ste),
        none(O::Pof),
    ]);
    steps(&mut m, 3);
    assert_eq!(m.cpu.sp, 0xFDFF);
    assert_eq!(
        ram(&m, 0xFDFF),
        0b0010_0101,
        "flag register: C=bit 0 … H=bit 7"
    );
    assert_eq!(flags(&m), "C-I--Z--", "PSF changes no flag");
    steps(&mut m, 4);
    assert_eq!(flags(&m), "-E------");
    steps(&mut m, 1);
    assert_eq!(flags(&m), "C-I--Z--", "POF restores all 8 flags");
    assert_eq!(m.cpu.sp, STACK_RESET);
}

#[test]
fn pof_restores_the_halt_flag_too() {
    let mut m = machine(&[
        raw(O::Mov, AddrMode::RegImm8, A as u8, 0, 0x80, 0),
        raw(O::Psh, AddrMode::Reg, A as u8, 0, 0, 0),
        none(O::Pof),
        reg_imm8(O::Mov, B, 1),
    ]);
    steps(&mut m, 3);
    assert!(m.cpu.is_halted());
    m.run(100);
    assert_eq!(m.cpu.reg(B), 0, "halted by POF");
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
    // Return address (the next instruction) pushed LSB first, so that RET
    // pops the MSB first: MSB on top of the stack.
    assert_eq!(m.cpu.sp, 0xFDFE);
    assert_eq!((ram(&m, 0xFDFF), ram(&m, 0xFDFE)), (0x04, 0x00));
    steps(&mut m, 2);
    assert_eq!(m.cpu.pc, at(1), "RET");
    assert_eq!(m.cpu.sp, STACK_RESET);
    steps(&mut m, 1);
    assert_eq!((m.cpu.reg(A), m.cpu.reg(B)), (1, 2));
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
    let pop = raw(O::Pop, AddrMode::Reg, A as u8, 0, 0, 0);
    let psh = raw(O::Psh, AddrMode::Reg, B as u8, 0, 0, 0);
    let mut m = Image::new()
        .program(&[imm16(O::Wsp, 0xFFFF), pop, reg_imm8(O::Mov, B, 0x77), psh])
        .byte(0xFFFF, 0x42)
        .build(bare());
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0x42);
    assert_eq!(
        m.cpu.sp, 0x0000,
        "no overflow flag: the 16-bit pointer wraps"
    );
    steps(&mut m, 2);
    assert_eq!(m.cpu.sp, 0xFFFF, "and back");
    assert_eq!(ram(&m, 0xFFFF), 0x77);
}

// ----- RamReg / ImmReg / Offset

#[test]
fn pointer_offset_crosses_a_page_boundary() {
    // (0x12F0) + 0x20 = 0x1310: the carry goes into the high byte.
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, X, 0x12),
            reg_imm8(O::Mov, Y, 0xF0),
            ramreg_offset(O::Lod, X, Y, A, 0x20),
            reg_imm8(O::Mov, B, 0x99),
            ramreg_offset(O::Str, X, Y, B, 0x21),
        ])
        .byte(0x1310, 0x5A)
        .byte(0x1210, 0xEE) // where a carry-less offset would read
        .build(bare());
    steps(&mut m, 3);
    assert_eq!(m.cpu.reg(A), 0x5A);
    steps(&mut m, 2);
    assert_eq!(ram(&m, 0x1311), 0x99);
    assert_eq!(
        (m.cpu.reg(X), m.cpu.reg(Y)),
        (0x12, 0xF0),
        "pointer unchanged"
    );
}

#[test]
fn pointer_offset_wraps_around_16_bits() {
    // (0xFFF0) + 0x20 = 0x0010
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, X, 0xFF),
            reg_imm8(O::Mov, Y, 0xF0),
            ramreg_offset(O::Lod, X, Y, A, 0x20),
            ramreg_offset(O::Inc, X, Y, A, 0x21),
        ])
        .byte(0x0010, 0x33)
        .byte(0x0011, 0xFF)
        .build(bare());
    steps(&mut m, 3);
    assert_eq!(m.cpu.reg(A), 0x33);
    steps(&mut m, 1);
    assert_eq!(ram(&m, 0x0011), 0x00);
    assert_eq!(flags(&m), "C-I--Z--");
}

#[test]
fn offset_zero_is_the_plain_pointer() {
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, X, 0x90),
            reg_imm8(O::Mov, Y, 0x00),
            ramreg_offset(O::Lod, X, Y, A, 0),
            ramreg_immreg(O::Lod, X, Y, B),
        ])
        .byte(0x9000, 0x21)
        .build(bare());
    steps(&mut m, 4);
    assert_eq!((m.cpu.reg(A), m.cpu.reg(B)), (0x21, 0x21));
    assert_eq!(
        ramreg_offset(O::Lod, X, Y, A, 0),
        ramreg_immreg(O::Lod, X, Y, A)
    );
}

#[test]
fn alu_with_pointer_offset_uses_r3() {
    // R3 = R3 op $((R1:R2) + V2); R1 and R2 are not changed.
    let at_9005 = |op| ramreg_offset(op, X, Y, A, 5);
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, X, 0x90),
            reg_imm8(O::Mov, Y, 0x00),
            reg_imm8(O::Mov, A, 0xF0),
            at_9005(O::Add), // 0xF0 + 0x20 = 0x10, C
            at_9005(O::Adc), // 0x10 + 0x20 + 1 = 0x31
            at_9005(O::Sub), // 0x31 - 0x20 = 0x11
            none(O::Stc),
            at_9005(O::Sbb), // 0x11 - 0x20 - 1 = 0xF0, C N
            at_9005(O::And), // 0xF0 & 0x20 = 0x20
            at_9005(O::Or),  // 0x20
            at_9005(O::Xor), // 0x00, Z
            at_9005(O::Cmp), // 0x00 < 0x20
        ])
        .byte(0x9005, 0x20)
        .build(bare());
    steps(&mut m, 4);
    assert_eq!(m.cpu.reg(A), 0x10);
    assert!(m.cpu.flag(Flag::Carry));
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0x31);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0x11);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0xF0);
    assert!(m.cpu.flag(Flag::Carry) && m.cpu.flag(Flag::Negative));
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0x20);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0x20);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(A), 0x00);
    assert!(m.cpu.flag(Flag::Zero));
    steps(&mut m, 1);
    assert!(m.cpu.flag(Flag::Inferior));
    assert_eq!((m.cpu.reg(X), m.cpu.reg(Y)), (0x90, 0x00));
}

#[test]
fn inc_dec_reg16_mode_is_gone() {
    // Mode 0x6 (Reg16) is not listed for INC / DEC any more: NOP.
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, C, 0x90),
            reg_imm8(O::Mov, D, 0x00),
            reg16(O::Inc, C, D),
        ])
        .byte(0x9000, 7)
        .build(bare());
    steps(&mut m, 3);
    assert_eq!(ram(&m, 0x9000), 7);
}

// ----- Stack / Offset

#[test]
fn stack_offset_load_store_and_alu() {
    let mut m = machine(&[
        reg_imm8(O::Mov, A, 0x11),
        raw(O::Psh, AddrMode::Reg, A as u8, 0, 0, 0), // $(0xFDFF) = 0x11
        reg_imm8(O::Mov, A, 0x22),
        raw(O::Psh, AddrMode::Reg, A as u8, 0, 0, 0), // $(0xFDFE) = 0x22
        stack_offset(O::Lod, B, 0),                   // B = top = 0x22
        stack_offset(O::Lod, C, 1),                   // C = 0x11
        reg_imm8(O::Mov, D, 0x05),
        stack_offset(O::Str, D, 1), // $(0xFDFF) = 5
        stack_offset(O::Add, D, 0), // 5 + 0x22 = 0x27
        stack_offset(O::Sub, D, 1), // 0x27 - 5 = 0x22
        stack_offset(O::Cmp, D, 0), // 0x22 = 0x22
        stack_offset(O::Xor, D, 0), // 0
        stack_offset(O::Or, D, 1),  // 5
        stack_offset(O::And, D, 0), // 5 & 0x22 = 0
        none(O::Stc),
        stack_offset(O::Adc, D, 1), // 0 + 5 + 1 = 6
        none(O::Stc),
        stack_offset(O::Sbb, D, 1), // 6 - 5 - 1 = 0
    ]);
    steps(&mut m, 6);
    assert_eq!((m.cpu.reg(B), m.cpu.reg(C)), (0x22, 0x11));
    assert_eq!(m.cpu.sp, 0xFDFE, "Stack / Offset does not move STK");
    steps(&mut m, 2);
    assert_eq!(ram(&m, 0xFDFF), 5);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(D), 0x27);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(D), 0x22);
    steps(&mut m, 1);
    assert!(m.cpu.flag(Flag::Equal));
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(D), 0);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(D), 5);
    steps(&mut m, 1);
    assert_eq!(m.cpu.reg(D), 0);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(D), 6);
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(D), 0);
    assert!(m.cpu.flag(Flag::Zero) && !m.cpu.flag(Flag::Carry));
}

#[test]
fn stack_offset_wraps_around_16_bits() {
    let mut m = Image::new()
        .program(&[imm16(O::Wsp, 0xFFF0), stack_offset(O::Lod, A, 0x20)])
        .byte(0x0010, 0x44)
        .build(bare());
    steps(&mut m, 2);
    assert_eq!(m.cpu.reg(A), 0x44);
}

#[test]
fn function_reads_its_arguments_with_stack_offset() {
    // Caller: psh 7, psh 5, cal add2, asp 2. add2 returns 7 + 5 in A,
    // using a 1-byte local. Inside add2, after ssp 1:
    //   STK+0 = local, STK+1 = return MSB, STK+2 = return LSB,
    //   STK+3 = 5 (last argument pushed), STK+4 = 7.
    let psh = |r: hbc2_core::Register| raw(O::Psh, AddrMode::Reg, r as u8, 0, 0, 0);
    let mut m = Image::new()
        .program(&[
            reg_imm8(O::Mov, A, 7),
            psh(A),
            reg_imm8(O::Mov, A, 5),
            psh(A),
            imm16(O::Cal, 0x2000),
            imm8(O::Asp, 2), // drop the arguments
            none(O::Hlt),
        ])
        .code(
            0x2000,
            &[
                imm8(O::Ssp, 1),            // one local
                stack_offset(O::Lod, A, 4), // a = 7
                stack_offset(O::Str, A, 0), // local = 7
                stack_offset(O::Lod, A, 3), // a = 5
                stack_offset(O::Add, A, 0), // a = 5 + local
                reg16(O::Rsp, X, Y),        // [x:y] = &local
                imm8(O::Asp, 1),            // free the local
                none(O::Ret),
            ],
        )
        .build(bare());
    steps(&mut m, 5);
    assert_eq!(m.cpu.pc, 0x2000);
    assert_eq!(m.cpu.sp, 0xFDFC);
    steps(&mut m, 8);
    assert_eq!(m.cpu.reg(A), 12);
    assert_eq!(
        (m.cpu.reg(X), m.cpu.reg(Y)),
        (0xFD, 0xFB),
        "RSP: address of the local"
    );
    assert_eq!(ram(&m, 0xFDFB), 7);
    assert_eq!(m.cpu.pc, at(5), "returned after CAL");
    steps(&mut m, 1);
    assert_eq!(m.cpu.sp, STACK_RESET, "stack balanced");
}

// ----- ASP, SSP, RSP, WSP

#[test]
fn stack_pointer_instructions() {
    let mut m = machine(&[
        imm8(O::Ssp, 0x10),    // 0xFDF0
        imm16(O::Ssp, 0x0300), // 0xFAF0
        imm8(O::Asp, 0x0F),    // 0xFAFF
        imm16(O::Asp, 0x0301), // 0xFE00
        reg16(O::Rsp, C, D),   // C:D = 0xFE00
        reg_imm8(O::Mov, A, 0x80),
        reg_imm8(O::Mov, B, 0x00),
        reg16(O::Wsp, A, B), // 0x8000
        imm16(O::Wsp, 0x0005),
        imm8(O::Ssp, 0x10), // 0xFFF5, wraps
    ]);
    let expected = [0xFDF0, 0xFAF0, 0xFAFF, 0xFE00];
    for sp in expected {
        steps(&mut m, 1);
        assert_eq!(m.cpu.sp, sp);
    }
    steps(&mut m, 1);
    assert_eq!((m.cpu.reg(C), m.cpu.reg(D)), (0xFE, 0x00));
    steps(&mut m, 3);
    assert_eq!(m.cpu.sp, 0x8000);
    steps(&mut m, 1);
    assert_eq!(m.cpu.sp, 0x0005);
    steps(&mut m, 1);
    assert_eq!(m.cpu.sp, 0xFFF5);
    assert_eq!(flags(&m), "--I-----", "no flags affected");
}
