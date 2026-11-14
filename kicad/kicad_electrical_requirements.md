# KiCad boards: electrical requirements and open findings

Review of 2026-11-11 for the schematics addr, alu, flags, ir, mem, registers, iod and clkgen (clkgen added later the same day). Updated 2026-11-14: cu (drawn 2026-11-13) compares as equivalent; the IOD changes for the TTGO VGA32 closed H1, H2, H3 and S3 on that board.

Two checks were run on each board:
- **Equivalence with Logisim:** `logisim/tools/kicadcmp.py`.
- **Electrical check of the KiCad netlist:** floating inputs, several drivers on one net, supply on every chip, decoupling, no-connect flags, footprints.

## Equivalence with Logisim

| Board | Chips | Result |
|---|---|---|
| alu | 29 | equivalent |
| flags | 15 | equivalent |
| ir | 7 | equivalent |
| mem | 8 | equivalent |
| registers (`regs.circ`) | 18 | equivalent |
| clkgen (`kicad/cu/hbc2_clkgen`, `circuits/cu/clkgen.circ`) | 5 | equivalent |
| cu (`kicad/cu/ctrl_unit/hbc_ctrl_unit`, `circuits/cu/cu.circ`) | 30 | equivalent (2026-11-14) |
| addr | 34 | 8 differences, not errors |
| iod | 12 | no wiring error (2026-11-14; expected differences only, see `NOTES.md`) |

No wiring error on any board.

- **addr:** Logisim splits the low address byte into the 541's output (`addr_bus_addr_lo_N`) and a read-back net (`addr_bus_addr_lo_in_N`). In hardware they are one wire. KiCad has one net per bit, `/addr_bus_addr_loN`, with U30 (latch low 541) as its only driver, and the comparer reports U30's Y pins as "only in KiCad".
- **iod:** the same split exists for `iod_addr_in` / `iod_addr_out`. Its bits 2–5 go nowhere in Logisim, so the comparer reports "KiCad net not found". The nets are correct in KiCad.
- **The connector net `Net-(D2-A)`**, reported on every board, is the supply input through D2 (see S1).
- **The uncommitted change to `addr/hbc2_addr.kicad_sch`** is a re-annotation only (U3–U36 became U1–U34, capacitors renumbered). Connectivity and the comparison result are unchanged.

Possible tool improvement: teach `kicadcmp.py` to merge Logisim `X_in_N` and `X_out_N` into one net when only one of them is wired, so that addr and iod report equivalent.

Commands (from `logisim/`):
```
python tools/kicadcmp.py circuits/<unit>/<circ>.circ ../kicad/<unit>/hbc2_<unit>.kicad_sch
```
The registers board uses `circuits/registers/regs.circ`.

## Must fix before the PCBs (hardware, not visible in Logisim)

### H1. IOD: the ESP32-side buffer must be an LVC244 at 3.3 V

**Done 2026-11-14.** U12 is an SN74LVC244AN. Its VCC is the TTGO's 3.3 V, through J5.8, with C12 decoupling it on +3V3; there is no regulator on the card. Its outputs go to the TTGO through R1–R5 (33 Ω; 220 Ω on `rst_n`). The text below is the original finding.

- KiCad U12 (Logisim U12) is drawn as a **74HCT244 on +5V**. It drives J5 to the ESP32: `iod_miso`, `iod_mcu_tx_full`, `iod_mcu_rx_full`, `iod_mcu_evt_pend`, `iod_mcu_rst_n`.
- The design calls for an **SN74LVC244AN powered at 3.3 V** (`logisim/notes/iod-requirements.md`, `chips.md`). Its inputs are 5 V tolerant and its outputs are at 3.3 V.
- As drawn, 5 V reaches the ESP32's GPIOs, which can damage it.
- Needed: change the part to 74LVC244, add a 3.3 V net for its VCC, and decouple it on 3.3 V.
- Where the 3.3 V comes from depends on the ESP32 decision (below): the module's own regulator, or a regulator on the board.
- The other direction is fine. The ESP32's 3.3 V outputs (`iod_sck`, `iod_mosi`, `iod_rclk_rx`, `iod_rclk_evt`, `iod_mbx_sel`) feed HCT/AHCT inputs, whose VIH is 2.0 V.

### H2. IOD: pull-downs on A8–A15

**Done 2026-11-14.** RN1 (10 kΩ × 8 bussed, common pin on GND) pulls `iod_addr8..15` down, and J4 (now 1 × 16) carries A0–A15. Breadboard check B5 (b) still applies.

- Decision D3 / rule I6: in the vector step the IOD drives A0–A7 only, and a **10 kΩ bussed network pulls A8–A15 to 0**. See `io-redesign-proposal.md` 9.9 and `iod-decisions.md` K2.
- Logisim leaves them out on purpose, because a pull inside the module would fight the address latch.
- The IOD board has no resistors, and J4 carries A0–A7 only.
- Needed: connector pins for A8–A15 and a 10 kΩ × 8 bussed SIP-9 (Bourns 4609X-101-103LF, common pin to GND).
- Breadboard check B5 (b) still decides whether the slow edge is acceptable. The fallback is a 74HCT541 with inputs tied low.

### H3. IOD: pulls on the ESP32 control lines

**Settled 2026-11-14 without parts on the card.** The TTGO sets every idle level itself:
- **MBX_SEL** (IO26) and **RCLK_EVT** (IO27) come through its mouse port, which pulls them up to 5 V (4.7 kΩ). Both idle high, so an ESP32 reset makes no rising edge.
- **RCLK_RX** (IO25) is held low by the audio RC (420 Ω to GND).

See `logisim/notes/iod-decisions.md`, "The TTGO VGA32 wired to the card". The text below is the original finding.

- `io-redesign-proposal.md` section 9 asks for **pull-downs on RCLK_RX and RCLK_EVT**, so that the ESP32's boot (floating or toggling GPIOs) can't make a storage-clock edge.
- **MBX_SEL** idles high. It is the 165's CLK INH, the LVC244's MISO enable and the TX clear on its rising edge. A **pull-up** keeps it inactive while the ESP32 boots.
- None of these resistors is on the board.
- Note: if the pull-up on MBX_SEL goes to 5 V, it sits on an ESP32 pin. Pull it to 3.3 V, or put it on the card side only if the ESP32 pin is protected.

### H4. Footprints on every board
- No component has a footprint. The only exception is the AS6C1008 (U2) on mem.
- The PCB can't be updated from the schematic until all of them are assigned:
  - DIP sockets for the chips (turned-pin for the 7 microcode EEPROMs on cu);
  - the capacitors;
  - D1 / D2;
  - the connector headers.

### H5. clkgen: the oscillator and the reset circuit aren't on the board
- `clkgen_osc` (J1.2) and `clkgen_rc` (J1.1) come in on a header. The board has no oscillator, and no reset resistor, capacitor, button or diode.
- Logisim leaves these parts out because they are analog (`cu-decisions.md` sections 13 and 14, `chips.md`). The test rig drives both nets, and the parts are fitted afterwards (`test-rig.md`).
- The design calls for:
  - a **4.000 MHz HCMOS oscillator module**: 5 V, DIP-8 or DIP-14 can, 45–55 % duty, reference still to choose;
  - the **reset RC on the 74HCT14 input (U1.1)**: a resistor, a capacitor, a push button across the capacitor, and a diode across the resistor (C17).
- Options:
  - put them on the board, with the oscillator on its own 100 nF and close to the clock 74HCT74 (U2) and the 74HCT10 (U4);
  - or keep J1 and decide where they go (a front panel, for example). The oscillator's 4 MHz edges are fast, so keep that wire short and run a ground next to it.
- If the oscillator is socketed for the rig tests, J1 can stay as the rig's input.

## Electrical warnings

### S1. Supply drop in the series Schottky diode (all boards)
- Each board takes its supply on pins 3 and 4 of its power connector, through D2 (Schottky, anode on the connector, cathode on +5V), with D1 (TVS) across +5V/GND.
- At a few hundred mA, a Schottky drops about 0.3–0.5 V, so the board runs at **about 4.5–4.7 V**.
- The minimum supply is **4.5 V** for the 74HCT parts, the AT28C256 and the AS6C1008.
- The worst cases are cu (seven EEPROMs always selected, about 350 mA) and addr (34 chips).
- Options:
  - a low-Vf Schottky sized for the board current, with Vf checked at that current;
  - reverse-polarity protection with a P-MOSFET (almost no drop);
  - a supply set to 5.1–5.2 V.
- Also check that the boards' rails stay close to each other. The cards drive each other's inputs, and HCT inputs tolerate VCC + 0.5 V at most.
- Choose part numbers for D1 and D2 (TVS stand-off voltage about 5 V, e.g. an SMAJ5.0A or a through-hole equivalent).

### S2. Shared buses between the boards
- IOD U6 (EVT port 595) drives the address lines A0–A7, and U9 (EVT data 595) drives the data bus. Both are enabled by `iod_vec_n` in the vector step.
- On the shared bus, every other driver must be off in that step:
  - the address latch's 541s on addr (U29/U30 in KiCad; `addr_lat_oe_n0/1` high in the cu word);
  - every data-bus driver (ALU, registers, PC/SP, mem, IOD STATUS and RX).
- The Logisim CPU tests cover this logically. Check it again when the cu schematic exists, and on the scope during the partial integration (breadboard checks B1, B3, B5).
- The same rule applies on every shared bus: **one /OE active at a time**. For example, mem has two AT28C256s on `mem_rom_d0..7` and three 541s on the CPU buses.

### S3. Part labels against the BOM

**IOD done 2026-11-14** (U3, U6 and U9 are labelled SN74AHCT595N). The other boards are still to check.

- The IOD 595s (U3, U6, U9) are labelled **74HCT595**. The part bought is the **SN74AHCT595N**: TI makes no 74HCT595 in DIP. The pinout is the same and the AHCT part is faster.
- Relabel them so that the PCB's BOM is right.
- Check the other part labels against `logisim/notes/chips.md` when assigning footprints. Examples: CD74HCT… parts, and the AS6C1008-xxP speed grade.

### S4. Unused pins (IOD and others)
- On every board, each unused output carries a no-connect flag, and each unused input is tied to GND or +5V. This was checked on all 7 boards.
- On the IOD, these are left open with a flag:
  - `iod_vec` (U5 Q2): unused; only its complement `iod_vec_n` is used;
  - the 74138's spare outputs;
  - the 7432's spare gate output;
  - the 74HCT74s' spare outputs.
- Do not put no-connect flags on the `iod_addr_in[2..5]` lines. They are the shared address nets to J4 and the 595 outputs.

- On clkgen, unused pins are handled as follows:
  - the 74HCT14's 4 spare inputs and the 74HCT10's spare gate go to GND;
  - pins 4 and 5 of 74HCT10 gate 2 go to +5V (that gate is the NOT of `osc` for FF2);
  - the 74HCT74s' unused /S and /R go to +5V;
  - the reset synchroniser's D1 (U3.2) goes to +5V;
  - the 541's G1/G2 go to GND, since the clock buffer is always enabled;
  - every unused output has a flag: 4 on the 14, gate 3 of the 10, and U3 /Q2.

### S5. Checked and clean (all 8 boards)
- No floating chip input. Every input is driven, tied, or on a connector.
- No net with two push-pull outputs, and no push-pull output sharing a net with a tri-state one.
- Every chip has VCC and GND.
- Decoupling: one 100 nF per chip, plus one 47 µF 16 V bulk capacitor and a TVS per board.

## The TTGO VGA32 wired to the IOD board (2026-11-14)

The ESP32 board is the LilyGO TTGO VGA32 v1.4. It sits next to the card, in a 3D-printed box, and isn't on the PCB. Logisim models only its interface.

| Card connector | TTGO end | Signals |
|---|---|---|
| J5, 2 × 4 (keyed, numbered like the TTGO's J2) | header J2, straight IDC ribbon | MOSI (IO12), SCK (IO14), not connected (IO13 = SD CS), MISO (IO2), tx_full (IO34), rx_full (IO39), GND, +3V3 |
| J6, 1 × 5 (numbered like the mini-DIN) | PS/2 mouse port | RCLK_EVT (IO27, pin 1), not connected (pin 2), GND (pin 3), +5V_ESP (pin 4), MBX_SEL (IO26, pin 5) |
| J7, 1 × 4 | soldered wires | RCLK_RX (IO25), evt_pend (IO35), rst_n (IO36), GND |

Electrical points:
- **Supply:** D3 (1N5817) takes +5V to `+5V_ESP`. The TTGO's own BAT20J on USB VBUS completes a diode OR, so neither supply feeds the other. The TTGO's ME6211 regulator still gives 3.3 V from about 4.3 V.
- **U12 runs from the TTGO's 3.3 V**, so it is powered exactly when the ESP32 is. For rig tests without the TTGO, supply 3.3 V on J5.8 (the Mega's 3.3 V pin).
- **Series resistors:** 33 Ω on U12's outputs against ringing on the cables; 220 Ω on `rst_n`, because the TTGO's button K1 grounds IO36.
- **ESP32 outputs** (SCK, MOSI, MBX_SEL, RCLK_RX, RCLK_EVT) drive HCT / AHCT inputs at 3.3 V or 5 V; VIH is 2.0 V.
- **MBX_SEL and RCLK_EVT** rise through the TTGO's 4.7 kΩ pull-ups, so their edges are slow. They are the active edges (TX clear, EVT store). Breadboard check B7.
- **Full record** (why each pin, firmware rules): `logisim/notes/iod-decisions.md`, "The TTGO VGA32 wired to the card".

## Method (to re-run)

1. Export each netlist:
   `"C:\Program Files\KiCad\10.0\bin\kicad-cli.exe" sch export netlist -o <out>.net kicad/<unit>/hbc2_<unit>.kicad_sch`
2. Electrical checks on the netlist:
   - chip inputs with no driver and no connector;
   - nets with more than one `output` pin, or with an `output` and a `tri_state` pin;
   - every `U` on GND and on a supply;
   - capacitors between +5V and GND;
   - empty footprint fields.
3. Run the comparer above for each board.
4. Run KiCad's ERC as well. It sees the no-connect flags and pin types, which the comparer does not check.
