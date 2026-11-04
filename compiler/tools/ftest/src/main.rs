use ftest::*;
mod hyp;

const QNAN: u32 = 0x7FC00000;

fn norm(x: f32) -> u32 {
    if x.is_nan() { QNAN } else { x.to_bits() }
}

struct Stats {
    n: u64,
    bad: u64,
    max_ticks: u64,
}

fn check_regs(r: &Run, abcd: [u8; 4], ptrs: (u16, u16), a_out: bool) -> bool {
    let (pa, pb) = ptrs;
    let mut ok = r.halted && r.sp == 0xFE00;
    for k in 0..4 {
        if !(a_out && k == 0) && r.regs[k] != abcd[k] {
            ok = false;
        }
    }
    ok && r.regs[4] == (pb >> 8) as u8
        && r.regs[5] == pb as u8
        && r.regs[6] == (pa >> 8) as u8
        && r.regs[7] == pa as u8
}

/// expected result and exception flags of a + b (b already negated for subtraction)
fn add_flags(a: u32, b: u32) -> (u32, u8) {
    let (x, y) = (f(a), f(b));
    let s = x + y;
    let mut fl = 0u8;
    if is_snan(a) || is_snan(b) || (x.is_infinite() && y.is_infinite() && s.is_nan()) {
        fl |= 1;
    }
    if !s.is_nan() && s.is_infinite() && x.is_finite() && y.is_finite() {
        fl |= 0x14;
    } else if s.is_finite() {
        let bb = s - x;
        let err = (x - (s - bb)) + (y - bb);
        if err != 0.0 {
            fl |= 0x10;
        }
    }
    (norm(s), fl)
}

fn test_arith(lab: &mut Lab, name: &str, count: u64) -> Stats {
    let mut r = Rng(0x1234_5678_9ABC_DEF1 ^ name.len() as u64);
    let ptrs = lab.ptrs(name);
    let mut st = Stats { n: 0, bad: 0, max_ticks: 0 };
    let mut cases: Vec<(u32, u32)> = Vec::new();
    for &a in special() {
        for &b in special() {
            cases.push((a, b));
        }
    }
    for _ in 0..count {
        cases.push(rand_pair(&mut r));
    }
    for (a, b) in cases {
        let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
        let fe_in = (r.next() as u8) & 0x1F;
        let run = lab.run(name, a, b, abcd, fe_in);
        let (exp, efl) = match name {
            "fadd" => add_flags(a, b),
            "fmul" => mul_flags(a, b),
            "fdiv" => div_flags(a, b),
            "fmod" => mod_flags(a, b),
            "fsub" => add_flags(a, b ^ 0x8000_0000),
            "fdim" => {
                if is_nan(a) || is_nan(b) {
                    (QNAN, if is_snan(a) || is_snan(b) { 1 } else { 0 })
                } else if f(a) > f(b) {
                    add_flags(a, b ^ 0x8000_0000)
                } else {
                    (0, 0)
                }
            }
            _ => unreachable!(),
        };
        st.n += 1;
        st.max_ticks = st.max_ticks.max(run.ticks);
        let ok = run.opa == exp
            && run.opb == b
            && run.fe == (fe_in | efl)
            && check_regs(&run, abcd, ptrs, false);
        if !ok {
            st.bad += 1;
            if st.bad <= 8 {
                println!(
                    "FAIL {name} {a:08X} {b:08X}: got {:08X} fe {:02X} (b {:08X}, halted {}, sp {:04X}) expected {exp:08X} fe {:02X}",
                    run.opa, run.fe, run.opb, run.halted, run.sp, fe_in | efl
                );
            }
        }
    }
    st
}

fn main() {
    // ftest [count] [modes...]
    let dir = work_dir();
    let count: u64 = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(20000);
    let which: Vec<String> = std::env::args().skip(2).collect();
    let mut lab = Lab::new(&dir);
    for nm in hyp::NAMES.iter().copied().chain(["fexp", "fexp2", "fexp10", "flog", "flog2", "flog10", "fpow", "fsin", "fcos", "ftan", "fasin", "facos", "fatan", "fatan2"]) {
        if which.iter().any(|w| w == nm) { test_math(&mut lab, nm, count); return; }
    }
    if which.iter().any(|w| w == "dbg") { dbg_ldexp(&mut lab); return; }
    if which.iter().any(|w| w == "same") {
        test_same(&mut lab);
        return;
    }
    if which.iter().any(|w| w == "round") {
        test_round(&mut lab, count);
        return;
    }
    if which.iter().any(|w| w == "atof") {
        test_atof(&mut lab, count);
        return;
    }
    if which.iter().any(|w| w == "ftoa") {
        test_ftoa(&mut lab, count);
        return;
    }
    if which.iter().any(|w| w == "sqrt") {
        test_sqrt(&mut lab, count);
        return;
    }
    if which.iter().any(|w| w == "conv") {
        test_conv(&mut lab, count);
        return;
    }
    if which.iter().any(|w| w == "misc") {
        test_misc(&mut lab, count);
        return;
    }
    for name in ["fadd", "fsub", "fdim", "fmul", "fdiv", "fmod"] {
        if !which.is_empty() && !which.iter().any(|w| w == name) {
            continue;
        }
        let st = test_arith(&mut lab, name, count);
        println!(
            "{name}: {} cases, {} bad, max {} cycles, code {} bytes",
            st.n,
            st.bad,
            st.max_ticks,
            lab.size(name)
        );
    }
}

const C: u8 = 1;
const E: u8 = 2;
const S: u8 = 0x10;
const Z: u8 = 0x20;
const F: u8 = 0x40;

fn class(x: u32) -> u8 {
    let e = (x >> 23) & 0xFF;
    let m = x & 0x7FFFFF;
    match (e, m) {
        (255, 0) => 1,
        (255, _) => 0,
        (0, 0) => 2,
        (0, _) => 3,
        _ => 4,
    }
}

/// (expected result a register, expected cpu flags mask+value, expected fe, expected opa)
fn test_misc(lab: &mut Lab, count: u64) {
    let names = [
        "fabs", "fneg", "fcopysign", "fsetnan", "fsignbit", "fclass", "fisnan", "fisinf", "fisfinite",
        "fisnormal", "fissnan", "fcmp", "fcmps", "feq", "fne", "flt", "fle", "fgt", "fge", "fisless",
        "fislessequal", "fisgreater", "fisgreaterequal", "fislessgreater", "fisunordered", "fmin", "fmax",
    ];
    for name in names {
        let ptrs = lab.ptrs(name);
        let mut r = Rng(0xABCDEF12345 ^ name.len() as u64);
        let mut cases: Vec<(u32, u32)> = Vec::new();
        for &a in special() {
            for &b in special() {
                cases.push((a, b));
            }
        }
        for _ in 0..count {
            cases.push(rand_pair(&mut r));
        }
        let (mut n, mut bad) = (0, 0);
        for (a, b) in cases {
            let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
            let fe_in = (r.next() as u8) & 0x1F;
            let run = lab.run(name, a, b, abcd, fe_in);
            let (x, y) = (f(a), f(b));
            let unord = x.is_nan() || y.is_nan();
            let cmpflags: u8 = if unord { C } else if x > y { S } else if x < y { F } else { E };
            let mut fe = fe_in;
            let mut exp_a: Option<u8> = None;
            let mut exp_opa = a;
            let mut exp_cmp: Option<u8> = None;
            let mut a_out = true;
            let boolean = |v: bool| Some(v as u8);
            match name {
                "fabs" => { exp_opa = a & 0x7FFF_FFFF; a_out = false }
                "fneg" => { exp_opa = a ^ 0x8000_0000; a_out = false }
                "fcopysign" => { exp_opa = (a & 0x7FFF_FFFF) | (b & 0x8000_0000); a_out = false }
                "fsetnan" => { exp_opa = QNAN; a_out = false }
                "fsignbit" => exp_a = Some((a >> 31) as u8),
                "fclass" => exp_a = Some(class(a)),
                "fisnan" => exp_a = boolean(is_nan(a)),
                "fisinf" => exp_a = boolean(class(a) == 1),
                "fisfinite" => exp_a = boolean(class(a) >= 2),
                "fisnormal" => exp_a = boolean(class(a) == 4),
                "fissnan" => exp_a = boolean(is_snan(a)),
                "fcmp" | "fcmps" => {
                    a_out = false;
                    exp_cmp = Some(cmpflags);
                    let inv = if name == "fcmp" { is_snan(a) || is_snan(b) } else { unord };
                    if inv { fe |= 1 }
                }
                "feq" | "fne" | "flt" | "fle" | "fgt" | "fge" | "fisless" | "fislessequal" | "fisgreater"
                | "fisgreaterequal" | "fislessgreater" | "fisunordered" => {
                    let v = match name {
                        "feq" => x == y, "fne" => x != y, "flt" => x < y, "fle" => x <= y, "fgt" => x > y,
                        "fge" => x >= y, "fisless" => x < y, "fislessequal" => x <= y, "fisgreater" => x > y,
                        "fisgreaterequal" => x >= y, "fislessgreater" => x < y || x > y, _ => unord,
                    };
                    exp_a = boolean(v);
                    let signaling = matches!(name, "flt" | "fle" | "fgt" | "fge");
                    let inv = if signaling { unord } else { is_snan(a) || is_snan(b) };
                    if inv { fe |= 1 }
                }
                "fmin" | "fmax" => {
                    a_out = false;
                    if is_snan(a) || is_snan(b) { fe |= 1 }
                    exp_opa = if is_nan(a) && is_nan(b) || is_snan(a) || is_snan(b) { QNAN }
                        else if is_nan(a) { b } else if is_nan(b) { a }
                        else if x == y { if name == "fmax" { a & b & 0x8000_0000 | (a & 0x7FFF_FFFF) } else { (a | b) & 0x8000_0000 | (a & 0x7FFF_FFFF) } }
                        else if (x < y) == (name == "fmin") { a } else { b };
                }
                _ => unreachable!(),
            }
            let mut ok = run.opa == exp_opa && run.opb == b && run.fe == fe && check_regs(&run, abcd, ptrs, a_out);
            if let Some(v) = exp_a {
                ok &= run.regs[0] == v && (name == "fclass" || (run.flags & Z != 0) == (v == 0));
            }
            if let Some(m) = exp_cmp {
                ok &= run.flags & (C | E | S | F) == m;
            }
            n += 1;
            if !ok {
                bad += 1;
                if bad <= 4 {
                    println!("FAIL {name} {a:08X} {b:08X}: opa {:08X} (exp {exp_opa:08X}) a {:02X} (exp {exp_a:?}) fe {:02X} (exp {fe:02X}) flags {:02X} exp {exp_cmp:?}", run.opa, run.regs[0], run.fe, run.flags);
                }
            }
        }
    println!("{name}: {n} cases, {bad} bad");
    }
}

pub fn test_same(lab: &mut Lab) {
    for base in ["fadd", "fsub", "fmul", "fdiv", "fmod"] {
        let name = format!("{base}_same");
        let mut r = Rng(77);
        let mut bad = 0;
        for _ in 0..3000 {
            let a = rand_float(&mut r);
            let run = lab.run(&name, a, 0, [1, 2, 3, 4], 0);
            let (exp, _) = if base == "fmod" { mod_flags(a, a) } else if base == "fdiv" { div_flags(a, a) } else if base == "fmul" { mul_flags(a, a) } else if base == "fadd" { add_flags(a, a) } else { add_flags(a, a ^ 0x8000_0000) };
            if run.opa != exp || !run.halted || run.sp != 0xFE00 {
                bad += 1;
                if bad < 4 { println!("FAIL {name} {a:08X}: {:08X} exp {exp:08X}", run.opa); }
            }
        }
        println!("{name}: {bad} bad");
    }
}

fn mul_flags(a: u32, b: u32) -> (u32, u8) {
    let (x, y) = (f(a), f(b));
    let r = x * y;
    let mut fl = 0u8;
    if is_snan(a) || is_snan(b) || (r.is_nan() && !x.is_nan() && !y.is_nan()) {
        fl |= 1;
    }
    if !r.is_nan() && r.is_infinite() && x.is_finite() && y.is_finite() {
        fl |= 0x14;
    } else if r.is_finite() {
        let p = x as f64 * y as f64;
        if r as f64 != p {
            fl |= 0x10;
            let tiny = p.abs() < 2f64.powi(-100) && ((p * 2f64.powi(200)) as f32).abs() < 2f32.powi(74);
            if tiny {
                fl |= 0x08;
            }
        }
    }
    (norm(r), fl)
}

fn div_flags(a: u32, b: u32) -> (u32, u8) {
    let (x, y) = (f(a), f(b));
    let r = x / y;
    let mut fl = 0u8;
    if is_snan(a) || is_snan(b) || (r.is_nan() && !x.is_nan() && !y.is_nan()) {
        fl |= 1;
    }
    if y == 0.0 && x.is_finite() && x != 0.0 {
        fl |= 2;
    } else if r.is_infinite() && x.is_finite() && y.is_finite() {
        fl |= 0x14;
    } else if r.is_finite() && x.is_finite() && y.is_finite() {
        let (xd, yd, rd) = (x.abs() as f64, y.abs() as f64, r.abs() as f64);
        if rd * yd != xd {
            fl |= 0x10;
            let minp = f32::MIN_POSITIVE as f64;
            let tiny = if rd < minp {
                true
            } else if rd == minp && rd * yd > xd {
                !(xd >= (minp - 2f64.powi(-151)) * yd)
            } else {
                false
            };
            if tiny {
                fl |= 0x08;
            }
        }
    }
    (norm(r), fl)
}

fn rand_int(r: &mut Rng) -> u32 {
    let v = r.next() as u32;
    match r.below(6) {
        0 => v >> r.below(32),
        1 => (v >> r.below(32)).wrapping_neg(),
        2 => [0, 1, 0xFFFFFFFF, 0x7FFFFFFF, 0x80000000, 0x80000001, 0x01000000, 0x01000001, 0x01000003, 0xFEFFFFFF, 0xFFFFFFFE][r.below(11) as usize],
        3 => (1u32 << r.below(32)).wrapping_add(r.below(5) as u32).wrapping_sub(2),
        _ => v,
    }
}

fn conv_expect(name: &str, a: u32) -> (u32, u8) {
    match name {
        "itof" => {
            let v = a as i32;
            let r = v as f32;
            (r.to_bits(), if r as i64 != v as i64 { 0x10 } else { 0 })
        }
        "utof" => {
            let r = a as f32;
            (r.to_bits(), if r as u64 != a as u64 { 0x10 } else { 0 })
        }
        "ftoi" => {
            let x = f(a);
            let r = x as i32 as u32;
            if x.is_nan() || x >= 2147483648.0 || x < -2147483648.0 { (r, 1) }
            else { (r, if x != x.trunc() { 0x10 } else { 0 }) }
        }
        _ => {
            let x = f(a);
            let r = x as u32;
            if x.is_nan() || x >= 4294967296.0 || x <= -1.0 { (r, 1) }
            else { (r, if x != x.trunc() { 0x10 } else { 0 }) }
        }
    }
}

fn test_conv(lab: &mut Lab, count: u64) {
    for name in ["itof", "utof", "ftoi", "ftou"] {
        let ptrs = lab.ptrs(name);
        let mut r = Rng(0x5555_1234 ^ name.len() as u64 ^ name.as_bytes()[1] as u64);
        let mut cases: Vec<u32> = Vec::new();
        if name.starts_with('f') {
            cases.extend_from_slice(special());
            for _ in 0..count {
                let c = match r.below(4) {
                    0 => rand_float(&mut r),
                    1 => (rand_int(&mut r) as i32 as f32).to_bits(),
                    2 => (rand_int(&mut r) as f32).to_bits() ^ (r.below(3) as u32) ^ if r.below(2) == 0 { 0x8000_0000 } else { 0 },
                    _ => ((r.next() as u32 & 0x807FFFFF) | ((0x70 + r.below(0x50) as u32) << 23)),
                };
                cases.push(c);
            }
        } else {
            for _ in 0..count { cases.push(rand_int(&mut r)); }
        }
        let (mut n, mut bad, mut mx) = (0, 0, 0);
        for a in cases {
            let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
            let fe_in = (r.next() as u8) & 0x1F;
            let run = lab.run(name, a, 0, abcd, fe_in);
            let (exp, efl) = conv_expect(name, a);
            n += 1;
            mx = mx.max(run.ticks);
            let ok = run.opa == exp && run.opb == 0 && run.fe == (fe_in | efl) && check_regs(&run, abcd, ptrs, false);
            if !ok {
                bad += 1;
                if bad <= 6 {
                    println!("FAIL {name} {a:08X}: got {:08X} fe {:02X} halted {} sp {:04X} expected {exp:08X} fe {:02X}", run.opa, run.fe, run.halted, run.sp, fe_in | efl);
                }
            }
        }
        println!("{name}: {n} cases, {bad} bad, max {mx} cycles");
    }
}

fn mod_flags(a: u32, b: u32) -> (u32, u8) {
    let (x, y) = (f(a), f(b));
    let r = x % y;
    let mut fl = 0u8;
    if is_snan(a) || is_snan(b) || (r.is_nan() && !x.is_nan() && !y.is_nan()) {
        fl |= 1;
    }
    (norm(r), fl)
}

fn round_float(r: &mut Rng) -> u32 {
    match r.below(6) {
        0 => rand_float(r),
        1 => ((r.next() as i32 >> r.below(32)) as f32).to_bits(),
        2 => ((r.next() as i32 >> r.below(32)) as f32 + 0.5).to_bits(),
        3 => (((r.next() as i32 >> (8 + r.below(24))) as f32) + [0.5f32, 0.25, 0.75, 0.4999999, 0.5000001, -0.5][r.below(6) as usize]).to_bits(),
        4 => ((r.next() as u32 & 0x807FFFFF) | ((0x70 + r.below(0x20) as u32) << 23)),
        _ => (r.next() as u32 & 0x807FFFFF) | ((0x7E + r.below(26) as u32) << 23),
    }
}

fn round_expect(name: &str, a: u32) -> (u32, u8) {
    let x = f(a);
    let r = match name {
        "ftrunc" => x.trunc(),
        "ffloor" => x.floor(),
        "fceil" => x.ceil(),
        "fround" => x.round(),
        _ => x.round_ties_even(),
    };
    let mut fl = 0u8;
    if is_snan(a) { fl |= 1; }
    if name == "frint" && x.is_finite() && r != x { fl |= 0x10; }
    (norm(r), fl)
}

fn test_round(lab: &mut Lab, count: u64) {
    for name in ["ftrunc", "ffloor", "fceil", "fround", "frint", "fnearbyint"] {
        let ptrs = lab.ptrs(name);
        let mut r = Rng(0x7777 ^ name.len() as u64 ^ name.as_bytes()[2] as u64);
        let mut cases: Vec<u32> = special().to_vec();
        for _ in 0..count { cases.push(round_float(&mut r)); }
        let (mut n, mut bad, mut mx) = (0, 0, 0);
        for a in cases {
            let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
            let fe_in = (r.next() as u8) & 0x1F;
            let run = lab.run(name, a, 0, abcd, fe_in);
            let (exp, efl) = round_expect(if name == "fnearbyint" { "fnb" } else { name }, a);
            n += 1;
            mx = mx.max(run.ticks);
            let ok = run.opa == exp && run.fe == (fe_in | efl) && check_regs(&run, abcd, ptrs, false);
            if !ok {
                bad += 1;
                if bad <= 6 {
                    println!("FAIL {name} {a:08X}: got {:08X} fe {:02X} halted {} expected {exp:08X} fe {:02X}", run.opa, run.fe, run.halted, fe_in | efl);
                }
            }
        }
        println!("{name}: {n} cases, {bad} bad, max {mx} cycles");
    }
    test_scale(lab, count);
}

fn ldexp_expect(a: u32, n: i32) -> (u32, u8) {
    let x = f(a);
    if x.is_nan() { return (QNAN, if is_snan(a) { 1 } else { 0 }); }
    if x == 0.0 || x.is_infinite() { return (a, 0); }
    let nn = n.clamp(-400, 400);
    let p = (x as f64) * 2f64.powi(nn.clamp(-500, 500)) * 2f64.powi((nn - nn.clamp(-500, 500)).clamp(-500, 500));
    let r = p as f32;
    let mut fl = 0u8;
    if r.is_infinite() {
        fl |= 0x14;
    } else if r as f64 != p {
        fl |= 0x10;
        let pa = p.abs();
        let tiny = pa < 2f64.powi(-126) && (pa >= 2f64.powi(-170) && ((p * 2f64.powi(200)) as f32).abs() < 2f32.powi(74) || pa < 2f64.powi(-170));
        if tiny { fl |= 8; }
    }
    (r.to_bits(), fl)
}

fn test_scale(lab: &mut Lab, count: u64) {
    // ldexp
    let ptrs = lab.ptrs("ldexp");
    let mut r = Rng(0x999);
    let (mut n, mut bad) = (0, 0);
    let mut cases: Vec<(u32, i32)> = Vec::new();
    for &a in special() { for &k in &[0, 1, -1, 127, -127, 300, -300, 1000, -1000, 32767, -32768, 150, -150] { cases.push((a, k)); } }
    for _ in 0..count {
        let a = rand_float(&mut r);
        let k = match r.below(4) { 0 => r.below(700) as i32 - 350, 1 => r.next() as i16 as i32, 2 => r.below(60) as i32 - 30, _ => { let e = ((a >> 23) & 0xFF) as i32; -e + r.below(60) as i32 - 20 } };
        cases.push((a, k));
    }
    for (a, k) in cases {
        let k16 = k as i16;
        let abcd = [(k16 >> 8) as u8, k16 as u8, r.next() as u8, r.next() as u8];
        let fe_in = (r.next() as u8) & 0x1F;
        let run = lab.run("ldexp", a, 0, abcd, fe_in);
        let (exp, efl) = ldexp_expect(a, k16 as i32);
        n += 1;
        let ok = run.opa == exp && run.fe == (fe_in | efl) && check_regs(&run, abcd, ptrs, false);
        if !ok { bad += 1; if bad <= 6 { println!("FAIL ldexp {a:08X} {k16}: got {:08X} fe {:02X} expected {exp:08X} fe {:02X}", run.opa, run.fe, fe_in | efl); } }
    }
    println!("ldexp: {n} cases, {bad} bad");
    // frexp
    let ptrs = lab.ptrs("frexp");
    let (mut n, mut bad) = (0, 0);
    let mut cases: Vec<u32> = special().to_vec();
    for _ in 0..count { cases.push(rand_float(&mut r)); }
    for a in cases {
        let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
        let run = lab.run("frexp", a, 0, abcd, 0);
        let x = f(a);
        let (em, en) = if x == 0.0 || !x.is_finite() { (a, 0i32) } else {
            let sign = a & 0x8000_0000;
            let mut m = (a & 0x7FFF_FFFF) as u64;
            let mut e: i32;
            if m >> 23 == 0 { let mut fr = (a & 0x7FFFFF) as u32; let mut sh = 0; while fr & 0x800000 == 0 { fr <<= 1; sh += 1; } e = 1 - sh - 126; m = ((126u64) << 23) | (fr & 0x7FFFFF) as u64; }
            else { e = (m >> 23) as i32 - 126; m = (126u64 << 23) | (m & 0x7FFFFF); }
            let _ = &mut e;
            (sign | m as u32, e)
        };
        let got_n = ((run.regs[0] as u16) << 8 | run.regs[1] as u16) as i16 as i32;
        n += 1;
        let (pa, pb) = ptrs;
        let ok = run.opa == em && got_n == en && run.halted && run.sp == 0xFE00 && run.regs[2] == abcd[2] && run.regs[3] == abcd[3]
            && run.regs[4] == (pb >> 8) as u8 && run.regs[5] == pb as u8 && run.regs[6] == (pa >> 8) as u8 && run.regs[7] == pa as u8;
        if !ok { bad += 1; if bad <= 6 { println!("FAIL frexp {a:08X}: got {:08X} n {got_n} expected {em:08X} n {en}", run.opa); } }
    }
    println!("frexp: {n} cases, {bad} bad");
}

fn sqrt_expect(a: u32) -> (u32, u8) {
    let x = f(a);
    let r = x.sqrt();
    let mut fl = 0u8;
    if is_snan(a) || (r.is_nan() && !x.is_nan()) { fl |= 1; }
    if r.is_finite() && !x.is_nan() && (r as f64) * (r as f64) != x as f64 { fl |= 0x10; }
    (norm(r), fl)
}

fn test_sqrt(lab: &mut Lab, count: u64) {
    let name = "fsqrt";
    let ptrs = lab.ptrs(name);
    let mut r = Rng(0x5151);
    let mut cases: Vec<u32> = special().to_vec();
    for _ in 0..count {
        let a = rand_float(&mut r);
        cases.push(match r.below(4) { 0 => a & 0x7FFF_FFFF, 1 => (a & 0x007F_FFFF) , 2 => ((r.next() as u32 & 0x7FFFFF) | ((0x7E + r.below(4) as u32) << 23)), _ => a & 0x7FFF_FFFF });
    }
    for k in 0..300u32 { let v = (k as f32) * (k as f32); cases.push(v.to_bits()); cases.push((v + 1.0).to_bits()); }
    let (mut n, mut bad, mut mx) = (0, 0, 0);
    for a in cases {
        let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
        let fe_in = (r.next() as u8) & 0x1F;
        let run = lab.run(name, a, 0, abcd, fe_in);
        let (exp, efl) = sqrt_expect(a);
        n += 1;
        mx = mx.max(run.ticks);
        let ok = run.opa == exp && run.fe == (fe_in | efl) && check_regs(&run, abcd, ptrs, false);
        if !ok {
            bad += 1;
            if bad <= 8 { println!("FAIL fsqrt {a:08X}: got {:08X} fe {:02X} halted {} sp {:04X} expected {exp:08X} fe {:02X}", run.opa, run.fe, run.halted, run.sp, fe_in | efl); }
        }
    }
    println!("fsqrt: {n} cases, {bad} bad, max {mx} cycles, code {} bytes", lab.size(name));
}

fn ftoa_expect(a: u32) -> String {
    let x = f(a);
    if x.is_nan() { return "nan".into(); }
    let mut s = String::new();
    if a & 0x8000_0000 != 0 { s.push('-'); }
    if x.is_infinite() { s.push_str("inf"); return s; }
    if x == 0.0 { s.push('0'); return s; }
    let e = format!("{:e}", x.abs());
    let (mant, ex) = e.split_once('e').unwrap();
    let ex: i32 = ex.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let n = digits.len() as i32;
    let k = ex + 1;
    if k < -3 || k > 9 {
        s.push_str(&digits[..1]);
        if n > 1 { s.push('.'); s.push_str(&digits[1..]); }
        s.push('e');
        s.push(if ex < 0 { '-' } else { '+' });
        s.push_str(&format!("{:02}", ex.abs()));
    } else if k <= 0 {
        s.push_str("0.");
        for _ in 0..(-k) { s.push('0'); }
        s.push_str(&digits);
    } else {
        for i in 0..k { s.push(if i < n { digits.as_bytes()[i as usize] as char } else { '0' }); }
        if n > k { s.push('.'); s.push_str(&digits[k as usize..]); }
    }
    s
}

fn test_ftoa(lab: &mut Lab, count: u64) {
    let mut r = Rng(0xF70A);
    let ptrs = lab.ptrs("ftoa");
    let _ = ptrs;
    let mut cases: Vec<u32> = special().to_vec();
    for k in 0..40u32 { cases.push(((k as f32) * 1.5).to_bits()); cases.push(10f32.powi(k as i32 - 20).to_bits()); }
    cases.extend([0x3DCCCCCD, 0x3F800000, 0x4B800000, 0x4E6E6B28, 0x7F7FFFFF, 0x00000001, 0x00800000, 0x33800000, 0x358637BD, 0x4CBEBC20]);
    for _ in 0..count { cases.push(rand_float(&mut r)); }
    let (mut n, mut bad, mut mx) = (0, 0, 0);
    for a in cases {
        let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
        let run = lab.run_text("ftoa", a, &[], abcd, 0);
        let exp = ftoa_expect(a);
        let got: Vec<u8> = run.text.iter().cloned().take_while(|c| *c != 0).collect();
        let end = run.regs[6] as u16 * 256 + run.regs[7] as u16;
        n += 1;
        mx = mx.max(run.ticks);
        let ok = run.halted && got == exp.as_bytes() && end == run.txt + exp.len() as u16 && run.sp == 0xFE00
            && run.regs[0] == abcd[0] && run.regs[1] == abcd[1] && run.regs[2] == abcd[2] && run.regs[3] == abcd[3];
        if !ok {
            bad += 1;
            if bad <= 8 { println!("FAIL ftoa {a:08X}: got {:?} expected {:?} halted {} sp {:04X} ticks {}", String::from_utf8_lossy(&got), exp, run.halted, run.sp, run.ticks); }
        }
    }
    println!("ftoa: {n} cases, {bad} bad, max {mx} cycles, code {} bytes", lab.size("ftoa"));
}

// ---- atof
fn ref_grammar(s: &[u8]) -> Option<usize> {
    let mut i = 0;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') { i += 1; }
    let low = |k: usize| s.get(k).map(|c| c.to_ascii_lowercase()).unwrap_or(0);
    if low(i) == b'i' {
        if low(i + 1) == b'n' && low(i + 2) == b'f' {
            let n = i + 3;
            if &s.iter().skip(n).take(5).map(|c| c.to_ascii_lowercase()).collect::<Vec<_>>()[..] == b"inity" { return Some(n + 5); }
            return Some(n);
        }
        return None;
    }
    if low(i) == b'n' {
        if low(i + 1) == b'a' && low(i + 2) == b'n' { return Some(i + 3); }
        return None;
    }
    let mut nd = 0;
    while i < s.len() && s[i].is_ascii_digit() { i += 1; nd += 1; }
    if i < s.len() && s[i] == b'.' {
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() { i += 1; nd += 1; }
    }
    if nd == 0 { return None; }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        let mut k = i + 1;
        if k < s.len() && (s[k] == b'+' || s[k] == b'-') { k += 1; }
        let st = k;
        while k < s.len() && s[k].is_ascii_digit() { k += 1; }
        if k > st { i = k; }
    }
    Some(i)
}

/// (digits without leading/trailing zeros, decimal exponent of the first digit); None for zero
fn canon(s: &str) -> Option<(String, i64)> {
    let s = s.trim_start_matches(|c| c == '+' || c == '-');
    let (m, e) = match s.find(|c| c == 'e' || c == 'E') { Some(p) => (&s[..p], s[p + 1..].parse::<i64>().unwrap()), None => (s, 0) };
    let (ip, fp) = match m.find('.') { Some(p) => (&m[..p], &m[p + 1..]), None => (m, "") };
    let all: String = format!("{ip}{fp}");
    let lead = all.len() - all.trim_start_matches('0').len();
    let d = all.trim_start_matches('0').trim_end_matches('0').to_string();
    if d.is_empty() { return None; }
    Some((d, e + ip.len() as i64 - 1 - lead as i64))
}

fn atof_expect(s: &str) -> (Option<usize>, u32, u8) {
    let n = match ref_grammar(s.as_bytes()) { Some(n) => n, None => return (None, 0, 0) };
    let p = &s[..n];
    let r: f32 = p.parse().unwrap();
    let mut fl = 0u8;
    let lower = p.to_ascii_lowercase();
    if !(lower.contains("inf") || lower.contains("nan")) {
        let exact = if r.is_finite() { canon(&format!("{:.130e}", r as f64)) } else { None };
        let inp = canon(p);
        if r.is_infinite() { fl = 0x14; }
        else if inp != exact {
            fl |= 0x10;
            if r.abs() < f32::MIN_POSITIVE { fl |= 8; }
        }
    }
    (Some(n), norm(r), fl)
}

fn gen_text(r: &mut Rng) -> String {
    let sign = ["", "", "-", "+"][r.below(4) as usize];
    let body = match r.below(10) {
        0 => format!("{}", f(rand_float(r)).abs()),
        1 => format!("{:e}", f(rand_float(r)).abs()),
        2 => format!("{:.*e}", r.below(12) as usize, f(rand_float(r)).abs()),
        3 => format!("{:.*}", r.below(12) as usize, (f(rand_float(r)).abs() as f64).min(1e30)),
        4 => { let n = 1 + r.below(30); let mut s: String = (0..n).map(|_| (b'0' + r.below(10) as u8) as char).collect(); if r.below(2) == 0 { let p = r.below(s.len() as u64 + 1) as usize; s.insert(p, '.'); } if r.below(2) == 0 { s.push_str(&format!("e{}", r.below(120) as i64 - 60)); } s }
        5 => { let k = r.below(20) as usize; format!("0.{}{}", "0".repeat(k), 1 + r.below(99999)) }
        6 => { let m = r.below(100000); format!("{m}e{}", r.below(100) as i64 - 50) }
        7 => { let e = r.below(100) as i64 - 50; format!("{}e{e}", f((r.next() as u32 & 0x7FFFFFFF) | 0x3F000000).abs()) }
        8 => ["inf", "Infinity", "INF", "nan", "NaN", "infinit", "infx", "nanx", "nope", "i", "n", "", ".", "e5", "-", "+-1", "0x10", "1.2.3", "1e", "1e+", "1ex", "5.", ".5", "0.", "00012", "1E3", "0e999", "1e-9999", "1e9999", "1.5e+", " 1"][r.below(31) as usize].to_string(),
        _ => { let ex = r.below(60) as i64 - 30; format!("{}.{}e{ex}", 1 + r.below(9), r.next() % 100000000) }
    };
    let tail = ["", "", "", "x", " ", ",", "e", "abc"][r.below(8) as usize];
    format!("{sign}{body}{tail}")
}

fn test_atof(lab: &mut Lab, count: u64) {
    let mut r = Rng(0xA70F);
    let mut cases: Vec<String> = ["0", "1", "-1", "0.1", "3.14159265", "16777217", "16777216.5", "1e38", "3.4028235e38", "3.4028236e38", "3.4028235677973366e38", "3.4028234664e38", "1e39", "1e-45", "7e-46", "7.006492321624085e-46", "7.0064923216240854e-46", "1.4e-45", "1.1754944e-38", "1.17549421e-38", "1.1754942e-38", "4.9e-324", "123456789012345678901234567890", "0.000000000000000000000000000000000000000000001", "8.5e-46", "5e-46", "1e-46", "9.99999e-47", "2.2250738585072014e-308", "340282356779733661637539395458142568447", "340282356779733661637539395458142568448", "0.1e1", "1e0", "100e-2"].iter().map(|s| s.to_string()).collect();
    for k in 0..40 { cases.push(format!("{}", 2f64.powi(k * 8 - 149 - 20))); cases.push(format!("{:e}", 2f64.powi(k * 8 - 149 - 20) * 1.0000001)); }
    for _ in 0..count { cases.push(gen_text(&mut r)); }
    if let Ok(v) = std::env::var("ATOF_CASES") { cases = v.split(',').map(|s| s.to_string()).collect(); }
    let ptrs = lab.ptrs("atof");
    let (mut n, mut bad, mut mx) = (0, 0, 0);
    for s in cases {
        let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
        let fe_in = (r.next() as u8) & 0x1F;
        let run = lab.run_text("atof", 0xDEADBEEF, s.as_bytes(), abcd, fe_in);
        let (cons, eb, efl) = atof_expect(&s);
        n += 1;
        mx = mx.max(run.ticks);
        let iend = run.regs[4] as u16 * 256 + run.regs[5] as u16;
        let xend = run.regs[6] as u16 * 256 + run.regs[7] as u16;
        let carry = run.flags & 1 != 0;
        let ok = run.halted && run.sp == 0xFE00 && xend == ptrs.0 && run.regs[..4] == abcd
            && match cons {
                None => carry && run.opa == 0xDEADBEEF && iend == run.txt && run.fe == fe_in,
                Some(c) => !carry && run.opa == eb && iend == run.txt + c as u16 && run.fe == (fe_in | efl),
            };
        if !ok && std::env::var("ATOF_DUMP").is_ok() { let st = &run.stack; let base = 0x140 - 10 - 192; println!("frame R {:02x?}
 S {:02x?}
 vars {:02x?}", &st[base..base+32], &st[base+32..base+64], &st[base+160..base+192]); }
        if !ok {
            bad += 1;
            if bad <= 10 { println!("FAIL atof {:?}: got {:08X} fe {:02X} carry {} end {} halted {} sp {:04X}; expected {:?} {:08X} fe {:02X}", s, run.opa, run.fe, carry, iend as i32 - run.txt as i32, run.halted, run.sp, cons, eb, fe_in | efl); }
        }
    }
    println!("atof: {n} cases, {bad} bad, max {mx} cycles, code {} bytes", lab.size("atof"));
}

// ---------------------------------------------------------------- exp / log / pow
fn math_oracle(name: &str, a: u32, b: u32) -> f32 {
    if let Some(v) = hyp::oracle64(name, a, b) { return v as f32; }
    let x = f(a) as f64;
    let y = f(b) as f64;
    (match name {
        "fexp" => x.exp(),
        "fexp2" => x.exp2(),
        "fexp10" => 10f64.powf(x),
        "flog" => x.ln(),
        "flog2" => x.log2(),
        "flog10" => x.log10(),
        "fsin" => x.sin(),
        "fcos" => x.cos(),
        "ftan" => x.tan(),
        "fasin" => x.asin(),
        "facos" => x.acos(),
        "fatan" => x.atan(),
        "fatan2" => x.atan2(y),
        _ => x.powf(y),
    }) as f32
}

fn ulps(got: u32, exp: u32) -> u64 {
    if got == exp { return 0; }
    if is_nan(got) || is_nan(exp) { return 1 << 40; }
    let m = |v: u32| -> i64 { if v & 0x8000_0000 != 0 { -((v & 0x7FFF_FFFF) as i64) } else { v as i64 } };
    (m(got) - m(exp)).unsigned_abs()
}

fn math_flags(name: &str, a: u32, b: u32, r: f32) -> (u8, u8) {
    if let Some(v) = hyp::flags(name, a, b, r) { return v; }
    // (flags, mask of flags that are compared)
    let x = f(a);
    let y = f(b);
    let two = name == "fpow";
    if matches!(name, "fsin" | "fcos" | "ftan") {
        if is_snan(a) { return (1, 0x1F); }
        if x.is_nan() { return (0, 0x1F); }
        if x.is_infinite() { return (1, 0x1F); }
        if x == 0.0 { return (0, 0x1F); }
        if x.abs() < f32::MIN_POSITIVE && name != "fcos" { return (0x18, 0x1F); }
        return (0x10, 0x1F);
    }
    if name == "fatan" {
        if is_snan(a) { return (1, 0x1F); }
        if x.is_nan() { return (0, 0x1F); }
        if x == 0.0 { return (0, 0x1F); }
        if x.abs() < f32::MIN_POSITIVE { return (0x18, 0x1F); }
        return (0x10, 0x1F);
    }
    if name == "fasin" || name == "facos" {
        if is_snan(a) { return (1, 0x1F); }
        if x.is_nan() { return (0, 0x1F); }
        if x.abs() > 1.0 { return (1, 0x1F); }
        if name == "fasin" {
            if x == 0.0 { return (0, 0x1F); }
            if x.abs() < f32::MIN_POSITIVE { return (0x18, 0x1F); }
            return (0x10, 0x1F);
        }
        if x == 1.0 { return (0, 0x1F); }
        return (0x10, 0x1F);
    }
    if name == "fatan2" {
        let (yy, xx) = (x, y);
        if is_snan(a) || is_snan(b) { return (1, 0x1F); }
        if yy.is_nan() || xx.is_nan() { return (0, 0x1F); }
        if yy == 0.0 { return (if xx.is_sign_negative() { 0x10 } else { 0 }, 0x1F); }
        if xx == 0.0 { return (0x10, 0x1F); }
        if yy.is_infinite() { return (0x10, 0x1F); }
        if xx.is_infinite() { return (if xx > 0.0 { 0 } else { 0x10 }, 0x1F); }
        if r == 0.0 || r.abs() < f32::MIN_POSITIVE { return (0x18, 0x1F); }
        return (0x10, 0x1F);
    }
    let mut fl = 0u8;
    let mut mask = 0x0Fu8;
    if name == "fpow" {
        if y == 0.0 || x == 1.0 { return (0, 0x1F); }
        if is_snan(a) || is_snan(b) { return (1, 0x1F); }
        if x.is_nan() || y.is_nan() { return (0, 0x1F); }
        if y == 1.0 { return (0, 0x1F); }
        if x < 0.0 && x.is_finite() && y.is_finite() && y != y.trunc() { return (1, 0x1F); }
        if x == 0.0 && y < 0.0 { return (2, 0x1F); }
        if !x.is_finite() || !y.is_finite() || x == 0.0 { return (0, 0x1F); }
        // finite non-zero x, finite y. Exact (no flag) only when |x| is a power of 2
        // and y log2 |x| an integer (that is what the library detects)
        let ax = a & 0x7FFF_FFFF;
        let l2 = if ax.count_ones() == 1 && ax < 0x0080_0000 { Some(ax.trailing_zeros() as f64 - 149.0) }
            else if ax & 0x7F_FFFF == 0 { Some(((ax >> 23) as i32 - 127) as f64) } else { None };
        if let Some(e) = l2 {
            let p = y as f64 * e;
            if p == p.trunc() && r.is_finite() && r != 0.0 { return (0, 0x1F); }
        }
        fl |= 0x10;
        if r.is_infinite() { fl |= 0x14; }
        else if r.abs() < f32::MIN_POSITIVE && r.to_bits() & 0x7FFF_FFFF != 0x0080_0000 { fl |= 0x08; }
        else if r == 0.0 { fl |= 0x08; }
        return (fl, 0x1F);
    }
    let _ = two;
    if is_snan(a) { return (1, 0x1F); }
    if x.is_nan() { return (0, 0x1F); }
    match name {
        "fexp" | "fexp2" | "fexp10" => {
            if x.is_infinite() { return (0, 0x1F); }
            let exact = x == 0.0 || (name == "fexp2" && x == x.trunc() && x >= -149.0 && x <= 127.0);
            if exact { return (0, 0x1F); }
            fl |= 0x10;
            mask = 0x1F;
            if r.is_infinite() { fl |= 0x04; }
            else if r.abs() < f32::MIN_POSITIVE && r.to_bits() & 0x7FFF_FFFF != 0x0080_0000 { fl |= 0x08; }
            else if r == 0.0 { fl |= 0x08; }
            (fl, mask)
        }
        _ => {
            if x < 0.0 { return (1, 0x1F); }
            if x == 0.0 { return (2, 0x1F); }
            if x == 1.0 { return (0, 0x1F); }
            if x == f32::INFINITY { return (0, 0x1F); }
            let pow2 = (a & 0x7FFF_FFFF).count_ones() == 1 || (a >> 23 != 0 && a & 0x7F_FFFF == 0);
            (if name == "flog2" && pow2 { 0 } else { 0x10 }, 0x1F)
        }
    }
}

fn gen_math(name: &str, r: &mut Rng) -> (u32, u32) {
    if let Some(v) = hyp::generate(name, r) { return v; }
    let u = |r: &mut Rng| (r.next() >> 11) as f64 / (1u64 << 53) as f64;
    let rf = |r: &mut Rng, lo: f64, hi: f64| (lo + (hi - lo) * u(r)) as f32;
    let a = match name {
        "fatan" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, -10.0, 10.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(), 4 => rf(r, 0.9, 1.1).to_bits(), 5 => rf(r, -1e9, 1e9).to_bits(), 6 => rf(r, -0.1, 0.1).to_bits(), _ => rf(r, -100.0, 100.0).to_bits() },
        "fasin" | "facos" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, -1.0, 1.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, 0.9, 1.0).to_bits(), 4 => rf(r, -1.0, -0.9).to_bits(), 5 => rf(r, -1e-3, 1e-3).to_bits(), 6 => (1.0f32 - (r.below(100) as f32) * 5.9604645e-8).to_bits() ^ (if r.below(2) == 0 { 0 } else { 0x8000_0000 }), _ => rf(r, -0.5, 0.5).to_bits() },
        "fatan2" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, -10.0, 10.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(), 4 => rf(r, -1e5, 1e5).to_bits(), 5 => rf(r, -3.0, 3.0).to_bits(), 6 => rf(r, -1e-20, 1e-20).to_bits(), _ => rf(r, -100.0, 100.0).to_bits() },
        "fsin" | "fcos" | "ftan" => match r.below(10) {
            0 => rand_float(r),
            1 => rf(r, -8.0, 8.0).to_bits(),
            2 => rf(r, -1e4, 1e4).to_bits(),
            3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => ((r.below(100000) as f64 + 1.0) * std::f64::consts::FRAC_PI_2) as f32 as u32 ^ (if r.below(2) == 0 { 0 } else { 0x8000_0000 }),
            5 => rf(r, 0.7, 0.9).to_bits(),
            6 => { let e = 20 + r.below(108) as i32; (((r.next() >> 11) as f64 / (1u64 << 53) as f64 + 1.0) * 2f64.powi(e)) as f32 as u32 },
            7 => ((((r.below(1u64 << 40)) as f64) + 1.0) * std::f64::consts::FRAC_PI_2) as f32 as u32,
            8 => rf(r, -0.01, 0.01).to_bits(),
            _ => rf(r, -100.0, 100.0).to_bits(),
        },
        "fexp" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, -110.0, 92.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(), 4 => rf(r, 80.0, 90.0).to_bits(), 5 => rf(r, -105.0, -85.0).to_bits(), _ => rf(r, -100.0, 89.0).to_bits() },
        "fexp2" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, -155.0, 130.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => (r.below(300) as f32 - 160.0).to_bits(), 4 => rf(r, 120.0, 130.0).to_bits(), 5 => rf(r, -152.0, -120.0).to_bits(), _ => rf(r, -130.0, 128.0).to_bits() },
        "fexp10" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, -48.0, 40.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(), 4 => rf(r, 36.0, 40.0).to_bits(), 5 => rf(r, -46.0, -36.0).to_bits(), _ => rf(r, -40.0, 38.5).to_bits() },
        "fpow" => match r.below(8) { 0 => rand_float(r), 1 => rf(r, 0.0, 10.0).to_bits(), 2 => rf(r, 0.5, 2.0).to_bits(), 3 => (1.0f32 + (r.below(2000) as f32 - 1000.0) * 1.1920929e-7).to_bits(), _ => (rand_float(r)) & 0x7FFF_FFFF },
        _ => match r.below(8) { 0 => rand_float(r), 1 => rand_float(r) & 0x7FFF_FFFF, 2 => (1.0f32 + (r.below(2000) as f32 - 1000.0) * 1.1920929e-7).to_bits(), 3 => rf(r, 0.0, 10.0).to_bits(), 4 => rf(r, 0.5, 2.0).to_bits(), 5 => ((r.next() as u32) & 0x007F_FFFF), _ => rand_float(r) & 0x7FFF_FFFF },
    };
    let b = if name == "fatan2" {
        match r.below(10) { 0 => rand_float(r), 1 => rf(r, -10.0, 10.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(), 4 => rf(r, -1e5, 1e5).to_bits(), 5 => rf(r, -3.0, 3.0).to_bits(), 6 => rf(r, -1e-20, 1e-20).to_bits(), 7 => rf(r, 1e20, 1e30).to_bits(), 8 => a ^ (if r.below(2) == 0 { 0 } else { 0x8000_0000 }), _ => rf(r, -100.0, 100.0).to_bits() }
    } else if name == "fpow" {
        match r.below(8) { 0 => rand_float(r), 1 => rf(r, -10.0, 10.0).to_bits(), 2 => (r.below(40) as f32 - 20.0).to_bits(), 3 => rf(r, -1.0, 1.0).to_bits(), 4 => rf(r, -100.0, 100.0).to_bits(), 5 => (r.below(40) as f32 - 20.0 + 0.5).to_bits(), 6 => rf(r, -1000.0, 1000.0).to_bits(), _ => rf(r, -3.0, 3.0).to_bits() }
    } else { 0 };
    (a, b)
}

fn test_math(lab: &mut Lab, name: &str, count: u64) {
    let ptrs = lab.ptrs(name);
    let mut r = Rng(0x9A9A_1234 ^ name.len() as u64 * 77);
    let mut cases: Vec<(u32, u32)> = Vec::new();
    for &s in special() { if name == "fpow" || name == "fatan2" || name == "fhypot" { for &t in special() { cases.push((s, t)); } } else { cases.push((s, 0)); } }
    for _ in 0..count { cases.push(gen_math(name, &mut r)); }
    let (mut n, mut bad, mut mx, mut mxu, mut off, mut flbad) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut hist = [0u64; 5];
    let (mut worst, mut esum, mut ecount) = (0f64, 0f64, 0u64);
    for (a, b) in cases {
        let abcd = [r.next() as u8, r.next() as u8, r.next() as u8, r.next() as u8];
        let fe_in = (r.next() as u8) & 0x1F;
        let run = lab.run(name, a, b, abcd, fe_in);
        let want = math_oracle(name, a, b);
        let exp = norm(want);
        let (efl, mask) = math_flags(name, a, b, want);
        n += 1;
        mx = mx.max(run.ticks);
        let u = ulps(run.opa, exp);
        if want.is_normal() && f(run.opa).is_normal() {
            let v = math_oracle64(name, a, b);
            let e = f(run.opa);
            let ulp = (f32::from_bits(e.to_bits() + 1) - e).abs() as f64;
            let err = ((e as f64) - v).abs() / ulp;
            if std::env::var("WORST").is_ok() && err > 0.55 { println!("worst {err:.3}: {a:08X} ({}) {b:08X} ({}) got {:08X}", f(a), f(b), run.opa); }
            if err > worst { worst = err; }
            esum += err; ecount += 1;
        }
        mxu = mxu.max(u.min(1000));
        hist[(u.min(4)) as usize] += 1;
        let regs_ok = check_regs(&run, abcd, ptrs, false);
        let fl_ok = (run.fe & mask) == ((fe_in | efl) & mask) && (run.fe & fe_in) == fe_in;
        if u > 0 { off += 1; }
        if std::env::var("SHOWU").is_ok() && u > 0 { println!("ULP {name} {a:08X} ({}) {b:08X} ({}): got {:08X} exp {exp:08X} {u}", f(a), f(b), run.opa); }
        if !fl_ok || !regs_ok || u > 1 {
            if !fl_ok { flbad += 1; }
            bad += 1;
            if bad <= 12 { println!("FAIL {name} {a:08X} ({}) {b:08X} ({}): got {:08X} fe {:02X} halted {} sp {:04X}; expected {exp:08X} ({}) fe {:02X} mask {mask:02X}; {u} ulp regs_ok {regs_ok}", f(a), f(b), run.opa, run.fe, run.halted, run.sp, f(exp), fe_in | efl); }
        }
    }
    println!("{name}: true error: worst {worst:.3} ulp, mean {:.3}", esum / ecount.max(1) as f64);
    println!("{name}: stack {} bytes", MAX_DEPTH.load(std::sync::atomic::Ordering::Relaxed));
    println!("{name}: {n} cases, {bad} bad (>1 ulp, flags or regs), flag-bad {flbad}, not-exact {off}, max ulp {mxu}, hist(0,1,2,3,4+) {:?}, max {mx} cycles, code {} bytes", hist, lab.size(name));
}

#[allow(dead_code)]
pub fn dbg_ldexp(lab: &mut Lab) {
    for (a, n) in [(0x3F9F1234u32, -128i16), (0x3FB2C345, -127), (0x3F800001, -127), (0x3F9F1234, -126), (0x3FBFFFFF, -127)] {
        let r = lab.run("ldexp", a, 0, [(n >> 8) as u8, n as u8, 0, 0], 0);
        println!("ldexp {a:08X} {n}: {:08X} fe {:02X} (f32 says {:08X})", r.opa, r.fe, (f(a) * 2f32.powi(n as i32 + 100) * 2f32.powi(-100)).to_bits());
    }
}

fn math_oracle64(name: &str, a: u32, b: u32) -> f64 {
    if let Some(v) = hyp::oracle64(name, a, b) { return v; }
    let x = f(a) as f64;
    let y = f(b) as f64;
    match name {
        "fexp" => x.exp(),
        "fexp2" => x.exp2(),
        "fexp10" => 10f64.powf(x),
        "flog" => x.ln(),
        "flog2" => x.log2(),
        "flog10" => x.log10(),
        "fsin" => x.sin(),
        "fcos" => x.cos(),
        "ftan" => x.tan(),
        "fasin" => x.asin(),
        "facos" => x.acos(),
        "fatan" => x.atan(),
        "fatan2" => x.atan2(y),
        _ => x.powf(y),
    }
}
