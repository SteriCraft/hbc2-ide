# Control unit: decisions

**Phase 1 (paper design, 2026-11-05): accepted.** Gianni's answers (all 10 recommendations accepted, plus the 74HCT10 replacement and the INT requirement) are in section 12. **Phase 2 (microcode) is in section 13. Phase 3 (circuits `cu` and `clk`) is in section 14**: there the 74HCT10 came back (section 12's replacement was reverted by Gianni).

Source of truth: `hbc2-core/src/cpu.rs`, `isa.rs` (`DECODE`, `INSTRUCTION_TABLE`). Requirements: control-requirements.md C1–C23, pc-sp-requirements.md P1–P4, registers-requirements.md, iod-requirements.md I1–I8. Interfaces: registers-, flags-, addr-, ir-mem-decisions.md, alu-report.md. Times are in ns from the rising `clk` edge that starts the step (500 ns), **max 25 °C (pass) / max 85 °C (information)**, bus loading included.

Planned modules (phase 3): `cu` (circuits/cu/cu.circ: microcode, sequencer, pipeline register, conditions, pending port latch) and `clk` (circuits/cu/clk.circ: clock and reset generator, section 7).

## 1. Control signal inventory

Every control input of every section, and what this design does with it. "ROM" = a microcode bit as is; "derived" = made from other microcode bits by gates placed **before** the pipeline register (section 2).

| Signal | W | Section | Active | Here |
|---|---|---|---|---|
| `alu_op` | 4 | alu | code | ROM (already encoded) |
| `flags_op` | 4 | flags | code | ROM (already encoded) |
| `flags_bit_val` | 1 | flags | high | ROM |
| `flags_oe_n` | 1 | flags | low | derived: `dbus` = 3 |
| `regs_we_n` | 1 | regs | low | ROM |
| `regs_a_sel` | 2 | regs | code | ROM (on the register read path, the tightest: no gate) |
| `regs_cu_idx` | 2 | regs | code | ROM |
| `regs_cu_bank_n` | 2 | regs | low pair | ROM (11 = port A off, 00 forbidden) |
| `regs_b_sel` | 2 | regs | code | ROM (read path) |
| `regs_a_lat_oe_n` | 1 | regs | low | derived: low ⇔ `regs_cu_bank_n` = 11 |
| `regs_b_lat_oe_n` | 1 | regs | low | derived: low ⇔ `regs_b_sel` = 11 and the constant off |
| `regs_const_oe_n` | 1 | regs | low | ROM |
| `regs_const_0` | 1 | regs | high | derived: NOT `alu_op` bit 3 |
| `regs_a_bus_oe_n` | 1 | regs | low | derived: `dbus` = 1 |
| `regs_res_oe_n` | 1 | regs | low | derived: `dbus` = 2 |
| `addr_base_sel`, `addr_base_oe_n` | 2 + 2 | addr | code / low | ROM |
| `addr_off_sel`, `addr_off_oe_n` | 1 + 2 | addr | code / low | ROM |
| `addr_inv`, `addr_cin` | 1 + 1 | addr | high | ROM |
| `addr_lat_ld_n`, `addr_pc_ld_n`, `addr_sp_ld_n` | 3 × 1 | addr | low | ROM (every combination is used: latch + PC, latch + SP, PC + SP at reset) |
| `addr_lat_oe_n` | 2 | addr | low | derived: `abus` |
| `addr_pc_oe_n`, `addr_sp_oe_n` | 2 + 2 | addr | low | derived: `dbus` = 4, 5, 6, 7 |
| `ir_b0_ld_n`, `ir_b1_ld_n`, `ir_v1_ld_n`, `ir_v2_ld_n` | 4 × 1 | ir | low | ROM (b0 and b1 also steer the fetch bypass, section 4.1) |
| `mem_re`, `mem_oe_n`, `mem_we_n`, `mem_wbuf_oe_n`, `mem_rom_oe_n` | 5 × 1 | mem | high / low | ROM (C19's four cases) |
| IE | 1 | IOD connector | high | derived: `abus` = 01 |
| RW | 1 | IOD connector | 1 = read | derived: 0 ⇔ `dbus` = 1 (OUT is the only I/O step where the CPU drives `bus_data`; RW matters only with IE) |
| INR | 1 | IOD connector | high | ROM |
| `cu_swi_ld_n` | 1 | cu | low | ROM: pending port latch load (C13), sets P |
| `cu_swi_oe_n` | 1 | cu | low | derived: `abus` = 11; pending port latch onto `bus_addr` low, clears P |
| `cu_next` | 4 | cu | — | ROM: next step (section 5) |
| `cu_force` | 1 | cu | high | ROM: forced microcode row (section 4.7) |
| `cu_cond_sel` | 2 | cu | code | ROM: condition (section 3) |

Totals: 55 section inputs + 9 control unit bits = 64. **Microcode: 54 bits** after the encodings below, so **7 × AT28C256** (56 bits, 2 spare). Pipeline register: 60 bits in 8 × 74HCT574 (4 spare) + the 4-bit step register (74HCT175).

## 2. Encodings

All decoders sit **between the EEPROMs and the pipeline register**, so every control output comes straight from a 74HCT574: valid 33 / 41 ns after the edge and glitch-free, as C5 and every section's timing assume. Section 6 shows the decode still fits in the step.

| Field | ROM bits | Gives | Logic | Saves | Why it can't lose a case |
|---|---|---|---|---|---|
| `dbus` | 3 | `bus_data` driver: 0 none, 1 regs bus A, 2 result latch, 3 flags, 4 PC hi, 5 PC lo, 6 SP hi, 7 SP lo (+ RW) | 74HCT138 | 4 | One-hot: at most one CPU driver on `bus_data` by construction (C5, C11). No step needs two. |
| `abus` | 2 | 00 latch drives `bus_addr`; 01 the same + IE (IN, OUT); 10 hardware vector step (latch off, the IOD drives 00:p); 11 software vector step (latch high byte 00, pending latch low byte) | 2 AND, 2 inverters, 1 NAND | 2 | These four are the only `bus_addr` cases (C12, C22, I6). At most one low-byte driver by construction. |
| A latch | 0 | `regs_a_lat_oe_n` = NAND(`regs_cu_bank_n`) | 1 NAND | 1 | Port A off ⇔ A latch on: bus A always has exactly one source (C5). |
| B latch | 0 | `regs_b_lat_oe_n` = NAND3(`regs_b_sel` = 11, `regs_const_oe_n`) | 1 NAND3 | 1 | The same for bus B. |
| constant | 0 | `regs_const_0` = NOT `alu_op3` | 1 inverter | 1 | The constant is only used by INC / DEC (ADD / SUB, op3 = 0 → 01) and the reset sweep (AND, op3 = 1 → 00). |

Kept as ROM bits: the register indexes (read path), the address section's selects and loads, the IR loads, the memory strobes, INR. Encoding them would save 1 bit each for 2–3 gates; 7 EEPROMs already fit.

The busiest steps still take one step each: "read → IR; latch, PC := PC + 1" (every fetch byte), "PC hi → RAM; latch := 0000; INR" (IE4), "IOD → I; latch, PC := FE00 + 2p" (vector), "result → RAM; latch := PC" (INC on memory), the reset word (PC and SP loads, ALU AND 00).

## 3. Microcode address

| A14–A9 | A8–A5 | A4–A1 | A0 |
|---|---|---|---|
| opcode o5..o0 | mode m3..m0 | step s3..s0 | cond |

- **Opcode and mode** come through **3 × 74HCT157**: from the IR, or from `bus_data` while that byte is being fetched (the bypass, section 4.1). `cu_force` = 1 drives their /G, so the outputs are 0: row (00, 0) (section 4.7).
- **Step**: the step register, which holds the previous word's `cu_next` (section 5).
- **Cond**: one of four inputs (half of a 74HCT153, `cu_cond_sel`). The microcode gives a word for each value; a word that doesn't branch fills both.

| `cu_cond_sel` | Input | Used by |
|---|---|---|
| 00 | `jc`: the jump condition. A 74HCT151 selected by `ir_op` bits 2..0 gives S Z F 0 C E N 1 (JMS JMZ JMF – JMC JME JMN JMP), so no microcode bits pick the flag. | JMx, the step that decides |
| 01 | `bnd_n` = NOR3(HW, SW, H), with HW = I · INT, SW = I · P | the last step of every instruction |
| 10 | `hw_q`: HW sampled at the end of the step before | W1 (section 4.3) |
| 11 | `x_n` = NOR3(SW, `cout_q`, `rst_early`) | the reset word, S1, the boot copy loop, W2 |

`cout_q` (`addr_cout` sampled), `hw_q` and P (pending software interrupt) are three flip-flops of a 74HCT175 clocked by `clk`, cleared by `rst_n`. P is set when the pending port latch loads and cleared when it drives the bus: P_next = NAND(NAND(P, `cu_swi_oe_n`), `cu_swi_ld_n`). `x_n` is only read where its other inputs are known to be 0: after reset P = 0; W1 and the copy loop keep the adder in a known state.

15 bits = 64 opcodes × 16 modes × 16 steps × 2 = 32 K words, exactly one AT28C256. No state bit is needed (section 4.7).

### Address map

| Rows (opcode, mode) | Steps | Holds |
|---|---|---|
| every row | 0 | the reset word (cond 0) and S1 (cond 1). The step register is cleared by `rst_n`, and the IR is random at power-up. |
| (00, 0): the forced row | 1 | the boundary pair: F0 / W1 (this is also NOP's step 1) |
| (00, 0) | 2–14 | W1 and W2 successors, IE1–IE8 (hardware and software); step 15 spare |
| NOP (00, 0–3), invalid opcodes 36–3F | 1 | the boundary pair (1-byte NOP) |
| (3F, B) | 2–11 | the boot (S2 onwards). 3F is an invalid opcode: only its step 1 is used otherwise. |
| valid opcode, listed mode | 1–7 | the instruction's steps after F0 |
| invalid mode | 1–4 | the stop sequence (section 4.5) |

About 90 % of the ROM stays unused: room for later instructions.

## 4. Sequences

### 4.1 Fetch: the bypass

With a pipeline register, the word for step k + 1 is read from the EEPROMs during step k. A byte loaded into the IR at the end of step k is only seen by the word for step k + 2, one step too late. So while a byte is fetched, the 157s give the EEPROMs **the byte on `bus_data`** instead of the IR. Their selects are `ir_b0_ld_n` (opcode, m3 m2) and `ir_b1_ld_n` (m1 m0), so they cost no microcode bit.

| Step | Memory (at the latch) | IR | EEPROM address in that step | Next word knows |
|---|---|---|---|---|
| F0 | read byte 0 | b0 | opcode, m3 m2 from `bus_data`; m1 m0 from the IR (stale, never decisive: C18); step 1 | the opcode: 1-byte instructions run (or NOP ends) from the next step |
| F1 | read byte 1 | b1 | opcode, m3 m2 from the IR; m1 m0 from `bus_data` | the full mode: length, invalid mode |
| F2 | read V1 (and the operand latches) | v1 | IR | — |
| F3 | read V2 | v2 | IR | — |

Each fetch step also does latch, PC := PC + 1. **An n-byte instruction takes n fetch steps**, and the word after the last one can act on any field. F0 is the same word for every instruction. Bytes past the length are not loaded (C18). The bypass costs 3 chips and saves 1–2 steps per instruction (no blind step).

### 4.2 Instruction boundary

| Last step of | Next word | Why |
|---|---|---|
| every instruction (default) | forced row, step 1; cond `bnd_n`: F0, or W1 if HW, SW or H | interrupts between instructions only; INT is sampled in this step |
| NOP, invalid opcode | F0 is the last step: its own step 1 holds the same pair | F0's `cu_cond_sel` is always `bnd_n` |
| STI | F0 (copied in STI's row), no check | C3: interrupts wait until the next instruction has run (`interrupt_delay`) |
| CLI | F0, no check | The check would still see I = 1 and could enter a handler whose IRT sets I again: the emulator never does that. |
| HLT, invalid mode | W1 (a copy in the instruction's row) | H is being set in that step; W1 checks for interrupts (I set) or stays halted |
| INT | a boundary step after P is set | the check must see P = 1 (the emulator takes it at once if I is set) |
| POF, IRT | the check comes after the step that writes I and H | so it sees their new values |

### 4.3 Interrupt entry (C2, C12, C13, C21)

| Step | Address section | Memory, buses | Flags, registers | Next |
|---|---|---|---|---|
| W1 | idle (sum 0) | — | — | `hw_q`: 1 → IE1 hw, 0 → W2 |
| W2 | idle | — | — | SW (`x_n`): 1 → IE1 sw, 0 → W1 (halted) |
| IE1 | latch, SP := SP − 1 | — | I := 0 (1010, 0) | |
| IE2 | latch, SP := SP − 1 | I → RAM (bus A, CU index 4) | H := 0 (1111, 0) | |
| IE3 | latch, SP := SP − 1 | PC lo → RAM | | |
| IE4 | latch := 0000 | PC hi → RAM | | INR = 1 (hardware only) |
| IE5 hw | latch, PC := FE00 + 2p | `abus` 10: the IOD drives 00:p and the data | I := `bus_data` | |
| IE5 sw | latch, PC := FE00 + 2p | `abus` 11: latch high 00, pending latch p | — | P := 0 |
| IE6 | latch := PC + 1 | read → V1 | | |
| IE7 | — | read → V2 | | |
| IE8 | latch, PC := Vx | | | F0 (I = 0: nothing to check) |

- **One INT sample per boundary.** The EEPROMs branch two ways per step, but the boundary has four outcomes: fetch, hardware, software, halt. So W1 spends one step, using `hw_q` = HW of the boundary step. "Take it?" and "hardware first?" therefore come from the same INT sample, as in cpu.rs.
- **The microcode remembers which interrupt it is.** IE1–IE5 exist twice in the microcode, so no extra flip-flop is needed.
- **INR comes in the last push step.** The IOD only reacts in the next step, so the I6 contract is unchanged and a step is saved (Q5).
- **Length:** a hardware entry takes 9 steps after the boundary (W1 + 8), a software one 10.

### 4.4 Halt

W1 ↔ W2 is the halt loop. With I = 1, an interrupt leaves it (`hw_q` or SW); with I = 0, only a reset does. It is entered from HLT, from an invalid mode, or from the boundary when POF set H.

### 4.5 Invalid opcode, invalid mode (C15, C18)

| Case (isa.rs `DECODE`) | Steps |
|---|---|
| opcode ≥ 36 | F0 only: a 1-byte NOP |
| opcode without operands, m3 m2 ≠ 00 | F0; S1: I := 0, latch, PC := PC − 1; S2: H := 1 → W1 |
| opcode with operands, mode not listed | F0, F1; S1: I := 0, PC − 1; S2: H := 1, PC − 1 → W1 |

**I is cleared first, then H** (C2 lists H first): W1's `hw_q` is sampled in S2, where I already reads 0, so a stopped CPU can't take an interrupt (Q7).

### 4.6 Reset and boot (C7, C16, C17, C20)

| Interval | `rst_early_n` | `rst_n` | Step register | Word | Address → next word |
|---|---|---|---|---|---|
| reset | 0 | 0 | 0 (cleared) | reset word | step 0, `x_n` = 0 → reset word |
| E(r−1) to E(r) | 1 | 0 | 0 | reset word | step 0, `x_n` = 1 → S1 |
| E(r) to E(r+1) | 1 | 1 | 0 (`rst_n` was low at E(r)) | S1, the first word with `rst_n` high (C17) | `bus_data` = FE through the bypass, step 0 → S1 |
| from E(r+1) | 1 | 1 | 2 | S1 again (harmless) | row (3F, B), step 2 → S2 |

| Word | Does |
|---|---|
| reset | C16's reset word (PC := 0000, SP := FE00 on every edge) + ALU AND with the constant 00 (C7 step 0: result latch := 00) |
| S1 (2 steps) | SP high (FE) → `bus_data` → IR b0 and b1 (row 3F, B); ALU AND 00 |
| S2–S9 | result latch (00) → registers 0–7 (C7 steps 1–8); S9 also latch := PC |
| C | latch, PC := PC + 1 with the boot copy strobes (C19); `x_n` = NOT `cout_q` |
| fix-up | latch, PC := PC − 1 |
| F0 | the first fetch at 0000, with no interrupt check before it |

**The loop leaves one step late.**
- `addr_cout` comes about 258 ns into the step, and the EEPROMs need 150 ns more than the step has left, so the loop tests `cout_q`.
- The step after FFFF copies 0000 a second time (the same byte) and leaves PC = 0001; the fix-up puts PC back to 0000.
- This adds 2 steps to C20's sequence (Q8).
- **Reset to first fetch:** 2 + 8 + 65,537 + 1 = **65,548 steps, 32.8 ms**.

### 4.7 Where the sequences live

| Option | Address bits | Result |
|---|---|---|
| A state bit in the address | step field 4 → 3 bits | Doesn't fit. In the sequence state the IR is arbitrary, so each sequence word must be copied into all 1,024 rows; that leaves 3 step bits + cond = 16 words for about 30. |
| Reserved opcodes 36–3F (54–63: 36 is free too) loaded into the IR | 0 | Needs a known byte on `bus_data`. Boot: yes (SP high = FE → opcode 3F). Interrupt entry: no known byte. |
| **Forced row**: `cu_force` disables the opcode/mode 157s → row (00, 0) | 0 (1 ROM bit) | Fits: 14 of the row's 16 steps used. Row (00, 0) is NOP's, which only uses step 1, and that step holds the pair the forced row needs anyway. |

**Chosen:**
- **Interrupt entry and the halt loop:** the forced row.
- **The boot:** reserved opcode 3F.
- **The reset:** step 0 of every row.

## 5. Sequencer

| Option | ROM bits | Chips | Branches |
|---|---|---|---|
| 74HCT161 with a "last step" bit (clear to 0) | 1 | 1 | Only "next" or "restart": no way to reach the forced row, the boot row or W1 |
| 74HCT161 with load (target from the ROM) | 5–6 (load, hold, 4 D) | 1 | Any target |
| **Next-step field `cu_next` in a 74HCT175** (chosen) | 4 | 1 | Any target, every step; conditions through the cond address bit |

- **No "last step" bit.** An instruction's last word has `cu_next` = 1 with `cu_force` = 1, so the next word is the boundary pair.
- **Reset:** the 175's /MR = `rst_n` holds step 0 (asynchronous, from power-up).
- **Leaving reset:** `rst_early_n` reaches the address through `x_n`, so the step in which `rst_n` is first high already runs S1, not the reset word (C17, section 4.6).

## 6. Pipeline register and timing

```
        E(k)                                              E(k+1)
          | step k: word W(k) in the 574s / 175           |
574 out   |XX|== W(k), from 33 / 41 ======================|XX
IR loaded at E(k) |XX|== from 38 / 48 ====================|
bus_data (fetch)  |XXXXXXXXXXX|== byte, from 187 / 224 ===|
EEPROM A  |XXXXXXXXXXXXXXXXXX|== stable by 214 / 258 =====|
EEPROM D  |XXXXXXXXXXXXXXXXXXXXXXXXXXX|== W(k+1) 364 / 408|
574 D     |XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX|== 400 / 453 ==| setup 12 / 15
```

The EEPROMs (AT28C256-15: tACC 150 ns, /CE and /OE tied low, /WE high) read the next word during the current step. The worst case is the bypass, a byte being fetched:

| Address input | Path | Word at the 574 (25 / 85 °C) | Margin |
|---|---|---|---|
| step register | 175 CP→Q (33 / 41) → address load (2 / 3) → tACC 150 → 138 decode (36 / 45) → 574 setup (12 / 15) | 233 / 254 | 267 / 246 |
| IR fields, right after a load | 377 CP→Q (38 / 48) → 157 I→Y (25 / 31) + load → 150 → 138 → setup | 263 / 292 | 237 / 208 |
| **bypass** (byte on `bus_data`) | RAM → `bus_data` 184 / 220 (timing.py) + 157 inputs on the bus (3 / 4) → 157 I→Y + load → 150 → 138 → setup | **412 / 468** | **88 / 32** |
| `cu_force` | 574 (33 / 41) → 157 /G→Y (29 / 36 est.) + load → 150 → 138 → setup | 262 / 290 | 238 / 210 |
| jump condition | IR op (38 / 48) → 151 S→Y (46 / 58 est.) → 153 I→Y (34 / 43) + load → 150 → 138 → setup | 318 / 362 | 182 / 138 |
| boundary, INT valid by 150 (Q6) | INT → 08 (24 / 30) → 27 (23 / 29) → 153 + load → 150 → 138 → setup | 431 / 465 | 69 / 35 |
| CU flip-flops (`hw_q`, `x_n` inputs), `rst_early` | 175 or 74 → 27 → 153 + load → 150 → 138 → setup | 290 / 326 | 210 / 174 |
| `cu_cond_sel` | 574 → 153 S→Y (34 / 43) + load → 150 → 138 → setup | 267 / 297 | 233 / 203 |
| `addr_cout` → `cout_q` | adder carry out (258 / 322) → 175 setup (20 / 25) | 278 / 347 | 222 / 153 |
| P next | 574 → 2 NAND (24 + 24 / 30 + 30 est.) → 175 setup | 101 / 126 | — |
| `cu_next` into the step register (no decode) | the same paths without the 138, 175 setup 20 / 25 | at most 384 / 448 (bypass) | 116 / 52 |
| control outputs to the sections | 574 CP→Q, no gate after it | **33 / 41** | as C5 assumes |

- **INT must be valid by 219 / 185 ns** for the boundary decision. iod-requirements.md I5 says 250 (provisional): Q6 proposes 150.
- **Every path passes at 25 °C and at 85 °C.**
- **Estimated delays,** to be replaced by the datasheet figures in phase 3: 74HCT151 (46 / 58), 157 /G (as S→Y), 74HCT00 (as the 08).
- **Hold times:** the AT28C256's output hold after an address change is 0 ns (tOH min). With unspecified minimum clock-to-Q delays, the 574's 5 ns hold can't be shown on paper. Breadboard check, like B1.
- **Power-up:** the 574s (no clear) hold a random word until the first `clk` edge, under 1 µs into the reset. A brief bus contention is possible then. The step register is at 0 from the start.

## 7. Clock and reset generator (module `clk`)

| Signal | From | To | Spec |
|---|---|---|---|
| `osc` | 4.000 MHz HCMOS oscillator module | FF1 CLK, NAND3, inverter → FF2 CLK | C6 |
| `clk` | 74HCT74 #1 FF1 Q (D = /Q1) | every flip-flop's CLK, FF2 D, the IOD | 2 MHz, 50 % whatever `osc`'s duty |
| `clk_n` | FF1 /Q | NAND3 only | |
| `clk_wr_n` | 74HCT10: NAND3(`clk_n`, `osc`, Q2) (kept, section 14; the section 12 replacement was reverted) | regs (670 /WE), the IOD (OUT capture) | low 250–375 (C6, D7) |
| `clk_mem_wr_n` | FF2 /Q (FF2: CLK = NOT `osc`, D = `clk`) | mem (RAM WE#) | low 125–375 (M1) |
| `rst_early_n` | 74HCT74 #2 FF1 Q (D = VCC) | `x_n` (as `rst_early`, FF1's /Q) | C17 |
| `rst_n` | FF2 Q (D = FF1 Q); both /CLR from the RC via 2 × 74HCT14 | flags, addr, cu, the IOD | asynchronous assert, release just after a `clk` edge |
| RC | R, C, push button across C, diode across R | 74HCT14 | power-up and button. C17. The reset only ends once the clock runs: FF1 needs edges. |

Missing so far (all added to this block in phase 3):

1. **Clock fan-out.**
   - **Load:** `clk` drives about 31 clock inputs: 13 × 377, 11 × 574, 4 × 175, the reset 74 and the IOD. That is about 390 pF on one 74HCT74 output, about +70 ns and slow edges.
   - **Fix:** one **74HCT541** as a clock buffer: 6 branches of about 5 inputs for `clk`, 1 for `clk_wr_n`, 1 for `clk_mem_wr_n`.
   - **Why all three go through it:** they are delayed equally, so the write-pulse analyses (D7, M1) stay valid.
   - **Skew:** the skew between outputs of one package isn't specified. Breadboard check, with B1 (Q9).
2. **Oscillator part and duty cycle.**
   - **Part:** a 5 V HCMOS 4.000 MHz module (DIP-8 or DIP-14 can); the reference to choose and check.
   - **Duty:** typically 45–55 %.
   - **What stays the same:** `clk` (÷ 2) and `clk_mem_wr_n` (osc fall to osc fall).
   - **`clk_wr_n` moves:** its rise moves by ±12.5 ns.
   - **Consequences at 25 °C:**
     - write data needed by 322 instead of 335 (C10, iod-requirements I3);
     - the 670 pulse is 36 ns at least (20 needed);
     - the address setup before the RAM's WE# falls drops to 12 ns.
   - **Result:** everything still passes at 25 °C. timing.py gets a duty parameter (Q9).
3. **74HCT10 status.** flags-decisions.md calls CD74HCT10E NRND at TI, but chips.md and C6 use it. Re-check before buying. Replacing it changes the `clk_wr_n` timing.
4. **Unused pins.** 74HCT74 /PRE (all four) and the clock FFs' /CLR are tied high.
5. **Board-level.**
   - A pull-down on INT, so it doesn't float without an IOD.
   - `rst_n` drives about 13 inputs (+20 ns, harmless: nothing samples it near an edge).

## 8. Cycle counts (first estimate)

Steps per instruction, F0 included, latch = PC at the start and end of each. At 2 MHz, one step = 0.5 µs.

| Instruction | None | Reg | RegImm8 | RegRam | mode 4 | StackOffset | Reg16 | Imm16 | Imm8 |
|---|---|---|---|---|---|---|---|---|---|
| ADD ADC SUB SBB AND OR XOR | | 4 | 5 | 8 | 8 | 7 | | | |
| CMP | | 3 | 4 | | 7 | 6 | | | |
| INC DEC | | 4 | | | 8 | | | 8 | |
| NOT | | 4 | | | | | | 8 | |
| SHL ASR SHR | | 4 | | | | | | | |
| MOV | | 4 | 3 | | | | | | |
| LOD, STR | | | | 6 | 6 | 5 | | | |
| PSH / POP | | 4 / 5 | | | | | | | |
| IN, OUT | | 4 | | | | | | | |
| JMP JMC JME JMN JMS JMZ JMF (taken or not) | | | | | | | 3 | 5 | |
| CAL | | | | | | | 5 | 7 | |
| ASP SSP | | | | | | | | 5 | 4 |
| WSP / RSP | | | | | | | 3 / 4 | 5 | |
| INT | | | | | | | | | 4 (6 if Q4 is refused) |
| NOP, invalid opcode | 1 | | | | | | | | |
| CLC … STF (14), HLT | 2 | | | | | | | | |
| PSF / POF / RET / IRT | 3 / 4 / 5 / 6 | | | | | | | | |
| invalid mode (then stopped) | 3 | 4 (any mode not listed) | | | | | | | |
| interrupt entry, after the boundary | hardware 9, software 10 | | | | | | | | |
| reset → first fetch | 65,548 | | | | | | | | |

Where the steps go:
- **ALU result:** an ALU step, then a write step. The write can't overlap the next F0, because both use `bus_data`.
- **Memory operand:** an address step, then a read (with latch := PC).
- **POP / POF:** need a separate latch := PC step (the adder does SP + 1).
- **Conditional jumps:** the step that decides is also the boundary step, so taken or not costs the same.
- **INT:** the pending latch takes V1 from `bus_data` during F2 (Q4), then one boundary step that sees P.

## 9. Chips, timing, EEPROMs

| Qty | Part | Role |
|---|---|---|
| 7 | AT28C256-15PU | microcode (54 bits + 2 spare) |
| 8 | 74HCT574 (model) | pipeline register (60 bits + 4 spare) |
| 2 | 74HCT175 | step register; CU state (`hw_q`, `cout_q`, P, 1 spare) |
| 3 | 74HCT157 | opcode / mode bypass and `cu_force` |
| 1 | 74HCT151 | jump condition by opcode |
| 1 | 74HCT153 | condition multiplexer (one half) |
| 1 | 74HCT138 | `dbus` decode |
| 4 | 74HCT00, 74HCT08 × 2, 74HCT27 | `abus`, bus A / B latch enables, HW, SW, `bnd_n`, `x_n`, P next (exact mapping in phase 3) |
| 1 + 1 | 74HCT377 + 74HCT541 | pending software interrupt port latch (C13) |
| **29** | | module `cu` |
| 2 + 1 + 1 + 1 | 74HCT74, 74HCT10 (NRND, spares; section 14), 74HCT14, 74HCT541 | module `clk`: clock, reset, clock buffer; plus the oscillator module |
| **34** | | control unit in all. Three inverters (the constant, `abus`) use the reset 74HCT14's spare gates. |

- **New parts:** 74HCT151 and 74HCT00. Their DIP and SOIC references and datasheets go into chips.md in phase 3; everything else is already in use or listed.
- **CPU total so far:** ALU 29 + flags 15 + registers 18 + address 34 + IR 7 + memory 8 + control 34 = **145 chips**, with **9 EEPROMs** (7 microcode + 2 boot ROM) and an oscillator.
- **Timing:** section 6. Every path passes at 25 °C; the tightest is the bypass, with 88 ns of margin (32 at 85 °C).
- **Bus loading added by the CU:**
  - `bus_data`: bypass inputs on bits 7..0 (2 on bits 7 and 6), plus the pending latch (Q4). That is +2 on the worst bit, about 410 pF instead of 386, +3 / +4 ns on the 6 mA drivers, and every C10 path keeps at least 135 ns (with Q9's 322 ns limit).
  - `bus_addr` low: one input less (the pending latch reads `bus_data`); its 541 output is already counted.

## 10. Questions for Gianni (each with my recommendation)

1. **Microcode width.** 7 × AT28C256, 54 bits, with the encodings of section 2 decoded *before* the pipeline register (8 × 574 + 175). **Recommended.** Decoding after the register saves 2 × 574, but the decoded signals can glitch and arrive later than C5's 41 ns.
2. **Fetch bypass.** The EEPROMs see the opcode and mode on `bus_data` while they are fetched (3 × 157). **Recommended**: NOP 1 step, n-byte fetch n steps. Without it, every instruction gets 1–2 blind steps.
3. **Where the sequences live** (4.7). **Recommended:**
   - interrupt entry and the halt loop in the forced row (00, 0);
   - the boot in the invalid-opcode row (3F, B), reached by loading the IR with SP high (FE);
   - the reset word in step 0 of every row.
4. **Pending port latch reads V1 from `bus_data`** during INT's fetch (C13 said `bus_addr`).
   - **Gain:** INT takes 4 steps instead of 6.
   - **Change:** INT no longer shows 00:V1 and I on the buses. The emulator sets `board.address_bus` / `data_bus` there, but nothing reads them and traces don't record them.
   - **Recommended.**
5. **INR in the last push step** (IE4) rather than in a step of its own after it (C21). The IOD contract (I6) is unchanged. **Recommended** (saves a step).
6. **INT valid by 150 ns** (I5: 250, provisional). The boundary decision goes INT → 3 gates → EEPROM → register and needs INT by 219 ns at 25 °C. **Recommended**: 150, easy for a synchronous IOD (a flip-flop's output).
7. **Invalid mode: I := 0 before H := 1** (C2 lists H first), so the halt loop never samples I = 1. **Recommended.**
8. **Boot loop leaves one step late** (cout sampled, 0000 copied twice, then PC − 1: 2 steps more than C20). **Recommended**: the EEPROMs can't branch on `addr_cout` within the same step. The alternative is a 16-input PC = FFFF detector.
9. **Clock block.**
   - A 74HCT541 clock buffer (with `clk_wr_n` and `clk_mem_wr_n` through the same chip).
   - An oscillator spec of 45–55 % duty, with C10 / I3 going from 335 to 322 ns.
   - **Recommended.** The alternative is an 8 MHz oscillator divided by 2 for an exact 4 MHz, which needs another flip-flop.
10. **Microcode in Logisim.**
    - **Problem:** every `at28c256` instance shares one ROM contents (M3), and the 7 microcode ROMs differ.
    - **Recommended:** turn the simulation-only `HALF` pin into a 4-bit `IMAGE` pin (16 × 32 KiB inside the model: boot ROM 0–1, microcode 2–8), listed in `netlist.SIM_ONLY`.
    - **Alternative:** 7 copies of the model.

## 11. For phases 2–4

- **Phase 2 (microcode source language + generator):**
  - **Source:** one readable source file. Each instruction × mode is a list of steps, each step a set of named actions, for example:
    ```
    add reg:    alu add a=r1 b=r2, flags czn
                write r1 <- res
    lod regram: lat := vx
                read -> r1, lat := pc
    ```
    The fetch steps (from `AddrMode::length`), the boundary, the stops and the sequences are added by the generator.
  - **Generator output:** it places the words (section 3 map), applies the encodings, and writes 7 images of 32 KiB, a listing, and the cycle-count table for the emulator.
  - **Generator checks:**
    - C5 (one source per bus, `regs_a_bus_oe_n` never with `regs_we_n`, bank ≠ 00);
    - C11 / P4 (no address load from bus A / B while writing);
    - C19's memory cases, C18 (1-byte opcodes don't depend on m1 m0; no field used past the length);
    - C9's step order, C1 (no register write in an ALU step);
    - every reachable (row, step, cond) defined.
- **Phase 3:** modules `cu` and `clk` from section 9's chips; tests against the generator's images and the emulator (`hbc2 vectors cu`); the datasheets for 151, 00 and the oscillator; timing.py `--section cu`.
- **Phase 4 (cpu.circ, `hbc2 compare`):**
  - **Trace lines:** the bench writes one when `ir_b0_ld_n` is low (each F0), and an INT line at each vector step (`abus` 1x).
  - **Cycle-accurate emulator:** it must sample INT in each instruction's last step (and in W1 / W2 when halted), and decide hardware or software from that same sample (4.3).
- **Changes to the other notes, after the go-ahead:**
  - C2 (Q7), C13 (Q4), C20 (Q8), C21 (Q5);
  - I3 / C10 (Q9), I5 (Q6);
  - chips.md (151, 00, the clock buffer, the oscillator).

## 12. Gianni's answers (2026-11-05)

**Go-ahead. All 10 recommendations of section 10 accepted**, and three more instructions:

| # | Decision | Applied in |
|---|---|---|
| Q1 | 7 × AT28C256, 54 bits, decoded before 8 × 574 + the 175 | section 13 (bit layout) |
| Q2 | fetch bypass (3 × 157) | section 13 |
| Q3 | forced row (00, 0), boot row (3F, B), reset word in step 0 | section 13 |
| Q4 | the pending port latch reads V1 from `bus_data` during INT's V1 fetch | control-requirements.md C13 |
| Q5 | INR in the last push step (IE4) | C21 |
| Q6 | INT valid by 150 ns, from a flip-flop clocked by `clk` | iod-requirements.md I5 (Gianni's wording) |
| Q7 | invalid mode: I := 0, then H := 1 | C2 |
| Q8 | boot loop leaves one step late, then PC − 1 | C20 |
| Q9 | 74HCT541 clock buffer; oscillator 45–55 % duty, register write data by 322 ns | C6, C10, iod-requirements.md I3 |
| Q10 | `at28c256` model: the simulation-only `HALF` pin becomes a 4-bit `IMAGE` pin (phase 3) | phase 3 |
| — | **Replace the 74HCT10** (CD74HCT10E is NRND at TI) with parts already in the design | below, C6, chips.md |
| — | iod-requirements.md: **INT comes from a flip-flop clocked by `clk`, valid by 150 ns** | I5 |
| — | update the notes listed in section 11 | done with this section |

### The 74HCT10 replacement

**Reverted 2026-11-05 (Gianni, section 14):** 3.5 ns before the next edge at 55 % duty is too tight; the 74HCT10 (NRND, still made) stays, with spares. The analysis below is kept for the record.

The 74HCT10 had two jobs: `clk_wr_n` = NAND3(`clk_n`, `osc`, Q2), and NOT `osc` for FF2's clock. Candidates, at max 25 °C (pass) / 85 °C (info), with the 670 needing a /WE pulse of 20 / 25 ns and a 5 ns hold before the next edge (registers-decisions.md D7):

| Option | `clk_wr_n` | Shortest /WE pulse | Latest /WE rise + hold | Result |
|---|---|---|---|---|
| 74HCT10 (before) | NAND3(`clk_n`, `osc`, Q2) | 49 / 30 | 428 / 440 | NRND |
| 74HCT11 + 74HCT14 (suggested) | NOT AND3(`clk_n`, `osc`, Q2) | **13** / −15 | 464 / 485 | fails the pulse: the start of the pulse goes through two gates |
| 74HCT27 + 74HCT14 | NOT NOR3(`clk`, `osc_n`, Q2_n) | **18** / −9 | 491 / 519 | fails the pulse |
| 74HCT32 × 2 gates + 74HCT14 (chosen, then reverted) | (`osc_n` OR Q2_n) OR `clk` | **49 / 30** | **484** / 510 | passes at 25 °C |
| 74HCT32, the other order | (`clk` OR Q2_n) OR `osc_n` | 25 / 0 | 460 / 480 | passes at 25 °C, but 12.5 ns with a 45 % duty |

- **How it works:** `clk_wr_n` = `clk` OR `osc_n` OR Q2_n is low only while `clk` is low, `osc` is high and Q2 is high, the same window as before. `osc_n` comes from a spare gate of the reset block's 74HCT14 and also clocks FF2 (the 10's second gate did that). Q2_n is FF2's /Q, which is `clk_mem_wr_n` already.
- **Why this order:** `clk` goes through one OR, so the start of the pulse is as early as with the 74HCT10 (the pulse is unchanged). Only the end moves: `osc_n` (14, 32 ns) and two ORs instead of one NAND.
- **Glitch-free:** an OR output only falls when all its inputs are low, and at each edge one input moves while another holds the output high (at 250 `clk` and `osc_n` both fall: the output falls once both are low; at 500 Q2_n is already high, from about 435 ns).
- **Margins at 25 °C:** pulse 49 ns (36 with a 45 % duty), latest rise + hold 484 ns (16 ns before the edge; **3.5 ns with a 55 % duty**). The write data deadline (C10, 335 / 322 ns) and `clk_mem_wr_n` (M1) don't change; FF2 clocks 8 ns later (14 instead of 10), which only shifts `clk_mem_wr_n`'s two edges together.
- **At 85 °C (information):** the hold is 10 ns short. Breadboard check with B1 (breadboard-checks.md), with the duty cycle measured. Fallback if it fails there: a faster inverter for `osc_n` (a 74HCT00 gate, 24 / 30 ns: +8 ns of margin), or the 74ACT74 fallback.
- **Chips:** module `clk` keeps 5 chips: 2 × 74HCT74, 74HCT32 (2 gates spare), 74HCT14, 74HCT541. The 74HCT10 leaves chips.md's lists.

## 13. Phase 2: the microcode (2026-11-05)

**Done:** the microcode for the whole instruction set, interrupts, halt and boot, compiled into the 7 EEPROM images, checked by the generator's rules and by a datapath model against the emulator. No circuit yet (phase 3).

| File | What |
|---|---|
| `hbc2/crates/hbc2-ucode/microcode/hbc2.uc` | **the microcode source** (its header is the language reference) |
| `hbc2/crates/hbc2-ucode/src/` | `source.rs` parser, `generate.rs` + `place.rs` generator, `word.rs` bit layout, `model.rs` datapath model, `check.rs` comparisons with the emulator |
| `logisim/circuits/cu/microcode/ucode0.bin` … `ucode6.bin` | the 7 AT28C256 images (32 KiB each; EEPROM k = bits 8k..8k+7) |
| `logisim/circuits/cu/microcode/listing.txt` | every used address: word, source line, next step, condition, the fields that aren't idle |
| [cu-cycles.md](cu-cycles.md) | cycle counts per instruction × mode (generated) |

Rebuild: `hbc2 ucode -o logisim/circuits/cu/microcode --cycles logisim/notes/cu-cycles.md` (add `--check` for the emulator comparisons). Tests: `cargo test -p hbc2-ucode`.

### 13.1 The source language

One line = one step. Example (ADD … RegRam, 8 steps with F0):

```
instr add adc sub sbb and or xor : regram
     f1                                   # macro: bus<-ram, ir1<-bus, lat<-pc+1, pc<-pc+1
     fv1
     fv2
     lat<-vx
     bus<-ram, lat<-pc
     alu=$alu, a=r1, flags<-$fl           # $alu, $fl: per-opcode parameters
     r1<-bus, bus<-res, end
```

| Kind | Syntax |
|---|---|
| bus_data source | `bus<-ram a res flags pchi pclo sphi splo io` |
| memory, I/O | `ram<-bus`, `ram<-rom` (boot copy), `io<-bus` (OUT), `hwvec`, `swvec`, `inr` |
| loads | `ir0<-bus ir1<-bus v1<-bus v2<-bus`, `swi<-bus` (pending latch, sets P), `r1<-bus` … `reg7<-bus`, `i<-bus` |
| buses A / B | `a=r1 r2 r3 i reg0..reg7` (default: the A latch); `b=r2 r1 r3 one zero` (default: the B latch) |
| ALU, flags | `alu=add sub adc sbb and or xor not shl shr asr`; `flags<-czn zn sef bus`; `C<-0` … `H<-1` |
| address section | `lat<-E`, `pc<-E`, `sp<-E` (one sum per step): `pc pc±1 sp sp±1 sp±v1 sp±vx a:b a:b+v2 0:b vx ivt ivt.hi 0` |
| exits | default: the next line; `next L`; `if jc/noint/hwq/xn L1 else L0`; `end` (boundary); `end nocheck` (an F0 copy); `end halt` (a W1 copy); `dispatch` (F0); `restart` (S1) |
| blocks | `seq NAME` (forced, reset, boot, stop1, stop2); `instr OPS : MODES`; `macro`, `param`; `-` = no step after F0 (NOP) |

### 13.2 Bit layout (54 bits + 2 spare, `word.rs`)

| EEPROM | Bits 0 → 7 |
|---|---|
| 0 | `alu_op` 0–3, `flags_op` 0–3 |
| 1 | `flags_bit_val`, `regs_we_n`, `regs_a_sel` 0–1, `regs_cu_idx` 0–1, `regs_cu_bank_n` 0–1 |
| 2 | `regs_b_sel` 0–1, `regs_const_oe_n`, `dbus` 0–2, `abus` 0–1 |
| 3 | `addr_base_sel` 0–1, `addr_base_oe_n` 0–1, `addr_off_sel`, `addr_off_oe_n` 0–1, `addr_inv` |
| 4 | `addr_cin`, `addr_lat_ld_n`, `addr_pc_ld_n`, `addr_sp_ld_n`, `ir_b0_ld_n`, `ir_b1_ld_n`, `ir_v1_ld_n`, `ir_v2_ld_n` |
| 5 | `mem_re`, `mem_oe_n`, `mem_we_n`, `mem_wbuf_oe_n`, `mem_rom_oe_n`, `inr`, `cu_swi_ld_n`, `cu_force` |
| 6 | `cu_next` 0–3, `cu_cond_sel` 0–1, spare × 2 (kept at 1) |

Each section's controls sit on one or two EEPROMs, so the board routes them in groups. Codes: `dbus` and `abus` as in section 2; `cu_cond_sel` 00 jc, 01 noint (`bnd_n`), 10 hwq, 11 xn.

### 13.3 Placement (as built)

| Where | Holds |
|---|---|
| every row, step 0 | the reset word (cond 0), S1 (cond 1) |
| forced row (00, 0) | step 1: the boundary pair (W1 / F0); steps 2–14: W2 / IE1h, W1 / IE1s, IE2h–IE5h, IE2s–IE5s, IE6–IE8; step 15 unused |
| boot row (3F, B) | step 1: the boundary pair (3F is an invalid opcode); steps 2–12: S2–S9, the copy loop C, FIX, an F0 copy |
| opcode with operands, each m3 m2 group | step 1: the byte-1 fetch, the same word in the group's 4 rows (m1 m0 aren't known yet; the generator refuses a group whose modes differ there). Its condition is jc if a mode branches right after it (JMx Reg16) |
| that opcode's mode rows | steps 2 onwards: the instruction, or `stop2` for a mode it doesn't accept |
| opcode without operands | rows with m3 m2 = 00: the instruction from step 1 (the 4 rows identical); other rows: `stop1` |
| invalid opcode 36–3F, NOP | step 1: the boundary pair (1 step) |
| unused (24,220 of 32,768 words) | a trap word: H := 1, then the boundary, so the CPU halts there (with I = 1 an interrupt still gets in) |

8,548 words are used.

### 13.4 Generator rules (checked at each build; `hbc2 ucode` prints them)

| Rule | Checked on |
|---|---|
| at most one source on `bus_data`; a write, an IR or pending latch load, or `flags<-bus` needs one; bus A never with a register write (C5); `b=one` / `b=zero` agree with `regs_const_0` = NOT `alu_op3` | 458 steps |
| memory strobes only C19's four cases; RE never with IE or a vector step; one `bus_addr` mode per step; OUT drives bus A | 458 steps |
| one adder sum per step (C11); no address load from bus A / B in a write step (P4) | 458 steps |
| ALU flags only in a step with `alu=` and no write (C1, C9); `bus<-res` only right after a full ALU step (or the sweep's AND with 00) | 55 uses |
| flag changes per opcode = `Opcode::info().flags` | 54 opcodes |
| C18: the fetch steps load exactly the mode's length, in order; no field (R1, R2, R3, V1, V2) used before its byte is loaded | 107 programs |
| every opcode × mode: a block exactly for the modes isa.rs lists; the rest NOP (invalid opcode) or stop | 1,024 rows |
| every step has a next step; every next address of every placed word holds a word | 8,548 words |

### 13.5 Datapath model and tests

`model.rs` runs the 7 images as the hardware reads them:
- the encodings of section 2;
- buses A and B, with the 670's transparent write;
- the adder, with the reset gating (A1);
- the ALU (alu_op table) and the flag register (flags_op);
- the IR, memory (C19), the pending port latch and P, `hw_q` and `cout_q`;
- the 157 bypass, the condition multiplexer, the reset generator's two signals;
- the IOD stub (I3–I6).

An undriven value is "unknown": loading or writing one fails, and so does any bus with two drivers.

| Test (`cargo test -p hbc2-ucode`; `hbc2 ucode --check` runs the long version) | Result |
|---|---|
| every opcode (64) × every mode pattern (16) × 64 random states: one instruction (plus the software entry after INT with I set); registers, flags, PC, SP, RAM and port writes; cycles = cu-cycles.md | **65,536 / 65,536** equal to cpu.rs |
| 4,000 random programs (mostly valid instructions, IVT and Imm16 targets on instruction starts), hardware interrupts raised at random boundaries, halts woken by an interrupt | **217,100** instructions and entries equal, **9,435** of them interrupt entries |
| boot from 3 power-up states (reset held 2, 3, 7 steps) | RAM = the ROM image, state = `Cpu::new()`, latch 0000, **65,548** steps |
| 14 mutations of the source: STI without the delay, taken / not taken swapped, entry without H := 0, no INR, boot exit inverted, R1 for R3, RET byte order, CMP with C Z N, flags in the write step, two bus sources, P4, no next step, a missing mode, a short fetch | **all 14 caught** (6 by the rules, 8 by the comparisons) |

The tests take about 4 s in a debug build; the `--check` version takes about 10 s in release.

### 13.6 Cycle counts

See [cu-cycles.md](cu-cycles.md). **Every count equals section 8's estimate.** A hardware entry takes 9 steps after the boundary, a software one 10. A stop takes 3 steps (no operands) or 4 before the halt loop.

### 13.7 Decisions taken in phase 2

| # | Decision | Why |
|---|---|---|
| U1 | IE8 ends with the normal boundary (`end`), not a no-check F0 | I and H are 0 there, so the check always gives F0; one F0 copy less |
| U2 | The reset word also does the sweep's step 0 (AND 00 into the result latch), and so does S1, so S2 writes register 0 at once | no extra step: still 8 sweep steps (C7) |
| U3 | Unused addresses hold a trap word that halts (H := 1, then the boundary) | an unplaced address can't run garbage; the generator also proves none is reachable |
| U4 | Step 1 of an operand opcode is shared by the 4 rows of its m3 m2 group; its condition is the one branch any mode needs (jc for JMx Reg16) | m1 m0 are only on `bus_data` during that step |
| U5 | The model's power-up starts on the reset word, not on random 574 contents | section 6: a brief contention is possible before the first edge, which a step model can't show |
| U6 | INT's pending latch loads in the V1 fetch step (`fv1, swi<-bus`), then one boundary step | Q4 as accepted: INT takes 4 steps |

### 13.8 For phase 3

- `at28c256` model: the 4-bit `IMAGE` pin (Q10); images 2–8 = `ucode0.bin` … `ucode6.bin`.
- `timing.py --section cu`, with a duty-cycle parameter (Q9) and the datasheet figures for the 74HCT151, the 74HCT00 and the oscillator.
- Test vectors for `cu` from the generator's images and the model (`hbc2 vectors cu`), with the IOD stub.

## 14. Phase 3: the circuits `cu` and `clk` (2026-11-05)

Gianni's go-ahead (2026-11-05):

| Item | Answer | Where |
|---|---|---|
| `clk_wr_n` | **Back to the 74HCT10 NAND3** (section 12's 74HCT32 + 74HCT14 left 3.5 ns before the next edge at 55 % duty). NRND isn't obsolete: CD74HCT10E is still made and stocked (DigiKey, Rochester Electronics second source); TI has no SN74HCT10N and the Nexperia 74HCT10N is discontinued; **SN74ACT10N** (TI, PDIP-14, active) is the drop-in fallback. Buy spares. | chips.md, C6, D7, timing.py |
| timing.py, every section | rerun with the 74HCT10: all pass at 25 °C (/WE pulse 49 ns, latest rise + hold 428 ns: 72 ns before the edge; RAM WE# pulse 174 ns) | — |
| U1–U6 | accepted | section 13.7 |
| U5, hardware | the pipeline 574s' /OE come from the reset (`rst_rc`, high from power-up), and resistor networks pull every control line to its idle level while they are off; the sequencer's lines are pulled to the reset word's (`cu_cond_sel` = 11, `cu_force` = 0), see C24. **No extra chip:** `rst_rc` is the reset 74HCT14's first output, already there. | C24, breadboard-checks.md B3 |

### 14.1 Module `clkgen` (circuits/cu/clkgen.circ, designs/clkgen.py)

| Item | As built |
|---|---|
| Name | **`clkgen`**, not `clk`: Logisim-evolution doesn't allow a pin labelled like its circuit, and the module's output is `clk` |
| Chips (5) | U1 74HCT74 (FF1 `clk`, FF2 Q2), U2 74HCT10 (`clk_wr_n` = NAND3(`clk_n`, `osc`, Q2); NOT `osc` for FF2; gate 3 spare), U3 74HCT541 (clock buffer), U4 74HCT14 (RC → `rst_rc` → /CLR; 4 gates spare), U5 74HCT74 (reset synchroniser) |
| Inputs | `clkgen_osc` (the oscillator module's output), `clkgen_rc` (the RC node at the 74HCT14 input: the RC, button and diode are analog) |
| Outputs | `clk` (5 bits: the 541's Y1–Y5, one branch each: 0 `ir`, 1 `addr`, 2 `cu`, 3 `regs` + `flags`, 4 the reset flip-flops + the IOD), `clk_wr_n` (Y7), `clk_mem_wr_n` (Y8), `rst_n`, `rst_early` (FF1's /Q, for `x_n`), `rst_rc` |
| Clock loads per branch | 6 (6 × 377), 6 (6 × 377), **11** (8 × 574, 2 × 175, 377), 5 (3 × 574, 2 × 175), 2 (the reset 74 and the IOD). `cu`'s branch is the heaviest: `timing.py --section cu` counts its extra delay (later edges in `cu` only make the other sections' hold times easier) |
| New global names | `rst_early`, `rst_rc`, and `clk_0` … `clk_4` for the branches (conventions.md, conventions.py) |
| Test (`python designs/clkgen.py --test`) | 12 runs from Logisim's power-up, one row per osc half period: RC released, the button pressed again, waveforms as C6 / C17 give them: **848 / 848**. A deliberately wrong expectation (`clk_wr_n` without Q2, `rst_n` one edge early) fails 96 rows |

The 74HCT14's 4 spare gates stay in `clkgen`: `cu` takes its own 74HCT14 rather than send 4 signals out and back between the modules (section 14.2).

