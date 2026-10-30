//! Turns instruction words back into `.has` text.

use hbc2_core::isa::Instruction;
use hbc2_core::{AddrMode, Opcode, Register};

use crate::encode::forms;

/// Disassembles one instruction word, e.g. `lod a, $0x9000`.
pub fn disassemble(word: u32) -> String {
    disassemble_with(word, |_| None)
}

/// Like [`disassemble`], with `name_of` giving names for addresses (labels, variables).
pub fn disassemble_with(word: u32, name_of: impl Fn(u16) -> Option<String>) -> String {
    let i = Instruction(word);
    let (Some(op), Some(mode)) = (i.opcode(), i.mode()) else {
        return format!("??? ({word:#010x})");
    };
    let Some(form) = forms(op).iter().find(|f| f.mode == mode) else {
        return format!("??? ({word:#010x})");
    };

    let reg = |bits: u8| Register::from_bits(bits).name().to_string();
    let pair = || format!("[{}:{}]", reg(i.r1()), reg(i.r2()));
    let pair_offset = || match i.v2() {
        0 => pair(),
        n => format!("[{}:{} + {n}]", reg(i.r1()), reg(i.r2())),
    };
    let slot = || match i.v1() {
        0 => "[sp]".to_string(),
        n => format!("[sp + {n}]"),
    };
    let address = || name_of(i.vx()).unwrap_or_else(|| format!("${:#06x}", i.vx()));
    let value = || format!("{:#04x}", i.v1());

    let operands: Vec<String> = match mode {
        AddrMode::None => vec![],
        AddrMode::Reg if form.kinds.len() == 1 => vec![reg(i.r1())],
        AddrMode::Reg => vec![reg(i.r1()), reg(i.r2())],
        AddrMode::RegImm8 => vec![reg(i.r1()), value()],
        AddrMode::RegRam if op == Opcode::Str => vec![address(), reg(i.r1())],
        AddrMode::RegRam => vec![reg(i.r1()), address()],
        AddrMode::RamRegImmReg if op == Opcode::Str => vec![pair_offset(), reg(i.r3())],
        AddrMode::RamRegImmReg if form.kinds.len() == 1 => vec![pair_offset()],
        AddrMode::RamRegImmReg => vec![reg(i.r3()), pair_offset()],
        AddrMode::StackOffset if op == Opcode::Str => vec![slot(), reg(i.r1())],
        AddrMode::StackOffset => vec![reg(i.r1()), slot()],
        AddrMode::Reg16 => vec![pair()],
        // ASP / SSP: an amount, not an address.
        AddrMode::Imm16 if matches!(op, Opcode::Asp | Opcode::Ssp) => vec![i.vx().to_string()],
        AddrMode::Imm16 => vec![address()],
        AddrMode::Imm8 => vec![value()],
    };
    debug_assert_eq!(operands.len(), form.kinds.len());

    if operands.is_empty() {
        op.mnemonic().to_string()
    } else {
        format!("{} {}", op.mnemonic(), operands.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbc2_core::isa::encode;

    #[test]
    fn examples() {
        use Register::*;
        assert_eq!(
            disassemble(encode::reg_imm8(Opcode::Mov, A, 0x2A).0),
            "mov a, 0x2a"
        );
        assert_eq!(
            disassemble(encode::reg_ram(Opcode::Str, B, 0x9000).0),
            "str $0x9000, b"
        );
        assert_eq!(
            disassemble(encode::ramreg_immreg(Opcode::Lod, X, Y, C).0),
            "lod c, [x:y]"
        );
        assert_eq!(
            disassemble(encode::ramreg_offset(Opcode::Str, X, Y, C, 12).0),
            "str [x:y + 12], c"
        );
        assert_eq!(
            disassemble(encode::ramreg_immreg(Opcode::Inc, X, Y, A).0),
            "inc [x:y]"
        );
        assert_eq!(
            disassemble(encode::stack_offset(Opcode::Add, B, 3).0),
            "add b, [sp + 3]"
        );
        assert_eq!(
            disassemble(encode::stack_offset(Opcode::Str, B, 0).0),
            "str [sp], b"
        );
        assert_eq!(disassemble(encode::imm16(Opcode::Ssp, 300).0), "ssp 300");
        assert_eq!(disassemble(encode::none(Opcode::Ret).0), "ret");
        assert_eq!(disassemble(0xFFFF_FFFF), "??? (0xffffffff)");
        let named = disassemble_with(encode::imm16(Opcode::Jmp, 0x0300).0, |a| {
            (a == 0x0300).then(|| "_start".to_string())
        });
        assert_eq!(named, "jmp _start");
    }
}
