# Control unit: requirements from other sections

What the other sections need from the control unit (not designed yet). Add each new requirement here, with the section that needs it and why.

| # | Needed by | Requirement | Why |
|---|---|---|---|
| C1 | ALU (2026-11-04) | Drive `alu_op` (alu-report.md table) and give the ALU a full 500 ns step: operand register clock-to-Q + ALU + setup = 327 ns at 85 °C max. Put 01 on `alu_b` for INC/DEC (with the register/bus section, registers-requirements.md R1). | alu-decisions.md D9, D16, F1 |
| C2 | flag register (2026-11-04) | Drive `flags_op` and `flags_bit_val` from the flags_op table (flags-decisions.md). Schedule the **two-flag events as two steps**: interrupt entry = `1010` with 0 (I), then `1111` with 0 (H), overlapping the pushes and the vector fetch; invalid instruction = `1111` with 1 (H), then `1010` with 0 (I). `flags_op` must be valid 229 ns before the clock edge at 85 °C max. | The register writes one flag per clock; Gianni chose two steps over a fifth multiplexer source. |
| C3 | flag register (2026-11-04) | **STI delay**: I reads 1 right after STI, but interrupts must be ignored until the next instruction has run (`interrupt_delay` in cpu.rs). | Belongs to the control unit, not the register. |
| C4 | flag register (2026-11-04) | Drive `flags_oe_n` low only for PSF, when nothing else drives the data bus. | PSF (flags-decisions.md); bus side in registers-requirements.md R2. |
