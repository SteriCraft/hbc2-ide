use hbc2_asm::{FileSystem, Program, Target, assemble};
use hbc2_core::{Machine, MachineConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// float.has, with / only (a \ starts an escape in hbc2-asm strings)
pub fn float_has() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../lib/asm/math/float/float.has").replace('\\', "/")
}

/// the deepest stack seen by Lab::run (bytes below 0xFE00, the return address of the driver included)
pub static MAX_DEPTH: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);

/// where the driver programs are written
pub fn work_dir() -> String {
    let d = concat!(env!("CARGO_MANIFEST_DIR"), "/work");
    std::fs::create_dir_all(d).unwrap();
    d.to_string()
}

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

pub struct Prog {
    txt: u16,
    program: Program,
    opa: u16,
    opb: u16,
    fenv: u16,
}

pub struct Run {
    pub opa: u32,
    pub opb: u32,
    pub fe: u8,
    pub flags: u8,
    pub regs: [u8; 8],
    pub sp: u16,
    pub ticks: u64,
    pub halted: bool,
    /// bytes of stack used (below 0xFE00)
    pub depth: u16,
}

pub struct Lab {
    progs: HashMap<String, Prog>,
    dir: PathBuf,
}

impl Lab {
    pub fn new(dir: &str) -> Lab {
        Lab { progs: HashMap::new(), dir: PathBuf::from(dir) }
    }

    fn build(&mut self, name: &str) {
        if self.progs.contains_key(name) {
            return;
        }
        let (base, same) = match name.strip_suffix("_same") { Some(n) => (n, true), None => (name, false) };
        let ib = if same { "opa" } else { "opb" };
        let (xs, is) = match base { "ftoa" => ("txt", "opa"), "atof" => ("opa", "txt"), _ => ("opa", ib) };
        let src = format!(
            "_start:\n cli\n mov x, {xs}.msb\n mov y, {xs}.lsb\n mov i, {is}.msb\n mov j, {is}.lsb\n cal {base}\n hlt\n.bss opa 4\n.bss opb 4\n.bss txt 96\n.include \"{fh}\"\n", fh = float_has()
        );
        let path = self.dir.join(format!("drv_{name}.has"));
        std::fs::write(&path, src).unwrap();
        let program = match assemble(&path, &FileSystem, Target::Ram) {
            Ok(p) => p,
            Err(e) => {
                for d in e {
                    eprintln!("{d}");
                }
                panic!("assembly failed for {name}");
            }
        };
        let addr = |s: &str| program.symbol(s).unwrap_or_else(|| panic!("no symbol {s}")).address as u16;
        let p = Prog { txt: addr("txt"), opa: addr("opa"), opb: addr("opb"), fenv: addr("fenv_flags"), program };
        self.progs.insert(name.to_string(), p);
    }

    pub fn size(&mut self, name: &str) -> usize {
        self.build(name);
        self.progs[name].program.bytes_used
    }

    pub fn run(&mut self, name: &str, a: u32, b: u32, abcd: [u8; 4], fe_in: u8) -> Run {
        self.build(name);
        let p = &self.progs[name];
        let mut m = Machine::new(MachineConfig::default(), Some(&p.program.image)).unwrap();
        for k in 0..4 {
            m.cpu.registers[k] = abcd[k];
            m.board.ram.write(p.opa + k as u16, (a >> (24 - 8 * k)) as u8);
            m.board.ram.write(p.opb + k as u16, (b >> (24 - 8 * k)) as u8);
        }
        m.board.ram.write(p.fenv, fe_in);
        for k in 0..0x600u16 { m.board.ram.write(0xFE00 - 0x600 + k, 0xA5); }
        let mut ticks = 0u64;
        while !m.cpu.is_halted() && ticks < 20_000_000 {
            ticks += 1;
            m.tick();
        }
        let rd = |m: &Machine, at: u16| {
            (0..4).fold(0u32, |v, k| (v << 8) | m.board.ram.read(at + k) as u32)
        };
        Run {
            opa: rd(&m, p.opa),
            opb: rd(&m, p.opb),
            fe: m.board.ram.read(p.fenv),
            flags: m.cpu.flags.0,
            regs: m.cpu.registers,
            sp: m.cpu.sp,
            ticks,
            halted: m.cpu.is_halted(),
            depth: { let d = (0..0x600u16).find(|k| m.board.ram.read(0xFE00 - 0x600 + k) != 0xA5).map_or(0, |k| 0x600 - k); MAX_DEPTH.fetch_max(d, std::sync::atomic::Ordering::Relaxed); d },
        }
    }

    pub fn ptrs(&mut self, name: &str) -> (u16, u16) {
        self.build(name);
        (self.progs[name].opa, self.progs[name].opb)
    }
}

// ---- operand generators
const SPECIAL: [u32; 24] = [
    0x00000000, 0x80000000, 0x00000001, 0x80000001, 0x007FFFFF, 0x807FFFFF, 0x00800000, 0x80800000,
    0x00800001, 0x7F7FFFFF, 0xFF7FFFFF, 0x7F800000, 0xFF800000, 0x7FC00000, 0xFFC00000, 0x7F800001,
    0xFF800001, 0x7FBFFFFF, 0x7FC00001, 0x3F800000, 0xBF800000, 0x40000000, 0x3F800001, 0x3F7FFFFF,
];

pub fn special() -> &'static [u32] {
    &SPECIAL
}

pub fn rand_float(r: &mut Rng) -> u32 {
    match r.below(10) {
        0 => SPECIAL[r.below(24) as usize],
        1 => (r.next() as u32 & 0x807FFFFF) | ((r.below(4) as u32) << 23),
        2 => (r.next() as u32 & 0x807FFFFF) | ((0x7E + r.below(4) as u32) << 23),
        3 => (r.next() as u32 & 0x807FFFFF) | ((0xFC + r.below(4) as u32) << 23),
        _ => r.next() as u32,
    }
}

pub fn rand_pair(r: &mut Rng) -> (u32, u32) {
    let a = rand_float(r);
    let b = match r.below(10) {
        0 => rand_float(r),
        1 => a ^ 0x8000_0000,
        2 => (a ^ 0x8000_0000).wrapping_add(r.below(5) as u32).wrapping_sub(2),
        3 => a.wrapping_add(r.below(9) as u32).wrapping_sub(4),
        4..=6 => {
            let ea = ((a >> 23) & 0xFF) as i32;
            let eb = (ea + r.below(61) as i32 - 30).clamp(0, 254) as u32;
            (r.next() as u32 & 0x807FFFFF) | (eb << 23)
        }
        7 => {
            let ea = ((a >> 23) & 0xFF) as i32;
            let eb = (ea + r.below(5) as i32 - 2).clamp(0, 254) as u32;
            (r.next() as u32 & 0x807FFFFF) | (eb << 23)
        }
        _ => r.next() as u32,
    };
    (a, b)
}

pub fn is_nan(x: u32) -> bool {
    (x & 0x7FFF_FFFF) > 0x7F80_0000
}
pub fn is_snan(x: u32) -> bool {
    is_nan(x) && x & 0x0040_0000 == 0
}
pub fn f(x: u32) -> f32 {
    f32::from_bits(x)
}

pub struct TRun {
    pub stack: Vec<u8>,
    pub flags: u8,
    pub opa: u32,
    pub text: Vec<u8>,
    pub fe: u8,
    pub regs: [u8; 8],
    pub sp: u16,
    pub ticks: u64,
    pub halted: bool,
    pub txt: u16,
}

impl Lab {
    /// ftoa: float `a` in opa, text buffer filled with 0xEE; atof: text copied in txt (0-terminated).
    pub fn run_text(&mut self, name: &str, a: u32, text: &[u8], abcd: [u8; 4], fe_in: u8) -> TRun {
        self.build(name);
        let p = &self.progs[name];
        let mut m = Machine::new(MachineConfig::default(), Some(&p.program.image)).unwrap();
        for k in 0..4 {
            m.cpu.registers[k] = abcd[k];
            m.board.ram.write(p.opa + k as u16, (a >> (24 - 8 * k)) as u8);
        }
        for k in 0..96u16 {
            m.board.ram.write(p.txt + k, 0xEE);
        }
        for (k, c) in text.iter().enumerate() {
            m.board.ram.write(p.txt + k as u16, *c);
        }
        m.board.ram.write(p.txt + text.len() as u16, 0);
        m.board.ram.write(p.fenv, fe_in);
        let mut ticks = 0u64;
        while !m.cpu.is_halted() && ticks < 20_000_000 {
            ticks += 1;
            m.tick();
        }
        let opa = (0..4).fold(0u32, |v, k| (v << 8) | m.board.ram.read(p.opa + k) as u32);
        let t: Vec<u8> = (0..96u16).map(|k| m.board.ram.read(p.txt + k)).collect();
        TRun { stack: (0..0x140u16).map(|k| m.board.ram.read(0xFE00 - 0x140 + k)).collect(), flags: m.cpu.flags.0, opa, text: t, fe: m.board.ram.read(p.fenv), regs: m.cpu.registers, sp: m.cpu.sp, ticks, halted: m.cpu.is_halted(), txt: p.txt }
    }
}
