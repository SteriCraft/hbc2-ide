# Chips: what to buy, how fast they are, what Logisim has

The parts used or considered for the HBC-2, so the next module doesn't have to search again. Pinouts and wiring choices are in [ttl-chips.md](ttl-chips.md). Stock and prices (Octopart, 2026-11-04): [disponibilite-74hct.md](disponibilite-74hct.md).

Two tools read the tables below, so keep their column names:
- `tools/bom.py` reads the tables of **Parts in use** (one per module) and writes a bill of materials, such as [bom-alu.csv](bom-alu.csv). It fails if a circuit uses a chip that isn't listed there.
- `tools/timing.py` reads **Delays** and prints the path table of [alu-report.md](alu-report.md).

## Rules (Gianni, 2026-11-04)

- Every chip must be sold as a **74HCTxx**, the CMOS family with TTL-compatible inputs.
- Both DIP and **SMD (SOIC)** are fine; it doesn't have to be through-hole. DIP is for the breadboard, SOIC for the final PCBs.
- If a chip is missing from Logisim's `#TTL` library, it can be **modelled** (rule replaced 2026-11-04): one file per chip, `circuits/chips/<part>.circ`, made from the datasheet (kept next to it), with the datasheet's pin numbers and names in DIP order, VCC and GND included, and tested in `test_<part>` in the same file. Generic Logisim components are allowed inside a model only. A model instance is a chip like any other: `U<n>` label, counted by bom.py, listed below. See [conventions.md](conventions.md). Choices made under the old rule stand (cmp8 keeps its 7485s).

Sourcing rules, for every section:
- **Never use Nexperia DIP parts** (suffix `N`, such as 74HCT86N). They have been obsolete since 2015, and what is left is sold by unauthorised resellers.
- **Avoid TI variants that are NRND or have no offer left**: SN74HCT02N, SN74HCT157N, SN74HCT86N. Use the CD74HCT…E version of the same chip.
- **Use authorised distributors only** (Mouser, DigiKey, TME, LCSC). No eBay or AliExpress, where "74HCT" parts are sometimes remarked HC or LS chips.
- Before adding a part, check that its DIP and SOIC references are active, and write both in the table below.

## Parts in use

One table per module, so a module's parts can be read (and bought) on their own. A chip used by several modules is repeated, with the same references. When a new module gets its circuit, add its table here, in the module's section. bom.py reads all the tables.

### ALU

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT08 | quad 2-input AND | 7408 | SN74HCT08N | 74HCT08D (Nexperia) | 3 |
| 74HCT11 | triple 3-input AND | 7411 | CD74HCT11E | 74HCT11D (Nexperia) | 2 |
| 74HCT27 | triple 3-input NOR | 7427 | CD74HCT27E | 74HCT27D (Nexperia) | 1 |
| 74HCT32 | quad 2-input OR | 7432 | SN74HCT32N | 74HCT32D (Nexperia) | 2 |
| 74HCT85 | 4-bit magnitude comparator | 7485 | CD74HCT85E | CD74HCT85M or CD74HCT85M96 (TI) | 2 |
| 74HCT86 | quad 2-input XOR | 7486 | CD74HCT86E | 74HCT86D (Nexperia) | 5 |
| 74HCT138 | 3-to-8 decoder | 74138 | SN74HCT138N | 74HCT138D (Nexperia) | 1 |
| 74HCT153 | dual 4-to-1 multiplexer | 74153 | CD74HCT153E | 74HCT153D (Nexperia) | 1 |
| 74HCT157 | quad 2-to-1 multiplexer | 74157 | CD74HCT157E | 74HCT157D (Nexperia) | 2 |
| 74HCT240 | octal inverting buffer, tri-state | 74240 | CD74HCT240E | 74HCT240D (Nexperia) | 1 |
| 74HCT283 | 4-bit binary adder | 74283 | CD74HCT283E | CD74HCT283M96 (TI) | 3 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 6 |

### Flags

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT11 | triple 3-input AND | 7411 | CD74HCT11E | 74HCT11D (Nexperia) | 3 |
| 74HCT86 | quad 2-input XOR | 7486 | CD74HCT86E | 74HCT86D (Nexperia) | 1 |
| 74HCT138 | 3-to-8 decoder | 74138 | SN74HCT138N | 74HCT138D (Nexperia) | 2 |
| 74HCT153 | dual 4-to-1 multiplexer | 74153 | CD74HCT153E | 74HCT153D (Nexperia) | 4 |
| 74HCT157 | quad 2-to-1 multiplexer | 74157 | CD74HCT157E | 74HCT157D (Nexperia) | 2 |
| 74HCT175 | quad D flip-flop with clear | 74175 | CD74HCT175E | CD74HCT175M96 (TI) | 2 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 1 |

### Registers

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT32 | quad 2-input OR | 7432 | SN74HCT32N | 74HCT32D (Nexperia) | 1 |
| 74HCT153 | dual 4-to-1 multiplexer | 74153 | CD74HCT153E | 74HCT153D (Nexperia) | 4 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 2 |
| 74HCT574 | octal D flip-flop, tri-state | 74574 (model) | CD74HCT574E | CD74HCT574M96 (TI) | 3 |
| 74HCT670 | 4 × 4 register file, tri-state | 74670 | CD74HCT670E | CD74HCT670M (TI) | 8 |

### Address (PC, SP, adder, latch)

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT08 | quad 2-input AND | 7408 | SN74HCT08N | 74HCT08D (Nexperia) | 2 |
| 74HCT86 | quad 2-input XOR | 7486 | CD74HCT86E | 74HCT86D (Nexperia) | 4 |
| 74HCT153 | dual 4-to-1 multiplexer | 74153 | CD74HCT153E | 74HCT153D (Nexperia) | 8 |
| 74HCT157 | quad 2-to-1 multiplexer | 74157 | CD74HCT157E | 74HCT157D (Nexperia) | 4 |
| 74HCT283 | 4-bit binary adder | 74283 | CD74HCT283E | CD74HCT283M96 (TI) | 4 |
| 74HCT377 | octal D flip-flop, data enable | 74377 | CD74HCT377E | CD74HCT377M96 (TI) | 6 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 6 |

The 74HCT377 was added on 2026-11-04 for this section. TI lists CD74HCT377E (PDIP) and CD74HCT377M96 (SOIC, tape and reel) as active in the datasheet's package addendum (SCHS184C); the tube SOIC CD74HCT377M is obsolete. Stock not checked yet (Gianni's Octopart survey predates it).

### Instruction register

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT14 | hex Schmitt-trigger inverter | 7414 | SN74HCT14N | SN74HCT14D or 74HCT14D (Nexperia) | 1 |
| 74HCT377 | octal D flip-flop, data enable | 74377 | CD74HCT377E | CD74HCT377M96 (TI) | 6 |

### Memory

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT14 | hex Schmitt-trigger inverter | 7414 | SN74HCT14N | SN74HCT14D or 74HCT14D (Nexperia) | 1 |
| 74HCT32 | quad 2-input OR | 7432 | SN74HCT32N | 74HCT32D (Nexperia) | 1 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 3 |
| AS6C1008 | 128 K × 8 static RAM, 55 ns | as6c1008 (model) | AS6C1008-55PCN | AS6C1008-55SIN (Alliance) | 1 |
| AT28C256 | 32 K × 8 parallel EEPROM, 150 ns | at28c256 (model) | AT28C256-15PU | AT28C256-15SU (Microchip) | 2 |

The memories were added on 2026-11-05 (ir-mem-decisions.md section 4, M2–M4). AS6C1008-55PCN: PDIP-32 600 mil, 0–70 °C, DigiKey "active"; the SOIC is the industrial -55SIN (not checked on Octopart). AT28C256-15PU: PDIP-28, Microchip, active at Arrow / findchips; AT28C256-15SU is the SOIC-28 (not checked). Both are programmed / socketed parts, not 74HCT: the 74HCT rule doesn't apply to memories. Levels: the AT28C256 guarantees VOH 2.4 V only, so it drives the RAM through a 74HCT541 (M4).

### Not in a circuit yet

Listed for the modules to come or no longer used (74HCT02). Move a row to its module's table when a circuit uses it. The 74HCT74 and 74HCT10 are the clock generator's (control-requirements.md C6); a second 74HCT74 and the 74HCT14 are the reset generator's (C17).

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT02 | quad 2-input NOR | 7402 | CD74HCT02E | SN74HCT02D or 74HCT02D (Nexperia) | 0 |
| 74HCT10 | triple 3-input NAND | 7410 | CD74HCT10E | 74HCT10D (Nexperia) | 0 |
| 74HCT74 | dual D flip-flop, set / reset | 7474 | SN74HCT74N | 74HCT74D (Nexperia) | 0 |

The 74HCT02 is no longer used by the ALU (its S gate went with cmp8), but it stays listed for later sections. All DIP references are TI and active. The 74HCT85 was added on 2026-11-04 for cmp8: TI lists the CD74HCT85 as active, with about 1,400 CD74HCT85E at Mouser and 6,300 at DigiKey (aggregator figures, check in the basket). For the 74HCT283 in SOIC, take the TI part: the Nexperia 74HCT283D is discontinued. The 74HCT27 is as easy to buy as the others (CD74HCT27E: about 1,300 at Mouser).

## Delays

From the TI datasheets of the DIP references above, CL = 50 pF. **Max** columns are at VCC = 4.5 V, at 25 °C and over −40…85 °C. **Typ** is at 25 °C, but at the conditions each datasheet uses:
- SN74HCT parts (08, 32, 138, 541) give it at 4.5 V and 50 pF;
- CD74HCT parts give it only at **5 V and 15 pF**, which is quicker than 4.5 V and 50 pF.

So Typ is a rough guide only.

**Pass criterion** (Gianni, 2026-11-04): every path must meet its limit at **Max 25 °C**, bus loading included (`tools/timing.py`, which exits 1 otherwise). Max 85 °C is reported next to it, for information only.

| Part | Path | Typ | Max 25 °C | Max 85 °C | Datasheet |
|---|---|---|---|---|---|
| 74HCT02 | A/B → Y | 8 | 21 | 26 | CD74HCT02, SCHS400 |
| 74HCT08 | A/B → Y | 15 | 24 | 30 | SN74HCT08, SCLS063G |
| 74HCT10 | A/B/C → Y | 9 | 24 | 30 | CD74HCT10, SCHS404 (typ: 5 V, 15 pF) |
| 74HCT11 | A/B/C → Y | 11 | 28 | 35 | CD74HCT11, SCHS405 |
| 74HCT14 | A → Y | 20 | 32 | 40 | SN74HCT14, SCLS225G |
| 74HCT27 | A/B/C → Y | 9 | 23 | 29 | CD74HCT27, SCHS406 |
| 74HCT32 | A/B → Y | 15 | 24 | 30 | SN74HCT32, SCLS064G |
| 74HCT74 | CLK → Q | 20 | 28 | 35 | SN74HCT74, SCLS122 (Q or /Q; fmax 20 MHz at 85 °C) |
| 74HCT85 | A/B → A>B, A<B | 15 | 37 | 46 | CD74HCT85 |
| 74HCT85 | A/B → A=B | 17 | 40 | 50 | CD74HCT85 |
| 74HCT85 | cascade in → A>B, A<B | 12 | 30 | 38 | CD74HCT85 |
| 74HCT85 | cascade in → A=B | 13 | 31 | 39 | CD74HCT85 (see note) |
| 74HCT86 | A/B → Y | 13 | 32 | 40 | CD74HCT86, SCHS410A |
| 74HCT138 | A/B/C → Y | 23 | 36 | 45 | SN74HCT138, SCLS171F |
| 74HCT138 | G → Y | 22 | 33 | 42 | SN74HCT138, SCLS171F |
| 74HCT153 | S → Y | 14 | 34 | 43 | CD74HCT153, SCHS151C |
| 74HCT153 | I → Y | 14 | 34 | 43 | CD74HCT153, SCHS151C (see note) |
| 74HCT153 | E → Y | 11 | 27 | 34 | CD74HCT153, SCHS151C |
| 74HCT157 | I → Y | 12 | 25 | 31 | CD74HCT157, SCHS153C |
| 74HCT157 | S → Y | 15 | 29 | 36 | CD74HCT157, SCHS153C |
| 74HCT175 | CLK → Q | 13 | 33 | 41 | CD74HCT175, SCHS160C |
| 74HCT175 | /MR → Q | 17 | 35 | 44 | CD74HCT175, SCHS160C |
| 74HCT175 | setup D → CLK | 20 | 20 | 25 | CD74HCT175, SCHS160C (a minimum; no typ, 25 °C value repeated) |
| 74HCT240 | A → Y | 9 | 22 | 28 | CD74HCT240, SCHS167G |
| 74HCT240 | OE → Y | 12 | 30 | 38 | CD74HCT240, SCHS167G (typ: HC value) |
| 74HCT283 | Cin → S0 | 13 | 31 | 39 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → S1 | 18 | 43 | 54 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → S2 | 19 | 46 | 58 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → S3 | 22 | 53 | 66 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → C4 | 19 | 46 | 58 | CD74HCT283, SCHS176E |
| 74HCT283 | A/B → C4 | 20 | 48 | 60 | CD74HCT283, SCHS176E |
| 74HCT283 | A/B → S | 21 | 49 | 61 | CD74HCT283, SCHS176E |
| 74HCT377 | CP → Q | 16 | 38 | 48 | CD74HCT377, SCHS184C (typ: 5 V, 15 pF; standard outputs) |
| 74HCT377 | setup D, /E → CP | 12 | 12 | 15 | CD74HCT377, SCHS184C (a minimum; hold D 3, /E 5) |
| 74HCT541 | A → Y | 13 | 23 | 29 | SN74HCT541, SCLS306E |
| 74HCT541 | OE → Y | 21 | 30 | 38 | SN74HCT541, SCLS306E |
| 74HCT574 | CP → Q | 15 | 33 | 41 | CD74HCT574, SCHS183E |
| 74HCT574 | OE → Q | 12 | 30 | 38 | CD74HCT574, SCHS183E (setup D → CP 15, hold 5, at 85 °C) |
| 74HCT574 | setup D → CP | 12 | 12 | 15 | CD74HCT574, SCHS183E (a minimum; hold 5 at 85 °C) |
| 74HCT670 | read address → Q | 17 | 40 | 50 | CD74HCT670, SCHS195C |
| 74HCT670 | /WE → Q | 21 | 50 | 63 | CD74HCT670, SCHS195C |
| 74HCT670 | D → Q | 21 | 50 | 63 | CD74HCT670, SCHS195C (write transparent) |
| 74HCT670 | /RE → Q | 16 | 38 | 48 | CD74HCT670, SCHS195C |
| 74HCT670 | /RE → Q off | 14 | 35 | 44 | CD74HCT670, SCHS195C (output disable) |
| 74HCT670 | setup D → /WE | 12 | 12 | 15 | CD74HCT670, SCHS195C (a minimum, to /WE rising) |
| 74HCT670 | setup WA → /WE | 18 | 18 | 23 | CD74HCT670, SCHS195C (a minimum, to /WE falling) |
| AS6C1008 | address → DQ (tAA) | 55 | 55 | 55 | AS6C1008 v1.0 (-55; one figure, 0–70 °C) |
| AS6C1008 | CE#, CE2 → DQ (tACE) | 55 | 55 | 55 | AS6C1008 v1.0 |
| AS6C1008 | OE# → DQ (tOE) | 30 | 30 | 30 | AS6C1008 v1.0 |
| AS6C1008 | write pulse WE# (tWP) | 45 | 45 | 45 | AS6C1008 v1.0 (a minimum; tAS 0, tAW 50, tWR 0) |
| AS6C1008 | setup DIN → WE# rising (tDW) | 25 | 25 | 25 | AS6C1008 v1.0 (a minimum; tDH 0) |
| AT28C256 | address → I/O (tACC) | 150 | 150 | 150 | AT28C256 (Atmel 0006F, -15) |
| AT28C256 | /CE → I/O (tCE) | 150 | 150 | 150 | AT28C256 (Atmel 0006F) |
| AT28C256 | /OE → I/O (tOE) | 70 | 70 | 70 | AT28C256 (Atmel 0006F) |
| 74HCT670 | hold D, WA after /WE | 5 | 5 | 5 | CD74HCT670, SCHS195C (a minimum) |
| 74HCT670 | /WE pulse width | 20 | 20 | 25 | CD74HCT670, SCHS195C (a minimum) |
| 74HCT139 | A/B → Y | 14 | 34 | 43 | SN74HCT139, SCLS066E (not used: registers-decisions.md D5) |

Notes:
- The CD74HCT153 datasheet lists "I to Y" twice for the HCT types, at 24/30 ns (typ 9) and at 34/43 ns (typ 14). The table keeps the slower one.
- The CD74HCT85 datasheet names its last row "(A > B) IN to (A = B) OUT". It is read here as the cascade path to A=B.
- The CD74HCT240 gives no HCT typical for the enable time. 12 ns is the HC typical.
- At 5 V instead of 4.5 V, everything is about 10 % faster. Every Max 85 °C figure is about 1.25 × the Max 25 °C one.
- The Nexperia SOIC parts are in the same range, but they are specified differently (typ at 4.5 V and 50 pF). Check their datasheet before relying on a SOIC timing.

## Fallbacks if the hardware shows timing errors (74ACT)

Decision (Gianni, 2026-11-04): the CPU stays **all-HCT**. If the built hardware shows timing errors, these 74ACT parts are pin-compatible drop-in replacements: same pinout and logic, TTL-level inputs like HCT, so only the chip changes, not the board or the circuit. Not in any Parts in use table; move a row there only if a fallback is fitted.

| Part | Replaces | Where | What it gains (max 85 °C) |
|---|---|---|---|
| 74ACT153 | 74HCT153 | registers U1, U6, U11, U15 (index multiplexers); flags, the 4 input multiplexers | CD74ACT153: S → Y 20 ns, I → Y 16.4 ns, against 43. End to end (register read → ALU → flag setup) 520 → 477 ns with the 4 in the registers, 451 ns with the 8 (registers-decisions.md section 6) |
| 74ACT74 | 74HCT74 | clock generator (control-requirements.md C6) | /WE pulse about 75 ns instead of 30 ns at max 85 °C (registers-decisions.md D7) |
| 74ACT10 | 74HCT10 | clock generator (C6) | the same, with the 74ACT74 |

Before buying: check that the DIP and SOIC references are active and sold by an authorised distributor (Rules, above), and add them to this table. AC parts switch fast (edges of about 3 ns) and drive 24 mA: decouple each one (100 nF at the pins) and keep its lines short. In these positions they drive only local nets (the 670 addresses and enables, the 175 inputs, the clock generator), none of the buses.

## In Logisim but not a 74HCT part (don't use)

- **74181 / 74182** (4-bit ALU / carry lookahead): sold only as LS/F/S TTL, which are obsolete. No HCT version found. Two 74181s would have done add, sub, the logic operations and A = B on their own.
- **74381** (4-bit ALU): S/LS only.
- Some parts can't be checked from their Logisim name alone (7413, 7418, 7419, 7421, 7451, 7454, 747266). Check them before use. The 74670 is checked: it is a 74HCT part (below).

## In #TTL or modelled (checked 2026-11-04, Logisim-evolution 5.0)

| Part | In #TTL | Note |
|---|---|---|
| 7474, 74138, 74153, 74157, 74161, 74175, 74193, 74377, 74541, 74670 | yes | |
| 74377 | yes | checked against cd74hct377.pdf (TI SCHS184C) by [circuits/chips/test_74377.circ](../circuits/chips/test_74377.circ): loads on CP rising with /E low, /E is sampled at the edge (no gated clock: /E moving while CP is high loads nothing), 250 / 250 rows |
| 74574 | **no** | modelled: [circuits/chips/74574.circ](../circuits/chips/74574.circ) (circuit `ttl74574`), from cd74hct574.pdf |

Logisim's 74670 was checked against the CD74HCT670 datasheet (cd74hc670.pdf, which covers HCT) by [circuits/chips/test_74670.circ](../circuits/chips/test_74670.circ): writes are transparent while /WE is low (the addressed word follows D), reads with /RE high are high impedance. It needs no model.

Chip models live in `circuits/chips/`, one file per part, with the datasheet next to them. Rules: chips.md "Rules", conventions.md "Components".

## Real 74HCT parts missing from Logisim

- **74HCT688** (8-bit equality comparator): would give A = B in one chip, but it would have needed a model (not allowed then). cmp8 uses two 74HCT85 instead, which give S, E and F, and keeps them (alu-decisions.md D15).
- **74HCT4078** (8-input NOR/OR): would give Z in one chip instead of 74HCT27 + 74HCT11. Not used: the 4078 is mostly sold as HC, and the 27 + 11 cost one chip more but no new model.
- **74HCT540** (inverting 541): the 74HCT240 does the same job here.
- **74HCT154** (4-to-16 decoder): would remove the 7411 gate behind the shift-right enable.
