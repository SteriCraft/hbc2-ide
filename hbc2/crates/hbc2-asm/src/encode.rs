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
    /// A register pair: `[x:y]`
    P,
    /// An 8-bit value: `42`, `'A'`, `name.lsb`
    I,
    /// A memory address: `$0x9000`, `name`, `name + 1`
    M,
}

impl Kind {
    pub fn usage(self) -> &'static str {
        match self {
            Kind::R => "r",
            Kind::P => "[r:r]",
            Kind::I => "value",
            Kind::M => "address",
        }
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
];
const CMP: &[Form] = &[
    form(&[R, R], A::Reg),
    form(&[R, I], A::RegImm8),
    form(&[R, P], A::RamRegImmReg),
];
const MOV: &[Form] = &[form(&[R, R], A::Reg), form(&[R, I], A::RegImm8)];
const LOD: &[Form] = &[form(&[R, M], A::RegRam), form(&[R, P], A::RamRegImmReg)];
const STR: &[Form] = &[form(&[M, R], A::RegRam), form(&[P, R], A::RamRegImmReg)];
const INC_DEC: &[Form] = &[
    form(&[R], A::Reg),
    form(&[P], A::Reg16),
    form(&[M], A::Imm16),
];
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
            let operands: Vec<&str> = f.kinds.iter().map(|k| k.usage()).collect();
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
    Pair(Register, Register),
    Imm(u8),
    Mem(u16),
}

impl Resolved {
    pub fn kind(self) -> Kind {
        match self {
            Resolved::Reg(_) => R,
            Resolved::Pair(..) => P,
            Resolved::Imm(_) => I,
            Resolved::Mem(_) => M,
        }
    }
}

/// Finds the form matching the operands and encodes the instruction.
pub fn encode_instruction(op: Opcode, operands: &[Resolved]) -> Option<Instruction> {
    let kinds: Vec<Kind> = operands.iter().map(|o| o.kind()).collect();
    let form = forms(op).iter().find(|f| f.kinds == kinds.as_slice())?;

    use Resolved as V;
    let reg = |i: usize| match operands[i] {
        V::Reg(r) => r,
        _ => unreachable!("form checked"),
    };
    let pair = |i: usize| match operands[i] {
        V::Pair(h, l) => (h, l),
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
            let (h, l) = pair(0);
            encode::ramreg_immreg(op, h, l, reg(1))
        }
        A::RamRegImmReg => {
            let (h, l) = pair(1);
            encode::ramreg_immreg(op, h, l, reg(0))
        }
        A::Reg16 => {
            let (h, l) = pair(0);
            encode::reg16(op, h, l)
        }
        A::Imm16 => encode::imm16(op, mem(0)),
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
    }
}
