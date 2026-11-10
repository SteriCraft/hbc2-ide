#!/usr/bin/env python3
"""Worst-case delays of the ALU, flag-register, register-section and address-section paths, from the "Delays" table of
logisim/notes/chips.md (TI datasheets, CL = 50 pF).

Each path is a start (t = 0, or the latest of other paths) followed by a
list of chip stages. The three columns (typ, max 25 °C, max 85 °C) are
added up separately. Prints a Markdown table for alu-report.md
(--section alu), flags-decisions.md (--section flags) or
registers-decisions.md (--section regs).

Pass criterion (Gianni, 2026-11-04): every path with a limit (LIMITS) must
meet it at **max 25 °C**, bus loading included. Max 85 °C is reported too,
for information only. The exit code is 1 if a path fails at max 25 °C.

The paths are written by hand from the circuits (designs/*.py): change
them here when a module changes.

    python timing.py            # the ALU table
    python timing.py --section flags   # the flag register, within one 500 ns step
    python timing.py --section regs    # the register section and the end-to-end ALU step
    python timing.py --section addr    # PC, SP, the address adder and latch
    python timing.py --section mem     # the memory section, the fetch into the IR
    python timing.py --section cu      # the control unit: sequencer, clock generator, clock branches
    python timing.py --duty 45         # ... with the oscillator high 45 % of its period (45-55 %, Q9)
    python timing.py --limit 250   # ... and exit 1 if a shown path is over 250 ns at max 25 °C
"""

from __future__ import annotations

import argparse
import math
import re
import sys
from pathlib import Path

CHIPS_MD = Path(__file__).resolve().parent.parent / "notes" / "chips.md"


def delays(path: Path = CHIPS_MD) -> dict[tuple[str, str], tuple[int, int, int]]:
    """(part, path) -> (typ, max 25 °C, max 85 °C), from the Delays table."""
    text = path.read_text(encoding="utf8")
    m = re.search(r"^## Delays\n(.*?)(?=^## )", text, re.S | re.M)
    if not m:
        raise SystemExit(f"{path}: no '## Delays' section")
    out = {}
    for line in m.group(1).splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if line.startswith("|") and len(cells) >= 5 and cells[2].isdigit():
            out[(cells[0], cells[1])] = tuple(int(c) for c in cells[2:5])
    return out


# name: (shown, label, start, stages); start = names of earlier paths (latest one wins)
# A stage is (part, datasheet path, what it does here).
PATHS = {
    "b_c4": (False, "b to low C4", [], [
        ("74HCT86", "A/B → Y", "add_sub8: b xor sub"),
        ("74HCT283", "A/B → C4", "U6: low nibble carry"),
    ]),
    "cin_c4": (False, "carry in to low C4", [], [
        ("74HCT08", "A/B → Y", "add_sub8: use_c and cin"),
        ("74HCT86", "A/B → Y", "add_sub8: xor sub"),
        ("74HCT283", "Cin → C4", "U6: low nibble carry"),
    ]),
    "hi_s": (False, "high nibble sums", [], [
        ("74HCT86", "A/B → Y", "add_sub8: b xor sub"),
        ("74HCT283", "A/B → S", "U2 / U3: both carries in"),
        ("74HCT157", "I → Y", "U4: data"),
    ]),
    "select": (False, "carry select", ["b_c4", "cin_c4"], [
        ("74HCT157", "S → Y", "U4: low C4 picks cin 0 / 1"),
    ]),
    "sum": (True, "alu_r, arithmetic (bit 7)", ["select", "hi_s"], [
        ("74HCT541", "A → Y", "rmux8: sum buffer"),
    ]),
    "en138": (False, "decoder output", [], [
        ("74HCT138", "A/B/C → Y", "rmux8: op2..op0"),
    ]),
    "logic_data": (False, "logic data", [], [
        ("74HCT86", "A/B → Y", "logic8: slowest of 08/32/86"),
        ("74HCT541", "A → Y", "rmux8"),
    ]),
    "logic_en": (False, "logic enable", ["en138"], [
        ("74HCT541", "OE → Y", "rmux8: buffer enable"),
    ]),
    "logic": (True, "alu_r, AND OR XOR", ["logic_data", "logic_en"], []),
    "not_data": (False, "NOT data", [], [
        ("74HCT240", "A → Y", "rmux8: not a"),
    ]),
    "not_en": (False, "NOT enable", ["en138"], [
        ("74HCT240", "OE → Y", "rmux8: 240 enable"),
    ]),
    "not": (True, "alu_r, NOT", ["not_data", "not_en"], []),
    "shift_data": (False, "shift data", [], [
        ("74HCT153", "I → Y", "shift8: ASR fill"),
        ("74HCT541", "A → Y", "rmux8: sr buffer"),
    ]),
    "shift_en": (False, "shift-right enable", ["en138"], [
        ("74HCT11", "A/B/C → Y", "rmux8: and3 of nY5..nY7"),
        ("74HCT541", "OE → Y", "rmux8: sr buffer enable"),
    ]),
    "shift": (True, "alu_r, shifts", ["shift_data", "shift_en"], []),
    "c_add": (True, "adder carry / borrow", ["b_c4", "cin_c4"], [
        ("74HCT157", "S → Y", "U7: carry select"),
        ("74HCT86", "A/B → Y", "add_sub8: C4 xor sub"),
    ]),
    "c": (True, "alu_c", ["c_add"], [
        ("74HCT153", "I → Y", "shift8: C select"),
    ]),
    "n": (True, "alu_n", ["sum"], []),
    "z": (True, "alu_z", ["sum", "logic", "not", "shift"], [
        ("74HCT27", "A/B/C → Y", "flags8: nor3 ×3"),
        ("74HCT11", "A/B/C → Y", "flags8: and3"),
    ]),
    "s": (True, "alu_s, alu_f", [], [
        ("74HCT85", "A/B → A>B, A<B", "cmp8 U1: bits 3..0"),
        ("74HCT85", "cascade in → A>B, A<B", "cmp8 U2: bits 7..4"),
    ]),
    "e": (True, "alu_e", [], [
        ("74HCT85", "A/B → A=B", "cmp8 U1: bits 3..0"),
        ("74HCT85", "cascade in → A=B", "cmp8 U2: bits 7..4"),
    ]),
}

# Flag register (designs/flags.py), one clock step: the step starts on the
# clock edge that loads the operand / control registers (74HCT574 CP -> Q)
# and ends with the 74HCT175 setup before the next edge. The ALU paths above
# start when the operands are valid, so the register delay is added after
# them (sums don't depend on order).
FLAGS = {
    "f_alu": (True, "ALU flag → flip-flop (C Z N S E F)", ["c", "z", "n", "s", "e"], [
        ("74HCT574", "CP → Q", "operand register, before the ALU"),
        ("74HCT153", "I → Y", "flags: D2, ALU input"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_sel": (False, "flags_op to the selects", [], [
        ("74HCT574", "CP → Q", "control register: flags_op"),
        ("74HCT138", "A/B/C → Y", "U1 / U7: decode"),
        ("74HCT11", "A/B/C → Y", "pair selects"),
    ]),
    "f_ctl": (True, "flags_op → flip-flop (I, the slowest)", ["f_sel"], [
        ("74HCT153", "S → Y", "flags: next value"),
        ("74HCT86", "A/B → Y", "U12: not I"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_bit": (True, "single-flag write → flip-flop (I)", [], [
        ("74HCT574", "CP → Q", "control register: flags_op_1"),
        ("74HCT157", "S → Y", "bit-write lane"),
        ("74HCT153", "I → Y", "flags: D0"),
        ("74HCT86", "A/B → Y", "U12: not I"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_bus": (True, "flags_bus_in → flip-flop (I), from the bus being valid", [], [
        ("74HCT153", "I → Y", "flags: D1"),
        ("74HCT86", "A/B → Y", "U12: not I"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "f_q": (True, "clk → flag_c ... flag_h", [], [
        ("74HCT175", "CLK → Q", "flags: flip-flop"),
    ]),
    "f_psf": (True, "clk → flags_bus_out (PSF, already enabled)", ["f_q"], [
        ("74HCT541", "A → Y", "U6", ("bus_data",)),
    ]),
    "f_oe": (True, "flags_oe_n → flags_bus_out", [], [
        ("74HCT541", "OE → Y", "U6", ("bus_data",)),
    ]),
    "f_rst": (True, "rst_n → flags", [], [
        ("74HCT175", "/MR → Q", "flags: clear"),
    ]),
}
# ---- bus loading -----------------------------------------------------------------------
# chips.md delays are at CL = 50 pF. A stage given a 4th element (the nets it
# drives) gets the extra delay of the load on its worst bit beyond 50 pF.
#   pins: 10 pF per input (CI max), 20 pF per 3-state output, enabled or
#         not (CO, CD74HC670 datasheet; the 541 and 574 are similar);
#   wiring: 2 pF per pin (stub) + a backbone: 20 pF for the buses that run
#         across the boards, 5 pF for the nets local to one section;
#   slope: SN74HCT541 (6 mA bus driver) 50 -> 150 pF at 4.5 V: tpd 13/23/29 ->
#         20/33/42 ns, so 0.07 / 0.10 / 0.13 ns per pF (typ, max 25, max 85 °C).
#         Standard outputs (4 mA: 670, 153 ...) have no 150 pF data: scaled
#         by drive, 6 / 4 x that = 0.105 / 0.15 / 0.20 ns per pF.
# The pins are counted on the built circuits (netlist.py flatten, chip models
# included); the sections not built yet are estimates, listed with each bus.
PIN_PF = {"in": 10, "out": 20}
WIRE_PF_PER_PIN = 2
WIRE_PF_BUS = {"bus_a": 20, "bus_b": 20, "bus_data": 20, "bus_addr": 20}   # others: 5 (local)
SLOPE = {"bus": (0.07, 0.10, 0.13), "std": (0.105, 0.15, 0.20)}
DRIVE = {"74HCT541": "bus", "74HCT574": "bus", "74HCT240": "bus", "74HCT670": "std", "74HCT153": "std",
         "74HCT157": "std", "74HCT175": "std", "74AHCT595": "bus", "74HCT244": "bus"}
CIRCUITS = Path(__file__).resolve().parent.parent / "circuits"
# bus: [(file, top circuit, net), or ("est", inputs, outputs, what)]
BUSES = {
    "bus_a": [("registers/regs.circ", "regs", "regs_a"), ("alu/alu.circ", "alu", "alu_a"),
              ("addr/addr.circ", "addr", "addr_bus_a")],
    "bus_b": [("registers/regs.circ", "regs", "regs_b"), ("alu/alu.circ", "alu", "alu_b"),
              ("addr/addr.circ", "addr", "addr_bus_b")],
    "bus_data": [("registers/regs.circ", "regs", "regs_bus_in"), ("registers/regs.circ", "regs", "regs_bus_out"),
                 ("flags/flags.circ", "flags", "flags_bus_in"), ("flags/flags.circ", "flags", "flags_bus_out"),
                 ("ir/ir.circ", "ir", "ir_bus_in"),
                 ("addr/addr.circ", "addr", "addr_bus_out"),
                 ("mem/mem.circ", "mem", "mem_bus_in"), ("mem/mem.circ", "mem", "mem_bus_out"),
                 ("cu/cu.circ", "cu", "cu_bus_in"),
                 ("iod/iod.circ", "iod", "iod_bus_in"), ("iod/iod.circ", "iod", "iod_bus_out")],
    # the low byte (the worst: the IVT reads it back)
    "bus_addr": [("addr/addr.circ", "addr", "addr_bus_addr_lo"), ("addr/addr.circ", "addr", "addr_bus_addr_lo_in"),
                 ("mem/mem.circ", "mem", "mem_addr"), ("cu/cu.circ", "cu", "cu_bus_addr_lo"),
                 ("iod/iod.circ", "iod", "iod_addr_in"), ("iod/iod.circ", "iod", "iod_addr_out")],
    # the memory section's local data bus (RAM, the ROM buffer, the write and read buffers)
    "mem_d": [("mem/mem.circ", "mem", "mem_d")],
}
# nets local to the register section, driven by the index multiplexers
REGS_F = "registers/regs.circ"
BUSES.update({
    "a_idx": [(REGS_F, "regs", "regs_a_idx")],       # RA of copy A + WA of all 8
    "b_idx": [(REGS_F, "regs", "regs_b_idx")],
    "a_re_lo": [(REGS_F, "regs", "regs_a_re_lo_n")],     # /RE of 2 670s + the 32 (write enable)
    "a_re_hi": [(REGS_F, "regs", "regs_a_re_hi_n")],
    "b_re_lo": [(REGS_F, "regs", "regs_b_re_lo_n")],
    "b_re_hi": [(REGS_F, "regs", "regs_b_re_hi_n")],
})
# the control unit's EEPROM address lines (7 AT28C256 inputs each)
BUSES["cu_a"] = [("cu/cu.circ", "cu", "cu_a")]
BUS_AB = ("bus_a", "bus_b")
IDX = ("a_idx", "b_idx")
RE = ("a_re_lo", "a_re_hi", "b_re_lo", "b_re_hi")


def bus_pins(bus):
    """(inputs, outputs, [(what, inputs, outputs)]) on the worst bit of a bus."""
    import netlist
    flat = {}
    per_bit: dict[int, list[int]] = {}
    parts = []
    for src in BUSES[bus]:
        if src[0] == "est":
            _, i, o, what = src
            parts.append((what, i, o))
            for b in range(8):
                per_bit.setdefault(b, [0, 0])
                per_bit[b][0] += i
                per_bit[b][1] += o
            continue
        f, top, net = src
        if (f, top) not in flat:
            flat[(f, top)] = netlist.flatten(netlist.Project(CIRCUITS / f).circuits[top])["nets"]
        worst = (0, 0)
        for n in flat[(f, top)]:
            pin = [t for t in n["terminals"] if t["pin"] is None and t["ref"].startswith(net + "[")]
            m = re.fullmatch(re.escape(net) + r"(?:_(\d))?", n["name"])
            if pin:
                b = int(pin[0]["ref"][len(net) + 1:-1])
            elif m:
                b = int(m.group(1) or 0)
            else:
                continue
            chips = [t for t in n["terminals"] if t["pin"] is not None]
            i = sum(t["dir"] == "in" for t in chips)
            o = len(chips) - i
            per_bit.setdefault(b, [0, 0])
            per_bit[b][0] += i
            per_bit[b][1] += o
            worst = max(worst, (i, o), key=sum)
        parts.append((f"{top}: {net}", *worst))
    i, o = max(per_bit.values(), key=lambda v: v[0] * PIN_PF["in"] + v[1] * PIN_PF["out"])
    return i, o, parts


def bus_pf(bus):
    i, o, _ = bus_pins(bus)
    return i * PIN_PF["in"] + o * PIN_PF["out"] + (i + o) * WIRE_PF_PER_PIN + WIRE_PF_BUS.get(bus, 5)


_PF = {}


def derate(part, nets):
    """Extra delay (typ, max 25, max 85) of a stage driving the heaviest of nets."""
    for n in nets:
        if n not in _PF:
            _PF[n] = bus_pf(n)
    c = max(_PF[n] for n in nets)
    return tuple(math.ceil(k * max(0, c - 50)) for k in SLOPE[DRIVE[part]])


# Register section (designs/regs.py, notes/registers-decisions.md), one 500 ns
# step from the rising edge of clk. Two kinds of extra stages:
#   ("@", ns, what)        a fixed time in the step; ns may name an oscillator
#                          edge (OSC: rise1, fall0, fall1, high, period), which
#                          moves with the duty cycle (--duty)
#   ("path", name, what)   a whole earlier path added in series (end to end)
#   ("-", part, path, what) a chip delay subtracted: the earliest an edge can
#                          come when it may be that much early (minimum delays
#                          are taken as 0)
REGS = {
    "r_ctl": (False, "IR / control unit outputs", [], [
        ("74HCT377", "CP → Q", "IR fields (377, slower than the 574 control register)"),
    ]),
    "r_ra": (True, "index → word read (bits 1..0)", ["r_ctl"], [
        ("74HCT153", "S → Y", "U1 / U11: index multiplexer", IDX),
        ("74HCT670", "read address → Q", "port A / B", BUS_AB),
    ]),
    "r_re": (True, "bank enable (bit 2, or not bit 2 from the IR)", ["r_ctl"], [
        ("74HCT153", "I → Y", "U6 / U15: bank /RE", RE),
        ("74HCT670", "/RE → Q", "port A / B", BUS_AB),
    ]),
    "r_lat": (True, "operand latch onto bus A / B", ["r_ctl"], [
        ("74HCT574", "OE → Q", "U10 / U18 (data from the edge: CP → Q 41)", BUS_AB),
    ]),
    "r_const": (True, "constant 00 / 01 onto bus B", ["r_ctl"], [
        ("74HCT541", "OE → Y", "U9", ("bus_b",)),
    ]),
    "r_op": (True, "**operands valid** on regs_a / regs_b (alu_a, alu_b, address path)",
             ["r_ra", "r_re", "r_lat", "r_const"], []),
    "r_flags": (True, "**end to end**: register read → ALU (alu_z) → flag register setup", ["r_op"], [
        ("path", "z", "ALU, slowest flag"),
        ("74HCT153", "I → Y", "flags: D2, ALU input"),
        ("74HCT175", "setup D → CLK", "flags: flip-flop"),
    ]),
    "r_res": (True, "register read → ALU (alu_r) → result latch setup", ["r_op"], [
        ("path", "sum", "ALU, slowest result bit"),
        ("74HCT574", "setup D → CP", "U5: result latch"),
    ]),
    "r_dbus": (True, "register → data bus (STR, PSH, OUT, INT)", ["r_op"], [
        ("74HCT541", "A → Y", "U14", ("bus_data",)),
    ]),
    "r_resbus": (True, "result latch → data bus (enable)", ["r_ctl"], [
        ("74HCT574", "OE → Q", "U5", ("bus_data",)),
    ]),
    # The write strobe comes from the clock generator (control-requirements.md C6):
    # osc (4 MHz) -> 74HCT74 FF1 = clk; FF2 = clk sampled on the falling osc
    # edge; clk_wr_n = NAND3(not clk, osc, FF2) (74HCT10), low from the clk
    # fall (osc rise, 250) to the osc fall (375). Minimum delays are taken as 0:
    # t = 0 is clk rising, up to 35 ns after its osc edge, so the osc fall can
    # come 35 ns early (375 - 35 = 340), and the clk fall 35 ns early or late.
    "w_wa": (True, "write address + setup, before /WE falls", ["r_ctl"], [
        ("74HCT153", "S → Y", "U1: index multiplexer", ("a_idx",)),
        ("74HCT670", "setup WA → /WE", "all 8 chips"),
    ]),
    "w_en": (True, "bank write enable, stable before clk_wr_n falls", ["r_ctl"], [
        ("74HCT153", "I → Y", "U6: /RE words 4–7", ("a_re_hi",)),
        ("74HCT32", "A/B → Y", "U4: regs_we_n OR /RE"),
    ]),
    "w_fall_min": (True, "/WE falls, earliest", [], [
        ("@", "rise1", "osc rises"),
        ("-", "74HCT74", "CLK → Q", "t = 0 (clk rising) can be this late, no gate delay"),
    ]),
    "w_fall": (True, "/WE falls, latest", [], [
        ("@", "rise1", "osc rises"),
        ("74HCT74", "CLK → Q", "FF1: clk falls (late)"),
        ("74HCT10", "A/B/C → Y", "clk_wr_n"),
        ("74HCT32", "A/B → Y", "U4: bank /WE"),
    ]),
    "w_rise_min": (True, "/WE rises, earliest (D setup is to this edge)", [], [
        ("@", "fall1", "osc falls"),
        ("-", "74HCT74", "CLK → Q", "t = 0 can be this late, no gate delay"),
    ]),
    "w_data": (True, "**data on bus_data valid by** (the earliest rise − setup)", [], [
        ("path", "w_rise_min", "earliest /WE rise"),
        ("-", "74HCT670", "setup D → /WE", "all 8 chips"),
    ]),
    # the two ends move together with clk: the pulse is 125 ns of osc minus
    # what can delay its start only (FF1, the NAND, the OR)
    "w_pulse": (True, "/WE pulse, shortest", [], [
        ("@", "high", "osc rise to osc fall"),
        ("-", "74HCT74", "CLK → Q", "FF1"),
        ("-", "74HCT10", "A/B/C → Y", "the NAND"),
        ("-", "74HCT32", "A/B → Y", "the OR"),
    ]),
    "w_rise": (True, "/WE rises, latest, + hold", [], [
        ("@", "fall1", "osc falls"),
        ("74HCT10", "A/B/C → Y", "clk_wr_n"),
        ("74HCT32", "A/B → Y", "U4: bank /WE"),
        ("74HCT670", "hold D, WA after /WE", "all 8 chips"),
    ]),
}
# Address section (designs/addr.py, notes/addr-decisions.md), one 500 ns step.
# sum = BASE + (OFFSET xor inv) + cin through a 4 x 74HCT283 ripple; the
# latch, PC and SP (74HCT377) load it on the next edge. The slowest sum bit is
# bit 15 (the last 283's top sum), and PC's bits 15..9 go through the reset
# AND (74HCT08), so the PC is the worst destination.
RIPPLE = [
    ("74HCT283", "A/B → C4", "U: bits 3..0"),
    ("74HCT283", "Cin → C4", "bits 7..4"),
    ("74HCT283", "Cin → C4", "bits 11..8"),
    ("74HCT283", "Cin → S3", "bits 15..12: sum bit 15"),
]
TO_PC = [
    ("74HCT08", "A/B → Y", "reset AND, PC bits 15..9"),
    ("74HCT377", "setup D, /E → CP", "PC"),
]
ADDR = {
    "a_ctl": (False, "control unit outputs", [], [
        ("74HCT574", "CP → Q", "control register"),
    ]),
    "a_off": (True, "control (OFFSET select) → sum → PC setup", ["a_ctl"], [
        ("74HCT157", "S → Y", "OFFSET low byte: V2 / V1"),
        ("74HCT86", "A/B → Y", "inversion"),
        *RIPPLE, *TO_PC,
    ]),
    "a_base": (True, "control (BASE select) → sum → PC setup", ["a_ctl"], [
        ("74HCT153", "S → Y", "BASE"),
        *RIPPLE, *TO_PC,
    ]),
    "a_ir": (True, "V1 / V2 (IR) → sum → PC setup", [], [
        ("74HCT377", "CP → Q", "IR, V1 / V2 (designs/ir.py)"),
        ("74HCT157", "I → Y", "OFFSET"),
        ("74HCT86", "A/B → Y", "inversion"),
        *RIPPLE, *TO_PC,
    ]),
    "a_pc": (True, "PC / SP → BASE → sum → PC setup (PC + 1, SP − 1 ...)", [], [
        ("74HCT377", "CP → Q", "PC / SP, loaded on the edge"),
        ("74HCT153", "I → Y", "BASE"),
        *RIPPLE, *TO_PC,
    ]),
    "a_regs": (True, "**regs_a / regs_b → sum → PC setup** (Reg16, mode 4, IN / OUT)", ["r_op"], [
        ("74HCT153", "I → Y", "BASE"),
        *RIPPLE, *TO_PC,
    ]),
    "a_ivt": (True, "bus_addr low byte (the IOD's port) → IVT → sum → PC setup, from the bus being valid", [], [
        ("74HCT153", "I → Y", "BASE: IVT, p0 → bit 1"),
        *RIPPLE, *TO_PC,
    ]),
    "a_iod": (True, "**the IOD must drive bus_addr by** (vector step)", [], [
        ("@", 500, "the next edge"),
        ("-", "74HCT153", "I → Y", "BASE: IVT"),
        ("-", "74HCT283", "A/B → C4", "bits 3..0"),
        ("-", "74HCT283", "Cin → C4", "bits 7..4"),
        ("-", "74HCT283", "Cin → C4", "bits 11..8"),
        ("-", "74HCT283", "Cin → S3", "bits 15..12"),
        ("-", "74HCT08", "A/B → Y", "reset AND"),
        ("-", "74HCT377", "setup D, /E → CP", "PC"),
    ]),
    "a_lat": (True, "clk → latch → bus_addr valid (memory gets the margin, to the register-write limit)", [], [
        ("74HCT377", "CP → Q", "address latch"),
        ("74HCT541", "A → Y", "latch drivers", ("bus_addr",)),
    ]),
    "a_lat_oe": (True, "control → latch drivers on → bus_addr", ["a_ctl"], [
        ("74HCT541", "OE → Y", "latch drivers", ("bus_addr",)),
    ]),
    "a_bus_oe": (False, "control → PC / SP driver on", ["a_ctl"], [
        ("74HCT541", "OE → Y", "PC / SP drivers", ("bus_data",)),
    ]),
    "a_bus_q": (False, "PC / SP loaded on the edge → driver", [], [
        ("74HCT377", "CP → Q", "PC / SP"),
        ("74HCT541", "A → Y", "PC / SP drivers", ("bus_data",)),
    ]),
    "a_bus": (True, "PC / SP → bus_data (CAL, interrupt entry, RSP)", ["a_bus_oe", "a_bus_q"], []),
}
# Memory section (designs/mem.py, notes/ir-mem-decisions.md sections 4-5, M1-M4).
# The RAM's WE# = clk_mem_wr_n OR mem_we_n; clk_mem_wr_n is /Q2 of the clock
# generator's FF2 (clk sampled on the osc fall, through the NAND inverter):
# low from the osc fall at 125 to the one at 375. Minimum delays are 0, t = 0
# is clk rising, which can be FF1's delay after its osc edge (as for w_*).
MEM = {
    "m_read": (True, "**RAM read → bus_data** (address from the latch, tAA)", ["a_lat"], [
        ("AS6C1008", "address → DQ (tAA)", "RAM"),
        ("74HCT541", "A → Y", "read buffer", ("bus_data",)),
    ]),
    "m_read_ce": (True, "RAM read: control → CE2 → bus_data (tACE)", [], [
        ("74HCT574", "CP → Q", "control register: mem_re"),
        ("AS6C1008", "CE#, CE2 → DQ (tACE)", "RAM"),
        ("74HCT541", "A → Y", "read buffer", ("bus_data",)),
    ]),
    "m_read_oe": (True, "RAM read: control → read buffer on → bus_data", [], [
        ("74HCT574", "CP → Q", "control register: mem_oe_n"),
        ("74HCT541", "OE → Y", "read buffer", ("bus_data",)),
    ]),
    "m_fetch": (True, "fetch: RAM → bus_data → 14 → IR setup (inverted bits, the slowest)", ["m_read"], [
        ("74HCT14", "A → Y", "ir: NOT bus bit 5 / 2"),
        ("74HCT377", "setup D, /E → CP", "ir: X1 / X2"),
    ]),
    "m_fall_min": (False, "RAM WE# falls, earliest", [], [
        ("@", "fall0", "osc falls"),
        ("-", "74HCT74", "CLK → Q", "t = 0 (clk rising) can be FF1's delay late"),
    ]),
    "m_addr": (True, "address on bus_addr before the RAM's WE# falls (tAS 0)", ["a_lat"], []),
    "m_rise_min": (False, "RAM WE# rises, earliest", [], [
        ("@", "fall1", "osc falls"),
        ("-", "74HCT74", "CLK → Q", "t = 0 can be FF1's delay late"),
    ]),
    "m_wdl": (False, "RAM write data needed by (earliest rise − tDW)", ["m_rise_min"], [
        ("-", "AS6C1008", "setup DIN → WE# rising (tDW)", "RAM"),
    ]),
    "m_wdata": (True, "RAM write data: register → bus_data → write buffer → RAM", ["r_dbus"], [
        ("74HCT541", "A → Y", "write buffer", ("mem_d",)),
    ]),
    # both ends come from FF2 on osc falls 250 ns apart: the pulse loses what
    # can delay its start only (the NAND inverter, FF2, the OR)
    "m_pulse": (True, "RAM WE# pulse, shortest", [], [
        ("@", "period", "osc fall to osc fall"),
        ("-", "74HCT10", "A/B/C → Y", "NAND as the osc inverter"),
        ("-", "74HCT74", "CLK → Q", "FF2 (/Q2)"),
        ("-", "74HCT32", "A/B → Y", "mem: the OR"),
    ]),
    "m_rise": (True, "RAM WE# rises, latest (tWR = tDH = 0)", [], [
        ("@", "fall1", "osc falls"),
        ("74HCT10", "A/B/C → Y", "NAND as the osc inverter"),
        ("74HCT74", "CLK → Q", "FF2 (/Q2)"),
        ("74HCT32", "A/B → Y", "mem: the OR"),
    ]),
    "m_boot": (True, "boot copy: address → NOT A15 → ROM (tCE) → ROM buffer → RAM data", ["a_lat"], [
        ("74HCT14", "A → Y", "mem: NOT A15"),
        ("AT28C256", "/CE → I/O (tCE)", "ROM"),
        ("74HCT541", "A → Y", "ROM buffer", ("mem_d",)),
    ]),
    # M8: the ROMs' /OE is mem_rom_oe_n, so the first copy step waits for tOE too
    "m_boot_oe": (True, "boot copy: control → ROM /OE (tOE) → ROM buffer → RAM data", [], [
        ("74HCT574", "CP → Q", "control register: mem_rom_oe_n"),
        ("AT28C256", "/OE → I/O (tOE)", "ROM"),
        ("74HCT541", "A → Y", "ROM buffer", ("mem_d",)),
    ]),
}
# Control unit (designs/cu.py, notes/cu-decisions.md sections 6 and 14): the
# next microcode word must be at the pipeline 574s' D (setup) by the next
# edge. The EEPROM address comes from the step register, the IR or bus_data
# (the bypass), cu_force, and the condition multiplexer; the decode before the
# 574s adds its slowest gate path (abus: an inverter then an AND).
DECODE = [("74HCT14", "A → Y", "NOT abus bit"), ("74HCT08", "A/B → Y", "abus: latch high off / IE")]
TO_574 = [("AT28C256", "address → I/O (tACC)", "microcode"), *DECODE,
          ("74HCT574", "setup D → CP", "pipeline")]
CU = {
    "c_574": (True, "clk → pipeline outputs (every control line)", [], [
        ("74HCT574", "CP → Q", "pipeline register"),
    ]),
    "c_step": (True, "step register → EEPROM → decode → 574 setup", [], [
        ("74HCT175", "CLK → Q", "step register", ("cu_a",)), *TO_574,
    ]),
    "c_ir": (True, "IR fields (just loaded) → 157 → EEPROM → decode → 574 setup", [], [
        ("74HCT377", "CP → Q", "IR (designs/ir.py)"),
        ("74HCT157", "I → Y", "opcode / mode", ("cu_a",)), *TO_574,
    ]),
    "c_rom_bypass": (False, "bypass: RAM → bus_data → 157 → EEPROM data", ["m_read"], [
        ("74HCT157", "I → Y", "opcode / mode from bus_data", ("cu_a",)),
        ("AT28C256", "address → I/O (tACC)", "microcode"),
    ]),
    "c_bypass": (True, "**bypass**: a fetched byte → 157 → EEPROM → decode → 574 setup", ["c_rom_bypass"], [
        *DECODE, ("74HCT574", "setup D → CP", "pipeline"),
    ]),
    "c_next": (True, "bypass → EEPROM cu_next → step register setup (no decode)", ["c_rom_bypass"], [
        ("74HCT175", "setup D → CLK", "step register"),
    ]),
    "c_force": (True, "cu_force → 157 strobe → EEPROM → decode → 574 setup", ["c_574"], [
        ("74HCT157", "/E → Y", "row (00, 0)", ("cu_a",)), *TO_574,
    ]),
    "c_jc": (True, "jump condition: IR opcode → 151 → 153 → EEPROM → decode → 574 setup", [], [
        ("74HCT377", "CP → Q", "IR opcode bits 2..0"),
        ("74HCT151", "S (A/B/C) → Y", "jc"),
        ("74HCT153", "I → Y", "cond", ("cu_a",)), *TO_574,
    ]),
    "c_int": (True, "boundary: INT (valid by 150, I5) → 08 → 27 → 153 → EEPROM → decode → 574 setup", [], [
        ("@", 150, "INT valid (iod-requirements.md I5)"),
        ("74HCT08", "A/B → Y", "HW = I · INT"),
        ("74HCT27", "A/B/C → Y", "bnd_n"),
        ("74HCT153", "I → Y", "cond", ("cu_a",)), *TO_574,
    ]),
    "c_state": (True, "hw_q / cout_q / rst_early → 27 → 153 → EEPROM → decode → 574 setup", [], [
        ("74HCT175", "CLK → Q", "hw_q, cout_q (rst_early: a 74, quicker)"),
        ("74HCT27", "A/B/C → Y", "x_n"),
        ("74HCT153", "I → Y", "cond", ("cu_a",)), *TO_574,
    ]),
    "c_sel": (True, "cu_cond_sel → 153 select → EEPROM → decode → 574 setup", ["c_574"], [
        ("74HCT153", "S → Y", "cond", ("cu_a",)), *TO_574,
    ]),
    "c_cout": (True, "boot loop: PC + 1 → adder carry out → cout_q setup", [], [
        ("74HCT377", "CP → Q", "PC"),
        ("74HCT153", "I → Y", "BASE"),
        ("74HCT283", "A/B → C4", "bits 3..0"),
        ("74HCT283", "Cin → C4", "bits 7..4"),
        ("74HCT283", "Cin → C4", "bits 11..8"),
        ("74HCT283", "Cin → C4", "bits 15..12: addr_cout"),
        ("74HCT175", "setup D → CLK", "cout_q"),
    ]),
    "c_pnext": (True, "P next: cu_swi_oe_n / ld_n → 14 → 27 → 14 → P setup", ["c_574"], [
        ("74HCT14", "A → Y", "NOT cu_swi_ld_n"),
        ("74HCT27", "A/B/C → Y", "NOR"),
        ("74HCT14", "A → Y", "P next"),
        ("74HCT175", "setup D → CLK", "P"),
    ]),
    "c_swi": (True, "INT: V1 fetched (RAM → bus_data) → pending port latch setup", ["m_read"], [
        ("74HCT377", "setup D, /E → CP", "pending port latch"),
    ]),
    "c_swi_oe": (True, "software vector step: cu_swi_oe_n → 541 on → bus_addr low", ["c_574"], [
        ("74HCT541", "OE → Y", "pending port driver", ("bus_addr",)),
    ]),
}
# the clock generator's paths (module clkgen), shown with the control unit
CU.update({k: v for k, v in REGS.items() if k in ("w_fall", "w_rise_min", "w_data", "w_pulse", "w_rise")})
CU.update({k: v for k, v in MEM.items() if k in ("m_addr", "m_pulse", "m_rise")})
# The whole CPU (cpu.circ, cu-decisions.md section 15): paths that cross
# sections, end to end from the clock edge. The sections' own paths start
# from "control valid" (r_ctl, a_ctl: one register's CP -> Q); here a step
# starts at the later of the IR's 377s and the pipeline 574s (with the
# 574s' load), and the paths go through the next sections up to the
# flip-flop or deadline that ends them.
CPU = {
    "x_ctl": (False, "IR fields and control lines valid (the later of 377 and 574)", ["r_ctl", "c_574"], []),
    "x_ra": (False, "port A / B index → word read", ["x_ctl"], REGS["r_ra"][3]),
    "x_re_sel": (False, "port select → bank /RE", ["x_ctl"], [
        ("74HCT153", "S → Y", "bank /RE by regs_a_sel / regs_b_sel", RE),
        ("74HCT670", "/RE → Q", "port A / B", BUS_AB),
    ]),
    "x_re": (False, "IR bit 2 → bank /RE", ["x_ctl"], REGS["r_re"][3]),
    "x_lat": (False, "operand latch on", ["x_ctl"], REGS["r_lat"][3]),
    "x_const": (False, "constant on", ["x_ctl"], REGS["r_const"][3]),
    "x_op": (True, "edge → control → **bus A / bus B valid** (regs)", ["x_ra", "x_re_sel", "x_re", "x_lat", "x_const"], []),
    "x_flags": (True, "**edge → regs → ALU → flags** (alu_z, the slowest flag) → 175 setup", ["x_op"], REGS["r_flags"][3]),
    "x_res": (True, "edge → regs → ALU (alu_r) → result latch setup", ["x_op"], REGS["r_res"][3]),
    "x_aluop": (True, "edge → alu_op (574) → ALU → flags setup (op taken as late as an operand)", ["c_574"], REGS["r_flags"][3]),
    "x_addr": (True, "edge → regs → **address adder** → PC / SP / latch setup (Reg16, RamReg)", ["x_op"], ADDR["a_regs"][3]),
    "x_dbus": (False, "edge → regs → bus_data", ["x_op"], [("74HCT541", "A → Y", "regs: bus A → data bus", ("bus_data",))]),
    "x_wdata": (True, "edge → regs → bus_data → write buffer → RAM data", ["x_dbus"], [("74HCT541", "A → Y", "mem: write buffer", ("mem_d",))]),
    "x_out": (True, "edge → regs → bus_data: OUT data for the IOD (I4: by 242)", ["x_dbus"], []),
    "x_fetch": (True, "edge → latch → bus_addr → **RAM → bus_data → IR** setup (F0..F3)", [], [("path", "m_fetch", "")]),
    "x_mem_reg": (True, "edge → latch → RAM → bus_data (register write data, LOD / POP)", [], [("path", "m_read", "")]),
    "x_bypass": (True, "edge → RAM → bus_data → **cu bypass** → EEPROM → 574 setup", [], [("path", "c_bypass", "")]),
    "x_flag_jc": (True, "edge → flags (175) → cu jump condition (151 D) → 153 → EEPROM → 574 setup", ["f_q"], [
        ("74HCT151", "D → Y", "jc: a flag input"),
        ("74HCT153", "I → Y", "cond", ("cu_a",)),
        ("AT28C256", "address → I/O (tACC)", "microcode"),
        ("74HCT14", "A → Y", "NOT abus bit"),
        ("74HCT08", "A/B → Y", "abus: latch high off / IE"),
        ("74HCT574", "setup D → CP", "pipeline"),
    ]),
    "x_ivt": (True, "IOD port on bus_addr → IVT → adder → PC setup (vector step)", [], [("path", "a_ivt", "")]),
    "x_int": (True, "IOD INT (by 150, I5) → cu boundary → 574 setup", [], [("path", "c_int", "")]),
    "x_cout": (True, "edge → PC → adder carry out → cu cout_q setup (boot loop)", [], [("path", "c_cout", "")]),
}
# The I/O mailbox (designs/iod.py, iod-requirements.md I3-I6, I10-I16), in the IN / OUT step or the vector
# step. The decode starts when the port is on bus_addr (the latch, 72 ns) or IE is out of the pipeline;
# the STATUS flags are not synchronised (I11), so they have no path here.
IOD = {
    "i_dec": (False, "bus_addr (A0, A1, A6, A7) → 138 → /Y", ["a_lat"], [
        ("74HCT138", "A/B/C → Y", "decode (A6, A7 on /G2: G → Y, quicker)"),
    ]),
    "i_dec_ie": (False, "IE (574) → 138 G1 → /Y", [], [
        ("74HCT574", "CP → Q", "pipeline: IE"),
        ("74HCT138", "G → Y", "decode: G1 = IE"),
    ]),
    "i_sel": (True, "**decode settled** (/Y0 TX, /Y2 RX, /Y3 STATUS)", ["i_dec", "i_dec_ie"], []),
    "i_rx": (True, "**IN RX**: decode → RX 595 /OE → bus_data", ["i_sel"], [
        ("74AHCT595", "OE → Q", "RX", ("bus_data",)),
    ]),
    "i_status": (True, "**IN STATUS**: decode → 244 /OE → bus_data", ["i_sel"], [
        ("74HCT244", "OE → Y", "STATUS", ("bus_data",)),
    ]),
    "i_tx_sel": (True, "OUT TX strobe glitch-free: the decode settles before clk_wr_n can fall", ["i_sel"], []),
    "i_tx_data": (True, "OUT TX: bus_data valid, 165 setup before the earliest strobe rise", ["x_out"], [
        ("74HCT165", "setup A–H → SH/LD", "TX"),
    ]),
    "i_tx_end": (True, "OUT TX: the strobe rises, latest (165 loaded, TX full set); bus_data is held to the edge", [], [
        ("@", "fall1", "osc falls"),
        ("74HCT10", "A/B/C → Y", "clk_wr_n"),
        ("74HCT32", "A/B → Y", "/Y0 OR clk_wr_n"),
    ]),
    "i_rxclr": (True, "rx_clr: /Y2 settled → 74 setup (RX clears in the next step)", ["i_sel"], [
        ("74HCT74", "setup D → CLK", "rx_clr"),
    ]),
    "i_int": (True, "**INT** (evt_s, a 74 on clk)", [], [
        ("74HCT74", "CLK → Q", "evt_s"),
    ]),
    "i_vec_addr": (True, "**vector step**: VEC → EVT port 595 → bus_addr A0–A7", [], [
        ("74HCT74", "CLK → Q", "VEC (/Q)"),
        ("74AHCT595", "OE → Q", "EVT port", ("bus_addr",)),
    ]),
    "i_vec_data": (True, "**vector step**: VEC → EVT data 595 → bus_data (→ I)", [], [
        ("74HCT74", "CLK → Q", "VEC (/Q)"),
        ("74AHCT595", "OE → Q", "EVT data", ("bus_data",)),
    ]),
    "i_int_drop": (True, "INT low in the vector step (VEC → evt_s /CLR; the CPU doesn't sample it there)", [], [
        ("74HCT74", "CLK → Q", "VEC (/Q)"),
        ("74HCT74", "PRE or CLR → Q", "evt_s"),
    ]),
    "i_off_in": (True, "IN drivers off after the edge: IE → 138 → 595 / 244 disable (the slower)", [], [
        ("74HCT574", "CP → Q", "pipeline: IE"),
        ("74HCT138", "G → Y", "decode"),
        ("74HCT244", "OE → Y", "STATUS off (the RX 595: 10)"),
    ]),
    "i_off_vec": (True, "vector-step drivers off after the edge: VEC → 595 disable", [], [
        ("74HCT74", "CLK → Q", "VEC (/Q)"),
        ("74AHCT595", "OE → Q", "EVT port / data off"),
    ]),
}
SECTIONS = {"alu": PATHS, "flags": FLAGS, "regs": REGS, "addr": ADDR, "mem": MEM, "cu": CU, "cpu": CPU, "iod": IOD}
PATHS = {**PATHS, **FLAGS, **REGS, **ADDR, **MEM, **CU, **CPU, **IOD}

# The clock buffer's branches (clkgen's 74HCT541, cu-decisions.md section 14):
# the clock inputs on each, and the extra delay their load adds. The skew
# between branches is the difference (the 541's own output skew isn't
# specified: breadboard-checks.md B1).
CLOCK_BRANCHES = [
    ("clk_0: ir", [("ir/ir.circ", "ir", "clk")]),
    ("clk_1: addr", [("addr/addr.circ", "addr", "clk")]),
    ("clk_2: cu (pipeline 574s 1-5)", [("cu/cu.circ", "cu", "clk_0")]),
    ("clk_3: cu (574s 6-8, 175s, 377)", [("cu/cu.circ", "cu", "clk_1")]),
    ("clk_4: regs, flags", [("registers/regs.circ", "regs", "clk"), ("flags/flags.circ", "flags", "clk")]),
    ("clk_5: reset flip-flops, IOD", [("cu/clkgen.circ", "clkgen", "clk_5"), ("iod/iod.circ", "iod", "clk")]),
]


def clock_inputs(src):
    """Clock inputs of chips on a net (whole name, not a bus bit)."""
    import netlist
    if src[0] == "est":
        return src[1], src[2]
    f, top, net = src
    flat = netlist.flatten(netlist.Project(CIRCUITS / f).circuits[top])["nets"]
    count = 0
    for n in flat:
        if n["name"] == net or any(t["pin"] is None and t["ref"] == net for t in n["terminals"]):
            count = max(count, sum(1 for t in n["terminals"] if t["pin"] is not None and t["dir"] == "in"))
    return count, f"{top}: {net}"

# ---- pass criterion --------------------------------------------------------------------
# name: (op, bound). "<=": the path must end by the bound; ">=": it must last
# at least that long. The bound is a time in ns, another path (compared column
# by column), or a chip delay (part, path) from chips.md. Paths without a
# limit (the ALU outputs, the reference times of the write phase) are parts
# of a longer path that has one. The latest /WE fall is not compared with the
# earliest rise: they assume opposite clk skews; w_pulse checks the pulse.
STEP = 500                                   # ns, one step at 2 MHz
OSC_PERIOD = 250                             # ns, the 4 MHz oscillator
OSC: dict[str, float] = {}


def set_duty(pct: float) -> None:
    """Oscillator edges for a duty cycle (osc high pct % of its period). t = 0
    is clk rising = an osc rise; clk_wr_n follows the osc fall (C6)."""
    high = OSC_PERIOD * pct / 100
    high = int(high) if float(high).is_integer() else high
    OSC.update(rise1=OSC_PERIOD, fall0=high, fall1=OSC_PERIOD + high, high=high, period=OSC_PERIOD)


set_duty(50)
COL = {"typ": 0, "25": 1, "85": 2}
PASS = COL["25"]                             # the design must pass at max 25 °C
LIMITS = {
    # flag register: inside the step
    "f_alu": ("<=", STEP), "f_ctl": ("<=", STEP), "f_bit": ("<=", STEP),
    "f_psf": ("<=", STEP), "f_oe": ("<=", STEP),
    # register section
    "r_flags": ("<=", STEP), "r_res": ("<=", STEP), "r_dbus": ("<=", STEP),
    "r_resbus": ("<=", "w_data"),            # the result latch's data for a register write
    "w_wa": ("<=", "w_fall_min"), "w_en": ("<=", "w_fall_min"),
    "w_pulse": (">=", ("74HCT670", "/WE pulse width")),
    "w_rise": ("<=", STEP),                  # D and WA change at the next edge
    # address section
    "a_off": ("<=", STEP), "a_base": ("<=", STEP), "a_ir": ("<=", STEP), "a_pc": ("<=", STEP),
    "a_regs": ("<=", STEP), "a_ivt": ("<=", STEP),
    "a_lat": ("<=", "w_data"), "a_lat_oe": ("<=", "w_data"),   # a memory read must reach the data bus by then
    "a_bus": ("<=", "w_data"),               # RSP: SP into a register (C10, P3); also the memory write data
    # memory section
    "m_read": ("<=", "w_data"), "m_read_ce": ("<=", "w_data"), "m_read_oe": ("<=", "w_data"),
    "m_fetch": ("<=", STEP),
    "m_addr": ("<=", "m_fall_min"),
    "m_wdata": ("<=", "m_wdl"), "m_boot": ("<=", "m_wdl"), "m_boot_oe": ("<=", "m_wdl"),
    "m_pulse": (">=", ("AS6C1008", "write pulse WE# (tWP)")),
    "m_rise": ("<=", STEP),                  # the address and data change at the next edge
    # control unit: the next word at the 574s (and the step register) by the edge
    "c_step": ("<=", STEP), "c_ir": ("<=", STEP), "c_bypass": ("<=", STEP), "c_next": ("<=", STEP),
    "c_force": ("<=", STEP), "c_jc": ("<=", STEP), "c_int": ("<=", STEP), "c_state": ("<=", STEP),
    "c_sel": ("<=", STEP), "c_cout": ("<=", STEP), "c_pnext": ("<=", STEP), "c_swi": ("<=", STEP),
    "c_swi_oe": ("<=", "a_iod"),             # the vector step: the pending port is the IOD's place
    # the whole CPU
    "x_op": ("<=", STEP), "x_flags": ("<=", STEP), "x_res": ("<=", STEP), "x_aluop": ("<=", STEP),
    "x_addr": ("<=", STEP), "x_wdata": ("<=", "m_wdl"), "x_out": ("<=", 242), "x_fetch": ("<=", STEP),
    "x_mem_reg": ("<=", "w_data"), "x_bypass": ("<=", STEP), "x_flag_jc": ("<=", STEP), "x_ivt": ("<=", STEP),
    "x_int": ("<=", STEP), "x_cout": ("<=", STEP),
    # the I/O mailbox
    "i_rx": ("<=", "w_data"), "i_status": ("<=", "w_data"), "i_tx_sel": ("<=", "w_fall_min"),
    "i_tx_data": ("<=", "w_rise_min"), "i_tx_end": ("<=", STEP), "i_rxclr": ("<=", STEP),
    "i_int": ("<=", 150),                    # I5
    "i_vec_addr": ("<=", "a_iod"), "i_vec_data": ("<=", "w_data"),
    "i_off_in": ("<=", 100), "i_off_vec": ("<=", 100),   # I3 / I6 as amended (Q-C1)
}


def short(part: str) -> str:
    """A part name as printed: 74HCT541 -> 541; memories (AS6C1008, AT28C256) whole."""
    return part.removeprefix("74HCT")


def compute(table):
    done: dict[str, tuple[int, int, int]] = {}
    via: dict[str, str] = {}

    def run(stages):
        t, names = (0, 0, 0), []
        for part, p, *rest in stages:
            if part == "@":                          # a fixed time in the step
                p = OSC[p] if isinstance(p, str) else p
                d = (p, p, p)
                names.append(f"{rest[0]} ({p})")
            elif part == "path":                     # another path in series
                d = get(p)
                names.append(f"[{PATHS[p][1].replace('*', '')}] ({d[1]} / {d[2]})")
            elif part == "-":                        # a delay that makes an edge early
                part, p = p, rest[0]
                d = tuple(-x for x in table[(part, p)])
                names.append(f"− {short(part)} {p} ({-d[1]} / {-d[2]})")
            else:
                d = table[(part, p)]
                nets = rest[1:]
                if nets:                             # driving a loaded bus
                    extra = derate(part, nets[0])
                    names.append(f"{short(part)} {p} ({d[1]} + {extra[1]} / {d[2]} + {extra[2]} load)")
                    d = tuple(a + b for a, b in zip(d, extra))
                else:
                    names.append(f"{short(part)} {p} ({d[1]} / {d[2]})")
            t = tuple(a + b for a, b in zip(t, d))
        return t, names

    def get(name):
        if name in done:
            return done[name]
        _, label, start, stages = PATHS[name]
        starts = [get(s) for s in start]
        t0 = tuple(max(col) for col in zip(*starts)) if starts else (0, 0, 0)
        t, names = run(stages)
        done[name] = tuple(a + b for a, b in zip(t0, t))
        # the start that is latest in the pass column is the one shown
        lead = [via[max(start, key=lambda s: done[s][PASS])]] if start else []
        via[name] = " → ".join(lead + names)
        return done[name]

    for n in PATHS:
        get(n)
    return done, via


def margins(name, done, table):
    """(limit text, margin per column) of a path, or None if it has no limit."""
    if name not in LIMITS:
        return None
    op, b = LIMITS[name]
    if isinstance(b, int):
        lim, what = (b, b, b), None
    elif isinstance(b, tuple):
        lim, what = table[b], f"{short(b[0])} {b[1]}"
    else:
        lim, what = done[b], PATHS[b][1].replace("*", "").split(" (")[0]
    m = tuple((l - v) if op == "<=" else (v - l) for v, l in zip(done[name], lim))
    sign = "≤" if op == "<=" else "≥"
    need = f"{sign} {b}" if what is None else f"{sign} {lim[1]} / {lim[2]} ({what})"
    return need, m


def main(argv=None):
    ap = argparse.ArgumentParser(description="HBC-2 path delays from chips.md (ALU, flags, registers, address, memory, control unit).")
    ap.add_argument("--section", choices=[*SECTIONS, "all"], default="alu",
                    help="which paths to show (default alu); all: one line per section (worst margins)")
    ap.add_argument("--limit", type=int, help="also fail if a shown path is over this at max 25 °C")
    ap.add_argument("--duty", type=float, default=50,
                    help="oscillator duty cycle in %% (default 50; the part is specified 45-55, Q9)")
    args = ap.parse_args(argv)
    set_duty(args.duty)
    sys.stdout.reconfigure(encoding="utf-8")
    table = delays()
    done, via = compute(table)
    if args.section == "all":
        print("| Section | Paths with a limit | Worst margin 25 °C (pass) | Worst margin 85 °C (info) | Paths over at 85 °C |")
        print("|---|---|---|---|---|")
        bad = 0
        for sec, paths in SECTIONS.items():
            ms = []
            for n, (shown, label, _, _) in paths.items():
                lm = margins(n, done, table) if shown else None
                if lm:
                    ms.append((lm[1], label.replace("*", "")))
            if not ms:
                worst = max((done[n][PASS], label.replace("*", "")) for n, (sh, label, _, _) in paths.items() if sh)
                print(f"| {sec} | 0 (checked inside the cpu paths) | — (slowest: {worst[1]}, {worst[0]} ns) | — | — |")
                continue
            w25 = min(ms, key=lambda x: x[0][PASS])
            w85 = min(ms, key=lambda x: x[0][COL["85"]])
            over = [f"{lab} ({m[COL['85']]})" for m, lab in ms if m[COL["85"]] < 0]
            bad += sum(1 for m, _ in ms if m[PASS] < 0)
            print(f"| {sec} | {len(ms)} | {w25[0][PASS]} ({w25[1]}) | {w85[0][COL['85']]} ({w85[1]}) | {'; '.join(over) or '—'} |")
        print(f"\nOscillator duty: {args.duty:g} %. Pass (max 25 °C): " + ("all limits met" if not bad else f"{bad} FAIL"))
        sys.exit(1 if bad else 0)
    print("| Path | Through (per stage, max 25 / 85 °C, ns) | Typ | Max 25 °C | Max 85 °C "
          "| Limit (25 / 85 °C) | Margin 25 °C (pass) | Margin 85 °C (info) |")
    print("|---|---|---|---|---|---|---|---|")
    fail, hot = [], []
    for n, (shown, label, _, _) in SECTIONS[args.section].items():
        if not shown:
            continue
        t = done[n]
        cells = [str(x) for x in t]
        lm = margins(n, done, table)
        if lm is None and args.limit:
            lm = (f"≤ {args.limit}", tuple(args.limit - x for x in t))
        if lm:
            need, m = lm
            name = label.replace("*", "")
            if m[PASS] < 0:
                fail.append(f"{name} ({m[PASS]})")
            if m[COL["85"]] < 0:
                hot.append(f"{name} ({m[COL['85']]})")
            cells += [need] + [f"**{x}**" if x < 0 else str(x) for x in m[1:]]
        else:
            cells += ["—", "—", "—"]
        print(f"| {label} | {via[n]} | " + " | ".join(cells) + " |")
    print(f"Oscillator duty: {args.duty:g} % (osc falls at {OSC['fall0']:g} and {OSC['fall1']:g} ns)\n")
    if args.section == "cu":
        print("| Clock branch (clkgen 541) | Clock inputs | Load (pF) | Extra delay (max 25 / 85 °C, ns) | Counted |")
        print("|---|---|---|---|---|")
        extras = []
        for name, srcs in CLOCK_BRANCHES:
            got = [clock_inputs(s) for s in srcs]
            i = sum(k for k, _ in got)
            c = i * PIN_PF["in"] + i * WIRE_PF_PER_PIN + 10
            ex = [math.ceil(SLOPE["bus"][col] * max(0, c - 50)) for col in (1, 2)]
            extras.append(ex)
            print(f"| {name} | {i} | {c} | {ex[0]} / {ex[1]} | {'; '.join(w for _, w in got)} |")
        skew = [max(e[k] for e in extras) - min(e[k] for e in extras) for k in (0, 1)]
        print(f"\nSkew from the loads: {skew[0]} / {skew[1]} ns (max 25 / 85 °C); the 541's output skew comes on top (B1).\n")
    if args.section in ("regs", "addr", "mem", "cu"):
        print("\n| Bus (worst bit) | Inputs | 3-state outputs | Load (pF) | Extra delay, bus driver (max 25 / 85 °C, ns) "
              "| Extra delay, standard output (max 25 / 85 °C, ns) | Counted (worst bit of each) |")
        print("|---|---|---|---|---|---|---|")
        for bus in BUSES:
            i, o, parts = bus_pins(bus)
            c = bus_pf(bus)
            ex = [" / ".join(str(math.ceil(SLOPE[k][col] * max(0, c - 50))) for col in (1, 2)) for k in ("bus", "std")]
            what = "; ".join(f"{w}: {a} in, {b} out" for w, a, b in parts)
            print(f"| {bus} | {i} | {o} | {c} | {ex[0]} | {ex[1]} | {what} |")
    print("\nPass (max 25 °C): " + ("all limits met" if not fail else f"{len(fail)} FAIL: " + "; ".join(fail)))
    print("Max 85 °C (information only): " + ("all limits met" if not hot else f"{len(hot)} over: " + "; ".join(hot)))
    sys.exit(1 if fail else 0)


if __name__ == "__main__":
    main()
