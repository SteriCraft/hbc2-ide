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
| `add_sub8` | circuits/alu/add_sub8.circ, made by designs/add_sub8.py | 2 × 74HCT283, 3 × 74HCT86, 1 × 74HCT08 | 524,288 / 524,288 vs emulator |
| `alu` (ADD/ADC/SUB/SBB, flags C N Z) | circuits/alu/alu.circ, made by designs/alu.py; uses add_sub8 (library `file#add_sub8.circ`) | 1 × 74HCT27, 1 × 74HCT11 | 524,288 / 524,288 vs emulator |

`logisim/cpu.circ` holds only an empty `main` and loads alu and add_sub8 as libraries.

History: on 2026-11-04 the user deleted their test circuits add_sub8.circ and the old alu.circ. `hbc2 vectors add_sub8` still exists on the emulator side.

On 2026-11-04 add8 became add_sub8: B goes through XOR chips (A - B = A + not B + 1), and the flag c is a borrow after SUB/SBB, as in the emulator.
