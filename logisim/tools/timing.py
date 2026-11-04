#!/usr/bin/env python3
"""Worst-case delays of the ALU paths, from the "Delays" table of
logisim/notes/chips.md (TI datasheets, CL = 50 pF).

Each path is a start (t = 0, or the latest of other paths) followed by a
list of chip stages. The three columns (typ, max 25 °C, max 85 °C) are
added up separately. Prints a Markdown table for alu-report.md.

The paths are written by hand from the circuits (designs/*.py): change
them here when a module changes.

    python timing.py            # the table
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
    "sum": (True, "alu_r, arithmetic (bit 7)", [], [
        ("74HCT157", "S → Y", "arith8: B or 01, select = op2"),
        ("74HCT86", "A/B → Y", "add_sub8: b xor sub"),
        ("74HCT283", "A/B → C4", "low nibble carry"),
        ("74HCT283", "Cin → S3", "high nibble, bit 7"),
        ("74HCT541", "A → Y", "rmux8: sum buffer"),
    ]),
    "cin": (False, "carry in of the low 283", [], [
        ("74HCT08", "A/B → Y", "use_c and cin"),
        ("74HCT86", "A/B → Y", "xor sub"),
    ]),
    "en138": (False, "decoder output", [], [
        ("74HCT138", "A/B/C → Y", "rmux8: op2..op0"),
    ]),
    "logic_data": (False, "logic data", [], [
        ("74HCT86", "A/B → Y", "logic8: slowest of 08/32/86"),
        ("74HCT541", "A → Y", "rmux8"),
    ]),
    "logic": (True, "alu_r, AND OR XOR", ["logic_data", "logic_en"], []),
    "logic_en": (False, "logic enable", ["en138"], [
        ("74HCT541", "OE → Y", "rmux8: buffer enable"),
    ]),
    "not_en": (False, "NOT enable", ["en138"], [
        ("74HCT240", "OE → Y", "rmux8: 240 enable"),
    ]),
    "not_data": (False, "NOT data", [], [
        ("74HCT240", "A → Y", "rmux8: not a"),
    ]),
    "not": (True, "alu_r, NOT", ["not_data", "not_en"], []),
    "shift_en": (False, "shift-right enable", ["en138"], [
        ("74HCT11", "A/B/C → Y", "rmux8: and3 of nY5..nY7"),
        ("74HCT541", "OE → Y", "rmux8: sr buffer enable"),
    ]),
    "shift_data": (False, "shift data", [], [
        ("74HCT153", "I → Y", "shift8: ASR fill"),
        ("74HCT541", "A → Y", "rmux8: sr buffer"),
    ]),
    "shift": (True, "alu_r, shifts", ["shift_data", "shift_en"], []),
    "f": (True, "alu_f (borrow), carry out", [], [
        ("74HCT157", "S → Y", "arith8: B or 01"),
        ("74HCT86", "A/B → Y", "add_sub8: b xor sub"),
        ("74HCT283", "A/B → C4", "low nibble"),
        ("74HCT283", "Cin → C4", "high nibble"),
        ("74HCT86", "A/B → Y", "add_sub8: C4 xor sub"),
    ]),
    "c": (True, "alu_c", ["f"], [
        ("74HCT153", "I → Y", "shift8: C select"),
    ]),
    "n": (True, "alu_n", ["sum"], []),
    "z": (True, "alu_z, alu_e", ["sum", "logic", "logic_en", "not", "shift"], [
        ("74HCT27", "A/B/C → Y", "flags8: nor3 ×3"),
        ("74HCT11", "A/B/C → Y", "flags8: and3"),
    ]),
    "s": (True, "alu_s", ["z", "f"], [
        ("74HCT02", "A/B → Y", "flags8: nor(c, z)"),
    ]),
}


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
    ap = argparse.ArgumentParser(description="ALU path delays from chips.md.")
    ap.add_argument("--limit", type=int, help="exit 1 if a shown path is over this at max 85 °C")
    args = ap.parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    table = delays()
    done, via = compute(table)
    print("| Path | Through (max 85 °C per stage, ns) | Typ | Max 25 °C | Max 85 °C |")
    print("|---|---|---|---|---|")
    over = []
    for n, (shown, label, _, _) in PATHS.items():
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
