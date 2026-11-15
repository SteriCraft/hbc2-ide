# KiCad design notes

Technical details behind the schematics: why each board-level part is there and how it is wired. Status is in [README.md](README.md), open work in [TODO.md](TODO.md), parts in [parts.md](parts.md).

## Card slots: PCIe, through-hole (2026-11-14)

- **Decided by Gianni:** the cards stand vertically in a 3D-printed case and plug into the motherboard through **through-hole PCIe slots**, used as plain connectors (no PCIe protocol). He solders everything by hand.
- **Card edges:** 1.6 mm cards with a gold-finger edge. The KiCad footprints are `Connector_PCBEdge:BUS_PCIexpress_x1/x4/x8/x16`.
- **Motherboard footprint:** KiCad has none for the slot. The parts are Amphenol FCI 10018783 (x1 -10010TLF, x4 -11101TLF, x8 -11112TLF, x16 -10013TLF). Their footprints are in `lib/hbc2_slots.pretty`, written by `lib/gen_pcie_slots.py`: the x1 is copied from SamacSys, the x4, x8 and x16 are extrapolated (peg holes to check against a real slot).
- **Symbols:** `custom_symbols:HBC2_PCIe_x1/x4/x8/x16`, written by `lib/gen_pcie_symbols.py` on 2026-11-14.
  - Pins are numbered A1…An / B1…Bn, all passive, with names hidden. B is on the left, A on the right, and a gap row marks the key.
  - The default footprint is the slot. On a card, change it to `Connector_PCBEdge:BUS_PCIexpress_xN`.
  - The pin numbers match both footprints exactly (checked).
- **Pinout and slot order:** [slot-pinout.md](slot-pinout.md), drawn on the motherboard on 2026-11-15. Every shared signal has the same pin number on every card that uses it. Sizes: flags x4, alu x8, registers x16, ir x16, addr x16, cu x16, mem x8, iod x4, clkgen x1.
- **Motherboard: done 2026-11-15.** Its 66 connectors became 9 slots, plus RN1 (A8–A15 pull-downs) and the power input; ERC 0 / 0, nets checked against Logisim (motherboard-netmap.md).
- **Cards: done 2026-11-15.** Each card's 2.54 mm connectors (except the iod TTGO connectors and clkgen JP1) became one card edge, `HBC2_PCIe_xN` (J1 on every card, J3 on iod) with footprint `Connector_PCBEdge:BUS_PCIexpress_xN`. The slot pins per signal, with each card's own net names, are in [motherboard-netmap.md](motherboard-netmap.md). Checks on every card: edge pins equal its motherboard slot pin for pin, nets unchanged apart from the connectors, ERC 0 / 0, `kicadcmp.py` result unchanged. The old 4-pin power connectors are gone: power comes in on the edge's power pins.
  - **clkgen: done 2026-11-15.** Old J1 (clock/reset, 5 pins), J2 (clk0–5) and J3 (power) replaced by J1 `HBC2_PCIe_x1`, footprint `Connector_PCBEdge:BUS_PCIexpress_x1`: 11 signals, +5V on B1/B7/B13, GND on A1/A7/A13, the 19 other pins no-connect. All 36 pins match motherboard slot J9 (J67 before the renumbering) (card `clk0`–`clk5` = motherboard `clk_0`–`clk_5`). ERC 0 / 0; no other net changed; kicadcmp unchanged (only `clkgen_rc`, expected). The bulk capacitors C6/C7 moved 10.16 mm right.
- **Parts:** [parts.md](parts.md).

## clkgen: oscillator and reset (2026-11-14)

| Ref | Part | Value | Pins / nets |
|---|---|---|---|
| Y1 | ECS-2200BX-040, half-size DIP-8 can (`Oscillator:CXO_DIP8`, `Oscillator_DIP-8`) | 4.000 MHz, 5 V HCMOS | 1 NC (tied to +5V, safe for a replacement with an enable on pin 1), 4 GND (case), 5 → `clkgen_osc`, 8 +5V |
| R4 | resistor | 10 kΩ | +5V → `clkgen_rc` |
| C9 | electrolytic | 10 µF 16 V | + on `clkgen_rc`, − on GND |
| D1 | 1N4148 | | anode `clkgen_rc`, cathode +5V (empties C9 at power-off) |
| SW3 | tactile push button, NO | | `clkgen_rc` → R5 → GND |
| R5 | resistor | 100 Ω | in series with SW3 (limits the C9 discharge to 50 mA, τ = 1 ms) |

- `clkgen_rc` = U1.1 (74HCT14 input). `clkgen_osc` = U2.3 (FF1 CLK), U4.2 (NAND3), U4.3 (the gate that inverts `osc`).
- Power-up reset: τ = 100 ms; the 74HCT14 switches at about 1.9 V, about 48 ms. The oscillator starts in 10 ms max, so the clock runs well before the release (C17 needs edges).
- **Rig tests** happen on breadboards, where the rig drives both nets (no Y1, no C9 there). On the PCB, the old rig connector is gone (Nano driver, below).
- **Layout:** Y1 close to U2 and U4, with its own 100 nF at pin 8; C9 and R4 near U1.1; SW3 at the board edge. A front-panel button can go on a 2-pin header in parallel with SW3.
- **Datasheet:** `logisim/circuits/chips/ecs_2100x.pdf` (ECS-2100X series, NRND). It doesn't list the 2200 series; the ECS-2200BX is a 5 V part (confirmed by Gianni, 2026-11-14). Supply range 4.75–5.25 V, symmetry 45–55 % (Q9), rise / fall 5 ns, HCMOS load 50 pF, start-up 10 ms, 0–70 °C.
- **Supply:** the oscillator needs 4.75 V minimum. With the series Schottky gone (S1), the card gets the motherboard's 5 V less the plane drop.

## clkgen: Arduino Nano clock and reset driver (2026-11-14)

Goal: slow clocks and step by step from a button, with the boot (65,548 steps) still at full speed. The Nano sits in a socket on the clkgen board. Logisim: no change (`clkgen_osc` and `clkgen_rc` stay the module's inputs, like Y1 and the RC).

**Why it works:** every clkgen output follows `osc` edges in a fixed order, not gate delays, and every chip is static, so `osc` can run at any rate down to 0 Hz. The only electrical rule is clean edges (HCT clock inputs want under about 500 ns): a timer output is fine, a bare button or an RC isn't.

**Parts (on the schematic 2026-11-14, netlist and ERC checked):**

| Ref | Part | Connection |
|---|---|---|
| A1 | Arduino Nano v3 (`MCU_Module:Arduino_Nano_v3.x`, footprint `Module:Arduino_Nano`, in 2 × 1×15 female headers) | 5V pin fed from +5V through D2 (not VIN), both GND |
| D2 | 1N5819 | anode +5V, cathode the Nano's 5V pin: the card feeds the Nano, USB can never feed the card (S1) |
| R6 | 10 kΩ | +5V → A1: power sense. The sketch drives nothing while A1 reads low |
| JP1 | 3-pin jumper (`Jumper:Jumper_3_Open`, 1×03 header) | 1 = Y1.5, 2 = `clkgen_osc`, 3 = the Nano clock. Selects Y1 (standalone, no Nano) or the Nano |
| R3 | 33 Ω | D9 (OC1A) → JP1.3: damps the 4 MHz edges |
| R1 | 10 kΩ | `clkgen_osc` → GND: holds `osc` low while the Nano boots (its pins are inputs) |
| R2 | 220 Ω | D4 → `clkgen_rc`: the Nano holds the reset (OUTPUT LOW) or lets go (INPUT); limits the current into C9 |
| SW1 | tactile push button | D5 → GND, STEP (internal pull-up, software debounce) |
| SW2 | tactile push button | D6 → GND, RUN / STOP (internal pull-up, software debounce) |
| RV1 | 10 kΩ potentiometer | the Nano's 5V pin / GND, wiper on A0: speed in RUN (top of the range = 4 MHz) |
| | `clk` feedback | `clk_5` → D2 (INT0): the sketch knows the step phase |
| | `rst_n` feedback | `rst_n` → D3 (INT1): the sketch sees the reset release |

The Nano's D13 LED shows run / stop. The old rig connector J1 is removed (see "J1" below). Since 2026-11-15, J1 is the card edge (PCIe x1) and carries every clock, reset and power line; the old J1/J2/J3 sockets are gone.

**Sketch (to write):**
- Timer1 in CTC, toggle OC1A on compare: OCR1A = 1 with no prescaler gives exactly 4.000 MHz at 50 % (the 16 MHz crystal); the prescaler and OCR1A give slow rates down to well under 1 Hz. Timer0 (millis) is left alone.
- Power sense first: while A1 reads low, D9 and D4 stay inputs (Timer1 off). Start: D4 low (reset held), clock started, D4 released, wait for `rst_n` high, run until stopped (the boot copy takes 33 ms at 4 MHz).
- Stop: Timer1's clock off (TCCR1B = 0). The pin only changes on a compare match, so it keeps its level: no runt pulse.
- Step: toggle `osc` (force compare FOC1A, or D9 as a GPIO) until `clk_5` rises: one press = one CPU step (2 `osc` periods).
- **Auto-reset:** opening the serial port resets the Nano (DTR). Its pins become inputs: `osc` stops low (R1) and the CPU freezes, then the sketch starts again and reboots the CPU. Accepted; the sketch could skip the reset if `rst_n` is already high.

**J1 (old):** it only existed so the breadboard rig could drive `clkgen_osc` and `clkgen_rc`. The rig tests happen on breadboards, before the PCB, so the PCB doesn't need it. `kicadcmp.py` will then report the two nets under "Logisim top-level pins whose KiCad net has no connector": expected (they are driven on the board). Optional: test points on `clkgen_osc`, `clk_5` and `rst_n` for the scope.

**Loads:** the Nano's D2 and D3 pins add one input each (about 10 pF) on `clk_5` and `rst_n`: negligible next to the branch loads.

**Power (S1, settled 2026-11-14):** on a Nano v3, USB feeds the 5V pin through its own Schottky. D2 (1N5819) feeds that pin from the card, so the Nano runs from either side at about 4.7 V and can never power the CPU from USB. If USB is plugged in while the CPU is off, R6 lets the sketch see it and keep D9 and D4 as inputs, so nothing pushes current into the unpowered chips' input clamps.

**Firmware rule (ESP32):** the CPU can be stopped in the middle of a mailbox transfer, so the ESP32 never times out on the mailbox (iod-decisions.md firmware rules).

## Power (S1, decided 2026-11-14)

Budget (estimates): about 0.6 A typical, 1.6 A worst case for the whole CPU. The largest are cu (7 EEPROMs always selected, 60 pull-ups: up to about 450 mA) and iod (TTGO VGA32: up to about 450 mA in Wi-Fi bursts). The Mean Well GST25E05 (5 V 4 A) covers it.

**Cards (done on all 9, 2026-11-14):** no series diode and no TVS. The power connector goes straight to +5V / GND; keep the 47 µF and one 100 nF per chip. Pinout GND, +5V, +5V, GND (clkgen J3), so a card turned 180° still gets the right polarity.

**Motherboard input (drawn 2026-11-15 on `mb/hbc2_mb.kicad_sch`, bottom right):** J1 → SW1 → F1, then D1 and C1 across the rail, PWR_FLAGs on +5V and GND, ERC 0 / 0. Parts and footprints are in [parts.md](parts.md), "Motherboard". barrel jack → switch → fuse T 3.15 A → +5V plane. Across the rail: a unidirectional TVS (P6KE6V8A, 5.8 V stand-off) and 1000 µF 10 V. Reversed polarity drives the TVS forward and blows the fuse or trips the adapter's protection; in normal use only the fuse (about 0.05 V) is in series.
- GND pour on one layer, +5V as a wide pour or a ≥ 3 mm trace (1 mm × 30 cm drops about 0.3 V at 2 A). cu and iod closest to the input.
- Measure the rail at the farthest card under load. Below about 4.9 V (the oscillator needs 4.75 V): adjustable supply (e.g. Mean Well RS-25-5) set to 5.1 V.

**Things that can feed the CPU from outside:** the Nano (D2 + R6 sense, clkgen) and the TTGO (iod D1). Both supplies are diode-isolated. The TTGO's signal pins can still feed an unpowered CPU (see "IOD: the TTGO VGA32").

## IOD notes

- `iod_addr_in` and `iod_addr_out` are the same physical lines. One KiCad net per bit is correct.
- Do not put no-connect flags on those address lines. They are already on the unused chip pins: the spare 74138 outputs, the 7432's spare gate, and the spare 7474 outputs.
- `iod_vec` (U5 Q2) goes nowhere on purpose. Only `iod_vec_n` is used: it drives the EVT 595s' /OE, the evt_s /CLR and the evt_clr D input.
- The EVT 595s drive the address lines and the data bus in the vector step (/OE = `iod_vec_n`). Every other driver on those buses must be off then. Check this against the cu microcode once the cu schematic exists (S2).

## IOD: the TTGO VGA32 (2026-11-14)

The ESP32 board is the LilyGO TTGO VGA32 v1.4. It sits next to the I/O card, in a 3D-printed box, and is wired to it by three cables: Dupont female-female wires (short run) on plain vertical 2.54 mm pin headers on the card (Gianni, 2026-11-15). Nothing is keyed: the silkscreen names each pin.

| Card | Cable | TTGO end |
|---|---|---|
| J2 (2 × 4, numbered like the TTGO's header) | 8 Dupont wires | header J2: MOSI, SCK, (SD CS, not connected on the card), MISO, tx_full, rx_full, GND, +3V3 |
| J4 (1 × 5, numbered like the mini-DIN; pin 2 not connected) | cut PS/2 cable, Dupont crimps on pins 1, 3, 4, 5 | mouse port: RCLK_EVT (pin 1), GND (3), +5V_ESP (4), MBX_SEL (5) |
| J1 (1 × 4) | 3 wires + GND, Dupont ends at the card | soldered at IO25 (RCLK_RX), IO35 (evt_pend) and IO36 (rst_n) |

- **U12** (SN74LVC244AN) is powered from the TTGO's 3.3 V. There is no regulator on the card.
- **Series resistors:** 33 Ω on U12's outputs (R1, R3–R5), and 220 Ω on `rst_n` (R2), because the TTGO's button K1 sits on IO36.
- **D1** (1N5819) feeds the TTGO from +5V. With the TTGO's own diode on USB, the TTGO can be flashed while the HBC-2 runs.
- **Back-powering:** D1 only blocks the supply pin. With the HBC-2 off and the TTGO on USB, the TTGO's pull-ups on MBX_SEL and RCLK_EVT (and SCK / RCLK_RX when driven high) feed +5V through the 74HCT input diodes of U7, U8 and U10. Rule: unplug the Dupont wires (J4 at least) before the HBC-2 is switched off while the TTGO stays on USB. Details and the options not taken: iod-decisions.md, "Back-powering".
- **`iod_int`** is on the card edge (J3, pin A30). It goes to the control unit.
- **Full record** (pins, reasons, firmware rules): `logisim/notes/records/iod-decisions.md`, "The TTGO VGA32 wired to the card".
- **`kicadcmp.py`** lists the TTGO-side nets and D1/R1–R5 as board extras or connector nets with no Logisim pin. That is expected.
