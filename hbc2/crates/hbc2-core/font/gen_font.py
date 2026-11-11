"""Builds src/peripherals/font.rs: the native display's default 8 x 8 font,
Windows-1252 (Peripherals/Native/Reference.md, "Characters").

    python gen_font.py

Glyphs:
  0x20-0x7E   font8x8_basic.h       (Daniel Hepper, public domain)
  0xA0-0xFF   font8x8_ext_latin.h   (same)
  0x80-0x9F   drawn below (the Windows-1252 additions)
  0x00-0x1F, 0x7F and the five codes Windows-1252 leaves undefined: blank

font8x8 puts the leftmost pixel in bit 0; font.rs puts it in bit 7, as
DEFCHAR does.
"""

import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "src", "peripherals", "font.rs")


def load(name):
    """The glyphs of a font8x8 header, in array order (its comments repeat
    some code points, so they are not used)."""
    text = open(os.path.join(HERE, name), encoding="utf-8").read()
    body = text[text.index("= {") + 3:]
    rows = re.findall(r"\{\s*(0x[0-9A-Fa-f, x]*?)\s*\}", body)
    return [[int(v, 16) for v in r.split(",")] for r in rows]


def mirror(byte):
    return int(f"{byte:08b}"[::-1], 2)


def art(rows):
    """8 strings of '#' and '.', leftmost pixel first -> 8 bytes, bit 7 left."""
    rows = list(rows) + ["........"] * (8 - len(rows))
    assert len(rows) == 8, rows
    out = []
    for r in rows:
        assert len(r) == 8, r
        out.append(int(r.replace("#", "1").replace(".", "0"), 2))
    return out


def as_art(glyph):
    return [f"{b:08b}".replace("1", "#").replace("0", ".") for b in glyph]


basic = [[mirror(b) for b in g] for g in load("font8x8_basic.h")]
latin = [[mirror(b) for b in g] for g in load("font8x8_ext_latin.h")]
assert len(basic) == 128 and len(latin) == 96, (len(basic), len(latin))

font = [[0] * 8 for _ in range(256)]
for code in range(0x20, 0x7F):
    font[code] = basic[code]
for code in range(0xA0, 0x100):
    font[code] = latin[code - 0xA0]
font[0xAD] = basic[ord("-")]        # soft hyphen: blank in font8x8, shown as '-'


def letter(base, rows):
    """Some rows of a basic letter, to make room for an accent above it."""
    return [as_art(basic[ord(base)])[r] for r in rows]


CARON = [".#..#...", "..##...."]

cp1252 = {
    0x80: ["..####..", ".##..##.", "#####...", ".##.....", "#####...", ".##..##.", "..####.."],   # €
    0x82: ["........"] * 5 + ["..##....", "..##....", ".##....."],                            # ‚
    0x83: ["....###.", "...##...", ".#####..", "...##...", "...##...", "..##....", "###....."],   # ƒ
    0x84: ["........"] * 5 + [".##.##..", ".##.##..", "##.##..."],                            # „
    0x85: ["........"] * 5 + ["##.##.##", "##.##.##"],                                        # …
    0x86: ["..##....", "..##....", "######..", "..##....", "..##....", "..##....", "..##...."],   # †
    0x87: ["..##....", "######..", "..##....", "..##....", "..##....", "######..", "..##...."],   # ‡
    0x88: ["..##....", ".####...", "##..##.."],                                               # ˆ
    0x89: ["##...#..", "##..#...", "...#....", "..#.....", ".#......", "#..##.##", "...##.##"],   # ‰
    0x8A: CARON + letter("S", [0, 2, 3, 4, 6]),                                               # Š
    0x8B: ["........", "...##...", "..##....", ".##.....", "..##....", "...##..."],             # ‹
    0x8C: [".#######", "#..#....", "#..#....", "#..####.", "#..#....", "#..#....", ".#######"],   # Œ
    0x8E: CARON + letter("Z", [0, 2, 3, 4, 6]),                                               # Ž
    0x91: ["..##....", ".##.....", ".##....."],                                               # ‘
    0x92: [".##.....", ".##.....", "##......"],                                               # ’
    0x93: ["..##.##.", ".##.##..", ".##.##.."],                                               # “
    0x94: [".##.##..", ".##.##..", "##.##..."],                                               # ”
    0x95: ["........", "........", "..##....", ".####...", ".####...", "..##...."],             # •
    0x96: ["........", "........", "........", "#######."],                                   # –
    0x97: ["........", "........", "........", "########"],                                   # —
    0x98: [".###.##.", "##.###.."],                                                           # ˜
    0x99: ["###.#..#", ".#..####", ".#..#..#", ".#..#..#"],                                   # ™
    0x9A: CARON + letter("s", [2, 3, 4, 5, 6]),                                               # š
    0x9B: ["........", ".##.....", "..##....", "...##...", "..##....", ".##....."],             # ›
    0x9C: ["........", "........", ".##..##.", "#..##..#", "#..#####", "#..##...", ".##..###"],   # œ
    0x9E: CARON + letter("z", [2, 3, 4, 5, 6]),                                               # ž
    0x9F: ["##..##..", "........"] + letter("Y", [0, 3, 4, 5, 6]),                             # Ÿ
}
for code, rows in cp1252.items():
    font[code] = art(rows)

lines = [
    "//! The native display's default 8 × 8 font, Windows-1252: 256 glyphs of",
    "//! 8 rows, top row first, bit 7 = leftmost pixel (the order DEFCHAR uses).",
    "//!",
    "//! Generated by `font/gen_font.py` from Daniel Hepper's public domain",
    "//! font8x8 (`font8x8_basic.h`, `font8x8_ext_latin.h`), plus the",
    "//! Windows-1252 glyphs 0x80-0x9F drawn for the HBC-2. Don't edit by hand.",
    "",
    "pub static DEFAULT_FONT: [[u8; 8]; 256] = [",
]
for code, glyph in enumerate(font):
    bytes_ = ", ".join(f"0x{b:02X}" for b in glyph)
    lines.append(f"    [{bytes_}], // 0x{code:02X}")
lines.append("];")
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(lines) + "\n")
print("wrote", os.path.normpath(OUT))

if __name__ == "__main__" and os.environ.get("SHOW"):
    for code in [int(c, 16) for c in os.environ["SHOW"].split(",")]:
        print(hex(code))
        print("\n".join(as_art(font[code])))
