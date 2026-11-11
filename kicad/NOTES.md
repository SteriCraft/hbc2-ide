# KiCad schematics: status and to-do (2026-11-11)

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
| iod | `iod/` | 12 | 4 warnings, not errors: the in/out split of `iod_addr` |
| clkgen | `cu/` (project `hbc2_clkgen`) | 5 | equivalent (Logisim file `circuits/cu/clkgen.circ`). The oscillator and the reset RC aren't on the board yet (to-do 8). |
| **cu** | `cu/` | – | **missing**: `logisim/circuits/cu/cu.circ`, about 30 chips plus 7 microcode EEPROMs |

One circuit is left: cu.

No board has a wiring error. On all 8 boards:
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

1. **IOD U12 → SN74LVC244A at 3.3 V.** It is drawn as a 74HCT244 on +5V and would put 5 V on the ESP32's pins (H1).
2. **IOD: a 10 kΩ pull-down network on A8–A15**, plus connector pins for them. J4 carries A0–A7 only (H2).
3. **IOD: pull-downs on RCLK_RX and RCLK_EVT, and a pull-up on MBX_SEL**, so the ESP32's boot can't make a clock edge (H3).
4. **Footprints on every board.** None are assigned except the mem AS6C1008 (H4).
5. **Supply: the series Schottky diode (D2) drops 0.3–0.5 V.** That leaves about 4.5–4.7 V against the 4.5 V minimum. Choose the part or the protection scheme (S1).
6. **Relabel the IOD 595s as SN74AHCT595N.** Check the other part labels against `logisim/notes/chips.md` (S3).
7. **Run KiCad's ERC** on each board.
8. **clkgen: the 4.000 MHz oscillator module and the reset circuit** (resistor, capacitor, push button across the capacitor, diode across the resistor). They arrive on J1 (`clkgen_osc`, `clkgen_rc`). Put them on the board, or decide that J1 goes to a front panel (H5).

## IOD notes

- `iod_addr_in` and `iod_addr_out` are the same physical lines. One KiCad net per bit is correct.
- Do not put no-connect flags on those address lines. They are already on the unused chip pins: the spare 74138 outputs, the 7432's spare gate, and the spare 7474 outputs.
- `iod_vec` (U5 Q2) goes nowhere on purpose. Only `iod_vec_n` is used: it drives the EVT 595s' /OE, the evt_s /CLR and the evt_clr D input.
- The EVT 595s drive the address lines and the data bus in the vector step (/OE = `iod_vec_n`). Every other driver on those buses must be off then. Check this against the cu microcode once the cu schematic exists (S2).

## IOD: ESP32 on the PCB

The ESP32 can go on the IOD board, behind the LVC244. Logisim models only its interface. Pin list and constraints: `kicad_electrical_requirements.md`, section "ESP32 on the IOD board".

- The ESP32 has no symbol or footprint in the schematic yet. Use a DevKitC-style header or the bare module.
- 3.3 V supply, shared with the LVC244.
- Reset line `iod_mcu_rst_n`, and a reachable USB port for loading programs.
- `kicadcmp.py` will list it under "board extras" and not compare it.
