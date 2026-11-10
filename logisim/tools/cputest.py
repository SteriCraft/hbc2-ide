#!/usr/bin/env python3
"""The whole Logisim CPU against the emulator (cu-decisions.md section 15).

Runs circuits/cpu/test_cpu.circ headless (`logisim-evolution -tty table`)
from Logisim's power-up: reset, the whole boot (register sweep, 64 KiB copy
of the boot ROM into RAM), then the program. The bench prints its output
pins (main's connector, buses and test points, the IOD stub's int / vec)
after every oscillator half period, 4 rows per 500 ns step. This script
reads them as they come and:

- fails on any E (two drivers disagreeing) in any column, any row;
- checks C24: every control line at its pulled level while rst_rc is high;
- checks the boot copy: every RAM write with the ROM strobes carries the
  program's byte for that address, 65,537 writes, then the first fetch;
- writes the trace CSV (hbc2_core::trace's format) from what it sees:
  pc and the instruction bytes at the fetch steps, mem_w / io_w from the
  RAM write and OUT steps, the flags from the flag pins, and INT lines at
  the vector steps. The registers and SP are kept from their write events
  on the section ports (regs: the write index and bus_data at clk_wr_n's
  rising edge; addr: the adder's sum at the clock edge, computed from the
  ports) and checked against the hardware every time it shows them: each
  read of port A / port B (bus A / bus B), the address latch on bus_addr,
  PC / SP on bus_data, addr_cout against the computed carry (U14);
- counts the steps of every instruction (F0 to the next F0, or to W1 when
  an interrupt entry or the halt loop follows);
- for each hardware interrupt, notes the line during which INT rose and
  the port and data the stub put on the buses in the vector step: the
  emulator gets the same interrupt queued before that line (emutrace).

Then it runs `hbc2 trace` (or emutrace when hardware interrupts were taken)
on the same RAM image and `hbc2 compare`.

    python cputest.py run PROG.bin|PROG.has [--max-lines N] [--max-steps N] [--out DIR] [--rc-mutant]
    python cputest.py suite [--random N] [--jobs J] [--max-steps N]   # the test programs + N random ones
    python cputest.py mutants [PROG]        # deliberate wiring bugs in main: each must fail
    python cputest.py cycles DIR...       # steps per instruction x mode against notes/cu-cycles.md
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from collections import Counter, deque
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPO = ROOT.parent
LOGISIM = r"C:\Program Files\logisim-evolution\logisim-evolution.exe"
HBC2 = REPO / "hbc2" / "target" / "release" / "hbc2.exe"
EMUTRACE = ROOT / "tools" / "emutrace" / "target" / "release" / "emutrace.exe"
PROGRAMS = ROOT / "tests" / "cpu"
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "designs"))

BOOT_STEPS = 65548


# ---- the pulled levels (C24), from designs/cu.py's PIPE -----------------------------------------
def pulled_levels() -> dict:
    import cu as cu_design
    wide = cu_design.WIDE
    levels: dict = {}
    for q, _, level in cu_design.PIPE:
        name = q[3:]                                         # cu_<x> -> <x>
        if q in cu_design.INSIDE:
            continue
        base, _, bit = name.rpartition("_")
        if base in wide:
            levels.setdefault(base, [None] * wide[base])[int(bit)] = level
        else:
            levels[name] = [level]
    return {k: sum(b << i for i, b in enumerate(v)) for k, v in levels.items()}


# ---- rows -------------------------------------------------------------------------------------
def parse_value(tok: str, width: int):
    """(int or None, has_E)."""
    tok = tok.replace(" ", "")
    if tok.startswith("0x"):
        digits = tok[2:]
        if all(c in "0123456789abcdefABCDEF" for c in digits):
            return int(digits, 16), False
        return None, "E" in digits
    if all(c in "01" for c in tok):
        return int(tok, 2), False
    return None, "E" in tok


class Rows:
    """Reads Logisim's table rows: a dict per row, plus the columns holding an E."""

    def __init__(self, names, widths):
        self.names, self.widths = names, widths

    def parse(self, line: str):
        line = line.rstrip("\r\n")
        if not line or "[" in line or re.search(r"[g-zG-TV-Z_]", line.replace("0x", "")):
            return None                                       # the header
        toks = line.split("\t") if "\t" in line else line.split()
        if len(toks) != len(self.names):
            raise ValueError(f"row with {len(toks)} columns, expected {len(self.names)}: {line[:120]}")
        row, bad = {}, []
        for name, w, tok in zip(self.names, self.widths, toks):
            v, e = parse_value(tok, w)
            row[name] = v
            if name == "bus_addr":         # the low byte alone (the high byte floats in a vector step)
                t = tok.replace(" ", "")
                row["bus_addr_lo"] = parse_value("0x" + t[-2:] if t.startswith("0x") else t[-8:], 8)[0]
            if e:
                bad.append(name)
        return row, bad


# ---- the observer -------------------------------------------------------------------------------
REG_NAMES = "abcdijxy"


class Observer:
    def __init__(self, image: bytes, pulled: dict, max_lines: int, max_steps: int):
        self.image = image
        self.pulled = pulled
        self.max_lines, self.max_steps = max_lines, max_steps
        self.errors: list[str] = []
        self.err_count = Counter()
        self.step = -1                     # step index (rising clk edges seen)
        self.rows: list[dict] = []         # rows of the current step
        self.prev_clk = None
        self.nrows = 0
        self.pull_rows = 0
        self.contention = 0
        self.regs: list = [None] * 8
        self.pc = self.sp = self.lat = None
        self.snap = deque(maxlen=16)       # (step, regs, sp, flags) at each step's start
        self.started = False
        self.first_f0 = None
        self.rst_release = None
        self.copies = 0
        self.lines: list[str] = []
        self.cur = None                    # the line in progress
        self.writes = []                   # (step, kind, text) of the current line
        self.spans = []                    # (start step, line number)
        self.cycles = []                   # (op, mode, size, steps, next_pc, pc)
        self.int_prev = 0
        self.rises: deque = deque()
        self.inject: list[tuple] = []      # (line, port, data)
        self.hw = self.sw = 0
        self.prev_e = None
        self.halt_since = None
        self.stop_since = None
        self.done = None

    def err(self, kind, text):
        self.err_count[kind] += 1
        if self.err_count[kind] <= 5:
            self.errors.append(f"step {self.step}: {text}")

    # one row per oscillator half period
    def feed(self, row: dict, bad: list):
        self.nrows += 1
        if bad:
            self.contention += 1
            self.err("E", f"E (contention) on {', '.join(bad)}")
        if row["rst_rc"] == 1:
            self.pull_rows += 1
            for name, level in self.pulled.items():
                if row[name] != level:
                    self.err("pull", f"rst_rc high: {name} = {row[name]}, pulled level {level}")
        clk = row["clk"]
        if self.prev_clk == 0 and clk == 63:
            if self.rows:
                self.end_step()
            self.step += 1
            self.rows = []
        self.prev_clk = clk
        self.rows.append(row)

    def finish(self):
        if self.rows:
            self.end_step()

    # ---- a whole step: rows r0 (after the edge) .. r3 (before the next edge)
    def end_step(self):
        s, rows = self.step, self.rows
        if s < 0:
            return
        r0, E = rows[0], rows[-1]
        W = next((r for r in reversed(rows) if r["clk_wr_n"] == 0), E)
        flags = r0["flags"]
        self.snap.append((s, list(self.regs), self.sp, flags))
        if E["rst_n"] == 1 and self.rst_release is None:
            self.rst_release = s
        if E["rst_n"] != 1:
            self.prev_e = E
            self.write_regs(W)
            self.update(E, W)
            return

        # halt: H shows at the start of W1
        if self.started and flags is not None and flags & 0x80 and self.cur is not None and not self.cur.get("done"):
            self.close_line(s, halt=True)
        if flags is not None and flags & 0x80:
            self.halt_since = self.halt_since if self.halt_since is not None else s
        else:
            self.halt_since = None
        # stopped: H set and I clear, for good (an entry from the halt loop clears I one step before H)
        if flags is not None and flags & 0x84 == 0x80:
            self.stop_since = self.stop_since if self.stop_since is not None else s
        else:
            self.stop_since = None

        # INT from the stub: rises at a clock edge
        if r0["int"] == 1 and self.int_prev == 0:
            self.rises.append(s)
        self.int_prev = r0["int"]

        f0 = E["ir_b0_ld_n"] == 0 and E["mem_re"] == 1
        vector = E["addr_base_sel"] == 3 and E["addr_pc_ld_n"] == 0
        if f0:
            if not self.started:
                self.started = True
                self.first_f0 = s
                self.emit_line0(s)
            elif self.cur is not None and not self.cur.get("done"):
                self.close_line(s)
            self.cur = {"kind": "ins", "start": s, "pc": E["bus_addr"], "ir": [E["bus_data"]]}
            self.writes = []
            self.spans.append((s, len(self.lines)))
        elif self.started and self.cur is not None and self.cur["kind"] == "ins" and self.cur.get("fetching", True):
            # the fetch steps follow F0 back to back (F1 b1, F2 v1, F3 v2), at PC + 1, + 2, + 3;
            # later loads of V1 / V2 (RET, IRT, POF pops) are data
            k = len(self.cur["ir"])
            ld = ("ir_b1_ld_n", "ir_v1_ld_n", "ir_v2_ld_n")[k - 1] if k < 4 else None
            at = None if self.cur["pc"] is None else (self.cur["pc"] + k) & 0xFFFF
            if ld is not None and s == self.cur["start"] + k and E[ld] == 0 and E["mem_re"] == 1 and E["bus_addr"] == at:
                self.cur["ir"].append(E["bus_data"])
            else:
                self.cur["fetching"] = False
        if vector and self.started:
            hw = self.prev_e is not None and self.prev_e["inr"] == 1
            first = s - (5 if hw else 6)
            if self.cur is not None and not self.cur.get("done"):
                self.close_line(first, before=s - 4)
            keep = [w for w in self.writes if w[0] >= s - 4]
            self.cur = {"kind": "int", "start": first}
            self.writes = keep
            self.spans.append((first, len(self.lines)))
            if hw:
                self.hw += 1
                port, data = self.lo_byte(E), E["bus_data"]
                if not self.rises:
                    self.err("int", "hardware vector step without an INT rise")
                else:
                    r = self.rises.popleft()
                    line = max(n for st, n in self.spans if st <= r)
                    self.inject.append((line, port, data))
            else:
                self.sw += 1

        # memory and I/O writes
        if E["mem_we_n"] == 0:
            addr, data = E["bus_addr"], W["mem_d"]
            if E["mem_rom_oe_n"] == 0:
                self.copies += 1
                if addr is None or data is None or data != self.image[addr]:
                    self.err("copy", f"boot copy: RAM[{addr}] := {data}, ROM has "
                                     f"{None if addr is None else self.image[addr]}")
            elif self.started:
                self.writes.append((s, "m", f"{addr:04X}={data:02X}" if addr is not None and data is not None else "????"))
        if E["ie"] == 1 and E["rw"] == 0 and self.started:
            lo, data = self.lo_byte(W), W["bus_data"]
            self.writes.append((s, "i", f"{lo:02X}={data:02X}" if lo is not None and data is not None else "??"))

        self.write_regs(W)
        self.checks(E)
        self.update(E, W)
        self.prev_e = E
        if self.max_lines and len(self.lines) > self.max_lines:
            self.done = "max lines"
        elif self.max_steps and s >= self.max_steps:
            self.done = "max steps"
        elif self.halt_since is not None and flags is not None:
            if self.stop_since is not None and s - self.stop_since > 20:
                self.done = "stopped (H set, I clear)"
            elif s - self.halt_since > 300 and r0["int"] == 0:
                self.done = "halted, no interrupt to come"

    @staticmethod
    def lo_byte(row):
        return row["bus_addr_lo"]

    # ---- trace lines
    def state(self, step):
        for st, regs, sp, flags in self.snap:
            if st == step:
                return regs, sp, flags
        raise RuntimeError(f"no snapshot for step {step} (now {self.step})")

    def fmt(self, n, pc, ir, regs, sp, flags, mem_w, io_w):
        def h(v, d):
            return "?" * d if v is None else f"{v:0{d}X}"
        return ",".join([str(n), h(pc, 4), ir] + [h(r, 2) for r in regs] + [h(sp, 4), h(flags, 2), mem_w, io_w])

    def emit_line0(self, s):
        regs, sp, flags = self.state(s)
        E = self.rows[-1]
        self.lines.append(self.fmt(0, E["bus_addr"], "", regs, sp, flags, "", ""))

    def close_line(self, at, before=None, halt=False):
        """The current line ends before step `at` (its state: the one at `at`'s start)."""
        cur = self.cur
        regs, sp, flags = self.state(at)
        cut = before if before is not None else at + 10 ** 9
        ws = [w for w in self.writes if w[0] < cut]
        mem_w = ";".join(t for _, k, t in ws if k == "m")
        io_w = ";".join(t for _, k, t in ws if k == "i")
        n = len(self.lines)
        if cur["kind"] == "ins":
            ir = "".join("??" if b is None else f"{b:02X}" for b in cur["ir"])
            self.lines.append(self.fmt(n, cur["pc"], ir, regs, sp, flags, mem_w, io_w))
            b0 = cur["ir"][0]
            b1 = cur["ir"][1] if len(cur["ir"]) > 1 else None
            self.cycles.append((b0, b1, len(cur["ir"]), at - cur["start"], cur["pc"], halt))
        else:
            ms = [t for _, k, t in ws if k == "m"]
            pc = None
            if len(ms) == 3 and "?" not in "".join(ms):
                pc = int(ms[2][-2:], 16) << 8 | int(ms[1][-2:], 16)
            self.lines.append(self.fmt(n, pc, "INT", regs, sp, flags, mem_w, io_w))
        cur["done"] = True
        self.writes = [w for w in self.writes if w[0] >= cut]

    # ---- cross-checks: the kept state against what the hardware shows
    def reg_word(self, sel, row, port):
        """Port A (a) or B (b): the register word read, None if off, 'both'."""
        if sel is None:
            return "?"
        src = {"a": ("r1", "r2", "r3", "cu"), "b": ("r2", "r1", "r3", "off")}[port][sel]
        if src == "off":
            return None
        if src == "cu":
            idx, bank = row["regs_cu_idx"], row["regs_cu_bank_n"]
            if idx is None or bank is None:
                return "?"
            lo_n, hi_n = bank & 1, bank >> 1
        else:
            r, r2n = row[f"ir_{src}"], row[f"ir_{src}_2_n"]
            if r is None or r2n is None:
                return "?"
            idx, lo_n, hi_n = r & 3, r >> 2, r2n
        words = [w for w, en in ((idx, lo_n == 0), (4 + idx, hi_n == 0)) if en]
        return None if not words else words[0] if len(words) == 1 else "both"

    def checks(self, E):
        for port, sel, bus in (("a", E["regs_a_sel"], "bus_a"), ("b", E["regs_b_sel"], "bus_b")):
            w = self.reg_word(sel, E, port)
            if w == "both":
                self.err("bank", f"port {port.upper()} reads both banks")
            elif isinstance(w, int) and self.regs[w] is not None and E[bus] != self.regs[w]:
                self.err("reg", f"{bus} = {E[bus]}, register {REG_NAMES[w]} kept as {self.regs[w]}")
        lat_oe = E["addr_lat_oe_n"]
        if self.lat is not None and lat_oe is not None and E["bus_addr"] is not None:
            for byte in (0, 1):
                if not (lat_oe >> byte) & 1:
                    hw = (E["bus_addr"] >> 8 * byte) & 0xFF
                    if hw != (self.lat >> 8 * byte) & 0xFF:
                        self.err("lat", f"bus_addr byte {byte} = {hw:02X}, latch kept as {self.lat:04X}")
        for reg, val in (("pc", self.pc), ("sp", self.sp)):
            oe = E[f"addr_{reg}_oe_n"]
            if val is None or oe is None:
                continue
            for byte in (0, 1):
                if not (oe >> byte) & 1 and E["bus_data"] != (val >> 8 * byte) & 0xFF:
                    self.err(reg, f"bus_data = {E['bus_data']}, {reg.upper()} kept as {val:04X} (byte {byte})")

    def adder(self, E):
        """The address section's sum (and carry) from its ports, None if unknown."""
        sel, boe, ooe = E["addr_base_sel"], E["addr_base_oe_n"], E["addr_off_oe_n"]
        if None in (sel, boe, ooe, E["addr_off_sel"], E["addr_inv"], E["addr_cin"]):
            return None, None
        need_lo, need_hi = not boe & 1, not boe >> 1 & 1
        if sel == 0:
            base = self.pc
        elif sel == 1:
            base = self.sp
        elif sel == 2:
            a, b = E["bus_a"], E["bus_b"]
            base = None if (need_hi and a is None) or (need_lo and b is None) else ((a or 0) << 8) | (b or 0)
        else:
            p = self.lo_byte(E)
            p7 = 0 if E["rst_n"] == 0 else (None if p is None else p >> 7)
            hi = None if p7 is None else 0xFE | p7
            lo = None if p is None else (p << 1) & 0xFF
            base = None if (need_hi and hi is None) or (need_lo and lo is None) else ((hi or 0) << 8) | (lo or 0)
        if base is None and (need_lo or need_hi):
            return None, None
        base = (base or 0) & ((0xFF if need_lo else 0) | (0xFF00 if need_hi else 0))
        v1, v2 = E["ir_v1"], E["ir_v2"]
        lo = 0 if ooe & 1 else (v1 if E["addr_off_sel"] else v2)
        hi = 0 if ooe >> 1 & 1 else v1
        if lo is None or hi is None:
            return None, None
        off = (hi << 8 | lo) ^ (0xFFFF if E["addr_inv"] else 0)
        total = base + off + E["addr_cin"]
        return total & 0xFFFF, total >> 16

    def write_regs(self, W):
        """The register file: written while clk_wr_n is low (W), index = port A's."""
        if W["regs_we_n"] == 0:
            w = self.reg_word(W["regs_a_sel"], W, "a")
            for word in ([w] if isinstance(w, int) else [] if w is None else []):
                self.regs[word] = W["bus_data"]
            if w == "both":
                self.err("bank", "register write to both banks")
            if w == "?":
                self.regs = [None] * 8

    def update(self, E, W):
        # address section: the 377s load the sum at the edge
        lds = [E["addr_lat_ld_n"], E["addr_pc_ld_n"], E["addr_sp_ld_n"]]
        if 0 in lds or None in lds:
            total, cout = self.adder(E)
            if total is not None and E["addr_cout"] is not None and cout != E["addr_cout"] and E["rst_n"] == 1:
                self.err("cout", f"addr_cout = {E['addr_cout']}, the sum gives {cout}")
            if E["addr_lat_ld_n"] != 1:
                self.lat = total if E["addr_lat_ld_n"] == 0 else None
            if E["addr_pc_ld_n"] != 1:
                self.pc = None if total is None or E["addr_pc_ld_n"] is None else (
                    total if E["rst_n"] == 1 else total & 0x01FF)
            if E["addr_sp_ld_n"] != 1:
                self.sp = total if E["addr_sp_ld_n"] == 0 else None


# ---- running Logisim ---------------------------------------------------------------------------
def tree_with_rom(image: bytes, workdir: Path, cpu: Path | None = None) -> Path:
    """A copy of logisim/ (cpu.circ, circuits/) whose boot ROM holds `image`;
    `cpu` replaces cpu.circ (a mutant)."""
    shutil.copy(cpu or ROOT / "cpu.circ", workdir / "cpu.circ")
    shutil.copytree(ROOT / "circuits", workdir / "circuits", ignore=shutil.ignore_patterns("*.png", "*.pdf"))
    import at28c256 as rom_design
    text = (workdir / "circuits" / "chips" / "at28c256.circ").read_text(encoding="utf-8")
    start = text.index('<a name="contents">') + len('<a name="contents">')
    end = text.index("</a>", start)
    text = text[:start] + rom_design.contents(rom_design.rom(image)) + text[end:]
    (workdir / "circuits" / "chips" / "at28c256.circ").write_text(text, encoding="utf-8")
    return workdir / "circuits" / "cpu" / "test_cpu.circ"


def bench_columns():
    import test_cpu
    return [n for n, _ in test_cpu.OBS], [w for _, w in test_cpu.OBS]


def run_logisim(bench: Path, obs: Observer, log=None):
    names, widths = bench_columns()
    rows = Rows(names, widths)
    proc = subprocess.Popen([LOGISIM, "-t", "table", str(bench)], stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, bufsize=1 << 16)
    try:
        for line in proc.stdout:
            parsed = rows.parse(line)
            if parsed is None:
                continue
            obs.feed(*parsed)
            if obs.done:
                break
    finally:
        proc.kill()
        proc.wait()
    obs.finish()
    if obs.done is None:
        obs.done = "Logisim stopped (halt pin)"


def assemble(src: Path, out: Path):
    subprocess.run([str(HBC2), "asm", str(src), "-o", str(out)], check=True, capture_output=True)


def run(prog: Path, outdir: Path, max_lines=0, max_steps=0, name=None, cpu=None) -> dict:
    name = name or prog.stem
    outdir.mkdir(parents=True, exist_ok=True)
    if prog.suffix == ".has":
        binf = outdir / f"{name}.bin"
        assemble(prog, binf)
    else:
        binf = prog
    image = binf.read_bytes()
    t0 = time.time()
    obs = Observer(image, pulled_levels(), max_lines, max_steps)
    with tempfile.TemporaryDirectory(prefix="hbc2cpu_") as tmp:
        bench = tree_with_rom(image, Path(tmp), cpu)
        run_logisim(bench, obs)
    secs = time.time() - t0
    lg = outdir / f"{name}.logisim.csv"
    header = "n,pc,ir,a,b,c,d,i,j,x,y,sp,flags,mem_w,io_w"
    lg.write_text("\n".join([header] + obs.lines) + "\n", encoding="ascii", newline="\n")
    nlines = len(obs.lines) - 1
    em = outdir / f"{name}.emu.csv"
    if obs.inject:
        cmd = [str(EMUTRACE), "trace", str(binf), "--max", str(nlines), "-o", str(em)]
        for line, port, data in obs.inject:
            cmd += ["--int", f"{line}:{port:02X}:{data:02X}"]
        ref = "emutrace (hardware interrupts injected)"
    else:
        cmd = [str(HBC2), "trace", str(binf), "--max", str(nlines), "-o", str(em)]
        ref = "hbc2 trace"
    subprocess.run(cmd, check=True, capture_output=True)
    cmp = subprocess.run([str(HBC2), "compare", str(lg), str(em)], capture_output=True, text=True)
    result = {
        "name": name, "seconds": round(secs), "steps": obs.step + 1, "rows": obs.nrows, "end": obs.done,
        "reset_release_step": obs.rst_release, "first_fetch_step": obs.first_f0,
        "boot_steps": None if obs.first_f0 is None or obs.rst_release is None else obs.first_f0 - obs.rst_release,
        "boot_copies": obs.copies, "lines": nlines, "hw_int": obs.hw, "sw_int": obs.sw,
        "pull_rows": obs.pull_rows, "contention_rows": obs.contention, "errors": obs.errors,
        "error_counts": dict(obs.err_count), "reference": ref, "compare_exit": cmp.returncode,
        "compare": cmp.stdout.strip()[-1500:], "cycles": obs.cycles,
    }
    (outdir / f"{name}.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
    return result


def summary(r: dict) -> str:
    ok = r["compare_exit"] == 0 and not r["errors"] and r["contention_rows"] == 0 and r["lines"] > 0
    return (f"{'PASS' if ok else 'FAIL'} {r['name']}: {r['lines']} lines, {r['steps']} steps "
            f"({r['seconds']} s), boot {r['boot_steps']} steps, {r['boot_copies']} copy writes, "
            f"hw {r['hw_int']} / sw {r['sw_int']} interrupts, end: {r['end']}; ref {r['reference']}: "
            f"{r['compare'].splitlines()[0] if r['compare'] else '?'}"
            + ("" if not r["errors"] else "\n    " + "\n    ".join(r["errors"][:10])))


# ---- deliberate wiring bugs between sections (designs/cpu.py NETS) ---------------------------------
MUTANTS = [
    ("ALU carry in from Z instead of C", '"alu_cin": "flag_c"', '"alu_cin": "flag_z"'),
    ("R1 and R2 swapped into regs", '**{f"regs_{f}": f"ir_{f}" for f in ("r1", "r2", "r3", "r1_2_n", "r2_2_n", "r3_2_n")}',
     '**{f"regs_{f}": f"ir_{f}" for f in ("r3", "r1_2_n", "r2_2_n", "r3_2_n")}, "regs_r1": "ir_r2", "regs_r2": "ir_r1"'),
    ("V1 and V2 swapped into addr", '"addr_v1": "ir_v1", "addr_v2": "ir_v2"', '"addr_v1": "ir_v2", "addr_v2": "ir_v1"'),
    ("ALU N and S swapped into flags", '**{f"flags_alu_{f}": f"alu_{f}" for f in "cznsef"}',
     '**{f"flags_alu_{f}": f"alu_{f}" for f in "czef"}, "flags_alu_n": "alu_s", "flags_alu_s": "alu_n"'),
    ("cu's Z condition from flag C", '**{f"cu_{f}": f for f in FLAGS}', '**{f"cu_{f}": f for f in FLAGS if f != "flag_z"}, "cu_flag_z": "flag_c"'),
    ("pending port not on bus_addr", '"cu_bus_addr_lo": "bus_addr_lo"', '"cu_bus_addr_lo": "swi_nc"'),
    ("IVT port read from bus_data", '"addr_bus_addr_lo_in": "bus_addr_lo"', '"addr_bus_addr_lo_in": "bus_data"'),
    ("bus A into the ALU's B input", '"alu_a": "bus_a", "alu_b": "bus_b"', '"alu_a": "bus_a", "alu_b": "bus_a"'),
    ("flags' bus input from alu_r instead of bus_data", '"flags_bus_in": "bus_data"', '"flags_bus_in": "alu_r"'),
    ("INT not wired to cu", '"cu_int": "int"', '"cu_int": "int_nc"'),
]


def mutants(prog: Path, out: Path, jobs: int):
    src = (ROOT / "designs" / "cpu.py").read_text(encoding="utf-8")
    work = []
    for k, (name, a, b) in enumerate(MUTANTS):
        assert src.count(a) == 1, name
        path = ROOT / f"_mutant{k}.circ"
        code = src.replace(a, b).replace('OUT = ROOT / "cpu.circ"', f'OUT = ROOT / "{path.name}"')
        code = code[:code.index('if __name__ == "__main__":')] + "p.save(OUT, check=False)\n"
        script = ROOT / "designs" / f"_mutant{k}.py"
        script.write_text(code, encoding="utf-8")
        subprocess.run([sys.executable, str(script)], check=True, capture_output=True)
        script.unlink()
        work.append((k, name, path))
    res = []
    with ProcessPoolExecutor(jobs) as ex:
        futs = {ex.submit(run, prog, out, 0, BOOT_STEPS + 40000, f"mutant{k}", path): (k, name, path)
                for k, name, path in work}
        for f in as_completed(futs):
            k, name, path = futs[f]
            r = f.result()
            path.unlink()
            caught = r["compare_exit"] != 0 or bool(r["errors"]) or r["contention_rows"] > 0 or r["lines"] == 0
            how = (r["compare"].splitlines()[0] if r["compare_exit"] and r["compare"] else "")
            how += ("; " + r["errors"][0]) if r["errors"] else ""
            print(f"{'caught' if caught else 'MISSED'}: {name}: {how[:220]}", flush=True)
            res.append(caught)
    print(f"{sum(res)} / {len(res)} caught")
    return 0 if all(res) else 1


# ---- cycle counts against notes/cu-cycles.md ------------------------------------------------------
MNEMONICS = ("NOP ADC ADD AND CAL CLC CLE CLI CLN CLS CLZ CLF CMP DEC HLT IN OUT INC INT IRT JMC JME JMN JMP "
             "JMS JMZ JMF STR LOD MOV NOT OR POP PSH RET SHL ASR SHR STC STE STI STN STS STZ STF PSF POF SUB "
             "SBB XOR ASP SSP RSP WSP").split()
MODES = ("None", "Reg", "RegImm8", "RegRam", "RamRegImmReg", "StackOffset", "Reg16", "Imm16", "Imm8")


def cycle_table():
    """{mnemonic: [cell per mode 0..8, stop cell]}: each cell a set of accepted counts (empty: not listed)."""
    table = {}
    for line in (ROOT / "notes" / "cu-cycles.md").read_text(encoding="utf-8").splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) == 11 and cells[0] in MNEMONICS:
            table[cells[0]] = [{int(x) for x in c.split("/")} if c else set() for c in cells[1:]]
        if cells and cells[0].startswith("invalid opcode"):
            table["invalid"] = {int(cells[1])}
    return table


def classify(b0, b1, size):
    """(row, column label, column index or None) of a fetched instruction."""
    op = b0 >> 2
    if op >= len(MNEMONICS):
        return "invalid", "any", None
    mode = ((b0 & 3) << 2) | ((b1 or 0) >> 6 if size > 1 else 0)
    return MNEMONICS[op], MODES[mode] if mode < len(MODES) else f"mode {mode:X}", mode


def cycles(dirs):
    table = cycle_table()
    seen, bad = Counter(), Counter()
    examples = {}
    for d in dirs:
        for f in sorted(Path(d).glob("*.json")):
            if f.name == "suite.json":
                continue
            r = json.loads(f.read_text(encoding="utf-8"))
            if r.get("compare_exit") != 0:
                continue                       # only runs that matched the emulator
            ended = r["end"].startswith(("stopped", "halted"))
            for k, entry in enumerate(r["cycles"]):
                b0, b1, size, steps, pc = entry[:5]
                # runs written before the halt flag was recorded: the last line of a run ended by a halt
                halt = entry[5] if len(entry) > 5 else ended and k == len(r["cycles"]) - 1
                row, col, mode = classify(b0, b1, size)
                if halt and row in ("POF", "IRT"):
                    steps += 1     # H is loaded before their boundary step: it shows one step before W1
                if row == "invalid":
                    want, key = table["invalid"], ("invalid opcode", "any")
                else:
                    cells = table[row]
                    listed = mode is not None and mode < 9 and cells[mode]
                    want = cells[mode] if listed else cells[9]
                    key = (row, col if listed else "stop")
                seen[key] += 1
                if steps not in want:
                    bad[key] += 1
                    examples.setdefault(key, (f.stem, pc, steps, sorted(want)))
    listed = {(row, MODES[m]) for row, cells in table.items() if row != "invalid" for m in range(9) if cells[m]}
    stops = {(row, "stop") for row, cells in table.items() if row != "invalid" and cells[9]}
    hit = {k for k in seen if k in listed}
    print(f"instructions checked: {sum(seen.values())}, wrong counts: {sum(bad.values())}")
    print(f"instruction x mode covered: {len(hit)} / {len(listed)}; stop cases: "
          f"{len([k for k in seen if k in stops])} / {len(stops)}; invalid opcode: {seen[('invalid opcode', 'any')]}")
    for k, (name, pc, steps, want) in examples.items():
        print(f"  WRONG {k}: {name} pc {pc:04X}: {steps} steps, cu-cycles.md {want}")
    missing = sorted(listed - hit)
    print("not covered:", ", ".join(f"{r} {m}" for r, m in missing) or "none")
    return 1 if bad else 0


def main(argv=None):
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("run")
    a.add_argument("prog")
    a.add_argument("--max-lines", type=int, default=0)
    a.add_argument("--max-steps", type=int, default=0)
    a.add_argument("--out", default=str(Path(tempfile.gettempdir()) / "hbc2_cputest"))
    b = sub.add_parser("suite")
    b.add_argument("--random", type=int, default=0)
    b.add_argument("--jobs", type=int, default=6)
    b.add_argument("--max-steps", type=int, default=BOOT_STEPS + 40000)
    b.add_argument("--out", default=str(Path(tempfile.gettempdir()) / "hbc2_cputest"))
    b.add_argument("--only", default="")
    m = sub.add_parser("mutants")
    m.add_argument("prog", nargs="?", default=str(PROGRAMS / "allops.has"))
    m.add_argument("--jobs", type=int, default=10)
    m.add_argument("--out", default=str(Path(tempfile.gettempdir()) / "hbc2_cputest_mut"))
    c = sub.add_parser("cycles")
    c.add_argument("dirs", nargs="+")
    args = ap.parse_args(argv)
    if args.cmd == "cycles":
        return cycles(args.dirs)
    if args.cmd == "mutants":
        return mutants(Path(args.prog), Path(args.out), args.jobs)
    out = Path(args.out)
    if args.cmd == "run":
        r = run(Path(args.prog), out, args.max_lines, args.max_steps)
        print(summary(r))
        return 0 if r["compare_exit"] == 0 and not r["errors"] else 1
    jobs = []
    for src in sorted(PROGRAMS.glob("*.has")):
        if not args.only or re.search(args.only, src.stem):
            jobs.append((src, src.stem))
    for k in range(args.random):
        f = out / f"random{k}.bin"
        out.mkdir(parents=True, exist_ok=True)
        subprocess.run([str(EMUTRACE), "random", str(k), str(f)], check=True)
        jobs.append((f, f"random{k}"))
    results = []
    # one process per run: the observers parse thousands of rows a second each (threads would share the GIL)
    with ProcessPoolExecutor(args.jobs) as ex:
        futs = [ex.submit(run, src, out, 0, args.max_steps, name) for src, name in jobs]
        for f in as_completed(futs):
            r = f.result()
            results.append(r)
            print(summary(r), flush=True)
    (out / "suite.json").write_text(json.dumps([{k: v for k, v in r.items() if k != "cycles"} for r in results],
                                               indent=1), encoding="utf-8")
    bad = sum(1 for r in results if r["compare_exit"] != 0 or r["errors"] or r["contention_rows"])
    print(f"{len(results) - bad} / {len(results)} pass")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
