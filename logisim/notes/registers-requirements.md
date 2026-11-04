# Register and bus section: requirements from other sections

What the other sections need from the register / bus section (`circuits/registers/`, not designed yet). Add each new requirement here, with the section that needs it and why.

| # | Needed by | Requirement | Why |
|---|---|---|---|
| R1 | ALU (2026-11-04) | **Put 01 on `alu_b`** for INC and DEC. Probably one 74HCT541 with its inputs tied to 0000 0001, enabled onto the B operand path by the control unit. | INC/DEC are ADD/SUB with b = 01 (alu-decisions.md D16, fix F3); the ALU has no INC/DEC hardware of its own. |
