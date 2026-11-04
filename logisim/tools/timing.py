#!/usr/bin/env python3
"""Worst-case delays of the ALU, flag-register and register-section paths, from the "Delays" table of
logisim/notes/chips.md (TI datasheets, CL = 50 pF).

Each path is a start (t = 0, or the latest of other paths) followed by a
list of chip stages. The three columns (typ, max 25 °C, max 85 °C) are
added up separately. Prints a Markdown table for alu-report.md
(--section alu), flags-decisions.md (--section flags) or
registers-decisions.md (--section regs).

Pass criterion (Gianni, 2026-11-04): every path with a limit (LIMITS) must
meet it at **max 25 °C**, bus loading included. Max 85 °C is reported too,
for information only. The exit code is 1 if a path fails at max 25 °C.

The paths are written by hand from the circuits (designs/*.py): change
them here when a module changes.

    python timing.py            # the ALU table
    python timing.py --section flags   # the flag register, within one 500 ns step
    python timing.py --section regs    # the register section and the end-to-end ALU step
    python timing.py --limit 250   # ... and exit 1 if a shown path is over 250 ns at max 25 °C
"""

from __future__ import annotations

import argparse
import math
import re
import sys
from pathlib import Path

CHIPS_MD = Path(__file__).resolve().parent.parent / "notes" / "chips.md"


def delays(path: Path = CHIPS_MD) -> dict[tuple[str, str], tuple[int, int, int]]:
    """(part, path) -> (typ, max 25 °C, max 85 °C), from the Delays table."""
    text = path.read_text(encoding="utf8")
    m = re.search(r"^## Delays\n(.*?)(?=^## )", text, re.S | re.M)
    if not m:
        raise SystemExit(f"{path}: no '## Delays' section")
    out = {}
    for line in m.group(1).splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if line.startswith("|") and len(cells) >= 5 and cells[2].isdigit():
            out[(cells[0], cells[1])] = tuple(int(c) for c in cells[2:5])
    return out


# name: (shown, label, start, stages); start = names of earlier paths (latest one wins)
# A stage is (part, datasheet path, what it does here).
PATHS = {
    "b_c4": (False, "b to low C4", [], [
        ("74HCT86", "A/B → Y", "add_sub8: b xor sub"),
        ("74HCT283", "A/B → C4", "U6: low nibble carry"),
    ]),
    "cin_c4": (False, "carry in to low C4", [], [
        ("74HCT08", "A/B → Y", "add_sub8: use_c and cin"),
        ("74HCT86", "A/B → Y", "add_sub8: xor sub"),
        ("74HCT283", "Cin → C4", "U6: low nibble carry"),
    ]),
    "hi_s": (False, "high nibble sums", [], [
        ("74HCT86", "A/B → Y", "add_sub8: b xor sub"),
        ("74HCT283", "A/B → S", "U2 / U3: both carries in"),
        ("74HCT157", "I → Y", "U4: data"),
    ]),
    "select": (False, "carry select", ["b_c4", "cin_c4"], [
        ("74HCT157", "S → Y", "U4: low C4 picks cin 0 / 1"),
    ]),
    "sum": (True, "alu_r, arithmetic (bit 7)", ["select", "hi_s"], [
        ("74HCT541", "A → Y", "rmux8: sum buffer"),
    ]),
    "en138": (False, "decoder output", [], [
        ("74HCT138", "A/B/C → Y", "rmux8: op2..op0"),
    ]),
    "logic_data": (False, "logic data", [], [
        ("74HCT86", "A/B → Y", "logic8: slowest of 08/32/86"),
        ("74HCT541", "A → Y", "rmux8"),
    ]),
    "logic_en": (False, "logic enable", ["en138"], [
        ("74HCT541", "OE → Y", "rmux8: buffer enable"),
    ]),
    "logic": (True, "alu_r, AND OR XOR", ["logic_data", "logic_en"], []),
    "not_data": (False, "NOT data", [], [
        ("74HCT240", "A → Y", "rmux8: not a"),
    ]),
    "not_en": (False, "NOT enable", ["en138"], [
        ("74HCT240", "OE → Y", "rmux8: 240 enable"),
    ]),
    "not": (True, "alu_r, NOT", ["not_data", "not_en"], []),
    "shift_data": (False, "shift data", [], [
        ("74HCT153", "I → Y", "shift8: ASR fill"),
        ("74HCT541", "A → Y", "rmux8: sr buffer"),
    ]),
    "shift_en": (False, "shift-right enable", ["en138"], [
        ("74HCT11", "A/B/C → Y", "rmux8: and3 of nY5..nY7"),
        ("74HCT541", "OE → Y", "rmux8: sr buffer enable"),
    ]),
    "shift": (True, "alu_r, shifts", ["shift_data", "shift_en"], []),
    "c_add": (True, "adder carry / borrow", ["b_c4", "cin_c4"], [
        ("74HCT157", "S → Y", "U7: carry select"),
        ("74HCT86", "A/B → Y", "add_sub8: C4 xor sub"),
    ]),
    "c": (True, "alu_c", ["c_add"], [
        ("74HCT153", "I → Y", "shift8: C select"),
    ]),
    "n": (True, "alu_n", ["sum"], []),
    "z": (True, "alu_z", ["sum", "logic", "not", "shift"], [
        ("74HCT27", "A/B/C → Y", "flags8: nor3 ×3"),
        ("74HCT11", "A/B/C → Y", "flags8: and3"),
    ]),
    "s": (True, "alu_s, alu_f", [], [
        ("74HCT85", "A/B → A>B, A<B", "cmp8 U1: bits 3..0"),
        ("74HCT85", "cascade in → A>B, A<B", "cmp8 U2: bits 7..4"),
    ]),
    "e": (True, "alu_e", [], [
        ("74HCT85", "A/B → A=B", "cmp8 U1: bits 3..0"),
        ("74HCT85", "cascade in → A=B", "cmp8 U2: bits 7..4"),
    ]),
}

# Flag register (designs/flags.py), one clock step: the step starts on the
# clock edge that loads the operand / control registers (74HCT574 CP -> Q)
# and ends with the 74HCT175 setup before the next edge. The ALU paths above
# start when the operands are valid, so the register delay is added after
# them (sums don't depend on order).
FLAGS = {
    "f_alu": (True, "ALU flag → flip-flop (C Z N S E F)", ["c", "z", "n", "s", "e"], [
        ("74HCT574", "CP → Q", "operand register, before the ALU"),
        ("74HCT153", "I → Y", "flags: D2, ALU input"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_sel": (False, "flags_op to the selects", [], [
        ("74HCT574", "CP → Q", "control register: flags_op"),
        ("74HCT138", "A/B/C → Y", "U1 / U7: decode"),
        ("74HCT11", "A/B/C → Y", "pair selects"),
    ]),
    "f_ctl": (True, "flags_op → flip-flop (I, the slowest)", ["f_sel"], [
        ("74HCT153", "S → Y", "flags: next value"),
        ("74HCT86", "A/B → Y", "U12: not I"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_bit": (True, "single-flag write → flip-flop (I)", [], [
        ("74HCT574", "CP → Q", "control register: flags_op_1"),
        ("74HCT157", "S → Y", "bit-write lane"),
        ("74HCT153", "I → Y", "flags: D0"),
        ("74HCT86", "A/B → Y", "U12: not I"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_bus": (True, "flags_bus_in → flip-flop (I), from the bus being valid", [], [
        ("74HCT153", "I → Y", "flags: D1"),
        ("74HCT86", "A/B → Y", "U12: not I"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_q": (True, "clk → flag_c ... flag_h", [], [
        ("74HCT175", "CLK → Q", "flags: flip-flop"),
    ]),
    "f_psf": (True, "clk → flags_bus_out (PSF, already enabled)", ["f_q"], [
        ("74HCT541", "A → Y", "U6", ("bus_data",)),
    ]),
    "f_oe": (True, "flags_oe_n → flags_bus_out", [], [
        ("74HCT541", "OE → Y", "U6", ("bus_data",)),
    ]),
    "f_rst": (True, "rst_n → flags", [], [
        ("74HCT175", "/MR → Q", "flags: clear"),
    ]),
}
# ---- bus loading -----------------------------------------------------------------------
# chips.md delays are at CL = 50 pF. A stage given a 4th element (the nets it
# drives) gets the extra delay of the load on its worst bit beyond 50 pF.
#   pins: 10 pF per input (CI max), 20 pF per 3-state output, enabled or
#         not (CO, CD74HC670 datasheet; the 541 and 574 are similar);
#   wiring: 2 pF per pin (stub) + a backbone: 20 pF for the buses that run
#         across the boards, 5 pF for the nets local to one section;
#   slope: SN74HCT541 (6 mA bus driver) 50 -> 150 pF at 4.5 V: tpd 13/23/29 ->
#         20/33/42 ns, so 0.07 / 0.10 / 0.13 ns per pF (typ, max 25, max 85 °C).
#         Standard outputs (4 mA: 670, 153 ...) have no 150 pF data: scaled
#         by drive, 6 / 4 x that = 0.105 / 0.15 / 0.20 ns per pF.
# The pins are counted on the built circuits (netlist.py flatten, chip models
# included); the sections not built yet are estimates, listed with each bus.
PIN_PF = {"in": 10, "out": 20}
WIRE_PF_PER_PIN = 2
WIRE_PF_BUS = {"bus_a": 20, "bus_b": 20, "bus_data": 20}       # others: 5 (local)
SLOPE = {"bus": (0.07, 0.10, 0.13), "std": (0.105, 0.15, 0.20)}
DRIVE = {"74HCT541": "bus", "74HCT574": "bus", "74HCT240": "bus", "74HCT670": "std", "74HCT153": "std"}
CIRCUITS = Path(__file__).resolve().parent.parent / "circuits"
# bus: [(file, top circuit, net), or ("est", inputs, outputs, what)]
BUSES = {
    "bus_a": [("registers/regs.circ", "regs", "regs_a"), ("alu/alu.circ", "alu", "alu_a"),
              ("est", 1, 0, "address path: adder input, R1 high byte (not built)")],
    "bus_b": [("registers/regs.circ", "regs", "regs_b"), ("alu/alu.circ", "alu", "alu_b"),
              ("est", 1, 0, "address path: adder input, R2 low byte (not built)")],
    "bus_data": [("registers/regs.circ", "regs", "regs_bus_in"), ("registers/regs.circ", "regs", "regs_bus_out"),
                 ("flags/flags.circ", "flags", "flags_bus_in"), ("flags/flags.circ", "flags", "flags_bus_out"),
                 ("est", 4, 0, "IR: opcode, byte 1, V1, V2 latches (not built)"),
                 ("est", 2, 2, "PC: load hi / lo, drive hi / lo for CAL (not built)"),
                 ("est", 2, 2, "SP: load hi / lo, drive hi / lo for RSP (not built)"),
                 ("est", 0, 3, "RAM, ROM, I/O port data (not built)")],
}
# nets local to the register section, driven by the index multiplexers
REGS_F = "registers/regs.circ"
BUSES.update({
    "a_idx": [(REGS_F, "regs", "regs_a_idx")],       # RA of copy A + WA of all 8
    "b_idx": [(REGS_F, "regs", "regs_b_idx")],
    "a_re_lo": [(REGS_F, "regs", "regs_a_re_lo_n")],     # /RE of 2 670s + the 32 (write enable)
    "a_re_hi": [(REGS_F, "regs", "regs_a_re_hi_n")],
    "b_re_lo": [(REGS_F, "regs", "regs_b_re_lo_n")],
    "b_re_hi": [(REGS_F, "regs", "regs_b_re_hi_n")],
})
BUS_AB = ("bus_a", "bus_b")
IDX = ("a_idx", "b_idx")
RE = ("a_re_lo", "a_re_hi", "b_re_lo", "b_re_hi")


def bus_pins(bus):
    """(inputs, outputs, [(what, inputs, outputs)]) on the worst bit of a bus."""
    import netlist
    flat = {}
    per_bit: dict[int, list[int]] = {}
    parts = []
    for src in BUSES[bus]:
        if src[0] == "est":
            _, i, o, what = src
            parts.append((what, i, o))
            for b in range(8):
                per_bit.setdefault(b, [0, 0])
                per_bit[b][0] += i
                per_bit[b][1] += o
            continue
        f, top, net = src
        if (f, top) not in flat:
            flat[(f, top)] = netlist.flatten(netlist.Project(CIRCUITS / f).circuits[top])["nets"]
        worst = (0, 0)
        for n in flat[(f, top)]:
            pin = [t for t in n["terminals"] if t["pin"] is None and t["ref"].startswith(net + "[")]
            m = re.fullmatch(re.escape(net) + r"(?:_(\d))?", n["name"])
            if pin:
                b = int(pin[0]["ref"][len(net) + 1:-1])
            elif m:
                b = int(m.group(1) or 0)
            else:
                continue
            chips = [t for t in n["terminals"] if t["pin"] is not None]
            i = sum(t["dir"] == "in" for t in chips)
            o = len(chips) - i
            per_bit.setdefault(b, [0, 0])
            per_bit[b][0] += i
            per_bit[b][1] += o
            worst = max(worst, (i, o), key=sum)
        parts.append((f"{top}: {net}", *worst))
    i, o = max(per_bit.values(), key=lambda v: v[0] * PIN_PF["in"] + v[1] * PIN_PF["out"])
    return i, o, parts


def bus_pf(bus):
    i, o, _ = bus_pins(bus)
    return i * PIN_PF["in"] + o * PIN_PF["out"] + (i + o) * WIRE_PF_PER_PIN + WIRE_PF_BUS.get(bus, 5)


_PF = {}


def derate(part, nets):
    """Extra delay (typ, max 25, max 85) of a stage driving the heaviest of nets."""
    for n in nets:
        if n not in _PF:
            _PF[n] = bus_pf(n)
    c = max(_PF[n] for n in nets)
    return tuple(math.ceil(k * max(0, c - 50)) for k in SLOPE[DRIVE[part]])


# Register section (designs/regs.py, notes/registers-decisions.md), one 500 ns
# step from the rising edge of clk. Two kinds of extra stages:
#   ("@", ns, what)        a fixed time in the step (the oscillator's edges)
#   ("path", name, what)   a whole earlier path added in series (end to end)
#   ("-", part, path, what) a chip delay subtracted: the earliest an edge can
#                          come when it may be that much early (minimum delays
#                          are taken as 0)
REGS = {
    "r_ctl": (False, "IR / control unit outputs", [], [
        ("74HCT574", "CP → Q", "IR fields, control register"),
    ]),
    "r_ra": (True, "index → word read (bits 1..0)", ["r_ctl"], [
        ("74HCT153", "S → Y", "U1 / U11: index multiplexer", IDX),
        ("74HCT670", "read address → Q", "port A / B", BUS_AB),
    ]),
    "r_re": (True, "bank enable (bit 2, or not bit 2 from the IR)", ["r_ctl"], [
        ("74HCT153", "I → Y", "U6 / U15: bank /RE", RE),
        ("74HCT670", "/RE → Q", "port A / B", BUS_AB),
    ]),
    "r_lat": (True, "operand latch onto bus A / B", ["r_ctl"], [
        ("74HCT574", "OE → Q", "U10 / U18 (data from the edge: CP → Q 41)", BUS_AB),
    ]),
    "r_const": (True, "constant 00 / 01 onto bus B", ["r_ctl"], [
        ("74HCT541", "OE → Y", "U9", ("bus_b",)),
    ]),
    "r_op": (True, "**operands valid** on regs_a / regs_b (alu_a, alu_b, address path)",
             ["r_ra", "r_re", "r_lat", "r_const"], []),
    "r_flags": (True, "**end to end**: register read → ALU (alu_z) → flag register setup", ["r_op"], [
        ("path", "z", "ALU, slowest flag"),
        ("74HCT153", "I → Y", "flags: D2, ALU input"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "r_res": (True, "register read → ALU (alu_r) → result latch setup", ["r_op"], [
        ("path", "sum", "ALU, slowest result bit"),
        ("74HCT574", "setup D → CP", "U5: result latch"),
    ]),
    "r_dbus": (True, "register → data bus (STR, PSH, OUT, INT)", ["r_op"], [
        ("74HCT541", "A → Y", "U14", ("bus_data",)),
    ]),
    "r_resbus": (True, "result latch → data bus (enable)", ["r_ctl"], [
        ("74HCT574", "OE → Q", "U5", ("bus_data",)),
    ]),
    # The write strobe comes from the clock generator (control-requirements.md C6):
    # osc (4 MHz) -> 74HCT74 FF1 = clk; FF2 = clk sampled on the falling osc
    # edge; clk_wr_n = NAND3(not clk, osc, FF2) (74HCT10), low from the clk
    # fall (osc rise, 250) to the osc fall (375). Minimum delays are taken as 0:
    # t = 0 is clk rising, up to 35 ns after its osc edge, so the osc fall can
    # come 35 ns early (375 - 35 = 340), and the clk fall 35 ns early or late.
    "w_wa": (True, "write address + setup, before /WE falls", ["r_ctl"], [
        ("74HCT153", "S → Y", "U1: index multiplexer", ("a_idx",)),
        ("74HCT670", "setup WA → /WE", "all 8 chips"),
    ]),
    "w_en": (True, "bank write enable, stable before clk_wr_n falls", ["r_ctl"], [
        ("74HCT153", "I → Y", "U6: /RE words 4–7", ("a_re_hi",)),
        ("74HCT32", "A/B → Y", "U4: regs_we_n OR /RE"),
    ]),
    "w_fall_min": (True, "/WE falls, earliest", [], [
        ("@", 250, "osc rises"),
        ("-", "74HCT74", "CLK → Q", "t = 0 (clk rising) can be this late, no gate delay"),
    ]),
    "w_fall": (True, "/WE falls, latest", [], [
        ("@", 250, "osc rises"),
        ("74HCT74", "CLK → Q", "FF1: clk falls (late)"),
        ("74HCT10", "A/B/C → Y", "clk_wr_n"),
        ("74HCT32", "A/B → Y", "U4: bank /WE"),
    ]),
    "w_rise_min": (True, "/WE rises, earliest (D setup is to this edge)", [], [
        ("@", 375, "osc falls"),
        ("-", "74HCT74", "CLK → Q", "t = 0 can be this late, no gate delay"),
    ]),
    "w_data": (True, "**data on bus_data valid by** (the earliest rise − setup)", [], [
        ("path", "w_rise_min", "earliest /WE rise"),
        ("-", "74HCT670", "setup D → /WE", "all 8 chips"),
    ]),
    # the two ends move together with clk: the pulse is 125 ns of osc minus
    # what can delay its start only (FF1, the NAND, the OR)
    "w_pulse": (True, "/WE pulse, shortest", [], [
        ("@", 125, "osc rise to osc fall"),
        ("-", "74HCT74", "CLK → Q", "FF1"),
        ("-", "74HCT10", "A/B/C → Y", "the NAND"),
        ("-", "74HCT32", "A/B → Y", "the OR"),
    ]),
    "w_rise": (True, "/WE rises, latest, + hold", [], [
        ("@", 375, "osc falls"),
        ("74HCT10", "A/B/C → Y", "clk_wr_n"),
        ("74HCT32", "A/B → Y", "U4: bank /WE"),
        ("74HCT670", "hold D, WA after /WE", "all 8 chips"),
    ]),
}
SECTIONS = {"alu": PATHS, "flags": FLAGS, "regs": REGS}
PATHS = {**PATHS, **FLAGS, **REGS}

# ---- pass criterion --------------------------------------------------------------------
# name: (op, bound). "<=": the path must end by the bound; ">=": it must last
# at least that long. The bound is a time in ns, another path (compared column
# by column), or a chip delay (part, path) from chips.md. Paths without a
# limit (the ALU outputs, the reference times of the write phase) are parts
# of a longer path that has one. The latest /WE fall is not compared with the
# earliest rise: they assume opposite clk skews; w_pulse checks the pulse.
STEP = 500                                   # ns, one step at 2 MHz
COL = {"typ": 0, "25": 1, "85": 2}
PASS = COL["25"]                             # the design must pass at max 25 °C
LIMITS = {
    # flag register: inside the step
    "f_alu": ("<=", STEP), "f_ctl": ("<=", STEP), "f_bit": ("<=", STEP),
    "f_psf": ("<=", STEP), "f_oe": ("<=", STEP),
    # register section
    "r_flags": ("<=", STEP), "r_res": ("<=", STEP), "r_dbus": ("<=", STEP),
    "r_resbus": ("<=", "w_data"),            # the result latch's data for a register write
    "w_wa": ("<=", "w_fall_min"), "w_en": ("<=", "w_fall_min"),
    "w_pulse": (">=", ("74HCT670", "/WE pulse width")),
    "w_rise": ("<=", STEP),                  # D and WA change at the next edge
}


def compute(table):
    done: dict[str, tuple[int, int, int]] = {}
    via: dict[str, str] = {}

    def run(stages):
        t, names = (0, 0, 0), []
        for part, p, *rest in stages:
            if part == "@":                          # a fixed time in the step
                d = (p, p, p)
                names.append(f"{rest[0]} ({p})")
            elif part == "path":                     # another path in series
                d = get(p)
                names.append(f"[{PATHS[p][1].replace('*', '')}] ({d[1]} / {d[2]})")
            elif part == "-":                        # a delay that makes an edge early
                part, p = p, rest[0]
                d = tuple(-x for x in table[(part, p)])
                names.append(f"− {part[5:]} {p} ({-d[1]} / {-d[2]})")
            else:
                d = table[(part, p)]
                nets = rest[1:]
                if nets:                             # driving a loaded bus
                    extra = derate(part, nets[0])
                    names.append(f"{part[5:]} {p} ({d[1]} + {extra[1]} / {d[2]} + {extra[2]} load)")
                    d = tuple(a + b for a, b in zip(d, extra))
                else:
                    names.append(f"{part[5:]} {p} ({d[1]} / {d[2]})")
            t = tuple(a + b for a, b in zip(t, d))
        return t, names

    def get(name):
        if name in done:
            return done[name]
        _, label, start, stages = PATHS[name]
        starts = [get(s) for s in start]
        t0 = tuple(max(col) for col in zip(*starts)) if starts else (0, 0, 0)
        t, names = run(stages)
        done[name] = tuple(a + b for a, b in zip(t0, t))
        # the start that is latest in the pass column is the one shown
        lead = [via[max(start, key=lambda s: done[s][PASS])]] if start else []
        via[name] = " → ".join(lead + names)
        return done[name]

    for n in PATHS:
        get(n)
    return done, via


def margins(name, done, table):
    """(limit text, margin per column) of a path, or None if it has no limit."""
    if name not in LIMITS:
        return None
    op, b = LIMITS[name]
    if isinstance(b, int):
        lim, what = (b, b, b), None
    elif isinstance(b, tuple):
        lim, what = table[b], f"{b[0][5:]} {b[1]}"
    else:
        lim, what = done[b], PATHS[b][1].replace("*", "").split(" (")[0]
    m = tuple((l - v) if op == "<=" else (v - l) for v, l in zip(done[name], lim))
    sign = "≤" if op == "<=" else "≥"
    need = f"{sign} {b}" if what is None else f"{sign} {lim[1]} / {lim[2]} ({what})"
    return need, m


def main(argv=None):
    ap = argparse.ArgumentParser(description="ALU, flag-register and register-section path delays from chips.md.")
    ap.add_argument("--section", choices=SECTIONS, default="alu", help="which paths to show (default alu)")
    ap.add_argument("--limit", type=int, help="also fail if a shown path is over this at max 25 °C")
    args = ap.parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    table = delays()
    done, via = compute(table)
    print("| Path | Through (per stage, max 25 / 85 °C, ns) | Typ | Max 25 °C | Max 85 °C "
          "| Limit (25 / 85 °C) | Margin 25 °C (pass) | Margin 85 °C (info) |")
    print("|---|---|---|---|---|---|---|---|")
    fail, hot = [], []
    for n, (shown, label, _, _) in SECTIONS[args.section].items():
        if not shown:
            continue
        t = done[n]
        cells = [str(x) for x in t]
        lm = margins(n, done, table)
        if lm is None and args.limit:
            lm = (f"≤ {args.limit}", tuple(args.limit - x for x in t))
        if lm:
            need, m = lm
            name = label.replace("*", "")
            if m[PASS] < 0:
                fail.append(f"{name} ({m[PASS]})")
            if m[COL["85"]] < 0:
                hot.append(f"{name} ({m[COL['85']]})")
            cells += [need] + [f"**{x}**" if x < 0 else str(x) for x in m[1:]]
        else:
            cells += ["—", "—", "—"]
        print(f"| {label} | {via[n]} | " + " | ".join(cells) + " |")
    if args.section == "regs":
        print("\n| Bus (worst bit) | Inputs | 3-state outputs | Load (pF) | Extra delay, bus driver (max 25 / 85 °C, ns) "
              "| Extra delay, standard output (max 25 / 85 °C, ns) | Counted (worst bit of each) |")
        print("|---|---|---|---|---|---|---|")
        for bus in BUSES:
            i, o, parts = bus_pins(bus)
            c = bus_pf(bus)
            ex = [" / ".join(str(math.ceil(SLOPE[k][col] * max(0, c - 50))) for col in (1, 2)) for k in ("bus", "std")]
            what = "; ".join(f"{w}: {a} in, {b} out" for w, a, b in parts)
            print(f"| {bus} | {i} | {o} | {c} | {ex[0]} | {ex[1]} | {what} |")
    print("\nPass (max 25 °C): " + ("all limits met" if not fail else f"{len(fail)} FAIL: " + "; ".join(fail)))
    print("Max 85 °C (information only): " + ("all limits met" if not hot else f"{len(hot)} over: " + "; ".join(hot)))
    sys.exit(1 if fail else 0)


if __name__ == "__main__":
    main()
