# Flag register: what changes the flags, and the design

Module `flags`, in [circuits/flags/flags.circ](../circuits/flags/flags.circ), made by `designs/flags.py`. Built 2026-11-04.

The source of truth is the emulator: `hbc2/crates/hbc2-core/src/cpu.rs`, plus `Opcode::info().flags` in isa.rs. Bit order: **C0 E1 I2 N3 S4 Z5 F6 H7**.

## Every way a flag changes (from cpu.rs)

| Event | Flags written | Value | cpu.rs |
|---|---|---|---|
| ADD ADC SUB SBB INC DEC | C Z N | ALU (C = carry, or borrow after SUB/SBB/DEC) | `set_czn` |
| SHL ASR | C Z N | ALU (C = bit shifted out) | `Shl`/`Asr` + `set_zn` |
| AND OR XOR NOT SHR | Z N | ALU (SHR leaves C alone) | `set_zn` |
| CMP | S E F | a > b, a = b, a < b, unsigned | `Cmp` |
| POF | all 8, I and H included | popped byte | `Flags(self.pop(board))` |
| PSF | none | pushes the 8 flags | `push(self.flags.0)` |
| CLC CLE CLI CLN CLS CLZ CLF | one flag | 0 | `set_flag_if_none` |
| STC STE STI STN STS STZ STF | one flag | 1 | `set_flag_if_none` |
| HLT | H | 1 | `set_flag_if_none(Halt, true)` |
| IRT | I | 1 | `flags.set(Interrupt, true)` |
| Interrupt entry (hardware or INT) | I and H | both 0 | `enter_interrupt` |
| Invalid instruction (valid opcode, mode it doesn't accept) | H and I | H = 1, I = 0 | `execute_next`, `i.stops()` |
| Reset | all 8 | I = 1, the rest 0 | `reset` |

Each of the flag-changing instructions is checked against the real CPU by `hbc2 vectors flags_cpu`, and the group lists (C Z N, Z N, S E F, all 8) are taken from `Opcode::info().flags`, which `flags_model` asserts for every instruction of each group.

Things that differ from what was expected before reading the code:
- **IRT does not pop I from a byte.** It pops three bytes, the return address and the *register* I (the interrupt data register, not the flag), then sets the I flag to 1. In the register it's a single-flag write, like STI.
- **HLT:** `Opcode::info()` listed H and I, but cpu.rs only sets H. Gianni confirmed HLT doesn't affect I, and the table was fixed on 2026-11-04 (isa.rs: HLT lists H only; the one emulator change he authorised).
- **Interrupt entry clears H too**, so an interrupt wakes a halted CPU (with I = 1). It happens whether the CPU was halted or not.
- **An invalid instruction changes two flags** (H = 1, I = 0), and so does interrupt entry (I = 0, H = 0). The register writes one flag per clock, so the control unit spends two steps on each (see the mapping below). Gianni kept the two steps (2026-11-04): interrupt entry already takes several steps (pushes and vector fetch), so the two flag writes overlap them at no cost, and the invalid-mode case halts anyway. The control unit must schedule them: [control-requirements.md](control-requirements.md) C2.
- **STI takes effect after the next instruction** (`interrupt_delay`). That belongs to the control unit, not to this register: the I flag is 1 right after STI, the control unit must ignore it for one instruction.

## Interface

| Pin | Width | Side | Meaning |
|---|---|---|---|
| `flags_op` | 4 | top | what to store on the next `clk` edge, table below |
| `flags_bit_val` | 1 | top | value for a single-flag write |
| `flags_oe_n` | 1 | top | 0: drive the flags onto `flags_bus_out` (PSF) |
| `clk`, `rst_n` | 1 | top | rising edge stores; `rst_n` low clears at once (I = 1, the rest 0) |
| `flags_alu_c` `_z` `_n` `_s` `_e` `_f` | 1 | left | from the ALU (`alu_c` ... `alu_f`) |
| `flags_bus_in` | 8 | left | data bus, for POF |
| `flag_c` ... `flag_h` | 1 | right | the 8 flags (jump conditions; `flag_c` to `alu_cin`) |
| `flags_bus_out` | 8 | right | data bus, tri-state, for PSF |

### flags_op

| flags_op | Stores | Used by |
|---|---|---|
| 0000 | nothing (hold) | every step that doesn't change the flags |
| 0001 | C Z N from the ALU | ADD ADC SUB SBB INC DEC SHL ASR |
| 0010 | Z N from the ALU | AND OR XOR NOT SHR |
| 0011 | S E F from the ALU | CMP (alu_op 0001) |
| 0100 | all 8 from `flags_bus_in` | POF |
| 0101 to 0111 | nothing (hold) | free |
| 1fff | flag fff ← `flags_bit_val` (fff = bit number) | CLx/STx: 1000/1001/1010/1011/1100/1101/1110 = C E I N S Z F; HLT = 1111 with 1; IRT = 1010 with 1 |

Two-flag events, two steps each: interrupt entry = 1010 with 0 (I), then 1111 with 0 (H). Invalid instruction = 1111 with 1 (H), then 1010 with 0 (I). Reset = `rst_n`.

**Why one 4-bit code instead of one load line per group** (the suggested starting point was `flags_czn_ld`, `flags_zn_ld`, `flags_sef_ld`, `flags_bus_ld`, `flags_bit_ld` + `flags_bit_sel` + `flags_bit_val`): with one code, two loads can't be active at once, so there's no clash to define. It also takes 6 control lines instead of 9. It costs one 74HCT138 (U1).

## Design

| Chip | Part | Does |
|---|---|---|
| U1 | 74HCT138 | decodes flags_op 0xxx (enabled when op3 = 0): /Y1 = C Z N, /Y2 = Z N, /Y3 = S E F, /Y4 = bus |
| U7 | 74HCT138 | decodes flags_op 1fff (G1 = op3): one active-low strobe per flag |
| U2, U8, U13 | 74HCT11 | the 8 select lines (AND3 of active-low signals), and Z N load = /Y1 and /Y2 |
| U3, U9 | 74HCT157 | single-flag write: the flag written gets `flags_bit_val`, its partner its own value |
| U4, U10, U14, U15 | 74HCT153 | next value of each flag pair: hold / ALU / bus / single write |
| U5, U11 | 74HCT175 | the 8 flip-flops (C E I N, S Z F H) |
| U12 | 74HCT86 | inverts I's next value (3 gates spare) |
| U6 | 74HCT541 | PSF: flags onto the data bus |

**15 chips** ([bom-flags.csv](bom-flags.csv), `python tools/bom.py flags`): 74HCT153 × 4, 74HCT11 × 3, 74HCT138 × 2, 74HCT157 × 2, 74HCT175 × 2, 74HCT86 × 1, 74HCT541 × 1.

- **No gated clock.** Every flip-flop takes `clk`; holding is a multiplexer input (the flag's own output on D3).
- **Pairs.** A 74HCT153's two halves share their select lines, so the flags go in pairs that load together: (C, I), (E, H), (Z, N), (S, F). A pair either has the same ALU load (Z N; S F) or one flag the ALU never writes (I, H), which gets its own value on the ALU input. Each pair's two flags differ in bit 1 of their number, so on a single-flag write `flags_op_1` tells them apart (the 74HCT157s).
- **Inverted selects.** The select lines are AND3s of active-low signals, so they are low when a load is active. The 153 data inputs are wired in that order: (S1, S0) = 11 hold, 10 ALU, 01 bus, 00 single write.
- **I stored inverted.** The 175 clears to 0 and has no preset, so the I flip-flop stores not I and `flag_i` comes from its /Q: a reset gives I = 1. Its next value goes through a 74HCT86 gate wired as an inverter. The 74HCT10 (3-input NAND), which would have given the selects in the right polarity with a spare gate for this, is NRND at TI (CD74HCT10E) and Nexperia DIP is excluded, so it isn't used.
- **Layout:** chips drawn facing south (the datasheet's top view: pin 1 top left), every pin on a tunnel named after its signal.

## Timing

`python tools/timing.py --section flags` (pass criterion: max 25 °C; 85 °C for information), the register's clock-to-output taken as the CD74HCT574's (41 ns):

| Path | Through (per stage, max 25 / 85 °C, ns) | Typ | Max 25 °C | Max 85 °C | Limit (25 / 85 °C) | Margin 25 °C (pass) | Margin 85 °C (info) |
|---|---|---|---|---|---|---|---|
| ALU flag → flip-flop (C Z N S E F) | 08 A/B → Y (24 / 30) → 86 A/B → Y (32 / 40) → 283 Cin → C4 (46 / 58) → 157 S → Y (29 / 36) → 541 A → Y (23 / 29) → 27 A/B/C → Y (23 / 29) → 11 A/B/C → Y (28 / 35) → 574 CP → Q (33 / 41) → 153 I → Y (34 / 43) → 175 setup D → CLK (20 / 25) | 144 | 292 | 366 | ≤ 500 | 208 | 134 |
| flags_op → flip-flop (I, the slowest) | 574 CP → Q (33 / 41) → 138 A/B/C → Y (36 / 45) → 11 A/B/C → Y (28 / 35) → 153 S → Y (34 / 43) → 86 A/B → Y (32 / 40) → 175 setup D → CLK (20 / 25) | 96 | 183 | 229 | ≤ 500 | 317 | 271 |
| single-flag write → flip-flop (I) | 574 CP → Q (33 / 41) → 157 S → Y (29 / 36) → 153 I → Y (34 / 43) → 86 A/B → Y (32 / 40) → 175 setup D → CLK (20 / 25) | 77 | 148 | 185 | ≤ 500 | 352 | 315 |
| flags_bus_in → flip-flop (I), from the bus being valid | 153 I → Y (34 / 43) → 86 A/B → Y (32 / 40) → 175 setup D → CLK (20 / 25) | 47 | 86 | 108 | — | — | — |
| clk → flag_c ... flag_h | 175 CLK → Q (33 / 41) | 13 | 33 | 41 | — | — | — |
| clk → flags_bus_out (PSF, already enabled) | 175 CLK → Q (33 / 41) → 541 A → Y (23 + 37 / 29 + 49 load) | 52 | 93 | 119 | ≤ 500 | 407 | 381 |
| flags_oe_n → flags_bus_out | 541 OE → Y (30 + 37 / 38 + 49 load) | 47 | 67 | 87 | ≤ 500 | 433 | 413 |
| rst_n → flags | 175 /MR → Q (35 / 44) | 17 | 35 | 44 | — | — | — |

The data-bus rows include its load (about 420 pF, registers-decisions.md section 6), added 2026-11-04. Worst case: an ALU flag into the register, **292 ns at max 25 °C, 208 ns of margin** in the 500 ns step at 2 MHz (the pass criterion); 366 ns at max 85 °C (134 ns, for information). The prompt quoted alu_s at 321 ns: that was before fix F2; S/E/F now take 84–89 ns, and alu_z (257 ns) is the slowest ALU output.

## Tests

| Vectors | What | Result |
|---|---|---|
| `hbc2 vectors flags` | every flags_op × bit value, from every state of the 8 flags, data inputs the opposite of the state and then random, PSF output on and off; reset from every state | **66,816 / 66,816** |
| `hbc2 vectors flags_cpu` | the real CPU: every ALU instruction (8 operand pairs), POF, PSF, the 16 single-flag instructions (incl. HLT, IRT) from every state with H = 0; interrupt entry from every state with I = 1; invalid instruction; reset from every state. The rows drive the register as in the flags_op table; the flags at the end must equal the CPU's | **70,656 / 70,656** |

Both are sequences (Logisim's `<set>`/`<seq>` columns), run by `python designs/flags.py --test`.

## Gianni's decisions (2026-11-04)

1. **flags_op as one 4-bit code**, with its 74HCT138: approved.
2. **Two-step interrupt entry and invalid instruction**: kept. A fifth multiplexer source for I and H (1–2 chips) isn't worth it; the control unit schedules them (control-requirements.md C2).
3. **HLT in `Opcode::info()`**: fixed, HLT lists only H (isa.rs). The emulator's full test suite was run afterwards.
4. **74HCT175**: approved.
