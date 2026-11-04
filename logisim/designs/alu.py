#!/usr/bin/env python3
"""ALU -> circuits/alu/alu.circ

For now the ALU only adds, through the add8 module (circuits/alu/add8.circ,
loaded as a Logisim library):

    alu_s, alu_c = alu_a + alu_b + alu_cin

Layout follows the conventions: data in on the left, result on the right,
the carry flag at the bottom, in the schematic and in the symbol.

    python alu.py           # write, check
    python alu.py --test    # ... and run it in Logisim against the emulator (hbc2 vectors)
    python alu.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project  # noqa: E402

OUT = ROOT / "circuits" / "alu" / "alu.circ"

p = Project()
add8 = p.library(OUT.parent / "add8.circ")
c = p.circuit("alu")

u = c.sub(add8, "add8", (500, 300))

# data inputs, left; a and cin jog onto the adder's ports so the pins sit
# 60 px apart
c.wire(c.input((130, 260), "alu_a", 8), (230, 260), (230, u["add8_a"][1]), u["add8_a"])
c.wire(c.input((130, 320), "alu_b", 8), u["add8_b"])
c.wire(c.input((130, 380), "alu_cin"), (240, 380), (240, u["add8_cin"][1]), u["add8_cin"])

# result, right
c.wire(u["add8_s"], c.output((650, 300), "alu_s", 8))

# carry flag, bottom
x = u["add8_cout"][0] + 60
c.wire(u["add8_cout"], (x, u["add8_cout"][1]), c.output((x, 450), "alu_c", facing="north"))

c.symbol(left=["alu_a", "alu_b", "alu_cin"], right=["alu_s"], bottom=["alu_c"])

if __name__ == "__main__":
    bad = p.save(OUT)
    if "--png" in sys.argv:
        import render
        import netlist
        print(render.render(netlist.Project(OUT).circuits["alu"], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, "alu")
    sys.exit(1 if bad else 0)
