#!/usr/bin/env python3
"""Flag register -> circuits/flags/flags.circ

The 8 flags of the HBC-2, bit order C0 E1 I2 N3 S4 Z5 F6 H7. Every way the
emulator changes them, and why the register is built this way:
notes/flags-decisions.md.

Every flag is clocked on every clk edge (no gated clock); a 74HCT153 in
front of each flip-flop picks its next value. The two halves of a 153 share
their select lines, so the flags go in pairs that load together:

    pair   153 half 1 / 2   ALU load        bit write
    C I    C / I            flags_op 0001   C0 / I2
    E H    E / H            flags_op 0011   E1 / H7
    Z N    Z / N            0001, 0010      Z5 / N3
    S F    S / F            flags_op 0011   S4 / F6

    153 inputs: D3 hold (the flag itself), D2 ALU (a flag the ALU doesn't
    write holds there too), D1 data bus, D0 bit write.

flags_op (4 bits, one code at a time, so two loads can't clash):

    0000 hold   0001 C Z N <- ALU   0010 Z N <- ALU   0011 S E F <- ALU
    0100 all 8 <- data bus (POF)    0101..0111 hold
    1fff flag fff <- flags_bit_val  (CLC..STF, CLI/STI, HLT, IRT)

Decoding: U1 (74HCT138) decodes 0xxx into active-low loads, U7 (74HCT138)
decodes 1fff into one active-low strobe per flag. Each pair's select lines
are AND3s (74HCT11) of active-low signals, so they come out inverted:
(S1, S0) = 11 hold, 10 ALU, 01 bus, 00 bit write; the 153 data inputs are
wired in that order. A bit write selects its pair's D0; two 74HCT157 then
give the written flag flags_bit_val and its partner its own value: each
pair's flags differ in bit 1 of their number, so flags_op_1 picks.

Storage: 2 x 74HCT175, clocked by clk, cleared by rst_n. The 175 has no
preset, so I is stored inverted (a 74HCT86 gate inverts its next value) and
read from /Q: a reset gives I = 1, the rest 0, like the emulator.

PSF: a 74HCT541 drives the 8 flags onto flags_bus_out while flags_oe_n is
low (high impedance otherwise).

Chips are drawn facing south (datasheet pin order down the left side, back
up the right) and wired by tunnels named after the signals.

    python flags.py           # write, check, and write notes/bom-flags.csv
    python flags.py --test    # ... and run it in Logisim (hbc2 vectors flags, flags_cpu)
    python flags.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "flags"
OUT = ROOT / "circuits" / "flags" / f"{M}.circ"
FLAGS = "ceinszfh"                     # bit order: C0 E1 I2 N3 S4 Z5 F6 H7
P = f"{M}_"

p = Project()
c = p.circuit(M)
COL = {k: 600 + 420 * k for k in range(6)}
ROW = {k: 160 + 280 * k for k in range(4)}
chips = []


def chip(part, col, row, pins, note=""):
    u = c.ttl(part, (COL[col], ROW[row]), facing="south")
    chips.append(u)
    pin_tunnels(c, u, pins)
    if note:
        c.text((COL[col] - 10, ROW[row] - 50), note)
    return u


def flag(f):
    return f"flag_{f}"


# ---- decoding ------------------------------------------------------------------------
# 74138: 1 A, 2 B, 3 C, 4 /G2A, 5 /G2B, 6 G1, 7 /Y7, 9 /Y6 .. 15 /Y0
OPS = {15: None, 14: "op_czn_n", 13: "op_zn_n", 12: "op_sef_n", 11: "op_bus_n"}
chip("74138", 0, 0, {1: P + "op_0", 2: P + "op_1", 3: P + "op_2", 4: P + "op_3", 5: "gnd", 6: "vcc",
                     **{n: P + s for n, s in OPS.items() if s}}, "flags_op 0xxx")
chip("74138", 0, 1, {1: P + "op_0", 2: P + "op_1", 3: P + "op_2", 4: "gnd", 5: "gnd", 6: P + "op_3",
                     **{15 - k if k < 7 else 7: f"{P}bit_{FLAGS[k]}_n" for k in range(8)}}, "flags_op 1fff")

# select lines: AND3 of active-low signals (74HCT11: 1A 1B 1C -> 1Y, 2A..2C -> 2Y, 3A..3C -> 3Y)
PAIRS = [("c", "i", "op_czn_n"), ("e", "h", "op_sef_n"), ("z", "n", "zn_ld_n"), ("s", "f", "op_sef_n")]
gates = []
for a, b, alu in PAIRS:
    hits = [f"{P}bit_{a}_n", f"{P}bit_{b}_n"]
    gates.append(([P + "op_bus_n"] + hits, f"{P}{a}{b}_sel1"))
    gates.append(([P + alu] + hits, f"{P}{a}{b}_sel0"))
gates.append(([P + "op_czn_n", P + "op_zn_n", "vcc"], P + "zn_ld_n"))     # Z N load on 0001 and 0010
G11 = [(("1A", "1B", "1C"), "1Y"), (("2A", "2B", "2C"), "2Y"), (("3A", "3B", "3C"), "3Y")]
for k in range(3):
    pins = {}
    for (ins, out), (srcs, dst) in zip(G11, gates[3 * k:3 * k + 3]):
        pins.update(dict(zip(ins, srcs)))
        pins[out] = dst
    chip("7411", 1, k, pins, "pair selects" if k == 0 else "")

# bit-write lanes: 74HCT157, Y = SELECT ? B : A, SELECT = flags_op_1. The flag
# whose bit 1 is 0 takes the value on A, its partner on B.
LANES = [(("1A", "1B", "1Y"), ("2A", "2B", "2Y")), (("3A", "3B", "3Y"), ("4A", "4B", "4Y"))]
for k, pairs in enumerate((PAIRS[:2], PAIRS[2:])):
    pins = {"SELECT": P + "op_1", "nSTROBE": "gnd"}
    for (lo, hi), (a, b, _) in zip(LANES, pairs):
        for (pa, pb, py), f in ((lo, a), (hi, b)):
            low = FLAGS.index(f) & 2 == 0
            pins.update({pa: P + "bit_val" if low else flag(f), pb: flag(f) if low else P + "bit_val",
                         py: f"{P}{f}_bit"})
    chip("74157", 2, k, pins, "bit write lanes")

# ---- next value of each flag: 74HCT153 per pair ---------------------------------------
# D3 hold, D2 ALU, D1 bus, D0 bit write; select pins carry the inverted code.
for k, (a, b, _) in enumerate(PAIRS):
    pins = {"S1": f"{P}{a}{b}_sel1", "S0": f"{P}{a}{b}_sel0", "n1E": "gnd", "n2E": "gnd"}
    for half, f in (("1", a), ("2", b)):
        alu = f"{P}alu_{f}" if f in "czsenf" else flag(f)
        pins.update({f"{half}D3": flag(f), f"{half}D2": alu, f"{half}D1": f"{P}bus_in_{FLAGS.index(f)}",
                     f"{half}D0": f"{P}{f}_bit", f"{half}Y": f"{P}{f}_next"})
    chip("74153", 3, k, pins, f"{a.upper()} {b.upper()}")

# ---- storage: 2 x 74HCT175, I inverted ----------------------------------------------
# 175: 1 /CLR, 2 Q1, 3 /Q1, 4 D1, 5 D2, 6 /Q2, 7 Q2, 9 CLK, 10 Q3, 11 /Q3, 12 D3, 13 D4, 14 /Q4, 15 Q4
for k, group in enumerate(("cein", "szfh")):
    pins = {"nCLR": "rst_n", "CLK": "clk"}
    for j, f in enumerate(group, 1):
        if f == "i":                                   # stored inverted, read from /Q
            pins.update({f"D{j}": P + "i_next_n", f"nQ{j}": flag("i")})
        else:
            pins.update({f"D{j}": f"{P}{f}_next", f"Q{j}": flag(f)})
    chip("74175", 4, k, pins, "C E I N (I inverted)" if k == 0 else "S Z F H")
chip("7486", 4, 2, {"1A": P + "i_next", "1B": "vcc", "1Y": P + "i_next_n",
                    "2A": "gnd", "2B": "gnd", "3A": "gnd", "3B": "gnd", "4A": "gnd", "4B": "gnd"}, "not I")

# ---- PSF: the flags onto the data bus --------------------------------------------------
chip("74541", 5, 0, {"nOE1": P + "oe_n", "nOE2": "gnd",
                     **{f"A{k + 1}": flag(FLAGS[k]) for k in range(8)},
                     **{f"Y{k + 1}": f"{P}bus_out_{k}" for k in range(8)}}, "PSF")

# chips numbered top to bottom, then left to right
for i, u in enumerate(sorted(chips, key=lambda u: (u.loc[1], u.loc[0])), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ------------------------------------------------------------------------
X_IN, y = 220, 120
for name, w in (("flags_op", 4), ("flags_bit_val", 1), ("flags_oe_n", 1), ("clk", 1), ("rst_n", 1),
                ("flags_alu_c", 1), ("flags_alu_z", 1), ("flags_alu_n", 1), ("flags_alu_s", 1),
                ("flags_alu_e", 1), ("flags_alu_f", 1), ("flags_bus_in", 8)):
    bus_tunnels(c, (X_IN, y), name, w)
    y += 60 if w == 1 else 20 * w + 40
X_OUT, y = COL[5] + 400, ROW[1]
for f in FLAGS:
    bus_tunnels(c, (X_OUT, y), flag(f), 1, output=True)
    y += 40
bus_tunnels(c, (X_OUT, y + 20), "flags_bus_out", 8, output=True)

c.symbol(top=["flags_op", "flags_bit_val", "flags_oe_n", "clk", "rst_n"],
         left=[f"flags_alu_{f}" for f in "cznsef"] + ["flags_bus_in"],
         right=[flag(f) for f in FLAGS] + ["flags_bus_out"])

if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom      # the bill of materials follows the circuit (notes/bom-flags.csv)
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-flags.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-flags.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "flags")
        bad += not logitest.check_emulator(OUT, M, "flags_cpu")
    sys.exit(1 if bad else 0)
