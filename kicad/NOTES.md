# KiCad schematics: status and to-do (2026-11-11, updated 2026-11-14)

Electrical findings and open hardware items are in [kicad_electrical_requirements.md](kicad_electrical_requirements.md).

## Schematic status

| Unit | Folder | Chips | Against Logisim (`kicadcmp.py`, 2026-11-11) |
|---|---|---|---|
| addr | `addr/` | 34 | 8 differences, not errors: the in/out split of the low address byte (see below) |
| alu | `alu/` | 29 | equivalent |
| flags | `flags/` | 15 | equivalent |
| ir | `ir/` | 7 | equivalent |
| mem | `mem/` | 8 | equivalent |
| registers | `registers/` | 18 | equivalent (Logisim file `circuits/registers/regs.circ`) |
| iod | `iod/` | 12 | no wiring error (2026-11-14). The differences are expected: the in/out split of `iod_addr`, the series resistors R1–R5 on U12's outputs, U12 on +3V3, and the new connector nets (TTGO, A8–A15). ERC: 0 errors, 0 warnings. |
| clkgen | `cu/` (project `hbc2_clkgen`) | 5 | equivalent (Logisim file `circuits/cu/clkgen.circ`). The oscillator and the reset RC aren't on the board yet (to-do 8). |
| cu | `cu/` (project `ctrl_unit/hbc_ctrl_unit`) | 30 | equivalent (Logisim file `circuits/cu/cu.circ`; checked 2026-11-14). The 60 pull resistors are board extras. |

Every circuit is drawn (cu on 2026-11-13).

No board has a wiring error. On the 8 boards reviewed on 2026-11-11 (cu wasn't drawn yet):
- no floating input and no output conflict;
- every chip has VCC and GND, with one 100 nF capacitor per chip;
- every unused output carries a no-connect flag.

Check a schematic with (run from `logisim/`):
`python tools/kicadcmp.py circuits/<unit>/<circ>.circ ../kicad/<unit>/hbc2_<unit>.kicad_sch`

### Known comparer noise (not errors)

- **Logisim splits a bus into `X_in` and `X_out` nets** where the hardware has one wire.
  - On addr: `addr_bus_addr_lo_in_N` and the latch 541's output, KiCad U30. The comparer reports U30's Y pins as "only in KiCad".
  - On iod: `iod_addr_in` and `iod_addr_out`. Bits 2–5 of the input are reported as "KiCad net not found".
  - Possible fix: teach `kicadcmp.py` to merge the two nets.
- **`Net-(D2-A)` on the power connector** is the supply input through the Schottky diode D2, so it has no Logisim pin.
- **The uncommitted change to `addr/hbc2_addr.kicad_sch`** is a re-annotation only (U3–U36 became U1–U34). Connectivity is unchanged.

## To do before the PCBs

Details are in `kicad_electrical_requirements.md`.

1. ~~**IOD U12 → SN74LVC244A at 3.3 V.**~~ Done 2026-11-14 (H1). It takes its 3.3 V from the TTGO, through J5.8.
2. ~~**IOD: a 10 kΩ pull-down network on A8–A15**, plus connector pins for them.~~ Done 2026-11-14 (H2): RN1, and A8–A15 on J4 (now 1 × 16).
3. ~~**IOD: pull-downs on RCLK_RX and RCLK_EVT, and a pull-up on MBX_SEL.**~~ Not needed (H3): the TTGO's own resistors set those levels (see "IOD: the TTGO VGA32" below).
4. **Footprints on every board.** None are assigned except the mem AS6C1008 (H4).
5. **Supply: the series Schottky diode (D2) drops 0.3–0.5 V.** That leaves about 4.5–4.7 V against the 4.5 V minimum. Choose the part or the protection scheme (S1).
6. ~~**Relabel the IOD 595s as SN74AHCT595N.**~~ Done 2026-11-14. Still to do: check the other boards' part labels against `logisim/notes/chips.md` (S3).
7. **Run KiCad's ERC** on each board.
8. **clkgen: the 4.000 MHz oscillator module and the reset circuit** (resistor, capacitor, push button across the capacitor, diode across the resistor). They arrive on J1 (`clkgen_osc`, `clkgen_rc`). Put them on the board, or decide that J1 goes to a front panel (H5).

## IOD notes

- `iod_addr_in` and `iod_addr_out` are the same physical lines. One KiCad net per bit is correct.
- Do not put no-connect flags on those address lines. They are already on the unused chip pins: the spare 74138 outputs, the 7432's spare gate, and the spare 7474 outputs.
- `iod_vec` (U5 Q2) goes nowhere on purpose. Only `iod_vec_n` is used: it drives the EVT 595s' /OE, the evt_s /CLR and the evt_clr D input.
- The EVT 595s drive the address lines and the data bus in the vector step (/OE = `iod_vec_n`). Every other driver on those buses must be off then. Check this against the cu microcode once the cu schematic exists (S2).

## IOD: the TTGO VGA32 (2026-11-14)

The ESP32 board is the LilyGO TTGO VGA32 v1.4. It sits next to the I/O card, in a 3D-printed box, and is wired to it by three cables:

| Card | Cable | TTGO end |
|---|---|---|
| J5 (2 × 4, keyed, numbered like the TTGO's header) | straight IDC ribbon | header J2: MOSI, SCK, (SD CS, not connected on the card), MISO, tx_full, rx_full, GND, +3V3 |
| J6 (1 × 5, numbered like the mini-DIN; pin 2 not connected) | PS/2 cable | mouse port: RCLK_EVT (pin 1), GND (3), +5V_ESP (4), MBX_SEL (5) |
| J7 (1 × 4) | 3 wires + GND | soldered at IO25 (RCLK_RX), IO35 (evt_pend) and IO36 (rst_n) |

- **U12** (SN74LVC244AN) is powered from the TTGO's 3.3 V. There is no regulator on the card.
- **Series resistors:** 33 Ω on U12's outputs (R1, R3–R5), and 220 Ω on `rst_n` (R2), because the TTGO's button K1 sits on IO36.
- **D3** (1N5817) feeds the TTGO from +5V. With the TTGO's own diode on USB, the TTGO can be flashed while the HBC-2 runs.
- **`iod_int`** moved to J1, the CPU-side connector. It goes to the control unit.
- **Full record** (pins, reasons, firmware rules): `logisim/notes/iod-decisions.md`, "The TTGO VGA32 wired to the card".
- **`kicadcmp.py`** lists the TTGO-side nets and D3/RN1/R1–R5 as board extras or connector nets with no Logisim pin. That is expected.
