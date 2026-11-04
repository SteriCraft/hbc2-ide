#!/usr/bin/env python3
"""Build Logisim-evolution 5.0 circuit files from Python.

A design script describes components and wires on the 10 px grid; this module
writes the .circ file. Port positions come from netlist.py, so a chip pin can
be asked for by name instead of computing offsets by hand.

    from circgen import Project
    p = Project()
    c = p.circuit("add4")
    u1 = c.ttl("74283", (300, 200), label="U1")
    a = c.input((100, 100), "A", width=4)
    ...
    c.wire(a, (u1["A1"][0], 100), u1["A1"])   # orthogonal polyline
    p.save("add4.circ")                        # writes, then runs circcheck

Coordinates follow Logisim: x to the right, y down, everything on a 10 px grid.

Routing helpers:
  * Circuit.wire(*points)       orthogonal polyline
  * Circuit.tap(at, to, bits)   1-end splitter from a bus point to a bit wire
  * Circuit.bus(start, taps)    straight bus line with several taps, split at each tap
  * nested_rails(pins, ...)     rail order for pins on one chip side, so that the
                                L-shaped wires leaving that side never cross

When saving, a wire whose end lands in the middle of another wire splits that
wire (a T-junction, as Logisim's editor does). Component ports are never joined
that way: a port lying in the middle of a wire is reported by the checker.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist as nl  # noqa: E402

Pt = tuple[int, int]

LIBS = ["#Wiring", "#Gates", "#Plexers", "#Arithmetic", "#FPArithmetic", "#Memory", "#I/O",
        "#TTL", "#TCL", "#Base", "#BFH-Praktika", "#Input/Output-Extra", "#Soc"]
LIB_ID = {d: str(i) for i, d in enumerate(LIBS)}

HEADER = """<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<project source="5.0.0" version="1.0">
  This file is intended to be loaded by Logisim-evolution v5.0.0(https://github.com/logisim-evolution/).

  <lib desc="#Wiring" name="0">
    <tool name="Pin">
      <a name="appearance" val="classic"/>
    </tool>
  </lib>
@LIBS@
  <main name="@MAIN@"/>
  <options>
    <a name="gateUndefined" val="ignore"/>
    <a name="simlimit" val="1000"/>
    <a name="simrand" val="0"/>
  </options>
  <mappings>
    <tool map="Button2" name="Poke Tool"/>
    <tool map="Button3" name="Menu Tool"/>
    <tool map="Ctrl Button1" name="Menu Tool"/>
  </mappings>
  <toolbar>
    <tool name="Poke Tool"/>
    <tool name="Edit Tool"/>
    <tool name="Wiring Tool"/>
    <tool name="Text Tool"/>
    <sep/>
    <tool name="Pin"/>
    <tool name="Pin">
      <a name="facing" val="west"/>
      <a name="type" val="output"/>
    </tool>
  </toolbar>
"""


def _esc(s) -> str:
    return (str(s).replace("&", "&amp;").replace('"', "&quot;")
            .replace("<", "&lt;").replace(">", "&gt;"))


def _on_grid(p: Pt) -> bool:
    return p[0] % 10 == 0 and p[1] % 10 == 0


# --------------------------------------------------------------------------
# Components
# --------------------------------------------------------------------------
class Part:
    """A placed component. Index it with a port name (or TTL pin number) to
    get the port location: chip["A1"], chip[5], splitter["end0"], pin["pin"]."""

    def __init__(self, lib: str | None, name: str, loc: Pt, attrs: dict):
        self.lib, self.name, self.loc = lib, name, loc
        self.attrs = {k: str(v) for k, v in attrs.items()}
        self.ports: list[nl.Port] = []
        kind = None
        if lib == "#TTL" and name in nl.TTL:
            kind = "ttl"
        elif lib == "#Wiring" and name == "Splitter":
            kind = "splitter"
        elif lib == "#Wiring" and name in ("Pin", "Tunnel"):
            kind = name.lower()
        elif lib == "#Wiring" and name in nl.WIRING_SINGLE:
            kind = nl.WIRING_SINGLE[name][0]
        if kind:
            c = nl.Comp(kind=kind, name=name, lib=lib or "", loc=loc, attrs=self.attrs)
            if kind == "ttl":
                nl.make_ttl(c)
            elif kind == "splitter":
                nl.make_splitter(c)
            else:
                c.ports = [nl.Port("pin", loc, 1, "inout")]
            self.ports = c.ports

    @property
    def label(self) -> str:
        return self.attrs.get("label", "")

    def __getitem__(self, key) -> Pt:
        if isinstance(key, int):
            hits = [p for p in self.ports if p.pin == key]
        else:
            hits = [p for p in self.ports if p.name == key]
            if not hits:   # "S1" for "∑1"
                hits = [p for p in self.ports if p.name.replace("∑", "S") == key]
        if len(hits) != 1:
            names = ", ".join(f"{p.pin}:{p.name}" if p.pin else p.name for p in self.ports)
            why = "ambiguous (use the pin number)" if hits else "unknown"
            raise KeyError(f"{self.name} at {self.loc}: port {key!r} is {why}; ports: {names}")
        return hits[0].loc

    def xml(self) -> str:
        lib = f' lib="{LIB_ID[self.lib]}"' if self.lib else ""
        head = f'    <comp{lib} loc="({self.loc[0]},{self.loc[1]})" name="{_esc(self.name)}"'
        if not self.attrs:
            return head + "/>"
        body = "".join(f'\n      <a name="{_esc(k)}" val="{_esc(v)}"/>' for k, v in sorted(self.attrs.items()))
        return head + ">" + body + "\n    </comp>"


# --------------------------------------------------------------------------
# Circuit
# --------------------------------------------------------------------------
class Circuit:
    def __init__(self, name: str):
        self.name = name
        self.parts: list[Part] = []
        self.wires: list[tuple[Pt, Pt]] = []

    # ---- generic ----
    def add(self, lib: str | None, name: str, loc: Pt, **attrs) -> Part:
        """Any component. lib is a library description such as "#Gates", or
        None for a subcircuit of the same project."""
        if lib is not None and lib not in LIB_ID:
            raise ValueError(f"unknown library {lib!r}; known: {', '.join(LIBS)}")
        if not _on_grid(loc):
            raise ValueError(f"{name} at {loc}: not on the 10 px grid")
        p = Part(lib, name, loc, attrs)
        self.parts.append(p)
        return p

    def wire(self, *pts: Pt) -> None:
        """Orthogonal polyline through the given points."""
        for a, b in zip(pts, pts[1:]):
            if a[0] != b[0] and a[1] != b[1]:
                raise ValueError(f"diagonal wire {a} -> {b}")
            if not (_on_grid(a) and _on_grid(b)):
                raise ValueError(f"wire {a} -> {b}: not on the 10 px grid")
            if a != b:
                self.wires.append((a, b))

    # ---- common components ----
    def ttl(self, chip: str, loc: Pt, label: str = "", facing: str = "east", **attrs) -> Part:
        if chip not in nl.TTL:
            raise ValueError(f"unknown TTL chip {chip}")
        if facing != "east":
            attrs["facing"] = facing
        if label:
            attrs["label"] = label
        return self.add("#TTL", chip, loc, **attrs)

    def input(self, loc: Pt, label: str, width: int = 1, facing: str = "east") -> Pt:
        """Input pin; returns its port (= loc). facing east: body on the left."""
        attrs = {"appearance": "NewPins", "facing": facing, "label": label}
        if width != 1:
            attrs["width"] = width
        self.add("#Wiring", "Pin", loc, **attrs)
        return loc

    def output(self, loc: Pt, label: str, width: int = 1, facing: str = "west") -> Pt:
        """Output pin; returns its port (= loc). facing west: body on the right."""
        attrs = {"appearance": "NewPins", "facing": facing, "label": label, "type": "output"}
        if width != 1:
            attrs["width"] = width
        self.add("#Wiring", "Pin", loc, **attrs)
        return loc

    def tunnel(self, loc: Pt, label: str, width: int = 1, facing: str = "west") -> Pt:
        attrs = {"facing": facing, "label": label}
        if width != 1:
            attrs["width"] = width
        self.add("#Wiring", "Tunnel", loc, **attrs)
        return loc

    def text(self, loc: Pt, text: str, **attrs) -> None:
        self.add("#Base", "Text", loc, text=text, **attrs)

    def constant(self, loc: Pt, value: int, width: int = 1, facing: str = "east") -> Pt:
        attrs = {"facing": facing, "value": hex(value)}
        if width != 1:
            attrs["width"] = width
        self.add("#Wiring", "Constant", loc, **attrs)
        return loc

    def splitter(self, loc: Pt, width: int, ends: list, facing: str = "east",
                 appear: str = "left", spacing: int = 1) -> Part:
        """Splitter with combined port at loc. ends[k] is a bit number or a
        list of bit numbers carried by end k; bits not listed are unused."""
        attrs = {"facing": facing, "fanout": len(ends), "incoming": width, "appear": appear}
        if spacing != 1:
            attrs["spacing"] = spacing
        owner = {}
        for k, bits in enumerate(ends):
            for b in ([bits] if isinstance(bits, int) else bits):
                if b in owner:
                    raise ValueError(f"splitter at {loc}: bit {b} is on two ends")
                owner[b] = k
        for b in range(width):
            attrs[f"bit{b}"] = owner.get(b, "none")
        return self.add("#Wiring", "Splitter", loc, **attrs)

    def tap(self, at: Pt, to: Pt, bits, width: int) -> Part:
        """1-end splitter whose combined port is at `at` (on a bus) and whose
        end is at `to`. `to` must be one of the offsets a splitter can reach
        from `at`: (+-20, +-10), (+-10, +-20) or straight (+-20, 0)/(0, +-20)."""
        for facing in ("east", "west", "north", "south"):
            for appear in ("left", "right", "center"):
                trial = Part("#Wiring", "Splitter", at, {"facing": facing, "fanout": 1, "appear": appear,
                                                         "incoming": width})
                if trial.ports[1].loc == to:
                    return self.splitter(at, width, [bits], facing=facing, appear=appear)
        raise ValueError(f"no splitter goes from {at} to {to}")

    def bus(self, start: Pt, taps: list, width: int, end: Pt | None = None) -> list[Pt]:
        """A straight bus line from `start` with 1-end taps.

        taps: list of (end_point, bits). The bus runs vertically if every end
        point is 20 px left or right of start's x, horizontally if 20 px above
        or below start's y. Each tap sits 10 px before its end point (seen from
        `start`), so the bit wire leaves at end_point. The line stops at the
        last tap, or at `end` if given. Returns the tap points on the bus."""
        x0, y0 = start
        vertical = all(abs(e[0] - x0) == 20 for e, _ in taps)
        horizontal = all(abs(e[1] - y0) == 20 for e, _ in taps)
        if not (vertical or horizontal):
            raise ValueError(f"bus at {start}: tap ends must all be 20 px to the side of the bus line")
        points = []
        for e, bits in taps:
            if vertical:
                step = 10 if e[1] > y0 else -10
                at = (x0, e[1] - step)
            else:
                step = 10 if e[0] > x0 else -10
                at = (e[0] - step, y0)
            self.tap(at, e, bits, width)
            points.append(at)
        line = sorted(set(points + [start] + ([end] if end else [])),
                      key=lambda p: abs(p[0] - x0) + abs(p[1] - y0))
        for a, b in zip(line, line[1:]):
            self.wire(a, b)
        return points

    # ---- output ----
    def _split_wires(self) -> list[tuple[Pt, Pt]]:
        """Split wires at the ends of other wires (T-junctions)."""
        ends = {p for w in self.wires for p in w}
        out = set()
        for a, b in self.wires:
            if a[0] == b[0]:
                lo, hi = sorted((a[1], b[1]))
                cuts = sorted({p[1] for p in ends if p[0] == a[0] and lo < p[1] < hi})
                pts = [(a[0], y) for y in [lo] + cuts + [hi]]
            else:
                lo, hi = sorted((a[0], b[0]))
                cuts = sorted({p[0] for p in ends if p[1] == a[1] and lo < p[0] < hi})
                pts = [(x, a[1]) for x in [lo] + cuts + [hi]]
            out.update(zip(pts, pts[1:]))
        return sorted(out)

    def xml(self) -> str:
        lines = [f'  <circuit name="{_esc(self.name)}">',
                 '    <a name="appearance" val="logisim_evolution"/>',
                 f'    <a name="circuit" val="{_esc(self.name)}"/>',
                 '    <a name="circuitnamedboxfixedsize" val="true"/>',
                 '    <a name="simulationFrequency" val="1.0"/>']
        lines += [p.xml() for p in self.parts]
        lines += [f'    <wire from="({a[0]},{a[1]})" to="({b[0]},{b[1]})"/>' for a, b in self._split_wires()]
        lines.append("  </circuit>")
        return "\n".join(lines)


# --------------------------------------------------------------------------
# Routing helpers
# --------------------------------------------------------------------------
def nested_rails(pins: dict, toward: str, first: int = 20, pitch: int = 20) -> dict:
    """Rail positions for pins on one straight chip edge.

    pins:   {name: port location}, all on one horizontal or vertical line.
    toward: where the wires leave: "left"/"right" for pins on a horizontal
            edge, "up"/"down" for pins on a vertical edge.
    Each pin's wire goes straight away from the chip to its rail, then turns
    toward the exit. The pin closest to the exit gets the rail closest to the
    chip, so no two of these wires cross.

    Returns {name: offset from the edge}, first, first + pitch, ... Add or
    subtract it from the edge coordinate depending on which side the rails are.
    """
    if toward in ("left", "right"):
        key = lambda n: pins[n][0]   # noqa: E731
    elif toward in ("up", "down"):
        key = lambda n: pins[n][1]   # noqa: E731
    else:
        raise ValueError(f"toward must be left/right/up/down, not {toward!r}")
    order = sorted(pins, key=key, reverse=toward in ("right", "down"))
    return {n: first + i * pitch for i, n in enumerate(order)}


# --------------------------------------------------------------------------
# Project
# --------------------------------------------------------------------------
class Project:
    def __init__(self):
        self.circuits: list[Circuit] = []

    def circuit(self, name: str) -> Circuit:
        c = Circuit(name)
        self.circuits.append(c)
        return c

    def xml(self, main: str | None = None) -> str:
        if not self.circuits:
            raise ValueError("project has no circuit")
        libs = "\n".join(f'  <lib desc="{d}" name="{i}"/>' for i, d in enumerate(LIBS) if i)
        head = HEADER.replace("@LIBS@", libs).replace("@MAIN@", _esc(main or self.circuits[0].name))
        return head + "\n".join(c.xml() for c in self.circuits) + "\n</project>\n"

    def save(self, path, main: str | None = None, check: bool = True) -> int:
        """Write the file, then run circcheck and the conventions check on it.
        Returns the number of errors."""
        path = Path(path)
        path.write_text(self.xml(main), encoding="utf-8")
        n_wires = sum(len(c._split_wires()) for c in self.circuits)
        n_parts = sum(len(c.parts) for c in self.circuits)
        print(f"wrote {path} ({n_parts} components, {n_wires} wires)")
        if not check:
            return 0
        import circcheck
        import conventions
        return circcheck.report(path) + conventions.report(path)
