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

import os
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

    def __init__(self, lib: "str | Library | None", name: str, loc: Pt, attrs: dict):
        self.lib, self.name, self.loc = lib, name, loc
        self.attrs = {k: str(v) for k, v in attrs.items()}
        self.ports: list[nl.Port] = []
        kind = None
        if isinstance(lib, Library):          # subcircuit from another .circ file
            c = nl.Comp(kind="sub", name=name, lib="file#", loc=loc, attrs=self.attrs, sub=lib[name])
            nl.make_ports(None, c)
            self.ports = c.ports
        elif (lib, name) in nl.GENERIC:
            kind = "generic"
        elif lib == "#TTL" and name in nl.TTL:
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
            elif kind == "generic":
                nl.make_generic(c)
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
        lib_id = self.lib.ident if isinstance(self.lib, Library) else LIB_ID.get(self.lib)
        lib = f' lib="{lib_id}"' if self.lib else ""
        head = f'    <comp{lib} loc="({self.loc[0]},{self.loc[1]})" name="{_esc(self.name)}"'
        if not self.attrs:
            return head + "/>"
        body = "".join(f'\n      <a name="{_esc(k)}" val="{_esc(v)}"/>' for k, v in sorted(self.attrs.items()))
        return head + ">" + body + "\n    </comp>"


class Library:
    """Another .circ file loaded as a Logisim library (Load Library > Logisim
    Library). Get one from Project.library(); index it with a circuit name."""

    def __init__(self, ident: str, path: Path):
        self.ident, self.path = ident, path.resolve()
        self.project = nl.Project(self.path)

    def __getitem__(self, name: str) -> nl.Circuit:
        if name not in self.project.circuits:
            raise KeyError(f"{self.path.name} has no circuit {name!r}; circuits: {', '.join(self.project.circuits)}")
        return self.project.circuits[name]

    def desc(self, saved_in: Path) -> str:
        """file#<path relative to the saving file>, written as Logisim writes
        it on this OS (backslashes on Windows), so a save in Logisim keeps it."""
        rel = os.path.relpath(self.path, Path(saved_in).resolve().parent)
        return "file#" + rel.replace("/", os.sep)


# --------------------------------------------------------------------------
# Custom symbol (appearance)
# --------------------------------------------------------------------------
CHAR_W = 7          # approximate width of a Dialog 12 character, in px


def _r10(v: float) -> int:
    return int(-(-v // 10) * 10)       # round up to the grid


def symbol_xml(name: str, sides: dict, pins: dict) -> str:
    """<appear> block: a box with the module name, ports on the given sides.

    sides: {"top": [labels], "left": [...], "right": [...], "bottom": [...]}
    pins:  {label: (loc of the Pin in the circuit, width, is_output)}
    Every pin must appear on exactly one side. Labels are drawn inside the
    box, next to their port; the anchor is the first right port (or the
    first left one)."""
    side = {k: list(sides.get(k, [])) for k in ("top", "left", "right", "bottom")}
    listed = [lab for v in side.values() for lab in v]
    if sorted(listed) != sorted(pins):
        raise ValueError(f"symbol of {name}: sides list {sorted(listed)}, pins are {sorted(pins)}")
    slot = lambda lab: _r10(len(lab) * CHAR_W + 20)                     # noqa: E731
    row_w = lambda labs: sum(slot(lab) for lab in labs)                 # noqa: E731
    lw = max((len(lab) * CHAR_W for lab in side["left"]), default=0)
    rw = max((len(lab) * CHAR_W for lab in side["right"]), default=0)
    W = _r10(max(row_w(side["top"]), row_w(side["bottom"]), lw + rw + len(name) * 9 + 60, 100))
    top_band = 30 if side["top"] else 10
    bot_band = 30 if side["bottom"] else 10
    rows = max(len(side["left"]), len(side["right"]), 1)
    H = top_band + rows * 30 + bot_band
    x0, y0 = 60, 50
    L = []
    T = ('<text dominant-baseline="alphabetic" font-family="Dialog" font-size="12" '
         'text-anchor="{a}" x="{x}" y="{y}">{t}</text>')
    ports = []                                     # (x, y, label)

    def stub(x, y, horizontal, width):
        t = 4 if width > 1 else 3
        if horizontal:
            L.append(f'<rect height="{t}" stroke="none" width="10" x="{x}" y="{y - t // 2}"/>')
        else:
            L.append(f'<rect height="10" stroke="none" width="{t}" x="{x - 1}" y="{y}"/>')

    L.append(f'<rect fill="none" height="{H}" stroke="#000000" stroke-width="2" width="{W}" x="{x0}" y="{y0}"/>')
    for edge in ("top", "bottom"):
        labs = side[edge]
        x = x0 + (W - row_w(labs)) // 20 * 10      # keep ports on the 10 px grid
        for lab in labs:
            cx = x + slot(lab) // 2 // 10 * 10
            x += slot(lab)
            if edge == "top":
                ports.append((cx, y0 - 10, lab))
                stub(cx, y0 - 10, False, pins[lab][1])
                L.append(T.format(a="middle", x=cx, y=y0 + 17, t=_esc(lab)))
            else:
                ports.append((cx, y0 + H + 10, lab))
                stub(cx, y0 + H, False, pins[lab][1])
                L.append(T.format(a="middle", x=cx, y=y0 + H - 8, t=_esc(lab)))
    for edge in ("left", "right"):
        for i, lab in enumerate(side[edge]):
            y = y0 + top_band + 20 + 30 * i
            if edge == "left":
                ports.append((x0 - 10, y, lab))
                stub(x0 - 10, y, True, pins[lab][1])
                L.append(T.format(a="start", x=x0 + 5, y=y + 4, t=_esc(lab)))
            else:
                ports.append((x0 + W + 10, y, lab))
                stub(x0 + W, y, True, pins[lab][1])
                L.append(T.format(a="end", x=x0 + W - 5, y=y + 4, t=_esc(lab)))
    mid = y0 + top_band + rows * 30 // 2
    L.append('<text dominant-baseline="alphabetic" font-family="Dialog" font-size="14" font-weight="bold" '
             f'text-anchor="middle" x="{x0 + W // 2}" y="{mid + 5}">{_esc(name)}</text>')
    anchor = next(((x, y) for x, y, lab in ports if lab in side["right"][:1]),
                  next(((x, y) for x, y, lab in ports if lab in side["left"][:1]), ports[0][:2]))
    L.append(f'<circ-anchor facing="east" x="{anchor[0]}" y="{anchor[1]}"/>')
    for x, y, lab in ports:
        (px, py), _, out = pins[lab]
        L.append(f'<circ-port dir="{"out" if out else "in"}" pin="{px},{py}" x="{x}" y="{y}"/>')
    return "    <appear>\n" + "".join(f"      {ln}\n" for ln in L) + "    </appear>"


def dip_symbol_xml(title: str, order: list, pins: dict, extra: list = ()) -> str:
    """<appear> block of a chip model, drawn like Logisim's TTL chips: pins
    1..n/2 along the bottom from the left, n/2+1..n along the top from the
    right, 20 px apart. Each pin shows its number and its datasheet name.

    order: pin labels in pin-number order (pin 1 first).
    pins:  {label: (loc of the Pin in the circuit, width, is_output)}
    The anchor is pin 1.
    extra: simulation-only pins (no number, not on the real chip: a model's
    data-in pins, ir-mem-decisions.md M2 / M3), on the left edge, 10 px apart."""
    n = len(order)
    if n % 2 or sorted(list(order) + list(extra)) != sorted(pins) or len(extra) > 8:
        raise ValueError(f"DIP symbol of {title}: pins {sorted(order)} vs circuit pins {sorted(pins)}")
    half = n // 2
    W, H, x0, y0 = half * 20, 80, 50, 50
    L = [f'<rect fill="none" height="{H}" stroke="#000000" stroke-width="2" width="{W}" x="{x0}" y="{y0}"/>',
         f'<rect fill="none" height="16" stroke="#000000" stroke-width="2" width="6" x="{x0}" y="{y0 + H // 2 - 8}"/>']
    T = ('<text dominant-baseline="alphabetic" font-family="Dialog" font-size="{s}" '
         'text-anchor="middle" x="{x}" y="{y}">{t}</text>')
    ports = []
    for i, lab in enumerate(order):
        num = i + 1
        if i < half:                                  # bottom, left to right
            x, y = x0 + 20 * i + 10, y0 + H + 10
            L.append(f'<rect height="10" stroke="none" width="3" x="{x - 1}" y="{y0 + H}"/>')
            L.append(T.format(s=8, x=x, y=y0 + H - 4, t=num))
            L.append(T.format(s=8, x=x, y=y0 + H - 14, t=_esc(lab)))
        else:                                         # top, right to left
            x, y = x0 + W - 20 * (i - half) - 10, y0 - 10
            L.append(f'<rect height="10" stroke="none" width="3" x="{x - 1}" y="{y}"/>')
            L.append(T.format(s=8, x=x, y=y0 + 10, t=num))
            L.append(T.format(s=8, x=x, y=y0 + 20, t=_esc(lab)))
        ports.append((x, y, lab))
    for k, lab in enumerate(extra):                   # left edge, outside the numbering
        x, y = x0 - 10, y0 + 10 * k
        L.append(f'<rect height="2" stroke="none" width="10" x="{x}" y="{y - 1}"/>')
        L.append(f'<text dominant-baseline="central" font-family="Dialog" font-size="7" '
                 f'text-anchor="start" x="{x0 + 3}" y="{y}">{_esc(lab)}</text>')
        ports.append((x, y, lab))
    L.append('<text dominant-baseline="alphabetic" font-family="Dialog" font-size="12" font-weight="bold" '
             f'text-anchor="middle" x="{x0 + W // 2}" y="{y0 + H // 2 + 5}">{_esc(title)}</text>')
    L.append(f'<circ-anchor facing="east" x="{ports[0][0]}" y="{ports[0][1]}"/>')
    for x, y, lab in ports:
        (px, py), _, out = pins[lab]
        L.append(f'<circ-port dir="{"out" if out else "in"}" pin="{px},{py}" x="{x}" y="{y}"/>')
    return "    <appear>\n" + "".join(f"      {ln}\n" for ln in L) + "    </appear>"


# --------------------------------------------------------------------------
# Circuit
# --------------------------------------------------------------------------
class Circuit:
    def __init__(self, name: str):
        self.name = name
        self.parts: list[Part] = []
        self.wires: list[tuple[Pt, Pt]] = []
        self._symbol: dict | None = None
        self._dip: tuple | None = None

    # ---- generic ----
    def add(self, lib: "str | Library | None", name: str, loc: Pt, **attrs) -> Part:
        """Any component. lib is a library description such as "#Gates", a
        Library (subcircuit of another file), or None for a subcircuit of the
        same project."""
        if lib is not None and not isinstance(lib, Library) and lib not in LIB_ID:
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

    def sub(self, lib: Library, name: str, loc: Pt, label: str = "", facing: str = "east") -> Part:
        """Instance of circuit `name` from a library file. part["<pin label>"]
        gives the location of that port."""
        attrs = {}
        if facing != "east":
            attrs["facing"] = facing
        if label:
            attrs["label"] = label
        return self.add(lib, name, loc, **attrs)

    def symbol(self, top=(), left=(), right=(), bottom=()) -> None:
        """Custom symbol: each list names the pins (by label) on that side.
        Conventions: data in left, outputs right, control top, flags bottom."""
        self._symbol = {"top": list(top), "left": list(left), "right": list(right), "bottom": list(bottom)}

    def model(self, chip: "Circuit", loc: Pt, label: str = "") -> Part:
        """Instance of a chip model drawn in the same project (its test
        bench). part[n] / part["D0"] give the pin locations, from the DIP
        symbol: pin 1 is the anchor, at loc."""
        if chip._dip is None:
            raise ValueError(f"{chip.name} has no DIP symbol")
        order = chip._dip[1]
        widths = {p.label: (int(p.attrs.get("width", 1)), p.attrs.get("type") == "output")
                  for p in chip.parts if p.lib == "#Wiring" and p.name == "Pin"}
        half = len(order) // 2
        part = self.add(None, chip.name, loc, **({"label": label} if label else {}))
        for i, lab in enumerate(order):
            dx, dy = (20 * i, 0) if i < half else (20 * half - 20 * (i - half) - 20, -100)
            w, out = widths[lab]
            part.ports.append(nl.Port(lab, (loc[0] + dx, loc[1] + dy), w, "out" if out else "in", i + 1))
        for k, lab in enumerate(chip._dip[2]):          # extra pins: left edge, no number
            w, out = widths[lab]
            part.ports.append(nl.Port(lab, (loc[0] - 20, loc[1] - 90 + 10 * k), w, "out" if out else "in"))
        return part

    def dip_symbol(self, title: str, order: list, extra: list = ()) -> None:
        """Symbol of a chip model (circuits/chips/): a DIP outline with the
        pins in datasheet order, `order` = pin labels from pin 1 to pin n;
        `extra` = simulation-only pins on the left edge (no pin number)."""
        self._dip = (title, list(order), list(extra))

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

    def move(self, dx: int, dy: int) -> None:
        """Shift the whole drawing. Call it last: Part locations taken before
        the move keep their old values. Used to keep a margin on the left and
        top, where Logisim can't show labels that fall below x = 0 or y = 0."""
        if not _on_grid((dx, dy)):
            raise ValueError(f"move ({dx}, {dy}): not on the 10 px grid")
        for p in self.parts:
            p.loc = (p.loc[0] + dx, p.loc[1] + dy)
            for port in p.ports:
                port.loc = (port.loc[0] + dx, port.loc[1] + dy)
        self.wires = [((a[0] + dx, a[1] + dy), (b[0] + dx, b[1] + dy)) for a, b in self.wires]

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
        custom = self._symbol is not None or self._dip is not None
        lines = [f'  <circuit name="{_esc(self.name)}">',
                 f'    <a name="appearance" val="{"custom" if custom else "logisim_evolution"}"/>',
                 f'    <a name="circuit" val="{_esc(self.name)}"/>',
                 '    <a name="circuitnamedboxfixedsize" val="true"/>',
                 '    <a name="simulationFrequency" val="1.0"/>']
        if custom:
            pins = {p.label: (p.loc, int(p.attrs.get("width", 1)), p.attrs.get("type") == "output")
                    for p in self.parts if p.lib == "#Wiring" and p.name == "Pin"}
            lines.append(dip_symbol_xml(self._dip[0], self._dip[1], pins, self._dip[2]) if self._dip else symbol_xml(self.name, self._symbol, pins))
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


def rails_left(c: "Circuit", chip: Part, signals: dict, line_x: dict, taps: dict, ends: dict,
               first: int = 20, pitch: int = 20) -> None:
    """The add_sub8 wiring style for a chip facing east: every pin in
    `signals` leaves its edge (top pins up, bottom pins down) on a nested
    rail, then runs left to:

      ("<bus>", bit)  a 1-bit tap 20 px right of line_x[bus]; the tap end is
                      added to taps[bus] (draw the bus later with c.bus)
      ("<line>", None) a 1-bit vertical line at line_x[line]; the rail's y is
                      added to ends[line] (draw the line through them later)
      ("gnd", None) / ("vcc", None)  a Ground / Power facing west, 10 px left
                      of the chip's anchor
      ("tunnel", name)  a 1-bit tunnel facing east, 10 px left of the anchor
                      (use pitch=30 so the tunnel boxes don't touch)
    A line is a bus when it is a key of `taps`, a 1-bit line when it is a key
    of `ends`."""
    ys = [p.loc[1] for p in chip.ports]
    for edge_y, away in ((min(ys), -1), (max(ys), 1)):     # top edge, bottom edge
        pins = {n: chip[n] for n in signals if chip[n][1] == edge_y}
        for name, depth in nested_rails(pins, toward="left", first=first, pitch=pitch).items():
            line, bit = signals[name]
            y = edge_y + away * depth
            if line == "tunnel":
                end = (chip.loc[0] - 10, y)
                c.tunnel(end, bit, facing="east")
            elif line in ("gnd", "vcc"):
                end = (chip.loc[0] - 10, y)
                c.add("#Wiring", "Ground" if line == "gnd" else "Power", end, facing="west")
            elif line in taps:
                end = (line_x[line] + 20, y)
                taps[line].append((end, bit))
            elif line in ends:
                end = (line_x[line], y)
                ends[line].append(y)
            else:
                raise KeyError(f"{chip.label}: {name} goes to unknown line {line!r}")
            c.wire(end, (chip[name][0], y), chip[name])


def pin_tunnels(c: "Circuit", chip: Part, names: dict, first: int = 20, stagger: int = 30, levels: int = 2) -> None:
    """Wire chip pins to tunnels: a stub straight out of the chip, then a
    tunnel. names maps a pin (number or name) to a tunnel label, or to
    "gnd" / "vcc" for a Ground / Power. Works for chips on any facing: pins
    on the left or right side get 20 px stubs (tunnel text runs sideways);
    top and bottom pins cycle through `levels` depths (2 by default) so the
    tunnel boxes don't touch; 20 px pin pitch with long labels needs ~6."""
    xs = {p.loc[0] for p in chip.ports}
    ys = {p.loc[1] for p in chip.ports}
    TUNNEL = {(-1, 0): "east", (1, 0): "west", (0, 1): "north", (0, -1): "south"}
    CONST = {(-1, 0): "west", (1, 0): "east", (0, 1): "south", (0, -1): "north"}
    if len(xs) == 2:                                           # pins on the left and right sides
        stubs = [(k, (-1 if chip[k][0] == min(xs) else 1, 0), first) for k in names]
    else:                                                      # pins on the top and bottom
        stubs = []
        for edge_y, away in ((max(ys), 1), (min(ys), -1)):
            row = sorted((k for k in names if chip[k][1] == edge_y), key=lambda k: chip[k][0])
            stubs += [(k, (0, away), first + stagger * (i % levels)) for i, k in enumerate(row)]
    for k, d, depth in stubs:
        x, y = chip[k]
        end = (x + d[0] * depth, y + d[1] * depth)
        c.wire((x, y), end)
        if names[k] in ("gnd", "vcc"):
            c.add("#Wiring", "Ground" if names[k] == "gnd" else "Power", end, facing=CONST[d])
        else:
            c.tunnel(end, names[k], facing=TUNNEL[d])


def bus_tunnels(c: "Circuit", at: Pt, label: str, width: int, output: bool = False, gap: int = 20) -> None:
    """A pin `label` at `at`, split into 1-bit tunnels named label_0,
    label_1, ... (bit 0 first, going down). An input pin faces east
    (tunnels on its right), an output pin faces west (tunnels on its left).
    A 1-bit pin gets one tunnel named `label`."""
    x, y = at
    s = -1 if output else 1
    (c.output if output else c.input)(at, label, width)
    if width == 1:
        c.wire(at, (x + 20 * s, y))
        c.tunnel((x + 20 * s, y), label, facing="east" if output else "west")
        return
    c.wire(at, (x + 20 * s, y))
    sp = c.splitter((x + 20 * s, y), width, [[k] for k in range(width)], facing="west" if output else "east",
                    appear="left" if output else "right", spacing=gap // 10)
    for k in range(width):
        e = sp[f"end{k}"]
        c.wire(e, (e[0] + 20 * s, e[1]))
        c.tunnel((e[0] + 20 * s, e[1]), f"{label}_{k}", facing="east" if output else "west")


# --------------------------------------------------------------------------
# Project
# --------------------------------------------------------------------------
class Project:
    def __init__(self):
        self.circuits: list[Circuit] = []
        self.libraries: list[Library] = []

    def circuit(self, name: str) -> Circuit:
        c = Circuit(name)
        self.circuits.append(c)
        return c

    def library(self, path) -> Library:
        """Load another .circ file as a library (relative path written on save)."""
        path = Path(path).resolve()
        for lib in self.libraries:
            if lib.path == path:
                return lib
        lib = Library(str(len(LIBS) + len(self.libraries)), path)
        self.libraries.append(lib)
        return lib

    def xml(self, main: str | None = None, path=None) -> str:
        """The .circ text. `path` (where it will be saved) is needed when
        the project loads libraries, to write their relative paths."""
        if not self.circuits:
            raise ValueError("project has no circuit")
        if self.libraries and path is None:
            raise ValueError("a project with libraries needs the output path")
        libs = [f'  <lib desc="{d}" name="{i}"/>' for i, d in enumerate(LIBS) if i]
        libs += [f'  <lib desc="{_esc(lib.desc(path))}" name="{lib.ident}"/>' for lib in self.libraries]
        head = HEADER.replace("@LIBS@", "\n".join(libs)).replace("@MAIN@", _esc(main or self.circuits[0].name))
        return head + "\n".join(c.xml() for c in self.circuits) + "\n</project>\n"

    def save(self, path, main: str | None = None, check: bool = True) -> int:
        """Write the file, then run circcheck and the conventions check on it.
        Returns the number of errors."""
        path = Path(path)
        path.write_text(self.xml(main, path), encoding="utf-8")
        n_wires = sum(len(c._split_wires()) for c in self.circuits)
        n_parts = sum(len(c.parts) for c in self.circuits)
        print(f"wrote {path} ({n_parts} components, {n_wires} wires)")
        if not check:
            return 0
        import circcheck
        import conventions
        return circcheck.report(path) + conventions.report(path)
