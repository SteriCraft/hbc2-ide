#!/usr/bin/env python3
"""Worst-case delays of the ALU and flag-register paths, from the "Delays" table of
logisim/notes/chips.md (TI datasheets, CL = 50 pF).

Each path is a start (t = 0, or the latest of other paths) followed by a
list of chip stages. The three columns (typ, max 25 °C, max 85 °C) are
added up separately. Prints a Markdown table for alu-report.md
(--section alu) or flags-decisions.md (--section flags).

The paths are written by hand from the circuits (designs/*.py): change
them here when a module changes.

    python timing.py            # the ALU table
    python timing.py --section flags   # the flag register, within one 500 ns step
    python timing.py --limit 250   # ... and exit 1 if a max 85 °C is over 250 ns
"""

from __future__ import annotations

import argparse
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
        ("74HCT541", "A → Y", "U6"),
    ]),
    "f_oe": (True, "flags_oe_n → flags_bus_out", [], [
        ("74HCT541", "OE → Y", "U6"),
    ]),
    "f_rst": (True, "rst_n → flags", [], [
        ("74HCT175", "/MR → Q", "flags: clear"),
    ]),
}
SECTIONS = {"alu": PATHS, "flags": FLAGS}
PATHS = {**PATHS, **FLAGS}


def compute(table):
    done: dict[str, tuple[int, int, int]] = {}
    via: dict[str, str] = {}

    def run(stages):
        t, names = (0, 0, 0), []
        for part, p, _ in stages:
            d = table[(part, p)]
            t = tuple(a + b for a, b in zip(t, d))
            names.append(f"{part[5:]} {p} ({d[2]})")
        return t, names

    def get(name):
        if name in done:
            return done[name]
        _, label, start, stages = PATHS[name]
        starts = [get(s) for s in start]
        t0 = tuple(max(col) for col in zip(*starts)) if starts else (0, 0, 0)
        t, names = run(stages)
        done[name] = tuple(a + b for a, b in zip(t0, t))
        # the start that is latest at 85 °C is the one that counts
        lead = [via[max(start, key=lambda s: done[s][2])]] if start else []
        via[name] = " → ".join(lead + names)
        return done[name]

    for n in PATHS:
        get(n)
    return done, via


def main(argv=None):
    ap = argparse.ArgumentParser(description="ALU and flag-register path delays from chips.md.")
    ap.add_argument("--section", choices=SECTIONS, default="alu", help="which paths to show (default alu)")
    ap.add_argument("--limit", type=int, help="exit 1 if a shown path is over this at max 85 °C")
    args = ap.parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    table = delays()
    done, via = compute(table)
    print("| Path | Through (max 85 °C per stage, ns) | Typ | Max 25 °C | Max 85 °C |")
    print("|---|---|---|---|---|")
    over = []
    for n, (shown, label, _, _) in SECTIONS[args.section].items():
        if not shown:
            continue
        t = done[n]
        hot = args.limit and t[2] > args.limit
        over += [label] if hot else []
        cells = [str(x) for x in t]
        if hot:
            cells[2] = f"**{cells[2]}**"
        print(f"| {label} | {via[n]} | " + " | ".join(cells) + " |")
    if args.limit:
        print(f"\n{len(over)} paths over {args.limit} ns at max 85 °C" + (": " + "; ".join(over) if over else ""))
    sys.exit(1 if over else 0)


if __name__ == "__main__":
    main()
