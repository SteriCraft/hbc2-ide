#!/usr/bin/env python3
"""AS6C1008 model -> circuits/chips/as6c1008.circ (circuits ttlas6c1008 and test_as6c1008)

128 K x 8 static RAM, Alliance Memory (as6c1008.pdf next to the models, v1.2,
Mar 2023):

    pinout (P-DIP 32)  1 NC, 2 A16, 3 A14, 4 A12, 5 A7, 6 A6, 7 A5, 8 A4, 9 A3,
                       10 A2, 11 A1, 12 A0, 13-15 DQ0-DQ2, 16 Vss, 17-21 DQ3-DQ7,
                       22 CE#, 23 A10, 24 OE#, 25 A11, 26 A9, 27 A8, 28 A13,
                       29 WE#, 30 CE2, 31 A15, 32 Vcc
    truth table        CE# CE2 OE# WE# | I/O
                        H   X   X   X  | High-Z   (standby)
                        X   L   X   X  | High-Z   (standby)
                        L   H   H   H  | High-Z   (output disable)
                        L   H   L   H  | DOUT     (read)
                        L   H   X   L  | DIN      (write; outputs off)

A write is a level: while CE# = L, CE2 = H and WE# = L the addressed cell
takes DIN, and keeps the last value when the write ends (WRITE CYCLE 1/2).

Logisim pins are one-way (ir-mem-decisions.md M2), so the DQ pins are
tri-state outputs and the chip reads its data from 8 extra, simulation-only
inputs DQ0_in..DQ7_in (left edge of the symbol, no pin number). Wire both to
the same net: that is what the real pin does.

Inside: a #Memory RAM (17-bit address, separate data in / out, byte
enables, trigger "high": an asynchronous level write), NOT and AND gates
for the truth table, a Controlled Buffer for the outputs. Logisim's RAM
powers up at 0; the real chip's content is random, so no test relies on it.
NC (pin 1) is an output pin left unconnected inside, so it can stay open.

    python as6c1008.py           # write, check
    python as6c1008.py --test    # ... and run the truth-table sequences in Logisim
"""

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

PART = "as6c1008"
OUT = ROOT / "circuits" / "chips" / f"{PART}.circ"
ORDER = (["NC", "A16", "A14", "A12", "A7", "A6", "A5", "A4", "A3", "A2", "A1", "A0", "DQ0", "DQ1", "DQ2", "GND"]
         + ["DQ3", "DQ4", "DQ5", "DQ6", "DQ7", "CE#", "A10", "OE#", "A11", "A9", "A8", "A13", "WE#", "CE2", "A15",
            "VCC"])
EXTRA = [f"DQ{k}_in" for k in range(8)]

p = Project()
m = p.circuit("ttl" + PART)

# ---- pins, each on a tunnel of the same name -------------------------------------------------
y = 100
for name in [f"A{k}" for k in range(17)] + EXTRA + ["CE#", "CE2", "OE#", "WE#", "VCC", "GND"]:
    if name in ("VCC", "GND"):
        m.input((100, y), name)                  # supply pins: on the symbol only
    else:
        bus_tunnels(m, (100, y), name, 1)
    y += 40
for k in range(8):
    bus_tunnels(m, (1300, 100 + 40 * k), f"DQ{k}", 1, output=True)
m.output((1300, 100 + 40 * 8), "NC")              # not connected inside

# ---- address and data in: splitters into the RAM --------------------------------------------
RX, RY = 700, 500
sa = m.splitter((RX - 60, RY + 10), 17, [[k] for k in range(17)], facing="west", appear="left", spacing=2)
for k in range(17):
    m.tunnel(sa[f"end{k}"], f"A{k}", facing="east")
m.wire((RX - 60, RY + 10), (RX, RY + 10))
sd = m.splitter((RX - 60, RY + 480), 8, [[k] for k in range(8)], facing="west", appear="left", spacing=2)
for k in range(8):
    m.tunnel(sd[f"end{k}"], f"DQ{k}_in", facing="east")
m.wire((RX - 60, RY + 480), (RX - 30, RY + 480), (RX - 30, RY + 80), (RX, RY + 80))
m.add("#Memory", "RAM", (RX, RY), addrWidth=17, dataWidth=8, enables="byte", trigger="high",
      databus="bibus", appearance="logisim_evolution", type="volatile")
m.wire((RX, RY + 50), (RX - 20, RY + 50))
m.tunnel((RX - 20, RY + 50), "write", facing="east")
m.add("#Wiring", "Power", (RX - 10, RY + 60), facing="west")
m.wire((RX - 10, RY + 60), (RX, RY + 60))        # the RAM's own OE: always on (the buffer gates)

# ---- outputs: buffer enabled by "read", splitter to DQ0..DQ7 ---------------------------------
BX = RX + 300
m.wire((RX + 240, RY + 80), (BX - 20, RY + 80))
m.add("#Gates", "Controlled Buffer", (BX, RY + 80), width=8)
m.wire((BX - 10, RY + 90), (BX - 10, RY + 120))
m.tunnel((BX - 10, RY + 120), "read", facing="north")
m.wire((BX, RY + 80), (BX + 40, RY + 80))
sq = m.splitter((BX + 40, RY + 80), 8, [[k] for k in range(8)], facing="east", appear="right", spacing=2)
for k in range(8):
    m.tunnel(sq[f"end{k}"], f"DQ{k}", facing="west")

# ---- truth table: NOT gates, two AND gates ---------------------------------------------------
GX, GY = 400, 1100
for i, (a, y_) in enumerate((("CE#", "ce"), ("OE#", "oe"), ("WE#", "we"))):
    yy = GY + 60 * i
    m.tunnel((GX - 30, yy), a, facing="east")
    m.add("#Gates", "NOT Gate", (GX, yy))
    m.tunnel((GX, yy), y_, facing="west")


def and4(loc, ins, out):
    """AND Gate (4 inputs, size 50) at loc; each input fans out to a tunnel."""
    x, y0 = loc
    m.add("#Gates", "AND Gate", loc, inputs=4, size=50)
    for dy, j, row, name in zip((-20, -10, 10, 20), (0, 1, 1, 0), (-60, -30, 30, 60), ins):
        xv = x - 60 - 10 * j
        m.wire((x - 50, y0 + dy), (xv, y0 + dy), (xv, y0 + row), (x - 120, y0 + row))
        if name == "vcc":
            m.add("#Wiring", "Power", (x - 120, y0 + row), facing="east")
        else:
            m.tunnel((x - 120, y0 + row), name, facing="east")
    m.wire(loc, (x + 20, y0))
    m.tunnel((x + 20, y0), out, facing="west")


and4((800, 1100), ["ce", "CE2", "we", "vcc"], "write")     # CE# L, CE2 H, WE# L
and4((800, 1300), ["ce", "CE2", "oe", "WE#"], "read")      # CE# L, CE2 H, OE# L, WE# H
m.text((RX + 100, RY - 40), "AS6C1008 (Alliance v1.2): 128 K x 8 SRAM, level write, outputs off while writing")
m.dip_symbol("AS6C1008", ORDER, EXTRA)

# ---- test bench --------------------------------------------------------------------------------
t = p.circuit(f"test_{PART}")
u = t.model(m, (600, 500), label="U1")
names = {"CE#": "ce_n", "CE2": "ce2", "OE#": "oe_n", "WE#": "we_n"}
names.update({f"A{k}": f"a_{k}" for k in range(17)})
names.update({f"DQ{k}": f"dq_{k}" for k in range(8)})
pin_tunnels(t, u, names, levels=6, stagger=20)
for k, lab in enumerate(EXTRA):                  # extras: 10 px apart on the left, staggered stubs
    x, y = u[lab]
    end = (x - (20 if k % 2 == 0 else 100), y)
    t.wire((x, y), end)
    t.tunnel(end, f"din_{k}", facing="east")
for i, (name, w) in enumerate((("ce_n", 1), ("ce2", 1), ("oe_n", 1), ("we_n", 1))):
    bus_tunnels(t, (100, 100 + 60 * i), name, w)
bus_tunnels(t, (100, 400), "a", 17)
bus_tunnels(t, (100, 820), "din", 8)
bus_tunnels(t, (1300, 100), "dq", 8, output=True)

INPUTS = {"ce_n": 1, "ce2": 1, "oe_n": 1, "we_n": 1, "a": 17, "din": 8}
OUTPUTS = {"dq": 8}


def sequences():
    """The truth table as sequences, against a Python dict of the written
    cells: writes (data changing during the write, the last value kept),
    writes inhibited by CE# / CE2, every mode's outputs, reads of written
    cells only (power-up content is not defined on the real chip)."""
    import logitest
    rng = random.Random(1008)
    sets = []
    for _ in range(4):
        rows, mem = [], {}
        st = {"ce_n": 1, "ce2": 0, "oe_n": 1, "we_n": 1, "a": 0, "din": 0}

        def row(**kw):
            st.update(kw)
            sel = st["ce_n"] == 0 and st["ce2"] == 1
            if sel and st["we_n"] == 0:
                mem[st["a"]] = st["din"]
            q = mem.get(st["a"]) if sel and st["oe_n"] == 0 and st["we_n"] == 1 else logitest.FLOAT
            if q is None:
                return                           # unwritten cell: not checked, row skipped
            rows.append({**st, "dq": q})

        addrs = [0, 1, 0x1FFFF, 0x10000, 0x0FFFF] + [rng.randrange(1 << 17) for _ in range(60)]
        for a in addrs:
            # a write with OE# low (outputs off while WE# is low), the data changing during it
            row(a=a, din=rng.randrange(256), ce_n=0, ce2=1, oe_n=rng.randrange(2), we_n=1)
            row(we_n=0)
            row(din=rng.randrange(256))
            row(we_n=1)
        for a in addrs:                          # writes inhibited: nothing changes
            row(a=a, din=rng.randrange(256), ce_n=1, ce2=rng.randrange(2), we_n=0)
            row(we_n=1)
            row(ce_n=rng.randrange(2), ce2=0, we_n=0)
            row(we_n=1)
        for a in addrs:                          # every mode: read, output disable, standby
            for ce_n, ce2, oe_n in ((0, 1, 0), (0, 1, 1), (1, 1, 0), (0, 0, 0), (1, 0, 0)):
                row(a=a, ce_n=ce_n, ce2=ce2, oe_n=oe_n, we_n=1)
        sets.append(rows)
    return sets


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, f"test_{PART}", INPUTS, OUTPUTS, sequences(), buses=["dq"])
    sys.exit(1 if bad else 0)
