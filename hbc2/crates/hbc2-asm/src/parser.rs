//! Turns the tokens of one line into statements: labels, instructions and `.data`.

use hbc2_core::{Opcode, Register};

use crate::diagnostic::{Loc, Report};
use crate::lexer::{Tok, Token};

/// An 8-bit value, resolved once addresses are known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value8 {
    Number(i64),
    /// High byte of a label or variable address (`name.msb`).
    Msb(String),
    /// Low byte of a label or variable address (`name.lsb`).
    Lsb(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Address {
    /// `$0x9000`
    Absolute(u32),
    /// `name`, `name + 3`, `name - 4`
    Symbol { name: String, offset: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Operand {
    Reg(Register),
    /// `[x:y]` or `[x:y + n]`: x = most significant byte, n = offset (0-255,
    /// a character, or `name.msb` / `name.lsb`).
    Pair(Register, Register, Value8),
    /// `[sp]` or `[sp + n]`: the byte at the stack pointer + n (same offsets).
    Stack(Value8),
    Imm(Value8),
    /// A number from 256 to 65535 (`asp` / `ssp` only).
    Wide(u16),
    Mem(Address),
}

#[derive(Clone, Debug)]
pub(crate) struct Op {
    pub operand: Operand,
    pub col: u32,
}

#[derive(Clone, Debug)]
pub(crate) enum DataValue {
    Bytes(Vec<u8>),
    Byte(Value8, u32),
    /// The address of a label or variable on `size` bytes, most significant
    /// first (`.data16 v handler`): (address, size, column).
    Address(Address, u32, u32),
}

impl DataValue {
    pub fn size(&self) -> u32 {
        match self {
            DataValue::Bytes(b) => b.len() as u32,
            DataValue::Byte(..) => 1,
            DataValue::Address(_, size, _) => *size,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Stmt {
    Label {
        name: String,
        address: Option<u32>,
    },
    Instr {
        opcode: Opcode,
        operands: Vec<Op>,
    },
    Data {
        name: String,
        address: Option<u32>,
        values: Vec<DataValue>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Located {
    pub loc: Loc,
    pub stmt: Stmt,
}

/// Registers are case-insensitive.
pub(crate) fn register(name: &str) -> Option<Register> {
    Register::ALL
        .into_iter()
        .find(|r| r.name().eq_ignore_ascii_case(name))
}

/// Mnemonics and register names cannot be used as names.
pub(crate) fn is_reserved(name: &str) -> bool {
    register(name).is_some() || Opcode::from_mnemonic(name).is_some()
}

type ParseResult<T> = Result<T, (u32, String)>;

struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<&'a Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn next(&mut self) -> Option<&'a Tok> {
        let tok = self.peek();
        self.pos += 1;
        tok
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Column of the current token (or just after the last one).
    fn col(&self) -> u32 {
        match self.tokens.get(self.pos) {
            Some(t) => t.col,
            None => self.tokens.last().map_or(1, |t| t.col + 1),
        }
    }

    fn error<T>(&self, message: impl Into<String>) -> ParseResult<T> {
        Err((self.col(), message.into()))
    }

    fn found(&self) -> String {
        self.peek().map_or("end of line".into(), Tok::describe)
    }

    fn expect(&mut self, expected: Tok, what: &str) -> ParseResult<()> {
        if self.peek() == Some(&expected) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected {what}, found {}", self.found()))
        }
    }
}

pub(crate) fn parse_line(
    file: usize,
    line: u32,
    tokens: &[Token],
    report: &mut Report,
) -> Vec<Located> {
    let mut out = Vec::new();
    let mut cursor = Cursor { tokens, pos: 0 };
    if let Err((col, message)) = statement(&mut cursor, file, line, &mut out) {
        report.error(Loc::new(file, line, col), message);
    }
    out
}

fn statement(c: &mut Cursor, file: usize, line: u32, out: &mut Vec<Located>) -> ParseResult<()> {
    let loc = Loc::new(file, line, c.col());
    match c.peek() {
        Some(Tok::Directive(d)) if data_width(d).is_some() => {
            let width = data_width(d).unwrap();
            c.next();
            out.push(Located {
                loc,
                stmt: data(c, width)?,
            });
            Ok(())
        }
        Some(Tok::Directive(d)) => c.error(format!("unknown directive '.{d}'")),
        Some(Tok::Ident(name)) if c.tokens.get(1).map(|t| &t.tok) == Some(&Tok::Colon) => {
            let name = definition_name(c, name)?;
            c.next(); // ':'
            let address = if c.peek() == Some(&Tok::Dollar) {
                Some(absolute(c)?)
            } else {
                None
            };
            out.push(Located {
                loc,
                stmt: Stmt::Label { name, address },
            });
            if c.at_end() {
                Ok(())
            } else if address.is_some() {
                c.error(
                    "nothing can follow a label's address; put the instruction on the next line",
                )
            } else {
                let loc = Loc::new(file, line, c.col());
                out.extend(
                    instruction(c)?
                        .into_iter()
                        .map(|stmt| Located { loc, stmt }),
                );
                Ok(())
            }
        }
        Some(Tok::Ident(_)) => {
            out.extend(
                instruction(c)?
                    .into_iter()
                    .map(|stmt| Located { loc, stmt }),
            );
            Ok(())
        }
        _ => c.error(format!(
            "expected an instruction, a label or a directive, found {}",
            c.found()
        )),
    }
}

/// Validates a label / variable name and moves past it.
fn definition_name(c: &mut Cursor, name: &str) -> ParseResult<String> {
    if name.contains('.') {
        return c.error(format!("'{name}': names cannot contain '.'"));
    }
    if is_reserved(name) {
        return c.error(format!("'{name}' is a reserved word"));
    }
    c.next();
    Ok(name.to_string())
}

/// One instruction, or several for a register list (`psh a, b` / `pop a, b`).
fn instruction(c: &mut Cursor) -> ParseResult<Vec<Stmt>> {
    let Some(Tok::Ident(mnemonic)) = c.peek() else {
        return c.error(format!("expected an instruction, found {}", c.found()));
    };
    let Some(opcode) = Opcode::from_mnemonic(mnemonic) else {
        return c.error(format!("unknown instruction '{mnemonic}'"));
    };
    c.next();
    let wide = matches!(opcode, Opcode::Asp | Opcode::Ssp);
    let mut operands = Vec::new();
    while !c.at_end() {
        let col = c.col();
        operands.push(Op {
            operand: operand(c, wide)?,
            col,
        });
        if !c.at_end() {
            c.expect(Tok::Comma, "',' between operands")?;
            if c.at_end() {
                return c.error("expected an operand after ','");
            }
        }
    }
    if matches!(opcode, Opcode::Psh | Opcode::Pop) && operands.len() > 1 {
        return register_list(opcode, operands);
    }
    Ok(vec![Stmt::Instr { opcode, operands }])
}

/// `psh a, b, c` pushes a, b, then c. `pop a, b, c` pops c, b, then a, so
/// the same list restores what `psh` saved.
fn register_list(opcode: Opcode, operands: Vec<Op>) -> ParseResult<Vec<Stmt>> {
    let mnemonic = opcode.mnemonic();
    let mut seen: Vec<Register> = Vec::new();
    for op in &operands {
        let Operand::Reg(r) = op.operand else {
            return Err((
                op.col,
                format!("{mnemonic} lists only take registers: {mnemonic} a, b, …"),
            ));
        };
        if seen.contains(&r) {
            return Err((op.col, format!("register '{}' is listed twice", r.name())));
        }
        seen.push(r);
    }
    let mut stmts: Vec<Stmt> = operands
        .into_iter()
        .map(|op| Stmt::Instr {
            opcode,
            operands: vec![op],
        })
        .collect();
    if opcode == Opcode::Pop {
        stmts.reverse();
    }
    Ok(stmts)
}

/// `wide`: numbers up to 65535 are allowed (`asp` / `ssp`).
fn operand(c: &mut Cursor, wide: bool) -> ParseResult<Operand> {
    match c.peek() {
        Some(Tok::LBracket) => {
            c.next();
            if let Some(Tok::Ident(name)) = c.peek()
                && name.eq_ignore_ascii_case("sp")
            {
                c.next();
                let offset = bracket_offset(c, "[sp + 2]")?;
                return Ok(Operand::Stack(offset));
            }
            if let Some(Tok::Ident(name)) = c.peek()
                && name.len() == 2
                && name.chars().all(|ch| register(&ch.to_string()).is_some())
            {
                let (high, low) = name.split_at(1);
                return c.error(format!("write register pairs with a colon: [{high}:{low}]"));
            }
            let high = expect_register(c)?;
            c.expect(Tok::Colon, "':' between the two registers, as in [x:y]")?;
            let low = expect_register(c)?;
            let offset = bracket_offset(c, "[x:y + 2]")?;
            Ok(Operand::Pair(high, low, offset))
        }
        Some(Tok::Number(_) | Tok::Minus) if wide => {
            let col = c.col();
            let n = if c.peek() == Some(&Tok::Minus) {
                c.next();
                -number(c)?
            } else {
                number(c)?
            };
            match n {
                0..=255 => Ok(Operand::Imm(Value8::Number(n))),
                256..=0xFFFF => Ok(Operand::Wide(n as u16)),
                _ if n < 0 => Err((
                    col,
                    "the amount is 0 to 65535 (asp moves the stack pointer up, ssp down)".into(),
                )),
                _ => Err((col, format!("{n} does not fit in 16 bits (0 to 65535)"))),
            }
        }
        Some(Tok::Dollar) => Ok(Operand::Mem(Address::Absolute(absolute(c)?))),
        Some(Tok::Ident(name)) => {
            if let Some(r) = register(name) {
                c.next();
                return Ok(Operand::Reg(r));
            }
            if let Some(value) = byte_of(name) {
                c.next();
                return Ok(Operand::Imm(value));
            }
            let name = reference_name(c, name)?;
            let offset = match c.peek() {
                Some(Tok::Plus) => {
                    c.next();
                    number(c)?
                }
                Some(Tok::Minus) => {
                    c.next();
                    -number(c)?
                }
                _ => 0,
            };
            Ok(Operand::Mem(Address::Symbol { name, offset }))
        }
        Some(Tok::Number(_) | Tok::Minus | Tok::Char(_)) => Ok(Operand::Imm(value8(c)?)),
        Some(Tok::Str(_)) => c.error("strings are only allowed in .data"),
        _ => c.error(format!("expected an operand, found {}", c.found())),
    }
}

/// The end of a bracket operand: `]`, or `+ n]` with n from 0 to 255, a
/// character, or `name.msb` / `name.lsb`.
fn bracket_offset(c: &mut Cursor, example: &str) -> ParseResult<Value8> {
    let offset = match c.peek() {
        Some(Tok::Plus) => {
            c.next();
            let col = c.col();
            match c.peek() {
                Some(Tok::Number(_)) => {
                    let n = number(c)?;
                    if !(0..=255).contains(&n) {
                        return Err((col, format!("offset {n} is out of range (0 to 255)")));
                    }
                    Value8::Number(n)
                }
                Some(Tok::Char(_) | Tok::Ident(_)) => value8(c)?,
                _ => {
                    return c.error(format!(
                        "expected an offset (0 to 255, or name.lsb / name.msb), found {}",
                        c.found()
                    ));
                }
            }
        }
        Some(Tok::Minus) => {
            return c.error(format!("offsets are added, from 0 to 255: {example}"));
        }
        _ => Value8::Number(0),
    };
    c.expect(Tok::RBracket, "']'")?;
    Ok(offset)
}

/// `name.msb` / `name.lsb`
fn byte_of(name: &str) -> Option<Value8> {
    let (base, suffix) = name.rsplit_once('.')?;
    if base.is_empty() || base.contains('.') {
        return None;
    }
    match suffix.to_ascii_lowercase().as_str() {
        "msb" => Some(Value8::Msb(base.to_string())),
        "lsb" => Some(Value8::Lsb(base.to_string())),
        _ => None,
    }
}

/// A name used as an operand (label or variable).
fn reference_name(c: &mut Cursor, name: &str) -> ParseResult<String> {
    if name.contains('.') {
        return c.error(format!(
            "'{name}': only '.msb' and '.lsb' can follow a name"
        ));
    }
    if Opcode::from_mnemonic(name).is_some() {
        return c.error(format!("'{name}' is an instruction; missing line break?"));
    }
    c.next();
    Ok(name.to_string())
}

fn expect_register(c: &mut Cursor) -> ParseResult<Register> {
    match c.peek() {
        Some(Tok::Ident(name)) if register(name).is_some() => {
            c.next();
            Ok(register(name).unwrap())
        }
        _ => c.error(format!(
            "expected a register (a b c d i j x y), found {}",
            c.found()
        )),
    }
}

fn number(c: &mut Cursor) -> ParseResult<i64> {
    match c.peek() {
        Some(Tok::Number(n)) => {
            c.next();
            Ok(*n)
        }
        _ => c.error(format!("expected a number, found {}", c.found())),
    }
}

/// `$` followed by a number: an absolute address (up to 20 bits for the EEPROM).
fn absolute(c: &mut Cursor) -> ParseResult<u32> {
    c.expect(Tok::Dollar, "'$'")?;
    let col = c.col();
    let n = number(c)?;
    if !(0..=0xF_FFFF).contains(&n) {
        return Err((
            col,
            format!("address {n:#x} is out of range (max $0xFFFFF)"),
        ));
    }
    Ok(n as u32)
}

/// An 8-bit value: number (-128 to 255), character, or name.msb / name.lsb.
fn value8(c: &mut Cursor) -> ParseResult<Value8> {
    let col = c.col();
    match c.peek() {
        Some(Tok::Char(ch)) => {
            c.next();
            Ok(Value8::Number(*ch as i64))
        }
        Some(Tok::Ident(name)) => match byte_of(name) {
            Some(value) => {
                c.next();
                Ok(value)
            }
            None => c.error(format!(
                "expected a number, a character or name.msb / name.lsb, found '{name}'"
            )),
        },
        _ => {
            let negative = c.peek() == Some(&Tok::Minus);
            if negative {
                c.next();
            }
            let n = number(c)?;
            let n = if negative { -n } else { n };
            if !(-128..=255).contains(&n) {
                return Err((col, format!("{n} does not fit in 8 bits (-128 to 255)")));
            }
            Ok(Value8::Number(n))
        }
    }
}

/// Bytes per value of `.data`, `.data16` and `.data32`.
fn data_width(directive: &str) -> Option<u32> {
    match directive {
        "data" => Some(1),
        "data16" => Some(2),
        "data32" => Some(4),
        _ => None,
    }
}

/// `.data name [$address] value, value, …` (`width` = 1), or `.data16` /
/// `.data32`, whose values take 2 / 4 bytes each, most significant first.
fn data(c: &mut Cursor, width: u32) -> ParseResult<Stmt> {
    let usage = match width {
        1 => ".data expects a name, an optional $address, then values: .data name 1, 2, \"text\"",
        2 => ".data16 expects a name, an optional $address, then values: .data16 name 1000, label",
        _ => {
            ".data32 expects a name, an optional $address, then values: .data32 name 100000, label"
        }
    };
    let Some(Tok::Ident(name)) = c.peek() else {
        return c.error(usage);
    };
    let name = definition_name(c, name)?;
    let address = if c.peek() == Some(&Tok::Dollar) {
        Some(absolute(c)?)
    } else {
        None
    };
    if c.at_end() {
        return c.error(usage);
    }
    let mut values = Vec::new();
    loop {
        let col = c.col();
        match c.peek() {
            Some(Tok::Str(bytes)) if width == 1 => {
                c.next();
                values.push(DataValue::Bytes(bytes.clone()));
            }
            _ if width == 1 => values.push(DataValue::Byte(value8(c)?, col)),
            _ => values.push(wide_value(c, width)?),
        }
        if c.at_end() {
            break;
        }
        c.expect(Tok::Comma, "',' between values")?;
    }
    Ok(Stmt::Data {
        name,
        address,
        values,
    })
}

/// A `.data16` / `.data32` value: a number, a character, `$address`, or a
/// label or variable (its address; `name + n` allowed).
fn wide_value(c: &mut Cursor, width: u32) -> ParseResult<DataValue> {
    let col = c.col();
    let bits = width * 8;
    let number = match c.peek() {
        Some(Tok::Char(ch)) => {
            c.next();
            *ch as i64
        }
        Some(Tok::Dollar) => {
            c.next();
            number(c)?
        }
        Some(Tok::Minus) => {
            c.next();
            -number(c)?
        }
        Some(Tok::Number(_)) => number(c)?,
        Some(Tok::Str(_)) => return c.error("strings are only allowed in .data"),
        Some(Tok::Ident(name)) => {
            if byte_of(name).is_some() {
                return c.error(format!(
                    "'{name}' is one byte: use .data for name.msb / name.lsb"
                ));
            }
            let name = reference_name(c, name)?;
            let offset = match c.peek() {
                Some(Tok::Plus) => {
                    c.next();
                    number(c)?
                }
                Some(Tok::Minus) => {
                    c.next();
                    -number(c)?
                }
                _ => 0,
            };
            return Ok(DataValue::Address(
                Address::Symbol { name, offset },
                width,
                col,
            ));
        }
        _ => return c.error(format!("expected a value, found {}", c.found())),
    };
    let min = -(1i64 << (bits - 1));
    let max = (1i64 << bits) - 1;
    if !(min..=max).contains(&number) {
        return Err((
            col,
            format!("{number} does not fit in {bits} bits ({min} to {max})"),
        ));
    }
    // Negative numbers in two's complement; most significant byte first.
    let bytes = (number as u64).to_be_bytes()[8 - width as usize..].to_vec();
    Ok(DataValue::Bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(line: &str) -> Result<Vec<Stmt>, String> {
        let mut report = Report::default();
        let stmts = parse_line(0, 1, &lex_line(line).unwrap(), &mut report);
        match report.entries.first() {
            Some((_, _, message)) => Err(message.clone()),
            None => Ok(stmts.into_iter().map(|s| s.stmt).collect()),
        }
    }

    fn operands(line: &str) -> Vec<Operand> {
        match parse(line).unwrap().pop().unwrap() {
            Stmt::Instr { operands, .. } => operands.into_iter().map(|o| o.operand).collect(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn operand_kinds() {
        use Register::*;
        assert_eq!(
            operands("MOV A, 'z'"),
            [Operand::Reg(A), Operand::Imm(Value8::Number(b'z' as i64))]
        );
        assert_eq!(
            operands("str [x:y], b"),
            [Operand::Pair(X, Y, Value8::Number(0)), Operand::Reg(B)]
        );
        assert_eq!(
            operands("lod a, [x:y + 0x10]"),
            [Operand::Reg(A), Operand::Pair(X, Y, Value8::Number(0x10))]
        );
        assert_eq!(
            operands("add a, [SP + 3]"),
            [Operand::Reg(A), Operand::Stack(Value8::Number(3))]
        );
        assert_eq!(
            operands("str [sp], a"),
            [Operand::Stack(Value8::Number(0)), Operand::Reg(A)]
        );
        assert_eq!(operands("ssp 300"), [Operand::Wide(300)]);
        assert_eq!(operands("asp 2"), [Operand::Imm(Value8::Number(2))]);
        assert_eq!(
            operands("lod a, $0x9000"),
            [Operand::Reg(A), Operand::Mem(Address::Absolute(0x9000))]
        );
        assert_eq!(
            operands("lod a, buffer - 2"),
            [
                Operand::Reg(A),
                Operand::Mem(Address::Symbol {
                    name: "buffer".into(),
                    offset: -2
                })
            ]
        );
        assert_eq!(
            operands("mov x, msg.MSB"),
            [Operand::Reg(X), Operand::Imm(Value8::Msb("msg".into()))]
        );
        assert_eq!(
            operands("add a, -1"),
            [Operand::Reg(A), Operand::Imm(Value8::Number(-1))]
        );
    }

    #[test]
    fn labels() {
        assert!(matches!(
            &parse("loop:").unwrap()[..],
            [Stmt::Label { address: None, .. }]
        ));
        assert!(matches!(
            &parse("h: $0xF000").unwrap()[..],
            [Stmt::Label {
                address: Some(0xF000),
                ..
            }]
        ));
        assert!(matches!(
            &parse("loop: inc a").unwrap()[..],
            [Stmt::Label { .. }, Stmt::Instr { .. }]
        ));
        assert!(parse("h: $0xF000 nop").is_err());
        assert!(parse("mov: nop").is_err());
    }

    #[test]
    fn data() {
        let stmts = parse(".data msg $0x0100 \"Hi\", 0, v.lsb").unwrap();
        let Stmt::Data {
            name,
            address,
            values,
        } = &stmts[0]
        else {
            panic!()
        };
        assert_eq!((name.as_str(), *address), ("msg", Some(0x0100)));
        assert_eq!(values.iter().map(DataValue::size).sum::<u32>(), 4);
        assert!(parse(".data x").is_err());
        assert!(parse(".data x 256").is_err());
    }

    /// (mnemonic, register) of each statement.
    fn expanded(line: &str) -> Vec<(&'static str, Register)> {
        parse(line)
            .unwrap()
            .into_iter()
            .map(|s| match s {
                Stmt::Instr { opcode, operands } => match operands[..] {
                    [
                        Op {
                            operand: Operand::Reg(r),
                            ..
                        },
                    ] => (opcode.mnemonic(), r),
                    _ => panic!("{operands:?}"),
                },
                other => panic!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn register_lists() {
        use Register::*;
        assert_eq!(
            expanded("psh a, i, y"),
            [("psh", A), ("psh", I), ("psh", Y)]
        );
        assert_eq!(
            expanded("POP a, i, y"),
            [("pop", Y), ("pop", I), ("pop", A)]
        );
        assert_eq!(expanded("pop b"), [("pop", B)]);
        assert_eq!(parse("loop: psh a, b").unwrap().len(), 3); // label + 2
        assert!(parse("psh a, a").unwrap_err().contains("twice"));
        assert!(
            parse("pop a, [x:y]")
                .unwrap_err()
                .contains("only take registers")
        );
        assert!(
            parse("psh a, 5")
                .unwrap_err()
                .contains("only take registers")
        );
        assert!(parse("psh a,").unwrap_err().contains("after ','"));
    }

    /// Bytes of a `.data16` / `.data32` line made only of numbers.
    fn data_bytes(line: &str) -> Vec<u8> {
        let Stmt::Data { values, .. } = parse(line).unwrap().remove(0) else {
            panic!()
        };
        values
            .into_iter()
            .flat_map(|v| match v {
                DataValue::Bytes(b) => b,
                other => panic!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn wide_data() {
        assert_eq!(data_bytes(".data16 v 0x1234, 1"), [0x12, 0x34, 0, 1]);
        assert_eq!(data_bytes(".DATA32 v 100000"), [0x00, 0x01, 0x86, 0xA0]);
        assert_eq!(data_bytes(".data32 v -1"), [0xFF; 4]);
        assert_eq!(
            data_bytes(".data16 v -32768, 'A'"),
            [0x80, 0x00, 0x00, 0x41]
        );
        assert_eq!(data_bytes(".data16 v $0x0100 $0x9000"), [0x90, 0x00]);
        let Stmt::Data { values, .. } = parse(".data16 v handler + 4").unwrap().remove(0) else {
            panic!()
        };
        assert!(matches!(
            &values[..],
            [DataValue::Address(Address::Symbol { name, offset: 4 }, 2, _)] if name == "handler"
        ));
        assert!(parse(".data16 v 65536").unwrap_err().contains("16 bits"));
        assert!(parse(".data16 v -32769").unwrap_err().contains("16 bits"));
        assert!(
            parse(".data32 v 0x1_0000_0000")
                .unwrap_err()
                .contains("32 bits")
        );
        assert!(parse(".data16 v \"hi\"").unwrap_err().contains("strings"));
        assert!(parse(".data16 v h.msb").unwrap_err().contains("one byte"));
        assert!(parse(".data16 v").is_err());
    }

    #[test]
    fn errors() {
        assert_eq!(parse("foo a").unwrap_err(), "unknown instruction 'foo'");
        assert!(parse("mov a b").unwrap_err().contains("','"));
        assert!(parse("mov a, 300").unwrap_err().contains("8 bits"));
        assert!(parse("str [xy], b").unwrap_err().contains("[x:y]"));
        assert!(
            parse("lod a, [x:y + 256]")
                .unwrap_err()
                .contains("0 to 255")
        );
        assert!(parse("lod a, [x:y - 1]").unwrap_err().contains("added"));
        assert!(parse("lod a, [sp - 1]").unwrap_err().contains("[sp + 2]"));
        assert!(parse("lod a, [sp + 1").unwrap_err().contains("']'"));
        assert!(parse("ssp -1").unwrap_err().contains("asp"));
        assert!(parse("asp 65536").unwrap_err().contains("16 bits"));
        assert!(parse("mov a, 300").unwrap_err().contains("8 bits"));
        assert!(parse(".org 5").unwrap_err().contains("unknown directive"));
    }
}
