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

The memories were added on 2026-11-05 (ir-mem-decisions.md section 4, M2–M4). AS6C1008-55PCN: PDIP-32 600 mil, 0–70 °C, DigiKey "active"; the SOIC is the industrial -55SIN (not checked on Octopart). AT28C256-15PU: PDIP-28, Microchip, active at Arrow / findchips; AT28C256-15SU is the SOIC-28 (not checked). Both are programmed / socketed parts, not 74HCT: the 74HCT rule doesn't apply to memories. Levels: the AT28C256 guarantees VOH 2.4 V only (DS20006386B p. 7) and the AS6C1008 needs VIH 0.7 × VCC = 3.5 V (v1.2 p. 4), so the ROMs drive the RAM through a 74HCT541 (M4, re-checked against these revisions on 2026-11-10, ir-mem-decisions.md section 11). Datasheets: `circuits/chips/as6c1008.pdf` (Alliance v1.2, March 2023) and `at28c256.pdf` (Microchip DS20006386B, September 2022).

### Clock and reset (clkgen)

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT10 | triple 3-input NAND | 7410 | CD74HCT10E | 74HCT10D (Nexperia) | 1 |
| 74HCT14 | hex Schmitt-trigger inverter | 7414 | SN74HCT14N | SN74HCT14D or 74HCT14D (Nexperia) | 1 |
| 74HCT74 | dual D flip-flop, set / reset | 7474 | SN74HCT74N | 74HCT74D (Nexperia) | 2 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 1 |

Added 2026-11-05 (cu-decisions.md section 14): the clock generator (74HCT74 + 74HCT10, C6), the reset generator (74HCT14 + 74HCT74, C17) and the clock buffer (74HCT541). Off the logic: a **4.000 MHz HCMOS oscillator module** (5 V, DIP-8 or DIP-14 can, 45–55 % duty; reference to choose) and the RC (resistor, capacitor, push button, diode).

### Control unit (cu)

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT00 | quad 2-input NAND | 7400 | CD74HCT00E | CD74HCT00M96 (TI) | 1 |
| 74HCT08 | quad 2-input AND | 7408 | SN74HCT08N | 74HCT08D (Nexperia) | 2 |
| 74HCT14 | hex Schmitt-trigger inverter | 7414 | SN74HCT14N | SN74HCT14D or 74HCT14D (Nexperia) | 1 |
| 74HCT27 | triple 3-input NOR | 7427 | CD74HCT27E | 74HCT27D (Nexperia) | 1 |
| 74HCT138 | 3-to-8 decoder | 74138 | SN74HCT138N | 74HCT138D (Nexperia) | 1 |
| 74HCT151 | 8-to-1 multiplexer | 74151 | CD74HCT151E | CD74HCT151M96 (TI) | 1 |
| 74HCT153 | dual 4-to-1 multiplexer | 74153 | CD74HCT153E | 74HCT153D (Nexperia) | 1 |
| 74HCT157 | quad 2-to-1 multiplexer | 74157 | CD74HCT157E | 74HCT157D (Nexperia) | 3 |
| 74HCT175 | quad D flip-flop with clear | 74175 | CD74HCT175E | CD74HCT175M96 (TI) | 2 |
| 74HCT377 | octal D flip-flop, data enable | 74377 | CD74HCT377E | CD74HCT377M96 (TI) | 1 |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) | 1 |
| 74HCT574 | octal D flip-flop, tri-state | ttl74574 (model) | CD74HCT574E | CD74HCT574M96 (TI) | 8 |
| AT28C256 | 32 K × 8 parallel EEPROM, 150 ns | at28c256 (model) | AT28C256-15PU | AT28C256-15SU (Microchip) | 7 |

Added 2026-11-05 (cu-decisions.md section 14). New parts: **74HCT00 = CD74HCT00E** (TI SCHS398, active; the SN74HCT00N is NRND, so not that one) and **74HCT151 = CD74HCT151E** (TI SCHS150D, active). Off the logic: the pull resistors of C24, **bussed resistor networks, 10 kΩ, SIP-9** (8 resistors and a common pin; e.g. Bourns 4609X-101-103LF, to check before buying), one network per group of up to 8 lines pulled the same way: `bom.py` counts them from the Pull Resistors.

### I/O mailbox (iod, notes/bom-iod.csv)

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT32 | quad 2-input OR | 7432 | SN74HCT32N | 74HCT32D (Nexperia) | 1 |
| 74HCT74 | dual D flip-flop, set / reset | 7474 | SN74HCT74N | 74HCT74D (Nexperia) | 4 |
| 74HCT138 | 3-to-8 decoder | 74138 | SN74HCT138N | 74HCT138D (Nexperia) | 1 |
| 74HCT165 | 8-bit parallel-in / serial-out shift register | 74165 | CD74HCT165E | CD74HCT165M96 (TI) | 1 |
| 74HCT244 | octal buffer, tri-state, two 4-bit halves | 74244 | CD74HCT244E (STATUS); SN74LVC244AN at 3.3 V (the ESP32 side) | 74HCT244D (Nexperia); SN74LVC244ADWR (TI) | 2 |
| 74HCT595 | 8-bit shift register + storage register, tri-state | 74595 (model) | SN74AHCT595N | SN74AHCT595D (TI) | 3 |

Added 2026-11-11 (iod-requirements.md, target design). Two exceptions to the 74HCT rule, both decided with the mailbox (io-redesign-proposal.md 9.8, Q-C11):
- **SN74AHCT595N:** TI makes no 74HCT595 in DIP (test-rig.md). The AHCT part has the same pinout and TTL-level inputs, and is faster (OE → Q 10.6 ns).
- **SN74LVC244AN:** powered at 3.3 V, its 5 V-tolerant inputs take the card's 5 V signals to the ESP32. It is the same Logisim chip as the 74HCT244, so `bom.py` counts both in one row.

### Whole CPU (cpu.circ `main`, notes/bom-cpu.csv)

| Part | Function | DIP | Qty | Sections | Spares to buy |
|---|---|---|---|---|---|
| 74HCT00 | quad 2-input NAND | CD74HCT00E | 1 | cu | 1 |
| 74HCT08 | quad 2-input AND | SN74HCT08N | 7 | add_sub8 addr cu logic8 | 2 |
| 74HCT10 | triple 3-input NAND | CD74HCT10E | 1 | clkgen | 1 |
| 74HCT11 | triple 3-input AND | CD74HCT11E | 5 | flags flags8 rmux8 | 1 |
| 74HCT14 | hex Schmitt-trigger inverter | SN74HCT14N | 4 | clkgen cu ir mem | 1 |
| 74HCT27 | triple 3-input NOR | CD74HCT27E | 2 | cu flags8 | 1 |
| 74HCT32 | quad 2-input OR | SN74HCT32N | 4 | logic8 mem regs | 1 |
| 74HCT74 | dual D flip-flop, set / reset | SN74HCT74N | 2 | clkgen | 1 |
| 74HCT85 | 4-bit magnitude comparator | CD74HCT85E | 2 | cmp8 | 1 |
| 74HCT86 | quad 2-input XOR | CD74HCT86E | 10 | add_sub8 addr flags logic8 | 2 |
| AT28C256 | 32 K × 8 parallel EEPROM, 150 ns | AT28C256-15PU | 9 | cu mem | 2 |
| AS6C1008 | 128 K × 8 static RAM, 55 ns | AS6C1008-55PCN | 1 | mem | 1 |
| 74HCT138 | 3-to-8 decoder | SN74HCT138N | 4 | cu flags rmux8 | 1 |
| 74HCT151 | 8-to-1 multiplexer | CD74HCT151E | 1 | cu | 1 |
| 74HCT153 | dual 4-to-1 multiplexer | CD74HCT153E | 18 | addr cu flags regs shift8 | 3 |
| 74HCT157 | quad 2-to-1 multiplexer | CD74HCT157E | 11 | add_sub8 addr cu flags | 2 |
| 74HCT175 | quad D flip-flop with clear | CD74HCT175E | 4 | cu flags | 1 |
| 74HCT240 | octal inverting buffer, tri-state | CD74HCT240E | 1 | rmux8 | 1 |
| 74HCT283 | 4-bit binary adder | CD74HCT283E | 7 | add_sub8 addr | 2 |
| 74HCT377 | octal D flip-flop, data enable | CD74HCT377E | 13 | addr cu ir | 2 |
| 74HCT541 | octal buffer, tri-state | SN74HCT541N | 20 | addr clkgen cu flags mem regs rmux8 | 3 |
| 74HCT574 | octal D flip-flop, tri-state | CD74HCT574E | 11 | cu regs | 2 |
| 74HCT670 | 4 × 4 register file, tri-state | CD74HCT670E | 8 | regs | 2 |
| resistor network | 10 kΩ × 8, bussed, SIP-9 (pull resistors, C24): 36 up, 24 down | 4609X-101-103LF (Bourns) | 8 | cu | 1 |

Final count (2026-11-10, phase 4): **146 chips in 22 part numbers** (counted by `bom.py` on `main`, every section instance included; spares as `bom.py` suggests them, except the 74HCT10: at least 3, NRND), plus **8 resistor networks** (C24's 60 pull resistors: 36 up, 24 down). Off the logic: the 4.000 MHz oscillator module, the reset RC (resistor, capacitor, push button, diode), decoupling capacitors (one 100 nF per chip), sockets. `python designs/cpu.py` rewrites bom-cpu.csv; `bom.py cpu.circ --check notes/bom-cpu.csv` checks it. The shopping list for the breadboard build, with spares and everything off the logic: [breadboard-order.md](breadboard-order.md).

### Not in a circuit yet

Listed for the modules to come or no longer used (74HCT02). Move a row to its module's table when a circuit uses it.

| Part | Function | Logisim | DIP | SOIC | Qty |
|---|---|---|---|---|---|
| 74HCT02 | quad 2-input NOR | 7402 | CD74HCT02E | SN74HCT02D or 74HCT02D (Nexperia) | 0 |

**74HCT10: NRND, used anyway (Gianni, 2026-11-05).** TI marks CD74HCT10E "not recommended for new designs" but still makes it: DigiKey stocks it (6-week factory lead time, Rochester Electronics as a second source). It is the only HCT 3-input NAND in DIP: TI has no SN74HCT10N, and the Nexperia 74HCT10N is discontinued. Gianni kept it in the clock generator over the 74HCT32 + 74HCT14 replacement, whose pulse end left 3.5 ns before the next edge at 55 % duty (cu-decisions.md section 14). **Buy spares** (at least 3 for the one fitted). If it can no longer be bought: **SN74ACT10N** (TI, PDIP-14, active, 9.5 ns max at 5 V) fits the same socket, see the 74ACT fallbacks below.

The 74HCT02 is no longer used by the ALU (its S gate went with cmp8), but it stays listed for later sections. All DIP references are TI and active. The 74HCT85 was added on 2026-11-04 for cmp8: TI lists the CD74HCT85 as active, with about 1,400 CD74HCT85E at Mouser and 6,300 at DigiKey (aggregator figures, check in the basket). For the 74HCT283 in SOIC, take the TI part: the Nexperia 74HCT283D is discontinued. The 74HCT27 is as easy to buy as the others (CD74HCT27E: about 1,300 at Mouser).

## Delays

From the TI datasheets of the DIP references above, CL = 50 pF. **Max** columns are at VCC = 4.5 V, at 25 °C and over −40…85 °C. **Typ** is at 25 °C, but at the conditions each datasheet uses:
- SN74HCT parts (08, 32, 138, 541) give it at 4.5 V and 50 pF;
- CD74HCT parts give it only at **5 V and 15 pF**, which is quicker than 4.5 V and 50 pF.

So Typ is a rough guide only.

**Pass criterion** (Gianni, 2026-11-04): every path must meet its limit at **Max 25 °C**, bus loading included (`tools/timing.py`, which exits 1 otherwise). Max 85 °C is reported next to it, for information only.

| Part | Path | Typ | Max 25 °C | Max 85 °C | Datasheet |
|---|---|---|---|---|---|
| 74HCT00 | A/B → Y | 8 | 20 | 25 | CD74HCT00, SCHS398 (typ: 5 V, 15 pF) |
| 74HCT02 | A/B → Y | 8 | 21 | 26 | CD74HCT02, SCHS400 |
| 74HCT08 | A/B → Y | 15 | 24 | 30 | SN74HCT08, SCLS063G |
| 74HCT10 | A/B/C → Y | 9 | 24 | 30 | CD74HCT10, SCHS404 (typ: 5 V, 15 pF) |
| 74HCT11 | A/B/C → Y | 11 | 28 | 35 | CD74HCT11, SCHS405 |
| 74HCT14 | A → Y | 20 | 32 | 40 | SN74HCT14, SCLS225G |
| 74HCT27 | A/B/C → Y | 9 | 23 | 29 | CD74HCT27, SCHS406 |
| 74HCT32 | A/B → Y | 15 | 24 | 30 | SN74HCT32, SCLS064G |
| 74HCT74 | CLK → Q | 20 | 28 | 35 | SN74HCT74, SCLS122 (Q or /Q; fmax 20 MHz at 85 °C) |
| 74HCT74 | PRE or CLR → Q | 21 | 35 | 44 | SN74HCT74, SCLS169G |
| 74HCT74 | setup D → CLK | 12 | 12 | 15 | SN74HCT74, SCLS169G (hold 0) |
| 74HCT165 | setup A–H → SH/LD | 20 | 20 | 25 | CD74HCT165 (a minimum; hold not given) |
| 74HCT165 | CLK → QH | 17 | 40 | 50 | CD74HCT165 (setup A–H → SH/LD 20 / 25; typ: 5 V, 15 pF) |
| 74HCT244 | A → Y | 13 | 25 | 31 | CD74HCT244 (typ: 5 V, 15 pF) |
| 74HCT244 | OE → Y | 15 | 30 | 38 | CD74HCT244, enable and disable |
| 74AHCT595 | OE → Q | 7 | 11 | 15 | SN74AHCT595, SCLS374Q (enable 10.6 / 14.4, disable 10.3 / 13.2; 85 °C column: −40…125 °C) |
| 74AHCT595 | RCLK → Q | 6 | 10 | 12 | SN74AHCT595, SCLS374Q (9.4 / 11.5) |
| 74HCT85 | A/B → A>B, A<B | 15 | 37 | 46 | CD74HCT85 |
| 74HCT85 | A/B → A=B | 17 | 40 | 50 | CD74HCT85 |
| 74HCT85 | cascade in → A>B, A<B | 12 | 30 | 38 | CD74HCT85 |
| 74HCT85 | cascade in → A=B | 13 | 31 | 39 | CD74HCT85 (see note) |
| 74HCT86 | A/B → Y | 13 | 32 | 40 | CD74HCT86, SCHS410A |
| 74HCT138 | A/B/C → Y | 23 | 36 | 45 | SN74HCT138, SCLS171F |
| 74HCT138 | G → Y | 22 | 33 | 42 | SN74HCT138, SCLS171F |
| 74HCT151 | S (A/B/C) → Y | 17 | 41 | 51 | CD74HCT151, SCHS150D (typ: 5 V, 15 pF) |
| 74HCT151 | D → Y | 16 | 38 | 48 | CD74HCT151, SCHS150D |
| 74HCT151 | /G → Y | 12 | 29 | 36 | CD74HCT151, SCHS150D |
| 74HCT153 | S → Y | 14 | 34 | 43 | CD74HCT153, SCHS151C |
| 74HCT153 | I → Y | 14 | 34 | 43 | CD74HCT153, SCHS151C (see note) |
| 74HCT153 | E → Y | 11 | 27 | 34 | CD74HCT153, SCHS151C |
| 74HCT157 | I → Y | 12 | 25 | 31 | CD74HCT157, SCHS153C |
| 74HCT157 | S → Y | 15 | 29 | 36 | CD74HCT157, SCHS153C |
| 74HCT157 | /E → Y | 12 | 27 | 34 | CD74HCT157, SCHS153C (enable: outputs forced low) |
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
| AS6C1008 | address → DQ (tAA) | 55 | 55 | 55 | AS6C1008 v1.2, p. 5 (-55; one figure, 0–70 °C) |
| AS6C1008 | CE#, CE2 → DQ (tACE) | 55 | 55 | 55 | AS6C1008 v1.2, p. 5 |
| AS6C1008 | OE# → DQ (tOE) | 30 | 30 | 30 | AS6C1008 v1.2, p. 5 |
| AS6C1008 | write pulse WE# (tWP) | 45 | 45 | 45 | AS6C1008 v1.2, p. 5 (a minimum; tAS 0, tAW 50, tWR 0) |
| AS6C1008 | setup DIN → WE# rising (tDW) | 25 | 25 | 25 | AS6C1008 v1.2, p. 5 (a minimum; tDH 0) |
| AT28C256 | address → I/O (tACC) | 150 | 150 | 150 | AT28C256, Microchip DS20006386B, p. 11 (-15, −40–85 °C) |
| AT28C256 | /CE → I/O (tCE) | 150 | 150 | 150 | AT28C256, DS20006386B, p. 11 |
| AT28C256 | /OE → I/O (tOE) | 70 | 70 | 70 | AT28C256, DS20006386B, p. 11 |
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
| 74ACT10 | 74HCT10 | clock generator (C6) | the same, with the 74ACT74; also the replacement if the NRND CD74HCT10E can no longer be bought (SN74ACT10N, TI, PDIP-14, active 2026-11-05) |

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
| 74165 | yes | checked against cd74hct165.pdf (TI, truth table 7-1) by [circuits/chips/test_74165.circ](../circuits/chips/test_74165.circ): asynchronous parallel load while SH/LD is low, shift on CLK rising with CLK INH low, CLK ignored with CLK INH high. Not checked: CLK INH rising while CLK is low (an edge on the real chip; the mailbox doesn't depend on it). |
| 74595 | **no** | modelled: [circuits/chips/74595.circ](../circuits/chips/74595.circ) (circuit `ttl74595`), from sn74ahct595.pdf (TI SCLS374Q) |
| 74574 | **no** | modelled: [circuits/chips/74574.circ](../circuits/chips/74574.circ) (circuit `ttl74574`), from cd74hct574.pdf |

Logisim's 74670 was checked against the CD74HCT670 datasheet (cd74hc670.pdf, which covers HCT) by [circuits/chips/test_74670.circ](../circuits/chips/test_74670.circ): writes are transparent while /WE is low (the addressed word follows D), reads with /RE high are high impedance. It needs no model.

Chip models live in `circuits/chips/`, one file per part, with the datasheet next to them. Rules: chips.md "Rules", conventions.md "Components".

## Real 74HCT parts missing from Logisim

- **74HCT688** (8-bit equality comparator): would give A = B in one chip, but it would have needed a model (not allowed then). cmp8 uses two 74HCT85 instead, which give S, E and F, and keeps them (alu-decisions.md D15).
- **74HCT4078** (8-input NOR/OR): would give Z in one chip instead of 74HCT27 + 74HCT11. Not used: the 4078 is mostly sold as HC, and the 27 + 11 cost one chip more but no new model.
- **74HCT540** (inverting 541): the 74HCT240 does the same job here.
- **74HCT154** (4-to-16 decoder): would remove the 7411 gate behind the shift-right enable.
