//! Parser of the microcode source language (microcode/hbc2.uc).
//!
//! A file is a list of `macro`, `param`, `seq` and `instr` statements. A
//! block (`seq` or `instr`) holds one step per line, each a comma-separated
//! list of actions and at most one exit. The header comment of hbc2.uc lists
//! every action.

use std::collections::HashMap;

/// Source of a data bus value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusSrc {
    Ram,
    A,
    Res,
    Flags,
    PcHi,
    PcLo,
    SpHi,
    SpLo,
    /// The IOD, in an IN step.
    Io,
}

/// Port A index (also the write index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortA {
    R1,
    R2,
    R3,
    /// A fixed index from the control unit (0-7; I = 4).
    Cu(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortB {
    R2,
    R1,
    R3,
    One,
    Zero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrByte {
    B0,
    B1,
    V1,
    V2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagsOp {
    Czn,
    Zn,
    Sef,
    Bus,
    /// Flag number (C0 E1 I2 N3 S4 Z5 F6 H7) and value.
    Single(u8, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Lat,
    Pc,
    Sp,
}

/// The adder's sum (addr-decisions.md section 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expr {
    Pc,
    PcPlus1,
    PcMinus1,
    Sp,
    SpPlus1,
    SpMinus1,
    SpPlusV1,
    SpMinusV1,
    SpPlusVx,
    SpMinusVx,
    /// bus A : bus B (R1:R2).
    Ab,
    AbPlusV2,
    /// 00 : bus B (I/O port).
    ZeroB,
    Vx,
    Ivt,
    /// The IVT's high byte only (the reset word).
    IvtHi,
    Zero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cond {
    Jc,
    NoInt,
    Hwq,
    Xn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    Next(String),
    If(Cond, String, String),
    End,
    EndNoCheck,
    EndHalt,
    Dispatch,
    Restart,
}

/// One step, as written.
#[derive(Clone, Debug, Default)]
pub struct Step {
    pub bus: Vec<BusSrc>,
    pub ram_write: bool,
    pub ram_copy: bool,
    pub io_out: bool,
    pub loads: Vec<IrByte>,
    pub swi: bool,
    pub a: Option<PortA>,
    pub b: Option<PortB>,
    pub write: Option<PortA>,
    pub alu: Option<u8>,
    pub flags: Vec<FlagsOp>,
    pub addr: Vec<(Target, Expr)>,
    pub hwvec: bool,
    pub swvec: bool,
    pub inr: bool,
    pub exit: Option<Exit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Seq(String),
    Instr {
        ops: Vec<String>,
        modes: Vec<String>,
    },
}

#[derive(Clone, Debug)]
pub struct SrcLine {
    pub label: Option<String>,
    pub text: String,
    /// 1-based line number in the file.
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub kind: BlockKind,
    /// No line: the block's only line is `-` (nothing after F0).
    pub lines: Vec<SrcLine>,
    pub line: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Source {
    pub macros: HashMap<String, String>,
    pub params: HashMap<String, HashMap<String, String>>,
    pub blocks: Vec<Block>,
}

/// A parse error, with its line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, Error> {
    Err(Error {
        line,
        message: message.into(),
    })
}

fn is_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Splits the file into statements and blocks. Steps are parsed later, per
/// opcode (parameters differ).
pub fn parse(text: &str) -> Result<Source, Error> {
    let mut src = Source::default();
    let mut current: Option<Block> = None;
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let code = raw.split('#').next().unwrap_or("");
        if code.trim().is_empty() {
            continue;
        }
        let indented = code.starts_with(' ') || code.starts_with('\t');
        let code = code.trim();
        let first = code.split_whitespace().next().unwrap_or("");
        let is_statement = !indented && matches!(first, "macro" | "param" | "seq" | "instr");
        if !is_statement {
            let Some(block) = current.as_mut() else {
                return err(line, "a step outside a block");
            };
            // A label is a name followed by ':' and a blank ("a:b" in an
            // expression has no blank after its colon).
            let (label, body) = match code.split_once(':') {
                Some((l, b)) if is_name(l) && (b.is_empty() || b.starts_with([' ', '\t'])) => {
                    (Some(l.to_string()), b.trim())
                }
                _ => (None, code),
            };
            if body == "-" {
                if !block.lines.is_empty() || label.is_some() {
                    return err(line, "`-` must be a block's only line");
                }
                continue;
            }
            block.lines.push(SrcLine {
                label,
                text: body.to_string(),
                line,
            });
            continue;
        }
        if let Some(block) = current.take() {
            src.blocks.push(block);
        }
        let rest = code[first.len()..].trim();
        match first {
            "macro" => {
                let Some((name, body)) = rest.split_once('=') else {
                    return err(line, "macro NAME = actions");
                };
                let name = name.trim();
                if !is_name(name) || src.macros.contains_key(name) {
                    return err(line, format!("bad or duplicate macro name `{name}`"));
                }
                src.macros.insert(name.to_string(), body.trim().to_string());
            }
            "param" => {
                let mut words = rest.split_whitespace();
                let Some(name) = words.next() else {
                    return err(line, "param NAME op=value ...");
                };
                let mut map = HashMap::new();
                for w in words {
                    let Some((op, v)) = w.split_once('=') else {
                        return err(line, format!("`{w}`: expected op=value"));
                    };
                    map.insert(op.to_string(), v.to_string());
                }
                if src.params.insert(name.to_string(), map).is_some() {
                    return err(line, format!("duplicate param `{name}`"));
                }
            }
            "seq" => {
                if !is_name(rest) {
                    return err(line, "seq NAME");
                }
                current = Some(Block {
                    kind: BlockKind::Seq(rest.to_string()),
                    lines: Vec::new(),
                    line,
                });
            }
            "instr" => {
                let Some((ops, modes)) = rest.split_once(':') else {
                    return err(line, "instr OPS : MODES");
                };
                let ops: Vec<String> = ops.split_whitespace().map(str::to_string).collect();
                let modes: Vec<String> = modes.split_whitespace().map(str::to_string).collect();
                if ops.is_empty() || modes.is_empty() {
                    return err(line, "instr OPS : MODES");
                }
                current = Some(Block {
                    kind: BlockKind::Instr { ops, modes },
                    lines: Vec::new(),
                    line,
                });
            }
            _ => unreachable!(),
        }
    }
    if let Some(block) = current.take() {
        src.blocks.push(block);
    }
    Ok(src)
}

impl Source {
    /// Parses one step for one opcode (`op`: mnemonic for `$param`, or None
    /// in a `seq`).
    pub fn step(&self, l: &SrcLine, op: Option<&str>) -> Result<Step, Error> {
        let text = self.substitute(l, op)?;
        let mut step = Step::default();
        self.items(&text, l.line, &mut step, 0)?;
        Ok(step)
    }

    fn substitute(&self, l: &SrcLine, op: Option<&str>) -> Result<String, Error> {
        let mut out = String::new();
        let mut rest = l.text.as_str();
        while let Some(i) = rest.find('$') {
            out.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            let n = after
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            let name = &after[..n];
            let Some(map) = self.params.get(name) else {
                return err(l.line, format!("unknown param `${name}`"));
            };
            let Some(op) = op else {
                return err(l.line, format!("`${name}` outside an instr block"));
            };
            let Some(v) = map.get(op) else {
                return err(l.line, format!("param `${name}` has no value for `{op}`"));
            };
            out.push_str(v);
            rest = &after[n..];
        }
        out.push_str(rest);
        Ok(out)
    }

    fn items(&self, text: &str, line: usize, s: &mut Step, depth: u32) -> Result<(), Error> {
        if depth > 8 {
            return err(line, "macros nested too deep");
        }
        for item in text.split(',') {
            let item = item.trim();
            if item.is_empty() {
                return err(line, "empty action");
            }
            if let Some(body) = self.macros.get(item) {
                self.items(&body.clone(), line, s, depth + 1)?;
                continue;
            }
            if let Some(exit) = parse_exit(item, line)? {
                if s.exit.is_some() {
                    return err(line, "two exits in one step");
                }
                s.exit = Some(exit);
                continue;
            }
            action(item, line, s)?;
        }
        Ok(())
    }
}

fn parse_exit(item: &str, line: usize) -> Result<Option<Exit>, Error> {
    let w: Vec<&str> = item.split_whitespace().collect();
    Ok(Some(match w.as_slice() {
        ["end"] => Exit::End,
        ["end", "nocheck"] => Exit::EndNoCheck,
        ["end", "halt"] => Exit::EndHalt,
        ["dispatch"] => Exit::Dispatch,
        ["restart"] => Exit::Restart,
        ["next", l] => Exit::Next(l.to_string()),
        ["if", c, l1, "else", l0] => {
            let c = match *c {
                "jc" => Cond::Jc,
                "noint" => Cond::NoInt,
                "hwq" => Cond::Hwq,
                "xn" => Cond::Xn,
                _ => return err(line, format!("unknown condition `{c}`")),
            };
            Exit::If(c, l1.to_string(), l0.to_string())
        }
        ["end", ..] | ["next", ..] | ["if", ..] => return err(line, format!("bad exit `{item}`")),
        _ => return Ok(None),
    }))
}

fn port_a(name: &str) -> Option<PortA> {
    Some(match name {
        "r1" => PortA::R1,
        "r2" => PortA::R2,
        "r3" => PortA::R3,
        "i" => PortA::Cu(4),
        _ => {
            let n: u8 = name.strip_prefix("reg")?.parse().ok()?;
            if n > 7 {
                return None;
            }
            PortA::Cu(n)
        }
    })
}

fn expr(name: &str) -> Option<Expr> {
    Some(match name {
        "pc" => Expr::Pc,
        "pc+1" => Expr::PcPlus1,
        "pc-1" => Expr::PcMinus1,
        "sp" => Expr::Sp,
        "sp+1" => Expr::SpPlus1,
        "sp-1" => Expr::SpMinus1,
        "sp+v1" => Expr::SpPlusV1,
        "sp-v1" => Expr::SpMinusV1,
        "sp+vx" => Expr::SpPlusVx,
        "sp-vx" => Expr::SpMinusVx,
        "a:b" => Expr::Ab,
        "a:b+v2" => Expr::AbPlusV2,
        "0:b" => Expr::ZeroB,
        "vx" => Expr::Vx,
        "ivt" => Expr::Ivt,
        "ivt.hi" => Expr::IvtHi,
        "0" => Expr::Zero,
        _ => return None,
    })
}

pub const ALU_OPS: &[(&str, u8)] = &[
    ("add", 0b0000),
    ("sub", 0b0001),
    ("adc", 0b0010),
    ("sbb", 0b0011),
    ("and", 0b1000),
    ("or", 0b1001),
    ("xor", 0b1010),
    ("not", 0b1011),
    ("shl", 0b1100),
    ("shr", 0b1110),
    ("asr", 0b1111),
];

pub const FLAG_NAMES: [&str; 8] = ["C", "E", "I", "N", "S", "Z", "F", "H"];

fn action(item: &str, line: usize, s: &mut Step) -> Result<(), Error> {
    let bad = || err::<()>(line, format!("unknown action `{item}`"));
    match item {
        "ram<-bus" => s.ram_write = true,
        "ram<-rom" => s.ram_copy = true,
        "io<-bus" => s.io_out = true,
        "ir0<-bus" => s.loads.push(IrByte::B0),
        "ir1<-bus" => s.loads.push(IrByte::B1),
        "v1<-bus" => s.loads.push(IrByte::V1),
        "v2<-bus" => s.loads.push(IrByte::V2),
        "swi<-bus" => s.swi = true,
        "hwvec" => s.hwvec = true,
        "swvec" => s.swvec = true,
        "inr" => s.inr = true,
        _ => {
            if let Some(src) = item.strip_prefix("bus<-") {
                s.bus.push(match src {
                    "ram" => BusSrc::Ram,
                    "a" => BusSrc::A,
                    "res" => BusSrc::Res,
                    "flags" => BusSrc::Flags,
                    "pchi" => BusSrc::PcHi,
                    "pclo" => BusSrc::PcLo,
                    "sphi" => BusSrc::SpHi,
                    "splo" => BusSrc::SpLo,
                    "io" => BusSrc::Io,
                    _ => return bad(),
                });
            } else if let Some(g) = item.strip_prefix("flags<-") {
                s.flags.push(match g {
                    "czn" => FlagsOp::Czn,
                    "zn" => FlagsOp::Zn,
                    "sef" => FlagsOp::Sef,
                    "bus" => FlagsOp::Bus,
                    _ => return bad(),
                });
            } else if let Some(a) = item.strip_prefix("a=") {
                if s.a.is_some() {
                    return err(line, "two a= in one step");
                }
                let Some(p) = port_a(a) else {
                    return bad();
                };
                s.a = Some(p);
            } else if let Some(b) = item.strip_prefix("b=") {
                if s.b.is_some() {
                    return err(line, "two b= in one step");
                }
                s.b = Some(match b {
                    "r1" => PortB::R1,
                    "r2" => PortB::R2,
                    "r3" => PortB::R3,
                    "one" => PortB::One,
                    "zero" => PortB::Zero,
                    _ => return bad(),
                });
            } else if let Some(op) = item.strip_prefix("alu=") {
                if s.alu.is_some() {
                    return err(line, "two alu= in one step");
                }
                let Some(&(_, code)) = ALU_OPS.iter().find(|(n, _)| *n == op) else {
                    return bad();
                };
                s.alu = Some(code);
            } else if let Some((dst, src)) = item.split_once("<-") {
                if let Some(n) = FLAG_NAMES.iter().position(|f| *f == dst) {
                    let v = match src {
                        "0" => false,
                        "1" => true,
                        _ => return bad(),
                    };
                    s.flags.push(FlagsOp::Single(n as u8, v));
                } else if let (Some(t), Some(e)) = (
                    match dst {
                        "lat" => Some(Target::Lat),
                        "pc" => Some(Target::Pc),
                        "sp" => Some(Target::Sp),
                        _ => None,
                    },
                    expr(src),
                ) {
                    s.addr.push((t, e));
                } else if src == "bus" {
                    let Some(p) = port_a(dst) else {
                        return bad();
                    };
                    if s.write.is_some() {
                        return err(line, "two register writes in one step");
                    }
                    s.write = Some(p);
                } else {
                    return bad();
                }
            } else {
                return bad();
            }
        }
    }
    Ok(())
}
