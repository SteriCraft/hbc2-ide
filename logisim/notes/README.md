# Logisim notes

These notes are short, one subject per file, so I only open the one I need.

| File | Subject |
|---|---|
| [conventions.md](conventions.md) | HBC-2 design rules: components, names, chips, layout, files, checks |
| [tools.md](tools.md) | The scripts in `logisim/tools/` and how to use them |
| [ttl-chips.md](ttl-chips.md) | Pinout quirks of the TTL parts, and how each one is wired |
| [open-points.md](open-points.md) | Rule conflicts and questions still waiting for an answer |

Modules done so far:

| Module | File | Chips | Test |
|---|---|---|---|
| `add8` | circuits/alu/add8.circ, made by designs/add8.py | 2 × 74HCT283 | 131,072 / 131,072 vs emulator |
| `alu` (add only, for now) | circuits/alu/alu.circ, made by designs/alu.py; uses add8 (library `file#add8.circ`) | none of its own | 131,072 / 131,072 vs emulator |

`logisim/cpu.circ` holds only an empty `main` and loads alu and add8 as libraries.

History: on 2026-11-04 the user deleted their test circuits add_sub8.circ and the old alu.circ. `hbc2 vectors add_sub8` still exists on the emulator side.
