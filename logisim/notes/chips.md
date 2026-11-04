# Chips: what to buy, how fast they are, what Logisim has

The parts used or considered for the HBC-2, so the next module doesn't have to search again. Pinouts and wiring choices are in [ttl-chips.md](ttl-chips.md). Stock and prices (Octopart, 2026-11-04): [disponibilite-74hct.md](disponibilite-74hct.md).

Two tools read the tables below, so keep their column names:
- `tools/bom.py` reads **Parts in use** and writes a bill of materials, such as [bom-alu.csv](bom-alu.csv). It fails if a circuit uses a chip that isn't listed there.
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

| Part | Function | Logisim | DIP | SOIC |
|---|---|---|---|---|
| 74HCT02 | quad 2-input NOR | 7402 | CD74HCT02E | SN74HCT02D or 74HCT02D (Nexperia) |
| 74HCT08 | quad 2-input AND | 7408 | SN74HCT08N | 74HCT08D (Nexperia) |
| 74HCT11 | triple 3-input AND | 7411 | CD74HCT11E | 74HCT11D (Nexperia) |
| 74HCT27 | triple 3-input NOR | 7427 | CD74HCT27E | 74HCT27D (Nexperia) |
| 74HCT32 | quad 2-input OR | 7432 | SN74HCT32N | 74HCT32D (Nexperia) |
| 74HCT85 | 4-bit magnitude comparator | 7485 | CD74HCT85E | CD74HCT85M or CD74HCT85M96 (TI) |
| 74HCT86 | quad 2-input XOR | 7486 | CD74HCT86E | 74HCT86D (Nexperia) |
| 74HCT138 | 3-to-8 decoder | 74138 | SN74HCT138N | 74HCT138D (Nexperia) |
| 74HCT153 | dual 4-to-1 multiplexer | 74153 | CD74HCT153E | 74HCT153D (Nexperia) |
| 74HCT157 | quad 2-to-1 multiplexer | 74157 | CD74HCT157E | 74HCT157D (Nexperia) |
| 74HCT240 | octal inverting buffer, tri-state | 74240 | CD74HCT240E | 74HCT240D (Nexperia) |
| 74HCT283 | 4-bit binary adder | 74283 | CD74HCT283E | CD74HCT283M96 (TI) |
| 74HCT541 | octal buffer, tri-state | 74541 | SN74HCT541N | 74HCT541D (Nexperia) |

The 74HCT02 is no longer used by the ALU (its S gate went with cmp8), but it stays listed for later sections. All DIP references are TI and active. The 74HCT85 was added on 2026-11-04 for cmp8: TI lists the CD74HCT85 as active, with about 1,400 CD74HCT85E at Mouser and 6,300 at DigiKey (aggregator figures, check in the basket). For the 74HCT283 in SOIC, take the TI part: the Nexperia 74HCT283D is discontinued. The 74HCT27 is as easy to buy as the others (CD74HCT27E: about 1,300 at Mouser).

## Delays

From the TI datasheets of the DIP references above, CL = 50 pF. **Max** columns are at VCC = 4.5 V, at 25 °C and over −40…85 °C. **Typ** is at 25 °C, but at the conditions each datasheet uses:
- SN74HCT parts (08, 32, 138, 541) give it at 4.5 V and 50 pF;
- CD74HCT parts give it only at **5 V and 15 pF**, which is quicker than 4.5 V and 50 pF.

So Typ is a rough guide only. Use Max 85 °C for timing budgets.

| Part | Path | Typ | Max 25 °C | Max 85 °C | Datasheet |
|---|---|---|---|---|---|
| 74HCT02 | A/B → Y | 8 | 21 | 26 | CD74HCT02, SCHS400 |
| 74HCT08 | A/B → Y | 15 | 24 | 30 | SN74HCT08, SCLS063G |
| 74HCT11 | A/B/C → Y | 11 | 28 | 35 | CD74HCT11, SCHS405 |
| 74HCT27 | A/B/C → Y | 9 | 23 | 29 | CD74HCT27, SCHS406 |
| 74HCT32 | A/B → Y | 15 | 24 | 30 | SN74HCT32, SCLS064G |
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
| 74HCT240 | A → Y | 9 | 22 | 28 | CD74HCT240, SCHS167G |
| 74HCT240 | OE → Y | 12 | 30 | 38 | CD74HCT240, SCHS167G (typ: HC value) |
| 74HCT283 | Cin → S0 | 13 | 31 | 39 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → S1 | 18 | 43 | 54 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → S2 | 19 | 46 | 58 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → S3 | 22 | 53 | 66 | CD74HCT283, SCHS176E |
| 74HCT283 | Cin → C4 | 19 | 46 | 58 | CD74HCT283, SCHS176E |
| 74HCT283 | A/B → C4 | 20 | 48 | 60 | CD74HCT283, SCHS176E |
| 74HCT283 | A/B → S | 21 | 49 | 61 | CD74HCT283, SCHS176E |
| 74HCT541 | A → Y | 13 | 23 | 29 | SN74HCT541, SCLS306E |
| 74HCT541 | OE → Y | 21 | 30 | 38 | SN74HCT541, SCLS306E |

Notes:
- The CD74HCT153 datasheet lists "I to Y" twice for the HCT types, at 24/30 ns (typ 9) and at 34/43 ns (typ 14). The table keeps the slower one.
- The CD74HCT85 datasheet names its last row "(A > B) IN to (A = B) OUT". It is read here as the cascade path to A=B.
- The CD74HCT240 gives no HCT typical for the enable time. 12 ns is the HC typical.
- At 5 V instead of 4.5 V, everything is about 10 % faster. Every Max 85 °C figure is about 1.25 × the Max 25 °C one.
- The Nexperia SOIC parts are in the same range, but they are specified differently (typ at 4.5 V and 50 pF). Check their datasheet before relying on a SOIC timing.

## In Logisim but not a 74HCT part (don't use)

- **74181 / 74182** (4-bit ALU / carry lookahead): sold only as LS/F/S TTL, which are obsolete. No HCT version found. Two 74181s would have done add, sub, the logic operations and A = B on their own.
- **74381** (4-bit ALU): S/LS only.
- Some parts can't be checked from their Logisim name alone (7413, 7418, 7419, 7421, 7451, 7454, 74670, 747266). Check them before use.

## Real 74HCT parts missing from Logisim

- **74HCT688** (8-bit equality comparator): would give A = B in one chip, but it would have needed a model (not allowed then). cmp8 uses two 74HCT85 instead, and keeps them, which give S, E and F (alu-decisions.md D15).
- **74HCT4078** (8-input NOR/OR): would give Z in one chip instead of 74HCT27 + 74HCT11. Not used: the 4078 is mostly sold as HC, and the 27 + 11 cost one chip more but no new model.
- **74HCT540** (inverting 541): the 74HCT240 does the same job here.
- **74HCT154** (4-to-16 decoder): would remove the 7411 gate behind the shift-right enable.
