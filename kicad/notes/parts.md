# Parts on the boards

Every chip and component on the 9 cards and the motherboard, counted from the KiCad netlists on 2026-11-15, with the references chosen so far. The PCB build reuses the chips tested on the breadboards. The breadboard orders, the test rig and the tools are in [../../logisim/notes/breadboard-order.md](../../logisim/notes/breadboard-order.md).

**Status:** **decided** = reference chosen; **to choose** = not chosen yet.

Sourcing rules (Gianni, 2026-11-04, full text in [../../logisim/notes/chips.md](../../logisim/notes/chips.md)):
- 74HCT parts only. The iod has two exceptions: the SN74AHCT595N and the SN74LVC244AN at 3.3 V.
- TI DIP references (SN74HCT…N, CD74HCT…E). Never a Nexperia DIP part, and never a TI NRND variant, except the 74HCT10.
- Buy from authorised distributors only.

## Chips per card (158)

| Part | DIP reference | flags | alu | regs | ir | addr | cu | mem | iod | clkgen | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 74HCT00 | CD74HCT00E | | | | | | 1 | | | | 1 |
| 74HCT08 | SN74HCT08N | | 3 | | | 2 | 2 | | | | 7 |
| 74HCT10 (NRND) | CD74HCT10E | | | | | | | | | 1 | 1 |
| 74HCT11 | CD74HCT11E | 3 | 2 | | | | | | | | 5 |
| 74HCT14 | SN74HCT14N | | | | 1 | | 1 | 1 | | 1 | 4 |
| 74HCT27 | CD74HCT27E | | 1 | | | | 1 | | | | 2 |
| 74HCT32 | SN74HCT32N | | 2 | 1 | | | | 1 | 1 | | 5 |
| 74HCT74 | SN74HCT74N | | | | | | | | 4 | 2 | 6 |
| 74HCT85 | CD74HCT85E | | 2 | | | | | | | | 2 |
| 74HCT86 | CD74HCT86E | 1 | 5 | | | 4 | | | | | 10 |
| 74HCT138 | SN74HCT138N | 2 | 1 | | | | 1 | | 1 | | 5 |
| 74HCT151 | CD74HCT151E | | | | | | 1 | | | | 1 |
| 74HCT153 | CD74HCT153E | 4 | 1 | 4 | | 8 | 1 | | | | 18 |
| 74HCT157 | CD74HCT157E | 2 | 2 | | | 4 | 3 | | | | 11 |
| 74HCT165 | CD74HCT165E | | | | | | | | 1 | | 1 |
| 74HCT175 | CD74HCT175E | 2 | | | | | 2 | | | | 4 |
| 74HCT240 | CD74HCT240E | | 1 | | | | | | | | 1 |
| 74HCT244 | CD74HCT244E | | | | | | | | 1 | | 1 |
| 74HCT283 | CD74HCT283E | | 3 | | | 4 | | | | | 7 |
| 74HCT377 | CD74HCT377E | | | | 6 | 6 | 1 | | | | 13 |
| 74HCT541 | SN74HCT541N | 1 | 6 | 2 | | 6 | 1 | 3 | | 1 | 20 |
| 74HCT574 | CD74HCT574E | | | 3 | | | 8 | | | | 11 |
| 74HCT670 | CD74HCT670E | | | 8 | | | | | | | 8 |
| 74AHCT595 | SN74AHCT595N | | | | | | | | 3 | | 3 |
| 74LVC244 (3.3 V) | SN74LVC244AN | | | | | | | | 1 | | 1 |
| EEPROM 32 K × 8, 150 ns | AT28C256-15PU | | | | | | 7 | 2 | | | 9 |
| SRAM 128 K × 8, 55 ns | AS6C1008-55PCN | | | | | | | 1 | | | 1 |
| **Chips** | | **15** | **29** | **18** | **7** | **34** | **30** | **8** | **12** | **5** | **158** |

These counts match the Logisim BOMs (`logisim/notes/bom-*.csv`: 146 for the CPU, plus 12 for the iod). The Mouser references and the spares are in breadboard-order.md.

## DIP sockets (one per chip)

| Socket | KiCad footprint | Qty | Cards | Status |
|---|---|---|---|---|
| DIP-14, 300 mil | `Package_DIP:DIP-14_W7.62mm_Socket` | 41 | every card | type to choose: turned-pin or open-frame |
| DIP-16, 300 mil | `DIP-16_W7.62mm_Socket` | 60 | addr 16, regs 12, flags 10, alu 9, cu 8, iod 5 | same |
| DIP-20, 300 mil | `DIP-20_W7.62mm_Socket` | 47 | addr 12, cu 10, alu 7, ir 6, regs 5, mem 3, iod 2, clkgen 1, flags 1 | same |
| DIP-28, 600 mil | `DIP-28_W15.24mm_Socket` | 9 | cu 7, mem 2 | same. The turned-pin pin savers bought for the ROMs (breadboard orders 6 and 7) can sit in them |
| DIP-32, 600 mil | `DIP-32_W15.24mm_Socket` | 1 | mem | same |

Buy about 10 % extra of each size. These parts get no socket: the resistor networks (soldered), the oscillator Y1, and the Arduino Nano, which sits on headers.

## Other components per card

| Part | Footprint | flags | alu | regs | ir | addr | cu | mem | iod | clkgen | Total | Status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 100 nF decoupling | `C_Rect_L7.0mm_W3.5mm_P2.50mm_P5.00mm` (all values) | 1 | 7 | 2 | | 6 | 8 | 6 | 9 | 4 | 43 | **decided:** 20 from the ceramic kit + 30 to buy, 5 mm pitch (7 spare) |
| 68 nF decoupling | same | | | 3 | 6 | 6 | 9 | | | 1 | 25 | **decided:** ceramic kit |
| 47 nF decoupling | same | 3 | 3 | 9 | 1 | 1 | 4 | 1 | 2 | 1 | 25 | **decided:** ceramic kit |
| 33 nF decoupling | same | 4 | 4 | 4 | | 12 | 1 | | | | 25 | **decided:** ceramic kit |
| 22 nF decoupling | same | 4 | 8 | | | 8 | 4 | | 1 | | 25 | **decided:** ceramic kit |
| 10 nF decoupling | same | 4 | 8 | 1 | 1 | 2 | 5 | 2 | 1 | 1 | 25 | **decided:** ceramic kit |
| 47 µF 16 V electrolytic (bulk) | `CP_Radial_D5.0mm_P2.00mm` | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 9 | to choose; footprint provisional |
| 10 µF 16 V electrolytic (reset RC) | `CP_Radial_D5.0mm_P2.00mm` | | | | | | | | | 1 | 1 | to choose |
| 10 kΩ × 8 bussed network, SIP-9 | `R_Array_SIP9` | | | | | | 8 | | | | 8 | **decided:** Bourns 4609X-101-103LF |
| 33 Ω, ¼ W | `R_Axial_DIN0207_…P10.16mm` | | | | | | | | 4 | 1 | 5 | to choose |
| 220 Ω, ¼ W | same | | | | | | | | 1 | 1 | 2 | to choose |
| 10 kΩ, ¼ W | same | | | | | | | | | 3 | 3 | to choose (breadboard: Yageo MFR-25FBF52-10K) |
| 100 Ω, ¼ W | same | | | | | | | | | 1 | 1 | to choose (breadboard: Yageo MFR-25FBF52-100R) |
| 1N5819 Schottky | `D_DO-41_SOD81_P10.16mm` | | | | | | | | 1 | 1 | 2 | **owned** (10 in stock). Same family as the 1N5817, about 50 mV more drop at our currents (TTGO still gets about 4.55 V) |
| 1N4148 | `D_DO-35_SOD27_P7.62mm` | | | | | | | | | 1 | 1 | to choose (breadboard: onsemi 1N4148) |
| Card edge (gold fingers) | `Connector_PCBEdge:BUS_PCIexpress_xN` | x4 | x8 | x16 | x16 | x16 | x16 | x8 | x4 | x1 | 9 | **decided:** part of the PCB |

**Decoupling values (2026-11-15).** One cap per chip, one beside the 47 µF at the card edge, one for Y1: 168 in all. The value is set per chip from what Gianni has: an unbranded ceramic disc kit (50 V, 5 mm pitch, probably Y5V) plus 30× 100 nF to buy. The footprint takes both 2.5 mm and 5 mm pitch. Each chip gets at least its tier's minimum (the clock is 2 MHz):

- 100 nF minimum: AT28C256, AS6C1008, Y1. Also on 100 nF: all 74HCT541, the 595s (AHCT), 244s (one is LVC), the 240 and the 74HCT74s.
- 47 nF minimum (octal outputs and the card-edge cap): 574 and 377 on 68 nF (plus the clkgen edge cap); 670, 175, 85, 151, 165, clkgen's 74HCT14 and the other edge caps on 47 nF.
- 22 nF minimum, 10 nF for gates: 153 and 283 on 33 nF; 157, 138 and 86 on 22 nF; the gates (08, 32, 11, 27, 14, 00, 10, and the flags' 86) on 10 nF.

### clkgen only

| Ref | Part | Footprint | Status |
|---|---|---|---|
| Y1 | 4.000 MHz oscillator, 5 V HCMOS, half-size DIP-8 | `Oscillator_DIP-8` | **decided:** ECS-2200BX-040 |
| A1 | Arduino Nano v3 | `Module:Arduino_Nano` | **owned**. Mount it on 2 × 1×15 female headers (to choose) |
| RV1 | 10 kΩ potentiometer | `Potentiometer_Alps_RK09K_Single_Vertical` | to choose |
| SW1–SW3 | 6 mm tactile buttons: STEP, RUN/STOP, RESET | `SW_PUSH_6mm` | to choose (breadboard: Omron B3F-1000) |
| JP1 | 1 × 3 pin header + jumper | `PinHeader_1x03_P2.54mm_Vertical` | to choose |
| TP1–TP3 | test points `clkgen_osc`, `clk_5`, `rst_n` | `TestPoint_THTPad_D1.5mm_Drill0.7mm` | pads only, nothing to buy |

### iod only: the TTGO cables

| Ref | Part | Footprint | Status |
|---|---|---|---|
| J2 | 2 × 4 pin header, 2.54 mm, vertical + 8 Dupont female-female wires (10–15 cm) | `Connector_PinHeader_2.54mm:PinHeader_2x04_P2.54mm_Vertical` | **decided** (2026-11-15); header to choose |
| J4 | 1 × 5 pin header, vertical + a PS/2 (mini-DIN 6) male cable, cut, with Dupont female crimps | `Connector_PinHeader_2.54mm:PinHeader_1x05_P2.54mm_Vertical` | **decided**; cable and crimps to choose |
| J1 | 1 × 4 pin header, vertical + 3 wires and a GND wire, soldered on the TTGO, Dupont female at the card | `Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical` | **decided** |
| — | LilyGO TTGO VGA32 **v1.4** | off the card, in its own box | **to buy** (not owned yet) |

## Motherboard

| Ref | Part | Value | Footprint | Status |
|---|---|---|---|---|
| J2, J4, J6, J8, J10, J3, J5, J7, J9 | PCIe slots, through-hole, vertical (see below) | | `hbc2_slots:PCIe_xN_Slot_Amphenol_…` | **decided** |
| RN1 | 10 kΩ × 8 bussed network, SIP-9 (A8–A15 pull-downs) | | `Resistor_THT:R_Array_SIP9` | **decided:** Bourns 4609X-101-103LF |
| J1 | DC barrel jack, 2.1 × 5.5 mm, centre + | | `BarrelJack_CUI_PJ-102AH_Horizontal` | **decided:** CUI PJ-102AH. Check that it fits the GST25E05's plug |
| SW1 | Power switch: a rocker on the case, ≥ 3 A, wired to a 2-pin JST XH header | | `JST_XH_B2B-XH-A_1x02_P2.50mm_Vertical` | rocker to choose. JST B2B-XH-A + XHP-2 housing + crimps |
| F1 | 5 × 20 mm fuse + 2 PCB clips | T 3.15 A | `Fuseholder_Clip-5x20mm_Keystone_3517_…` | **decided:** Keystone 3517 clips. Buy spare fuses |
| D1 | Unidirectional TVS diode, drawn with the Zener symbol | P6KE6V8A | `D_DO-15_P12.70mm_Horizontal` | **decided** |
| C1 | Electrolytic capacitor, 10 mm can, 5 mm pitch | 1000 µF 10 V | `CP_Radial_D10.0mm_P5.00mm` | to choose |

The power input design is in [design-notes.md](design-notes.md), "Power (S1)".

### PCIe card slots (decided 2026-11-14)

| Size | Contacts | Cards | Qty | Spares | Amphenol FCI (Mouser) |
|---|---|---|---|---|---|
| x1 | 36 | clkgen | 1 | 1 | 10018783-10010TLF |
| x4 | 64 | flags, iod | 2 | 1 | 10018783-11101TLF |
| x8 | 98 | alu, mem | 2 | 1 | 10018783-11112TLF |
| x16 | 164 | registers, ir, addr, cu | 4 | 1 | 10018783-10013TLF |

- **Footprints:** `kicad/lib/hbc2_slots.pretty`, written by `lib/gen_pcie_slots.py`. The x1 comes from SamacSys. The x4, x8 and x16 are extrapolated: check their peg holes against the datasheet or a real slot before you order the motherboard. Details are in [slot-pinout.md](slot-pinout.md), "Footprints".
- **What to check on a slot:**
  - vertical, through-hole with **solder tails** (not SMT, not press-fit);
  - for a 1.57 mm card, 1.0 mm pitch;
  - the same family for every size, so pin 1 lines up.
- **Soldering:**
  - Iron at 330–350 °C, about 370 °C or a bigger tip on the ground pins. Use a 1–1.6 mm chisel tip, 0.5 mm solder and flux.
  - Seat the slot, solder two opposite corners, check it sits flat and square, then solder the rest.
  - **No hot air:** it can warp the housing.
  - A 164-pin slot is very hard to remove by hand.
- **Test first:** solder one slot, then plug in a bare test card to check the fit and the height of the case's guide rails.

## PCB order options

- **Cards:** 1.6 mm thick, with **hard-gold, bevelled gold fingers** (the JLCPCB / PCBWay option). Plain ENIG is enough for a few dozen insertions.
- **Motherboard:** 4 layers, recommended in [slot-pinout.md](slot-pinout.md), "Tracks across the slots".

## Power supply and case

| Part | Status |
|---|---|
| Mean Well GST25E05 (5 V 4 A) | **decided**; bought with breadboard order 1. Budget: about 0.6 A typical, 1.6 A worst case |
| Mean Well RS-25-5 (adjustable) | only if the farthest card reads below 4.9 V under load |
| Case | 3D-printed, guide rails about 20 mm apart, a top or back bracket; the TTGO goes in its own box |
