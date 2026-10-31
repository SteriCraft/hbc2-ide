//! Runtime helpers (compiler/runtime) against Rust models: each helper is
//! called directly on the emulator, with the registers of ABI.md section 11,
//! for special and random operands.

use std::sync::OnceLock;

use hbc2_asm::{Program, Target, assemble_file};
use hbc2_cc::runtime_dir;
use hbc2_core::{Machine, MachineConfig, Register, StopReason};

use Register::*;

const RETURN_TO: &str = "test_return";
const VAR: u16 = 0x9000; // variable for the assignment operators

fn program() -> &'static Program {
    static PROGRAM: OnceLock<Program> = OnceLock::new();
    PROGRAM.get_or_init(|| {
        let dir = std::env::temp_dir().join("hbc2-cc-helpers");
        std::fs::create_dir_all(&dir).unwrap();
        let rt = runtime_dir();
        let main = dir.join("main.has");
        std::fs::write(
            &main,
            format!(
                ".include \"{}\"\n.include \"{}\"\n_main:\n    ret\n{RETURN_TO}:\n    jmp {RETURN_TO}\n",
                slashes(&rt.join("crt0.has")),
                slashes(&rt.join("runtime.has"))
            ),
        )
        .unwrap();
        assemble_file(&main, Target::Ram).unwrap_or_else(|e| {
            panic!(
                "{}",
                e.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
            )
        })
    })
}

/// A path for .include: forward slashes (a backslash starts an escape)
fn slashes(path: &std::path::Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn address(name: &str) -> u16 {
    program()
        .symbol(name)
        .unwrap_or_else(|| panic!("no helper {name}"))
        .address as u16
}

/// Calls `helper` with the registers set, after writing `memory`; returns
/// the machine once the helper has returned.
fn call(helper: &str, regs: &[(Register, u8)], memory: &[(u16, u8)]) -> Machine {
    let config = MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    };
    let mut m = Machine::new(config, Some(&program().image)).unwrap();
    for &(r, v) in regs {
        m.cpu.set_reg(r, v);
    }
    for &(a, v) in memory {
        m.board.ram.write(a, v);
    }
    let back = address(RETURN_TO);
    m.board.ram.write(0xFDFF, back as u8); // the return address, as cal pushes it
    m.board.ram.write(0xFDFE, (back >> 8) as u8);
    m.cpu.sp = 0xFDFE;
    m.cpu.pc = address(helper);
    m.breakpoints.set(back, true);
    let result = m.run(5_000_000);
    assert_eq!(
        result.stop,
        Some(StopReason::Breakpoint(back)),
        "{helper} did not return"
    );
    assert_eq!(m.cpu.sp, 0xFE00, "{helper}: stack not balanced");
    m
}

fn r16(m: &Machine, h: Register, l: Register) -> u16 {
    u16::from_be_bytes([m.cpu.reg(h), m.cpu.reg(l)])
}

fn r32(m: &Machine) -> u32 {
    u32::from_be_bytes([m.cpu.reg(C), m.cpu.reg(D), m.cpu.reg(A), m.cpu.reg(B)])
}

fn pair(h: Register, l: Register, v: u16) -> [(Register, u8); 2] {
    [(h, (v >> 8) as u8), (l, v as u8)]
}

/// Left in [c:d], right in [a:b]
fn args16(left: u16, right: u16) -> Vec<(Register, u8)> {
    [pair(C, D, left), pair(A, B, right)].concat()
}

/// Left in [x:y]:[i:j], right in [c:d]:[a:b]
fn args32(left: u32, right: u32) -> Vec<(Register, u8)> {
    [
        pair(X, Y, (left >> 16) as u16),
        pair(I, J, left as u16),
        pair(C, D, (right >> 16) as u16),
        pair(A, B, right as u16),
    ]
    .concat()
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn values16(n: usize) -> Vec<u16> {
    let mut v = vec![
        0, 1, 2, 3, 7, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0x8001, 0xFFFE, 0xFFFF,
    ];
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    while v.len() < n {
        let x = rng.next();
        // small and large values
        v.push(if x & 1 == 0 {
            (x >> 8) as u16
        } else {
            (x >> 8) as u16 & 0x3F
        });
    }
    v
}

fn values32(n: usize) -> Vec<u32> {
    let mut v = vec![
        0,
        1,
        2,
        7,
        0xFF,
        0x100,
        0xFFFF,
        0x10000,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
        100_000,
        0x12_3456,
        0xFF_FFFF,
        0x100_0000,
    ];
    let mut rng = Rng(0x0FED_CBA9_8765_4321);
    while v.len() < n {
        let x = rng.next();
        v.push(match x % 5 {
            0 => (x >> 16) as u32,
            1 => (x >> 16) as u32 & 0xFFFF,
            2 => (x >> 16) as u32 & 0xFF,
            3 => (x >> 16) as u32 & 0xFF_FFFF,
            _ => ((x >> 16) as u32 & 0xFFFF).wrapping_neg(),
        });
    }
    v
}

/// Every pair of the values (n values → n² cases)
fn pairs<T: Copy>(values: &[T]) -> impl Iterator<Item = (T, T)> + '_ {
    values
        .iter()
        .flat_map(move |&l| values.iter().map(move |&r| (l, r)))
}

fn check16(helper: &str, model: impl Fn(u16, u16) -> Option<u16>) {
    for (l, r) in pairs(&values16(40)) {
        let Some(want) = model(l, r) else { continue };
        let m = call(helper, &args16(l, r), &[]);
        assert_eq!(r16(&m, A, B), want, "{helper}({l:#06x}, {r:#06x})");
    }
}

fn check32(helper: &str, n: usize, model: impl Fn(u32, u32) -> Option<u32>) {
    for (l, r) in pairs(&values32(n)) {
        let Some(want) = model(l, r) else { continue };
        let m = call(helper, &args32(l, r), &[]);
        assert_eq!(r32(&m), want, "{helper}({l:#010x}, {r:#010x})");
    }
}

fn check32_bool(helper: &str, model: impl Fn(u32, u32) -> bool) {
    for (l, r) in pairs(&values32(30)) {
        let m = call(helper, &args32(l, r), &[]);
        assert_eq!(
            r16(&m, A, B),
            model(l, r) as u16,
            "{helper}({l:#010x}, {r:#010x})"
        );
    }
}

fn s16(v: u16) -> i16 {
    v as i16
}

fn s32(v: u32) -> i32 {
    v as i32
}

#[test]
fn compare_16() {
    check16("__ccltu", |l, r| Some((l < r) as u16));
    check16("__ccgtu", |l, r| Some((l > r) as u16));
    check16("__ccltequ", |l, r| Some((l <= r) as u16));
    check16("__ccgtequ", |l, r| Some((l >= r) as u16));
    check16("__cclt", |l, r| Some((s16(l) < s16(r)) as u16));
    check16("__ccgt", |l, r| Some((s16(l) > s16(r)) as u16));
    check16("__cclteq", |l, r| Some((s16(l) <= s16(r)) as u16));
    check16("__ccgteq", |l, r| Some((s16(l) >= s16(r)) as u16));
}

#[test]
fn compare_32() {
    check32_bool("__cceql", |l, r| l == r);
    check32_bool("__ccnel", |l, r| l != r);
    check32_bool("__ccltul", |l, r| l < r);
    check32_bool("__ccgtul", |l, r| l > r);
    check32_bool("__ccltequl", |l, r| l <= r);
    check32_bool("__ccgtequl", |l, r| l >= r);
    check32_bool("__ccltl", |l, r| s32(l) < s32(r));
    check32_bool("__ccgtl", |l, r| s32(l) > s32(r));
    check32_bool("__cclteql", |l, r| s32(l) <= s32(r));
    check32_bool("__ccgteql", |l, r| s32(l) >= s32(r));
}

#[test]
fn arithmetic_32() {
    check32("__plusl", 30, |l, r| Some(l.wrapping_add(r)));
    check32("__minusl", 30, |l, r| Some(l.wrapping_sub(r)));
    check32("__bandl", 30, |l, r| Some(l & r));
    check32("__orl", 30, |l, r| Some(l | r));
    check32("__xorl", 30, |l, r| Some(l ^ r));
    for v in values32(200) {
        let m = call("__cpll", &args32(0, v), &[]);
        assert_eq!(r32(&m), !v, "__cpll({v:#x})");
        let m = call("__negatel", &args32(0, v), &[]);
        assert_eq!(r32(&m), v.wrapping_neg(), "__negatel({v:#x})");
    }
}

#[test]
fn multiply() {
    check16("__mul", |l, r| Some(l.wrapping_mul(r)));
    check32("__mull", 30, |l, r| Some(l.wrapping_mul(r)));
}

#[test]
fn divide_16() {
    check16("__divu", |l, r| Some(l.checked_div(r).unwrap_or(0xFFFF)));
    check16("__remu", |l, r| Some(l.checked_rem(r).unwrap_or(l)));
    check16("__div", |l, r| {
        (r != 0).then(|| s16(l).wrapping_div(s16(r)) as u16)
    });
    check16("__rem", |l, r| {
        (r != 0).then(|| s16(l).wrapping_rem(s16(r)) as u16)
    });
}

#[test]
fn divide_32() {
    check32("__divul", 25, |l, r| {
        Some(l.checked_div(r).unwrap_or(0xFFFF_FFFF))
    });
    check32("__remul", 25, |l, r| Some(l.checked_rem(r).unwrap_or(l)));
    check32("__divl", 25, |l, r| {
        (r != 0).then(|| s32(l).wrapping_div(s32(r)) as u32)
    });
    check32("__reml", 25, |l, r| {
        (r != 0).then(|| s32(l).wrapping_rem(s32(r)) as u32)
    });
}

#[test]
fn divide_by_zero() {
    // all bits set, the remainder is the left operand (signed too)
    for l in values16(40) {
        for (helper, want) in [("__divu", 0xFFFF), ("__div", 0xFFFF), ("__remu", l), ("__rem", l)] {
            let m = call(helper, &args16(l, 0), &[]);
            assert_eq!(r16(&m, A, B), want, "{helper}({l:#06x}, 0)");
        }
    }
    for l in values32(40) {
        for (helper, want) in [("__divul", u32::MAX), ("__divl", u32::MAX), ("__remul", l), ("__reml", l)] {
            let m = call(helper, &args32(l, 0), &[]);
            assert_eq!(r32(&m), want, "{helper}({l:#010x}, 0)");
        }
    }
}

/// Many more pairs than the tests above, for the multiply and divide
/// helpers (their paths depend on how many bytes each operand has).
#[test]
fn multiply_divide_many() {
    check16("__mul", |l, r| Some(l.wrapping_mul(r)));
    let v = values32(90);
    for (l, r) in pairs(&v) {
        let cases = [
            ("__mull", l.wrapping_mul(r)),
            ("__divul", l.checked_div(r).unwrap_or(u32::MAX)),
            ("__remul", l.checked_rem(r).unwrap_or(l)),
        ];
        for (helper, want) in cases {
            let m = call(helper, &args32(l, r), &[]);
            assert_eq!(r32(&m), want, "{helper}({l:#010x}, {r:#010x})");
        }
    }
}

#[test]
fn shifts_16() {
    for v in values16(30) {
        for n in 0..20u16 {
            let shl = if n < 16 { v << n } else { 0 };
            let shru = if n < 16 { v >> n } else { 0 };
            let shr = (s16(v) >> n.min(15)) as u16;
            for (helper, want) in [("__shl", shl), ("__shru", shru), ("__shr", shr)] {
                let m = call(helper, &args16(v, n), &[]);
                assert_eq!(r16(&m, A, B), want, "{helper}({v:#06x}, {n})");
            }
        }
        // a count with a high byte is larger than 15
        let m = call("__shl", &args16(v, 0x0101), &[]);
        assert_eq!(r16(&m, A, B), 0, "__shl({v:#06x}, 0x101)");
    }
}

#[test]
fn shifts_32() {
    for v in values32(20) {
        for n in [0u32, 1, 2, 7, 8, 9, 15, 16, 17, 24, 31, 32, 40] {
            let shl = if n < 32 { v << n } else { 0 };
            let shru = if n < 32 { v >> n } else { 0 };
            let shr = (s32(v) >> n.min(31)) as u32;
            for (helper, want) in [("__shll", shl), ("__shrul", shru), ("__shrl", shr)] {
                // the count is the right operand: an int in [a:b]
                let m = call(helper, &args32(v, n), &[]);
                assert_eq!(r32(&m), want, "{helper}({v:#010x}, {n})");
            }
        }
    }
}

/// Assignment operators: the variable at VAR, its address in [c:d] (char,
/// int) or [x:y] (long). Checks the variable and the result.
fn assign_op(helper: &str, size: usize, old: u32, value: u32, new: u32, result: u32) {
    let mut memory = Vec::new();
    for k in 0..size {
        memory.push((VAR + k as u16, (old >> (8 * (size - 1 - k))) as u8));
    }
    let regs: Vec<(Register, u8)> = match size {
        1 => [pair(C, D, VAR).to_vec(), vec![(B, value as u8)]].concat(),
        2 => [pair(C, D, VAR), pair(A, B, value as u16)].concat(),
        _ => args32(u32::from(VAR) << 16, value)
            .into_iter()
            .filter(|(r, _)| *r != I && *r != J)
            .collect(),
    };
    let m = call(helper, &regs, &memory);
    let mut stored = 0u32;
    for k in 0..size {
        stored = stored << 8 | u32::from(m.board.ram.read(VAR + k as u16));
    }
    let got = match size {
        1 => u32::from(m.cpu.reg(B)),
        2 => u32::from(r16(&m, A, B)),
        _ => r32(&m),
    };
    let ctx = format!("{helper}(old {old:#x}, value {value:#x})");
    assert_eq!(stored, new, "{ctx}: variable");
    assert_eq!(got, result, "{ctx}: result");
}

type Op = fn(u32, u32) -> Option<u32>;

#[test]
fn assignment_operators() {
    // (suffix-less name, model on the given width, signed?)
    let ops: [(&str, Op); 10] = [
        ("pluseq", |a, b| Some(a.wrapping_add(b))),
        ("minuseq", |a, b| Some(a.wrapping_sub(b))),
        ("andeq", |a, b| Some(a & b)),
        ("oreq", |a, b| Some(a | b)),
        ("xoreq", |a, b| Some(a ^ b)),
        ("muleq", |a, b| Some(a.wrapping_mul(b))),
        ("divequ", |a, b| a.checked_div(b)),
        ("remequ", |a, b| a.checked_rem(b)),
        ("shleq", |a, b| Some(if b < 32 { a << b } else { 0 })),
        ("shrequ", |a, b| Some(if b < 32 { a >> b } else { 0 })),
    ];
    for (size, suffix, mask) in [(1, "c", 0xFFu32), (2, "", 0xFFFF), (4, "l", 0xFFFF_FFFF)] {
        let bits = size as u32 * 8;
        let values: Vec<u32> = values32(16).into_iter().map(|v| v & mask).collect();
        for &(name, model) in &ops {
            // divequ etc.: unsigned names take "u" before the size suffix
            let helper = match (name.strip_suffix('u'), suffix) {
                (Some(base), "c") => format!("__{base}uc"),
                (Some(base), s) => format!("__{base}u{s}"),
                (None, s) => format!("__{name}{s}"),
            };
            for (old, value) in pairs(&values) {
                let value = if name.starts_with("sh") {
                    value % (bits + 2)
                } else {
                    value
                };
                // shifts of a char: the int shift of the widened value, cut
                let new = if name.starts_with("sh") && size == 1 {
                    let wide = if name == "shleq" {
                        if value < 16 { old << value } else { 0 }
                    } else if value < 16 {
                        old >> value
                    } else {
                        0
                    };
                    Some(wide & mask)
                } else {
                    model(old, value).map(|n| n & mask)
                };
                let Some(new) = new else { continue };
                assign_op(&helper, size, old, value, new, new);
            }
        }
        // signed forms: diveq, remeq, shreq on signed values
        let sext = |v: u32| -> i64 { ((v as i64) << (64 - bits)) >> (64 - bits) };
        for (old, value) in pairs(&values) {
            let (o, v) = (sext(old), sext(value));
            if v != 0 && !(o == -(1i64 << (bits - 1)) && v == -1) {
                assign_op(
                    &format!("__diveq{suffix}"),
                    size,
                    old,
                    value,
                    ((o / v) as u32) & mask,
                    ((o / v) as u32) & mask,
                );
                assign_op(
                    &format!("__remeq{suffix}"),
                    size,
                    old,
                    value,
                    ((o % v) as u32) & mask,
                    ((o % v) as u32) & mask,
                );
            }
            let n = value % (bits + 2);
            let shift_width = if size == 1 { 16 } else { bits };
            let sh = (o >> n.min(shift_width - 1)) as u32 & mask;
            assign_op(&format!("__shreq{suffix}"), size, old, n, sh, sh);
        }
        // ++ / --: the result is the old value
        for (old, value) in pairs(&values) {
            let up = old.wrapping_add(value) & mask;
            let down = old.wrapping_sub(value) & mask;
            assign_op(&format!("__postinc{suffix}"), size, old, value, up, old);
            assign_op(&format!("__postdec{suffix}"), size, old, value, down, old);
        }
    }
}

/// A switch table at 0x8000 whose cases jump to 0xA000 + 4k, default 0xB000.
fn switch_case(helper: &str, width: usize, cases: &[u32], value: u32) -> u16 {
    let mut table = vec![(cases.len() >> 8) as u8, cases.len() as u8];
    for (k, &c) in cases.iter().enumerate() {
        for b in (0..width).rev() {
            table.push((c >> (8 * b)) as u8);
        }
        let target = 0xA000 + 4 * k as u16;
        table.extend(target.to_be_bytes());
    }
    table.extend(0xB000u16.to_be_bytes());
    let memory: Vec<(u16, u8)> = table
        .iter()
        .enumerate()
        .map(|(k, &v)| (0x8000 + k as u16, v))
        .collect();
    let mut regs = pair(X, Y, 0x8000).to_vec();
    match width {
        1 => regs.push((B, value as u8)),
        2 => regs.extend(pair(A, B, value as u16)),
        _ => regs.extend(
            args32(0, value)
                .into_iter()
                .filter(|(r, _)| [A, B, C, D].contains(r)),
        ),
    }
    let config = MachineConfig {
        monitor: false,
        rtc: false,
        keyboard: false,
        eeprom: None,
    };
    let mut m = Machine::new(config, Some(&program().image)).unwrap();
    for &(r, v) in &regs {
        m.cpu.set_reg(r, v);
    }
    for &(a, v) in &memory {
        m.board.ram.write(a, v);
    }
    m.cpu.pc = address(helper);
    for k in 0..cases.len() as u16 {
        m.breakpoints.set(0xA000 + 4 * k, true);
    }
    m.breakpoints.set(0xB000, true);
    match m.run(1_000_000).stop {
        Some(StopReason::Breakpoint(at)) => at,
        None => panic!("{helper} did not jump"),
    }
}

#[test]
fn switch() {
    for (helper, width, cases) in [
        ("__switchc", 1, vec![3u32, 70, 0, 255]),
        ("__switch", 2, vec![3, 70, 0x0300, 0x0003, 0xFFFF, 0x4600]),
        (
            "__switchl",
            4,
            vec![100_000, 3, 0x0003_0000, 0xFFFF_FFFF, 0x0100_0000],
        ),
    ] {
        let mask = if width == 4 {
            u32::MAX
        } else {
            (1 << (8 * width)) - 1
        };
        let mut probes: Vec<u32> = cases.clone();
        probes.extend([1, 2, 4, 0x7F, 0x8000_0003 & mask, 0x0000_0300 & mask]);
        for value in probes {
            let want = cases
                .iter()
                .position(|&c| c == value)
                .map_or(0xB000, |k| 0xA000 + 4 * k as u16);
            assert_eq!(
                switch_case(helper, width, &cases, value),
                want,
                "{helper}({value:#x})"
            );
        }
        assert_eq!(
            switch_case(helper, width, &[], 3),
            0xB000,
            "{helper}: no case"
        );
    }
}
