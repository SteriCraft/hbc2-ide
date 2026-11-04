# ALU: decisions (unattended run of 2026-11-04)

Gianni asked me to finish the ALU overnight and to write down every choice I made without asking. Each entry gives the question, the options, the choice and the reason. The list of points to review is at the end. The summary is in [alu-report.md](alu-report.md).

## What the emulator does (read from hbc2-core/src/cpu.rs)

| Instr. | Result | Flags written |
|---|---|---|
| ADD / ADC | a + b (+ C) | C = carry, Z, N |
| SUB / SBB | a - b (- C) | C = borrow, Z, N |
| INC / DEC | a + 1 / a - 1 (`overflowing_add/sub`) | C = carry (a = FF) / borrow (a = 00), Z, N |
| AND / OR / XOR | a op b | Z, N |
| NOT | not a | Z, N |
| SHL | a << 1 | C = bit 7 of a, Z, N |
| ASR | arithmetic: (a >> 1) with bit 7 kept | C = bit 0 of a, Z, N |
| SHR | **logical**: a >> 1, bit 7 = 0 | Z, N (C untouched) |
| CMP | none | S = a > b, E = a = b, F = a < b, **unsigned** |

Z = result is 0, N = bit 7 of the result, for every instruction that writes them.

## D1. Where the expected values come from

Question: the ALU instructions other than ADD/SUB are written inline in `cpu.rs`, so `hbc2_core::alu` has no function for them. The conventions say to move such code into a pure function first, but tonight's rule is "never change the emulator".
Options: (a) copy the emulator's expressions into the vector generator; (b) move the code into `alu.rs`, which changes the emulator; (c) run the real CPU on each instruction.
Choice: **(c)**. `hbc2 vectors` (hbc2-cli/src/vectors.rs, test tooling, not the emulator) now loads one instruction in Reg mode, sets A, B and the carry, calls `Cpu::tick`, and reads the register and the flags back. It also checks that the flags missing from the instruction table (`Opcode::info().flags`) are left untouched. Those flags are written `x` (don't care) in the vectors, because the control unit won't store them.
Reason: the reference really is the emulator's code, with nothing copied and nothing changed.

## D2. Exhaustive limit raised to 2^21, chunked runs

Question: the full ALU has 4 + 8 + 8 + 1 = 21 input bits, and the old limit was 2^20.
Choice: `MAX_BITS = 21`. `logitest.check_emulator` now splits a vector file into chunks of 524,288 rows, the size known to work in Logisim.
Reason: the prompt asks for an exhaustive test of every alu_op, A, B and C.

## D3. logic8 contains AND, OR and XOR, but not NOT

Options for NOT A: (a) 2 × 74HCT04, i.e. 8 inverters; (b) a XOR with FF, which needs B forced to FF, so 2 more chips; (c) an inverting tri-state buffer, the 74HCT240, as NOT's source on the result bus.
Choice: **(c)**. The result multiplexer needs one buffer per source anyway, and the 240 is that buffer and the inverter at once, so NOT costs no extra chip.

## D4. Layout of logic8

Three columns (AND, OR, XOR), each with two chips stacked, in the add_sub8 style: nested rails, vertical lines with 1-bit taps. a and b reach each column through tunnels. Input pins sit at x = 220 so their labels show; Gianni had moved add_sub8 and alu for that same reason. circgen now has `Circuit.move(dx, dy)`, and add_sub8.py and alu.py reproduce his shifted layouts.

## Need Gianni's review

1. **SHR**: the prompt says "like ASR, but the outgoing bit is discarded". In the emulator SHR is a *logical* shift (bit 7 becomes 0, not a copy of bit 7) and leaves C untouched. I followed the emulator. Check that this is what the manual means.
