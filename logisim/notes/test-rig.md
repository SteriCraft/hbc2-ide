# Card test rig

Each card (clkgen, ir, alu, flags, regs, addr, mem, cu) is tested alone on breadboards, by replaying on the real chips the same vectors that test its circuit in Logisim (`hbc2 vectors <card>`). The tested chips then move onto the card's DIP PCB (in sockets). What to buy, in which order: [breadboard-order.md](breadboard-order.md).

The rig is generic: a chain of drive bits, a chain of sense bits and a few clock pins. Which rig bit is wired to which card signal is written per card in a **map file**, `logisim/rig/<card>.map`, read by the replay tool. The card connector's pinout doesn't change the rig: the map's optional third column records it once it is decided.

## Hardware

An **Arduino Mega 2560** (5 V I/O, USB to the PC; the same board as the EEPROM programmer, on other pins) and four BB830 boards:

| Rig bits | Parts | What they do |
|---|---|---|
| **D0–D63** (drive) | 8 × **SN74AHCT595N**, each output through a **470 Ω** isolated network (Bourns 4116R-1-471LF, DIP-16) | Drive the card's inputs. Chip *k* (QA–QH) = D8k–D8k+7; MOSI feeds chip 7 and chip 0 is the far end, so chip 0's byte is sent first. Each chip's **/OE** has its own Mega pin, so a chip carrying a bus can let go of it. |
| **S0–S63** (sense) | 8 × **CD74HCT165E**, each input with a **100 kΩ** to a common line (Bourns 4609X-101-104LF, bussed SIP-9) | Read the card's outputs. Chip *k* (A–H) = S8k–S8k+7; chip 0's QH goes to MISO. All network commons go to one Mega pin, **PULL**. |
| **C0–C7** (clocks) | Mega pins 2–9, each through **100 Ω** | The card's clock inputs (`clk`, `clk_wr_n`, `clk_mem_wr_n`, `clkgen_osc`), set after the D bits of the same row. |

Why these parts:
- **TI makes no 74HCT595 in DIP** (SN74HCT595: TSSOP only). The SN74AHCT595N is pin-compatible, has TTL-level inputs and is active. It is a rig part, not a CPU part, so the 74HCT rule of chips.md doesn't apply. Its fast edges are slowed by the 470 Ω.
- **470 Ω in series** with every drive output: a wiring mistake, or a card driving a bus the rig also drives, costs at most about 10 mA per pin instead of a short.
- **100 kΩ pulls switched by PULL** to tell a driven output from a floating one (Z, the `Z` of the vector files): read once with PULL high, once with PULL low. A driven bit reads the same both times; a floating bit follows PULL. 100 kΩ is weak enough that the cu's own 10 kΩ pull resistors (C24) win: a pulled control line reads as its pulled level, as the cu vectors expect.

Mega pins (the programmer uses A0–A14 for the address, 34–41 for the data, 42–44 for /CE, /OE, /WE, so both can stay wired):

| Pin | Signal |
|---|---|
| 51 (MOSI) | SER of 595 chip 7 (the chain runs 7 → 0) |
| 52 (SCK) | SRCLK of every 595, CLK of every 165 |
| 50 (MISO) | QH of 165 chip 0 (the chain runs 7 → 0, SER of chip 7 to GND) |
| 53 (SS) | output, unused (SPI master) |
| 30 | RCLK of every 595 (latch) |
| 31 | SH/LD of every 165 |
| 32 | CLK INH of every 165 (high while the 595s are shifted) |
| 33 | PULL |
| 22–29 | /OE of 595 chips 0–7 |
| 2–9 | C0–C7 |

The 595s' /SRCLR go to VCC. Power: the rig and the card from the 5 V supply, the Mega from USB, **GNDs joined, the Mega's 5 V pin not connected** to the supply. One 100 nF per rig chip.

Chain sizes: the eight cards need at most 59 D bits (addr), 63 S bits (cu) and 2 C bits (`python tools/rigmap.py all`). If a card ever needs more, add a chip to a chain and raise `RIG` in rigmap.py.

## Map files

One line per wire, `#` starts a comment:

```
# rig         pins                                [connector]
D3            alu_op[3]
S0            alu_n
C0            clk
VCC           some_input                          (tied on the bench)
D16+S8        flags_bus_in[0]+flags_bus_out[0]    J1.17
```

- `name[k]` is bit *k* of a multi-bit pin of the card circuit, a bare name a 1-bit pin. The names are the circuit's pins, which are also the vector files' columns.
- `D` and `C` bits only drive inputs, `S` bits only read outputs. `VCC` / `GND` tie an input on the bench.
- **`+` joins the two sides of one wire**: a bus the card both reads and drives is one input pin and one output pin in Logisim (e.g. `flags_bus_in` / `flags_bus_out`, both `bus_data`). On the bench it is one wire, driven by a D bit and read by an S bit.
- A 595 carrying a joined wire carries only that bus (its /OE goes off while the card drives the bus).
- The third column is the card's connector pin, free text, optional, unique per card. The rig ignores it; it is there for when the pinout is decided.

`tools/rigmap.py`:
- `gen <card>` writes a default map from the circuit's pins: clocks on C bits, the plain inputs packed from D0, the outputs from S0, each bus on its own 595 above them. It refuses to overwrite an existing map (`--force`): edit the map by hand once wiring starts.
- `check <card> [--vectors file]` checks it: every input bit mapped exactly once, no rig bit used twice, directions right, buses alone on their 595s, the connector pins unique, and with `--vectors` the vector header against the card's pins. Unmapped outputs are warnings (not checked by the replay).
- `all` checks every map and prints the rig usage.

The maps in `logisim/rig/` are the generated defaults (2026-11-10). Wire the bench from them.

## Replay

To write: Mega firmware and a PC tool (`tools/replay.py`, not done yet). For each row of the vector file:

1. **Drive.** Shift the D values (inputs; `x` = keep the last value) into the 595s, then latch (RCLK). For a 595 carrying a bus: /OE on only if the row expects the card's side of that bus to be `Z` (the card isn't driving it) and its input value isn't `x`; otherwise /OE off. A row where the card drives the bus and the bus's input column holds a different value can't be applied on one wire: the tool reports it.
2. **Clocks.** Set the C pins that change, after the latch. The vectors never change data and a clock in the same row (data on the `clk` = 0 row, the edge on the next), so this order matches Logisim.
3. **Settle**, then **sense** with PULL high, then PULL low (each read after 20 µs for the 100 kΩ to charge the line), and compare: `0` / `1` = driven to that level both times, `Z` = followed PULL both times, `x` = not checked. A mismatch prints the set, the row, the pin bit and the two reads.

Sets: each set starts from Logisim's power-up. The rig doesn't power-cycle the card; the vectors write `x` for every output that isn't known since power-up (`Out::U`), so the sets can run one after another. Speed: about 50 µs of rig time per row plus the USB transfer, a few thousand rows per second: the cu's 4.2 million rows take 20–30 minutes.

Per card:
- **clkgen**: its sequences are in `designs/clkgen.py` (Python), not `hbc2 vectors`: they need an export to a vector file. The oscillator and the RC are left off for the test (the rig drives `clkgen_osc` and `clkgen_rc`), then fitted to check the free-running clock and the reset on the scope.
- **mem**: the two ROMs must hold the pattern of `vectors/mem.rs` `rom_pattern` (the at28c256 model's default image), written with the programmer; that image needs an export too.
- **cu**: the seven EEPROMs hold the microcode images of `circuits/cu/microcode` (`hbc2 ucode`).

What the rig can't test: speed. It steps the clock slowly, so paths across cards, bus loading and clock skew are left for the PCBs or the optional partial integration (breadboard-order.md, order 8; breadboard-checks.md B1–B3).
