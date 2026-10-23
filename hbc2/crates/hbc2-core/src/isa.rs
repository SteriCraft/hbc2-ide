//! Instruction set of the HBC-2 CPU (see "Instruction format and addressing
//! modes" and "Instruction set" in the HBC-2 Instruction Manual).
//!
//! Instruction format (32 bits, big-endian in memory):
//!
//! ```text
//! | Opcode (6b) | A.M. (4b) | R1 (3b) | R2 (3b) | V1 (8b) | V2 (8b) |
//! ```
//!
//! R3 (RamReg / ImmReg mode) lives in the low 3 bits of V1, and Vx is V1:V2.
//!
//! [`INSTRUCTION_TABLE`] is the single source of truth for mnemonics, allowed
//! addressing modes and affected flags. The CPU, the assembler, the
//! disassembler and the GUI should all read it rather than repeat it.

/// Instruction size in bytes.
pub const INSTRUCTION_SIZE: u16 = 4;
/// RAM size in bytes (16-bit address bus).
pub const RAM_SIZE: usize = 0x1_0000;
/// The stack occupies 0x0000-0x00FF (the stack pointer is 8 bits).
pub const STACK_START: u16 = 0x0000;
/// Interrupt vector table: 0x0100-0x02FF, one 16-bit address (MSB first) per I/O port.
pub const IVT_START: u16 = 0x0100;
pub const IVT_SIZE: u16 = 0x0200;
/// Execution starts here after power-on.
pub const PROGRAM_START: u16 = 0x0300;

/// The 8 general purpose registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Register {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    I = 4,
    J = 5,
    X = 6,
    Y = 7,
}

impl Register {
    pub const ALL: [Register; 8] = [
        Register::A,
        Register::B,
        Register::C,
        Register::D,
        Register::I,
        Register::J,
        Register::X,
        Register::Y,
    ];

    /// Decodes a 3-bit register selector (upper bits are ignored).
    pub const fn from_bits(bits: u8) -> Register {
        Register::ALL[(bits & 0x7) as usize]
    }

    pub const fn name(self) -> &'static str {
        ["a", "b", "c", "d", "i", "j", "x", "y"][self as usize]
    }
}

/// Bits of the flag register.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Flag {
    Carry = 0,
    Equal = 1,
    /// Set when the CPU accepts interrupts.
    Interrupt = 2,
    Negative = 3,
    Superior = 4,
    Zero = 5,
    Inferior = 6,
    Halt = 7,
}

impl Flag {
    pub const ALL: [Flag; 8] = [
        Flag::Carry,
        Flag::Equal,
        Flag::Interrupt,
        Flag::Negative,
        Flag::Superior,
        Flag::Zero,
        Flag::Inferior,
        Flag::Halt,
    ];

    /// One-letter name, as used in the manual.
    pub const fn letter(self) -> char {
        ['C', 'E', 'I', 'N', 'S', 'Z', 'F', 'H'][self as usize]
    }

    pub const fn name(self) -> &'static str {
        [
            "Carry",
            "Equal",
            "Interrupt",
            "Negative",
            "Superior",
            "Zero",
            "Inferior",
            "Halt",
        ][self as usize]
    }
}

/// Addressing modes (4-bit field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AddrMode {
    /// No operand.
    None = 0x0,
    /// Two 8-bit values from R1 and R2.
    Reg = 0x1,
    /// R1 and the immediate V1.
    RegImm8 = 0x2,
    /// R1 and the 16-bit address Vx.
    RegRam = 0x3,
    /// R1:R2 form a 16-bit address (R1 = MSB); R3 is used directly.
    RamRegImmReg = 0x4,
    /// R1:R2 form a 16-bit value (R1 = MSB).
    Reg16 = 0x5,
    /// The 16-bit immediate Vx.
    Imm16 = 0x6,
    /// The 8-bit immediate V1.
    Imm8 = 0x7,
}

impl AddrMode {
    pub const ALL: [AddrMode; 8] = [
        AddrMode::None,
        AddrMode::Reg,
        AddrMode::RegImm8,
        AddrMode::RegRam,
        AddrMode::RamRegImmReg,
        AddrMode::Reg16,
        AddrMode::Imm16,
        AddrMode::Imm8,
    ];

    /// Decodes the 4-bit addressing mode field. Values 0x8-0xF are not implemented.
    pub const fn from_bits(bits: u8) -> Option<AddrMode> {
        let bits = bits & 0xF;
        if bits < 8 {
            Some(AddrMode::ALL[bits as usize])
        } else {
            None
        }
    }

    /// Name as written in the manual.
    pub const fn name(self) -> &'static str {
        [
            "None",
            "Reg",
            "Reg / Imm8",
            "Reg / Ram",
            "RamReg / ImmReg",
            "Reg16",
            "Imm16",
            "Imm8",
        ][self as usize]
    }
}

/// The 48 implemented opcodes (6-bit field).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Opcode {
    Nop = 0x00,
    Adc = 0x01,
    Add = 0x02,
    And = 0x03,
    Cal = 0x04,
    Clc = 0x05,
    Cle = 0x06,
    Cli = 0x07,
    Cln = 0x08,
    Cls = 0x09,
    Clz = 0x0A,
    Clf = 0x0B,
    Cmp = 0x0C,
    Dec = 0x0D,
    Hlt = 0x0E,
    In = 0x0F,
    Out = 0x10,
    Inc = 0x11,
    Int = 0x12,
    Irt = 0x13,
    Jmc = 0x14,
    Jme = 0x15,
    Jmn = 0x16,
    Jmp = 0x17,
    Jms = 0x18,
    Jmz = 0x19,
    Jmf = 0x1A,
    Str = 0x1B,
    Lod = 0x1C,
    Mov = 0x1D,
    Not = 0x1E,
    Or = 0x1F,
    Pop = 0x20,
    Psh = 0x21,
    Ret = 0x22,
    Shl = 0x23,
    Asr = 0x24,
    Shr = 0x25,
    Stc = 0x26,
    Ste = 0x27,
    Sti = 0x28,
    Stn = 0x29,
    Sts = 0x2A,
    Stz = 0x2B,
    Stf = 0x2C,
    Sub = 0x2D,
    Sbb = 0x2E,
    Xor = 0x2F,
}

/// Number of implemented opcodes.
pub const OPCODE_COUNT: usize = 48;

impl Opcode {
    /// Decodes the 6-bit opcode field. Values 0x30-0x3F are not implemented.
    pub const fn from_bits(bits: u8) -> Option<Opcode> {
        let bits = (bits & 0x3F) as usize;
        if bits < OPCODE_COUNT {
            Some(INSTRUCTION_TABLE[bits].opcode)
        } else {
            None
        }
    }

    pub const fn info(self) -> &'static OpcodeInfo {
        &INSTRUCTION_TABLE[self as usize]
    }

    pub const fn mnemonic(self) -> &'static str {
        self.info().mnemonic
    }

    /// Looks up an opcode from its mnemonic (case-insensitive).
    pub fn from_mnemonic(mnemonic: &str) -> Option<Opcode> {
        INSTRUCTION_TABLE
            .iter()
            .find(|i| i.mnemonic.eq_ignore_ascii_case(mnemonic))
            .map(|i| i.opcode)
    }

    /// True if the manual allows this addressing mode for this opcode.
    pub fn accepts(self, mode: AddrMode) -> bool {
        self.info().modes.contains(&mode)
    }
}

/// Static description of one opcode, from the manual's instruction set table.
#[derive(Debug)]
pub struct OpcodeInfo {
    pub opcode: Opcode,
    pub mnemonic: &'static str,
    pub modes: &'static [AddrMode],
    pub flags: &'static [Flag],
}

use AddrMode as M;
use Flag as F;

const NONE: &[AddrMode] = &[M::None];
const ALU: &[AddrMode] = &[M::Reg, M::RegImm8, M::RegRam];
const JUMP: &[AddrMode] = &[M::Reg16, M::Imm16];
const MEM: &[AddrMode] = &[M::RamRegImmReg, M::RegRam];
const INC_DEC: &[AddrMode] = &[M::Reg, M::Reg16, M::Imm16];
const REG: &[AddrMode] = &[M::Reg];
const CNZ: &[Flag] = &[F::Carry, F::Negative, F::Zero];
const ZN: &[Flag] = &[F::Zero, F::Negative];

const fn op(
    opcode: Opcode,
    mnemonic: &'static str,
    modes: &'static [AddrMode],
    flags: &'static [Flag],
) -> OpcodeInfo {
    OpcodeInfo {
        opcode,
        mnemonic,
        modes,
        flags,
    }
}

/// The instruction set table, indexed by opcode value.
pub static INSTRUCTION_TABLE: [OpcodeInfo; OPCODE_COUNT] = [
    op(Opcode::Nop, "nop", NONE, &[]),
    op(Opcode::Adc, "adc", ALU, CNZ),
    op(Opcode::Add, "add", ALU, CNZ),
    op(Opcode::And, "and", ALU, ZN),
    op(Opcode::Cal, "cal", JUMP, &[]),
    op(Opcode::Clc, "clc", NONE, &[F::Carry]),
    op(Opcode::Cle, "cle", NONE, &[F::Equal]),
    op(Opcode::Cli, "cli", NONE, &[F::Interrupt]),
    op(Opcode::Cln, "cln", NONE, &[F::Negative]),
    op(Opcode::Cls, "cls", NONE, &[F::Superior]),
    op(Opcode::Clz, "clz", NONE, &[F::Zero]),
    op(Opcode::Clf, "clf", NONE, &[F::Inferior]),
    op(
        Opcode::Cmp,
        "cmp",
        &[M::Reg, M::RegImm8, M::RamRegImmReg],
        &[F::Superior, F::Equal, F::Inferior],
    ),
    op(Opcode::Dec, "dec", INC_DEC, CNZ),
    op(Opcode::Hlt, "hlt", NONE, &[F::Halt, F::Interrupt]),
    op(Opcode::In, "in", REG, &[]),
    op(Opcode::Out, "out", REG, &[]),
    op(Opcode::Inc, "inc", INC_DEC, CNZ),
    op(Opcode::Int, "int", &[M::Imm8], &[]),
    op(Opcode::Irt, "irt", NONE, &[F::Interrupt]),
    op(Opcode::Jmc, "jmc", JUMP, &[]),
    op(Opcode::Jme, "jme", JUMP, &[]),
    op(Opcode::Jmn, "jmn", JUMP, &[]),
    op(Opcode::Jmp, "jmp", JUMP, &[]),
    op(Opcode::Jms, "jms", JUMP, &[]),
    op(Opcode::Jmz, "jmz", JUMP, &[]),
    op(Opcode::Jmf, "jmf", JUMP, &[]),
    op(Opcode::Str, "str", MEM, &[]),
    op(Opcode::Lod, "lod", MEM, &[]),
    op(Opcode::Mov, "mov", &[M::Reg, M::RegImm8], &[]),
    op(Opcode::Not, "not", &[M::Reg, M::Imm16], ZN),
    op(Opcode::Or, "or", ALU, ZN),
    op(Opcode::Pop, "pop", REG, &[]),
    op(Opcode::Psh, "psh", REG, &[]),
    op(Opcode::Ret, "ret", NONE, &[]),
    op(Opcode::Shl, "shl", REG, &[F::Carry, F::Zero, F::Negative]),
    op(Opcode::Asr, "asr", REG, &[F::Carry, F::Zero, F::Negative]),
    op(Opcode::Shr, "shr", REG, ZN),
    op(Opcode::Stc, "stc", NONE, &[F::Carry]),
    op(Opcode::Ste, "ste", NONE, &[F::Equal]),
    op(Opcode::Sti, "sti", NONE, &[F::Interrupt]),
    op(Opcode::Stn, "stn", NONE, &[F::Negative]),
    op(Opcode::Sts, "sts", NONE, &[F::Superior]),
    op(Opcode::Stz, "stz", NONE, &[F::Zero]),
    op(Opcode::Stf, "stf", NONE, &[F::Inferior]),
    op(Opcode::Sub, "sub", ALU, CNZ),
    op(Opcode::Sbb, "sbb", ALU, CNZ),
    op(Opcode::Xor, "xor", ALU, ZN),
];

/// Field accessors for a raw 32-bit instruction word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Instruction(pub u32);

impl Instruction {
    #[inline(always)]
    pub const fn opcode_bits(self) -> u8 {
        (self.0 >> 26) as u8
    }
    #[inline(always)]
    pub const fn mode_bits(self) -> u8 {
        ((self.0 >> 22) & 0xF) as u8
    }
    #[inline(always)]
    pub const fn r1(self) -> u8 {
        ((self.0 >> 19) & 0x7) as u8
    }
    #[inline(always)]
    pub const fn r2(self) -> u8 {
        ((self.0 >> 16) & 0x7) as u8
    }
    /// R3 is stored in the low 3 bits of the V1 field.
    #[inline(always)]
    pub const fn r3(self) -> u8 {
        ((self.0 >> 8) & 0x7) as u8
    }
    #[inline(always)]
    pub const fn v1(self) -> u8 {
        (self.0 >> 8) as u8
    }
    #[inline(always)]
    pub const fn v2(self) -> u8 {
        self.0 as u8
    }
    /// V1:V2 as a 16-bit value (V1 = MSB).
    #[inline(always)]
    pub const fn vx(self) -> u16 {
        self.0 as u16
    }

    pub const fn opcode(self) -> Option<Opcode> {
        Opcode::from_bits(self.opcode_bits())
    }

    pub const fn mode(self) -> Option<AddrMode> {
        AddrMode::from_bits(self.mode_bits())
    }

    /// Big-endian bytes, as stored in memory.
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
}

/// Builds raw instruction words, one function per addressing mode.
/// Used by tests and tools; the assembler has its own front end.
pub mod encode {
    use super::{AddrMode, Instruction, Opcode, Register};

    /// Packs every field. Out-of-range bits are masked off.
    pub const fn raw(op: Opcode, mode: AddrMode, r1: u8, r2: u8, v1: u8, v2: u8) -> Instruction {
        Instruction(
            ((op as u32) << 26)
                | ((mode as u32) << 22)
                | (((r1 & 7) as u32) << 19)
                | (((r2 & 7) as u32) << 16)
                | ((v1 as u32) << 8)
                | v2 as u32,
        )
    }

    pub const fn none(op: Opcode) -> Instruction {
        raw(op, AddrMode::None, 0, 0, 0, 0)
    }
    pub const fn reg(op: Opcode, r1: Register, r2: Register) -> Instruction {
        raw(op, AddrMode::Reg, r1 as u8, r2 as u8, 0, 0)
    }
    pub const fn reg_imm8(op: Opcode, r1: Register, v1: u8) -> Instruction {
        raw(op, AddrMode::RegImm8, r1 as u8, 0, v1, 0)
    }
    pub const fn reg_ram(op: Opcode, r1: Register, address: u16) -> Instruction {
        raw(
            op,
            AddrMode::RegRam,
            r1 as u8,
            0,
            (address >> 8) as u8,
            address as u8,
        )
    }
    pub const fn ramreg_immreg(
        op: Opcode,
        r1: Register,
        r2: Register,
        r3: Register,
    ) -> Instruction {
        raw(op, AddrMode::RamRegImmReg, r1 as u8, r2 as u8, r3 as u8, 0)
    }
    pub const fn reg16(op: Opcode, r1: Register, r2: Register) -> Instruction {
        raw(op, AddrMode::Reg16, r1 as u8, r2 as u8, 0, 0)
    }
    pub const fn imm16(op: Opcode, value: u16) -> Instruction {
        raw(op, AddrMode::Imm16, 0, 0, (value >> 8) as u8, value as u8)
    }
    pub const fn imm8(op: Opcode, v1: u8) -> Instruction {
        raw(op, AddrMode::Imm8, 0, 0, v1, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_indexed_by_opcode() {
        for (i, info) in INSTRUCTION_TABLE.iter().enumerate() {
            assert_eq!(info.opcode as usize, i, "{} is out of place", info.mnemonic);
        }
    }

    #[test]
    fn unimplemented_values_decode_to_none() {
        assert_eq!(Opcode::from_bits(0x30), None);
        assert_eq!(AddrMode::from_bits(0x8), None);
    }

    #[test]
    fn fields_round_trip() {
        let i = encode::raw(Opcode::Xor, AddrMode::RegRam, 5, 3, 0xAB, 0xCD);
        assert_eq!(i.opcode(), Some(Opcode::Xor));
        assert_eq!(i.mode(), Some(AddrMode::RegRam));
        assert_eq!(
            (i.r1(), i.r2(), i.v1(), i.v2(), i.vx()),
            (5, 3, 0xAB, 0xCD, 0xABCD)
        );
        assert_eq!(
            encode::ramreg_immreg(Opcode::Str, Register::A, Register::B, Register::Y).r3(),
            7
        );
    }

    #[test]
    fn mnemonic_lookup() {
        assert_eq!(Opcode::from_mnemonic("JMP"), Some(Opcode::Jmp));
        assert_eq!(Opcode::from_mnemonic("xyz"), None);
    }
}
