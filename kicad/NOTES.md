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
| clkgen | `cu/` (project `hbc2_clkgen`) | 5 | equivalent (Logisim file `circuits/cu/clkgen.circ`). Oscillator Y1, the reset circuit and the Nano driver added 2026-11-14; references renumbered from 1 the same day (the tables below use the new numbers). D1/D2 power input removed (S1). ERC: 0 errors, 0 warnings. |
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
- **The uncommitted change to `addr/hbc2_addr.kicad_sch`** is a re-annotation only (U3–U36 became U1–U34). Connectivity is unchanged.

## To do before the PCBs

Details are in `kicad_electrical_requirements.md`.

1. ~~**IOD U12 → SN74LVC244A at 3.3 V.**~~ Done 2026-11-14 (H1). It takes its 3.3 V from the TTGO, through J5.8.
2. ~~**IOD: a 10 kΩ pull-down network on A8–A15**, plus connector pins for them.~~ Done 2026-11-14 (H2): RN1, and A8–A15 on J4 (now 1 × 16).
3. ~~**IOD: pull-downs on RCLK_RX and RCLK_EVT, and a pull-up on MBX_SEL.**~~ Not needed (H3): the TTGO's own resistors set those levels (see "IOD: the TTGO VGA32" below).
4. **Footprints on every board.** None are assigned except the mem AS6C1008 (H4). clkgen: assigned on 2026-11-14 (chips in DIP sockets `Package_DIP:DIP-xx_W7.62mm_Socket`, 100 nF `C_Disc_D5.0mm_W2.5mm_P2.50mm`, card connectors `PinSocket_1x0n`, RV1 panel pot `Alps_RK09K`); still to set: D2 (1N5817, `Diode_THT:D_DO-41_SOD81_P10.16mm_Horizontal`), R6, and the power connector J3 (depends on the motherboard connector).
5. **Supply (S1), decided 2026-11-14:** the cards lose their series Schottky and their TVS; protection moves to the motherboard input. See "Power (S1)" below. Done on all 9 cards on 2026-11-14: D1/D2 removed, power connector GND, +5V, +5V, GND (addr J4, alu J5, flags J5, ir J10, mem J1, registers J7, iod J2, cu J4, clkgen J3). ERC 0 / 0 on each; no other pin changed net.
6. ~~**Relabel the IOD 595s as SN74AHCT595N.**~~ Done 2026-11-14. Still to do: check the other boards' part labels against `logisim/notes/chips.md` (S3).
7. **Run KiCad's ERC** on each board.
8. ~~**clkgen: the 4.000 MHz oscillator module and the reset circuit.**~~ Done 2026-11-14 (H5). See "clkgen: oscillator and reset" below.
9. **clkgen: the Arduino Nano clock and reset driver**: on the schematic 2026-11-14; the sketch is still to write. See "clkgen: Arduino Nano clock and reset driver" below.

## clkgen: oscillator and reset (2026-11-14)

| Ref | Part | Value | Pins / nets |
|---|---|---|---|
| Y1 | ECS-2200BX-040, half-size DIP-8 can (`Oscillator:CXO_DIP8`, `Oscillator_DIP-8`) | 4.000 MHz, 5 V HCMOS | 1 NC (tied to +5V, safe for a replacement with an enable on pin 1), 4 GND (case), 5 → `clkgen_osc`, 8 +5V |
| R3 | resistor | 10 kΩ | +5V → `clkgen_rc` |
| C9 | electrolytic | 10 µF 16 V | + on `clkgen_rc`, − on GND |
| D1 | 1N4148 | | anode `clkgen_rc`, cathode +5V (empties C9 at power-off) |
| SW3 | tactile push button, NO | | `clkgen_rc` → R5 → GND |
| R5 | resistor | 100 Ω | in series with SW3 (limits the C9 discharge to 50 mA, τ = 1 ms) |

- `clkgen_rc` = U1.1 (74HCT14 input). `clkgen_osc` = U2.3 (FF1 CLK), U4.2 (NAND3), U4.3 (the gate that inverts `osc`).
- Power-up reset: τ = 100 ms; the 74HCT14 switches at about 1.9 V, about 48 ms. The oscillator starts in 10 ms max, so the clock runs well before the release (C17 needs edges).
- **Rig tests** happen on breadboards, where the rig drives both nets (no Y1, no C9 there). On the PCB, the old rig connector is gone (Nano driver, below).
- **Layout:** Y1 close to U2 and U4, with its own 100 nF at pin 8; C9 and R3 near U1.1; SW3 at the board edge. A front-panel button can go on a 2-pin header in parallel with SW3.
- **Datasheet:** `logisim/circuits/chips/ecs_2100x.pdf` (ECS-2100X series, NRND). It doesn't list the 2200 series: confirm the ECS-2200BX is a 5 V part before ordering. Supply range 4.75–5.25 V, symmetry 45–55 % (Q9), rise / fall 5 ns, HCMOS load 50 pF, start-up 10 ms, 0–70 °C.
- **Supply:** the oscillator needs 4.75 V minimum. With the series Schottky gone (S1), the card gets the motherboard's 5 V less the plane drop.

## clkgen: Arduino Nano clock and reset driver (2026-11-14)

Goal: slow clocks and step by step from a button, with the boot (65,548 steps) still at full speed. The Nano sits in a socket on the clkgen board. Logisim: no change (`clkgen_osc` and `clkgen_rc` stay the module's inputs, like Y1 and the RC).

**Why it works:** every clkgen output follows `osc` edges in a fixed order, not gate delays, and every chip is static, so `osc` can run at any rate down to 0 Hz. The only electrical rule is clean edges (HCT clock inputs want under about 500 ns): a timer output is fine, a bare button or an RC isn't.

**Parts (on the schematic 2026-11-14, netlist and ERC checked):**

| Ref | Part | Connection |
|---|---|---|
| A1 | Arduino Nano v3 (`MCU_Module:Arduino_Nano_v3.x`, footprint `Module:Arduino_Nano`, in 2 × 1×15 female headers) | 5V pin fed from +5V through D2 (not VIN), both GND |
| D2 | 1N5817 | anode +5V, cathode the Nano's 5V pin: the card feeds the Nano, USB can never feed the card (S1) |
| R6 | 10 kΩ | +5V → A1: power sense. The sketch drives nothing while A1 reads low |
| JP1 | 3-pin jumper (`Jumper:Jumper_3_Open`, 1×03 header) | 1 = Y1.5, 2 = `clkgen_osc`, 3 = the Nano clock. Selects Y1 (standalone, no Nano) or the Nano |
| R4 | 33 Ω | D9 (OC1A) → JP1.3: damps the 4 MHz edges |
| R1 | 10 kΩ | `clkgen_osc` → GND: holds `osc` low while the Nano boots (its pins are inputs) |
| R2 | 220 Ω | D4 → `clkgen_rc`: the Nano holds the reset (OUTPUT LOW) or lets go (INPUT); limits the current into C9 |
| SW1 | tactile push button | D5 → GND, STEP (internal pull-up, software debounce) |
| SW2 | tactile push button | D6 → GND, RUN / STOP (internal pull-up, software debounce) |
| RV1 | 10 kΩ potentiometer | the Nano's 5V pin / GND, wiper on A0: speed in RUN (top of the range = 4 MHz) |
| | `clk` feedback | `clk_5` → D2 (INT0): the sketch knows the step phase |
| | `rst_n` feedback | `rst_n` → D3 (INT1): the sketch sees the reset release |

The Nano's D13 LED shows run / stop. The old rig connector J1 is removed (see "J1" below). After the renumbering, J1 and J2 are the clock / reset output connectors (5 and 6 pins) and J3 the power connector.

**Sketch (to write):**
- Timer1 in CTC, toggle OC1A on compare: OCR1A = 1 with no prescaler gives exactly 4.000 MHz at 50 % (the 16 MHz crystal); the prescaler and OCR1A give slow rates down to well under 1 Hz. Timer0 (millis) is left alone.
- Power sense first: while A1 reads low, D9 and D4 stay inputs (Timer1 off). Start: D4 low (reset held), clock started, D4 released, wait for `rst_n` high, run until stopped (the boot copy takes 33 ms at 4 MHz).
- Stop: Timer1's clock off (TCCR1B = 0). The pin only changes on a compare match, so it keeps its level: no runt pulse.
- Step: toggle `osc` (force compare FOC1A, or D9 as a GPIO) until `clk_5` rises: one press = one CPU step (2 `osc` periods).
- **Auto-reset:** opening the serial port resets the Nano (DTR). Its pins become inputs: `osc` stops low (R1) and the CPU freezes, then the sketch starts again and reboots the CPU. Accepted; the sketch could skip the reset if `rst_n` is already high.

**J1 (old):** it only existed so the breadboard rig could drive `clkgen_osc` and `clkgen_rc`. The rig tests happen on breadboards, before the PCB, so the PCB doesn't need it. `kicadcmp.py` will then report the two nets under "Logisim top-level pins whose KiCad net has no connector": expected (they are driven on the board). Optional: test points on `clkgen_osc`, `clk_5` and `rst_n` for the scope.

**Loads:** the Nano's D2 and D3 pins add one input each (about 10 pF) on `clk_5` and `rst_n`: negligible next to the branch loads.

**Power (S1, settled 2026-11-14):** on a Nano v3, USB feeds the 5V pin through its own Schottky. D2 (1N5817) feeds that pin from the card, so the Nano runs from either side at about 4.7 V and can never power the CPU from USB. If USB is plugged in while the CPU is off, R6 lets the sketch see it and keep D9 and D4 as inputs, so nothing pushes current into the unpowered chips' input clamps.

**Firmware rule (ESP32):** the CPU can be stopped in the middle of a mailbox transfer, so the ESP32 never times out on the mailbox (iod-decisions.md firmware rules).

## Power (S1, decided 2026-11-14)

Budget (estimates): about 0.6 A typical, 1.6 A worst case for the whole CPU. The largest are cu (7 EEPROMs always selected, 60 pull-ups: up to about 450 mA) and iod (TTGO VGA32: up to about 450 mA in Wi-Fi bursts). The Mean Well GST25E05 (5 V 4 A) covers it.

**Cards (done on all 9, 2026-11-14):** no series diode and no TVS. The power connector goes straight to +5V / GND; keep the 47 µF and one 100 nF per chip. Pinout GND, +5V, +5V, GND (clkgen J3), so a card turned 180° still gets the right polarity.

**Motherboard input (to design):** barrel jack → switch → fuse T 3.15 A → +5V plane. Across the rail: a unidirectional TVS (P6KE6V8A, 5.8 V stand-off) and 1000 µF 10 V. Reversed polarity drives the TVS forward and blows the fuse or trips the adapter's protection; in normal use only the fuse (about 0.05 V) is in series.
- GND pour on one layer, +5V as a wide pour or a ≥ 3 mm trace (1 mm × 30 cm drops about 0.3 V at 2 A). cu and iod closest to the input.
- Measure the rail at the farthest card under load. Below about 4.9 V (the oscillator needs 4.75 V): adjustable supply (e.g. Mean Well RS-25-5) set to 5.1 V.

**Things that can feed the CPU from outside:** the Nano (D2 + R6 sense, clkgen) and the TTGO (iod D3). Both are diode-isolated.

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
