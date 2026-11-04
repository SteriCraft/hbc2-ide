#!/usr/bin/env python3
"""Bench for Logisim's own 74670 -> circuits/chips/test_74670.circ

The 74HCT670 (4 x 4 register file) is in Logisim's #TTL library, so it needs
no model. The register file design depends on one awkward behaviour, so this
bench checks Logisim's chip against the datasheet (cd74hc670.pdf, next to the
models), write and read mode select tables:

    /WE low   the addressed word takes D and stays transparent: Q follows D
              when RA = WA (data to output 63 ns max at 85 °C, HCT)
    /WE high  the words keep their value, D and WA are ignored
    /RE low   Q = the word at RA, whatever /WE does (note 2)
    /RE high  Q is high impedance

Logisim names the pins 1-based (D1..D4, Q1..Q4); TI names them D0..D3,
Q0..Q3. Same pins, so the bench wires them by number.

    python test_74670.py           # write, check
    python test_74670.py --test    # ... and run the datasheet sequences in Logisim
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "test_74670"
OUT = ROOT / "circuits" / "chips" / f"{M}.circ"

# TI pinout: 1 D1, 2 D2, 3 D3, 4 RA1, 5 RA0, 6 Q3, 7 Q2, 8 GND, 9 Q1, 10 Q0,
#            11 /RE, 12 /WE, 13 WA1, 14 WA0, 15 D0, 16 VCC
PINS = {15: "d_0", 1: "d_1", 2: "d_2", 3: "d_3", 14: "wa_0", 13: "wa_1", 5: "ra_0", 4: "ra_1",
        12: "we_n", 11: "re_n", 10: "q_0", 9: "q_1", 7: "q_2", 6: "q_3"}

p = Project()
c = p.circuit(M)
u = c.ttl("74670", (500, 300), label="U1")
pin_tunnels(c, u, PINS)
for i, (name, w) in enumerate((("we_n", 1), ("re_n", 1), ("wa", 2), ("ra", 2), ("d", 4))):
    bus_tunnels(c, (100, 100 + 90 * i), name, w)
bus_tunnels(c, (900, 100), "q", 4, output=True)
c.text((560, 180), "Logisim's own 74670: behaviour vs the TI datasheet")

INPUTS = {"we_n": 1, "re_n": 1, "wa": 2, "ra": 2, "d": 4}
OUTPUTS = {"q": 4}


def sequences():
    """The datasheet tables as sequences. Rows change one thing at a time
    around /WE edges (setup and hold: D and WA stay put while /WE rises)."""
    import logitest
    sets = []
    init = [0x5, 0xA, 0x3, 0xC]
    for wa in range(4):
        for ra in range(4):
            words = list(init)
            rows = []
            state = {"we_n": 1, "re_n": 0, "wa": 0, "ra": ra, "d": 0}

            def row(**kw):
                state.update(kw)
                if state["we_n"] == 0:
                    words[state["wa"]] = state["d"]
                q = logitest.FLOAT if state["re_n"] else words[state["ra"]]
                rows.append({**state, "q": q})

            # load the four words (first rows: q is unknown, so not checked)
            for a, v in enumerate(init):
                row(wa=a, d=v)
                row(we_n=0)
                row(we_n=1)
            for r in rows:
                r["q"] = None
            row()
            # /WE low on wa: the word follows D (Q too when ra = wa)
            row(wa=wa)
            row(we_n=0)
            for v in range(16):
                row(d=v)
            row(d=9)
            row(we_n=1)                  # latched: 9
            for v in range(16):          # D ignored now
                row(d=v)
            for a in range(4):           # WA ignored now
                row(wa=a)
            row(re_n=1)                  # off: high impedance
            row(we_n=0, wa=ra, d=6)      # write while off: still Z
            row(we_n=1)
            row(re_n=0)                  # 6 in the read word now
            sets.append(rows)
    return sets


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, M, INPUTS, OUTPUTS, sequences())
    sys.exit(1 if bad else 0)
