// Tests of expm1, log1p, sinh, cosh, tanh, asinh, acosh, atanh, cbrt, hypot.
use ftest::*;

pub const NAMES: [&str; 10] = ["fexpm1", "flog1p", "fsinh", "fcosh", "ftanh", "fasinh", "facosh", "fatanh", "fcbrt", "fhypot"];

pub fn oracle64(name: &str, a: u32, b: u32) -> Option<f64> {
    let x = f(a) as f64;
    let y = f(b) as f64;
    Some(match name {
        "fexpm1" => x.exp_m1(),
        "flog1p" => x.ln_1p(),
        "fsinh" => x.sinh(),
        "fcosh" => x.cosh(),
        "ftanh" => x.tanh(),
        "fasinh" => x.asinh(),
        "facosh" => x.acosh(),
        "fatanh" => x.atanh(),
        "fcbrt" => x.cbrt(),
        "fhypot" => x.hypot(y),
        _ => return None,
    })
}

// odd mantissa and exponent of a finite non-zero float: |x| = m 2^e
fn odd_me(x: f32) -> (u128, i32) {
    let b = x.to_bits() & 0x7FFF_FFFF;
    let (mut m, mut e) = if b >> 23 == 0 { ((b & 0x7F_FFFF) as u128, -149) } else { (((b & 0x7F_FFFF) | 0x80_0000) as u128, (b >> 23) as i32 - 150) };
    while m & 1 == 0 { m >>= 1; e += 1; }
    (m, e)
}

fn hypot_exact(a: f32, b: f32, r: f32) -> bool {
    if a == 0.0 || b == 0.0 { return true; }
    let (ma, ea) = odd_me(a);
    let (mb, eb) = odd_me(b);
    let (mr, er) = odd_me(r);
    let (sa, sb, sr) = (ma * ma, mb * mb, mr * mr);
    let (xa, xb, xr) = (2 * ea, 2 * eb, 2 * er);
    let lo = xa.min(xb).min(xr);
    let hi = xa.max(xb).max(xr);
    if hi - lo > 70 { return false; }
    ((sa << (xa - lo)) + (sb << (xb - lo))) == (sr << (xr - lo))
}

fn cbrt_exact(x: f32, r: f32) -> bool {
    if x == 0.0 { return true; }
    let (mx, ex) = odd_me(x);
    let (mr, er) = odd_me(r);
    mr * mr * mr == mx && 3 * er == ex
}

pub fn flags(name: &str, a: u32, b: u32, r: f32) -> Option<(u8, u8)> {
    if !NAMES.contains(&name) { return None; }
    let x = f(a);
    let y = f(b);
    let m = 0x1Fu8;
    let sub = |v: f32| v != 0.0 && v.abs() < f32::MIN_POSITIVE;
    if name == "fhypot" {
        if x.is_infinite() || y.is_infinite() { return Some((0, m)); }
        if is_snan(a) || is_snan(b) { return Some((1, m)); }
        if x.is_nan() || y.is_nan() { return Some((0, m)); }
        if r.is_infinite() { return Some((0x14, m)); }
        if hypot_exact(x, y, r) { return Some((0, m)); }
        if sub(r) { return Some((0x18, m)); }
        return Some((0x10, m));
    }
    if is_snan(a) { return Some((1, m)); }
    if x.is_nan() { return Some((0, m)); }
    let fl = match name {
        "fexpm1" => {
            if x == f32::INFINITY || x == f32::NEG_INFINITY || x == 0.0 { 0 }
            else if sub(x) { 0x18 } else if r.is_infinite() { 0x14 } else { 0x10 }
        }
        "flog1p" => {
            if x < -1.0 { 1 } else if x == -1.0 { 2 } else if x == f32::INFINITY || x == 0.0 { 0 } else if sub(x) { 0x18 } else { 0x10 }
        }
        "fsinh" => {
            if x.is_infinite() || x == 0.0 { 0 } else if sub(x) { 0x18 } else if r.is_infinite() { 0x14 } else { 0x10 }
        }
        "fcosh" => {
            if x.is_infinite() || x == 0.0 { 0 } else if r.is_infinite() { 0x14 } else { 0x10 }
        }
        "ftanh" => {
            if x.is_infinite() || x == 0.0 { 0 } else if sub(x) { 0x18 } else { 0x10 }
        }
        "fasinh" => {
            if x.is_infinite() || x == 0.0 { 0 } else if sub(x) { 0x18 } else { 0x10 }
        }
        "facosh" => {
            if x < 1.0 { 1 } else if x == 1.0 || x == f32::INFINITY { 0 } else { 0x10 }
        }
        "fatanh" => {
            if x.abs() > 1.0 { 1 } else if x.abs() == 1.0 { 2 } else if x == 0.0 { 0 } else if sub(x) { 0x18 } else { 0x10 }
        }
        _ => {
            // fcbrt
            if x.is_infinite() || x == 0.0 { 0 } else if cbrt_exact(x, r) { 0 } else { 0x10 }
        }
    };
    Some((fl, m))
}

pub fn generate(name: &str, r: &mut Rng) -> Option<(u32, u32)> {
    if !NAMES.contains(&name) { return None; }
    let u = |r: &mut Rng| (r.next() >> 11) as f64 / (1u64 << 53) as f64;
    let rf = |r: &mut Rng, lo: f64, hi: f64| (lo + (hi - lo) * u(r)) as f32;
    let sg = |r: &mut Rng, v: u32| if r.below(2) == 0 { v } else { v ^ 0x8000_0000 };
    let a = match name {
        "fexpm1" => match r.below(9) {
            0 => rand_float(r), 1 => rf(r, -20.0, 89.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -0.5, 0.5).to_bits(),
            4 => rf(r, -1e-3, 1e-3).to_bits(), 5 => rf(r, 0.3, 0.4).to_bits(), 6 => rf(r, -0.4, -0.3).to_bits(), 7 => rf(r, 80.0, 89.0).to_bits(), _ => rf(r, -18.5, 3.0).to_bits(),
        },
        "flog1p" => match r.below(9) {
            0 => rand_float(r), 1 => rf(r, -1.0, 1.0).to_bits(), 2 => rf(r, -0.5, 10.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => (-1.0f32 + (r.below(2000) as f32) * 5.9604645e-8).to_bits(), 5 => rf(r, 0.0, 1e6).to_bits(), 6 => rf(r, -0.1, 0.1).to_bits(), 7 => rf(r, 0.0, 1e38).to_bits(), _ => rf(r, -1e-7, 1e-7).to_bits(),
        },
        "fsinh" | "fcosh" => match r.below(9) {
            0 => rand_float(r), 1 => rf(r, -20.0, 20.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => rf(r, 80.0, 90.0).to_bits(), 5 => rf(r, -90.0, -80.0).to_bits(), 6 => rf(r, 0.3, 0.4).to_bits(), 7 => rf(r, -0.4, 0.4).to_bits(), _ => rf(r, -100.0, 100.0).to_bits(),
        },
        "ftanh" => match r.below(8) {
            0 => rand_float(r), 1 => rf(r, -10.0, 10.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => rf(r, 8.0, 10.0).to_bits(), 5 => rf(r, -0.3, 0.3).to_bits(), 6 => rf(r, 0.1, 0.25).to_bits(), _ => rf(r, -5.0, 5.0).to_bits(),
        },
        "fasinh" => match r.below(9) {
            0 => rand_float(r), 1 => rf(r, -100.0, 100.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => rf(r, -1e6, 1e6).to_bits(), 5 => rf(r, -1e12, 1e12).to_bits(), 6 => rf(r, -10.0, 10.0).to_bits(), 7 => rf(r, -1e38, 1e38).to_bits(), _ => rf(r, -8192.0 * 2.0, 8192.0 * 2.0).to_bits(),
        },
        "facosh" => match r.below(9) {
            0 => rand_float(r), 1 => rf(r, 1.0, 100.0).to_bits(), 2 => rf(r, 1.0, 2.0).to_bits(), 3 => (1.0f32 + (r.below(2000) as f32) * 1.1920929e-7).to_bits(),
            4 => rf(r, 1.0, 1e6).to_bits(), 5 => rf(r, 1.0, 1e12).to_bits(), 6 => rf(r, 1.0, 10.0).to_bits(), 7 => rf(r, 1.0, 1e38).to_bits(), _ => rf(r, 0.0, 20000.0).to_bits(),
        },
        "fatanh" => match r.below(9) {
            0 => rand_float(r), 1 => rf(r, -1.0, 1.0).to_bits(), 2 => rf(r, -0.5, 0.5).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => (1.0f32 - (r.below(2000) as f32) * 5.9604645e-8).to_bits(), 5 => rf(r, 0.9, 1.0).to_bits(), 6 => rf(r, -0.1, 0.1).to_bits(), 7 => rf(r, -0.01, 0.01).to_bits(), _ => rf(r, -0.99, 0.99).to_bits(),
        },
        "fcbrt" => match r.below(8) {
            0 => rand_float(r), 1 => rf(r, -100.0, 100.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e6, 1e6).to_bits(),
            4 => { let n = r.below(200) as f32 - 100.0; (n * n * n).to_bits() } 5 => rf(r, -1e38, 1e38).to_bits(), 6 => (r.next() as u32) & 0x00FF_FFFF, _ => rf(r, -10.0, 10.0).to_bits(),
        },
        _ => match r.below(9) {
            // fhypot
            0 => rand_float(r), 1 => rf(r, -100.0, 100.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => rf(r, -1e30, 1e30).to_bits(), 5 => rf(r, -1e-20, 1e-20).to_bits(), 6 => { let k = r.below(60) as f32 + 1.0; (k * 3.0).to_bits() } 7 => rf(r, 1e37, 3e38).to_bits(), _ => rf(r, -10.0, 10.0).to_bits(),
        },
    };
    let a = sg(r, a);
    let b = if name == "fhypot" {
        match r.below(10) {
            0 => rand_float(r), 1 => rf(r, -100.0, 100.0).to_bits(), 2 => rf(r, -1.0, 1.0).to_bits(), 3 => rf(r, -1e-3, 1e-3).to_bits(),
            4 => rf(r, -1e30, 1e30).to_bits(), 5 => rf(r, -1e-20, 1e-20).to_bits(), 6 => { let k = r.below(60) as f32 + 1.0; (k * 4.0).to_bits() } 7 => rf(r, 1e37, 3e38).to_bits(),
            8 => { let q = rf(r, 0.5, 1.0); sg(r, (f(a).abs() * q).to_bits()) } _ => rf(r, -10.0, 10.0).to_bits(),
        }
    } else { 0 };
    Some((a, b))
}
