# Motherboard net map (2026-11-14, slots 2026-11-15)

One line per CPU signal (names from `logisim/cpu.circ`), with its slot pin and the cards it joins. The first card listed drives the signal (for a shared bus it is only the first one listed). Generated from the card schematics: 164 signals, 410 card pins, every pin used exactly once (regenerated with slot pins on 2026-11-15).

Not on the motherboard: iod J1, J2, J4 (cables to the TTGO) and clkgen JP1. Power: on every card edge and slot, B = +5V and A = GND on pins 1, 7, …, 49, 61, 73 (as many as the size has).

**On the motherboard itself:**
- **A8–A15 pull-downs:** a 10 kΩ × 8 bussed network (Bourns 4609X-101-103LF, `Device:R_Network08`, `Resistor_THT:R_Array_SIP9`), common pin on GND, on `bus_addr_hi0..7`. They moved off the iod card on 2026-11-14 (the vector step: the EVT 595 drives A0–A7 and nothing drives A8–A15).
- The power input (`design-notes.md`, "Power (S1)").
- The power input (J1, SW1, F1, D1, C1, drawn 2026-11-15) and the slots' power pins: +5V on side B, GND on side A, pins 1, 7, …, 49, 61, 73 ([slot-pinout.md](slot-pinout.md)).

**Motherboard slots** (`mb/hbc2_mb.kicad_sch`, finished 2026-11-15): the 66 connectors of 2026-11-14 became 9 PCIe slots (symbols `custom_symbols:HBC2_PCIe_xN`, footprints `hbc2_slots`), pins as in [slot-pinout.md](slot-pinout.md). Every net carries the CPU name of this table, not the card's own name. Checked on 2026-11-15: every slot pin matches the plan, the 159 inter-card nets of `logisim/cpu.circ` (flattened with `tools/netlist.py`) join exactly the same cards on the motherboard (renamed: `bus_addr[0..7]` = `bus_addr_lo0..7`, `bus_addr[8..15]` = `bus_addr_hi0..7`, `alu_n` = `alu_r7`, `ir_r3[0..2]` = `ir_v1_0..2`; iod isn't in cpu.circ, its 5 own nets match this table), ERC 0 / 0.

| Slot | Card | Size |
|---|---|---|
| J2 | flags | x4 |
| J4 | alu | x8 |
| J6 | registers | x16 |
| J8 | ir | x16 |
| J10 | addr | x16 |
| J3 | cu | x16 |
| J5 | mem | x8 |
| J7 | iod | x4 |
| J9 | clkgen | x1 |

**Cards (converted 2026-11-15):** every card's 2.54 mm connectors became one card edge, the same `HBC2_PCIe_xN` symbol with footprint `Connector_PCBEdge:BUS_PCIexpress_xN`. Kept off the edge: iod J1, J2, J4 (TTGO cables) and clkgen JP1. Since every signal has one pin number on every card, the table below gives one slot pin per signal, with each card's own net name. Checked the same day: each card's edge pins match its motherboard slot pin for pin (signals, +5V, GND, unused), card nets unchanged apart from the connectors, ERC 0 / 0 and `kicadcmp.py` unchanged on every card. The edge symbol is J1 on every card except iod, where it is J3 (its J1, J2 and J4 go to the TTGO). References renumbered by Gianni on 2026-11-15; the slot table above uses the new motherboard numbers.

**Names that differ from the card's own:**
- `clk_2` / `clk_3` (clkgen `clk2` / `clk3`, B12 / A12) go to the cu's `clk0` / `clk1`. `clk_4` feeds flags and registers; `clk_5` feeds iod.
- `ir_v1_0..2` also go to registers (`regs_r3_0..2`: R3 is V1's low 3 bits).
- `alu_r7` also goes to flags (`flags_alu_n`).

| Signal | Slot pin | Driver (card net) | Other cards (card net) |
|---|---|---|---|
| bus_data0 | B2 | addr `addr_bus0` | cu `cu_bus_in0`, flags `flags_bus0`, ir `ir_bus_in0`, mem `mem_bus0`, registers `regs_bus0`, iod `iod_bus0` |
| bus_data1 | A2 | addr `addr_bus1` | cu `cu_bus_in1`, flags `flags_bus1`, ir `ir_bus_in1`, mem `mem_bus1`, registers `regs_bus1`, iod `iod_bus1` |
| bus_data2 | B3 | addr `addr_bus2` | cu `cu_bus_in2`, flags `flags_bus2`, ir `ir_bus_in2`, mem `mem_bus2`, registers `regs_bus2`, iod `iod_bus2` |
| bus_data3 | A3 | addr `addr_bus3` | cu `cu_bus_in3`, flags `flags_bus3`, ir `ir_bus_in3`, mem `mem_bus3`, registers `regs_bus3`, iod `iod_bus3` |
| bus_data4 | B4 | addr `addr_bus4` | cu `cu_bus_in4`, flags `flags_bus4`, ir `ir_bus_in4`, mem `mem_bus4`, registers `regs_bus4`, iod `iod_bus4` |
| bus_data5 | A4 | addr `addr_bus5` | cu `cu_bus_in5`, flags `flags_bus5`, ir `ir_bus_in5`, mem `mem_bus5`, registers `regs_bus5`, iod `iod_bus5` |
| bus_data6 | B5 | addr `addr_bus6` | cu `cu_bus_in6`, flags `flags_bus6`, ir `ir_bus_in6`, mem `mem_bus6`, registers `regs_bus6`, iod `iod_bus6` |
| bus_data7 | A5 | addr `addr_bus7` | cu `cu_bus_in7`, flags `flags_bus7`, ir `ir_bus_in7`, mem `mem_bus7`, registers `regs_bus7`, iod `iod_bus7` |
| bus_addr_lo0 | B6 | addr `addr_bus_addr_lo0` | cu `cu_bus_addr_lo0`, mem `mem_addr0`, iod `iod_addr0` |
| bus_addr_lo1 | A6 | addr `addr_bus_addr_lo1` | cu `cu_bus_addr_lo1`, mem `mem_addr1`, iod `iod_addr1` |
| bus_addr_lo2 | B8 | addr `addr_bus_addr_lo2` | cu `cu_bus_addr_lo2`, mem `mem_addr2`, iod `iod_addr2` |
| bus_addr_lo3 | A8 | addr `addr_bus_addr_lo3` | cu `cu_bus_addr_lo3`, mem `mem_addr3`, iod `iod_addr3` |
| bus_addr_lo4 | B9 | addr `addr_bus_addr_lo4` | cu `cu_bus_addr_lo4`, mem `mem_addr4`, iod `iod_addr4` |
| bus_addr_lo5 | A9 | addr `addr_bus_addr_lo5` | cu `cu_bus_addr_lo5`, mem `mem_addr5`, iod `iod_addr5` |
| bus_addr_lo6 | B10 | addr `addr_bus_addr_lo6` | cu `cu_bus_addr_lo6`, mem `mem_addr6`, iod `iod_addr6` |
| bus_addr_lo7 | A10 | addr `addr_bus_addr_lo7` | cu `cu_bus_addr_lo7`, mem `mem_addr7`, iod `iod_addr7` |
| bus_addr_hi0 | B33 | addr `addr_bus_addr_hi0` | mem `mem_addr8` |
| bus_addr_hi1 | A33 | addr `addr_bus_addr_hi1` | mem `mem_addr9` |
| bus_addr_hi2 | B34 | addr `addr_bus_addr_hi2` | mem `mem_addr10` |
| bus_addr_hi3 | A34 | addr `addr_bus_addr_hi3` | mem `mem_addr11` |
| bus_addr_hi4 | B35 | addr `addr_bus_addr_hi4` | mem `mem_addr12` |
| bus_addr_hi5 | A35 | addr `addr_bus_addr_hi5` | mem `mem_addr13` |
| bus_addr_hi6 | B36 | addr `addr_bus_addr_hi6` | mem `mem_addr14` |
| bus_addr_hi7 | A36 | addr `addr_bus_addr_hi7` | mem `mem_addr15` |
| bus_a0 | B38 | registers `regs_a0` | addr `addr_bus_a0`, alu `alu_a0` |
| bus_a1 | A38 | registers `regs_a1` | addr `addr_bus_a1`, alu `alu_a1` |
| bus_a2 | B39 | registers `regs_a2` | addr `addr_bus_a2`, alu `alu_a2` |
| bus_a3 | A39 | registers `regs_a3` | addr `addr_bus_a3`, alu `alu_a3` |
| bus_a4 | B40 | registers `regs_a4` | addr `addr_bus_a4`, alu `alu_a4` |
| bus_a5 | A40 | registers `regs_a5` | addr `addr_bus_a5`, alu `alu_a5` |
| bus_a6 | B41 | registers `regs_a6` | addr `addr_bus_a6`, alu `alu_a6` |
| bus_a7 | A41 | registers `regs_a7` | addr `addr_bus_a7`, alu `alu_a7` |
| bus_b0 | B42 | registers `regs_b0` | addr `addr_bus_b0`, alu `alu_b0` |
| bus_b1 | A42 | registers `regs_b1` | addr `addr_bus_b1`, alu `alu_b1` |
| bus_b2 | B44 | registers `regs_b2` | addr `addr_bus_b2`, alu `alu_b2` |
| bus_b3 | A44 | registers `regs_b3` | addr `addr_bus_b3`, alu `alu_b3` |
| bus_b4 | B45 | registers `regs_b4` | addr `addr_bus_b4`, alu `alu_b4` |
| bus_b5 | A45 | registers `regs_b5` | addr `addr_bus_b5`, alu `alu_b5` |
| bus_b6 | B46 | registers `regs_b6` | addr `addr_bus_b6`, alu `alu_b6` |
| bus_b7 | A46 | registers `regs_b7` | addr `addr_bus_b7`, alu `alu_b7` |
| alu_r0 | B33 | alu `alu_r0` | registers `regs_alu_r0` |
| alu_r1 | A33 | alu `alu_r1` | registers `regs_alu_r1` |
| alu_r2 | B34 | alu `alu_r2` | registers `regs_alu_r2` |
| alu_r3 | A34 | alu `alu_r3` | registers `regs_alu_r3` |
| alu_r4 | B35 | alu `alu_r4` | registers `regs_alu_r4` |
| alu_r5 | A35 | alu `alu_r5` | registers `regs_alu_r5` |
| alu_r6 | B36 | alu `alu_r6` | registers `regs_alu_r6` |
| alu_r7 | A30 | alu `alu_r7` | registers `regs_alu_r7`, flags `flags_alu_n` |
| ir_v1_0 | B75 | ir `ir_v1_0` | addr `addr_bus_v1_0`, registers `regs_r3_0` |
| ir_v1_1 | A75 | ir `ir_v1_1` | addr `addr_bus_v1_1`, registers `regs_r3_1` |
| ir_v1_2 | B76 | ir `ir_v1_2` | addr `addr_bus_v1_2`, registers `regs_r3_2` |
| ir_v1_3 | A76 | ir `ir_v1_3` | addr `addr_bus_v1_3` |
| ir_v1_4 | B77 | ir `ir_v1_4` | addr `addr_bus_v1_4` |
| ir_v1_5 | A77 | ir `ir_v1_5` | addr `addr_bus_v1_5` |
| ir_v1_6 | B78 | ir `ir_v1_6` | addr `addr_bus_v1_6` |
| ir_v1_7 | A78 | ir `ir_v1_7` | addr `addr_bus_v1_7` |
| ir_v2_0 | B79 | ir `ir_v2_0` | addr `addr_bus_v2_0` |
| ir_v2_1 | A79 | ir `ir_v2_1` | addr `addr_bus_v2_1` |
| ir_v2_2 | B80 | ir `ir_v2_2` | addr `addr_bus_v2_2` |
| ir_v2_3 | A80 | ir `ir_v2_3` | addr `addr_bus_v2_3` |
| ir_v2_4 | B81 | ir `ir_v2_4` | addr `addr_bus_v2_4` |
| ir_v2_5 | A81 | ir `ir_v2_5` | addr `addr_bus_v2_5` |
| ir_v2_6 | B82 | ir `ir_v2_6` | addr `addr_bus_v2_6` |
| ir_v2_7 | A82 | ir `ir_v2_7` | addr `addr_bus_v2_7` |
| ir_op0 | B67 | ir `ir_op0` | cu `cu_ir_op0` |
| ir_op1 | A67 | ir `ir_op1` | cu `cu_ir_op1` |
| ir_op2 | B68 | ir `ir_op2` | cu `cu_ir_op2` |
| ir_op3 | A68 | ir `ir_op3` | cu `cu_ir_op3` |
| ir_op4 | B69 | ir `ir_op4` | cu `cu_ir_op4` |
| ir_op5 | A69 | ir `ir_op5` | cu `cu_ir_op5` |
| ir_mode0 | B70 | ir `ir_mode0` | cu `cu_ir_mode0` |
| ir_mode1 | A70 | ir `ir_mode1` | cu `cu_ir_mode1` |
| ir_mode2 | B71 | ir `ir_mode2` | cu `cu_ir_mode2` |
| ir_mode3 | A71 | ir `ir_mode3` | cu `cu_ir_mode3` |
| ir_r1_0 | B50 | ir `ir_r1_0` | registers `regs_r1_0` |
| ir_r1_1 | A50 | ir `ir_r1_1` | registers `regs_r1_1` |
| ir_r1_2 | B51 | ir `ir_r1_2` | registers `regs_r1_2` |
| ir_r2_0 | A51 | ir `ir_r2_0` | registers `regs_r2_0` |
| ir_r2_1 | B52 | ir `ir_r2_1` | registers `regs_r2_1` |
| ir_r2_2 | A52 | ir `ir_r2_2` | registers `regs_r2_2` |
| ir_r1_2_n | B53 | ir `ir_r1_2_n` | registers `regs_r1_2_n` |
| ir_r2_2_n | A53 | ir `ir_r2_2_n` | registers `regs_r2_2_n` |
| ir_r3_2_n | B54 | ir `ir_r3_2_n` | registers `regs_r3_2_n` |
| alu_c | B28 | alu `alu_c` | flags `flags_alu_c` |
| alu_z | A28 | alu `alu_z` | flags `flags_alu_z` |
| alu_s | B29 | alu `alu_s` | flags `flags_alu_s` |
| alu_e | A29 | alu `alu_e` | flags `flags_alu_e` |
| alu_f | B30 | alu `alu_f` | flags `flags_alu_f` |
| flag_c | B20 | flags `flag_c` | cu `cu_flag_c`, alu `alu_cin` |
| flag_e | A20 | flags `flag_e` | cu `cu_flag_e` |
| flag_i | B21 | flags `flag_i` | cu `cu_flag_i` |
| flag_n | A21 | flags `flag_n` | cu `cu_flag_n` |
| flag_s | B22 | flags `flag_s` | cu `cu_flag_s` |
| flag_z | A22 | flags `flag_z` | cu `cu_flag_z` |
| flag_f | B23 | flags `flag_f` | cu `cu_flag_f` |
| flag_h | A23 | flags `flag_h` | cu `cu_flag_h` |
| addr_cout | B50 | addr `addr_cout` | cu `cu_addr_cout` |
| int | A30 | iod `iod_int` | cu `cu_int` |
| alu_op0 | B47 | cu `cu_alu_op0` | alu `alu_op0` |
| alu_op1 | A47 | cu `cu_alu_op1` | alu `alu_op1` |
| alu_op2 | B48 | cu `cu_alu_op2` | alu `alu_op2` |
| alu_op3 | A48 | cu `cu_alu_op3` | alu `alu_op3` |
| flags_op0 | B24 | cu `cu_flags_op0` | flags `flags_op0` |
| flags_op1 | A24 | cu `cu_flags_op1` | flags `flags_op1` |
| flags_op2 | B26 | cu `cu_flags_op2` | flags `flags_op2` |
| flags_op3 | A26 | cu `cu_flags_op3` | flags `flags_op3` |
| flags_bit_val | B27 | cu `cu_flags_bit_val` | flags `flags_bit_val` |
| flags_oe_n | A27 | cu `cu_flags_oe_n` | flags `flags_oe_n` |
| regs_we_n | B32 | cu `cu_regs_we_n` | registers `regs_we_n` |
| regs_a_lat_oe_n | A59 | cu `cu_regs_a_lat_oe_n` | registers `regs_a_lat_oe_n` |
| regs_b_lat_oe_n | B60 | cu `cu_regs_b_lat_oe_n` | registers `regs_b_lat_oe_n` |
| regs_const_oe_n | A60 | cu `cu_regs_const_oe_n` | registers `regs_const_oe_n` |
| regs_const_0 | B62 | cu `cu_regs_const_0` | registers `regs_const_0` |
| regs_a_bus_oe_n | A62 | cu `cu_regs_a_bus_oe_n` | registers `regs_a_bus_oe_n` |
| regs_res_oe_n | A32 | cu `cu_regs_res_oe_n` | registers `regs_res_oe_n` |
| regs_a_sel0 | B63 | cu `cu_regs_a_sel0` | registers `regs_a_sel0` |
| regs_a_sel1 | A63 | cu `cu_regs_a_sel1` | registers `regs_a_sel1` |
| regs_b_sel0 | B64 | cu `cu_regs_b_sel0` | registers `regs_b_sel0` |
| regs_b_sel1 | A64 | cu `cu_regs_b_sel1` | registers `regs_b_sel1` |
| regs_cu_bank_n0 | B65 | cu `cu_regs_cu_bank_n0` | registers `regs_cu_bank_n0` |
| regs_cu_bank_n1 | A65 | cu `cu_regs_cu_bank_n1` | registers `regs_cu_bank_n1` |
| regs_cu_idx0 | B66 | cu `cu_regs_cu_idx0` | registers `regs_cu_idx0` |
| regs_cu_idx1 | A66 | cu `cu_regs_cu_idx1` | registers `regs_cu_idx1` |
| addr_inv | A50 | cu `cu_addr_inv` | addr `addr_inv` |
| addr_off_sel | B51 | cu `cu_addr_off_sel` | addr `addr_off_sel` |
| addr_cin | A51 | cu `cu_addr_cin` | addr `addr_cin` |
| addr_lat_ld_n | B52 | cu `cu_addr_lat_ld_n` | addr `addr_lat_ld_n` |
| addr_pc_ld_n | A52 | cu `cu_addr_pc_ld_n` | addr `addr_pc_ld_n` |
| addr_sp_ld_n | B53 | cu `cu_addr_sp_ld_n` | addr `addr_sp_ld_n` |
| addr_base_sel0 | A53 | cu `cu_addr_base_sel0` | addr `addr_base_sel0` |
| addr_base_sel1 | B54 | cu `cu_addr_base_sel1` | addr `addr_base_sel1` |
| addr_base_oe_n0 | A54 | cu `cu_addr_base_oe_n0` | addr `addr_base_oe_n0` |
| addr_base_oe_n1 | B55 | cu `cu_addr_base_oe_n1` | addr `addr_base_oe_n1` |
| addr_off_oe_n0 | A55 | cu `cu_addr_off_oe_n0` | addr `addr_off_oe_n0` |
| addr_off_oe_n1 | B56 | cu `cu_addr_off_oe_n1` | addr `addr_off_oe_n1` |
| addr_lat_oe_n0 | A56 | cu `cu_addr_lat_oe_n0` | addr `addr_lat_oe_n0` |
| addr_lat_oe_n1 | B57 | cu `cu_addr_lat_oe_n1` | addr `addr_lat_oe_n1` |
| addr_pc_oe_n0 | A57 | cu `cu_addr_pc_oe_n0` | addr `addr_pc_oe_n0` |
| addr_pc_oe_n1 | B58 | cu `cu_addr_pc_oe_n1` | addr `addr_pc_oe_n1` |
| addr_sp_oe_n0 | A58 | cu `cu_addr_sp_oe_n0` | addr `addr_sp_oe_n0` |
| addr_sp_oe_n1 | B59 | cu `cu_addr_sp_oe_n1` | addr `addr_sp_oe_n1` |
| ir_b0_ld_n | B72 | cu `cu_ir_b0_ld_n` | ir `ir_b0_ld_n` |
| ir_b1_ld_n | A72 | cu `cu_ir_b1_ld_n` | ir `ir_b1_ld_n` |
| ir_v1_ld_n | B74 | cu `cu_ir_v1_ld_n` | ir `ir_v1_ld_n` |
| ir_v2_ld_n | A74 | cu `cu_ir_v2_ld_n` | ir `ir_v2_ld_n` |
| mem_re | B28 | cu `cu_mem_re` | mem `mem_re` |
| mem_oe_n | A28 | cu `cu_mem_oe_n` | mem `mem_oe_n` |
| mem_we_n | B29 | cu `cu_mem_we_n` | mem `mem_we_n` |
| mem_wbuf_oe_n | A29 | cu `cu_mem_wbuf_oe_n` | mem `mem_wbuf_oe_n` |
| mem_rom_oe_n | B30 | cu `cu_mem_rom_oe_n` | mem `mem_rom_oe_n` |
| ie | A17 | cu `cu_ie` | iod `iod_ie` |
| rw | B18 | cu `cu_rw` | iod `iod_rw` |
| inr | A18 | cu `cu_inr` | iod `iod_inr` |
| clk_0 | B11 | clkgen `clk0` | ir `clk` |
| clk_1 | A11 | clkgen `clk1` | addr `clk` |
| clk_2 | B12 | clkgen `clk2` | cu `clk0` |
| clk_3 | A12 | clkgen `clk3` | cu `clk1` |
| clk_4 | B14 | clkgen `clk4` | flags `clk`, registers `clk` |
| clk_5 | A14 | clkgen `clk5` | iod `clk` |
| clk_wr_n | B15 | clkgen `clk_wr_n` | registers `clk_wr_n`, iod `clk_wr_n` |
| clk_mem_wr_n | A15 | clkgen `clk_mem_wr_n` | mem `clk_mem_wr_n` |
| rst_n | B16 | clkgen `rst_n` | cu `rst_n`, flags `rst_n`, addr `rst_n`, iod `rst_n` |
| rst_early | A16 | clkgen `rst_early` | cu `rst_early` |
| rst_rc | B17 | clkgen `rst_rc` | cu `rst_rc` |
