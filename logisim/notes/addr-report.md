# Address section: report (phase 2, night of 2026-11-04)

PC, SP, the 16-bit address adder and the address latch are built and tested: module `addr` in [circuits/addr/addr.circ](../circuits/addr/addr.circ), made by [designs/addr.py](../designs/addr.py). Every timing limit is met at max 25 °C. Design and decisions: [addr-decisions.md](addr-decisions.md) (phase 2 decisions: section 4, A1–A9).

Commits: `7eee36f` (Logisim's 74377 checked, 377 delays), `f7ed5d1` (the module, tests, timing), then the notes; pushed after Gianni's review (2026-11-05).

## Chips

One module, 34 chips (phase 1 planned 32; +2 × 74HCT08 for the reset, A1). Bill of materials: [bom-addr.csv](bom-addr.csv); references in chips.md "Address".

| Qty | Part | Role |
|---|---|---|
| 8 | 74HCT153 | BASE multiplexer: PC / SP / R1:R2 / IVT, /E per byte gives 00 |
| 4 | 74HCT157 | OFFSET: high byte V1 or 00, low byte V2 / V1 or 00 |
| 4 | 74HCT86 | offset inversion (subtraction, −1) |
| 4 | 74HCT283 | 16-bit ripple adder |
| 6 | 74HCT377 | address latch, PC, SP (2 each) |
| 6 | 74HCT541 | latch → address bus (2), PC → data bus (2), SP → data bus (2) |
| 2 | 74HCT08 | reset: PC bits 15..9 and the IVT's p7 AND `rst_n` |

New part: **74HCT377** (CD74HCT377E DIP, CD74HCT377M96 SOIC; TI SCHS184C, datasheet in circuits/chips/). Logisim's 74377 was checked against it (250 / 250 rows; /E is sampled at the edge, no gated clock).

## Reset (Gianni's requirement)

PC = 0000 and SP = FE00 come from `rst_n`, not from a sequence: while `rst_n` is low the control unit holds a reset word (BASE = the IVT's high byte, so the sum is FE00 → SP), and the two 74HCT08 force PC's bits 15..9 to 0 (PC = 0000 from the same sum) and the IVT's p7 to 0 (so the bus doesn't matter). Cost: 2 chips. Why this one: addr-decisions.md A1 compares five options; the self-contained version (no reliance on the control unit's reset word) costs 5–6 chips.

## Timing

`python tools/timing.py --section addr`; the 377's real figures (CP → Q 38 / 48 ns) replace the 574 placeholders. The criterion is max 25 °C; 85 °C is for information.

| Path | Max 25 °C | Max 85 °C | Limit | Margin 25 / 85 °C |
|---|---|---|---|---|
| control (OFFSET select) → adder → PC setup | 323 | 404 | 500 | 177 / 96 |
| V1 / V2 → adder → PC setup | 319 | 399 | 500 | 181 / 101 |
| PC / SP → adder → PC setup (PC + 1, SP − 1) | 301 | 378 | 500 | 199 / 122 |
| control (BASE select) → adder → PC setup | 296 | 371 | 500 | 204 / 129 |
| **regs_a / regs_b → adder → PC setup** (Reg16, mode 4, IN / OUT) | **416** | **525** | 500 | **84 / −25** |
| bus_addr (IOD port) → IVT → adder → PC setup | 263 | 330 | 500 | the IOD must drive the bus by 237 / 170 ns |
| clk → latch → bus_addr valid | 71 | 90 | 335 / 325 | 264 / 235 (left for memory) |
| PC / SP → bus_data | 96 | 121 | 335 / 325 | 239 / 204 |

Pass at 25 °C: all limits met. At 85 °C one path is over: the register-operand path, by 25 ns (the register section's end-to-end path is also over at 85 °C, by 20 ns). The ALU, flag and register sections still pass with the new loads; the data bus is now counted at 372 pF instead of about 420 (PC and SP add no inputs to it).

## Tests

| Test | Rows | Result |
|---|---|---|
| `hbc2 vectors addr` (model: all 512 adder input combinations, every enable, the reset, 40 random sequences) | 31,640 | 31,640 passed |
| `hbc2 vectors addr_cpu` (the real CPU: 48 PC / SP pairs incl. wrap corners × 40 events: every memory mode, stack, CAL / RET / IRT, jumps, SP instructions, IN / OUT, INT + software entry, hardware entry, invalid instruction; each ends with a reset) | 65,114 | 65,114 passed |
| `test_74377` (Logisim's chip vs the datasheet) | 250 | 250 passed |
| Mutation check: p7 gate removed / a carry broken | | caught by both tests |

`circcheck` and `conventions.py`: 0 errors, 0 warnings. Run everything with `python designs/addr.py --test`.

## Requirements

- pc-sp-requirements.md P1–P4: met (status written there).
- control-requirements.md: C11–C15 copied from phase 1, **C16 new** (the reset word, a synchronous release of `rst_n`). C10's bus load updated (372 pF).

## Need Gianni's review

**Reviewed 2026-11-05: all 8 accepted as recommended** (addr-decisions.md section 5). Item 2 became control-requirements.md C17 (reset generator: RC + 74HCT14 + 74HCT74); item 4 is to check on the breadboard with a scope (breadboard-checks.md B1).

1. **Reset relies on the control unit's reset word** (C16). The alternative that keeps everything inside `addr` costs 3–4 more chips (A1). Which do you want?
2. **`rst_n` must be released synchronously to `clk`** (a flip-flop in the reset generator), and must cover at least one `clk` edge. The flag register's asynchronous /MR doesn't care, but this section does.
3. **Register-operand path at 85 °C: −25 ns** (416 ns, +84 margin at 25 °C). Within the criterion; the fallback is 74ACT153 for the 8 BASE multiplexers (about −27 ns), as for the register section.
4. **377 hold times** (/E 5 ns, D 3 ns) vs the control register's minimum CP → Q, which TI doesn't specify for the 574. Probably fine (typ 15 ns), but unproven on paper.
5. **The address bus is two 8-bit pins** (`addr_bus_addr_hi`, `addr_bus_addr_lo`) plus `addr_bus_addr_lo_in`, not one 16-bit `bus_addr`: per-byte enables and Logisim vectors can't expect a half-floating 16-bit value (A2). OK with the naming?
6. **74HCT377 stock** not checked on Octopart (your survey predates it); TI lists the DIP and the SOIC T&R as active, the tube SOIC (CD74HCT377M) as obsolete.
7. **34 chips instead of 32** (the reset gates).
8. The step details the bench settled for the control unit (A8): the last push of an interrupt entry also loads latch := 0000; RET / IRT take one step for each byte into V1 / V2.

## What's left (other sections)

The control unit (it must meet C1–C16), the IR (C8, C14), the pending port latch (C13), memory, the IOD; then `cpu.circ` instantiating the sections.
