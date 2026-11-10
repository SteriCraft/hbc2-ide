#!/usr/bin/env python3
"""Bench for Logisim's own 74165 -> circuits/chips/test_74165.circ

The 74HCT165 (8-bit parallel-in / serial-out shift register) is in
Logisim's #TTL library, so it needs no model. The mailbox (iod) uses it as
its TX latch: the CPU loads it from bus_data, the MCU shifts it out. This
bench checks Logisim's chip against the datasheet (cd74hct165.pdf, TI,
truth table 7-1):

    SH/LD low                  parallel load, whatever CLK: QA..QH = A..H,
                               QH = H, /QH = not H (asynchronous)
    SH/LD high, CLK INH low    CLK rising: QA = SER, QB..QH = QA..QG
    SH/LD high, CLK INH high   no change (CLK ignored)

Not checked: a rising CLK INH while CLK is low (the real chip ORs the two,
so that is a clock edge; TI says to raise CLK INH only while CLK is high).
The mailbox raises CLK INH (MBX_SEL) with SCK low after a read: one more
shift of a byte already read, harmless; the model needn't agree.

Logisim's pin names: Shift/Load, Clock, Clock_Inhibit, Serial_Input,
P0..P7 (= A..H), Q7 (= QH), Q7n (= /QH). Same pin numbers as TI: 1 SH/LD,
2 CLK, 3-6 E-H, 7 /QH, 9 QH, 10 SER, 11-14 A-D, 15 CLK INH.

    python test_74165.py           # write, check
    python test_74165.py --test    # ... and run the datasheet sequences in Logisim
"""

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "test_74165"
OUT = ROOT / "circuits" / "chips" / f"{M}.circ"
P_PINS = (11, 12, 13, 14, 3, 4, 5, 6)              # A..H
PINS = {1: "sh_ld_n", 2: "clk", 15: "clk_inh", 10: "ser", 9: "qh", 7: "qh_n",
        **{p: f"p_{k}" for k, p in enumerate(P_PINS)}}

p = Project()
c = p.circuit(M)
u = c.ttl("74165", (500, 300), label="U1")
pin_tunnels(c, u, PINS)
for i, (name, w) in enumerate((("sh_ld_n", 1), ("clk", 1), ("clk_inh", 1), ("ser", 1), ("p", 8))):
    bus_tunnels(c, (100, 100 + 60 * i), name, w)
bus_tunnels(c, (900, 100), "qh", 1, output=True)
bus_tunnels(c, (900, 160), "qh_n", 1, output=True)
c.text((560, 180), "Logisim's own 74165: behaviour vs the TI datasheet")

INPUTS = {"sh_ld_n": 1, "clk": 1, "clk_inh": 1, "ser": 1, "p": 8}
OUTPUTS = {"qh": 1, "qh_n": 1}


def sequences():
    rnd = random.Random(165)
    rows = []
    st = {"sh_ld_n": 1, "clk": 0, "clk_inh": 0, "ser": 0, "p": 0}
    r = [None]

    def row(**kw):
        old = dict(st)
        st.update(kw)
        if st["sh_ld_n"] == 0:
            r[0] = st["p"]
        elif old["clk"] == 0 and st["clk"] == 1 and st["clk_inh"] == 0 and r[0] is not None:
            r[0] = ((r[0] << 1) | st["ser"]) & 0xFF          # bit k = stage A + k: QH = bit 7
        qh = None if r[0] is None else (r[0] >> 7) & 1
        rows.append({**st, "qh": qh, "qh_n": None if qh is None else 1 - qh})

    for _ in range(40):
        v = rnd.randrange(256)
        row(p=v)
        row(sh_ld_n=0)                       # load, CLK low
        row(clk=1)                           # CLK moves during the load: still a load
        row(clk=0)
        row(p=v ^ 0xFF)                      # inputs move during the load: follows them
        row(p=v)
        row(sh_ld_n=1)
        row(p=rnd.randrange(256))            # inputs move after the load: no change
        for _ in range(rnd.randrange(3, 10)):
            row(ser=rnd.randrange(2))
            row(clk=1)                       # shift
            row(clk=0)
        row(clk=1)                           # a shift, then CLK INH rises with CLK high (TI's rule)
        row(clk_inh=1)
        row(clk=0)
        row(clk=1)                           # inhibited: no shift
        row(clk=0)
        row(clk_inh=0)                       # CLK INH falls with CLK low: no edge
    return [rows]


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, M, INPUTS, OUTPUTS, sequences())
    sys.exit(1 if bad else 0)
