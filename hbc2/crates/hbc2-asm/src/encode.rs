//! Operand forms of each instruction, and their encoding.
//!
//! Operand order is always destination first:
//! `str address, r` stores r; `lod r, address` loads r; `out r1, r2` writes r2 to port r1.

use hbc2_core::isa::{Instruction, encode};
use hbc2_core::{AddrMode, Opcode, Register};

/// Kind of a written operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A register: `a`
    R,
    /// A register pair: `[x:y]`, or `[x:y + n]` when the form allows an offset
    P,
    /// A byte on the stack: `[sp]`, `[sp + n]`
    S,
    /// An 8-bit value: `42`, `'A'`, `name.lsb`
    I,
    /// A 16-bit number (256 to 65535): `300`
    W,
    /// A memory address: `$0x9000`, `name`, `name + 1`
    M,
}

impl Kind {
    pub fn usage(self) -> &'static str {
        match self {
            Kind::R => "r",
            Kind::P => "[r:r]",
            Kind::S => "[sp + n]",
            Kind::I => "value",
            Kind::W => "value16",
            Kind::M => "address",
        }
    }
}

impl Form {
    /// True if a register pair in this form can have an offset (`[x:y + n]`).
    pub fn takes_offset(&self) -> bool {
        self.mode == AddrMode::RamRegImmReg
    }
}

/// One accepted way of writing an instruction.
#[derive(Debug)]
pub struct Form {
    pub kinds: &'static [Kind],
    pub mode: AddrMode,
}

const fn form(kinds: &'static [Kind], mode: AddrMode) -> Form {
    Form { kinds, mode }
}

use AddrMode as A;
use Kind::*;

const NO_OPERAND: &[Form] = &[form(&[], A::None)];
const ALU: &[Form] = &[
    form(&[R, R], A::Reg),
    form(&[R, I], A::RegImm8),
    form(&[R, M], A::RegRam),
    form(&[R, P], A::RamRegImmReg),
    form(&[R, S], A::StackOffset),
];
const CMP: &[Form] = &[
    form(&[R, R], A::Reg),
    form(&[R, I], A::RegImm8),
    form(&[R, P], A::RamRegImmReg),
    form(&[R, S], A::StackOffset),
];
const MOV: &[Form] = &[form(&[R, R], A::Reg), form(&[R, I], A::RegImm8)];
const LOD: &[Form] = &[
    form(&[R, M], A::RegRam),
    form(&[R, P], A::RamRegImmReg),
    form(&[R, S], A::StackOffset),
];
const STR: &[Form] = &[
    form(&[M, R], A::RegRam),
    form(&[P, R], A::RamRegImmReg),
    form(&[S, R], A::StackOffset),
];
// `inc [x:y]` is RamReg / ImmReg / Offset with offset 0 (no data register).
const INC_DEC: &[Form] = &[
    form(&[R], A::Reg),
    form(&[P], A::RamRegImmReg),
    form(&[M], A::Imm16),
];
const STACK_MOVE: &[Form] = &[form(&[I], A::Imm8), form(&[W], A::Imm16)];
const RSP: &[Form] = &[form(&[P], A::Reg16)];
const WSP: &[Form] = &[form(&[P], A::Reg16), form(&[M], A::Imm16)];
const NOT: &[Form] = &[form(&[R], A::Reg), form(&[M], A::Imm16)];
const JUMP: &[Form] = &[form(&[M], A::Imm16), form(&[P], A::Reg16)];
const IO: &[Form] = &[form(&[R, R], A::Reg)];
const INT: &[Form] = &[form(&[I], A::Imm8)];
const ONE_REG: &[Form] = &[form(&[R], A::Reg)];

pub fn forms(op: Opcode) -> &'static [Form] {
    use Opcode as O;
    match op {
        O::Adc | O::Add | O::And | O::Or | O::Sub | O::Sbb | O::Xor => ALU,
        O::Cmp => CMP,
        O::Mov => MOV,
        O::Lod => LOD,
        O::Str => STR,
        O::Inc | O::Dec => INC_DEC,
        O::Not => NOT,
        O::Cal | O::Jmp | O::Jmc | O::Jme | O::Jmn | O::Jms | O::Jmz | O::Jmf => JUMP,
        O::In | O::Out => IO,
        O::Int => INT,
        O::Asp | O::Ssp => STACK_MOVE,
        O::Rsp => RSP,
        O::Wsp => WSP,
        O::Psh | O::Pop | O::Shl | O::Asr | O::Shr => ONE_REG,
        O::Nop
        | O::Clc
        | O::Cle
        | O::Cli
        | O::Cln
        | O::Cls
        | O::Clz
        | O::Clf
        | O::Hlt
        | O::Irt
        | O::Ret
        | O::Stc
        | O::Ste
        | O::Sti
        | O::Stn
        | O::Sts
        | O::Stz
        | O::Stf
        | O::Psf
        | O::Pof => NO_OPERAND,
    }
}

/// "mov r, r | mov r, value"
pub fn usage(op: Opcode) -> String {
    let mut usages: Vec<String> = forms(op)
        .iter()
        .map(|f| {
            let operands: Vec<&str> = f
                .kinds
                .iter()
                .map(|&k| {
                    if k == P && f.takes_offset() {
                        "[r:r + n]"
                    } else {
                        k.usage()
                    }
                })
                .collect();
            if operands.is_empty() {
                op.mnemonic().to_string()
            } else {
                format!("{} {}", op.mnemonic(), operands.join(", "))
            }
        })
        .collect();
    if matches!(op, Opcode::Psh | Opcode::Pop) {
        // Register lists, expanded by the parser.
        usages.push(format!("{} r, r, …", op.mnemonic()));
    }
    usages.join(" | ")
}

/// An operand once names are resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    Reg(Register),
    /// (high, low, offset)
    Pair(Register, Register, u8),
    Stack(u8),
    Imm(u8),
    Wide(u16),
    Mem(u16),
}

impl Resolved {
    pub fn kind(self) -> Kind {
        match self {
            Resolved::Reg(_) => R,
            Resolved::Pair(..) => P,
            Resolved::Stack(_) => S,
            Resolved::Imm(_) => I,
            Resolved::Wide(_) => W,
            Resolved::Mem(_) => M,
        }
    }
}

/// Finds the form matching the operands and encodes the instruction.
/// A pair with an offset only matches forms that take one.
pub fn encode_instruction(op: Opcode, operands: &[Resolved]) -> Option<Instruction> {
    let kinds: Vec<Kind> = operands.iter().map(|o| o.kind()).collect();
    let has_offset = operands
        .iter()
        .any(|o| matches!(o, Resolved::Pair(_, _, n) if *n != 0));
    let form = forms(op)
        .iter()
        .find(|f| f.kinds == kinds.as_slice() && (!has_offset || f.takes_offset()))?;

    use Resolved as V;
    let reg = |i: usize| match operands[i] {
        V::Reg(r) => r,
        _ => unreachable!("form checked"),
    };
    let pair = |i: usize| match operands[i] {
        V::Pair(h, l, offset) => (h, l, offset),
        _ => unreachable!("form checked"),
    };
    let slot = |i: usize| match operands[i] {
        V::Stack(offset) => offset,
        _ => unreachable!("form checked"),
    };
    let imm = |i: usize| match operands[i] {
        V::Imm(v) => v,
        _ => unreachable!("form checked"),
    };
    let mem = |i: usize| match operands[i] {
        V::Mem(a) => a,
        _ => unreachable!("form checked"),
    };

    Some(match form.mode {
        A::None => encode::none(op),
        A::Reg => {
            let r2 = if operands.len() > 1 {
                reg(1)
            } else {
                Register::A
            };
            encode::reg(op, reg(0), r2)
        }
        A::RegImm8 => encode::reg_imm8(op, reg(0), imm(1)),
        A::RegRam if op == Opcode::Str => encode::reg_ram(op, reg(1), mem(0)),
        A::RegRam => encode::reg_ram(op, reg(0), mem(1)),
        A::RamRegImmReg if op == Opcode::Str => {
            let (h, l, offset) = pair(0);
            encode::ramreg_offset(op, h, l, reg(1), offset)
        }
        // inc / dec: no data register (V1 = 0x00).
        A::RamRegImmReg if operands.len() == 1 => {
            let (h, l, offset) = pair(0);
            encode::ramreg_offset(op, h, l, Register::A, offset)
        }
        A::RamRegImmReg => {
            let (h, l, offset) = pair(1);
            encode::ramreg_offset(op, h, l, reg(0), offset)
        }
        A::StackOffset if op == Opcode::Str => encode::stack_offset(op, reg(1), slot(0)),
        A::StackOffset => encode::stack_offset(op, reg(0), slot(1)),
        A::Reg16 => {
            let (h, l, _) = pair(0);
            encode::reg16(op, h, l)
        }
        A::Imm16 => match operands[0] {
            V::Wide(n) => encode::imm16(op, n),
            _ => encode::imm16(op, mem(0)),
        },
        A::Imm8 => encode::imm8(op, imm(0)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbc2_core::isa::INSTRUCTION_TABLE;

    #[test]
    fn forms_match_the_instruction_table() {
        for info in INSTRUCTION_TABLE.iter() {
            let mut modes: Vec<AddrMode> = forms(info.opcode).iter().map(|f| f.mode).collect();
            let mut expected = info.modes.to_vec();
            modes.sort_by_key(|m| *m as u8);
            expected.sort_by_key(|m| *m as u8);
            assert_eq!(modes, expected, "{}", info.mnemonic);
        }
    }

    #[test]
    fn usage_text() {
        assert_eq!(usage(Opcode::Mov), "mov r, r | mov r, value");
        assert_eq!(usage(Opcode::Ret), "ret");
        assert_eq!(usage(Opcode::Pop), "pop r | pop r, r, …");
        assert_eq!(
            usage(Opcode::Lod),
            "lod r, address | lod r, [r:r + n] | lod r, [sp + n]"
        );
        assert_eq!(usage(Opcode::Wsp), "wsp [r:r] | wsp address");
        assert_eq!(usage(Opcode::Ssp), "ssp value | ssp value16");
    }

    #[test]
    fn offsets() {
        use Register::*;
        use Resolved as V;
        assert_eq!(
            encode_instruction(Opcode::Inc, &[V::Pair(X, Y, 0)]),
            Some(encode::ramreg_offset(Opcode::Inc, X, Y, A, 0)),
            "inc [x:y] uses mode 0x4"
        );
        assert_eq!(
            encode_instruction(Opcode::Str, &[V::Pair(X, Y, 3), V::Reg(B)]),
            Some(encode::ramreg_offset(Opcode::Str, X, Y, B, 3))
        );
        assert_eq!(
            encode_instruction(Opcode::Str, &[V::Stack(3), V::Reg(B)]),
            Some(encode::stack_offset(Opcode::Str, B, 3))
        );
        assert_eq!(encode_instruction(Opcode::Jmp, &[V::Pair(X, Y, 1)]), None);
        assert_eq!(
            encode_instruction(Opcode::Ssp, &[V::Wide(300)]),
            Some(encode::imm16(Opcode::Ssp, 300))
        );
    }
}
