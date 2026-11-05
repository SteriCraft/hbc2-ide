#!/usr/bin/env python3
"""Instruction register -> circuits/ir/ir.circ

Four bytes loaded from the data bus, each with its own load enable, usable
in any step (control-requirements.md C14, C18). Why it is built this way:
notes/ir-mem-decisions.md section 2.

    byte 0  o5 o4 o3 o2 o1 o0 m3 m2   -> ir_op (6), ir_mode bits 3..2
    byte 1  m1 m0 a2 a1 a0 b2 b1 b0   -> ir_mode bits 1..0, ir_r1 (a), ir_r2 (b)
    V1      -> ir_v1, ir_r3 = V1 bits 2..0
    V2      -> ir_v2

6 x 74HCT377 (clocked by clk on every edge, data enable /E) and 1 x 74HCT14.
The register section wants bit 2 of R1, R2 and R3 also inverted, without an
inverter after the register (C8): the 14 inverts bus bits 5 and 2 before two
more 377s, X1 (loaded with byte 1: not R1 bit 2, not R2 bit 2) and X2
(loaded with V1: not R3 bit 2). No decode here: the control unit's microcode
ROM is indexed by ir_op and ir_mode (Gianni, 2026-11-05).

    python ir.py           # write, check, and write notes/bom-ir.csv
    python ir.py --test    # ... and run it in Logisim (hbc2 vectors ir, ir_cpu)
    python ir.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "ir"
OUT = ROOT / "circuits" / "ir" / f"{M}.circ"
P = f"{M}_"

p = Project()
c = p.circuit(M)
COL = {k: 600 + 420 * k for k in range(3)}
ROW = {k: 160 + 280 * k for k in range(3)}
chips = []


def chip(part, col, row, pins, note=""):
    u = c.ttl(part, (COL[col], ROW[row]), facing="south")
    chips.append(u)
    pin_tunnels(c, u, pins)
    if note:
        c.text((COL[col] - 10, ROW[row] - 50), note)
    return u


def n(name, k=None):
    return f"{P}{name}" if k is None else f"{P}{name}_{k}"


def reg(col, row, ld, q, note):
    """A 377 loading bus_data bits 0..7 into the nets q[0..7] (None: unused)."""
    pins = {"nCLKen": n(ld), "CLK": "clk"}
    for k in range(8):
        pins[f"D{k + 1}"] = n("bus_in", k)
        if q[k]:
            pins[f"Q{k + 1}"] = q[k]
    chip("74377", col, row, pins, note)


reg(0, 0, "b0_ld_n", [n("mode", 2), n("mode", 3)] + [n("op", k) for k in range(6)], "byte 0: opcode, mode 3..2")
reg(0, 1, "b1_ld_n", [n("r2", k) for k in range(3)] + [n("r1", k) for k in range(3)] + [n("mode", 0), n("mode", 1)],
    "byte 1: mode 1..0, R1, R2")
reg(1, 0, "v1_ld_n", [n("v1", k) for k in range(8)], "V1 (R3 = bits 2..0)")
reg(1, 1, "v2_ld_n", [n("v2", k) for k in range(8)], "V2")

# X1, X2: the inverted bits 2, inverted before the latch (C8). Unused D inputs to GND.
x1 = {"nCLKen": n("b1_ld_n"), "CLK": "clk", "D1": n("bus_5_n"), "Q1": n("r1_2_n"),
      "D2": n("bus_2_n"), "Q2": n("r2_2_n")}
x2 = {"nCLKen": n("v1_ld_n"), "CLK": "clk", "D1": n("bus_2_n"), "Q1": n("r3_2_n")}
for pins in (x1, x2):
    for k in range(8):
        pins.setdefault(f"D{k + 1}", "gnd")
chip("74377", 2, 0, x1, "X1: not R1 bit 2, not R2 bit 2 (with byte 1)")
chip("74377", 2, 1, x2, "X2: not R3 bit 2 (with V1)")
inv = {"1A": n("bus_in", 5), "1Y": n("bus_5_n"), "2A": n("bus_in", 2), "2Y": n("bus_2_n")}
inv.update({f"{g}A": "gnd" for g in range(3, 7)})
chip("7414", 0, 2, inv, "NOT bus bits 5, 2 (for X1, X2)")

for i, u in enumerate(sorted(chips, key=lambda u: (u.loc[1], u.loc[0])), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ---------------------------------------------------------------------------
TOP = [("clk", 1), (P + "b0_ld_n", 1), (P + "b1_ld_n", 1), (P + "v1_ld_n", 1), (P + "v2_ld_n", 1)]
LEFT = [(P + "bus_in", 8)]
RIGHT = [(P + "op", 6), (P + "mode", 4), (P + "r1", 3), (P + "r2", 3), (P + "r3", 3),
         (P + "r1_2_n", 1), (P + "r2_2_n", 1), (P + "r3_2_n", 1), (P + "v1", 8), (P + "v2", 8)]
X_IN, y = 220, 120
for name, w in TOP + LEFT:
    bus_tunnels(c, (X_IN, y), name, w)
    y += 60 if w == 1 else 20 * w + 40
X_OUT, y = COL[2] + 400, ROW[0]
for name, w in RIGHT:
    names = [n("v1", k) for k in range(3)] if name == P + "r3" else None
    bus_tunnels(c, (X_OUT, y), name, w, output=True, names=names)
    y += 60 if w == 1 else 20 * w + 60

c.symbol(top=[n for n, _ in TOP], left=[n for n, _ in LEFT], right=[n for n, _ in RIGHT])

INPUTS = dict(TOP + LEFT)
OUTPUTS = dict(RIGHT)

if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-ir.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-ir.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "ir")
        bad += not logitest.check_emulator(OUT, M, "ir_cpu")
    sys.exit(1 if bad else 0)
