#!/usr/bin/env python3
"""Extract netlists from Logisim-evolution (.circ) files.

Two views are available:

* Per circuit (default): every net of every circuit, as drawn. A net is a
  group of wires, ports and tunnels that touch. Buses stay buses; splitters
  and subcircuits are listed like any other component.

* Flattened (--flat): the whole design expanded from one top circuit, at bit
  level. Subcircuits are opened, splitters are resolved, and each net lists
  the real component pins (TTL chips, top-level pins, I/O, ...) that share
  one electrical node. Power, Ground and Constant components, and the hidden
  VCC/GND pins of TTL chips, join two global nets named VCC and GND.

Usage:
    python netlist.py                    # every circuit file in ../circuits
    python netlist.py cpu.circ           # one file (looked up in ../circuits too)
    python netlist.py cpu.circ --flat    # flatten from the file's main circuit
    python netlist.py cpu.circ --flat alu --json -o alu.json

Component geometry (port positions, TTL pinouts, splitter layout, default
subcircuit appearance) follows Logisim-evolution 5.0.0. Components the script
does not know are reported on stderr; wires touching them are not connected.
"""

from __future__ import annotations

import argparse
import json
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path

LOGISIM_DIR = Path(__file__).resolve().parent.parent
CIRCUITS_DIR = LOGISIM_DIR / "circuits"


def all_circ_files() -> list[Path]:
    """logisim/*.circ, then circuits/**/*.circ."""
    files = sorted(LOGISIM_DIR.glob("*.circ")) + sorted(CIRCUITS_DIR.rglob("*.circ"))
    return [f for f in files if not f.name.startswith(".")]

# --------------------------------------------------------------------------
# TTL library (#TTL), extracted from logisim-evolution-5.0.0-all.jar.
# chip: (pins, height, vcc pin, gnd pin, outputs, in/outs, unused pins,
#        names of the remaining pins in pin order, without VCC/GND/unused)
# --------------------------------------------------------------------------
TTL = {
    "7400": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "7402": (14, 60, 14, 7, (1, 4, 10, 13), (), (),
             ["1Y", "1A", "1B", "2Y", "2A", "2B", "3A", "3B", "3Y", "4A", "4B", "4Y"]),
    "7404": (14, 60, 14, 7, (2, 4, 6, 8, 10, 12), (), (),
             ["1A", "1Y", "2A", "2Y", "3A", "3Y", "4Y", "4A", "5Y", "5A", "6Y", "6A"]),
    "7408": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "7410": (14, 60, 14, 7, (6, 8, 12), (), (),
             ["1A", "1B", "2A", "2B", "2C", "2Y", "3Y", "3A", "3B", "3C", "1Y", "1C"]),
    "7411": (14, 60, 14, 7, (6, 8, 12), (), (),
             ["1A", "1B", "2A", "2B", "2C", "2Y", "3Y", "3A", "3B", "3C", "1Y", "1C"]),
    "74125": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["n1OE", "1A", "1Y", "n2OE", "2A", "2Y", "3Y", "3A", "n3OE", "4Y", "4A", "n4OE"]),
    "7413": (14, 60, 14, 7, (6, 8), (), (3, 11),
             ["A0", "B0", "C0", "D0", "Y0", "Y1", "D1", "C1", "B1", "A1"]),
    "74138": (16, 60, 16, 8, (7, 9, 10, 11, 12, 13, 14, 15), (), (),
             ["A", "B", "C", "nG2A", "nG2B", "G1", "nY7", "nY6", "nY5", "nY4", "nY3", "nY2", "nY1", "nY0"]),
    "74139": (16, 60, 16, 8, (4, 5, 6, 7, 9, 10, 11, 12), (), (),
             ["1nG", "1A", "1B", "1nY0", "1nY1", "1nY2", "1nY3", "2nY3", "2nY2", "2nY1", "2nY0", "2B", "2A", "2nG"]),
    "7414": (14, 60, 14, 7, (2, 4, 6, 8, 10, 12), (), (),
             ["1A", "1Y", "2A", "2Y", "3A", "3Y", "4Y", "4A", "5Y", "5A", "6Y", "6A"]),
    "74151": (16, 60, 16, 8, (5, 6), (), (),
             ["D3", "D2", "D1", "D0", "Y", "W", "nG", "C", "B", "A", "D7", "D6", "D5", "D4"]),
    "74153": (16, 80, 16, 8, (7, 9), (), (),
             ["n1E", "S1", "1D3", "1D2", "1D1", "1D0", "1Y", "2Y", "2D0", "2D1", "2D2", "2D3", "S0", "n2E"]),
    "74157": (16, 60, 16, 8, (4, 7, 9, 12), (), (),
             ["SELECT", "1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3B", "3A", "4Y", "4B", "4A", "nSTROBE"]),
    "74158": (16, 60, 16, 8, (4, 7, 9, 12), (), (),
             ["SELECT", "1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3B", "3A", "4Y", "4B", "4A", "nSTROBE"]),
    "74161": (16, 60, 16, 8, (11, 12, 13, 14, 15), (), (),
             ["MR/CLR", "CP/CLK", "D0/A", "D1/B", "D2/C", "D3/D", "CE/ENP", "PE/LOAD", "CET/ENT", "Q3/QD", "Q2/QC", "A1/QB", "A0/QA", "TC/RC0"]),
    "74163": (16, 60, 16, 8, (11, 12, 13, 14, 15), (), (),
             ["MR/CLR", "CP/CLK", "D0/A", "D1/B", "D2/C", "D3/D", "CE/ENP", "PE/LOAD", "CET/ENT", "Q3/QD", "Q2/QC", "A1/QB", "A0/QA", "TC/RC0"]),
    "74164": (14, 60, 14, 7, (3, 4, 5, 6, 10, 11, 12, 13), (), (),
             ["A", "B", "QA", "QB", "QC", "QD", "Clock", "Clear", "QE", "QF", "QG", "QH"]),
    "74165": (16, 60, 16, 8, (7, 9), (), (),
             ["Shift/Load", "Clock", "P4", "P5", "P6", "P7", "Q7n", "Q7", "Serial_Input", "P0", "P1", "P2", "P3", "Clock_Inhibit"]),
    "74166": (16, 60, 16, 8, (13,), (), (),
             ["Serial_Input", "P0", "P1", "P2", "P3", "Clock_Inhibit", "Clock", "Clear", "P4", "P5", "P6", "Q7", "P7", "Shift/Load"]),
    "74173": (16, 60, 16, 8, (3, 4, 5, 6), (), (),
             ["nM", "nN", "Q1", "Q2", "Q3", "Q4", "CLK", "nG1", "nG2", "D4", "D3", "D2", "D1", "CLR"]),
    "74175": (16, 60, 16, 8, (2, 3, 6, 7, 10, 11, 14, 15), (), (),
             ["nCLR", "Q1", "nQ1", "D1", "D2", "nQ2", "Q2", "CLK", "Q3", "nQ3", "D3", "D4", "nQ4", "Q4"]),
    "7418": (14, 60, 14, 7, (6, 8), (), (3, 11),
             ["A0", "B0", "C0", "D0", "Y0", "Y1", "D1", "C1", "B1", "A1"]),
    "74181": (24, 60, 24, 12, (9, 10, 11, 13, 14, 15, 16, 17), (), (),
             ["B0", "A0", "S3", "S2", "S1", "S0", "nCi", "M", "F0", "F1", "F2", "F3", "A=B", "Pn", "Co", "Gn", "B3", "A3", "B2", "A2", "B1", "A1"]),
    "74182": (16, 60, 16, 8, (7, 9, 10, 11, 12), (), (),
             ["G1", "P1", "G0", "P0", "G3", "P3", "P", "Cnz", "G", "Cny", "Cnx", "Cn", "G2", "P2"]),
    "7419": (14, 60, 14, 7, (2, 4, 6, 8, 10, 12), (), (),
             ["1A", "1Y", "2A", "2Y", "3A", "3Y", "4Y", "4A", "5Y", "5A", "6Y", "6A"]),
    "74192": (16, 60, 16, 8, (2, 3, 6, 7, 12, 13), (), (),
             ["Data_Input_B", "Data_Output_B", "Data_Output_A", "Count_Down", "Count_Up", "Data_Output_C", "Data_Output_D", "Data_Input_D", "Data_Input_C", "Load", "Carry", "Borrow", "Clear", "Data_Input_A"]),
    "74193": (16, 60, 16, 8, (2, 3, 6, 7, 12, 13), (), (),
             ["Data_Input_B", "Data_Output_B", "Data_Output_A", "Count_Down", "Count_Up", "Data_Output_C", "Data_Output_D", "Data_Input_D", "Data_Input_C", "Load", "Carry", "Borrow", "Clear", "Data_Input_A"]),
    "74194": (16, 60, 16, 8, (12, 13, 14, 15), (), (),
             ["nCLR", "SR", "A", "B", "C", "D", "SL", "S0", "S1", "CLK", "QD", "QC", "QB", "QA"]),
    "7420": (14, 60, 14, 7, (6, 8), (), (3, 11),
             ["A0", "B0", "C0", "D0", "Y0", "Y1", "D1", "C1", "B1", "A1"]),
    "7421": (14, 60, 14, 7, (6, 8), (), (3, 11),
             ["A0", "B0", "C0", "D0", "Y0", "Y1", "D1", "C1", "B1", "A1"]),
    "7424": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "74240": (20, 60, 20, 10, (3, 5, 7, 9, 12, 14, 16, 18), (), (),
             ["n1G", "1A1", "n2Y4", "1A2", "n2Y3", "1A3", "n2Y2", "1A4", "n2Y1", "2A1", "n1Y4", "2A2", "n1Y3", "2A3", "n1Y2", "2A4", "n1Y1", "n2G"]),
    "74241": (20, 60, 20, 10, (3, 5, 7, 9, 12, 14, 16, 18), (), (),
             ["n1G", "1A1", "2Y4", "1A2", "2Y3", "1A3", "2Y2", "1A4", "2Y1", "2A1", "1Y4", "2A2", "1Y3", "2A3", "1Y2", "2A4", "1Y1", "2G"]),
    "74244": (20, 60, 20, 10, (3, 5, 7, 9, 12, 14, 16, 18), (), (),
             ["n1G", "1A1", "2Y4", "1A2", "2Y3", "1A3", "2Y2", "1A4", "2Y1", "2A1", "1Y4", "2A2", "1Y3", "2A3", "1Y2", "2A4", "1Y1", "n2G"]),
    "74245": (20, 60, 20, 10, (), (2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16, 17, 18), (),
             ["DIR", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "B8", "B7", "B6", "B5", "B4", "B3", "B2", "B1", "nOE"]),
    "74266": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "7427": (14, 60, 14, 7, (6, 8, 12), (), (),
             ["1A", "1B", "2A", "2B", "2C", "2Y", "3Y", "3A", "3B", "3C", "1Y", "1C"]),
    "74273": (20, 80, 20, 10, (2, 5, 6, 9, 12, 15, 16, 19), (), (),
             ["nCLR", "Q1", "D1", "D2", "Q2", "Q3", "D3", "D4", "Q4", "CLK", "Q5", "D5", "D6", "Q6", "Q7", "D7", "D8", "Q8"]),
    "74283": (16, 60, 16, 8, (1, 4, 9, 10, 13), (), (),
             ["∑2", "B2", "A2", "∑1", "A1", "B1", "CIN", "C4", "∑4", "B4", "A4", "∑3", "A3", "B3"]),
    "74299": (20, 60, 20, 10, (4, 5, 6, 7, 8, 13, 14, 15, 16, 17), (), (),
             ["S0", "nOE1", "nOE2", "IOG", "IOE", "IOC", "IOA", "QA", "nCLR", "SR", "CLK", "IOB", "IOD", "IOF", "IOH", "QH", "SL", "S1"]),
    "7430": (14, 60, 14, 7, (8,), (), (9, 10, 13),
             ["A", "B", "C", "D", "E", "F", "Y", "G", "H"]),
    "7432": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "7434": (14, 60, 14, 7, (2, 4, 6, 8, 10, 12), (), (),
             ["1A", "1Y", "2A", "2Y", "3A", "3Y", "4Y", "4A", "5Y", "5A", "6Y", "6A"]),
    "7436": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "74377": (20, 80, 20, 10, (2, 5, 6, 9, 12, 15, 16, 19), (), (),
             ["nCLKen", "Q1", "D1", "D2", "Q2", "Q3", "D3", "D4", "Q4", "CLK", "Q5", "D5", "D6", "Q6", "Q7", "D7", "D8", "Q8"]),
    "7438": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "74381": (20, 60, 20, 10, (8, 9, 11, 12, 13, 14), (), (),
             ["A1", "B1", "A0", "B0", "S0", "S1", "S2", "F0", "F1", "F2", "F3", "Gn", "Pn", "Ci", "B3", "A3", "B2", "A2"]),
    "7442": (16, 60, 16, 8, (1, 2, 3, 4, 5, 6, 7, 9, 10, 11), (), (),
             ["O0", "O1", "O2", "O3", "O4", "O5", "O6", "O7", "O8", "O9", "D", "C", "B", "A"]),
    "7443": (16, 60, 16, 8, (1, 2, 3, 4, 5, 6, 7, 9, 10, 11), (), (),
             ["O0", "O1", "O2", "O3", "O4", "O5", "O6", "O7", "O8", "O9", "D", "C", "B", "A"]),
    "7444": (16, 60, 16, 8, (1, 2, 3, 4, 5, 6, 7, 9, 10, 11), (), (),
             ["O0", "O1", "O2", "O3", "O4", "O5", "O6", "O7", "O8", "O9", "D", "C", "B", "A"]),
    "7447": (16, 60, 16, 8, (9, 10, 11, 12, 13, 14, 15), (4,), (),
             ["B", "C", "LT", "BI/RBO", "RBI", "D", "A", "e", "d", "c", "b", "a", "g", "f"]),
    "7451": (14, 60, 14, 7, (6, 8), (), (11, 12),
             ["A1", "A2", "B2", "C2", "D2", "Y2", "Y1", "C1", "D1", "B1"]),
    "7454": (14, 60, 14, 7, (8,), (), (6, 11, 12),
             ["A", "C", "D", "E", "F", "Y", "G", "H", "B"]),
    "74541": (20, 60, 20, 10, (11, 12, 13, 14, 15, 16, 17, 18), (), (),
             ["nOE1", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "Y8", "Y7", "Y6", "Y5", "Y4", "Y3", "Y2", "Y1", "nOE2"]),
    "7458": (14, 60, 14, 7, (6, 8), (), (),
             ["A0", "A1", "B1", "C1", "D1", "Y1", "Y0", "D0", "E0", "F0", "B0", "C0"]),
    "7464": (14, 70, 14, 7, (8,), (), (),
             ["A", "E", "F", "G", "H", "I", "Y", "J", "K", "B", "C", "D"]),
    "74670": (16, 60, 16, 8, (6, 7, 9, 10), (), (),
             ["D2", "D3", "D4", "RA1", "RA0", "Q4", "Q3", "Q2", "Q1", "nOE", "nWE", "WA1", "WA0", "D1"]),
    "747266": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "7474": (14, 60, 14, 7, (5, 6, 8, 9), (), (),
             ["nCLR1", "D1", "CLK1", "nPRE1", "Q1", "nQ1", "nQ2", "Q2", "nPRE2", "CLK2", "D2", "nCLR2"]),
    "7476": (16, 60, 16, 8, (10, 11, 13, 14), (), (),
             ["CLK1", "nPRE1", "nCLR1", "J1", "CLK2", "nPRE2", "nCLR2", "J2", "nQ2", "Q2", "K2", "nQ1", "Q1", "K1"]),
    "7485": (16, 60, 16, 8, (5, 6, 7), (), (),
             ["B3", "A<B", "A=B", "A>B", "A>B", "A=B", "A<B", "B0", "A0", "B1", "A1", "A2", "B2", "A3"]),
    "7486": (14, 60, 14, 7, (3, 6, 8, 11), (), (),
             ["1A", "1B", "1Y", "2A", "2B", "2Y", "3Y", "3A", "3B", "4Y", "4A", "4B"]),
    "7487": (14, 60, 14, 7, (3, 6, 9, 12), (), (4, 11),
             ["C", "A1", "Y1", "A2", "Y2", "B", "Y3", "A3", "Y4", "A4"]),
    "7493": (14, 60, 5, 10, (8, 9, 11, 12), (), (4, 6, 7, 13),
             ["CKB", "R0(1)", "R0(2)", "QC", "QB", "QD", "QA", "CKA"]),
}

DEGREES = {"east": 0, "north": 90, "west": 180, "south": 270}


def rotate(dx: int, dy: int, frm: str, to: str) -> tuple[int, int]:
    """Rotate an offset the way Logisim's Location.rotate does (y points down)."""
    deg = (DEGREES[to] - DEGREES[frm]) % 360
    if deg == 0:
        return dx, dy
    if deg == 90:
        return dy, -dx
    if deg == 180:
        return -dx, -dy
    return -dy, dx


def parse_loc(text: str) -> tuple[int, int]:
    x, y = text.strip("()").split(",")
    return int(x), int(y)


def parse_int(text: str | None, default: int) -> int:
    if text is None or text == "":
        return default
    return int(text, 0)


class UnionFind:
    def __init__(self):
        self.parent = {}

    def add(self, a):
        self.parent.setdefault(a, a)

    def find(self, a):
        self.parent.setdefault(a, a)
        root = a
        while self.parent[root] != root:
            root = self.parent[root]
        while self.parent[a] != root:
            self.parent[a], a = root, self.parent[a]
        return root

    def union(self, a, b):
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.parent[rb] = ra


# --------------------------------------------------------------------------
# Data model
# --------------------------------------------------------------------------
@dataclass
class Port:
    name: str
    loc: tuple[int, int]
    width: int          # 0 = adapts to the net (probe, pull resistor)
    dir: str            # in / out / inout (seen from the component)
    pin: int | None = None   # TTL pin number


@dataclass
class Comp:
    kind: str           # pin tunnel splitter ttl sub const power ground io ignored unsupported
    name: str           # factory name, e.g. "Pin", "74283", "add8"
    lib: str            # library description, e.g. "#Wiring"; "" for subcircuits
    loc: tuple[int, int]
    attrs: dict
    ident: str = ""
    ports: list[Port] = field(default_factory=list)
    bit_end: list = field(default_factory=list)   # splitter: end index per bit
    sub: "Circuit | None" = None                  # subcircuit definition
    sub_pins: list = field(default_factory=list)  # subcircuit: child pin comp per port

    @property
    def label(self) -> str:
        return self.attrs.get("label", "")


@dataclass
class Net:
    id: int
    name: str
    named: bool         # name came from a label
    width: int
    members: list[tuple[int, int]]  # (comp index, port index)


class Circuit:
    def __init__(self, project: "Project", elem: ET.Element):
        self.project = project
        self.name = elem.get("name")
        self.attrs = read_attrs(elem)
        self.elem = elem
        self.comps: list[Comp] = []
        self.wires: list[tuple[tuple[int, int], tuple[int, int]]] = []
        self.nets: list[Net] = []
        self.port_net: dict[tuple[int, int], int] = {}
        self._iface = None
        self._built = False
        self._building = False

    # ---- interface (how an instance of this circuit looks from outside) ----
    def pins(self) -> list[Comp]:
        return [c for c in self.comps if c.kind == "pin"]

    def interface(self):
        """Return (anchor facing, [(offset from anchor, pin comp)])."""
        if self._iface is not None:
            return self._iface
        pins = self.pins()
        appear = self.elem.find("appear")
        style = self.attrs.get("appearance", "logisim_evolution")
        if appear is not None and appear.find("circ-anchor") is not None:
            anchor = appear.find("circ-anchor")
            ax, ay = int(anchor.get("x")), int(anchor.get("y"))
            facing = anchor.get("facing", "east")
            by_loc = {c.loc: c for c in pins}
            ports = []
            for p in appear.iter("circ-port"):
                pin = by_loc.get(parse_loc(p.get("pin")))
                if pin is None:
                    self.project.warn(f"{self.name}: appearance port refers to a missing pin at {p.get('pin')}")
                    continue
                ports.append(((int(p.get("x")) - ax, int(p.get("y")) - ay), pin))
            self._iface = (facing, ports)
        elif style in ("logisim_evolution", "custom"):
            self._iface = ("east", evolution_appearance(self, pins))
        else:
            self.project.warn(f"{self.name}: appearance '{style}' is not supported; "
                              "its instances will have no ports")
            self._iface = ("east", [])
        return self._iface

    # ---- nets ----
    def build(self):
        if self._built:
            return
        if self._building:
            raise SystemExit(f"error: circuit {self.name} contains itself")
        self._building = True
        for c in self.comps:
            make_ports(self, c)
        self._make_idents()
        self._make_nets()
        self._built = True
        self._building = False

    def _make_idents(self):
        seen = {}
        for c in self.comps:
            base = c.label if c.label and c.kind not in ("pin", "tunnel") else f"{c.name}@{c.loc[0]},{c.loc[1]}"
            if c.kind == "pin" and c.label:
                base = c.label
            base = base.replace(" ", "_")
            n = seen.get(base, 0)
            seen[base] = n + 1
            c.ident = base if n == 0 else f"{base}#{n + 1}"

    def _make_nets(self):
        uf = UnionFind()
        # Logisim connects things only where they share a point: wire ends,
        # ports, or a wire end on a port. A wire crossing a port does not.
        for a, b in self.wires:
            uf.union(a, b)
        # Tunnels with the same label are connected.
        tunnels = {}
        for c in self.comps:
            if c.kind == "tunnel" and c.label:
                if c.label in tunnels:
                    uf.union(tunnels[c.label], c.loc)
                else:
                    tunnels[c.label] = c.loc

        groups: dict = {}
        for ci, c in enumerate(self.comps):
            for pi, p in enumerate(c.ports):
                groups.setdefault(uf.find(p.loc), []).append((ci, pi))

        used_names = {}
        ordered = sorted(groups.values(), key=lambda m: min(self.comps[ci].ports[pi].loc[::-1] for ci, pi in m))
        for members in ordered:
            nid = len(self.nets)
            widths = {self.comps[ci].ports[pi].width for ci, pi in members} - {0}
            if len(widths) > 1:
                where = ", ".join(f"{self.comps[ci].ident}.{self.comps[ci].ports[pi].name}"
                                  f"={self.comps[ci].ports[pi].width}" for ci, pi in members)
                self.project.warn(f"{self.name}: bit width mismatch on one net: {where}")
            width = max(widths) if widths else 1
            name, named = self._net_name(members, nid)
            n = used_names.get(name, 0)
            used_names[name] = n + 1
            if n:
                name = f"{name}#{n + 1}"
            self.nets.append(Net(nid, name, named, width, members))
            for m in members:
                self.port_net[m] = nid

    def _net_name(self, members, nid):
        pins = [self.comps[ci].label for ci, _ in members if self.comps[ci].kind == "pin" and self.comps[ci].label]
        if pins:
            return sorted(pins)[0], True
        tun = [self.comps[ci].label for ci, _ in members if self.comps[ci].kind == "tunnel" and self.comps[ci].label]
        if tun:
            return sorted(tun)[0], True
        return f"N{nid}", False


def read_attrs(elem: ET.Element) -> dict:
    attrs = {}
    for a in elem.findall("a"):
        val = a.get("val")
        attrs[a.get("name")] = val if val is not None else (a.text or "")
    return attrs


def evolution_appearance(circ: Circuit, pins: list[Comp]):
    """Port offsets of the default "logisim_evolution" appearance
    (DefaultEvolutionAppearance.build), relative to the anchor."""
    west = sorted((p for p in pins if p.attrs.get("type") != "output"), key=lambda p: (p.loc[1], p.loc[0]))
    east = sorted((p for p in pins if p.attrs.get("type") == "output"), key=lambda p: (p.loc[1], p.loc[0]))
    max_left = max((len(p.label) * 8 for p in west), default=0)
    max_right = max((len(p.label) * 8 for p in east), default=0)
    title = len(circ.name) * 8
    fixed = circ.attrs.get("circuitnamedboxfixedsize", "true") == "true"
    text_width = 200 if fixed else max(max_left + max_right + 35, title + 15)
    width = (text_width // 10) * 10 + 20
    if east:
        ax, ay = width, 10
    elif west:
        ax, ay = 0, 10
    else:
        ax, ay = 0, 0
    rx = 50 + (9 - (ax + 9) % 10)
    ry = 50 + (9 - (ay + 9) % 10)
    anchor = (rx + ax, ry + ay)
    out = []
    for i, p in enumerate(west):
        out.append(((rx - anchor[0], ry + 10 + 20 * i - anchor[1]), p))
    for i, p in enumerate(east):
        out.append(((rx + width - anchor[0], ry + 10 + 20 * i - anchor[1]), p))
    return out


# --------------------------------------------------------------------------
# Ports of each component kind
# --------------------------------------------------------------------------
WIRING_SINGLE = {  # factory name -> (kind, default width, direction)
    "Constant": ("const", 1, "out"),
    "Power": ("power", 1, "out"),
    "Ground": ("ground", 1, "out"),
    "Probe": ("io", 0, "in"),
    "Clock": ("io", 1, "out"),
    "Pull Resistor": ("io", 0, "inout"),
}
IO_SINGLE = {"LED": ("io", 1, "in"), "Button": ("io", 1, "out")}
IGNORED = {"Text", "Label", "Image"}


def make_ports(circ: Circuit, c: Comp):
    a = c.attrs
    loc = c.loc
    if c.kind == "pin":
        out = a.get("type") == "output"
        c.ports = [Port(c.label or "pin", loc, parse_int(a.get("width"), 1), "in" if out else "out")]
    elif c.kind == "tunnel":
        c.ports = [Port(c.label or "tunnel", loc, parse_int(a.get("width"), 1), "inout")]
    elif c.kind in ("const", "power", "ground", "io"):
        _, dflt, d = WIRING_SINGLE.get(c.name) or IO_SINGLE[c.name]
        c.ports = [Port(c.name.lower().replace(" ", "_"), loc, parse_int(a.get("width"), dflt), d)]
    elif c.kind == "splitter":
        make_splitter(c)
    elif c.kind == "ttl":
        make_ttl(c)
    elif c.kind == "sub":
        child = c.sub
        child.build()
        frm, iface = child.interface()
        facing = a.get("facing", "east")
        for (dx, dy), pin in iface:
            rx, ry = rotate(dx, dy, frm, facing)
            out = pin.attrs.get("type") == "output"
            c.ports.append(Port(pin.label or f"pin@{pin.loc[0]},{pin.loc[1]}", (loc[0] + rx, loc[1] + ry),
                                parse_int(pin.attrs.get("width"), 1), "out" if out else "in"))
            c.sub_pins.append(child.comps.index(pin))


def make_splitter(c: Comp):
    a = c.attrs
    facing = a.get("facing", "east")
    fanout = parse_int(a.get("fanout"), 2)
    bits = parse_int(a.get("incoming"), 2)
    appear = a.get("appear", "left")
    gap = parse_int(a.get("spacing"), 1) * 10
    justify = 0 if appear in ("center", "legacy") else (1 if appear == "right" else -1)
    if facing in ("north", "south"):
        m = 1 if facing == "north" else -1
        dx0 = gap * ((fanout + 1) // 2 - 1) if justify == 0 else (-10 if m * justify < 0 else 10 + gap * (fanout - 1))
        dy0 = -m * 20
        ddx, ddy = -gap, 0
    else:
        m = -1 if facing == "west" else 1
        dx0 = m * 20
        dy0 = -gap * (fanout // 2) if justify == 0 else (10 if m * justify > 0 else -(10 + gap * (fanout - 1)))
        ddx, ddy = 0, gap
    ends = []
    for i in range(bits):
        v = a.get(f"bit{i}")
        if v is None:
            ends.append(min(i, fanout - 1))      # Logisim default: bit i -> end i
        elif v == "none":
            ends.append(None)
        else:
            ends.append(min(int(v), fanout - 1))
    c.bit_end = ends
    c.ports = [Port("combined", c.loc, bits, "inout")]
    x, y = c.loc
    for k in range(fanout):
        w = sum(1 for e in ends if e == k)
        c.ports.append(Port(f"end{k}", (x + dx0 + k * ddx, y + dy0 + k * ddy), w, "inout"))


def make_ttl(c: Comp):
    pins, height, vcc, gnd, outs, inouts, unused, names = TTL[c.name]
    facing = c.attrs.get("facing", "east")
    power = c.attrs.get("VccGndPorts", "false") == "true"
    length = pins * 10
    half = pins // 2
    names = iter(names)
    for i in range(pins):
        num = i + 1
        if i < half:
            off = {"east": (20 * i + 10, height - 30), "west": (-20 * i - 10, 30 - height),
                   "north": (height - 30, -20 * i - 10), "south": (30 - height, 20 * i + 10)}[facing]
        else:
            j = i - half
            off = {"east": (length - 20 * j - 10, -30), "west": (-length + 20 * j + 10, 30),
                   "north": (-30, -length + 20 * j + 10), "south": (30, length - 20 * j - 10)}[facing]
        loc = (c.loc[0] + off[0], c.loc[1] + off[1])
        if num in unused:
            continue
        if num in (vcc, gnd):
            if power:
                c.ports.append(Port("VCC" if num == vcc else "GND", loc, 1, "in", num))
            continue
        d = "out" if num in outs else "inout" if num in inouts else "in"
        c.ports.append(Port(next(names), loc, 1, d, num))


# --------------------------------------------------------------------------
# Project loading
# --------------------------------------------------------------------------
class Project:
    def __init__(self, path: Path, loaded: dict | None = None):
        self.path = path
        self.loaded = loaded if loaded is not None else {}
        self.loaded[path.resolve()] = self
        self.warnings: list[str] = []
        root = ET.parse(path).getroot()
        self.libs = {lib.get("name"): lib.get("desc") for lib in root.findall("lib")}
        # file# libraries, loaded up front like Logisim does: lib id -> (desc, Project or None)
        self.file_libs: dict[str, tuple[str, "Project | None"]] = {
            i: (d, self._external(d)) for i, d in self.libs.items() if d and d.startswith("file#")}
        main = root.find("main")
        self.main = main.get("name") if main is not None else None
        self.circuits: dict[str, Circuit] = {}
        for e in root.findall("circuit"):
            self.circuits[e.get("name")] = Circuit(self, e)
        for circ in self.circuits.values():
            self._read_circuit(circ)

    def warn(self, msg: str):
        if msg not in self.warnings:
            self.warnings.append(msg)

    def lib_path(self, desc: str) -> Path:
        """File of a "file#" library. Logisim writes paths relative to the
        loading file, with the OS separator (backslashes on Windows)."""
        return (self.path.parent / desc[5:].replace("\\", "/")).resolve()

    def _external(self, desc: str) -> "Project | None":
        if not desc.startswith("file#"):
            return None
        p = self.lib_path(desc)
        if not p.exists():
            self.warn(f"library file {desc[5:]} not found ({p})")
            return None
        if p not in self.loaded:
            Project(p, self.loaded)
        return self.loaded[p]

    def libraries(self) -> list["Project"]:
        """Every project loaded with this one (this file first), each once."""
        return list(dict.fromkeys(self.loaded.values()))

    def _read_circuit(self, circ: Circuit):
        for w in circ.elem.findall("wire"):
            circ.wires.append((parse_loc(w.get("from")), parse_loc(w.get("to"))))
        for e in circ.elem.findall("comp"):
            lib_id = e.get("lib")
            lib = self.libs.get(lib_id, "") if lib_id is not None else ""
            name = e.get("name")
            c = Comp(kind="unsupported", name=name, lib=lib, loc=parse_loc(e.get("loc")), attrs=read_attrs(e))
            if lib_id is None:
                if name in self.circuits:
                    c.kind, c.sub = "sub", self.circuits[name]
            elif lib.startswith("file#"):
                ext = self._external(lib)
                if ext and name in ext.circuits:
                    c.kind, c.sub = "sub", ext.circuits[name]
            elif lib == "#Wiring":
                if name == "Pin":
                    c.kind = "pin"
                elif name == "Tunnel":
                    c.kind = "tunnel"
                elif name == "Splitter":
                    c.kind = "splitter"
                elif name in WIRING_SINGLE:
                    c.kind = WIRING_SINGLE[name][0]
            elif lib == "#TTL" and name in TTL:
                c.kind = "ttl"
            elif lib == "#I/O" and name in IO_SINGLE:
                c.kind = "io"
            if name in IGNORED:
                c.kind = "ignored"
            if c.kind == "unsupported":
                self.warn(f"{circ.name}: component '{name}' ({lib or 'no library'}) is not supported; "
                          f"wires touching it at {c.loc} are left unconnected")
            if c.kind != "ignored":
                circ.comps.append(c)


# --------------------------------------------------------------------------
# Per-circuit netlist
# --------------------------------------------------------------------------
def port_label(c: Comp, p: Port) -> str:
    if c.kind == "ttl":
        return f"{p.pin}:{p.name}"
    return p.name


def connection(c: Comp, p: Port) -> str:
    if c.kind == "pin":
        return f"{'output' if c.attrs.get('type') == 'output' else 'input'} pin {c.ident}"
    if c.kind in ("tunnel", "const", "power", "ground", "io") and len(c.ports) == 1:
        return c.ident
    return f"{c.ident}.{port_label(c, p)}"


def circuit_dict(circ: Circuit) -> dict:
    circ.build()
    comps = []
    for c in circ.comps:
        comps.append({
            "ref": c.ident, "type": c.name, "kind": c.kind, "lib": c.lib, "loc": list(c.loc),
            "ports": [{"name": port_label(c, p), "loc": list(p.loc), "width": p.width, "dir": p.dir,
                       "net": circ.nets[circ.port_net[(ci, pi)]].name}
                      for pi, p in enumerate(c.ports)
                      for ci in [circ.comps.index(c)]],
        })
    nets = []
    for n in circ.nets:
        nets.append({
            "name": n.name, "width": n.width,
            "connections": [connection(circ.comps[ci], circ.comps[ci].ports[pi]) for ci, pi in n.members],
        })
    return {"name": circ.name, "components": comps, "nets": nets}


def circuit_text(d: dict) -> str:
    lines = [f"=== circuit {d['name']} ===", "", "Components:"]
    for c in d["components"]:
        kind = "" if c["kind"] in ("sub", "ttl", "pin") else f" [{c['kind']}]"
        what = {"ttl": f"TTL {c['type']}", "sub": f"subcircuit {c['type']}"}.get(c["kind"], c["type"])
        lines.append(f"  {c['ref']:<28} {what}{kind}")
    lines += ["", "Nets:"]
    for n in d["nets"]:
        bits = f"{n['width']} bits" if n["width"] > 1 else "1 bit"
        flag = "   (unconnected)" if len(n["connections"]) < 2 else ""
        lines.append(f"  {n['name']} ({bits}){flag}")
        for conn in n["connections"]:
            lines.append(f"      {conn}")
    return "\n".join(lines) + "\n"


# --------------------------------------------------------------------------
# Flattened bit-level netlist
# --------------------------------------------------------------------------
def flatten(top: Circuit) -> dict:
    uf = UnionFind()
    terminals = []    # (node, dict)
    names = {}        # node -> list of (priority, name)
    VCC, GND = ("VCC",), ("GND",)
    uf.add(VCC)
    uf.add(GND)
    stack = []

    def node(path, circ, ci, pi, bit):
        return (path, circ.name, circ.port_net[(ci, pi)], bit)

    def expand(circ: Circuit, path: tuple):
        circ.build()
        if circ.name in stack:
            raise SystemExit(f"error: circuit {circ.name} contains itself")
        stack.append(circ.name)
        prefix = "/".join(path)
        depth = len(path)
        for n in circ.nets:
            if n.named:
                for b in range(n.width):
                    nm = n.name if n.width == 1 else f"{n.name}[{b}]"
                    full = f"{prefix}/{nm}" if prefix else nm
                    names.setdefault((path, circ.name, n.id, b), []).append((depth, full))
        for ci, c in enumerate(circ.comps):
            ref = f"{prefix}/{c.ident}" if prefix else c.ident
            if c.kind == "splitter":
                count = {}
                for i, k in enumerate(c.bit_end):
                    if k is None:
                        continue
                    j = count.get(k, 0)
                    count[k] = j + 1
                    uf.union(node(path, circ, ci, 0, i), node(path, circ, ci, k + 1, j))
            elif c.kind == "sub":
                child_path = path + (c.ident,)
                expand(c.sub, child_path)
                for pi, p in enumerate(c.ports):
                    child_ci = c.sub_pins[pi]
                    for b in range(p.width):
                        uf.union(node(path, circ, ci, pi, b), node(child_path, c.sub, child_ci, 0, b))
            elif c.kind == "ttl":
                for pi, p in enumerate(c.ports):
                    terminals.append((node(path, circ, ci, pi, 0),
                                      {"ref": ref, "type": c.name, "pin": p.pin, "name": p.name, "dir": p.dir}))
                if c.attrs.get("VccGndPorts", "false") != "true":
                    _, _, vcc, gnd, *_ = TTL[c.name]
                    terminals.append((VCC, {"ref": ref, "type": c.name, "pin": vcc, "name": "VCC", "dir": "in"}))
                    terminals.append((GND, {"ref": ref, "type": c.name, "pin": gnd, "name": "GND", "dir": "in"}))
            elif c.kind in ("power", "ground"):
                for b in range(c.ports[0].width):
                    uf.union(VCC if c.kind == "power" else GND, node(path, circ, ci, 0, b))
            elif c.kind == "const":
                value = parse_int(c.attrs.get("value"), 1)
                for b in range(c.ports[0].width):
                    uf.union(VCC if value >> b & 1 else GND, node(path, circ, ci, 0, b))
            elif c.kind == "pin" and depth == 0 or c.kind == "io":
                p = c.ports[0]
                width = p.width or circ.nets[circ.port_net[(ci, 0)]].width
                if c.kind == "pin":
                    kind = "output pin" if c.attrs.get("type") == "output" else "input pin"
                else:
                    kind = p.dir
                for b in range(width):
                    nm = "" if width == 1 else f"[{b}]"
                    terminals.append((node(path, circ, ci, 0, b),
                                      {"ref": ref + nm, "type": c.name, "pin": None, "name": None, "dir": kind}))
        stack.pop()

    expand(top, ())

    groups = {}
    for nd, t in terminals:
        groups.setdefault(uf.find(nd), []).append(t)
    best = {}
    for nd, cands in names.items():
        r = uf.find(nd)
        for cand in cands:
            if r not in best or (cand[0], len(cand[1]), cand[1]) < (best[r][0], len(best[r][1]), best[r][1]):
                best[r] = cand
    nets = []
    used = set()
    auto = 0
    roots = sorted(groups, key=lambda r: (r not in (uf.find(VCC), uf.find(GND)), str(best.get(r, ("", "~"))[1])))
    for r in roots:
        if r == uf.find(VCC) and r == uf.find(GND):
            name = "VCC"
            print("warning: VCC and GND are shorted together", file=sys.stderr)
        elif r == uf.find(VCC):
            name = "VCC"
        elif r == uf.find(GND):
            name = "GND"
        elif r in best:
            name = best[r][1]
        else:
            auto += 1
            name = f"N{auto}"
        base, k = name, 1
        while name in used:
            k += 1
            name = f"{base}#{k}"
        used.add(name)
        terms = sorted(groups[r], key=lambda t: (t["type"] != "Pin", t["ref"], t["pin"] or 0))
        nets.append({"name": name, "terminals": terms})
    return {"top": top.name, "nets": nets}


def flat_text(d: dict) -> str:
    lines = [f"=== flattened netlist, top circuit {d['top']} ===", ""]
    for n in d["nets"]:
        flag = "   (only one terminal)" if len(n["terminals"]) < 2 else ""
        lines.append(f"{n['name']}{flag}")
        for t in n["terminals"]:
            if t["pin"] is not None:
                lines.append(f"    {t['ref']}.{t['pin']}({t['name']})  [{t['dir']}]")
            else:
                lines.append(f"    {t['ref']}  [{t['dir']}]")
    return "\n".join(lines) + "\n"


# --------------------------------------------------------------------------
# Command line
# --------------------------------------------------------------------------
def resolve_file(arg: str) -> Path:
    p = Path(arg)
    if p.is_file():
        return p
    stem = Path(arg).stem
    for cand in (LOGISIM_DIR / arg, LOGISIM_DIR / f"{arg}.circ", CIRCUITS_DIR / arg, CIRCUITS_DIR / f"{arg}.circ",
                 CIRCUITS_DIR / stem / f"{stem}.circ", *CIRCUITS_DIR.glob(f"*/{stem}.circ")):
        if cand.is_file():
            return cand
    raise SystemExit(f"error: circuit file not found: {arg}")


def main(argv=None):
    ap = argparse.ArgumentParser(description="Extract netlists from Logisim-evolution .circ files.")
    ap.add_argument("files", nargs="*", help=f"circuit files (default: every .circ in {LOGISIM_DIR})")
    ap.add_argument("--circuit", "-c", action="append", help="only list this circuit (repeatable)")
    ap.add_argument("--flat", nargs="?", const="", metavar="TOP",
                    help="flattened bit-level netlist from TOP (default: the file's main circuit)")
    ap.add_argument("--json", action="store_true", help="JSON output instead of text")
    ap.add_argument("-o", "--output", help="write to this file instead of stdout")
    args = ap.parse_args(argv)

    files = [resolve_file(f) for f in args.files] or all_circ_files()
    if not files:
        raise SystemExit(f"error: no .circ files in {LOGISIM_DIR}")

    results = []
    text = []
    warnings = []
    for f in files:
        proj = Project(f)
        if args.flat is not None:
            top_name = args.flat or proj.main
            if top_name not in proj.circuits:
                raise SystemExit(f"error: {f.name} has no circuit named '{top_name}' "
                                 f"(available: {', '.join(proj.circuits)})")
            d = flatten(proj.circuits[top_name])
            d["file"] = f.name
            results.append(d)
            text.append(f"# {f.name}\n" + flat_text(d))
        else:
            wanted = args.circuit or list(proj.circuits)
            missing = [n for n in wanted if n not in proj.circuits]
            if missing:
                raise SystemExit(f"error: {f.name} has no circuit named {', '.join(missing)}")
            circs = [circuit_dict(proj.circuits[n]) for n in wanted]
            results.append({"file": f.name, "main": proj.main, "circuits": circs})
            text.append(f"# {f.name}\n\n" + "\n".join(circuit_text(c) for c in circs))
        for p in proj.loaded.values():
            warnings += [f"{p.path.name}: {w}" for w in p.warnings]

    out = json.dumps(results if len(results) > 1 else results[0], indent=2, ensure_ascii=False) + "\n" \
        if args.json else "\n".join(text)
    if args.output:
        Path(args.output).write_text(out, encoding="utf-8")
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stdout.write(out)
    for w in dict.fromkeys(warnings):
        print(f"warning: {w}", file=sys.stderr)


if __name__ == "__main__":
    main()
