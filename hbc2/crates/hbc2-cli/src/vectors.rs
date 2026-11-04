//! `hbc2 vectors`: Logisim test vectors for the CPU circuits.
//!
//! The expected outputs come from the emulator's own code (`hbc2_core::alu`),
//! so a circuit that passes behaves exactly like the emulator. The vectors
//! cover every input combination; a module with more than 2^20 is refused.

use std::io::{BufWriter, Write};

use hbc2_core::alu;

/// A combinational circuit: its pins (name, width) and its behaviour.
struct Module {
    name: &'static str,
    inputs: &'static [(&'static str, u32)],
    outputs: &'static [(&'static str, u32)],
    /// Input values, in `inputs` order -> output values, in `outputs` order.
    eval: fn(&[u32]) -> Vec<u32>,
}

const MAX_BITS: u32 = 20;

const MODULES: &[Module] = &[
    Module {
        name: "add_sub8",
        inputs: &[
            ("add_sub8_sub", 1),
            ("add_sub8_usec", 1),
            ("add_sub8_cin", 1),
            ("add_sub8_a", 8),
            ("add_sub8_b", 8),
        ],
        outputs: &[("add_sub8_s", 8), ("add_sub8_c", 1)],
        // ADD/ADC when sub = 0, SUB/SBB when sub = 1; usec selects ADC/SBB,
        // which use the carry flag (cin).
        eval: |v| {
            let (sub, usec, cin) = (v[0] != 0, v[1] != 0, v[2] != 0);
            let (a, b) = (v[3] as u8, v[4] as u8);
            let (s, c) = if sub {
                alu::sub(a, b, usec && cin)
            } else {
                alu::add(a, b, usec && cin)
            };
            vec![s as u32, c as u32]
        },
    },
    Module {
        name: "alu",
        inputs: &[("alu_a", 8), ("alu_b", 8), ("alu_cin", 1)],
        outputs: &[("alu_s", 8), ("alu_c", 1)],
        // For now the ALU only adds (ADD / ADC).
        eval: |v| {
            let (s, c) = alu::add(v[0] as u8, v[1] as u8, v[2] != 0);
            vec![s as u32, c as u32]
        },
    },
    Module {
        name: "add8",
        inputs: &[("add8_a", 8), ("add8_b", 8), ("add8_cin", 1)],
        outputs: &[("add8_s", 8), ("add8_cout", 1)],
        eval: |v| {
            let (s, c) = alu::add(v[0] as u8, v[1] as u8, v[2] != 0);
            vec![s as u32, c as u32]
        },
    },
];

fn names() -> String {
    MODULES
        .iter()
        .map(|m| m.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// `hbc2 vectors <module> [-o file]`, or `hbc2 vectors --list`.
pub fn vectors(args: &[String]) -> Result<(), String> {
    let Some(name) = args.first() else {
        return Err(format!(
            "usage: hbc2 vectors <module> [-o file]\nmodules: {}",
            names()
        ));
    };
    if name == "--list" {
        for m in MODULES {
            println!("{}", m.name);
        }
        return Ok(());
    }
    let module = MODULES
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("no module {name:?}; modules: {}", names()))?;
    let out = crate::option(args, "-o");
    match out {
        Some(path) => {
            let file = std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?;
            write(module, &mut BufWriter::new(file))
        }
        None => write(module, &mut BufWriter::new(std::io::stdout().lock())),
    }
    .map_err(|e| format!("writing vectors: {e}"))
}

fn write(m: &Module, w: &mut impl Write) -> std::io::Result<()> {
    let bits: u32 = m.inputs.iter().map(|&(_, width)| width).sum();
    if bits > MAX_BITS {
        return Err(std::io::Error::other(format!(
            "{}: {bits} input bits, more than the {MAX_BITS} an exhaustive test allows",
            m.name
        )));
    }
    let header: Vec<String> = m
        .inputs
        .iter()
        .chain(m.outputs)
        .map(|&(n, width)| {
            if width > 1 {
                format!("{n}[{width}]")
            } else {
                n.to_string()
            }
        })
        .collect();
    writeln!(w, "{}", header.join(" "))?;

    let mut values = vec![0u32; m.inputs.len()];
    for row in 0u32..(1 << bits) {
        // first input = most significant bits of the row counter
        let mut rest = row;
        for (i, &(_, width)) in m.inputs.iter().enumerate().rev() {
            values[i] = rest & ((1 << width) - 1);
            rest >>= width;
        }
        let outputs = (m.eval)(&values);
        let cells = m
            .inputs
            .iter()
            .zip(&values)
            .chain(m.outputs.iter().zip(&outputs));
        let mut first = true;
        for (&(_, width), &v) in cells {
            if !first {
                w.write_all(b" ")?;
            }
            first = false;
            write!(w, "{:0width$b}", v, width = width as usize)?;
        }
        w.write_all(b"\n")?;
    }
    w.flush()
}
