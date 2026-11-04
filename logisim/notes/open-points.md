# Open points (rule conflicts to settle with the user)

- **add_sub8 symbol layout.** The custom symbol has the control inputs (`sub`, `usec`, `cin`) on the left with the data, and the flag `c` on the right, at the top. The layout rule wants control on top and flags at the bottom. Only the names were updated (2026-11-04). Moving the ports would change the symbol's shape, so ask first.
- **add_sub8 chip notes.** Rule 4 suggests a Text for non-obvious chips: U1 (7408, carry gating), U2 (7486, carry/borrow inversion), U5/U6 (7486, B inversion bits 3..0 / 7..4). Not added yet: it's the user's hand-drawn layout.

Settled:
- 2026-11-04: reference model = emulator code via `hbc2 vectors`.
- 2026-11-04: chip numbering has no exceptions.
- 2026-11-04: Constant is allowed in modules.
- 2026-11-04: floating subcircuit inputs are warnings.
- 2026-11-04: file layout: one module per `circuits/<m>/<m>.circ`, and cpu.circ holds only main and loads libraries.
- 2026-11-04: folders are functional units (`circuits/alu/add8.circ`), not one folder per module.
