# Open points (rule conflicts to settle with the user)

None right now.

Settled:
- 2026-11-04: reference model = emulator code via `hbc2 vectors`.
- 2026-11-04: chip numbering has no exceptions.
- 2026-11-04: Constant is allowed in modules.
- 2026-11-04: floating subcircuit inputs are warnings.
- 2026-11-04: file layout: one module per `circuits/<m>/<m>.circ`, and cpu.circ holds only main and loads libraries.
- 2026-11-04: folders are functional units (`circuits/alu/add8.circ`), not one folder per module.
- 2026-11-04: add_sub8 symbol: control (sub, usec, cin) on top, a/b left, s right, c bottom.
- 2026-11-04: add_sub8 role notes added next to U1, U2, U5 and U6.
