# Logisim notes

These notes are short, one subject per file, so I only open the one I need.

| File | Subject |
|---|---|
| [conventions.md](conventions.md) | HBC-2 design rules: components, names, chips, layout, files, checks |
| [tools.md](tools.md) | The scripts in `logisim/tools/` and how to use them |
| [ttl-chips.md](ttl-chips.md) | Pinout quirks of the TTL parts, and how each one is wired |
| [open-points.md](open-points.md) | Rule conflicts and questions still waiting for an answer |
| [chips.md](chips.md) | Which 74HCT parts to buy: availability, packages, delays, parts missing from Logisim |
| [alu-decisions.md](alu-decisions.md) | ALU design decisions (night of 2026-11-04) and points for Gianni to review |
| [alu-report.md](alu-report.md) | ALU summary: alu_op table, modules and chips, delays, test results, what's left |

Modules done so far (all in circuits/alu/, each made by designs/<module>.py):

| Module | Chips | Test (vs emulator) |
|---|---|---|
| `add_sub8` | 2 × 74HCT283, 3 × 74HCT86, 1 × 74HCT08 | 524,288 / 524,288 |
| `arith8` (ADD ADC SUB SBB INC DEC) | add_sub8 + 2 × 74HCT157 | 1,048,576 / 1,048,576 |
| `logic8` (AND OR XOR) | 2 × 74HCT08, 2 × 74HCT32, 2 × 74HCT86 | 65,536 / 65,536 |
| `shift8` (shifts, C select) | 1 × 74HCT153 | 2,048 / 2,048 |
| `flags8` (Z N S E F) | 74HCT27, 74HCT11, 74HCT02 | 512 / 512 |
| `rmux8` (result bus, NOT) | 6 × 74HCT541, 74HCT240, 74HCT138, 74HCT11 | 65,760 sampled |
| `alu` (all of the above) | none of its own; 27 chips in total | see alu-report.md |

`logisim/cpu.circ` holds only an empty `main` and loads alu and add_sub8 as libraries.

History: on 2026-11-04 the user deleted their test circuits add_sub8.circ and the old alu.circ. `hbc2 vectors add_sub8` still exists on the emulator side.

On 2026-11-04 add8 became add_sub8: B goes through XOR chips (A - B = A + not B + 1), and the flag c is a borrow after SUB/SBB, as in the emulator.
