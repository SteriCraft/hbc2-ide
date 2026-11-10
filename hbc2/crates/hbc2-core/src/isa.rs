//! Instruction set of the HBC-2 CPU (see "Instruction format and addressing
//! modes" and "Instruction set" in the HBC-2 Instruction Manual).
//!
//! Instruction format (1 to 4 bytes, in this order in memory):
//!
//! ```text
//! | Opcode (6b) | A.M. (4b) | R1 (3b) | R2 (3b) | V1 (8b) | V2 (8b) |
//! ```
//!
//! R3 (RamReg / ImmReg / Offset mode) lives in the low 3 bits of V1, and Vx is V1:V2.
//!
//! An instruction is this format with the bytes it doesn't use left off. The
//! CPU reads the first byte: an opcode without operands is 1 byte long (its 2
//! addressing mode bits there must be 0). Otherwise it reads the second byte,
//! and the addressing mode gives the length ([`AddrMode::length`]). See
//! [`Instruction::size`] and [`Instruction::stops`] for invalid instructions.
//!
//! [`Instruction`] holds the 4-byte form; the bytes past the length are 0.
//!
//! [`INSTRUCTION_TABLE`] is the single source of truth for mnemonics, allowed
//! addressing modes and affected flags. The CPU, the assembler, the
//! disassembler and the GUI should all read it rather than repeat it.

/// Size in bytes of the longest instruction.
pub const MAX_INSTRUCTION_SIZE: u16 = 4;
/// RAM size in bytes (16-bit address bus).
pub const RAM_SIZE: usize = 0x1_0000;
/// Stack pointer value at reset. The stack grows downward: the first push
/// writes at 0xFDFF, just below the interrupt vector table.
pub const STACK_RESET: u16 = 0xFE00;
/// Interrupt vector table: 0xFE00-0xFFFF, one 16-bit address (MSB first) per I/O port.
pub const IVT_START: u16 = 0xFE00;
pub const IVT_SIZE: u16 = 0x0200;
/// Execution starts here after power-on.
pub const PROGRAM_START: u16 = 0x0000;

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
    /// R1:R2 form a 16-bit address (R1 = MSB) plus the offset V2; R3 is used directly.
    RamRegImmReg = 0x4,
    /// The byte at STK + V1; R1 holds the data.
    StackOffset = 0x5,
    /// R1:R2 form a 16-bit value (R1 = MSB).
    Reg16 = 0x6,
    /// The 16-bit immediate Vx.
    Imm16 = 0x7,
    /// The 8-bit immediate V1.
    Imm8 = 0x8,
}

impl AddrMode {
    pub const ALL: [AddrMode; 9] = [
        AddrMode::None,
        AddrMode::Reg,
        AddrMode::RegImm8,
        AddrMode::RegRam,
        AddrMode::RamRegImmReg,
        AddrMode::StackOffset,
        AddrMode::Reg16,
        AddrMode::Imm16,
        AddrMode::Imm8,
    ];

    /// Decodes the 4-bit addressing mode field. Values 0x9-0xF are not implemented.
    pub const fn from_bits(bits: u8) -> Option<AddrMode> {
        let bits = bits & 0xF;
        if (bits as usize) < AddrMode::ALL.len() {
            Some(AddrMode::ALL[bits as usize])
        } else {
            None
        }
    }

    /// Length in bytes of an instruction in this mode.
    pub const fn length(self) -> u16 {
        [1, 2, 3, 4, 4, 3, 2, 4, 3][self as usize]
    }

    /// Name as written in the manual.
    pub const fn name(self) -> &'static str {
        [
            "None",
            "Reg",
            "Reg / Imm8",
            "Reg / Ram",
            "RamReg / ImmReg / Offset",
            "Stack / Offset",
            "Reg16",
            "Imm16",
            "Imm8",
        ][self as usize]
    }
}

/// The 54 implemented opcodes (6-bit field).
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
    Psf = 0x2D,
    Pof = 0x2E,
    Sub = 0x2F,
    Sbb = 0x30,
    Xor = 0x31,
    Asp = 0x32,
    Ssp = 0x33,
    Rsp = 0x34,
    Wsp = 0x35,
}

/// Number of implemented opcodes.
pub const OPCODE_COUNT: usize = 54;

impl Opcode {
    /// Decodes the 6-bit opcode field. Values 0x36-0x3F are not implemented.
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

    /// False for the opcodes without operands (mode None only, 1 byte).
    /// An opcode has operands in all its forms or in none.
    pub const fn takes_operands(self) -> bool {
        !matches!(self.info().modes[0], AddrMode::None)
    }
}

/// Clock frequency of the hardware CPU: one control-unit step every 500 ns.
pub const HARDWARE_CLOCK_HZ: u64 = 2_000_000;

/// Clock cycles of a hardware interrupt entry, from the end of the
/// instruction before it to the handler's first fetch (W1, IE1-IE8 in
/// `logisim/notes/cu-cycles.md`).
pub const HARDWARE_INTERRUPT_CYCLES: u8 = 9;

/// Clock cycles of a software interrupt entry, from the end of `INT` (or of
/// the instruction that set I) to the handler's first fetch (W1, W2, IE1-IE8).
pub const SOFTWARE_INTERRUPT_CYCLES: u8 = 10;

/// Clock cycles of one turn of the halt loop (W1, W2): a halted CPU looks
/// for an interrupt once per turn.
pub const HALT_LOOP_CYCLES: u8 = 2;

/// Clock cycles from the reset release to the first fetch: the boot ROM
/// copies 64 KiB into RAM. The emulator fills RAM at once at power-on, then
/// waits this long before the first instruction.
pub const HARDWARE_BOOT_CYCLES: u64 = 65_548;

/// Clock cycles of each instruction, by opcode bits and addressing mode bits,
/// from the fetch of its first byte to the next instruction's: the hardware
/// control unit's counts (`logisim/notes/cu-cycles.md`, generated from the
/// microcode; `cargo test -p hbc2-ucode` checks this table against it).
/// A conditional jump takes as long taken or not. For an instruction that
/// [stops](Instruction::stops), the cycles before the halt loop.
static CYCLES: [[u8; 16]; 64] = cycle_table();

const fn cycle_table() -> [[u8; 16]; 64] {
    let mut table = [[1u8; 16]; 64]; // invalid opcodes: 1-cycle NOP
    let mut op = 0;
    while op < OPCODE_COUNT {
        let opcode = INSTRUCTION_TABLE[op].opcode;
        let mut mode = 0;
        while mode < 16 {
            let stops = DECODE[op][mode] & STOPS != 0;
            table[op][mode] = match (stops, AddrMode::from_bits(mode as u8)) {
                (false, Some(m)) if opcode.takes_operands() => cycles(opcode, m),
                (false, _) => cycles(opcode, AddrMode::None),
                (true, _) if opcode.takes_operands() => 4,
                (true, _) => 3,
            };
            mode += 1;
        }
        op += 1;
    }
    table
}

/// Cycles of an opcode in one of its addressing modes.
const fn cycles(opcode: Opcode, mode: AddrMode) -> u8 {
    use AddrMode as M;
    use Opcode as O;
    match opcode {
        O::Nop => 1,
        O::Adc | O::Add | O::And | O::Or | O::Sub | O::Sbb | O::Xor => match mode {
            M::Reg => 4,
            M::RegImm8 => 5,
            M::StackOffset => 7,
            _ => 8, // Reg / Ram, RamReg / ImmReg / Offset
        },
        O::Cmp => match mode {
            M::Reg => 3,
            M::RegImm8 => 4,
            M::StackOffset => 6,
            _ => 7, // RamReg / ImmReg / Offset
        },
        O::Cal => match mode {
            M::Reg16 => 5,
            _ => 7, // Imm16
        },
        O::Jmc | O::Jme | O::Jmn | O::Jmp | O::Jms | O::Jmz | O::Jmf | O::Wsp => match mode {
            M::Reg16 => 3,
            _ => 5, // Imm16
        },
        O::Inc | O::Dec | O::Not => match mode {
            M::Reg => 4,
            _ => 8, // Imm16, RamReg / ImmReg / Offset
        },
        O::Str | O::Lod => match mode {
            M::StackOffset => 5,
            _ => 6, // Reg / Ram, RamReg / ImmReg / Offset
        },
        O::Mov => match mode {
            M::RegImm8 => 3,
            _ => 4, // Reg
        },
        O::Asp | O::Ssp => match mode {
            M::Imm8 => 4,
            _ => 5, // Imm16
        },
        O::In | O::Out | O::Int | O::Psh | O::Shl | O::Asr | O::Shr | O::Rsp => 4,
        O::Pop | O::Ret => 5,
        O::Irt => 6,
        O::Psf => 3,
        O::Pof => 4,
        O::Clc | O::Cle | O::Cli | O::Cln | O::Cls | O::Clz | O::Clf => 2,
        O::Stc | O::Ste | O::Sti | O::Stn | O::Sts | O::Stz | O::Stf => 2,
        O::Hlt => 2,
    }
}

/// How the CPU reads an instruction, by opcode bits and addressing mode bits:
/// its size in bytes, plus [`STOPS`] when the CPU stops on it.
static DECODE: [[u8; 16]; 64] = decode_table();
const STOPS: u8 = 0x80;

const fn decode_table() -> [[u8; 16]; 64] {
    let mut table = [[1u8; 16]; 64]; // invalid opcodes: 1-byte NOP
    let mut op = 0;
    while op < OPCODE_COUNT {
        let info = &INSTRUCTION_TABLE[op];
        let mut mode = 0;
        while mode < 16 {
            table[op][mode] = if !info.opcode.takes_operands() {
                // Only the 2 mode bits of the first byte are read.
                if mode & 0b1100 == 0 { 1 } else { STOPS | 1 }
            } else {
                match AddrMode::from_bits(mode as u8) {
                    Some(m) if mode_listed(info.modes, m) => m.length() as u8,
                    _ => STOPS | 2,
                }
            };
            mode += 1;
        }
        op += 1;
    }
    table
}

const fn mode_listed(modes: &[AddrMode], mode: AddrMode) -> bool {
    let mut i = 0;
    while i < modes.len() {
        if modes[i] as u8 == mode as u8 {
            return true;
        }
        i += 1;
    }
    false
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
const ALU: &[AddrMode] = &[
    M::Reg,
    M::RegImm8,
    M::RegRam,
    M::RamRegImmReg,
    M::StackOffset,
];
const JUMP: &[AddrMode] = &[M::Reg16, M::Imm16];
const MEM: &[AddrMode] = &[M::RamRegImmReg, M::RegRam, M::StackOffset];
const INC_DEC: &[AddrMode] = &[M::Reg, M::Imm16, M::RamRegImmReg];
const STACK_MOVE: &[AddrMode] = &[M::Imm8, M::Imm16];
const REG: &[AddrMode] = &[M::Reg];
const CNZ: &[Flag] = &[F::Carry, F::Negative, F::Zero];
const ZN: &[Flag] = &[F::Zero, F::Negative];
const ALL_FLAGS: &[Flag] = &Flag::ALL;

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
        &[M::Reg, M::RegImm8, M::RamRegImmReg, M::StackOffset],
        &[F::Superior, F::Equal, F::Inferior],
    ),
    op(Opcode::Dec, "dec", INC_DEC, CNZ),
    op(Opcode::Hlt, "hlt", NONE, &[F::Halt]),
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
    op(Opcode::Psf, "psf", NONE, &[]),
    op(Opcode::Pof, "pof", NONE, ALL_FLAGS),
    op(Opcode::Sub, "sub", ALU, CNZ),
    op(Opcode::Sbb, "sbb", ALU, CNZ),
    op(Opcode::Xor, "xor", ALU, ZN),
    op(Opcode::Asp, "asp", STACK_MOVE, &[]),
    op(Opcode::Ssp, "ssp", STACK_MOVE, &[]),
    op(Opcode::Rsp, "rsp", &[M::Reg16], &[]),
    op(Opcode::Wsp, "wsp", &[M::Reg16, M::Imm16], &[]),
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

    /// The 4-byte form, in memory order. Only the first [`Instruction::size`]
    /// bytes are part of the instruction.
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    /// Bytes the CPU reads for this instruction (1 to 4):
    /// - an invalid opcode is a 1-byte NOP;
    /// - an opcode without operands is 1 byte;
    /// - otherwise the addressing mode gives the length.
    ///
    /// When [`Instruction::stops`], this is the bytes read before stopping.
    #[inline(always)]
    pub fn size(self) -> u16 {
        (self.decode() & !STOPS) as u16
    }

    /// True if the CPU stops on this instruction (H set, I cleared): a valid
    /// opcode with an addressing mode it doesn't accept. For an opcode without
    /// operands, that's mode bits in the first byte that aren't 0.
    #[inline(always)]
    pub fn stops(self) -> bool {
        self.decode() & STOPS != 0
    }

    /// The instruction the CPU reads from these 4 bytes: the bytes past its
    /// size are cleared.
    #[inline(always)]
    pub fn fetched(word: u32) -> Instruction {
        const MASK: [u32; 5] = [0, 0xFF00_0000, 0xFFFF_0000, 0xFFFF_FF00, 0xFFFF_FFFF];
        Instruction(word & MASK[Instruction(word).size() as usize])
    }

    /// Clock cycles this instruction takes on the hardware (see [`CYCLES`]).
    #[inline(always)]
    pub fn cycles(self) -> u8 {
        CYCLES[self.opcode_bits() as usize][self.mode_bits() as usize]
    }

    #[inline(always)]
    fn decode(self) -> u8 {
        DECODE[self.opcode_bits() as usize][self.mode_bits() as usize]
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
    /// R3 = data register, (R1:R2) = address, no offset.
    pub const fn ramreg_immreg(
        op: Opcode,
        r1: Register,
        r2: Register,
        r3: Register,
    ) -> Instruction {
        ramreg_offset(op, r1, r2, r3, 0)
    }
    /// R3 = data register, (R1:R2) + offset = address.
    pub const fn ramreg_offset(
        op: Opcode,
        r1: Register,
        r2: Register,
        r3: Register,
        offset: u8,
    ) -> Instruction {
        raw(
            op,
            AddrMode::RamRegImmReg,
            r1 as u8,
            r2 as u8,
            r3 as u8,
            offset,
        )
    }
    /// R1 = data register, STK + offset = address.
    pub const fn stack_offset(op: Opcode, r1: Register, offset: u8) -> Instruction {
        raw(op, AddrMode::StackOffset, r1 as u8, 0, offset, 0)
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
        assert_eq!(Opcode::from_bits(0x35), Some(Opcode::Wsp));
        assert_eq!(Opcode::from_bits(0x36), None);
        assert_eq!(AddrMode::from_bits(0x8), Some(AddrMode::Imm8));
        assert_eq!(AddrMode::from_bits(0x9), None);
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
    fn sizes() {
        use Register::*;
        let size = |i: Instruction| (i.size(), i.stops());
        assert_eq!(size(encode::none(Opcode::Ret)), (1, false));
        assert_eq!(size(encode::reg(Opcode::Mov, A, B)), (2, false));
        assert_eq!(size(encode::reg16(Opcode::Jmp, X, Y)), (2, false));
        assert_eq!(size(encode::reg_imm8(Opcode::Mov, A, 5)), (3, false));
        assert_eq!(size(encode::stack_offset(Opcode::Lod, A, 2)), (3, false));
        assert_eq!(size(encode::imm8(Opcode::Int, 2)), (3, false));
        assert_eq!(size(encode::reg_ram(Opcode::Lod, A, 0x9000)), (4, false));
        assert_eq!(
            size(encode::ramreg_offset(Opcode::Str, X, Y, A, 1)),
            (4, false)
        );
        assert_eq!(size(encode::imm16(Opcode::Jmp, 0x0300)), (4, false));
        for info in INSTRUCTION_TABLE.iter() {
            for &mode in info.modes {
                let i = encode::raw(info.opcode, mode, 0, 0, 0, 0);
                assert_eq!(
                    size(i),
                    (mode.length(), false),
                    "{} {mode:?}",
                    info.mnemonic
                );
            }
        }
    }

    #[test]
    fn invalid_instructions() {
        let word = |op: u8, mode: u8| Instruction(((op as u32) << 26) | ((mode as u32) << 22));
        // Invalid opcode: 1-byte NOP, whatever follows.
        assert_eq!(word(0x36, 0xF).size(), 1);
        assert!(!word(0x3F, 0x3).stops());
        // Valid opcode, mode it doesn't accept: stops after reading 2 bytes.
        let add_imm16 = word(Opcode::Add as u8, AddrMode::Imm16 as u8);
        assert_eq!((add_imm16.size(), add_imm16.stops()), (2, true));
        assert!(word(Opcode::Jmp as u8, 0x9).stops());
        assert!(word(Opcode::Mov as u8, AddrMode::None as u8).stops());
        // No-operand opcode: only the 2 mode bits of the first byte count.
        assert_eq!(word(Opcode::Ret as u8, 0b0011).size(), 1);
        assert!(!word(Opcode::Ret as u8, 0b0011).stops());
        assert!(word(Opcode::Ret as u8, 0b0100).stops());
        assert_eq!(word(Opcode::Nop as u8, 0b1000).size(), 1);
        assert!(word(Opcode::Nop as u8, 0b1000).stops());
    }

    #[test]
    fn fetched_clears_the_bytes_past_the_size() {
        let ret = encode::none(Opcode::Ret);
        assert_eq!(Instruction::fetched(ret.0 | 0x00AB_CDEF), ret);
        let mov = encode::reg_imm8(Opcode::Mov, Register::A, 5);
        assert_eq!(Instruction::fetched(mov.0 | 0xEE), mov);
    }

    #[test]
    fn mnemonic_lookup() {
        assert_eq!(Opcode::from_mnemonic("JMP"), Some(Opcode::Jmp));
        assert_eq!(Opcode::from_mnemonic("xyz"), None);
    }
}
