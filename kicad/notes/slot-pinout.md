# Card slots: order and pinout (2026-11-14)

**Status:** approved and drawn on the motherboard schematic on 2026-11-15 (slot references in [motherboard-netmap.md](motherboard-netmap.md)). The cards still have their 2.54 mm connectors: converting them to card edges is the next step.

## Mechanics

- **Cards:** the cards stand vertically in a 3D-printed case. They plug into a horizontal motherboard, like graphics cards in a PC.
- **Slots:** through-hole PCIe slots, bought as plain connectors. Nothing about PCIe as a protocol is used.
- **Card edge:** each card has gold fingers on its edge, 1.6 mm thick (the slot takes a 1.57 mm card), with a bevelled edge.
- **What to buy:** the parts are in [parts.md](parts.md).

## Pin numbering

- PCIe pins are numbered **B1–Bn** on one side of the card and **A1–An** on the other. Bn and An face each other.
- The four slot sizes share the same numbers from the key onwards:

| Size | Pins per side | Contacts | Slot length (approx.) |
|---|---|---|---|
| x1 | 1–18 | 36 | 25 mm |
| x4 | 1–32 | 64 | 39 mm |
| x8 | 1–49 | 98 | 56 mm |
| x16 | 1–82 | 164 | 89 mm |

- The key sits between pins 11 and 12 on every size.
- The slots are aligned on pin 1, so a given pin number is at the same place on every slot.

## The rule

**Every signal has one pin number, the same on every card that uses it.**

The slots are parallel, so a signal shared by several cards becomes one track across the motherboard, at the height of its pin.

No two signals use the same pin along the same stretch of motherboard. A pin number is reused only by signals whose runs don't overlap. For example, pin B33 carries `alu_r0` between the alu and registers slots, and `bus_addr_hi0` between the addr and mem slots.

This was checked by a script: 164 signals and 410 card pins, no pin used twice on a card, no two tracks on the same pin number between slots, every pin within its slot's size.

## Slot order and sizes

Left to right:

| Slot | Card | Size | Signals | Free signal pins | Why here |
|---|---|---|---|---|---|
| 1 | flags | x4 | 30 | 22 | next to the alu (alu flags, `alu_r7`, `flag_c`) |
| 2 | alu | x8 | 34 | 46 | between flags and registers (A, B, result buses) |
| 3 | registers | x16 | 61 | 81 | A/B buses with alu and addr, `ir_r*` with ir |
| 4 | ir | x16 | 48 | 94 | V1/V2 with addr, opcode/mode with cu |
| 5 | addr | x16 | 77 | 65 | V1/V2 with ir, 19 control lines with cu, high address with mem |
| 6 | cu | x16 | 96 | 46 | centre: talks to every card |
| 7 | mem | x8 | 30 | 50 | high address, 5 control lines |
| 8 | iod | x4 | 23 | 29 | end slot: its cables go out to the TTGO box |
| 9 | clkgen | x1 | 11 | 19 | end slot: Nano USB, buttons and the speed knob within reach |

**How the order was chosen:**
- The search covered all 181,440 orders.
- This order costs 423 slot-to-slot hops for the 164 signals; the shortest found costs 410.
- It was kept because it is the one where every track gets its own pin with no jog. Its neighbours share the most lines: flags–alu, alu–registers, ir–addr, addr–cu.

**Why some slots are bigger than their pin count needs:**
- The cu talks to everyone, so each line it shares with a card must sit at a pin number both slots have.
- With registers, ir and addr at x8, the cu's 96 signals would have to fit in pins 1–49 (80 signal contacts), which is impossible.
- A slot costs about the same at any size.

**Power:** pin 1, then every 6th pin up to 49, then 61 and 73.
- On every size: B = +5V, A = GND.
- Count: 11 GND and 11 +5V on an x16, 3 of each on an x1. That is one ground for every 5 to 7 signals.
- A card turned round can't go in (key), so the polarity can't be reversed.

## Zones

| Pins | On | Contents |
|---|---|---|
| 2–18 | every slot | data bus D0–D7 (2–5), low address A0–A7 (6, 8–10), clocks and resets `clk_0`–`clk_5`, `clk_wr_n`, `clk_mem_wr_n`, `rst_n`, `rst_early`, `rst_rc` (11–17), `ie`, `rw`, `inr` (17–18) |
| 20–30 | the x4 cards and up | flag outputs (20–23), `flags_op`/`bit_val`/`oe_n` (24–27). 28–30 hold two groups on separate stretches: alu → flags (`alu_c/z/s/e/f`, `alu_r7`) between slots 1–3, and cu → mem (5 lines) plus `int` between slots 6–8 |
| 32–48 | the x8 cards and up | `regs_we_n`, `regs_res_oe_n` (32). Pins 33–36 hold two groups on separate stretches: alu result R0–R6 (slots 2–3) and high address A8–A15 (slots 5–7). Then bus A (38–41), bus B (42, 44–46), `alu_op` (47–48) |
| 50–82 | x16 only | cu → addr, 19 lines (50–59). Pins 50–54 also carry the `ir_r*` lines (ir → registers), on another stretch (slots 3–4). Then cu → registers (59–66), opcode/mode/IR loads (67–74), V1/V2 (75–82) |

Every slot carries the data bus, the low address bus and the clocks at the same pins. A card that doesn't use one of those pins leaves it unconnected.

## Routing on the motherboard: to check before committing

### Footprints (Amphenol FCI 10018783, chosen 2026-11-14)

Library `kicad/lib/hbc2_slots.pretty` (registered in `mb/fp-lib-table` as `hbc2_slots`), written by `kicad/lib/gen_pcie_slots.py` on 2026-11-14:

| Size | Part | Footprint | Source |
|---|---|---|---|
| x1 | 10018783-10010TLF | `hbc2_slots:PCIe_x1_Slot_Amphenol_10018783-10010TLF` | SamacSys (Mouser library loader); the generated copy matches it hole for hole (38 holes) |
| x4 | 10018783-11101TLF | `hbc2_slots:PCIe_x4_Slot_Amphenol_10018783-11101TLF` | extrapolated |
| x8 | 10018783-11112TLF | `hbc2_slots:PCIe_x8_Slot_Amphenol_10018783-11112TLF` | extrapolated |
| x16 | 10018783-10013TLF | `hbc2_slots:PCIe_x16_Slot_Amphenol_10018783-10013TLF` | extrapolated |

**What supports the extrapolation:**
- **Pin spacing:** the pin pattern matches KiCad's card-edge footprint `BUS_PCIexpress_x16` on all 82 positions per side, 3 mm key gap included.
- **Body length:** with the x1 rule (last pin + 3.15 mm) the bodies come out at 39, 56 and 89 mm, the PCIe housing lengths.

**Not confirmed** (no drawing for x4/x8/x16 yet):
- **Pegs:** the second peg is assumed at last pin + 1.8 mm, as on the x1. Larger slots may add another peg or a retention latch.
- **Before ordering the motherboard:** get the Amphenol drawing (Mouser product page) or measure a real slot, then print the footprint 1:1 and lay the slot on it.

KiCad's own library has only the card-edge side (`Connector_PCBEdge:BUS_PCIexpress_x1/x4/x8/x16`), not the slot.

**Hole pattern** (from the x1 footprint, the same on every size):
- **Rows:** four rows of 0.7 mm holes on 1.2 mm pads, at y = −2 (A even), 0 (A odd), 2.5 (B odd) and 4.5 (B even).
- **Pitch:** pin n sits at x = n − 1 mm, and at x = n + 1 from pin 12 on, after the 3 mm key gap. Bn and An share the same x.
- **Gaps:** each row has a pad every 2 mm, so the gap between two pads is 0.8 mm. That is one track per gap per copper layer, for example a 0.2 mm track with 0.3 mm clearance (0.15 / 0.15 also works).
- **Mounting holes:** two non-plated 2.35 mm holes, one in the key gap and one at the end.

### Tracks across the slots (counted 2026-11-14)

A track for pin n runs at x = n.
- It goes straight through its own pad, or through that pad when the card leaves it unconnected.
- It goes through a gap in every other row it has to cross.
- Past the end of a shorter slot it is free.

Tracks that must cross each row of each slot, against the row's gaps on one layer:

| Slot | Size | B even | B odd | A odd | A even | Gaps per row, per layer |
|---|---|---|---|---|---|---|
| flags | x4 | 0 | 10 | 16 | 22 | 16 |
| alu | x8 | 20 | 30 | 35 | 38 | 24–25 |
| registers | x16 | 36 | 48 | 52 | 53 | 41 |
| ir | x16 | 49 | 61 | 66 | 66 | 41 |
| addr | x16 | 64 | 72 | 74 | 69 | 41 |
| cu | x16 | 67 | 59 | 48 | 30 | 41 |
| mem | x8 | 29 | 29 | 27 | 21 | 24–25 |
| iod | x4 | 21 | 18 | 15 | 8 | 16 |
| clkgen | x1 | 8 | 5 | 3 | 0 | 9 |

**Result:**
- **Two layers are not enough in practice.** The busiest rows (ir, addr) need 74 crossings against 82 gaps on two layers, which is 90 % use. Every track would also have to find a gap right next to its own x.
- **The crossings don't depend on the pin plan.** A signal between two neighbouring slots always crosses 3 rows in all, whichever row it sits on.
- **Recommendation: a 4-layer motherboard.** For example L1 signals, L2 GND plane, L3 signals plus the +5V pour, L4 signals. That gives three signal layers, 123 gaps per row (60 % at worst), and a solid ground under every track. The ground plane also fixes the power-drop concern of "Power (S1)" in design-notes.md.
- **Before the full layout:** route the data bus and the addr/cu control block across the ir, addr and cu slots.
- **Free pins:** every pin number now carries a signal on some stretch. A new signal later needs a pin free on both of its cards and along the stretch between them. The "free signal pins" column above counts each card's free pins.

## Full pinout

Each cell gives the signal and the cards it joins, in slot order. A cell with two signals reuses the pin on two separate stretches.

| Pin | Side B | Side A |
|---|---|---|
| 1 | **+5V** | **GND** |
| 2 | `bus_data0` (flags, regs, ir, addr, cu, mem, iod) | `bus_data1` (flags, regs, ir, addr, cu, mem, iod) |
| 3 | `bus_data2` (flags, regs, ir, addr, cu, mem, iod) | `bus_data3` (flags, regs, ir, addr, cu, mem, iod) |
| 4 | `bus_data4` (flags, regs, ir, addr, cu, mem, iod) | `bus_data5` (flags, regs, ir, addr, cu, mem, iod) |
| 5 | `bus_data6` (flags, regs, ir, addr, cu, mem, iod) | `bus_data7` (flags, regs, ir, addr, cu, mem, iod) |
| 6 | `bus_addr_lo0` (addr, cu, mem, iod) | `bus_addr_lo1` (addr, cu, mem, iod) |
| 7 | **+5V** | **GND** |
| 8 | `bus_addr_lo2` (addr, cu, mem, iod) | `bus_addr_lo3` (addr, cu, mem, iod) |
| 9 | `bus_addr_lo4` (addr, cu, mem, iod) | `bus_addr_lo5` (addr, cu, mem, iod) |
| 10 | `bus_addr_lo6` (addr, cu, mem, iod) | `bus_addr_lo7` (addr, cu, mem, iod) |
| 11 | `clk_0` (ir, clkgen) | `clk_1` (addr, clkgen) |
| 12 | `clk_2` (cu, clkgen) | `clk_3` (cu, clkgen) |
| 13 | **+5V** | **GND** |
| 14 | `clk_4` (flags, regs, clkgen) | `clk_5` (iod, clkgen) |
| 15 | `clk_wr_n` (regs, iod, clkgen) | `clk_mem_wr_n` (mem, clkgen) |
| 16 | `rst_n` (flags, addr, cu, iod, clkgen) | `rst_early` (cu, clkgen) |
| 17 | `rst_rc` (cu, clkgen) | `ie` (cu, iod) |
| 18 | `rw` (cu, iod) | `inr` (cu, iod) |
| 19 | **+5V** | **GND** |
| 20 | `flag_c` (flags, alu, cu) | `flag_e` (flags, cu) |
| 21 | `flag_i` (flags, cu) | `flag_n` (flags, cu) |
| 22 | `flag_s` (flags, cu) | `flag_z` (flags, cu) |
| 23 | `flag_f` (flags, cu) | `flag_h` (flags, cu) |
| 24 | `flags_op0` (flags, cu) | `flags_op1` (flags, cu) |
| 25 | **+5V** | **GND** |
| 26 | `flags_op2` (flags, cu) | `flags_op3` (flags, cu) |
| 27 | `flags_bit_val` (flags, cu) | `flags_oe_n` (flags, cu) |
| 28 | `alu_c` (flags, alu) / `mem_re` (cu, mem) | `alu_z` (flags, alu) / `mem_oe_n` (cu, mem) |
| 29 | `alu_s` (flags, alu) / `mem_we_n` (cu, mem) | `alu_e` (flags, alu) / `mem_wbuf_oe_n` (cu, mem) |
| 30 | `alu_f` (flags, alu) / `mem_rom_oe_n` (cu, mem) | `alu_r7` (flags, alu, regs) / `int` (cu, iod) |
| 31 | **+5V** | **GND** |
| 32 | `regs_we_n` (regs, cu) | `regs_res_oe_n` (regs, cu) |
| 33 | `alu_r0` (alu, regs) / `bus_addr_hi0` (addr, mem) | `alu_r1` (alu, regs) / `bus_addr_hi1` (addr, mem) |
| 34 | `alu_r2` (alu, regs) / `bus_addr_hi2` (addr, mem) | `alu_r3` (alu, regs) / `bus_addr_hi3` (addr, mem) |
| 35 | `alu_r4` (alu, regs) / `bus_addr_hi4` (addr, mem) | `alu_r5` (alu, regs) / `bus_addr_hi5` (addr, mem) |
| 36 | `alu_r6` (alu, regs) / `bus_addr_hi6` (addr, mem) | `bus_addr_hi7` (addr, mem) |
| 37 | **+5V** | **GND** |
| 38 | `bus_a0` (alu, regs, addr) | `bus_a1` (alu, regs, addr) |
| 39 | `bus_a2` (alu, regs, addr) | `bus_a3` (alu, regs, addr) |
| 40 | `bus_a4` (alu, regs, addr) | `bus_a5` (alu, regs, addr) |
| 41 | `bus_a6` (alu, regs, addr) | `bus_a7` (alu, regs, addr) |
| 42 | `bus_b0` (alu, regs, addr) | `bus_b1` (alu, regs, addr) |
| 43 | **+5V** | **GND** |
| 44 | `bus_b2` (alu, regs, addr) | `bus_b3` (alu, regs, addr) |
| 45 | `bus_b4` (alu, regs, addr) | `bus_b5` (alu, regs, addr) |
| 46 | `bus_b6` (alu, regs, addr) | `bus_b7` (alu, regs, addr) |
| 47 | `alu_op0` (alu, cu) | `alu_op1` (alu, cu) |
| 48 | `alu_op2` (alu, cu) | `alu_op3` (alu, cu) |
| 49 | **+5V** | **GND** |
| 50 | `ir_r1_0` (regs, ir) / `addr_cout` (addr, cu) | `ir_r1_1` (regs, ir) / `addr_inv` (addr, cu) |
| 51 | `ir_r1_2` (regs, ir) / `addr_off_sel` (addr, cu) | `ir_r2_0` (regs, ir) / `addr_cin` (addr, cu) |
| 52 | `ir_r2_1` (regs, ir) / `addr_lat_ld_n` (addr, cu) | `ir_r2_2` (regs, ir) / `addr_pc_ld_n` (addr, cu) |
| 53 | `ir_r1_2_n` (regs, ir) / `addr_sp_ld_n` (addr, cu) | `ir_r2_2_n` (regs, ir) / `addr_base_sel0` (addr, cu) |
| 54 | `ir_r3_2_n` (regs, ir) / `addr_base_sel1` (addr, cu) | `addr_base_oe_n0` (addr, cu) |
| 55 | `addr_base_oe_n1` (addr, cu) | `addr_off_oe_n0` (addr, cu) |
| 56 | `addr_off_oe_n1` (addr, cu) | `addr_lat_oe_n0` (addr, cu) |
| 57 | `addr_lat_oe_n1` (addr, cu) | `addr_pc_oe_n0` (addr, cu) |
| 58 | `addr_pc_oe_n1` (addr, cu) | `addr_sp_oe_n0` (addr, cu) |
| 59 | `addr_sp_oe_n1` (addr, cu) | `regs_a_lat_oe_n` (regs, cu) |
| 60 | `regs_b_lat_oe_n` (regs, cu) | `regs_const_oe_n` (regs, cu) |
| 61 | **+5V** | **GND** |
| 62 | `regs_const_0` (regs, cu) | `regs_a_bus_oe_n` (regs, cu) |
| 63 | `regs_a_sel0` (regs, cu) | `regs_a_sel1` (regs, cu) |
| 64 | `regs_b_sel0` (regs, cu) | `regs_b_sel1` (regs, cu) |
| 65 | `regs_cu_bank_n0` (regs, cu) | `regs_cu_bank_n1` (regs, cu) |
| 66 | `regs_cu_idx0` (regs, cu) | `regs_cu_idx1` (regs, cu) |
| 67 | `ir_op0` (ir, cu) | `ir_op1` (ir, cu) |
| 68 | `ir_op2` (ir, cu) | `ir_op3` (ir, cu) |
| 69 | `ir_op4` (ir, cu) | `ir_op5` (ir, cu) |
| 70 | `ir_mode0` (ir, cu) | `ir_mode1` (ir, cu) |
| 71 | `ir_mode2` (ir, cu) | `ir_mode3` (ir, cu) |
| 72 | `ir_b0_ld_n` (ir, cu) | `ir_b1_ld_n` (ir, cu) |
| 73 | **+5V** | **GND** |
| 74 | `ir_v1_ld_n` (ir, cu) | `ir_v2_ld_n` (ir, cu) |
| 75 | `ir_v1_0` (regs, ir, addr) | `ir_v1_1` (regs, ir, addr) |
| 76 | `ir_v1_2` (regs, ir, addr) | `ir_v1_3` (ir, addr) |
| 77 | `ir_v1_4` (ir, addr) | `ir_v1_5` (ir, addr) |
| 78 | `ir_v1_6` (ir, addr) | `ir_v1_7` (ir, addr) |
| 79 | `ir_v2_0` (ir, addr) | `ir_v2_1` (ir, addr) |
| 80 | `ir_v2_2` (ir, addr) | `ir_v2_3` (ir, addr) |
| 81 | `ir_v2_4` (ir, addr) | `ir_v2_5` (ir, addr) |
| 82 | `ir_v2_6` (ir, addr) | `ir_v2_7` (ir, addr) |
