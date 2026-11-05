#!/usr/bin/env python3
"""AT28C256 model -> circuits/chips/at28c256.circ (circuits ttlat28c256 and test_at28c256)

32 K x 8 parallel EEPROM (at28c256.pdf next to the models, Atmel 0006F),
used read-only as the boot ROM (ir-mem-decisions.md section 3):

    pinout (PDIP 28)  1 A14, 2 A12, 3 A7, 4 A6, 5 A5, 6 A4, 7 A3, 8 A2, 9 A1,
                      10 A0, 11-13 I/O0-I/O2, 14 GND, 15-19 I/O3-I/O7, 20 /CE,
                      21 A10, 22 /OE, 23 A11, 24 A9, 25 A8, 26 A13, 27 /WE, 28 VCC
    operating modes   /CE /OE /WE | I/O
                       L   L   H  | DOUT    (read)
                       H   X   X  | High-Z  (standby)
                       X   H   X  | High-Z  (output disable)
                      writes (/CE L, /OE H, /WE L) are the programmer's job:
                      not modelled; /WE is tied high on the board.

Each instance of a model shares the model's ROM, so the ROM holds 64 KiB and
a simulation-only input HALF (no pin number, ir-mem-decisions.md M3) picks
the half: image[HALF : A14..A0]. mem ties HALF low on the chip for
0000-7FFF and high on the one for 8000-FFFF.

    python at28c256.py                 # write (test pattern), check
    python at28c256.py --image F.bin   # ... with a 64 KiB boot image instead
    python at28c256.py --test          # ... and run the read / mode sequences in Logisim
"""

import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

PART = "at28c256"
OUT = ROOT / "circuits" / "chips" / f"{PART}.circ"
ORDER = (["A14", "A12", "A7", "A6", "A5", "A4", "A3", "A2", "A1", "A0", "I/O0", "I/O1", "I/O2", "GND"]
         + ["I/O3", "I/O4", "I/O5", "I/O6", "I/O7", "/CE", "A10", "/OE", "A11", "A9", "A8", "A13", "/WE", "VCC"])
EXTRA = ["HALF"]


def pattern() -> bytes:
    """Default image: every byte differs between the two halves, and
    neighbouring addresses differ, so a wrong half or address bit shows."""
    lo = [(x * 37 + (x >> 8) * 11 + 0x5A) & 0xFF for x in range(1 << 15)]
    return bytes(lo + [v ^ 0xFF for v in lo])


def contents(image: bytes) -> str:
    """Logisim's ROM contents attribute: header, then hex values, runs as n*v."""
    assert len(image) == 1 << 16
    out, i = [], 0
    while i < len(image):
        j = i
        while j < len(image) and image[j] == image[i]:
            j += 1
        out.append(f"{j - i}*{image[i]:x}" if j - i > 3 else " ".join([f"{image[i]:x}"] * (j - i)))
        i = j
    words = " ".join(out).split(" ")
    lines = [" ".join(words[k:k + 16]) for k in range(0, len(words), 16)]
    return "addr/data: 16 8\n" + "\n".join(lines) + "\n"


IMAGE = pattern()
if "--image" in sys.argv:
    IMAGE = Path(sys.argv[sys.argv.index("--image") + 1]).read_bytes()

p = Project()
m = p.circuit("ttl" + PART)
y = 100
for name in [f"A{k}" for k in range(15)] + EXTRA + ["/CE", "/OE", "/WE", "VCC", "GND"]:
    if name in ("VCC", "GND", "/WE"):
        m.input((100, y), name)                  # supply pins and /WE: on the symbol only
    else:
        bus_tunnels(m, (100, y), name, 1)
    y += 40
for k in range(8):
    bus_tunnels(m, (1300, 100 + 40 * k), f"I/O{k}", 1, output=True)

RX, RY = 700, 500
sa = m.splitter((RX - 60, RY + 10), 16, [[k] for k in range(16)], facing="west", appear="left", spacing=2)
for k in range(16):
    m.tunnel(sa[f"end{k}"], f"A{k}" if k < 15 else "HALF", facing="east")
m.wire((RX - 60, RY + 10), (RX, RY + 10))
m.add("#Memory", "ROM", (RX, RY), addrWidth=16, dataWidth=8, appearance="logisim_evolution",
      contents=contents(IMAGE))
BX = RX + 300
m.wire((RX + 240, RY + 60), (BX - 20, RY + 60))
m.add("#Gates", "Controlled Buffer", (BX, RY + 60), width=8)
m.wire((BX - 10, RY + 70), (BX - 10, RY + 100))
m.tunnel((BX - 10, RY + 100), "read", facing="north")
m.wire((BX, RY + 60), (BX + 40, RY + 60))
sq = m.splitter((BX + 40, RY + 60), 8, [[k] for k in range(8)], facing="east", appear="right", spacing=2)
for k in range(8):
    m.tunnel(sq[f"end{k}"], f"I/O{k}", facing="west")
# read = NOT /CE AND NOT /OE
GX, GY = 400, 1100
for i, (a, b) in enumerate((("/CE", "ce"), ("/OE", "oe"))):
    m.tunnel((GX - 30, GY + 60 * i), a, facing="east")
    m.add("#Gates", "NOT Gate", (GX, GY + 60 * i))
    m.tunnel((GX, GY + 60 * i), b, facing="west")
x, y0 = 800, 1100
m.add("#Gates", "AND Gate", (x, y0), inputs=4, size=50)
for dy, j, row, name in zip((-20, -10, 10, 20), (0, 1, 1, 0), (-60, -30, 30, 60), ("ce", "oe", "vcc", "vcc")):
    xv = x - 60 - 10 * j
    m.wire((x - 50, y0 + dy), (xv, y0 + dy), (xv, y0 + row), (x - 120, y0 + row))
    if name == "vcc":
        m.add("#Wiring", "Power", (x - 120, y0 + row), facing="east")
    else:
        m.tunnel((x - 120, y0 + row), name, facing="east")
m.wire((x, y0), (x + 20, y0))
m.tunnel((x + 20, y0), "read", facing="west")
m.text((RX + 100, RY - 40), "AT28C256 (Atmel 0006F) as a read-only boot ROM: 64 KiB image, HALF picks the chip's half")
m.dip_symbol("AT28C256", ORDER, EXTRA)

# ---- test bench --------------------------------------------------------------------------------
t = p.circuit(f"test_{PART}")
u = t.model(m, (600, 500), label="U1")
names = {"/CE": "ce_n", "/OE": "oe_n", "/WE": "we_n"}
names.update({f"A{k}": f"a_{k}" for k in range(15)})
names.update({f"I/O{k}": f"q_{k}" for k in range(8)})
pin_tunnels(t, u, names, levels=6, stagger=20)
t.wire(u["HALF"], (u["HALF"][0] - 20, u["HALF"][1]))
t.tunnel((u["HALF"][0] - 20, u["HALF"][1]), "half", facing="east")
for i, name in enumerate(("ce_n", "oe_n", "we_n", "half")):
    bus_tunnels(t, (100, 100 + 60 * i), name, 1)
bus_tunnels(t, (100, 400), "a", 15)
bus_tunnels(t, (1300, 100), "q", 8, output=True)

INPUTS = {"ce_n": 1, "oe_n": 1, "we_n": 1, "half": 1, "a": 15}
OUTPUTS = {"q": 8}


def sequences(image=None):
    """Every address of both halves read, then every mode on random
    addresses (/WE low included: no write, outputs as for /WE high)."""
    import logitest
    image = image or IMAGE
    rng = random.Random(28256)
    rows = []
    for h in (0, 1):
        for a in range(1 << 15):
            rows.append({"ce_n": 0, "oe_n": 0, "we_n": 1, "half": h, "a": a, "q": image[(h << 15) | a]})
    for _ in range(4000):
        r = {"ce_n": rng.randrange(2), "oe_n": rng.randrange(2), "we_n": rng.randrange(2),
             "half": rng.randrange(2), "a": rng.randrange(1 << 15)}
        on = r["ce_n"] == 0 and r["oe_n"] == 0
        r["q"] = image[(r["half"] << 15) | r["a"]] if on else logitest.FLOAT
        rows.append(r)
    return [rows]


if __name__ == "__main__":
    bad = p.save(OUT)
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_sequences(OUT, f"test_{PART}", INPUTS, OUTPUTS, sequences(), buses=["q"])
    sys.exit(1 if bad else 0)
