// Second half of generate.rs (included by generate.rs): rules over whole programs,
// placement, outputs.

// ----- Rules over whole programs

impl Gen {
    fn id_of(&self, seq: &str, label: &str) -> WordId {
        let p = self.seqs[seq];
        let i = self.progs[p].insts.iter().position(|i| i.label.as_deref() == Some(label)).unwrap_or(0);
        (p, i)
    }

    /// Predecessors of each step inside its program.
    fn preds(&self, p: usize) -> Vec<Vec<usize>> {
        let n = self.progs[p].insts.len();
        let mut preds = vec![Vec::new(); n];
        for (i, inst) in self.progs[p].insts.iter().enumerate() {
            if let Rx::Local(k) = inst.rx {
                let (a, b) = match k {
                    Key::Both(w) => (w, w),
                    Key::Pair(_, a, b) => (a, b),
                };
                for w in [a, b] {
                    if w.0 == p && !preds[w.1].contains(&i) {
                        preds[w.1].push(i);
                    }
                }
            }
        }
        preds
    }

    fn check_programs(&mut self, checks: &mut Vec<(String, usize)>) {
        // Every opcode × mode: a block exactly for the listed modes.
        let mut valid = 0;
        for info in INSTRUCTION_TABLE.iter() {
            for m in AddrMode::ALL {
                let listed = info.opcode.accepts(m);
                let defined = self.instr.contains_key(&(info.opcode as u8, m as u8));
                if listed && !defined {
                    self.errors.push(format!("{} {}: listed in isa.rs but no instr block", info.mnemonic, mode_name(m as u8)));
                }
                if defined && !listed {
                    self.errors.push(format!("{} {}: instr block for a mode isa.rs doesn't list", info.mnemonic, mode_name(m as u8)));
                }
                valid += listed as usize;
            }
        }
        checks.push((
            format!("opcode × mode rows covered (64 × 16): {valid} valid modes, invalid opcodes as NOP, the rest stop"),
            1024,
        ));

        // Flags as specified (isa.rs Opcode::info().flags).
        let mut flag_checked = 0;
        for info in INSTRUCTION_TABLE.iter() {
            let op = info.opcode;
            let mut mask = 0u8;
            for m in AddrMode::ALL {
                if let Some(&p) = self.instr.get(&(op as u8, m as u8)) {
                    for inst in &self.progs[p].insts {
                        mask |= inst.info.flags;
                    }
                }
            }
            if mask != flags_of(op) {
                self.errors.push(format!(
                    "{}: the microcode changes flags {} but isa.rs lists {}",
                    info.mnemonic,
                    flag_names(mask),
                    flag_names(flags_of(op))
                ));
            }
            flag_checked += 1;
        }
        checks.push(("opcodes whose flag changes equal isa.rs's list".into(), flag_checked));

        // C18: the fetch reads exactly the instruction's length, in order,
        // and no field is used before its byte is loaded.
        let mut c18 = 0;
        let stop2 = self.seqs["stop2"];
        let mut cases: Vec<(String, usize, usize, bool)> = self
            .instr
            .iter()
            .map(|(&(op, m), &p)| {
                let opcode = Opcode::from_bits(op).unwrap();
                let mode = AddrMode::from_bits(m).unwrap();
                let len = if opcode.takes_operands() { mode.length() as usize } else { 1 };
                (self.progs[p].name.clone(), p, len, opcode.takes_operands())
            })
            .collect();
        cases.push(("stop2".into(), stop2, 2, true));
        cases.sort();
        for (name, p, len, operands) in cases {
            let order = [IrByte::B1, IrByte::V1, IrByte::V2];
            let mut loaded = [true, false, false, false];
            for (i, inst) in self.progs[p].insts.iter().enumerate() {
                let inf = inst.info;
                let line = inst.line;
                if i + 1 < len {
                    if inf.fetch != Some(order[i]) {
                        self.errors.push(format!("{name}: line {line}: step {} must fetch {:?} (length {len})", i + 1, order[i]));
                    }
                } else if inf.fetch.is_some() {
                    self.errors.push(format!("{name}: line {line}: fetch past the instruction's length {len}"));
                }
                if inf.loads[0] || (inf.loads[1] && i + 1 >= len) {
                    self.errors.push(format!("{name}: line {line}: loads IR byte 0 / 1 outside the fetch"));
                }
                if (inf.loads[2] || inf.loads[3]) && inf.fetch.is_none() && operands {
                    self.errors.push(format!("{name}: line {line}: V1 / V2 loaded outside the fetch by an opcode with operands"));
                }
                if inf.uses_b1 && !loaded[1] || inf.uses_v1 && !loaded[2] || inf.uses_v2 && !loaded[3] {
                    self.errors.push(format!("{name}: line {line}: uses an IR field before (or without) loading it"));
                }
                for b in 0..4 {
                    loaded[b] |= inf.loads[b];
                }
            }
            if self.progs[p].insts.len() + 1 < len {
                self.errors.push(format!("{name}: shorter than its fetch"));
            }
            c18 += 1;
        }
        checks.push(("programs checked for C18 (fetch = length, fields used only once loaded)".into(), c18));

        // ALU result used only right after a full ALU step (C1, C9).
        let mut alu_uses = 0;
        for p in 0..self.progs.len() {
            let preds = self.preds(p);
            for (i, inst) in self.progs[p].insts.iter().enumerate() {
                if !inst.info.reads_res {
                    continue;
                }
                let mut pl: Vec<Info> = preds[i].iter().map(|&j| self.progs[p].insts[j].info).collect();
                if p == self.seqs["boot"] && i == 0 {
                    let s1 = self.id_of("reset", "S1");
                    pl.push(self.progs[s1.0].insts[s1.1].info);
                }
                if pl.is_empty() || pl.iter().any(|x| !x.alu_step()) {
                    self.errors.push(format!(
                        "{}: line {}: bus<-res, but a step before it is not a full ALU step (alu=, no register write)",
                        self.progs[p].name, inst.line
                    ));
                }
                alu_uses += 1;
            }
        }
        checks.push(("result latch uses, each right after a full ALU step".into(), alu_uses));
        let steps: usize = self.progs.iter().map(|p| p.insts.len()).sum();
        checks.push(("steps lowered with the per-step rules (one source per bus, C5, C11, C19, P4, C1, C9)".into(), steps));
    }
}

fn flag_names(mask: u8) -> String {
    let s: Vec<&str> = (0..8).filter(|b| mask & 1 << b != 0).map(|b| source::FLAG_NAMES[b]).collect();
    if s.is_empty() { "none".into() } else { s.join(" ") }
}

fn trap_word() -> Word {
    let mut t = Word::idle();
    t.set(word::FLAGS_OP, 0b1111);
    t.set(word::FLAGS_BIT_VAL, 1);
    t.set(word::CU_FORCE, 1);
    t.set(word::CU_NEXT, 1);
    t.set(word::CU_COND_SEL, cond::NOINT);
    t
}

// ----- Placement

impl Gen {
    fn put(&mut self, row: usize, step: u8, c: usize, p: Placed) {
        match self.slots[row][step as usize][c] {
            None => self.slots[row][step as usize][c] = Some(p),
            Some(old) if old == p => {}
            Some(old) => self.errors.push(format!(
                "row {:02X}/{:X} step {step} cond {c}: {} and {} both want it",
                row >> 4,
                row & 15,
                self.progs[old.id.0].name,
                self.progs[p.id.0].name
            )),
        }
    }

    /// Allocates steps in `rows` from the pinned keys, places every word
    /// reached, and returns the key → step map.
    fn alloc(&mut self, rows: &[usize], pinned: &[(Key, u8)], reserved: &[u8], forced: bool) -> HashMap<Key, u8> {
        let mut map: HashMap<Key, u8> = HashMap::new();
        let mut used: HashSet<u8> = reserved.iter().copied().collect();
        let mut q = VecDeque::new();
        for &(k, s) in pinned {
            map.insert(k, s);
            used.insert(s);
            q.push_back(k);
        }
        while let Some(k) = q.pop_front() {
            let s = map[&k];
            let (w1, w0) = match k {
                Key::Both(w) => (w, w),
                Key::Pair(_, a, b) => (a, b),
            };
            for (c, w) in [(1usize, w1), (0, w0)] {
                let inst_rx = self.progs[w.0].insts[w.1].rx;
                let word_forced = self.progs[w.0].forced;
                let (next, force, cond_sel) = match inst_rx {
                    Rx::End => (1, true, cond::NOINT),
                    Rx::Dispatch => (1, false, cond::NOINT),
                    Rx::Restart => (0, false, cond::XN),
                    Rx::Local(nk) => {
                        let cs = match nk {
                            Key::Both(_) => cond::JC,
                            Key::Pair(c, _, _) => cond_code(c),
                        };
                        if word_forced && !forced {
                            match self.forced_map.get(&nk) {
                                Some(&st) => (st, true, cs),
                                None => {
                                    self.errors.push(format!("{}: a forced-row target was not placed", self.progs[w.0].name));
                                    (0, true, cs)
                                }
                            }
                        } else {
                            let st = match map.get(&nk) {
                                Some(&st) => st,
                                None => match (0..16u8).find(|x| !used.contains(x)) {
                                    Some(st) => {
                                        used.insert(st);
                                        map.insert(nk, st);
                                        q.push_back(nk);
                                        st
                                    }
                                    None => {
                                        self.errors.push(format!("{}: out of steps (16 per row)", self.progs[w.0].name));
                                        return map;
                                    }
                                },
                            };
                            (st, forced, cs)
                        }
                    }
                };
                for &r in rows {
                    let next = if inst_rx == Rx::Restart && r == BOOT_ROW { self.boot_first } else { next };
                    self.put(r, s, c, Placed { id: w, next, force, cond_sel });
                }
            }
        }
        map
    }

    fn place(&mut self) {
        let f0 = self.id_of("forced", "F0");
        let w1 = self.id_of("forced", "W1");
        let boundary = Key::Pair(Cond::NoInt, f0, w1);
        // the forced row first: other rows jump into it
        self.forced_map = self.alloc(&[FORCED_ROW], &[(boundary, 1)], &[0], true);
        // the boot row's own steps (S1 there goes to the first one)
        let boot = self.seqs["boot"];
        self.boot_first = 2;
        self.alloc(&[BOOT_ROW], &[(Key::Both((boot, 0)), 2)], &[0, 1], false);
        // step 0 of every row: the reset word and S1
        let reset = self.id_of("reset", "reset");
        let s1 = self.id_of("reset", "S1");
        let reset_key = Key::Pair(Cond::Xn, s1, reset);
        if self.progs[reset.0].insts[reset.1].rx != Rx::Local(reset_key) {
            self.errors.push("the reset word must end with `if xn S1 else reset`".into());
        }
        let all: Vec<usize> = (0..1024).collect();
        self.alloc(&all, &[(reset_key, 0)], &[], false);

        let stop1 = self.seqs["stop1"];
        let stop2 = self.seqs["stop2"];
        for op in 0..64u8 {
            let rows = |g: u8| -> Vec<usize> { (0..4).map(|j| ((op as usize) << 4) | (g as usize * 4 + j)).collect() };
            let Some(opcode) = Opcode::from_bits(op) else {
                // invalid opcode: a 1-byte NOP whatever the mode bits
                let r: Vec<usize> = (0..16).map(|m| ((op as usize) << 4) | m).collect();
                self.alloc(&r, &[(boundary, 1)], &[0], false);
                continue;
            };
            if !opcode.takes_operands() {
                for g in 0..4u8 {
                    if g > 0 {
                        self.alloc(&rows(g), &[(Key::Both((stop1, 0)), 1)], &[0], false);
                    } else if opcode == Opcode::Nop {
                        self.alloc(&rows(0), &[(boundary, 1)], &[0], false);
                    } else {
                        let p = self.instr[&(op, 0)];
                        self.alloc(&rows(0), &[(Key::Both((p, 0)), 1)], &[0], false);
                    }
                }
                continue;
            }
            for g in 0..4u8 {
                let mut group_cond: Option<u8> = None;
                let mut f1_word: Option<u64> = None;
                let mut entries = Vec::new();
                for j in 0..4u8 {
                    let m = g * 4 + j;
                    let p = self.instr.get(&(op, m)).copied().unwrap_or(stop2);
                    let first = self.progs[p].insts[0].clone();
                    let Rx::Local(k) = first.rx else {
                        self.errors.push(format!("{}: the byte-1 fetch must lead to a step of the instruction", self.progs[p].name));
                        continue;
                    };
                    if let Key::Pair(c, _, _) = k {
                        let cc = cond_code(c);
                        if group_cond.is_some_and(|x| x != cc) {
                            self.errors.push(format!("{}: step 1 branches on two conditions in one m3 m2 group", opcode.mnemonic()));
                        }
                        group_cond = Some(cc);
                    }
                    let bits = first.word.controls();
                    if f1_word.is_some_and(|x| x != bits) {
                        self.errors.push(format!(
                            "{} modes {:X}-{:X}: step 1 differs between the modes of one m3 m2 group (m1 m0 aren't known yet)",
                            opcode.mnemonic(),
                            g * 4,
                            g * 4 + 3
                        ));
                    }
                    f1_word = Some(bits);
                    let row = ((op as usize) << 4) | m as usize;
                    self.alloc(&[row], &[(k, 2)], &[0, 1], false);
                    entries.push((row, p));
                }
                for (row, p) in entries {
                    let pl = Placed { id: (p, 0), next: 2, force: false, cond_sel: group_cond.unwrap_or(cond::JC) };
                    self.put(row, 1, 0, pl);
                    self.put(row, 1, 1, pl);
                }
            }
        }
    }

    fn word_at(&self, p: Placed) -> Word {
        let mut w = self.progs[p.id.0].insts[p.id.1].word;
        w.set(word::CU_NEXT, p.next);
        w.set(word::CU_FORCE, p.force as u8);
        w.set(word::CU_COND_SEL, p.cond_sel);
        w
    }

    /// Every word's possible next addresses hold a word.
    fn check_reachable(&mut self, checks: &mut Vec<(String, usize)>) {
        let mut n = 0;
        for row in 0..1024 {
            for s in 0..16 {
                for c in 0..2 {
                    let Some(p) = self.slots[row][s][c] else { continue };
                    let w = self.word_at(p);
                    let rows: Vec<usize> = if p.force {
                        vec![FORCED_ROW]
                    } else if self.progs[p.id.0].insts[p.id.1].rx == Rx::Restart {
                        vec![BOOT_ROW]
                    } else if w.get(word::IR_B0_LD_N) == 0 {
                        (0..1024).collect()
                    } else if w.get(word::IR_B1_LD_N) == 0 {
                        (0..4).map(|j| (row & !3) | j).collect()
                    } else {
                        vec![row]
                    };
                    for r in rows {
                        for cc in 0..2 {
                            if self.slots[r][p.next as usize][cc].is_none() {
                                self.errors.push(format!(
                                    "{} (line {}) can go to row {:02X}/{:X} step {} cond {cc}, which is empty",
                                    self.progs[p.id.0].name,
                                    self.progs[p.id.0].insts[p.id.1].line,
                                    r >> 4,
                                    r & 15,
                                    p.next
                                ));
                                return;
                            }
                        }
                    }
                    n += 1;
                }
            }
        }
        checks.push(("placed words whose every next address holds a word".into(), n));
    }

    /// Steps from a program step to its instruction's end (min, max).
    fn length(&self, id: WordId, depth: u32) -> (u32, u32) {
        if depth > 64 {
            return (1000, 1000);
        }
        let inst = &self.progs[id.0].insts[id.1];
        let tail = match inst.rx {
            Rx::End | Rx::Dispatch | Rx::Restart => (0, 0),
            Rx::Local(k) => {
                let (a, b) = match k {
                    Key::Both(w) => (w, w),
                    Key::Pair(_, a, b) => (a, b),
                };
                let f = |w: WordId| {
                    if self.progs[w.0].forced && !self.progs[id.0].forced { (0, 0) } else { self.length(w, depth + 1) }
                };
                let (x, y) = (f(a), f(b));
                (x.0.min(y.0), x.1.max(y.1))
            }
        };
        (1 + tail.0, 1 + tail.1)
    }

    fn finish(self, checks: Vec<(String, usize)>) -> Microcode {
        let mut words = vec![trap_word(); word::WORDS];
        let mut origin = vec![None; word::WORDS];
        for row in 0..1024 {
            for s in 0..16 {
                for c in 0..2 {
                    if let Some(p) = self.slots[row][s][c] {
                        let a = (row << 5) | (s << 1) | c;
                        words[a] = self.word_at(p);
                        origin[a] = Some(p.id);
                    }
                }
            }
        }
        let mut cycles = HashMap::new();
        let stop1 = self.seqs["stop1"];
        let stop2 = self.seqs["stop2"];
        for op in 0..64u8 {
            for m in 0..16u8 {
                let c = match Opcode::from_bits(op) {
                    None => (1, 1),
                    Some(Opcode::Nop) if m < 4 => (1, 1),
                    Some(o) => {
                        let p = match self.instr.get(&(op, m)) {
                            Some(&p) => p,
                            None if !o.takes_operands() && m < 4 => self.instr[&(op, 0)],
                            None if !o.takes_operands() => stop1,
                            None => stop2,
                        };
                        let (a, b) = self.length((p, 0), 0);
                        (1 + a, 1 + b)
                    }
                };
                cycles.insert((op, m), c);
            }
        }
        // interrupt entries: W1 .. IE8, after the boundary step
        let walk = |g: &Gen, mut id: WordId, pick1: &dyn Fn(Cond) -> bool| -> u32 {
            let mut n = 0;
            loop {
                n += 1;
                match g.progs[id.0].insts[id.1].rx {
                    Rx::Local(Key::Both(w)) => id = w,
                    Rx::Local(Key::Pair(c, a, b)) => id = if pick1(c) { a } else { b },
                    _ => return n,
                }
                if n > 64 {
                    return n;
                }
            }
        };
        let w1 = self.id_of("forced", "W1");
        let entry_hw = walk(&self, w1, &|_| true);
        let entry_sw = walk(&self, w1, &|c| c != Cond::Hwq && c != Cond::Xn);
        let boot = self.seqs["boot"];
        let boot_len = self.progs[boot].insts.len() as u64;
        // S1 twice, the boot row's steps once (the copy loop counted apart),
        // the copy loop 65,537 times
        let boot_steps = 2 + (boot_len - 1) + 65_537;
        Microcode { words, progs: self.progs, origin, cycles, entry_hw, entry_sw, boot_steps, checks }
    }
}

// ----- Outputs

impl Microcode {
    pub fn images(&self) -> Vec<Vec<u8>> {
        word::images(&self.words)
    }

    /// Every address holding the step labelled `label` of seq `seq`.
    pub fn addresses_of(&self, seq: &str, label: &str) -> Vec<usize> {
        let Some(p) = self.progs.iter().position(|p| p.name == seq) else { return Vec::new() };
        let Some(i) = self.progs[p].insts.iter().position(|i| i.label.as_deref() == Some(label)) else {
            return Vec::new();
        };
        (0..word::WORDS).filter(|&a| self.origin[a] == Some((p, i))).collect()
    }

    /// A model running this microcode, with the interrupt entry marked.
    pub fn model(&self) -> crate::model::Model {
        let mut m = crate::model::Model::new(&self.images());
        m.entry_marks = [self.addresses_of("forced", "IE1h"), self.addresses_of("forced", "IE1s")].concat();
        m
    }

    /// One line per used address, rows that are identical copies merged.
    pub fn listing(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "HBC-2 microcode listing (generated by `hbc2 ucode`; do not edit)");
        let _ = writeln!(out, "address = opcode(6) mode(4) step(4) cond(1); word = 56 bits in hex, EEPROM 0 = the low byte");
        let _ = writeln!(out, "Step 0 of every row holds the reset word (cond 0) and S1 (cond 1): listed for row 00/0 only.");
        let _ = writeln!(out, "Unused addresses hold the trap word {:014X}: H := 1, then the boundary (halt).\n", trap_word().0);
        let row_text = |row: usize, from: usize| -> String {
            let mut t = String::new();
            for s in from..16 {
                for c in 0..2 {
                    let a = (row << 5) | (s << 1) | c;
                    let Some((p, i)) = self.origin[a] else { continue };
                    let inst = &self.progs[p].insts[i];
                    let w = self.words[a];
                    let _ = writeln!(
                        t,
                        "  {s:2}.{c} {:014X}  {:<12} line {:>3}{:<6}  next {:2} {}{:<5}  {}",
                        w.0,
                        self.progs[p].name,
                        inst.line,
                        inst.label.as_ref().map(|l| format!(" {l}")).unwrap_or_default(),
                        w.get(word::CU_NEXT),
                        if w.bit(word::CU_FORCE) { "forced " } else { "" },
                        ["jc", "noint", "hwq", "xn"][w.get(word::CU_COND_SEL) as usize],
                        w.describe_controls(),
                    );
                }
            }
            t
        };
        let texts: Vec<String> = (0..1024).map(|r| row_text(r, 1)).collect();
        let name = |r: usize| {
            let op = (r >> 4) as u8;
            format!(
                "{:02X}/{:X} ({} {})",
                op,
                r & 15,
                Opcode::from_bits(op).map(|o| o.mnemonic()).unwrap_or("invalid"),
                mode_name((r & 15) as u8)
            )
        };
        let mut row = 0;
        while row < 1024 {
            let mut end = row + 1;
            while row != 0 && end < 1024 && texts[end] == texts[row] {
                end += 1;
            }
            if end - row > 1 {
                let _ = writeln!(out, "rows {} .. {}", name(row), name(end - 1));
            } else {
                let _ = writeln!(out, "row {}", name(row));
            }
            out.push_str(&if row == 0 { row_text(0, 0) } else { texts[row].clone() });
            row = end;
        }
        out
    }

    /// The cycle-count table (markdown), per instruction × mode.
    pub fn cycle_table(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Control unit: cycle counts\n");
        let _ = writeln!(
            out,
            "Generated by `hbc2 ucode` from `hbc2/crates/hbc2-ucode/microcode/hbc2.uc`; do not edit. Steps of 500 ns (2 MHz) per instruction, from its fetch of byte 0 (F0) to the next instruction's F0. Checked against the datapath model by `cargo test -p hbc2-ucode`. \"a / b\": jump not taken / taken.\n"
        );
        let modes: Vec<u8> = (0..9).collect();
        let _ = write!(out, "| Instruction |");
        for m in &modes {
            let _ = write!(out, " {} |", AddrMode::from_bits(*m).unwrap().name());
        }
        let _ = writeln!(out, " other modes (stop) |");
        let _ = writeln!(out, "|---|{}---|", "---|".repeat(modes.len()));
        for info in INSTRUCTION_TABLE.iter() {
            let op = info.opcode as u8;
            let _ = write!(out, "| {} |", info.mnemonic.to_uppercase());
            for m in &modes {
                if info.opcode.accepts(AddrMode::from_bits(*m).unwrap()) {
                    let (a, b) = self.cycles[&(op, *m)];
                    if a == b {
                        let _ = write!(out, " {a} |");
                    } else {
                        let _ = write!(out, " {a} / {b} |");
                    }
                } else {
                    let _ = write!(out, " |");
                }
            }
            let stop = (0..16u8).find(|m| !accepts_bits(info.opcode, *m)).map(|m| self.cycles[&(op, m)].0);
            let _ = writeln!(out, " {} |", stop.map(|s| s.to_string()).unwrap_or_default());
        }
        let _ = writeln!(out, "| invalid opcode (36–3F), any mode | 1 |{}", " |".repeat(modes.len()));
        let _ = writeln!(
            out,
            "\nStop column: a valid opcode with a mode it doesn't accept: I := 0, H := 1, PC back on the instruction, then the halt loop.\n"
        );
        let _ = writeln!(out, "| Sequence | Steps |");
        let _ = writeln!(out, "|---|---|");
        let _ = writeln!(out, "| hardware interrupt entry, after the boundary step (W1, IE1–IE8) | {} |", self.entry_hw);
        let _ = writeln!(out, "| software interrupt entry, after the boundary step (W1, W2, IE1–IE8) | {} |", self.entry_sw);
        let _ = writeln!(out, "| halt loop (W1, W2) | 2 per check |");
        let _ = writeln!(
            out,
            "| reset released → first fetch (S1 × 2, S2–S9, 65,537 copy steps, PC − 1) | {} ({:.1} ms) |",
            self.boot_steps,
            self.boot_steps as f64 * 0.0005
        );
        out
    }
}

/// True if the mode bits `m` run for this opcode (no stop).
pub fn accepts_bits(op: Opcode, m: u8) -> bool {
    if !op.takes_operands() {
        return m < 4;
    }
    AddrMode::from_bits(m).is_some_and(|mode| op.accepts(mode))
}
