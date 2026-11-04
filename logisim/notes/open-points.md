# Open points (rule conflicts to settle with the user)

None right now.

Settled:
- 2026-11-04: reference model = emulator code via `hbc2 vectors`.
- 2026-11-04: chip numbering has no exceptions.
- 2026-11-04: Constant is allowed in modules.
- 2026-11-04: floating subcircuit inputs are warnings.
- 2026-11-04: file layout: one module per `circuits/<m>/<m>.circ`, and cpu.circ holds only main and loads libraries.
- 2026-11-04: folders are functional units (`circuits/alu/add8.circ`), not one folder per module.
- 2026-11-04: 1-bit taps allowed for every TTL chip whose pins mix inputs and outputs (gates, 74283); see ttl-chips.md.
- 2026-11-04: add8 became add_sub8 (subtraction by B inversion); the user's earlier add_sub8 test is gone.
