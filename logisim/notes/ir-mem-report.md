# Instruction register and memory: report (phase 2, night of 2026-11-05)

The instruction register, the memory section and a combined bench are built and tested. Every timing limit is met at max 25 °C.
- **Instruction register:** module `ir` in [circuits/ir/ir.circ](../circuits/ir/ir.circ), made by [designs/ir.py](../designs/ir.py).
- **Memory section:** module `mem` in [circuits/mem/mem.circ](../circuits/mem/mem.circ), made by [designs/mem.py](../designs/mem.py).
- **Combined bench:** [circuits/mem/test_mem_cpu.circ](../circuits/mem/test_mem_cpu.circ). It runs `addr`, `ir` and `mem` together against the real emulator.

Design, Gianni's answers and the decisions taken during the night (M1–M7): [ir-mem-decisions.md](ir-mem-decisions.md), sections 1–9. Nothing is pushed.

## Chips: 15 (phase 1 planned 14)

| Module | Qty | Part | Role |
|---|---|---|---|
| ir | 6 | 74HCT377 | byte 0, byte 1, V1, V2; X1 (not R1 / R2 bit 2), X2 (not R3 bit 2) |
| ir | 1 | 74HCT14 | NOT bus bits 5 and 2, before X1 / X2 (C8) |
| mem | 1 | AS6C1008-55PCN | 128 K × 8 SRAM, A16 tied low: 0000–FFFF |
| mem | 2 | AT28C256-15PU | boot ROM, 0000–7FFF and 8000–FFFF |
| mem | 3 | 74HCT541 | read buffer (local bus → data bus), write buffer (data bus → local bus), **ROM buffer (M4, new)** |
| mem | 1 | 74HCT32 | RAM WE# = `clk_mem_wr_n` OR `mem_we_n` |
| mem | 1 | 74HCT14 | NOT A15, the high ROM's /CE |

Bills of materials: [bom-ir.csv](bom-ir.csv), [bom-mem.csv](bom-mem.csv). The parts are listed in chips.md under "Instruction register" and "Memory". The new delays are 74HCT14 (TI SCLS225G: 20 / 32 / 40 ns, the phase 1 estimate), AS6C1008 and AT28C256.

## What changed since phase 1

- **M1, write pulse (your answer 5).**
  - The RAM's WE# now comes from the clock generator's spare /Q2 output (`clk_mem_wr_n`), low from about 125 to 375 ns.
  - Shortest pulse: **174 ns at 25 °C** against tWP 45, a margin of 129 (it was 4). At 85 °C: 155 ns, margin 110.
  - No new chip. The 670 writes keep `clk_wr_n`.
  - The new tight point is the address setup before WE# falls (see Timing).
- **M4, level buffer.** The AT28C256 only guarantees VOH 2.4 V, and the AS6C1008 v1.0 wants VIH 3.5 V. The boot ROMs therefore sit on their own net and reach the RAM through a third 74HCT541 (+1 chip).
- **`addr_cout` (your answer 2).** The last 74HCT283's carry out is now an output of `addr`, on the symbol's bottom edge. No file instantiates `addr` yet (`usages.py addr`), so nothing needs rewiring.
- **IR as a 377.** The register section assumed a 574. With the 377, the register read paths are 5 ns slower at 25 °C and 7 ns slower at 85 °C. They still pass at 25 °C.
- **New global signal `clk_mem_wr_n`.** It is in conventions.md and conventions.py, and control-requirements.md C6 now describes it.

## Chip models (circuits/chips/)

| Model | Inside | Test | Rows |
|---|---|---|---|
| `as6c1008` (designs/as6c1008.py) | #Memory RAM (17-bit, asynchronous level write), NOT / AND gates for the truth table, output buffer | `test_as6c1008`: writes with the data changing during WE#, writes blocked by CE# / CE2, every mode | 3,260 / 3,260 |
| `at28c256` (designs/at28c256.py) | #Memory ROM, 64 KiB image (M3), output buffer; read-only | `test_at28c256`: every address of both halves, every mode | 69,536 / 69,536 |

Both models have extra pins that don't exist on the real chip, because Logisim can't do one thing it needs:
- **AS6C1008, `DQ0_in` … `DQ7_in` (M2):** a Logisim subcircuit pin is one-way, so the RAM reads its data through these and drives DQ0–DQ7. In `mem`, both sets of pins are wired to the same net.
- **AT28C256, `HALF` (M3):** every instance of a model shares one ROM contents, so each chip picks its half of the image with this pin.

Tool changes that came with the models:
- netlist.py knows Logisim's RAM, ROM and 4-input AND gate, with pin offsets read from the Logisim-evolution 5.0.0 sources.
- circgen.py draws the extra pins and writes the ROM contents as multi-line attributes.
- conventions.py names non-74 parts correctly.

## Tests

| Test | What | Rows | Result |
|---|---|---|---|
| `hbc2 vectors ir` | random load sequences, several enables at once, every byte through every register; fields from the emulator's `Instruction` accessors | 32,400 | pass |
| `hbc2 vectors ir_cpu` | every opcode × every mode pattern on the real CPU, loading exactly `size()` bytes; stale bytes checked; RET / IRT / vector reloads of V1, V2 | 17,444 | pass |
| `hbc2 vectors addr`, `addr_cpu` | re-run with `addr_cout` checked | 31,640 + 65,114 | pass |
| `hbc2 vectors mem` | full 64 KiB boot copy, then random reads / writes / idle steps on the emulator's `Ram`, both buses checked on every row | 226,614 | pass |
| `hbc2 vectors mem_cpu` on `test_mem_cpu` | details below the table | 302,556 | pass |

What the combined bench (`test_mem_cpu`) does:
- **Boot:** a reset, then the boot loop (C20). The ROM holds the program's power-on RAM image, and the loop ends on `addr_cout`.
- **Run:** 4,000 ticks of the real CPU running a random program. The bench fetches every instruction from the Logisim RAM into the IR and mirrors every RAM write the CPU makes.
- **I/O and interrupts:** IN and OUT go through the IOD stub. The CPU takes 54 hardware interrupts: the pushes, the vector step, V1 and V2 read from the IVT, and PC := V1:V2 through the IR, checked against the handler address.

Every bus is checked on every row, so a second driver fails the test. A mutation check caught each deliberate fault:
- models: read logic, top ROM address bit;
- `ir`: the inverter's input;
- `mem`: both ROMs selected at once, which fails as contention;
- `test_mem_cpu`: V1 / V2 swapped into `addr`.

circcheck and conventions.py report 0 errors. The warnings are expected: NC and /WE are unconnected inside the models, and the bench has no module of its own name (M6). To run everything: `python designs/<as6c1008|at28c256|ir|addr|mem|test_mem_cpu>.py --test`.

## Timing (`python tools/timing.py --section mem`)

| Path | Max 25 °C | Max 85 °C | Limit (25 / 85) | Margin 25 / 85 |
|---|---|---|---|---|
| **RAM read → bus_data** (latch → bus_addr → tAA → read buffer) | **184** | 220 | 335 / 325 | 151 / 105 |
| RAM read, control → CE2 (tACE) | 145 | 169 | 335 / 325 | 190 / 156 |
| RAM read, control → read buffer on | 97 | 123 | 335 / 325 | 238 / 202 |
| fetch → IR setup (through the 14) | 228 | 275 | 500 | 272 / 225 |
| RAM write data (register → bus_data → write buffer) | 242 | 309 | 322 / 315 | 80 / 6 |
| **RAM WE# pulse, shortest** | **174** | 155 | ≥ 45 (tWP) | 129 / 110 |
| RAM WE# rises, latest | 451 | 470 | 500 | 49 / 30 |
| address before WE# falls | 72 | 92 | 97 / 90 | **25 / −2** |
| boot copy (A15 → 14 → ROM tCE → ROM buffer) | 281 | 316 | 322 / 315 | 41 / **−1** |

Bus loads, now counted on the built circuits: `bus_data` 386 pF, `bus_addr` 158 pF, `mem_d` 83 pF.

All sections pass at 25 °C (alu, flags, regs, addr, mem). The paths over at 85 °C, for information:
- register end to end: 417 ns, 27 ns over;
- register operand into the address adder: 421 ns, 32 ns over;
- address before WE# falls: 2 ns over;
- boot copy: 1 ns over.

## Requirements written

- **control-requirements.md:**
  - C6 now includes `clk_mem_wr_n`.
  - C18: IR loads and decode, ignoring the stale mode bits.
  - C19: the memory strobe combinations.
  - C20: the boot loop.
  - C21: INR for exactly one step, then the vector step.
  - C22: the I/O steps.
  - **C23: the control unit is microcoded** (EEPROMs indexed by opcode, mode and step, with a pipeline register; your answer 4).
- **[iod-requirements.md](iod-requirements.md), new:**
  - I1–I8: the synchronous contract, no WAIT line.
  - I7: a future microcontroller-based IOD must answer through hardware latches and a FIFO that it fills asynchronously (your answer 7).
- **breadboard-checks.md B2:** measure the RAM's WE# pulse and the address setup before WE# falls, at the RAM pins.

## Need Gianni's review

**Reviewed 2026-11-05:** answers and what was done in ir-mem-decisions.md section 10. Items 2 and 3 closed 2026-11-10: the current datasheets keep VIH 0.7 × VCC, so M4 stays, and no delay changed (ir-mem-decisions.md section 11).

1. **The models' simulation-only pins break the "datasheet pins only" rule.** I couldn't ask, so I took the only workable option.
   - **What:** `DQ0_in`–`DQ7_in` on the AS6C1008 (M2) and `HALF` on the AT28C256 (M3). They are drawn on the left edge of the DIP outline, without pin numbers.
   - **Why:** Logisim can't model a bidirectional subcircuit pin or per-instance ROM contents. The alternative was a bare #Memory RAM or ROM inside `mem`, which would need #Gates in a module, which is also against the rules.
   - **Your call:** accept these pins as a documented exception, or pick another way.
2. **+1 chip, the ROM buffer (M4).** It exists only because the AS6C1008 v1.0 datasheet asks for VIH 0.7 × VCC.
   - If the current Alliance datasheet gives VIH 2.2 V (as some revisions do), the ROMs could drive the RAM directly. That saves the chip, and the boot copy gains about 23 ns.
   - **Please check** which datasheet revision applies.
3. **Datasheet revisions.**
   - `as6c1008.pdf` is v1.0 (2007), a distributor copy; alliancememory.com didn't serve the current one.
   - `at28c256.pdf` is an Atmel copy (0006F); Microchip's DS20006386B wasn't downloadable.
   - Both should be replaced with the current revisions, and their figures rechecked.
4. **Address setup before WE# falls: 25 ns at 25 °C, −2 ns at 85 °C.**
   - On paper this assumes FF1 at its maximum delay and FF2 at zero, in the same 74HCT74 package.
   - B2 measures it on the breadboard.
   - If it is too tight, a 74ACT74 in the clock generator would gain about 15 ns.
5. **`clk_mem_wr_n` is a new global signal** from the clock generator (C6), so the generator now has three outputs.
6. **`addr_cout` changes the ports of an accepted module.** No instance exists yet.
7. **The AT28C256 model is read-only.** Programming it is the TL866's job, and /WE is tied high on the board. The model's write modes are documented but not modelled.
8. **Scope of the combined bench (M5–M7).**
   - The register section is a stand-in: bench pins drive bus A:B and the data bus.
   - The IOD stub's queue lives in the vector generator.
   - The program is straight-line: no jumps, CAL, RET or IRT, which `addr_cpu` covers; no STR or POF; no SP moves. PC is set from the CPU after each instruction.
   - INT is untested until the pending port latch (C13) is built.
9. **Boot, follow-ups outside Logisim.**
   - The EEPROM peripheral's power-on DMA has no hardware equivalent now: the boot ROM replaces it.
   - The assembler needs a boot ROM output (two 32 KiB halves). That isn't done: the emulator and assembler are unchanged.
10. **The 74HCT-only rule** doesn't apply to the memories (AS6C1008, AT28C256). They are listed in chips.md with that note.
11. **SOIC references not checked on Octopart:** AS6C1008-55SIN (industrial grade) and AT28C256-15SU.
12. **The 85 °C paths** listed under Timing are information only. The worse two are the register end-to-end path (−27) and the register operand into the adder (−32); the IR's 377 makes them 5–7 ns worse. The fallbacks are as before: 74ACT153 for the index and BASE multiplexers.

## What's left

- **The control unit:** microcoded, and it must meet C1–C23.
- **The pending port latch** (C13).
- **The IOD**, against iod-requirements.md.
- **`cpu.circ`**, instantiating all the sections.
- ~~Re-check M4 against the current datasheets~~: done 2026-11-10, M4 kept (ir-mem-decisions.md section 11).
