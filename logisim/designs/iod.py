#!/usr/bin/env python3
"""I/O mailbox (C-min) -> circuits/iod/iod.circ

The CPU's I/O card: a mailbox of latches and flip-flops in front of an ESP32
I/O processor (notes/iod-requirements.md "Target design", I10-I16;
notes/io-redesign-proposal.md sections 9 and 9.8). 12 chips:

    U?  74HCT138   decode: G1 = IE, /G2A = A7, /G2B = A6; A = A0, B = RW, C = A1
                   /Y0 = OUT TX, /Y2 = IN RX, /Y3 = IN STATUS (ports 0x00-0x3F,
                   A2-A5 ignored; /Y4-/Y7 unused: a second MCU goes behind the
                   ESP32, never on the CPU bus)
        74HCT32    tx_wr_n = /Y0 OR clk_wr_n (low 250-375 ns of an OUT TX step)
        74HCT74 x4 TX full   /PRE = tx_wr_n (CPU), CLK = MBX_SEL rising with D = 0
                             (MCU, end of its read), /CLR = rst_n
                   RX full   CLK = RCLK_RX rising with D = 1 (MCU), /CLR = rx_clr_n
                   rx_clr    D = /Y2 on clk: low for the step after an IN RX; /CLR = rst_n
                   EVT       CLK = RCLK_EVT rising with D = 1 (MCU), /CLR = evt_clr_n
                   evt_clr   D = /VEC on clk: low for the step after the vector step
                   VEC       D = INR on clk: the vector step; /CLR = rst_n
                   evt_s     D = EVT on clk, /CLR = /VEC: INT (I5), 0 in the vector step
        74HCT165   TX: /PL = tx_wr_n loads bus_data, SCK shifts it out (QH = bit 7
                   first) while MBX_SEL (CLK INH) is low
        74HCT595 x3 (model) RX: SER = MOSI, RCLK_RX, /OE = /Y2 -> bus_data
                   EVT port: SER = MOSI, RCLK_EVT, /OE = /VEC -> bus_addr A0-A7
                   EVT data: SER = the port 595's QH', RCLK_EVT, /OE = /VEC -> bus_data
        74HCT244   STATUS -> bus_data (/1G = /2G = /Y3): bit 0 RX full, bit 1 TX
                   free, bit 2 EVT pending, bits 3-7 = 0
        74LVC244   (a 74244 here) 5 V -> 3.3 V for the ESP32: MISO (enabled by
                   MBX_SEL low), TX full, RX full, EVT pending, rst_n

A8-A15 are not touched: on the board a 10 kOhm network pulls them to 0 in
the vector step (I6). A Pull Resistor here would drive the module's output
pin to 0 and fight the address latch, so the simulation leaves them floating
there, as the old stub did (io-redesign-proposal.md 9.9).

The reference for the tests is the cycle-level model below (Card), until the
emulator gets its mailbox (step C3): `hbc2 vectors iod` then replaces it.

    python iod.py           # write, check, and write notes/bom-iod.csv
    python iod.py --test    # ... and run the sequences in Logisim
    python iod.py --png     # ... and draw it next to the .circ
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from circgen import Project, bus_tunnels, pin_tunnels  # noqa: E402

M = "iod"
OUT = ROOT / "circuits" / "iod" / f"{M}.circ"
P = f"{M}_"

p = Project()
c = p.circuit(M)
lib595 = p.library(ROOT / "circuits" / "chips" / "74595.circ")
COL = {k: 600 + 440 * k for k in range(4)}
ROW = {0: 200, 1: 560, 2: 1060}
chips = []


def place(u, pins, note, x, y, **kw):
    chips.append(u)
    pin_tunnels(c, u, pins, **kw)
    if note:
        c.text((x, y - 60), note)
    return u


def chip(part, col, row, pins, note=""):
    return place(c.ttl(part, (COL[col], ROW[row]), facing="south"), pins, note, COL[col] - 10, ROW[row])


def sr595(col, pins, note):
    x, y = COL[col] - 60, ROW[2] + 100              # DIP symbol: anchor = pin 1, bottom left
    return place(c.sub(lib595, "ttl74595", (x, y)), pins, note, x + 60, y - 160, stagger=20, levels=6)


def s(name):
    return P + name


def bits(name, ks):
    return {k: f"{P}{name}_{k}" for k in ks}


BUS_OUT = [s(f"bus_out_{k}") for k in range(8)]
ADDR_OUT = [s(f"addr_out_{k}") for k in range(8)]

# ---- row 0: decode, strobe, the TX / RX / rx_clr / EVT flip-flops ----------------------------------
chip("74138", 0, 0, {"A": s("addr_in_0"), "B": s("rw"), "C": s("addr_in_1"), "G1": s("ie"),
                     "nG2A": s("addr_in_7"), "nG2B": s("addr_in_6"),
                     "nY0": s("y0_n"), "nY2": s("y2_n"), "nY3": s("y3_n")},
     "decode: /Y0 OUT TX, /Y2 IN RX, /Y3 IN STATUS")
chip("7432", 1, 0, {"1A": s("y0_n"), "1B": "clk_wr_n", "1Y": s("tx_wr_n"),
                    "2A": "gnd", "2B": "gnd", "3A": "gnd", "3B": "gnd", "4A": "gnd", "4B": "gnd"},
     "TX strobe: /Y0 OR clk_wr_n")
chip("7474", 2, 0, {"nCLR1": "rst_n", "D1": "gnd", "CLK1": s("mbx_sel"), "nPRE1": s("tx_wr_n"),
                    "Q1": s("tx_full"), "nQ1": s("tx_free"),
                    "nCLR2": s("rx_clr_n"), "D2": "vcc", "CLK2": s("rclk_rx"), "nPRE2": "vcc",
                    "Q2": s("rx_full")},
     "TX full (CPU sets, MCU clears) | RX full (MCU sets, CPU clears)")
chip("7474", 3, 0, {"nCLR1": "rst_n", "D1": s("y2_n"), "CLK1": "clk", "nPRE1": "vcc", "Q1": s("rx_clr_n"),
                    "nCLR2": s("evt_clr_n"), "D2": "vcc", "CLK2": s("rclk_evt"), "nPRE2": "vcc",
                    "Q2": s("evt_pend")},
     "rx_clr (step after IN RX) | EVT pending (MCU sets)")

# ---- row 1: evt_clr / VEC, evt_s, TX 165, STATUS 244 ------------------------------------------------
chip("7474", 0, 1, {"nCLR1": "rst_n", "D1": s("vec_n"), "CLK1": "clk", "nPRE1": "vcc", "Q1": s("evt_clr_n"),
                    "nCLR2": "rst_n", "D2": s("inr"), "CLK2": "clk", "nPRE2": "vcc",
                    "Q2": s("vec"), "nQ2": s("vec_n")},
     "evt_clr (step after the vector step) | VEC (INR delayed one step)")
chip("7474", 1, 1, {"nCLR1": s("vec_n"), "D1": s("evt_pend"), "CLK1": "clk", "nPRE1": "vcc", "Q1": s("int"),
                    "nCLR2": "vcc", "D2": "gnd", "CLK2": "gnd", "nPRE2": "vcc"},
     "evt_s = INT (on clk, 0 in the vector step) | spare")
chip("74165", 2, 1, {"Shift/Load": s("tx_wr_n"), "Clock": s("sck"), "Clock_Inhibit": s("mbx_sel"),
                     "Serial_Input": "gnd", "Q7": s("tx_q"),
                     **{f"P{k}": s(f"bus_in_{k}") for k in range(8)}},
     "TX: loaded from bus_data, shifted out to the MCU (bit 7 first)")
chip("74244", 3, 1, {"n1G": s("y3_n"), "n2G": s("y3_n"),
                     "1A1": s("rx_full"), "1A2": s("tx_free"), "1A3": s("evt_pend"), "1A4": "gnd",
                     "2A1": "gnd", "2A2": "gnd", "2A3": "gnd", "2A4": "gnd",
                     "1Y1": BUS_OUT[0], "1Y2": BUS_OUT[1], "1Y3": BUS_OUT[2], "1Y4": BUS_OUT[3],
                     "2Y1": BUS_OUT[4], "2Y2": BUS_OUT[5], "2Y3": BUS_OUT[6], "2Y4": BUS_OUT[7]},
     "STATUS: RX full, TX free, EVT pending, 0 x 5")

# ---- row 2: the three 595s, the 3.3 V buffer -----------------------------------------------------------
QS = ["QA", "QB", "QC", "QD", "QE", "QF", "QG", "QH"]
sr595(0, {"SER": s("mosi"), "SRCLK": s("sck"), "RCLK": s("rclk_rx"), "/SRCLR": "vcc", "/OE": s("y2_n"),
          **{q: BUS_OUT[k] for k, q in enumerate(QS)}},
      "RX: from the MCU -> bus_data (IN RX)")
sr595(1, {"SER": s("mosi"), "SRCLK": s("sck"), "RCLK": s("rclk_evt"), "/SRCLR": "vcc", "/OE": s("vec_n"),
          "QH'": s("evt_link"), **{q: ADDR_OUT[k] for k, q in enumerate(QS)}},
      "EVT port -> bus_addr A0-A7 (vector step)")
sr595(2, {"SER": s("evt_link"), "SRCLK": s("sck"), "RCLK": s("rclk_evt"), "/SRCLR": "vcc", "/OE": s("vec_n"),
          **{q: BUS_OUT[k] for k, q in enumerate(QS)}},
      "EVT data -> bus_data (vector step, -> I)")
chip("74244", 3, 2, {"n1G": s("mbx_sel"), "n2G": "gnd",
                     "1A1": s("tx_q"), "1A2": "gnd", "1A3": "gnd", "1A4": "gnd", "1Y1": s("miso"),
                     "2A1": s("tx_full"), "2A2": s("rx_full"), "2A3": s("evt_pend"), "2A4": "rst_n",
                     "2Y1": s("mcu_tx_full"), "2Y2": s("mcu_rx_full"), "2Y3": s("mcu_evt_pend"),
                     "2Y4": s("mcu_rst_n")},
     "74LVC244 at 3.3 V: to the ESP32")

# chips numbered top to bottom (by the body's top edge), then left to right
def body_top(u):
    ys = [q.loc[1] for q in u.ports]
    return min(ys)


for i, u in enumerate(sorted(chips, key=lambda u: (body_top(u), u.loc[0])), 1):
    u.attrs["label"] = f"U{i}"

# ---- module pins ------------------------------------------------------------------------------------
X_IN, y = 220, 120
INPUTS = [("iod_bus_in", 8), ("iod_addr_in", 8), ("iod_ie", 1), ("iod_rw", 1), ("iod_inr", 1),
          ("clk", 1), ("clk_wr_n", 1), ("rst_n", 1),
          ("iod_sck", 1), ("iod_mosi", 1), ("iod_rclk_rx", 1), ("iod_rclk_evt", 1), ("iod_mbx_sel", 1)]
for name, w in INPUTS:
    names = None
    if name == "iod_addr_in":       # only A0, A1, A6, A7 are decoded (A2-A5 ignored: the mirrors)
        names = [s(f"addr_in_{k}") if k in (0, 1, 6, 7) else s(f"addr_in_{k}_nc") for k in range(8)]
    bus_tunnels(c, (X_IN, y), name, w, names=names)
    y += 60 if w == 1 else 20 * w + 40
X_OUT, y = COL[3] + 500, 120
OUTPUTS = [("iod_bus_out", 8), ("iod_addr_out", 8), ("iod_int", 1), ("iod_miso", 1),
           ("iod_mcu_tx_full", 1), ("iod_mcu_rx_full", 1), ("iod_mcu_evt_pend", 1), ("iod_mcu_rst_n", 1)]
for name, w in OUTPUTS:
    bus_tunnels(c, (X_OUT, y), name, w, output=True)
    y += 60 if w == 1 else 20 * w + 40

c.symbol(top=["iod_ie", "iod_rw", "iod_inr", "clk", "clk_wr_n", "rst_n"],
         left=["iod_bus_in", "iod_addr_in", "iod_sck", "iod_mosi", "iod_rclk_rx", "iod_rclk_evt", "iod_mbx_sel"],
         right=["iod_bus_out", "iod_addr_out", "iod_int", "iod_miso", "iod_mcu_tx_full", "iod_mcu_rx_full",
                "iod_mcu_evt_pend", "iod_mcu_rst_n"])


# ==== test ==============================================================================================
# The reference is the emulator's own model of the card (hbc2-core/src/mailbox.rs): `hbc2 vectors iod`
# (hbc2-cli/src/vectors/iod.rs) drives its chip functions from pin rows, in 4 sequences: every port IN and
# OUT, both handshakes with the 1 us rule broken at random, events and the vector step, reset.


if __name__ == "__main__":
    OUT.parent.mkdir(exist_ok=True)
    bad = p.save(OUT)
    import bom
    text, errors = bom.bom(str(OUT), M)
    for e in errors:
        print("error  " + e)
    (ROOT / "notes" / "bom-iod.csv").write_text(text, encoding="utf8", newline="\n")
    print("wrote notes/bom-iod.csv")
    bad += len(errors)
    if "--png" in sys.argv:
        import netlist
        import render
        print(render.render(netlist.Project(OUT).circuits[M], OUT.with_suffix(".png")))
    if "--test" in sys.argv:
        import logitest
        bad += not logitest.opens(OUT)
        bad += not logitest.check_emulator(OUT, M, "iod")
    sys.exit(1 if bad else 0)
