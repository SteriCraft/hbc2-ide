#!/usr/bin/env python3
"""Draw a Logisim-evolution circuit to PNG, to review a layout without Logisim.

The drawing is approximate (pin sizes, fonts and splitter shapes differ a
little from Logisim), but wires, ports and components are where Logisim puts
them. Thick lines are buses, dots are junctions. With --check, the problems
found by circcheck are circled in red (errors) or orange (warnings).

Usage:
    python render.py add8_ttl.circ                 # main circuit -> add8_ttl.png
    python render.py cpu.circ -c add_sub8 -o out.png --check
Needs Pillow.
"""

from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).resolve().parent))
import circcheck  # noqa: E402
import netlist as nl  # noqa: E402

WIRE = (0, 110, 0)
BUS = (0, 60, 0)
GRID = (226, 226, 226)


def _font(size):
    for name in ("arial.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def render(circ: nl.Circuit, out: Path, scale: int = 2, check: bool = False) -> Path:
    circ.build()
    K = scale
    uf = nl.UnionFind()
    for a, b in circ.wires:
        uf.union(a, b)
    width = {}
    for c in circ.comps:
        for p in c.ports:
            r = uf.find(p.loc)
            width[r] = max(width.get(r, 1), p.width or 1)
    texts = [(nl.parse_loc(e.get("loc")), nl.read_attrs(e).get("text", ""))
             for e in circ.elem.findall("comp") if e.get("name") == "Text"]

    pts = [p for w in circ.wires for p in w] + [c.loc for c in circ.comps] + [t[0] for t in texts]
    pts += [p.loc for c in circ.comps for p in c.ports]
    if not pts:
        raise SystemExit(f"circuit {circ.name} is empty")
    x0, y0 = min(p[0] for p in pts) - 140, min(p[1] for p in pts) - 60
    x1, y1 = max(p[0] for p in pts) + 160, max(p[1] for p in pts) + 60
    img = Image.new("RGB", ((x1 - x0) * K, (y1 - y0) * K), "white")
    d = ImageDraw.Draw(img)
    font, small = _font(11 * K), _font(8 * K)

    def P(x, y):
        return ((x - x0) * K, (y - y0) * K)

    for gx in range(x0 - x0 % 10, x1, 10):
        for gy in range(y0 - y0 % 10, y1, 10):
            d.point(P(gx, gy), fill=GRID)

    for c in circ.comps:
        x, y = c.loc
        if c.kind == "ttl":
            bx1, by1, bx2, by2 = circcheck.ttl_body(c)
            d.rectangle([P(bx1, by1), P(bx2, by2)], outline="black", width=K)
            d.text(P((bx1 + bx2) / 2, (by1 + by2) / 2), c.name, fill="black", font=font, anchor="mm")
            if c.label:
                d.text(P(bx1 - 5, (by1 + by2) / 2), c.label, fill="blue", font=font, anchor="rm")
            for p in c.ports:
                px, py = p.loc
                ex = min(max(px, bx1), bx2)
                ey = min(max(py, by1), by2)
                d.line([P(px, py), P(ex, ey)], fill="black", width=K)
                lx, ly = ex + (6 if ex == bx1 else -6 if ex == bx2 else 0), ey + (6 if ey == by1 else -6 if ey == by2 else 0)
                d.text(P(lx, ly), p.name, fill="gray", font=small, anchor="mm")
        elif c.kind == "pin":
            out_ = c.attrs.get("type") == "output"
            bits = nl.parse_int(c.attrs.get("width"), 1)
            bw = max(20, 10 * bits)
            face = c.attrs.get("facing", "west" if out_ else "east")
            # the body sits on the side opposite to `facing`
            box = {"east": (x - bw, y - 10, x, y + 10), "west": (x, y - 10, x + bw, y + 10),
                   "south": (x - bw // 2, y - 20, x + bw // 2, y), "north": (x - bw // 2, y, x + bw // 2, y + 20)}[face]
            r = [P(box[0], box[1]), P(box[2], box[3])]
            if out_:
                d.rounded_rectangle(r, radius=8 * K, outline="black", width=K)
            else:
                d.rectangle(r, outline="black", width=K)
            lab = {"east": (P(box[0] - 4, y), "rm"), "west": (P(box[2] + 4, y), "lm"),
                   "south": (P(x, box[1] - 4), "mb"), "north": (P(x, box[3] + 4), "mt")}[face]
            d.text(lab[0], c.label, fill="black", font=font, anchor=lab[1])
        elif c.kind == "splitter":
            ends = c.ports[1:]
            if all(e.loc[0] == ends[0].loc[0] for e in ends):      # ends in a column
                sx = (x + ends[0].loc[0]) // 2
                ey = [e.loc[1] for e in ends]
                near = min(max(y, min(ey)), max(ey))
                spine = [P(sx, min(ey)), P(sx, max(ey))]
                stem = [P(x, y), P(sx, near)]
                ticks = [(P(sx, e.loc[1]), e) for e in ends]
            else:                                                   # ends in a row
                sy = (y + ends[0].loc[1]) // 2
                ex = [e.loc[0] for e in ends]
                near = min(max(x, min(ex)), max(ex))
                spine = [P(min(ex), sy), P(max(ex), sy)]
                stem = [P(x, y), P(near, sy)]
                ticks = [(P(e.loc[0], sy), e) for e in ends]
            d.line(stem, fill="black", width=3 * K)
            d.line(spine, fill="black", width=3 * K)
            for i, (a, e) in enumerate(ticks):
                d.line([a, P(*e.loc)], fill="black", width=K)
                bits = [b for b, k in enumerate(c.bit_end) if k == i]
                txt = f"{bits[0]}" if len(bits) == 1 else f"{bits[-1]}-{bits[0]}" if bits else ""
                d.text((P(*e.loc)[0] - 3 * K, P(*e.loc)[1] - 2 * K), txt, fill=(0, 120, 0), font=small, anchor="rb")
        elif c.kind == "tunnel":
            tb = d.textbbox(P(x, y), c.label, font=font, anchor="mm")
            d.rectangle([tb[0] - 4 * K, tb[1] - 2 * K, tb[2] + 4 * K, tb[3] + 2 * K], outline="black", width=K)
            d.text(P(x, y), c.label, fill="black", font=font, anchor="mm")
        elif c.kind in ("const", "power", "ground", "io"):
            d.text(P(x, y - 8), c.name, fill="gray", font=small, anchor="mb")
            d.ellipse([P(x - 3, y - 3), P(x + 3, y + 3)], outline="black", width=K)
        elif c.kind == "sub" and c.ports:
            xs = [p.loc[0] for p in c.ports]
            ys = [p.loc[1] for p in c.ports]
            d.rectangle([P(min(xs), min(ys) - 10), P(max(xs), max(ys) + 10)], outline="black", width=K)
            d.text(P((min(xs) + max(xs)) / 2, max(ys) + 4), c.name, fill="black", font=font, anchor="mt")
            for p in c.ports:
                d.text(P(p.loc[0] + (4 if p.loc[0] == min(xs) else -4), p.loc[1]), p.name, fill="gray",
                       font=small, anchor="lm" if p.loc[0] == min(xs) else "rm")
        else:   # unknown geometry: mark the anchor
            d.rectangle([P(x - 15, y - 15), P(x + 15, y + 15)], outline=(150, 0, 150), width=K)
            d.text(P(x, y), c.name, fill=(150, 0, 150), font=small, anchor="mm")

    for (tx, ty), t in texts:
        d.text(P(tx, ty), t, fill="black", font=font, anchor="mm")

    ends = Counter()
    for a, b in circ.wires:
        bus = width.get(uf.find(a), 1) > 1
        d.line([P(*a), P(*b)], fill=BUS if bus else WIRE, width=3 * K if bus else K)
        ends[a] += 1
        ends[b] += 1
    ports = Counter(p.loc for c in circ.comps for p in c.ports)
    for pt, n in ends.items():
        if n + ports.get(pt, 0) >= 3:
            cx, cy = P(*pt)
            r = 4 * K if width.get(uf.find(pt), 1) > 1 else 3 * K
            d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=BUS)

    if check:
        issues, _ = circcheck.check_circuit(circ)
        for i in issues:
            cx, cy = P(*i.where)
            col = (220, 0, 0) if i.level == "error" else (240, 140, 0)
            d.ellipse([cx - 9 * K, cy - 9 * K, cx + 9 * K, cy + 9 * K], outline=col, width=2 * K)

    img.save(out)
    return out


def main(argv=None):
    ap = argparse.ArgumentParser(description="Draw a Logisim circuit to PNG.")
    ap.add_argument("file")
    ap.add_argument("-c", "--circuit", help="circuit name (default: the file's main circuit)")
    ap.add_argument("-o", "--output", help="PNG path (default: <file>[-<circuit>].png next to the file)")
    ap.add_argument("-s", "--scale", type=int, default=2)
    ap.add_argument("--check", action="store_true", help="circle the problems found by circcheck")
    args = ap.parse_args(argv)
    path = nl.resolve_file(args.file)
    proj = nl.Project(path)
    name = args.circuit or proj.main or next(iter(proj.circuits))
    if name not in proj.circuits:
        raise SystemExit(f"error: no circuit {name!r}; circuits: {', '.join(proj.circuits)}")
    out = Path(args.output) if args.output else path.with_name(
        path.stem + ("" if name == proj.main else f"-{name}") + ".png")
    print(render(proj.circuits[name], out, args.scale, args.check))


if __name__ == "__main__":
    main()
