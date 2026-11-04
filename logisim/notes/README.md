# Logisim notes

These notes are short, one subject per file, so I only open the one I need.

| File | Subject |
|---|---|
| [conventions.md](conventions.md) | HBC-2 design rules: components, names, chips, layout, checks |
| [tools.md](tools.md) | The scripts in `logisim/tools/` and how to use them |
| [ttl-chips.md](ttl-chips.md) | Pinout quirks of the TTL parts, and how each one is wired |
| [open-points.md](open-points.md) | Rule conflicts and questions still waiting for an answer |

Modules done so far:

| Module | File | Chips | Test |
|---|---|---|---|
| `add8` (practice) | circuits/add8_ttl.circ, made by designs/add8_ttl.py | 2 × 74HCT283 | 131,072 / 131,072 vs emulator |
| `add_sub8` (the user's) | circuits/cpu.circ | 1 × 08, 2 × 283, 3 × 86 (U1..U6) | 524,288 / 524,288 vs emulator |
