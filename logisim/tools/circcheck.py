#!/usr/bin/env python3
"""Check the wiring and layout of Logisim-evolution (.circ) files.

Finds the mistakes that are easy to make in Logisim and hard to see:

  errors    port in the middle of a wire (Logisim does not connect it),
            wires overlapping on the same line, diagonal wires,
            bit width mismatches
  warnings  wire end in the middle of another wire without a split,
            dangling wire ends, unconnected inputs, wires running through a
            chip, two separate wires on one line with a small gap (they look
            joined), coordinates off the 10 px grid
  info      number of wire crossings and of junctions, per circuit

Usage:
    python circcheck.py add8_ttl.circ            # looked up in ../circuits too
    python circcheck.py cpu.circ -c add_sub8     # one circuit only
    python circcheck.py file.circ --strict       # warnings also fail (exit 1)

Library files loaded by the file (Load Library > Logisim Library) are
checked too, each once (--no-libs to skip them); a missing one is an error.
"""

from __future__ import annotations

import argparse
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist as nl  # noqa: E402

GAP = 20   # collinear wires closer than this look joined


@dataclass
class Issue:
    level: str      # error / warning
    circuit: str
    where: tuple
    text: str


def _interior(w, p) -> bool:
    """p strictly inside segment w (axis-aligned)."""
    (x1, y1), (x2, y2) = w
    if x1 == x2 == p[0]:
        return min(y1, y2) < p[1] < max(y1, y2)
    if y1 == y2 == p[1]:
        return min(x1, x2) < p[0] < max(x1, x2)
    return False


def ttl_body(c: nl.Comp):
    """Chip body rectangle (x1, y1, x2, y2), without the pins."""
    pins, height = nl.TTL[c.name][0], nl.TTL[c.name][1]
    facing = c.attrs.get("facing", "east")
    corners = [nl.rotate(dx, dy, "east", facing) for dx, dy in ((0, -20), (pins * 10, height - 40))]
    xs = [c.loc[0] + dx for dx, _ in corners]
    ys = [c.loc[1] + dy for _, dy in corners]
    return min(xs), min(ys), max(xs), max(ys)


def _crosses_box(w, box) -> bool:
    (x1, y1), (x2, y2) = w
    bx1, by1, bx2, by2 = box
    if x1 == x2:
        return bx1 < x1 < bx2 and max(min(y1, y2), by1) < min(max(y1, y2), by2)
    return by1 < y1 < by2 and max(min(x1, x2), bx1) < min(max(x1, x2), bx2)


def check_circuit(circ: nl.Circuit) -> tuple[list[Issue], dict]:
    circ.build()
    name = circ.name
    issues: list[Issue] = []

    def add(level, where, text):
        issues.append(Issue(level, name, where, text))

    wires = circ.wires
    ends = Counter(p for w in wires for p in w)
    ports = defaultdict(list)          # loc -> [(comp, port)]
    for c in circ.comps:
        for p in c.ports:
            ports[p.loc].append((c, p))
    unknown = [c for c in circ.comps if c.kind == "unsupported"]

    # grid and shape
    for w in wires:
        a, b = w
        if a[0] != b[0] and a[1] != b[1]:
            add("error", a, f"diagonal wire {a} -> {b}")
        for p in w:
            if p[0] % 10 or p[1] % 10:
                add("warning", p, f"wire end {p} is off the 10 px grid")
    for c in circ.comps:
        if c.loc[0] % 10 or c.loc[1] % 10:
            add("warning", c.loc, f"{c.ident} is off the 10 px grid")

    # collinear wires: overlap or small gap
    def corner(p, axis):
        """A wire leaves p across the line: the gap reads as two turns, not one wire."""
        return any(p in w and (w[0][1] != w[1][1] if axis == "h" else w[0][0] != w[1][0]) for w in wires)

    rows = defaultdict(list)
    for w in wires:
        (x1, y1), (x2, y2) = w
        if y1 == y2:
            rows[("h", y1)].append((min(x1, x2), max(x1, x2)))
        elif x1 == x2:
            rows[("v", x1)].append((min(y1, y2), max(y1, y2)))
    for (axis, k), segs in rows.items():
        segs.sort()
        for (a1, a2), (b1, b2) in zip(segs, segs[1:]):
            pt = (lambda v: (v, k)) if axis == "h" else (lambda v: (k, v))
            if b1 < a2:
                add("error", pt(b1), f"wires overlap on the same line between {pt(b1)} and {pt(min(a2, b2))}")
            elif (0 < b1 - a2 <= GAP and not ports.get(pt(a2)) and not ports.get(pt(b1))
                  and not (corner(pt(a2), axis) and corner(pt(b1), axis))):
                add("warning", pt(a2), f"wires {pt(a2)} and {pt(b1)} are on one line with a {b1 - a2} px gap: "
                                       "they look joined but are not")

    # wire ends and ports in the middle of wires
    for p in set(ends) | set(ports):
        on = [w for w in wires if _interior(w, p)]
        if not on:
            continue
        if p in ports:
            who = ", ".join(f"{c.ident}.{nl.port_label(c, q)}" for c, q in ports[p])
            add("error", p, f"port {who} lies in the middle of a wire: Logisim does not connect it")
        else:
            add("warning", p, f"wire end {p} lies in the middle of another wire without a split")

    # dangling wire ends
    near_unknown = lambda p: any(abs(p[0] - c.loc[0]) <= 60 and abs(p[1] - c.loc[1]) <= 60  # noqa: E731
                                 for c in unknown)
    for p, n in ends.items():
        if n == 1 and p not in ports and not any(_interior(w, p) for w in wires) and not near_unknown(p):
            add("warning", p, f"dangling wire end at {p}")

    # unconnected ports
    for loc, plist in ports.items():
        if ends.get(loc) or len(plist) > 1:
            continue
        c, p = plist[0]
        if c.kind == "pin":
            if not (c.label in ("VCC", "GND") and nl.chip_model(circ)):   # unconnected by design
                add("warning", loc, f"pin {c.ident} is not connected")
        elif c.kind in ("ttl", "sub", "generic") and p.dir == "in" and p.name not in ("VCC", "GND"):
            add("warning", loc, f"input {c.ident}.{nl.port_label(c, p)} is not connected (floating)")
        elif c.kind == "splitter" and p.width:
            add("warning", loc, f"splitter {c.ident} {p.name} is not connected")
        elif c.kind == "tunnel":
            add("warning", loc, f"tunnel {c.label} is not connected")

    # a Ground / Power / Constant tied to a net that something else drives:
    # usually a rail that ran into a constant's port by accident
    for net in circ.nets:
        consts, drivers = [], []
        for ci, pi in net.members:
            c = circ.comps[ci]
            p = c.ports[pi]
            if c.kind in ("const", "power", "ground"):
                consts.append(c)
            elif (c.kind in ("ttl", "sub", "generic") and p.dir == "out") or \
                    (c.kind == "pin" and c.attrs.get("type") != "output"):
                drivers.append(f"{c.ident}.{nl.port_label(c, p)}")
        if consts and (drivers or len(consts) > 1):
            what = ", ".join([c.ident for c in consts] + drivers)
            add("error", consts[0].loc, f"constant driven by something else on net {net.name}: {what}")

    # wires through chips
    for c in circ.comps:
        if c.kind == "ttl":
            box = ttl_body(c)
            for w in wires:
                if _crosses_box(w, box):
                    add("warning", w[0], f"wire {w[0]} -> {w[1]} runs through {c.ident} ({c.name})")

    # width mismatches (netlist computes them while building)
    for msg in circ.project.warnings:
        if msg.startswith(f"{name}: bit width mismatch"):
            add("error", (0, 0), msg.split(": ", 1)[1])

    # stats
    hs = [w for w in wires if w[0][1] == w[1][1]]
    vs = [w for w in wires if w[0][0] == w[1][0]]
    crossings = sum(1 for h in hs for v in vs
                    if min(h[0][0], h[1][0]) < v[0][0] < max(h[0][0], h[1][0])
                    and min(v[0][1], v[1][1]) < h[0][1] < max(v[0][1], v[1][1]))
    junctions = sum(1 for p, n in ends.items() if n + len(ports.get(p, [])) >= 3)
    stats = {"components": len(circ.comps), "wires": len(wires), "crossings": crossings,
             "junctions": junctions, "unsupported": sorted({c.name for c in unknown})}
    return issues, stats


def check_file(path: Path, circuits=None, libs: bool = True):
    """[(file, circuit name, issues, stats)] for the file and, with libs, for
    every library file it loads (each once). `circuits` filters the first
    file only. A missing library file is an error on the first circuit."""
    proj = nl.Project(Path(path))
    out = []
    for p in proj.libraries() if libs else [proj]:
        missing = [d for d, lib in p.file_libs.values() if lib is None]
        for cname, circ in p.circuits.items():
            if p is proj and circuits and cname not in circuits:
                continue
            issues, stats = check_circuit(circ)
            for d in missing:
                issues.append(Issue("error", cname, (0, 0), f"library file {d[5:]} not found"))
            missing = []
            out.append((p.path, cname, issues, stats))
        for d in missing:      # a file with no circuit
            out.append((p.path, "-", [Issue("error", "-", (0, 0), f"library file {d[5:]} not found")],
                        {"components": 0, "wires": 0, "crossings": 0, "junctions": 0, "unsupported": []}))
    return out


def report(path, circuits=None, strict: bool = False, libs: bool = True) -> int:
    """Print the check of one file (and of the libraries it loads). Returns
    the number of failures."""
    failures = 0
    shown = None
    for fpath, cname, issues, st in check_file(Path(path), circuits, libs):
        if fpath != shown:
            print(f"[{_rel(fpath)}]")
            shown = fpath
        errs = sum(i.level == "error" for i in issues)
        warns = len(issues) - errs
        line = (f"{cname}: {st['components']} components, {st['wires']} wires, "
                f"{st['crossings']} crossings, {st['junctions']} junctions"
                f" -- {errs} errors, {warns} warnings")
        print(line)
        if st["unsupported"]:
            print(f"  note: not checked (geometry unknown): {', '.join(st['unsupported'])}")
        for i in sorted(issues, key=lambda i: (i.level != "error", i.where[1], i.where[0])):
            print(f"  {i.level:7} {i.text}")
        failures += errs + (warns if strict else 0)
    return failures


def _rel(path: Path) -> str:
    try:
        return Path(path).resolve().relative_to(nl.LOGISIM_DIR).as_posix()
    except ValueError:
        return str(path)


def main(argv=None):
    ap = argparse.ArgumentParser(description="Check wiring and layout of Logisim .circ files.")
    ap.add_argument("files", nargs="+")
    ap.add_argument("-c", "--circuit", action="append", help="only this circuit (repeatable)")
    ap.add_argument("--strict", action="store_true", help="warnings also make the check fail")
    ap.add_argument("--no-libs", action="store_true", help="don't check the library files it loads")
    args = ap.parse_args(argv)
    bad = 0
    for f in args.files:
        bad += report(nl.resolve_file(f), args.circuit, args.strict, not args.no_libs)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
