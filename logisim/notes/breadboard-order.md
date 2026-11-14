# Breadboard orders: one card at a time

The CPU is tested **one card at a time** on breadboards with the test rig ([test-rig.md](test-rig.md)), then built on **through-hole (DIP) PCBs with sockets**: the tested chips move onto the PCBs, so no SOIC set is bought. Chips come from [bom-cpu.csv](bom-cpu.csv) and the per-card files `bom-<card>.csv` (146 chips, 8 resistor networks, 2026-11-10).

The purchase is split into successive orders, one per card in the build order below, so it can be bought progressively. Each order lists only what wasn't bought before:
- **chips:** what the card needs (each card keeps its chips for its PCB), plus a part's spares with the first card that uses it. Spares: the `spare` column of bom-cpu.csv (10–20 % on the common parts, at least one of each), except the 74HCT10: **3 spares** (NRND, chips.md);
- **breadboards, wire, decoupling, tools:** bought once and reused. A tested card is taken down and its chips bagged and labelled until its PCB; only the clock card stays built.

Rules followed ([chips.md](chips.md), "Rules"): TI DIP parts for every 74HCT chip, never a Nexperia `N` part, no NRND TI variant except the 74HCT10, authorised distributors only. One supplier reference per line, **Mouser** unless stated. The rig's shift registers aren't CPU parts, so the 74HCT rule doesn't bind them (test-rig.md).

**Prices are estimates** (2026-11-10, € excluding VAT, at the quantity of the line), from the Octopart survey of [disponibilite-74hct.md](disponibilite-74hct.md) and distributor pages. Check stock and that each reference is still active in the basket. Mouser delivers free to France above about 50 €: orders 3 and 4 are below it, so buy them together.

## Build order

| Order | Card | Chips | Why here |
|---|---|---|---|
| 1 | rig + **clkgen** + **ir** | 5 + 7 | The rig first. clkgen is small and stays built. ir is small and sequential: it proves the rig, the clock pins and the replay on 7 chips. |
| 2 | **alu** | 29 | Combinational, exhaustive vectors: the simplest big card. |
| 3 | **flags** | 15 | Small; first bus on a joined wire (`bus_data`). |
| 4 | **regs** | 18 | The 670s; buses A, B and `bus_data`. |
| 5 | **addr** | 34 | The biggest datapath card (59 of the 64 D bits). |
| 6 | **mem** | 8 | First EEPROMs: the programmer is built here, on 2 ROMs. |
| 7 | **cu** | 30 | Last: needs the 7 microcode EEPROMs (programmer proven on mem), the most outputs (63 S bits), the most boards. |
| 8 | optional: partial integration | – | clkgen + cu + addr at full speed, for what the rig can't test (breadboard-checks.md B1, B3). |

## Running total

| Order | Content | Order € | Running total € |
|---|---|---|---|
| 1 | rig, supply, wire, tools, clkgen, ir | 271.75 | 271.75 |
| 2 | alu | 75.50 | 347.25 |
| 3 | flags | 14.15 | 361.40 |
| 4 | regs | 27.15 | 388.55 |
| 5 | addr | 54.80 | 443.35 |
| 6 | mem | 73.40 | 516.75 |
| 7 | cu | 128.25 | **645.00** |
| 8 | optional integration + debug LEDs | 78.80 | 723.80 |

**About 645 € excluding VAT for every card tested (about 775 € with 20 % VAT), 725 € with the optional integration**, against 880 € for the whole CPU on breadboards at once: 14 breadboards instead of 40, and no debug LEDs until the integration (the rig reads every output).

Breadboards owned: 7 after order 1 (rig 4, clkgen 1, ir 2), 11 after order 2, 12 after order 5, 14 after order 7, 17 with order 8.

## Order 1: test rig, common parts, clkgen, ir

### Rig

| Item | Part | Buy | Supplier reference | €/unit | Line € |
|---|---|---|---|---|---|
| Drive chain: 8-bit shift register, 3-state latched outputs (8 + 1 spare) | TI SN74AHCT595N | 9 | Mouser 595-SN74AHCT595N | 0.80 | 7.20 |
| Sense chain: 8-bit parallel-in shift register (8 + 1) | TI CD74HCT165E | 9 | Mouser 595-CD74HCT165E | 1.00 | 9.00 |
| 470 Ω × 8, isolated, DIP-16, in series with the drive outputs (8 + 1) | Bourns 4116R-1-471LF | 9 | Mouser 652-4116R-1LF-470 | 0.90 | 8.10 |
| 100 kΩ × 8, bussed, SIP-9, sense pulls switched by PULL (8 + 1) | Bourns 4609X-101-104LF | 9 | Mouser 652-4609X-1LF-100K | 0.45 | 4.05 |
| 100 Ω, ¼ W, in series with the C pins | Yageo MFR-25FBF52-100R | 10 | Mouser 603-MFR-25FBF52-100R | 0.08 | 0.80 |
| Male / male jumper wires, 40 × 150 mm (Mega → rig, rig → card) | Adafruit 758 | 3 | Mouser 485-758 | 4.00 | 12.00 |
| | | | | **Rig** | **41.15** |

The Arduino Mega 2560 is already owned. TI makes no 74HCT595 in DIP, hence the AHCT part (test-rig.md).

### Common parts (reused by every card)

| Item | Part | Buy | Supplier reference | €/unit | Line € |
|---|---|---|---|---|---|
| Breadboard, 830 points: rig 4, clkgen 1, ir 2 | BusBoard BB830 | 7 | Mouser 854-BB830 | 9.00 | 63.00 |
| Hook-up wire, 22 AWG solid, 6 colours × 7.6 m | Adafruit 1311 | 3 sets | Mouser 485-1311 | 15.00 | 45.00 |
| 5 V 4 A (20 W) adapter, EU plug, 2.1 mm barrel | Mean Well GST25E05-P1J | 1 | Mouser 709-GST25E05-P1J | 15.00 | 15.00 |
| Barrel jack, 2.1 mm | CUI PJ-102AH | 2 | Mouser 490-PJ-102AH | 1.00 | 2.00 |
| Decoupling 100 nF, 50 V, X7R, radial 2.54 mm | KEMET C320C104K5R5TA | 100 | Mouser 80-C320C104K5R | 0.15 | 15.00 |
| Bulk 10 µF, 25 V, one per breadboard (and the reset C) | Panasonic ECA-1EM100 | 25 | Mouser 667-ECA-1EM100 | 0.15 | 3.75 |
| Bulk 470 µF, 16 V, at the supply input | Panasonic ECA-1CM471 | 2 | Mouser 667-ECA-1CM471 | 0.40 | 0.80 |
| Wire stripper, 22–30 AWG solid | Klein Tools 11047 | 1 | Mouser 247-11047 | 25.00 | 25.00 |
| Flush cutter | Xuron 170-II | 1 | Mouser, manufacturer number 170-II | 10.00 | 10.00 |
| DIP extractor, 8–24 pins (logic chips) | Jonard EX-1 | 1 | Mouser, manufacturer number EX-1 | 8.00 | 8.00 |
| DIP extractor, 24–40 pins (EEPROMs, SRAM) | Jonard EX-2 | 1 | Mouser, manufacturer number EX-2 | 12.00 | 12.00 |
| | | | | **Common** | **199.55** |

100 decoupling capacitors cover the largest set built at once: the rig (16), clkgen (5) and the biggest card (addr, 34), with order 8's 75 as well. A multimeter is assumed to be on hand; the oscilloscope is needed from the clkgen check on (breadboard-checks.md).

**Current:** the rig and one card draw at most about 0.6 A (the cu, whose seven EEPROMs are always selected: 350 mA max). The 4 A adapter also covers the integration of order 8 (about 1 A worst case). Don't power anything from the Mega's 5 V pin.

### clkgen: off-the-logic parts

| Item | Part | Buy | Supplier reference | €/unit | Line € |
|---|---|---|---|---|---|
| 4.000 MHz oscillator, 5 V HCMOS, half-size DIP-8 can, 45–55 % duty (Q9) | ECS-2200BX-040 (chosen 2026-11-14; was ECS-2100AX-040) | 2 | Mouser, manufacturer number ECS-2200BX-040 | 3.00 | 6.00 |
| Reset R (10 kΩ) and spare pull-ups | Yageo MFR-25FBF52-10K | 25 | Mouser 603-MFR-25FBF52-10K | 0.08 | 2.00 |
| Reset diode across R | onsemi 1N4148 | 5 | Mouser 512-1N4148 | 0.10 | 0.50 |
| Reset push button | Omron B3F-1000 | 5 | Mouser 653-B3F-1000 | 0.30 | 1.50 |
| Series resistor for the button (100 Ω) | Yageo MFR-25FBF52-100R | 10 | Mouser 603-MFR-25FBF52-100R | 0.08 | 0.80 |
| | | | | **clkgen parts** | **10.80** |

ECS gives 45–55 % symmetry at 50 % VDD for 1–80 MHz. Octopart marks the part NRND, DigiKey active: check it in the basket (alternative: any 5 V HCMOS half-size can at 45–55 %). Reset RC (confirmed 2026-11-14, on the KiCad clkgen board): 10 kΩ from VCC, 10 µF to GND (τ = 100 ms; the reset C is one of the bulk 10 µF above), 1N4148 across R (cathode to VCC), button across C through 100 Ω. The datasheet on file (`circuits/chips/ecs_2100x.pdf`) covers the ECS-2100X only: confirm the 2200BX is 5 V in the basket.

### Chips: clkgen + ir

| Part | DIP reference | For the cards | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT10 (**NRND**) | CD74HCT10E | 1 | 3 | 4 | 595-CD74HCT10E | 0.90 | 3.60 |
| 74HCT14 | SN74HCT14N | 2 | 1 | 3 | 595-SN74HCT14N | 0.75 | 2.25 |
| 74HCT74 | SN74HCT74N | 2 | 1 | 3 | 595-SN74HCT74N | 0.80 | 2.40 |
| 74HCT377 | CD74HCT377E | 6 | 2 | 8 | 595-CD74HCT377E | 1.10 | 8.80 |
| 74HCT541 | SN74HCT541N | 1 | 3 | 4 | 595-SN74HCT541N | 0.80 | 3.20 |
| | | **12** | | **22** | | | **20.25** |

**Order 1 total: 41.15 + 199.55 + 10.80 + 20.25 = 271.75 €.**

## Order 2: alu

| Part | DIP reference | For the card | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT08 | SN74HCT08N | 3 | 2 | 5 | 595-SN74HCT08N | 0.70 | 3.50 |
| 74HCT11 | CD74HCT11E | 2 | 1 | 3 | 595-CD74HCT11E | 0.90 | 2.70 |
| 74HCT27 | CD74HCT27E | 1 | 1 | 2 | 595-CD74HCT27E | 0.90 | 1.80 |
| 74HCT32 | SN74HCT32N | 2 | 1 | 3 | 595-SN74HCT32N | 0.75 | 2.25 |
| 74HCT85 | CD74HCT85E | 2 | 1 | 3 | 595-CD74HCT85E | 1.00 | 3.00 |
| 74HCT86 | CD74HCT86E | 5 | 2 | 7 | 595-CD74HCT86E | 0.80 | 5.60 |
| 74HCT138 | SN74HCT138N | 1 | 1 | 2 | 595-SN74HCT138N | 0.75 | 1.50 |
| 74HCT153 | CD74HCT153E | 1 | 3 | 4 | 595-CD74HCT153E | 0.95 | 3.80 |
| 74HCT157 | CD74HCT157E | 2 | 2 | 4 | 595-CD74HCT157E | 0.85 | 3.40 |
| 74HCT240 | CD74HCT240E | 1 | 1 | 2 | 595-CD74HCT240E | 1.20 | 2.40 |
| 74HCT283 | CD74HCT283E | 3 | 2 | 5 | 595-CD74HCT283E | 0.95 | 4.75 |
| 74HCT541 | SN74HCT541N | 6 | – | 6 | 595-SN74HCT541N | 0.80 | 4.80 |
| | | **29** | | **46** | | | **39.50** |

Plus **4 × BB830** (854-BB830, 36.00 €): the alu takes 6 boards, 2 are free once ir is taken down (rig 4 and clkgen 1 stay).

**Order 2 total: 75.50 €.**

## Order 3: flags

| Part | DIP reference | For the card | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT11 | CD74HCT11E | 3 | – | 3 | 595-CD74HCT11E | 0.90 | 2.70 |
| 74HCT86 | CD74HCT86E | 1 | – | 1 | 595-CD74HCT86E | 0.80 | 0.80 |
| 74HCT138 | SN74HCT138N | 2 | – | 2 | 595-SN74HCT138N | 0.75 | 1.50 |
| 74HCT153 | CD74HCT153E | 4 | – | 4 | 595-CD74HCT153E | 0.95 | 3.80 |
| 74HCT157 | CD74HCT157E | 2 | – | 2 | 595-CD74HCT157E | 0.85 | 1.70 |
| 74HCT175 | CD74HCT175E | 2 | 1 | 3 | 595-CD74HCT175E | 0.95 | 2.85 |
| 74HCT541 | SN74HCT541N | 1 | – | 1 | 595-SN74HCT541N | 0.80 | 0.80 |
| | | **15** | | **16** | | | **14.15** |

3 boards, already owned. **Order 3 total: 14.15 €** (below the free-delivery threshold: buy with order 4).

## Order 4: regs

| Part | DIP reference | For the card | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT32 | SN74HCT32N | 1 | – | 1 | 595-SN74HCT32N | 0.75 | 0.75 |
| 74HCT153 | CD74HCT153E | 4 | – | 4 | 595-CD74HCT153E | 0.95 | 3.80 |
| 74HCT541 | SN74HCT541N | 2 | – | 2 | 595-SN74HCT541N | 0.80 | 1.60 |
| 74HCT574 | CD74HCT574E | 3 | 2 | 5 | 595-CD74HCT574E | 1.00 | 5.00 |
| 74HCT670 | CD74HCT670E | 8 | 2 | 10 | 595-CD74HCT670E | 1.60 | 16.00 |
| | | **18** | | **22** | | | **27.15** |

4 boards, already owned. **Order 4 total: 27.15 €.**

## Order 5: addr

| Part | DIP reference | For the card | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT08 | SN74HCT08N | 2 | – | 2 | 595-SN74HCT08N | 0.70 | 1.40 |
| 74HCT86 | CD74HCT86E | 4 | – | 4 | 595-CD74HCT86E | 0.80 | 3.20 |
| 74HCT153 | CD74HCT153E | 8 | – | 8 | 595-CD74HCT153E | 0.95 | 7.60 |
| 74HCT157 | CD74HCT157E | 4 | – | 4 | 595-CD74HCT157E | 0.85 | 3.40 |
| 74HCT283 | CD74HCT283E | 4 | – | 4 | 595-CD74HCT283E | 0.95 | 3.80 |
| 74HCT377 | CD74HCT377E | 6 | – | 6 | 595-CD74HCT377E | 1.10 | 6.60 |
| 74HCT541 | SN74HCT541N | 6 | – | 6 | 595-SN74HCT541N | 0.80 | 4.80 |
| | | **34** | | **34** | | | **30.80** |

Plus **1 × BB830** (9.00 €: 7 boards for the card) and **1 set of wire** (Adafruit 1311, 15.00 €).

**Order 5 total: 54.80 €.**

## Order 6: mem, and the EEPROM programmer

| Part | DIP reference | For the card | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT14 | SN74HCT14N | 1 | – | 1 | 595-SN74HCT14N | 0.75 | 0.75 |
| 74HCT32 | SN74HCT32N | 1 | – | 1 | 595-SN74HCT32N | 0.75 | 0.75 |
| AT28C256, 32 K × 8 EEPROM, 150 ns | AT28C256-15PU | 2 | 2 | 4 | 556-AT28C25615PU | 9.50 | 38.00 |
| AS6C1008, 128 K × 8 SRAM, 55 ns | AS6C1008-55PCN | 1 | 1 | 2 | 913-AS6C1008-55PCN | 4.50 | 9.00 |
| 74HCT541 | SN74HCT541N | 3 | – | 3 | 595-SN74HCT541N | 0.80 | 2.40 |
| | | **8** | | **11** | | | **50.90** |

| Item | Part | Buy | Supplier reference | €/unit | Line € |
|---|---|---|---|---|---|
| 28-pin ZIF socket, 600 mil (programmer) | Aries 28-6554-10 | 1 | Farnell 1169110 | 15.00 | 15.00 |
| 28-pin turned-pin socket, pin saver under each ROM (2 + 1) | Mill-Max 110-43-628-41-001000 | 3 | Mouser 575-11043628 | 2.50 | 7.50 |
| | | | | | **22.50** |

The programmer is the owned Mega with the ZIF on one breadboard (from the boards owned), on pins the rig doesn't use (test-rig.md). The ZIF is the only Farnell line; Mouser probably carries it under Aries' `535-` prefix (not checked), which would save a second delivery. mem takes 3 boards + 1 for the programmer, already owned.

**Order 6 total: 73.40 €.**

## Order 7: cu

| Part | DIP reference | For the card | Spares (first use) | Buy | Mouser | €/unit | Line € |
|---|---|---|---|---|---|---|---|
| 74HCT00 | CD74HCT00E | 1 | 1 | 2 | 595-CD74HCT00E | 0.65 | 1.30 |
| 74HCT08 | SN74HCT08N | 2 | – | 2 | 595-SN74HCT08N | 0.70 | 1.40 |
| 74HCT14 | SN74HCT14N | 1 | – | 1 | 595-SN74HCT14N | 0.75 | 0.75 |
| 74HCT27 | CD74HCT27E | 1 | – | 1 | 595-CD74HCT27E | 0.90 | 0.90 |
| AT28C256 (microcode) | AT28C256-15PU | 7 | – | 7 | 556-AT28C25615PU | 9.50 | 66.50 |
| 74HCT138 | SN74HCT138N | 1 | – | 1 | 595-SN74HCT138N | 0.75 | 0.75 |
| 74HCT151 | CD74HCT151E | 1 | 1 | 2 | 595-CD74HCT151E | 0.90 | 1.80 |
| 74HCT153 | CD74HCT153E | 1 | – | 1 | 595-CD74HCT153E | 0.95 | 0.95 |
| 74HCT157 | CD74HCT157E | 3 | – | 3 | 595-CD74HCT157E | 0.85 | 2.55 |
| 74HCT175 | CD74HCT175E | 2 | – | 2 | 595-CD74HCT175E | 0.95 | 1.90 |
| 74HCT377 | CD74HCT377E | 1 | – | 1 | 595-CD74HCT377E | 1.10 | 1.10 |
| 74HCT541 | SN74HCT541N | 1 | – | 1 | 595-SN74HCT541N | 0.80 | 0.80 |
| 74HCT574 | CD74HCT574E | 8 | – | 8 | 595-CD74HCT574E | 1.00 | 8.00 |
| Pull resistors (C24): 10 kΩ × 8, bussed, SIP-9 | Bourns 4609X-101-103LF | 8 | 1 | 9 | 652-4609X-1LF-10K | 0.45 | 4.05 |
| | | **30 + 8 networks** | | | | | **92.75** |

Plus **2 × BB830** (18.00 €: 9 boards for the card, with the rig and clkgen 14 in all) and **7 turned-pin sockets** for the microcode EEPROMs (575-11043628, 17.50 €).

The two EEPROM spares came with order 6. The pull-resistor networks: Mouser's number follows Bourns' format and wasn't checked; Farnell lists the part as 9356819.

**Order 7 total: 128.25 €.**

## Order 8 (optional): partial integration and debug LEDs

clkgen + cu + addr built together at full speed (17 boards; the rig is taken down), to measure on the scope what the rig can't: breadboard-checks.md B1 (control register → 377 hold times) and B3 (reset: pulled levels, no contention, PC = 0000 and SP = FE00 after the reset). The rest of the CPU is left out, so its control lines are only observed.

| Item | Part | Buy | Supplier reference | €/unit | Line € |
|---|---|---|---|---|---|
| Breadboard (17 boards in all) | BusBoard BB830 | 3 | Mouser 854-BB830 | 9.00 | 27.00 |
| Wire | Adafruit 1311 | 1 set | Mouser 485-1311 | 15.00 | 15.00 |
| 10-segment red bar graph, DIP-20 | Kingbright DC-10EWA | 8 | Mouser 604-DC-10EWA | 2.00 | 16.00 |
| 1 kΩ × 8 bussed SIP-9, bar graph cathodes to GND | Bourns 4609X-101-102LF | 10 | Mouser 652-4609X-1LF-1K | 0.45 | 4.50 |
| 3 mm red LED (single lines) | Kingbright WP710A10SRD | 50 | Mouser 604-WP710A10SRD | 0.15 | 7.50 |
| 1 kΩ, ¼ W | Yageo MFR-25FBF52-1K | 100 | Mouser 603-MFR-25FBF52-1K | 0.04 | 4.00 |
| LED drivers, between a bus and its bar graph | SN74HCT541N | 6 | Mouser 595-SN74HCT541N | 0.80 | 4.80 |
| | | | | | **78.80** |

**Drive the LEDs from buffers, not from the CPU's nets.** On the control lines a 3 mA LED fights the 10 kΩ pull resistors while the pipeline 574s are off (C24, B3); on a tri-state bus it holds the bus low when nothing drives it and adds a load `timing.py` doesn't model. The bar graphs go behind the six extra 541s; single LEDs only on always-driven nets (`clk`, `rst_n`, the clock generator outputs).

## Not included

- The **PCBs** and their DIP **sockets** (one per chip, 146 + the 8 resistor networks): ordered with the PCBs, once the connector pinout is decided.
- The IOD (still a stub).
- The replay firmware and `tools/replay.py` (test-rig.md, "Replay"), the programmer firmware, and the vector exports for clkgen and the mem ROM pattern.
- Multimeter, oscilloscope, USB cable for the Mega.
- The 74ACT fallbacks of chips.md.
