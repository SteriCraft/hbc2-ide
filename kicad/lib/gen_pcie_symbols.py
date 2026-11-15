"""Add (or replace) the HBC2_PCIe_x1/x4/x8/x16 symbols in kicad/custom_symbols.kicad_sym.

Plain connectors for the card slots (kicad/notes/slot-pinout.md): pins numbered A1..An and
B1..Bn like the slot footprint (lib/hbc2_slots.pretty) and the card-edge footprint
(Connector_PCBEdge:BUS_PCIexpress_xN), every pin passive, pin names hidden.
B pins on the left, A pins on the right, Bn and An on the same row, pin 1 at the
top, one empty row for the key (between pins 11 and 12).
Default footprint: the motherboard slot. On a card, change it to the card edge.

Usage: python gen_pcie_symbols.py
"""
import os
import re

LIB = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "custom_symbols.kicad_sym")
SIZES = {1: ("10018783-10010TLF", 18), 4: ("10018783-11101TLF", 32),
         8: ("10018783-11112TLF", 49), 16: ("10018783-10013TLF", 82)}
G = 2.54


def f(v):
    v = round(v, 4)
    return str(int(v)) if v == int(v) else f"{v:g}"


def prop(name, value, x, y, hide):
    return (f'\t\t(property "{name}" "{value}"\n\t\t\t(at {f(x)} {f(y)} 0)\n\t\t\t(show_name no)\n'
            f'\t\t\t(do_not_autoplace no)\n' + ('\t\t\t(hide yes)\n' if hide else '') +
            '\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n'
            + ('\t\t\t\t(justify left)\n' if name in ("Reference", "Value") else '') + '\t\t\t)\n\t\t)\n')


def pin(num, x, y, angle):
    font = '\t\t\t\t\t(effects\n\t\t\t\t\t\t(font\n\t\t\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t\t\t)\n\t\t\t\t\t)\n'
    return (f'\t\t\t(pin passive line\n\t\t\t\t(at {f(x)} {f(y)} {angle})\n\t\t\t\t(length {f(2 * G)})\n'
            f'\t\t\t\t(name "{num}"\n{font}\t\t\t\t)\n\t\t\t\t(number "{num}"\n{font}\t\t\t\t)\n\t\t\t)\n')


def row_y(n):
    return -(n - 1) * G if n <= 11 else -n * G


def symbol(lanes):
    part, npins = SIZES[lanes]
    name = f"HBC2_PCIe_x{lanes}"
    top, bottom = G, row_y(npins) - G
    fp = f"hbc2_slots:PCIe_x{lanes}_Slot_Amphenol_{part}"
    s = f'\t(symbol "{name}"\n\t\t(pin_names\n\t\t\t(offset 1.016)\n\t\t\t(hide yes)\n\t\t)\n'
    s += ('\t\t(exclude_from_sim no)\n\t\t(in_bom yes)\n\t\t(on_board yes)\n\t\t(in_pos_files yes)\n'
          '\t\t(duplicate_pin_numbers_are_jumpers no)\n')
    s += prop("Reference", "J", -5.08, top + 1.27, False)
    s += prop("Value", name, -5.08, bottom - 1.27, False)
    s += prop("Footprint", fp, 0, 0, True)
    s += prop("Datasheet", "", 0, 0, True)
    s += prop("Description",
              f"HBC-2 card slot, PCIe x{lanes} form ({2 * npins} contacts), used as a plain connector. "
              f"Motherboard: slot {fp}. Card: Connector_PCBEdge:BUS_PCIexpress_x{lanes}. "
              "Pinout in kicad/slot-pinout.md", 0, 0, True)
    s += prop("ki_keywords", "HBC2 PCIe slot card edge connector", 0, 0, True)
    s += prop("ki_fp_filters", f"PCIe_x{lanes}_Slot* BUS_PCIexpress_x{lanes}", 0, 0, True)
    s += (f'\t\t(symbol "{name}_0_1"\n\t\t\t(rectangle\n\t\t\t\t(start -5.08 {f(top)})\n'
          f'\t\t\t\t(end 5.08 {f(bottom)})\n\t\t\t\t(stroke\n\t\t\t\t\t(width 0.254)\n'
          '\t\t\t\t\t(type default)\n\t\t\t\t)\n\t\t\t\t(fill\n\t\t\t\t\t(type background)\n\t\t\t\t)\n\t\t\t)\n')
    for txt, x, y in (("B", -2.54, top - 1.27), ("A", 2.54, top - 1.27), ("key", 0, row_y(11) - G)):
        s += (f'\t\t\t(text "{txt}"\n\t\t\t\t(at {f(x)} {f(y)} 0)\n\t\t\t\t(effects\n\t\t\t\t\t(font\n'
              '\t\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t\t)\n\t\t\t\t)\n\t\t\t)\n')
    s += '\t\t)\n'
    s += f'\t\t(symbol "{name}_1_1"\n'
    for n in range(1, npins + 1):
        s += pin(f"B{n}", -10.16, row_y(n), 0)
        s += pin(f"A{n}", 10.16, row_y(n), 180)
    s += '\t\t)\n\t\t(embedded_fonts no)\n\t)\n'
    return name, s


def remove(text, name):
    m = re.search(r'\n\t\(symbol "' + re.escape(name) + r'"\n', text)
    if not m:
        return text
    i, depth = m.start() + 1, 0
    for j in range(i, len(text)):
        if text[j] == '(':
            depth += 1
        elif text[j] == ')':
            depth -= 1
            if depth == 0:
                return text[:i - 1] + text[j + 1:]
    raise ValueError(name)


if __name__ == "__main__":
    crlf = b"\r\n" in open(LIB, "rb").read(4096)
    text = open(LIB, encoding="utf8").read()
    new = ""
    for lanes in SIZES:
        name, s = symbol(lanes)
        text = remove(text, name)
        new += s
    end = text.rstrip().rfind(")")
    text = text[:end].rstrip("\n") + "\n" + new + ")\n"
    open(LIB, "w", encoding="utf8", newline="\r\n" if crlf else "\n").write(text)
    print("written:", ", ".join(f"HBC2_PCIe_x{n}" for n in SIZES))
