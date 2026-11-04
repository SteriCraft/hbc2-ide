#!/usr/bin/env python3
"""Bench for Logisim's own 74377 -> circuits/chips/test_74377.circ

The 74HCT377 (octal D flip-flop with data enable) is in Logisim's #TTL
library, so it needs no model. The address section relies on it as a
register with a synchronous load enable, so this bench checks Logisim's chip
against the datasheet (cd74hct377.pdf, TI SCHS184C, truth table):

    CP rising, /E low    Q takes D
    CP rising, /E high   no change
    CP not rising        no change, whatever /E and D do

/E is a synchronous input (setup 12 ns, hold 5 ns to CP rising): it does not
gate the clock. The sequences check that: /E falling or rising while CP is
high must not load anything.

Logisim names the pins D1..D8 / Q1..Q8 and nCLKen; TI names them D0..D7 /
Q0..Q7 and /E. Same pin numbers (1 /E, 11 CP).

    python test_74377.py           # write, check
    python test_74377.py --test    # ... and run the datasheet sequences in Logisim
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "test_74377"
OUT = ROOT / "circuits" / "chips" / f"{M}.circ"

# TI pinout: 1 /E, 2 Q0, 3 D0, 4 D1, 5 Q1, 6 Q2, 7 D2, 8 D3, 9 Q3, 10 GND,
#            11 CP, 12 Q4, 13 D4, 14 D5, 15 Q5, 16 Q6, 17 D6, 18 D7, 19 Q7, 20 VCC
D_PINS = (3, 4, 7, 8, 13, 14, 17, 18)
Q_PINS = (2, 5, 6, 9, 12, 15, 16, 19)
PINS = {1: "e_n", 11: "cp", **{p: f"d_{k}" for k, p in enumerate(D_PINS)},
        **{p: f"q_{k}" for k, p in enumerate(Q_PINS)}}

p = Project()
c = p.circuit(M)
u = c.ttl("74377", (500, 300), label="U1")
pin_tunnels(c, u, PINS)
for i, (name, w) in enumerate((("cp", 1), ("e_n", 1), ("d", 8))):
    bus_tunnels(c, (100, 100 + 90 * i), name, w)
bus_tunnels(c, (900, 100), "q", 8, output=True)
c.text((560, 180), "Logisim's own 74377: behaviour vs the TI datasheet")

INPUTS = {"cp": 1, "e_n": 1, "d": 8}
OUTPUTS = {"q": 8}


def sequences():
    """The truth table as sequences, one input changed per row."""
    sets = []
    vals = [0x00, 0xFF, 0x55, 0xAA, 0x01, 0x80, 0x3C, 0xC3] + [(37 * k + 11) & 0xFF for k in range(24)]
    rows = []
    state = {"cp": 0, "e_n": 0, "d": 0}
    q = [None]

    def row(**kw):
        old_cp = state["cp"]
        state.update(kw)
        if old_cp == 0 and state["cp"] == 1 and state["e_n"] == 0:
            q[0] = state["d"]
        rows.append({**state, "q": q[0]})

    # first load: power-up state not checked
    row(d=0x5A)
    row(cp=1)
    for v in vals:
        row(cp=0)
        row(d=v)              # D moves while CP is low: no change
        row(cp=1)             # load
        row(d=v ^ 0xFF)       # D moves while CP is high: no change
    # /E high: edges don't load
    for v in vals[:8]:
        row(cp=0, e_n=1)
        row(d=v)
        row(cp=1)
        row(d=v ^ 0x0F)
    # /E moves while CP is high or low: never a load (no gated clock)
    for v in vals[8:16]:
        row(cp=0, e_n=1, d=v)
        row(cp=1)
        row(e_n=0)            # /E falls with CP high: no change
        row(e_n=1)            # /E rises with CP high: no change
        row(e_n=0)
        row(cp=0)             # CP falls: no change
        row(cp=1)             # rising edge with /E low: load
        row(cp=0, e_n=1)
        row(e_n=0)            # /E falls with CP low: no change
        row(d=v ^ 0xFF)
        row(e_n=1)
    sets.append(rows)
    return sets


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, M, INPUTS, OUTPUTS, sequences())
    sys.exit(1 if bad else 0)
