#!/usr/bin/env python3
"""74HCT574 model -> circuits/chips/74574.circ (circuits 74574 and test_74574)

Octal D flip-flop, positive-edge triggered, 3-state outputs. Logisim's #TTL
library has no 74574, so this models it from the TI datasheet
(cd74hct574.pdf, next to the models, SCHS183E):

    pinout   1 /OE, 2-9 D0-D7, 10 GND, 11 CP, 12-19 Q7-Q0, 20 VCC
    table    /OE  CP  Dn | Qn
              L   ^   H  | H
              L   ^   L  | L
              L   L   X  | Q0 (no change)
              H   X   X  | Z
    "/OE controls the 3-state outputs and is independent of the register
    operation": the flip-flops still load while the outputs are off.

Inside: one 8-bit Register (eight D flip-flops: CP clocks it, never
disabled, never cleared), an 8-bit Controlled Buffer for the outputs, and a
NOT gate because /OE is active low. VCC and GND are pins of the symbol,
unconnected inside, like Logisim's own TTL chips.

    python 74574.py           # write, check
    python 74574.py --test    # ... and run the datasheet sequences in Logisim
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

PART = "74574"
OUT = ROOT / "circuits" / "chips" / f"{PART}.circ"
ORDER = ["/OE"] + [f"D{k}" for k in range(8)] + ["GND", "CP"] + [f"Q{k}" for k in range(7, -1, -1)] + ["VCC"]

p = Project()
m = p.circuit(PART)

X_PIN, X_SP, X_R, Y0 = 100, 200, 280, 200       # D pins, D splitter, register; Y0 = the 8-bit data line
X_B = X_R + 100                                  # output buffer
X_S2, X_Q = X_B + 40, X_B + 120                  # output splitter, Q pins

# D0..D7 -> splitter -> register D
sp = m.splitter((X_SP, Y0), 8, [[k] for k in range(8)], facing="west", appear="left", spacing=2)
for k in range(8):
    e = sp[f"end{k}"]
    m.wire(m.input((X_PIN, e[1]), f"D{k}"), e)
m.wire((X_SP, Y0), (X_R, Y0))
reg = m.add("#Memory", "Register", (X_R, Y0 - 30), width=8, appearance="logisim_evolution")
# CP from below, register always enabled, never cleared
y_cp = Y0 + 190
m.wire(m.input((X_PIN, y_cp), "CP"), (X_R - 30, y_cp), (X_R - 30, Y0 + 40), (X_R, Y0 + 40))
m.add("#Wiring", "Power", (X_R - 10, Y0 + 20), facing="west")
m.wire((X_R - 10, Y0 + 20), (X_R, Y0 + 20))
m.add("#Wiring", "Ground", (X_R + 30, Y0 + 60))
# Q -> buffer, enabled by NOT /OE -> splitter -> Q0..Q7
m.wire((X_R + 60, Y0), (X_B - 20, Y0))
m.add("#Gates", "Controlled Buffer", (X_B, Y0), width=8)
m.add("#Gates", "NOT Gate", (X_B - 10, Y0 + 40), facing="north")
m.wire((X_B - 10, Y0 + 40), (X_B - 10, Y0 + 10))
m.wire(m.input((X_B - 10, Y0 + 120), "/OE", facing="north"), (X_B - 10, Y0 + 70))
m.wire((X_B, Y0), (X_S2, Y0))
sq = m.splitter((X_S2, Y0), 8, [[k] for k in range(8)], facing="east", appear="right", spacing=2)
for k in range(8):
    e = sq[f"end{k}"]
    m.wire(e, m.output((X_Q, e[1]), f"Q{k}"))
# supply pins: on the symbol only
m.input((X_PIN, Y0 + 250), "VCC")
m.input((X_PIN, Y0 + 290), "GND")
m.text((X_R + 30, Y0 - 60), "74HCT574 (TI SCHS183E): edge-triggered, outputs off when /OE is high")
m.dip_symbol("74HCT574", ORDER)

# ---- test bench: the model, its pins on tunnels --------------------------------------
t = p.circuit(f"test_{PART}")
u = t.model(m, (400, 400), label="U1")
names = {"/OE": "oe_n", "CP": "cp"}
names.update({f"D{k}": f"d_{k}" for k in range(8)})
names.update({f"Q{k}": f"q_{k}" for k in range(8)})
pin_tunnels(t, u, names)
for i, (name, w) in enumerate((("oe_n", 1), ("cp", 1), ("d", 8))):
    bus_tunnels(t, (100, 100 + 60 * i), name, w)
bus_tunnels(t, (900, 100), "q", 8, output=True)

INPUTS = {"oe_n": 1, "cp": 1, "d": 8}
OUTPUTS = {"q": 8}


def sequences():
    """The truth table as sequences: every byte captured on a rising edge,
    held through D changes and the falling edge, outputs off and on."""
    import logitest
    sets = []
    for start in (0x00, 0xFF):
        rows, state, q = [], {"oe_n": 0, "cp": 0, "d": start}, [None]

        def row(edge=False, **kw):
            state.update(kw)
            if edge:
                q[0] = state["d"]
            rows.append({**state, "q": logitest.FLOAT if state["oe_n"] else q[0]})

        row()                                    # power-up: Q unknown
        row(cp=1, edge=True)
        row(cp=0)
        for v in range(256):
            v ^= start
            row(d=v)                             # CP low: no change
            row(cp=1, edge=True)                 # rising edge: Q = D
            row(d=v ^ 0xFF)                      # CP high: no change
            row(cp=0)                            # falling edge: no change
        row(oe_n=1)                              # outputs off
        row(d=0xA5)
        row(cp=1, edge=True)                     # loads while off
        row(cp=0)
        row(oe_n=0)                              # back on: A5
        sets.append(rows)
    return sets


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, f"test_{PART}", INPUTS, OUTPUTS, sequences())
    sys.exit(1 if bad else 0)
