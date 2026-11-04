#!/usr/bin/env python3
"""Shifts and carry select -> circuits/alu/shift8.circ

The shifted values are wiring (splitters):

    shift8_shl = a6..a0, 0          SHL
    shift8_sr  = fill, a7..a1       SHR (fill = 0) or ASR (fill = a7)

One 74HCT153 (U1), selects S1 = shift8_sel (alu_op bit 3) and
S0 = shift8_asr (alu_op bit 0):

    half 1: shift8_c   D0 = D1 = c_add (adder), D2 = a7 (SHL), D3 = a0 (ASR)
    half 2: fill       D0 = D1 = D2 = 0,        D3 = a7 (ASR)

so C is the adder's carry for alu_op 0xxx, bit 7 of a for SHL (1100) and
bit 0 of a for ASR (1111, and its alias 1101).

    python shift8.py           # write, check
    python shift8.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python shift8.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, rails_left  # noqa: E402

M = "shift8"
OUT = ROOT / "circuits" / "alu" / f"{M}.circ"

p = Project()
c = p.circuit(M)

X_IN, X_OUT = 220, 1000
Y_A = 200
LINE_X = {"a": 300, "sel": 360, "asr": 400, "c_add": 440, "c": 480, "fill": 520}

# ---- U1: carry select and fill --------------------------------------------------
mux = c.ttl("74153", (620, 720), label="U1")
c.text((900, 740), "c select, ASR fill")
taps = {"a": []}
ends = {k: [] for k in ("sel", "asr", "c_add", "c", "fill")}
rails_left(c, mux, {
    "n1E": ("gnd", None), "S1": ("sel", None), "1D3": ("a", 0), "1D2": ("a", 7),
    "1D1": ("c_add", None), "1D0": ("c_add", None), "1Y": ("c", None),
    "n2E": ("gnd", None), "S0": ("asr", None), "2D3": ("a", 7),
    "2D2": ("gnd", None), "2D1": ("gnd", None), "2D0": ("gnd", None), "2Y": ("fill", None),
}, LINE_X, taps, ends)

# ---- control pins, top: each drops to its line ------------------------------------
for name, x_pin, y_turn in (("sel", 360, None), ("asr", 500, 100), ("c_add", 640, 140)):
    x_line = LINE_X[name]
    top = c.input((x_pin, 40), f"{M}_{name}", facing="south")
    if y_turn is None:
        c.wire(top, (x_line, max(ends[name])))
    else:
        c.wire(top, (x_pin, y_turn), (x_line, y_turn), (x_line, max(ends[name])))

# ---- a: pin, the bus down to U1, and the two shift splitters ----------------------
c.wire(c.input((X_IN, Y_A), f"{M}_a", 8), (600, Y_A))
c.bus((LINE_X["a"], Y_A), taps["a"], width=8)
c.wire((560, Y_A), (560, 340), (600, 340))

# SR: a7..a1 -> sr6..sr0, fill -> sr7
s = c.splitter((600, Y_A), 8, [list(range(1, 8))])
j = c.splitter((800, 220), 8, [list(range(0, 7)), [7]], facing="west", appear="right", spacing=2)
c.wire(s["end0"], j["end0"])
c.wire(c.tunnel((740, j["end1"][1]), f"{M}_fill", facing="east"), j["end1"])
c.wire(j["combined"], c.output((X_OUT, j["combined"][1]), f"{M}_sr", 8))
c.text((700, 160), "sr = fill, a7..a1")

# SHL: a6..a0 -> shl7..shl1, 0 -> shl0
s = c.splitter((600, 340), 8, [list(range(0, 7))])
j = c.splitter((800, 340), 8, [[0], list(range(1, 8))], facing="west", appear="right", spacing=2)
c.wire(s["end0"], j["end1"])
c.wire(j["end0"], (760, j["end0"][1]))
c.add("#Wiring", "Ground", (760, j["end0"][1]), facing="west")
c.wire(j["combined"], c.output((X_OUT, j["combined"][1]), f"{M}_shl", 8))
c.text((700, 400), "shl = a6..a0, 0")

# ---- outputs of U1: c to the bottom, fill to its tunnel ---------------------------
x = LINE_X["c"]
c.wire((x, max(ends["c"])), c.output((x, 1000), f"{M}_c", facing="north"))
x = LINE_X["fill"]
c.wire((x, min(ends["fill"])), (x, 460), c.tunnel((x + 20, 460), f"{M}_fill"))

c.symbol(top=[f"{M}_sel", f"{M}_asr", f"{M}_c_add"], left=[f"{M}_a"],
         right=[f"{M}_shl", f"{M}_sr"], bottom=[f"{M}_c"])

# Gianni moved everything by (20, 60) in Logisim so the labels show.
c.move(20, 60)

if __name__ == "__main__":
    bad = p.save(OUT)
    if "--png" in sys.argv:
        import render
        import netlist
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M)
    sys.exit(1 if bad else 0)
