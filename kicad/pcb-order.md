# PCB stage: parts to buy (started 2026-11-14)

What the PCB build needs on top of the breadboard orders ([../logisim/notes/breadboard-order.md](../logisim/notes/breadboard-order.md)). The chips themselves come from the breadboard cards.

Prices and supplier references are filled in when ordering. Check every reference before buying: active, sold by an authorised distributor.

## Card slots: PCIe, through-hole

Decided 2026-11-14. The cards stand vertically in a 3D-printed case, plugged into the motherboard through PCIe slots used as plain connectors. The slot order and pinout are in [slot-pinout.md](slot-pinout.md).

| Size | Contacts | Cards | Qty | Spares | Amphenol FCI (Mouser) |
|---|---|---|---|---|---|
| x1 | 36 | clkgen | 1 | 1 | 10018783-10010TLF |
| x4 | 64 | flags, iod | 2 | 1 | 10018783-11101TLF |
| x8 | 98 | alu, mem | 2 | 1 | 10018783-11112TLF |
| x16 | 164 | registers, ir, addr, cu | 4 | 1 | 10018783-10013TLF |

References chosen by Gianni on 2026-11-14. Footprints: `kicad/lib/hbc2_slots.pretty` (x1 from SamacSys, x4/x8/x16 extrapolated: check the peg holes against a real slot before ordering the motherboard); see [slot-pinout.md](slot-pinout.md), "Footprints". Check the datasheet of each for through-hole solder tails and a 1.57 mm card.

What to check on a slot before buying:
- **Mounting:** vertical, **through-hole, solder tails**. Not SMT and not press-fit: both are hard to fit by hand.
- **Card:** 1.57 mm, 1.0 mm contact pitch.
- **Motherboard:** 4 layers recommended (slot-pinout.md, "Tracks across the slots").
- **Pegs:** board-lock pegs, if any, go in the footprint.
- **Same family for every size:** pin 1 then lines up at the same place on each slot.

**Soldering:** iron at 330–350 °C, about 370 °C or a bigger tip on ground pins. Use a 1–1.6 mm chisel tip, 0.5 mm solder and flux.
1. Seat the slot.
2. Solder two opposite corners.
3. Check it sits flat and square to the board.
4. Solder the rest.

No hot air: it can warp the housing. A 164-pin slot is very hard to remove by hand, so check its position before soldering it.

**Test before the run:** solder one slot, then plug in a bare test card to check the fit and the height of the case's guide rails.

## Card PCBs (order option, not a part)

- **Thickness:** 1.6 mm, needed for the slot.
- **Edge contacts:** the "gold fingers" option, hard gold with a bevelled edge (JLCPCB, PCBWay). Plain ENIG is enough for a few dozen insertions.
- **Card-edge footprints:** in KiCad, `Connector_PCBEdge:BUS_PCIexpress_x1/x4/x8/x16`. The motherboard slot footprint is drawn from the datasheet (KiCad has none).

## DIP sockets (one per chip)

Counted from the card schematics on 2026-11-14: 158 chips, the 146 of the CPU plus the 12 of iod.

| Socket | Qty | Cards |
|---|---|---|
| DIP-14, 300 mil | 41 | every card |
| DIP-16, 300 mil | 60 | addr 16, registers 12, flags 10, alu 9, cu 8, iod 5 |
| DIP-20, 300 mil | 47 | addr 12, cu 10, alu 7, ir 6, registers 5, mem 3, iod 2, clkgen 1, flags 1 |
| DIP-28, 600 mil | 9 | cu 7 (microcode EEPROMs), mem 2 |
| DIP-32, 600 mil | 1 | mem (AS6C1008) |

- **Spares:** buy about 10 % extra on each size.
- **Type:** turned-pin or open-frame, not chosen yet.
- **ROM sockets:** the turned-pin sockets bought as pin savers for the ROMs (breadboard orders 6 and 7) can sit in the PCB sockets.
- **Off the table:**
  - The 9 resistor networks (cu 8, motherboard 1) are soldered directly.
  - The Arduino Nano sits on 2 × 1×15 female headers.
  - The oscillator Y1 has no socket.

## Motherboard

| Ref | Part | Value | Footprint | Notes |
|---|---|---|---|---|
| RN1 | Bussed resistor network, SIP-9 | 10 kΩ × 8 | `Resistor_THT:R_Array_SIP9` | Bourns 4609X-101-103LF: A8–A15 pull-downs |
| J1 | DC barrel jack, 2.1 × 5.5 mm, centre + | | `Connector_BarrelJack:BarrelJack_CUI_PJ-102AH_Horizontal` | CUI PJ-102AH to check against the GST25E05 plug |
| SW1 | Power switch, rocker on the case, ≥ 3 A | | `Connector_JST:JST_XH_B2B-XH-A_1x02_P2.50mm_Vertical` | wired to a 2-pin JST XH header (plus the mating housing and crimps) |
| F1 | Fuse 5 × 20 mm + 2 PCB clips | T 3.15 A | `Fuse:Fuseholder_Clip-5x20mm_Keystone_3517_Inline_P23.11x6.76mm_D1.70mm_Horizontal` | Keystone 3517 clips; buy spare fuses |
| D1 | TVS, unidirectional | P6KE6V8A | `Diode_THT:D_DO-15_P12.70mm_Horizontal` | drawn with the Zener symbol (KiCad has no unidirectional TVS symbol) |
| C1 | Electrolytic capacitor | 1000 µF 10 V | `Capacitor_THT:CP_Radial_D10.0mm_P5.00mm` | 10 mm can, 5 mm pitch |

The power input design is in [NOTES.md](NOTES.md), "Power (S1)".

## Power supply

| Part | Qty | Notes |
|---|---|---|
| Mean Well GST25E05 (5 V, 4 A) | 1 | budget about 0.6 A typical, 1.6 A worst case |
| Mean Well RS-25-5 (adjustable) | 0 | only if the farthest card measures below 4.9 V under load |

## Case

- 3D-printed, with the card guide rails and a top or back bracket to hold the cards.
- Card spacing about 20 mm, as in a PC.
