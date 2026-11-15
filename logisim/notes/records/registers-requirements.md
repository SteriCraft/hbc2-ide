# Register and bus section: requirements from other sections

What the other sections need from the register / bus section (`circuits/registers/regs.circ`, built 2026-11-04, see [registers-decisions.md](registers-decisions.md)). Add each new requirement here, with the section that needs it and why.

| # | Needed by | Requirement | Why |
|---|---|---|---|
| R1 | ALU (2026-11-04) | **Put 01 on `alu_b`** for INC and DEC. Probably one 74HCT541 with its inputs tied to 0000 0001, enabled onto the B operand path by the control unit. | INC/DEC are ADD/SUB with b = 01 (alu-decisions.md D16, fix F3); the ALU has no INC/DEC hardware of its own. |
| R2 | flag register (2026-11-04) | `flags_bus_in` reads the data bus and `flags_bus_out` drives it (tri-state, `flags_oe_n`). The bus must be free when PSF drives it, and hold the popped byte through the clock edge for POF (valid by 392 ns into the step, flags-decisions.md "Timing"). | POF / PSF (flags-decisions.md). |
| R3 | registers (2026-11-04) | The register file can use the **74HCT670** (4 × 4, in Logisim's #TTL). Logisim's 74670 matches the datasheet, including the transparent write: while /WE is low, the addressed word follows D (data to output 63 ns max at 85 °C). Checked by `circuits/chips/test_74670.circ`. | Next section. |
| R4 | registers (2026-11-04) | The **74HCT574** is available as a chip model (`circuits/chips/74574.circ`, from the TI datasheet). | Next section. |

Status (2026-11-04, module `regs`):
- **R1 done**: U9 (74HCT541) puts 0000000c on bus B (`regs_b`, to `alu_b`) when `regs_const_oe_n` is low; c = `regs_const_0` (1 for INC/DEC, 0 for the reset zeros).
- **R2**: the section drives `bus_data` only through `regs_bus_out` (`regs_res_oe_n`, `regs_a_bus_oe_n`), both off unless the control unit enables them, so the bus is free for PSF (control-requirements.md C5). The section never stores the data bus on POF.
- **R3, R4 used**: 8 × 74HCT670, 3 × 74HCT574. The 574 model circuit is now named `ttl74574` (registers-decisions.md D12).
