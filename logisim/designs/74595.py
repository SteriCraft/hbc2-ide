#!/usr/bin/env python3
"""74HCT595 / SN74AHCT595 model -> circuits/chips/74595.circ (circuits ttl74595 and test_74595)

8-bit shift register with an 8-bit storage register and 3-state outputs.
Logisim's #TTL library has no 74595, so this models it from the TI
datasheet (sn74ahct595.pdf, next to the models, SCLS374Q, 2024; the
board part is SN74AHCT595N, the only DIP '595 TI makes, test-rig.md):

    pinout   1 QB, 2 QC, 3 QD, 4 QE, 5 QF, 6 QG, 7 QH, 8 GND, 9 QH',
             10 /SRCLR, 11 SRCLK, 12 RCLK, 13 /OE, 14 SER, 15 QA, 16 VCC
    table    /OE = H: QA-QH disabled; L: enabled
             /SRCLR = L: the shift register is cleared
             SRCLK rising (/SRCLR high): the first stage takes SER, the
                 others the previous stage's data
             RCLK rising: the shift register's data is stored in the
                 storage register (QA-QH show the storage register)
    QH' is the shift register's last stage (always driven), for cascading.

Inside: the shift register is an 8-bit Register clocked by SRCLK whose D is
its own Q shifted up by one with SER in bit 0 (bit k = stage A + k, so
bit 0 = QA, bit 7 = QH), cleared by NOT /SRCLR (the Register's clear is
asynchronous). The storage register is a second Register clocked by RCLK,
never cleared (the datasheet has no clear for it). The outputs go through
an 8-bit Controlled Buffer enabled by NOT /OE.

    python 74595.py           # write, check
    python 74595.py --test    # ... and run the datasheet sequences in Logisim
"""

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

PART = "74595"
OUT = ROOT / "circuits" / "chips" / f"{PART}.circ"
QS = ["QA", "QB", "QC", "QD", "QE", "QF", "QG", "QH"]
ORDER = QS[1:] + ["GND", "QH'", "/SRCLR", "SRCLK", "RCLK", "/OE", "SER", "QA", "VCC"]

p = Project()
m = p.circuit("ttl" + PART)


def stub(pt, name, width=1, side="R"):
    """A 20 px wire from a port to a tunnel."""
    dx = 20 if side == "R" else -20
    end = (pt[0] + dx, pt[1])
    m.wire(pt, end)
    m.tunnel(end, name, width, facing="west" if side == "R" else "east")


# ---- input pins on tunnels ------------------------------------------------------------------------
y = 100
for name in ("SER", "SRCLK", "/SRCLR", "RCLK", "/OE"):
    m.wire(m.input((100, y), name), (120, y))
    m.tunnel((120, y), name, facing="west")
    y += 40

# ---- shift register: Register R1, D = (Q << 1) | SER ----------------------------------------------
X1, Y1 = 500, 100                       # R1 anchor: D (X1, Y1+30), Q (X1+60, Y1+30)
m.add("#Memory", "Register", (X1, Y1), width=8, appearance="logisim_evolution")
d1 = (X1, Y1 + 30)
m.wire((X1 - 40, Y1 + 30), d1)
sd = m.splitter((X1 - 40, Y1 + 30), 8, [[k] for k in range(8)], facing="west", appear="left", spacing=2)
for k in range(8):
    e = sd[f"end{k}"]
    m.wire(e, (e[0] - 20, e[1]))
    m.tunnel((e[0] - 20, e[1]), "SER" if k == 0 else f"s{k - 1}", facing="east")
stub((X1 + 60, Y1 + 30), "s", 8)
m.wire((X1 - 20, Y1 + 70), (X1, Y1 + 70))
m.tunnel((X1 - 20, Y1 + 70), "SRCLK", facing="east")
m.wire((X1 - 20, Y1 + 50), (X1, Y1 + 50))
m.add("#Wiring", "Power", (X1 - 20, Y1 + 50), facing="west")
# clear = NOT /SRCLR, from below
m.add("#Gates", "NOT Gate", (X1 + 30, Y1 + 130), facing="north")
m.wire((X1 + 30, Y1 + 130), (X1 + 30, Y1 + 90))
m.wire((X1 + 30, Y1 + 160), (X1 + 30, Y1 + 180))
m.tunnel((X1 + 30, Y1 + 180), "/SRCLR", facing="north")

# the stages one by one (for D above, and QH')
XS, YS = 800, 100
ss = m.splitter((XS, YS), 8, [[k] for k in range(8)], facing="east", appear="right", spacing=2)
m.wire((XS - 20, YS), (XS, YS))
m.tunnel((XS - 20, YS), "s", 8, facing="east")
for k in range(8):
    e = ss[f"end{k}"]
    m.wire(e, (e[0] + 20, e[1]))
    m.tunnel((e[0] + 20, e[1]), f"s{k}", facing="west")
m.wire(m.output((1000, 100), "QH'"), (980, 100))
m.tunnel((980, 100), "s7", facing="east")

# ---- storage register R2 (RCLK), outputs through a buffer (NOT /OE) -------------------------------
X2, Y2 = 500, 400
m.add("#Memory", "Register", (X2, Y2), width=8, appearance="logisim_evolution")
m.wire((X2 - 20, Y2 + 30), (X2, Y2 + 30))
m.tunnel((X2 - 20, Y2 + 30), "s", 8, facing="east")
m.wire((X2 - 20, Y2 + 70), (X2, Y2 + 70))
m.tunnel((X2 - 20, Y2 + 70), "RCLK", facing="east")
m.wire((X2 - 20, Y2 + 50), (X2, Y2 + 50))
m.add("#Wiring", "Power", (X2 - 20, Y2 + 50), facing="west")
m.add("#Wiring", "Ground", (X2 + 30, Y2 + 90))
XB = X2 + 140
m.wire((X2 + 60, Y2 + 30), (XB - 20, Y2 + 30))
m.add("#Gates", "Controlled Buffer", (XB, Y2 + 30), width=8)
m.add("#Gates", "NOT Gate", (XB - 10, Y2 + 70), facing="north")
m.wire((XB - 10, Y2 + 70), (XB - 10, Y2 + 40))
m.wire((XB - 10, Y2 + 100), (XB - 10, Y2 + 120))
m.tunnel((XB - 10, Y2 + 120), "/OE", facing="north")
XO = XB + 60
m.wire((XB, Y2 + 30), (XO, Y2 + 30))
so = m.splitter((XO, Y2 + 30), 8, [[k] for k in range(8)], facing="east", appear="right", spacing=2)
for k in range(8):
    e = so[f"end{k}"]
    m.wire(e, m.output((e[0] + 80, e[1]), QS[k]))

# supply pins: on the symbol only
m.input((100, 400), "VCC")
m.input((100, 440), "GND")
m.text((X1 + 30, Y1 - 40), "shift register: bit 0 = QA takes SER on SRCLK rising; cleared by /SRCLR low")
m.text((X2 + 30, Y2 - 40), "storage register: takes the shift register on RCLK rising; QA-QH off when /OE is high")
m.dip_symbol("74HCT595", ORDER)

# ---- test bench --------------------------------------------------------------------------------
t = p.circuit(f"test_{PART}")
u = t.model(m, (400, 400), label="U1")
names = {"/OE": "oe_n", "/SRCLR": "srclr_n", "SRCLK": "srclk", "RCLK": "rclk", "SER": "ser", "QH'": "qh_s"}
names.update({q: f"q_{k}" for k, q in enumerate(QS)})
pin_tunnels(t, u, names)
for i, (name, w) in enumerate((("oe_n", 1), ("srclr_n", 1), ("srclk", 1), ("rclk", 1), ("ser", 1))):
    bus_tunnels(t, (100, 100 + 60 * i), name, w)
bus_tunnels(t, (900, 100), "q", 8, output=True)
bus_tunnels(t, (900, 400), "qh_s", 1, output=True)

INPUTS = {"oe_n": 1, "srclr_n": 1, "srclk": 1, "rclk": 1, "ser": 1}
OUTPUTS = {"q": 8, "qh_s": 1}


def sequences():
    """Shift bytes in (MSB first, like the mailbox's MCU), store them, check
    QA-QH and QH', the outputs off and on, a store while off, the clear."""
    import logitest
    rnd = random.Random(595)
    sets = []
    for _ in range(4):
        rows = []
        st = {"oe_n": 0, "srclr_n": 1, "srclk": 0, "rclk": 0, "ser": 0}
        sh = [None]                        # shift register (None = unknown until 8 shifts or a clear)
        known = [0]                        # how many stages are known
        store = [None]

        def row(**kw):
            old = dict(st)
            st.update(kw)
            if st["srclr_n"] == 0:
                sh[0], known[0] = 0, 8
            elif old["srclk"] == 0 and st["srclk"] == 1:
                sh[0] = ((sh[0] or 0) << 1 | st["ser"]) & 0xFF
                known[0] = min(8, known[0] + 1)
            if old["rclk"] == 0 and st["rclk"] == 1:
                store[0] = sh[0] if known[0] == 8 else None
            qh = (sh[0] >> 7) & 1 if known[0] == 8 else None
            q = logitest.FLOAT if st["oe_n"] else store[0]
            rows.append({**st, "q": q, "qh_s": qh})

        row()
        for _ in range(24):
            v = rnd.randrange(256)
            for b in range(7, -1, -1):        # MSB first: after 8 clocks bit 7 is QH
                row(ser=(v >> b) & 1)
                row(srclk=1)
                row(srclk=0, ser=rnd.randrange(2))
            row(rclk=1)                      # store: QA-QH = v
            row(rclk=0)
            if rnd.random() < 0.3:
                row(oe_n=1)                  # off
                row(oe_n=0)
        row(oe_n=1)
        row(srclr_n=0)                       # clear the shift register: QH' = 0, outputs unchanged
        row(srclr_n=1)
        row(rclk=1)                          # store while off: 00
        row(rclk=0)
        row(oe_n=0)
        row(ser=1)                           # one 1 in: QA stage only
        row(srclk=1)
        row(srclk=0)
        row(rclk=1)
        row(rclk=0)
        sets.append(rows)
    return sets


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, f"test_{PART}", INPUTS, OUTPUTS, sequences())
    sys.exit(1 if bad else 0)
