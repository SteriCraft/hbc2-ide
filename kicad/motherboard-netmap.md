# Motherboard net map (2026-11-14)

One line per CPU signal (names from `logisim/cpu.circ`), with the card connector pins it joins. The first card listed drives the signal (for a shared bus it is only the first one listed). Generated from the card schematics: 164 signals, 410 card pins, every pin used exactly once.

Not on the motherboard: iod J5, J6, J7 (cables to the TTGO) and clkgen JP1. Power: every card's 4-pin power connector is GND, +5V, +5V, GND (see `NOTES.md`, "Power (S1)").

**On the motherboard itself:**
- **A8–A15 pull-downs:** a 10 kΩ × 8 bussed network (Bourns 4609X-101-103LF, `Device:R_Network08`, `Resistor_THT:R_Array_SIP9`), common pin on GND, on `bus_addr_hi0..7`. They moved off the iod card on 2026-11-14 (the vector step: the EVT 595 drives A0–A7 and nothing drives A8–A15).
- The power input (`NOTES.md`, "Power (S1)").
- The 9 card power connectors (J4, J15, J18, J28, J33, J35, J47, J48, J58), each in its card's frame: GND, +5V, +5V, GND, on `power:GND` / `power:+5V` symbols.

**Motherboard references** (`mb/hbc2_mb.kicad_sch`, 2026-11-14): the schematic has one dashed frame and one KiCad group per card, connectors in card order; the motherboard numbers its connectors 1–66, so the card references in the table below are not the motherboard's. Every pin carries the same net name as on its card (checked against the card netlists).

| Card | Card connector → motherboard reference |
|---|---|
| addr | J1 → J1, J2 → J2, J3 → J3, J4 → J4 (power), J5 → J5, J6 → J6, J7 → J7, J8 → J8, J9 → J9, J10 → J10 |
| alu | J1 → J11, J2 → J12, J3 → J13, J4 → J14, J5 → J15 (power) |
| flags | J1 → J29, J2 → J30, J3 → J31, J4 → J32, J5 → J33 (power) |
| ir | J1 → J34, J2 → J36, J3 → J40, J4 → J41, J5 → J42, J6 → J43, J7 → J44, J8 → J45, J9 → J46, J10 → J47 (power) |
| mem | J1 → J48 (power), J2 → J49, J3 → J50, J4 → J51 |
| registers | J1 → J52, J2 → J53, J3 → J54, J4 → J55, J5 → J56, J6 → J57, J7 → J58 (power), J8 → J59, J9 → J60, J10 → J61, J11 → J62, J12 → J63, J13 → J64, J14 → J65, J15 → J66 |
| iod | J1 → J37, J2 → J35 (power), J3 → J38, J4 → J39 |
| cu | J1 → J22, J2 → J23, J3 → J19, J4 → J28 (power), J5 → J20, J6 → J21, J7 → J24, J8 → J25, J9 → J26, J10 → J27 |
| clkgen | J1 → J16, J2 → J17, J3 → J18 (power) |

**Names that differ from the card's own:**
- `clk_2` / `clk_3` (clkgen J2.3 / J2.4) go to the cu's `clk0` / `clk1`. `clk_4` feeds flags and registers; `clk_5` feeds iod.
- `ir_v1_0..2` also go to registers J3 (`regs_r3_0..2`: R3 is V1's low 3 bits).
- `alu_r7` also goes to flags J1.4 (`flags_alu_n`).
- Some connectors count down (addr J1 is `bus7..0`, ir J5/J6 `r_2..0`, ir J9 `r3, r2, r1`): only the routing is affected.

| Signal | Card, pin | Other cards, pins |
|---|---|---|
| bus_data0 | addr J1.8 | cu J2.1, flags J3.1, ir J2.1, mem J3.1, registers J12.1, iod J3.1 |
| bus_data1 | addr J1.7 | cu J2.2, flags J3.2, ir J2.2, mem J3.2, registers J12.2, iod J3.2 |
| bus_data2 | addr J1.6 | cu J2.3, flags J3.3, ir J2.3, mem J3.3, registers J12.3, iod J3.3 |
| bus_data3 | addr J1.5 | cu J2.4, flags J3.4, ir J2.4, mem J3.4, registers J12.4, iod J3.4 |
| bus_data4 | addr J1.4 | cu J2.5, flags J3.5, ir J2.5, mem J3.5, registers J12.5, iod J3.5 |
| bus_data5 | addr J1.3 | cu J2.6, flags J3.6, ir J2.6, mem J3.6, registers J12.6, iod J3.6 |
| bus_data6 | addr J1.2 | cu J2.7, flags J3.7, ir J2.7, mem J3.7, registers J12.7, iod J3.7 |
| bus_data7 | addr J1.1 | cu J2.8, flags J3.8, ir J2.8, mem J3.8, registers J12.8, iod J3.8 |
| bus_addr_lo0 | addr J9.8 | cu J10.1, mem J4.1, iod J4.1 |
| bus_addr_lo1 | addr J9.7 | cu J10.2, mem J4.2, iod J4.2 |
| bus_addr_lo2 | addr J9.6 | cu J10.3, mem J4.3, iod J4.3 |
| bus_addr_lo3 | addr J9.5 | cu J10.4, mem J4.4, iod J4.4 |
| bus_addr_lo4 | addr J9.4 | cu J10.5, mem J4.5, iod J4.5 |
| bus_addr_lo5 | addr J9.3 | cu J10.6, mem J4.6, iod J4.6 |
| bus_addr_lo6 | addr J9.2 | cu J10.7, mem J4.7, iod J4.7 |
| bus_addr_lo7 | addr J9.1 | cu J10.8, mem J4.8, iod J4.8 |
| bus_addr_hi0 | addr J10.8 | mem J4.9 |
| bus_addr_hi1 | addr J10.7 | mem J4.10 |
| bus_addr_hi2 | addr J10.6 | mem J4.11 |
| bus_addr_hi3 | addr J10.5 | mem J4.12 |
| bus_addr_hi4 | addr J10.4 | mem J4.13 |
| bus_addr_hi5 | addr J10.3 | mem J4.14 |
| bus_addr_hi6 | addr J10.2 | mem J4.15 |
| bus_addr_hi7 | addr J10.1 | mem J4.16 |
| bus_a0 | registers J6.1 | addr J5.8, alu J2.1 |
| bus_a1 | registers J6.2 | addr J5.7, alu J2.2 |
| bus_a2 | registers J6.3 | addr J5.6, alu J2.3 |
| bus_a3 | registers J6.4 | addr J5.5, alu J2.4 |
| bus_a4 | registers J6.5 | addr J5.4, alu J2.5 |
| bus_a5 | registers J6.6 | addr J5.3, alu J2.6 |
| bus_a6 | registers J6.7 | addr J5.2, alu J2.7 |
| bus_a7 | registers J6.8 | addr J5.1, alu J2.8 |
| bus_b0 | registers J15.1 | addr J6.8, alu J2.9 |
| bus_b1 | registers J15.2 | addr J6.7, alu J2.10 |
| bus_b2 | registers J15.3 | addr J6.6, alu J2.11 |
| bus_b3 | registers J15.4 | addr J6.5, alu J2.12 |
| bus_b4 | registers J15.5 | addr J6.4, alu J2.13 |
| bus_b5 | registers J15.6 | addr J6.3, alu J2.14 |
| bus_b6 | registers J15.7 | addr J6.2, alu J2.15 |
| bus_b7 | registers J15.8 | addr J6.1, alu J2.16 |
| alu_r0 | alu J3.1 | registers J11.1 |
| alu_r1 | alu J3.2 | registers J11.2 |
| alu_r2 | alu J3.3 | registers J11.3 |
| alu_r3 | alu J3.4 | registers J11.4 |
| alu_r4 | alu J3.5 | registers J11.5 |
| alu_r5 | alu J3.6 | registers J11.6 |
| alu_r6 | alu J3.7 | registers J11.7 |
| alu_r7 | alu J3.8 | registers J11.8, flags J1.4 |
| ir_v1_0 | ir J7.1 | addr J7.8, registers J3.1 |
| ir_v1_1 | ir J7.2 | addr J7.7, registers J3.2 |
| ir_v1_2 | ir J7.3 | addr J7.6, registers J3.3 |
| ir_v1_3 | ir J7.4 | addr J7.5 |
| ir_v1_4 | ir J7.5 | addr J7.4 |
| ir_v1_5 | ir J7.6 | addr J7.3 |
| ir_v1_6 | ir J7.7 | addr J7.2 |
| ir_v1_7 | ir J7.8 | addr J7.1 |
| ir_v2_0 | ir J8.1 | addr J8.8 |
| ir_v2_1 | ir J8.2 | addr J8.7 |
| ir_v2_2 | ir J8.3 | addr J8.6 |
| ir_v2_3 | ir J8.4 | addr J8.5 |
| ir_v2_4 | ir J8.5 | addr J8.4 |
| ir_v2_5 | ir J8.6 | addr J8.3 |
| ir_v2_6 | ir J8.7 | addr J8.2 |
| ir_v2_7 | ir J8.8 | addr J8.1 |
| ir_op0 | ir J3.1 | cu J3.1 |
| ir_op1 | ir J3.2 | cu J3.2 |
| ir_op2 | ir J3.3 | cu J3.3 |
| ir_op3 | ir J3.4 | cu J3.4 |
| ir_op4 | ir J3.5 | cu J3.5 |
| ir_op5 | ir J3.6 | cu J3.6 |
| ir_mode0 | ir J4.1 | cu J5.1 |
| ir_mode1 | ir J4.2 | cu J5.2 |
| ir_mode2 | ir J4.3 | cu J5.3 |
| ir_mode3 | ir J4.4 | cu J5.4 |
| ir_r1_0 | ir J5.3 | registers J1.1 |
| ir_r1_1 | ir J5.2 | registers J1.2 |
| ir_r1_2 | ir J5.1 | registers J1.3 |
| ir_r2_0 | ir J6.3 | registers J2.1 |
| ir_r2_1 | ir J6.2 | registers J2.2 |
| ir_r2_2 | ir J6.1 | registers J2.3 |
| ir_r1_2_n | ir J9.3 | registers J10.1 |
| ir_r2_2_n | ir J9.2 | registers J10.2 |
| ir_r3_2_n | ir J9.1 | registers J10.3 |
| alu_c | alu J1.6 | flags J1.6 |
| alu_z | alu J4.1 | flags J1.5 |
| alu_s | alu J4.2 | flags J1.3 |
| alu_e | alu J4.3 | flags J1.2 |
| alu_f | alu J4.4 | flags J1.1 |
| flag_c | flags J2.8 | cu J1.8, alu J1.5 |
| flag_e | flags J2.7 | cu J1.9 |
| flag_i | flags J2.6 | cu J1.10 |
| flag_n | flags J2.5 | cu J1.11 |
| flag_s | flags J2.4 | cu J1.12 |
| flag_z | flags J2.3 | cu J1.13 |
| flag_f | flags J2.2 | cu J1.14 |
| flag_h | flags J2.1 | cu J1.15 |
| addr_cout | addr J2.9 | cu J1.7 |
| int | iod J1.7 | cu J1.6 |
| alu_op0 | cu J7.1 | alu J1.1 |
| alu_op1 | cu J7.2 | alu J1.2 |
| alu_op2 | cu J7.3 | alu J1.3 |
| alu_op3 | cu J7.4 | alu J1.4 |
| flags_op0 | cu J8.1 | flags J4.1 |
| flags_op1 | cu J8.2 | flags J4.2 |
| flags_op2 | cu J8.3 | flags J4.3 |
| flags_op3 | cu J8.4 | flags J4.4 |
| flags_bit_val | cu J6.1 | flags J1.10 |
| flags_oe_n | cu J6.2 | flags J1.9 |
| regs_we_n | cu J6.3 | registers J8.1 |
| regs_a_lat_oe_n | cu J6.4 | registers J9.6 |
| regs_b_lat_oe_n | cu J6.5 | registers J9.5 |
| regs_const_oe_n | cu J6.6 | registers J9.4 |
| regs_const_0 | cu J6.7 | registers J9.3 |
| regs_a_bus_oe_n | cu J6.8 | registers J9.2 |
| regs_res_oe_n | cu J6.9 | registers J9.1 |
| regs_a_sel0 | cu J9.19 | registers J4.1 |
| regs_a_sel1 | cu J9.20 | registers J4.2 |
| regs_b_sel0 | cu J9.13 | registers J13.1 |
| regs_b_sel1 | cu J9.14 | registers J13.2 |
| regs_cu_bank_n0 | cu J9.15 | registers J14.1 |
| regs_cu_bank_n1 | cu J9.16 | registers J14.2 |
| regs_cu_idx0 | cu J9.17 | registers J5.1 |
| regs_cu_idx1 | cu J9.18 | registers J5.2 |
| addr_inv | cu J6.10 | addr J2.4 |
| addr_off_sel | cu J6.11 | addr J2.3 |
| addr_cin | cu J6.12 | addr J2.5 |
| addr_lat_ld_n | cu J6.13 | addr J2.6 |
| addr_pc_ld_n | cu J6.14 | addr J2.7 |
| addr_sp_ld_n | cu J6.15 | addr J2.8 |
| addr_base_sel0 | cu J9.11 | addr J3.1 |
| addr_base_sel1 | cu J9.12 | addr J3.2 |
| addr_base_oe_n0 | cu J9.9 | addr J3.3 |
| addr_base_oe_n1 | cu J9.10 | addr J3.4 |
| addr_off_oe_n0 | cu J9.7 | addr J3.5 |
| addr_off_oe_n1 | cu J9.8 | addr J3.6 |
| addr_lat_oe_n0 | cu J9.5 | addr J3.7 |
| addr_lat_oe_n1 | cu J9.6 | addr J3.8 |
| addr_pc_oe_n0 | cu J9.3 | addr J3.9 |
| addr_pc_oe_n1 | cu J9.4 | addr J3.10 |
| addr_sp_oe_n0 | cu J9.1 | addr J3.11 |
| addr_sp_oe_n1 | cu J9.2 | addr J3.12 |
| ir_b0_ld_n | cu J6.16 | ir J1.2 |
| ir_b1_ld_n | cu J6.17 | ir J1.3 |
| ir_v1_ld_n | cu J6.18 | ir J1.4 |
| ir_v2_ld_n | cu J6.19 | ir J1.5 |
| mem_re | cu J6.20 | mem J2.2 |
| mem_oe_n | cu J6.21 | mem J2.3 |
| mem_we_n | cu J6.22 | mem J2.4 |
| mem_wbuf_oe_n | cu J6.23 | mem J2.5 |
| mem_rom_oe_n | cu J6.24 | mem J2.6 |
| ie | cu J6.25 | iod J1.1 |
| rw | cu J6.26 | iod J1.2 |
| inr | cu J6.27 | iod J1.3 |
| clk_0 | clkgen J2.1 | ir J1.1 |
| clk_1 | clkgen J2.2 | addr J2.1 |
| clk_2 | clkgen J2.3 | cu J1.1 |
| clk_3 | clkgen J2.4 | cu J1.2 |
| clk_4 | clkgen J2.5 | flags J1.8, registers J8.3 |
| clk_5 | clkgen J2.6 | iod J1.4 |
| clk_wr_n | clkgen J1.1 | registers J8.2, iod J1.5 |
| clk_mem_wr_n | clkgen J1.2 | mem J2.1 |
| rst_n | clkgen J1.3 | cu J1.3, flags J1.7, addr J2.2, iod J1.6 |
| rst_early | clkgen J1.4 | cu J1.4 |
| rst_rc | clkgen J1.5 | cu J1.5 |
