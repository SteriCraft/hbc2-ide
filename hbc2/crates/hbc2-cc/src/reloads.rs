//! Removal of loads that put back what a register already holds: a `lod`
//! of a variable just stored from (or loaded into) that register, a `mov`
//! of a constant the register already has, a `mov` between two registers
//! that are already equal. Runs on each compiled file, after copt.
//!
//! Each register has a set of things it is known to equal: constants,
//! bytes of memory (a global or static byte `_x + n`, a stack slot, a byte
//! at `[x:y + n]`), or another register's unknown value. Everything is
//! forgotten where code can come from elsewhere (a label that is jumped to,
//! or that another file may call), after a call, a jump or anything not
//! understood. A store forgets what it may overwrite: through `[x:y]`, all
//! of memory; elsewhere, that byte and everything at `[x:y]`.
//!
//! `lod` and `mov` keep the flags, so removing one never changes a branch.
//!
//! Volatile memory is never remembered: cc2 lists it at the end of the file
//! (`;volatile _name`, `;volatile stack`, `;volatile pointer`, and
//! `;volatile *` for everything, which turns the pass off for the file).

use std::collections::HashSet;

const REGS: [&str; 8] = ["a", "b", "c", "d", "i", "j", "x", "y"];
const X: usize = 6;
const Y: usize = 7;

/// A byte of memory.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Place {
    /// `name + offset`
    Sym(String, i64),
    /// A stack slot: offset from SP minus the bytes pushed since the start
    /// of the code followed (the same slot keeps its key when SP moves).
    Stack(i64),
    /// `[x:y + offset]`, while x and y don't change.
    Xy(i64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Val {
    /// An 8-bit constant (a number) or `name.msb` / `name.lsb`.
    Const(String),
    Mem(Place),
    /// The unknown value of some register, copied with `mov`.
    Same(u32),
}

/// What is volatile in the file.
#[derive(Default)]
struct Volatiles {
    names: HashSet<String>,
    stack: bool,
    pointer: bool,
    all: bool,
}

impl Volatiles {
    fn read(asm: &str) -> Self {
        let mut v = Volatiles::default();
        for line in asm.lines() {
            match line.trim_end().strip_prefix(";volatile ") {
                Some("stack") => v.stack = true,
                Some("pointer") => v.pointer = true,
                Some("*") => v.all = true,
                Some(name) => {
                    v.names.insert(name.to_string());
                }
                None => {}
            }
        }
        v
    }

    fn of(&self, place: &Place) -> bool {
        match place {
            Place::Sym(name, _) => self.names.contains(name),
            Place::Stack(_) => self.stack,
            Place::Xy(_) => self.pointer,
        }
    }
}

fn reg(text: &str) -> Option<usize> {
    REGS.iter().position(|r| r.eq_ignore_ascii_case(text))
}

fn number(text: &str) -> Option<i64> {
    let t = text.trim();
    let (neg, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim()),
        None => (false, t),
    };
    let v = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()?
    } else if let Some(b) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        i64::from_str_radix(b, 2).ok()?
    } else if !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit()) {
        t.parse().ok()?
    } else {
        return None;
    };
    Some(if neg { -v } else { v })
}

fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        && reg(text).is_none()
}

/// `name`, `name + n`, `name - n`
fn symbol(text: &str) -> Option<(String, i64)> {
    let text = text.trim();
    for (sign, op) in [(1, '+'), (-1, '-')] {
        if let Some((name, n)) = text.split_once(op) {
            let name = name.trim();
            return (is_name(name)).then(|| number(n).map(|n| (name.to_string(), sign * n)))?;
        }
    }
    is_name(text).then(|| (text.to_string(), 0))
}

/// Inside brackets: `sp`, `sp + n`, `x:y`, `x:y + n`, with n a number.
fn bracket(inner: &str) -> Option<(String, i64)> {
    match inner.split_once('+') {
        Some((base, n)) => Some((base.trim().to_ascii_lowercase(), number(n)?)),
        None => Some((inner.trim().to_ascii_lowercase(), 0)),
    }
}

/// The 8-bit constant an operand stands for, as text (numbers normalized).
fn constant(text: &str) -> Option<String> {
    let text = text.trim();
    if let Some(n) = number(text) {
        return Some((n & 0xFF).to_string());
    }
    let (name, part) = text.split_once('.')?;
    (is_name(name) && (part == "msb" || part == "lsb")).then(|| text.to_string())
}

/// Splits an instruction line into its mnemonic and operands. None when
/// the line holds anything unusual (a quote, an empty operand).
fn instruction(line: &str) -> Option<(String, Vec<String>)> {
    if line.contains(['\'', '"']) {
        return None;
    }
    let code = line.split(';').next().unwrap_or("").trim();
    let (mnemonic, rest) = match code.split_once(char::is_whitespace) {
        Some((m, rest)) => (m, rest.trim()),
        None => (code, ""),
    };
    let operands: Vec<String> = if rest.is_empty() {
        Vec::new()
    } else {
        rest.split(',').map(|o| o.trim().to_string()).collect()
    };
    if operands.iter().any(String::is_empty) {
        return None;
    }
    Some((mnemonic.to_ascii_lowercase(), operands))
}

/// Every name used outside label definitions: labels that are jumped to,
/// called, or whose address is taken.
fn used_names(asm: &str) -> HashSet<String> {
    let mut used = HashSet::new();
    for line in asm.lines() {
        let t = line.trim_start();
        if t.starts_with(';') || label_of(line).is_some() {
            continue;
        }
        let code = t.split(';').next().unwrap_or("");
        for word in code.split(|c: char| !(c == '_' || c.is_ascii_alphanumeric())) {
            if !word.is_empty() {
                used.insert(word.to_string());
            }
        }
    }
    used
}

fn label_of(line: &str) -> Option<&str> {
    if line.starts_with([' ', '\t', ';', '.']) {
        return None;
    }
    let name = line.trim_end().strip_suffix(':')?;
    is_name(name).then_some(name)
}

/// What the registers hold.
struct State<'a> {
    regs: [Vec<Val>; 8],
    /// Bytes pushed since the start of the code followed.
    depth: i64,
    next_same: u32,
    volatiles: &'a Volatiles,
}

impl State<'_> {
    fn forget(&mut self) {
        for r in &mut self.regs {
            r.clear();
        }
        self.depth = 0;
    }

    fn place(&self, text: &str) -> Option<Place> {
        let text = text.trim();
        if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            let (base, n) = bracket(inner)?;
            return match base.as_str() {
                "sp" => Some(Place::Stack(n - self.depth)),
                "x:y" => Some(Place::Xy(n)),
                _ => None,
            };
        }
        let (name, n) = symbol(text)?;
        Some(Place::Sym(name, n))
    }

    /// The register r gets a new value, equal to `vals`.
    fn set(&mut self, r: usize, mut vals: Vec<Val>) {
        if r == X || r == Y {
            self.forget_where(|p| matches!(p, Place::Xy(_)));
            vals.retain(|v| !matches!(v, Val::Mem(Place::Xy(_))));
        }
        self.regs[r] = vals;
    }

    fn forget_where(&mut self, gone: impl Fn(&Place) -> bool) {
        for r in &mut self.regs {
            r.retain(|v| !matches!(v, Val::Mem(p) if gone(p)));
        }
    }

    /// Memory at `place` (None: anywhere) is written.
    fn written(&mut self, place: Option<&Place>) {
        match place {
            Some(Place::Sym(..)) | Some(Place::Stack(_)) => {
                let place = place.unwrap().clone();
                self.forget_where(|p| *p == place || matches!(p, Place::Xy(_)));
            }
            _ => self.forget_where(|_| true),
        }
    }

    /// SP went up: the slots below it may be overwritten by an interrupt.
    fn popped(&mut self) {
        let floor = -self.depth;
        self.forget_where(|p| matches!(p, Place::Stack(k) if *k < floor));
    }

    fn knows(&self, r: usize, val: &Val) -> bool {
        self.regs[r].contains(val)
    }

    /// Follows one instruction; false when it can be removed.
    fn step(&mut self, mnemonic: &str, ops: &[String]) -> bool {
        let reg_op = |i: usize| ops.get(i).and_then(|o| reg(o));
        match (mnemonic, ops.len()) {
            ("mov", 2) => {
                let Some(dst) = reg_op(0) else {
                    self.forget();
                    return true;
                };
                if let Some(src) = reg_op(1) {
                    if self.regs[dst].iter().any(|v| self.regs[src].contains(v)) {
                        return false;
                    }
                    if self.regs[src].is_empty() {
                        let same = Val::Same(self.next_same);
                        self.next_same += 1;
                        self.regs[src].push(same);
                    }
                    let vals = self.regs[src].clone();
                    self.set(dst, vals);
                    return true;
                }
                match constant(&ops[1]) {
                    Some(k) => {
                        let k = Val::Const(k);
                        if self.knows(dst, &k) {
                            return false;
                        }
                        self.set(dst, vec![k]);
                    }
                    None => self.set(dst, Vec::new()),
                }
                true
            }
            ("lod", 2) => {
                let Some(dst) = reg_op(0) else {
                    self.forget();
                    return true;
                };
                match self.place(&ops[1]).filter(|p| !self.volatiles.of(p)) {
                    Some(p) => {
                        let m = Val::Mem(p);
                        if self.knows(dst, &m) {
                            return false;
                        }
                        self.set(dst, vec![m]);
                    }
                    None => self.set(dst, Vec::new()),
                }
                true
            }
            ("str", 2) => {
                let place = self.place(&ops[0]);
                self.written(place.as_ref());
                if let (Some(p), Some(src)) = (place, reg_op(1))
                    && !self.volatiles.of(&p)
                    && !(matches!(p, Place::Xy(_)) && (src == X || src == Y))
                {
                    self.regs[src].push(Val::Mem(p));
                }
                true
            }
            (
                "add" | "adc" | "sub" | "sbb" | "and" | "or" | "xor" | "not" | "shl" | "shr"
                | "asr" | "inc" | "dec" | "in",
                1 | 2,
            ) => {
                match reg_op(0) {
                    Some(dst) => self.set(dst, Vec::new()),
                    None => {
                        let place = self.place(&ops[0]);
                        self.written(place.as_ref());
                    }
                }
                true
            }
            ("psh", n) if n > 0 => {
                for op in ops {
                    let Some(r) = reg(op) else {
                        self.forget();
                        return true;
                    };
                    self.depth += 1;
                    // the byte is now at [sp]
                    let slot = Place::Stack(-self.depth);
                    if !self.volatiles.of(&slot) {
                        self.regs[r].push(Val::Mem(slot));
                    }
                }
                true
            }
            ("pop", n) if n > 0 => {
                for op in ops.iter().rev() {
                    let Some(r) = reg(op) else {
                        self.forget();
                        return true;
                    };
                    // the register that pushed the byte (if it still holds
                    // it) and r become equal; the slot is forgotten below
                    let slot = Val::Mem(Place::Stack(-self.depth));
                    let vals = match (0..REGS.len()).find(|&s| self.regs[s].contains(&slot)) {
                        Some(s) => {
                            if self.regs[s]
                                .iter()
                                .all(|v| matches!(v, Val::Mem(Place::Stack(_))))
                            {
                                let same = Val::Same(self.next_same);
                                self.next_same += 1;
                                self.regs[s].push(same);
                            }
                            self.regs[s].clone()
                        }
                        None => Vec::new(),
                    };
                    self.depth -= 1;
                    self.set(r, vals);
                    self.popped();
                }
                true
            }
            ("asp" | "ssp", 1) => {
                let Some(n) = number(&ops[0]) else {
                    self.forget();
                    return true;
                };
                if mnemonic == "asp" {
                    self.depth -= n;
                    self.popped();
                } else {
                    self.depth += n;
                }
                true
            }
            ("psf", 0) => {
                self.depth += 1;
                true
            }
            ("pof", 0) => {
                self.depth -= 1;
                self.popped();
                true
            }
            ("rsp", 1) => {
                match ops[0].trim().to_ascii_lowercase().as_str() {
                    "[a:b]" => {
                        self.set(0, Vec::new());
                        self.set(1, Vec::new());
                    }
                    "[x:y]" => {
                        self.set(X, Vec::new());
                        self.set(Y, Vec::new());
                    }
                    _ => self.forget(),
                }
                true
            }
            ("cmp" | "out" | "jmc" | "jme" | "jmn" | "jms" | "jmz" | "jmf", _)
            | (
                "nop" | "clc" | "cle" | "cli" | "cln" | "cls" | "clz" | "clf" | "stc" | "ste"
                | "sti" | "stn" | "sts" | "stz" | "stf",
                0,
            ) => true,
            // cal, jmp, ret, irt, hlt, int, wsp and anything else
            _ => {
                self.forget();
                true
            }
        }
    }
}

/// Removes the reloads of a compiled file (see the module documentation).
pub(crate) fn remove_reloads(asm: &str) -> String {
    let volatiles = Volatiles::read(asm);
    if volatiles.all {
        return asm.to_string();
    }
    let used = used_names(asm);
    let mut state = State {
        regs: Default::default(),
        depth: 0,
        next_same: 0,
        volatiles: &volatiles,
    };
    let mut out = String::with_capacity(asm.len());
    for line in asm.lines() {
        let keep = if let Some(label) = label_of(line) {
            // reached from elsewhere: a jump, a call, another file
            if used.contains(label) || !label.starts_with("__") {
                state.forget();
            }
            true
        } else if line.starts_with([' ', '\t']) && !line.trim_start().starts_with(';') {
            match instruction(line) {
                Some((mnemonic, ops)) => state.step(&mnemonic, &ops),
                None => {
                    state.forget();
                    true
                }
            }
        } else {
            true // comments, .data, .bss
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::remove_reloads;

    /// The instructions kept, without tabs, one per line.
    fn run(asm: &str) -> String {
        let text: String = asm
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| {
                if l.ends_with(':') || l.starts_with(';') || l.starts_with('.') {
                    format!("{l}\n")
                } else {
                    format!("\t{l}\n")
                }
            })
            .collect();
        remove_reloads(&text)
            .lines()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn store_then_load_across_statements() {
        let out = run("
_f:
str _x, b
;#line 3
lod b, _x
lod a, _x + 1
lod a, _x + 1
mov c, 0
mov c, 0
ret
");
        assert_eq!(
            out,
            "_f:\nstr _x, b\n;#line 3\nlod a, _x + 1\nmov c, 0\nret"
        );
    }

    #[test]
    fn stack_slots_follow_sp() {
        let out = run("
_f:
lod b, [sp + 2]
psh a
lod b, [sp + 3]
lod b, [sp + 2]
pop a
lod b, [sp + 2]
asp 2
lod b, [sp + 0]
");
        // same slot after the push; [sp + 2] after the push is another one;
        // after asp 2, [sp + 0] is the first slot again
        assert_eq!(
            out,
            "_f:\nlod b, [sp + 2]\npsh a\nlod b, [sp + 2]\npop a\nlod b, [sp + 2]\nasp 2"
        );
    }

    #[test]
    fn popped_slots_are_forgotten() {
        // after the pop, the slot is below SP: d must not trust it, while
        // b and a are known equal
        let out = run("
_f:
psh a
lod d, [sp + 0]
pop b
psh c
lod d, [sp + 0]
mov a, b
");
        assert_eq!(
            out,
            "_f:\npsh a\nlod d, [sp + 0]\npop b\npsh c\nlod d, [sp + 0]"
        );
    }

    #[test]
    fn pop_gets_the_pushed_value() {
        let out = run("
_f:
lod b, _v
psh b, a
pop d, c
lod d, _v
mov c, a
");
        assert_eq!(out, "_f:\nlod b, _v\npsh b, a\npop d, c");
    }

    #[test]
    fn stores_forget_what_they_may_overwrite() {
        let out = run("
_f:
lod a, _x
lod b, [x:y + 1]
lod c, [sp + 4]
str _y, d
lod a, _x
lod b, [x:y + 1]
lod c, [sp + 4]
str [x:y + 0], d
lod a, _x
lod c, [sp + 4]
");
        // a store to _y can't change _x or a stack slot, it may change [x:y]
        assert_eq!(
            out,
            "_f:\nlod a, _x\nlod b, [x:y + 1]\nlod c, [sp + 4]\nstr _y, d\nlod b, [x:y + 1]\n\
             str [x:y + 0], d\nlod a, _x\nlod c, [sp + 4]"
        );
    }

    #[test]
    fn pointer_changes_forget_xy() {
        let out = run("
_f:
lod b, [x:y + 1]
add y, 1
lod b, [x:y + 1]
lod x, [x:y + 0]
lod b, [x:y + 1]
");
        assert_eq!(
            out,
            "_f:\nlod b, [x:y + 1]\nadd y, 1\nlod b, [x:y + 1]\nlod x, [x:y + 0]\nlod b, [x:y + 1]"
        );
    }

    #[test]
    fn labels_calls_and_jumps_forget() {
        let out = run("
_f:
lod b, _x
__L1_1:
lod b, _x
__L1_2:
lod b, _x
cal _g
lod b, _x
jmp __L1_1
lod b, _x
_h:
lod b, _x
");
        // __L1_2 is never jumped to: nothing forgotten there
        assert_eq!(
            out,
            "_f:\nlod b, _x\n__L1_1:\nlod b, _x\n__L1_2:\ncal _g\nlod b, _x\njmp __L1_1\nlod b, _x\n_h:\nlod b, _x"
        );
    }

    #[test]
    fn conditional_jumps_keep_and_alu_changes() {
        let out = run("
_f:
lod b, _x
jmz __L1_9
lod b, _x
add b, 1
lod b, _x
mov a, b
mov a, b
add a, 1
mov a, b
__L1_9:
");
        assert_eq!(
            out,
            "_f:\nlod b, _x\njmz __L1_9\nadd b, 1\nlod b, _x\nmov a, b\nadd a, 1\nmov a, b\n__L1_9:"
        );
    }

    #[test]
    fn constants_and_their_forms() {
        let out = run("
_f:
mov a, 0
mov a, 0x00
mov b, __T1_3.msb
mov b, __T1_3.msb
mov b, __T1_3.lsb
mov c, 255
mov c, -1
");
        assert_eq!(
            out,
            "_f:\nmov a, 0\nmov b, __T1_3.msb\nmov b, __T1_3.lsb\nmov c, 255"
        );
    }

    #[test]
    fn volatile_memory_is_reloaded() {
        let asm = "
_f:
lod b, _count
lod b, _count
str _flag, b
lod b, _flag
lod c, [sp + 1]
lod c, [sp + 1]
;volatile _count
;volatile stack
";
        assert_eq!(
            run(asm),
            "_f:\nlod b, _count\nlod b, _count\nstr _flag, b\nlod c, [sp + 1]\nlod c, [sp + 1]\n;volatile _count\n;volatile stack"
        );
        let all = format!("{asm};volatile *\n");
        assert!(run(&all).contains("str _flag, b\nlod b, _flag"));
    }

    #[test]
    fn memory_written_by_alu_and_unknown_lines() {
        let out = run("
_f:
lod b, _x
inc _x
lod b, _x
mov a, 'A'
lod b, _x
");
        assert_eq!(
            out,
            "_f:\nlod b, _x\ninc _x\nlod b, _x\nmov a, 'A'\nlod b, _x"
        );
    }
}
