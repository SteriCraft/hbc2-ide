"""Generate the vertical through-hole PCIe slot footprints (Amphenol FCI 10018783).

Only the x1 footprint was available (SamacSys, 10018783-10010TLF). The x4, x8 and
x16 are extrapolated from its pattern:
  - pin n at x = n - 1 (n <= 11), x = n + 1 (n >= 12, after the 3 mm key gap);
  - A odd y = 0, A even y = -2, B odd y = 2.5, B even y = 4.5;
  - holes 0.7 mm on 1.2 mm pads, A1 square;
  - two 2.35 mm non-plated peg holes at y = 1.25: x = 11.65 (key) and last pin + 1.8;
  - body from x = -2.85 to last pin + 3.15 (25 / 39 / 56 / 89 mm, the PCIe CEM
    housing lengths), y = -2.45 to 4.95.
The generated x1 is checked against the SamacSys file when it is found.

Usage: python gen_pcie_slots.py   (writes hbc2_slots.pretty/ next to this script)
"""
import os
import re
import sys

PARTS = {1: ("10018783-10010TLF", 18), 4: ("10018783-11101TLF", 32),
         8: ("10018783-11112TLF", 49), 16: ("10018783-10013TLF", 82)}
SAMACSYS_X1 = os.path.expanduser(
    "~/Documents/KiCad/mouser_libraries/SamacSys_Parts.pretty/1001878310010TLF.kicad_mod")


def px(n):
    return n - 1 if n <= 11 else n + 1


def pads(npins):
    out = []
    for n in range(1, npins + 1):
        out.append((f"A{n}", px(n), 0.0 if n % 2 else -2.0))
        out.append((f"B{n}", px(n), 2.5 if n % 2 else 4.5))
    return out


def fmt(v):
    return f"{v:.3f}".rstrip("0").rstrip(".") if v != int(v) else str(int(v))


def footprint(lanes):
    part, npins = PARTS[lanes]
    last = px(npins)
    x0, x1 = -2.85, last + 3.15
    y0, y1 = -2.45, 4.95
    name = f"PCIe_x{lanes}_Slot_Amphenol_{part}"
    L = [f'(footprint "{name}"', '\t(version 20240108)', '\t(generator "gen_pcie_slots")',
         '\t(layer "F.Cu")',
         f'\t(descr "PCIe x{lanes} vertical through-hole slot, {2 * npins} contacts, 1.0 mm pitch, '
         f'Amphenol FCI {part}. ' + ('Pattern from the SamacSys footprint.' if lanes == 1 else
                                     'Extrapolated from the x1 (SamacSys) pattern: check against the drawing.') + '")',
         '\t(tags "PCIe slot connector card edge vertical THT")', '\t(attr through_hole)']

    def text(kind, val, x, y, layer, hide=False):
        L.append(f'\t(property "{kind}" "{val}" (at {fmt(x)} {fmt(y)} 0) (layer "{layer}")'
                 + (' (hide yes)' if hide else '')
                 + ' (effects (font (size 1 1) (thickness 0.15))))')

    text("Reference", "REF**", (x0 + x1) / 2, y0 - 1.6, "F.SilkS")
    text("Value", name, (x0 + x1) / 2, y1 + 1.6, "F.Fab")

    def rect(ax, ay, bx, by, layer, w):
        for (sx, sy, ex, ey) in ((ax, ay, bx, ay), (bx, ay, bx, by), (bx, by, ax, by), (ax, by, ax, ay)):
            L.append(f'\t(fp_line (start {fmt(sx)} {fmt(sy)}) (end {fmt(ex)} {fmt(ey)}) '
                     f'(stroke (width {w}) (type solid)) (layer "{layer}"))')

    rect(x0, y0, x1, y1, "F.Fab", 0.1)
    rect(x0 - 0.15, -2.8, x1 + 0.15, 5.3, "F.SilkS", 0.12)       # clear of the 1.2 mm pads
    rect(x0 - 1.0, -3.6, x1 + 1.0, 6.1, "F.CrtYd", 0.05)
    L.append('\t(fp_circle (center -3.45 0) (end -3.25 0) (stroke (width 0.12) (type solid)) '
             '(fill yes) (layer "F.SilkS"))')                     # pin A1 marker
    L.append(f'\t(fp_text user "${{REFERENCE}}" (at {fmt((x0 + x1) / 2)} 1.25 0) (layer "F.Fab") '
             '(effects (font (size 1 1) (thickness 0.15))))')
    for (num, x, y) in pads(npins):
        shape = "rect" if num == "A1" else "circle"
        L.append(f'\t(pad "{num}" thru_hole {shape} (at {fmt(x)} {fmt(y)}) (size 1.2 1.2) (drill 0.7) '
                 '(layers "*.Cu" "*.Mask"))')
    for x in (11.65, last + 1.8):
        L.append(f'\t(pad "" np_thru_hole circle (at {fmt(x)} 1.25) (size 2.35 2.35) (drill 2.35) '
                 '(layers "*.Cu" "*.Mask"))')
    L.append(')')
    return name, "\n".join(L) + "\n"


def check_x1():
    if not os.path.exists(SAMACSYS_X1):
        print("SamacSys x1 not found, check skipped")
        return
    ref = {}
    for m in re.finditer(r'\(pad (\S+) (thru_hole|np_thru_hole) \w+ \(at ([-\d.]+) ([-\d.]+)\) '
                         r'\(size ([\d.]+) [\d.]+\) \(drill ([\d.]+)\)', open(SAMACSYS_X1).read()):
        name, kind, x, y, size, drill = m.groups()
        ref[name] = (float(x), float(y), float(size), float(drill))
    mine = {n: (x, y, 1.2, 0.7) for (n, x, y) in pads(18)}
    mine["MH1"] = (11.65, 1.25, 2.35, 2.35)
    mine["MH2"] = (px(18) + 1.8, 1.25, 2.35, 2.35)
    bad = [k for k in ref if k not in mine or any(abs(a - b) > 1e-6 for a, b in zip(ref[k], mine[k]))]
    print(f"x1 vs SamacSys: {len(ref)} holes compared, {len(bad)} differ {bad}")
    if bad:
        sys.exit(1)


if __name__ == "__main__":
    check_x1()
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "hbc2_slots.pretty")
    os.makedirs(out, exist_ok=True)
    for lanes in PARTS:
        name, text = footprint(lanes)
        open(os.path.join(out, name + ".kicad_mod"), "w", newline="\n").write(text)
        print("wrote", name)
