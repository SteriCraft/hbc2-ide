# Control unit: decisions

**Phase 1 (paper design, 2026-11-05): accepted.** Gianni's answers (all 10 recommendations accepted, plus the 74HCT10 replacement and the INT requirement) are in section 12. **Phase 2 (microcode) is in section 13. Phase 3 (circuits `cu` and `clk`) is in section 14**: there the 74HCT10 came back (section 12's replacement was reverted by Gianni). **Phase 4 (the whole CPU, cpu.circ, against the emulator) is in section 15.**

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
| Outputs | `clk` (6 bits: the 541's Y1–Y6, one branch each), `clk_wr_n` (Y7), `clk_mem_wr_n` (Y8), `rst_n`, `rst_early` (FF1's /Q, for `x_n`), `rst_rc` |
| Clock branches (section 7 item 1: about 5 inputs each) | 0 `ir` (6 × 377), 1 `addr` (6 × 377), 2 `cu` pipeline 574s 1–5, 3 `cu` 574s 6–8 + both 175s + the 377, 4 `regs` (3 × 574) + `flags` (2 × 175), 5 the reset 74 (2 clock inputs) + the IOD. Loads 6, 6, 5, 6, 5, 3: **skew from the loads 4 ns** at 25 °C (5 at 85 °C), `timing.py --section cu` |
| New global names | `rst_early`, `rst_rc`, and `clk_0` … `clk_5` for the branches (conventions.md, conventions.py) |
| Test (`python designs/clkgen.py --test`) | 12 runs from Logisim's power-up, one row per osc half period: RC released, the button pressed again, waveforms as C6 / C17 give them: **848 / 848**. A deliberately wrong expectation (`clk_wr_n` without Q2, `rst_n` one edge early) fails 96 rows |

### 14.2 Module `cu` (circuits/cu/cu.circ, designs/cu.py)

**30 chips** (phase 1 planned 29: see U7):

| Role | Chips |
|---|---|
| address | 3 × 74HCT157 (opcode / mode from the IR or, in the step that loads it, from `bus_data`; /E = `cu_force`), 74HCT151 (jc), 74HCT153 (one half: the condition), 74HCT175 (step register) |
| microcode | 7 × AT28C256 (the `at28c256` model, IMAGE 2–8) |
| decode | 74HCT138 (`dbus`), 74HCT00, 2 × 74HCT08, 74HCT14, 74HCT27 (gate by gate below) |
| pipeline | 8 × 74HCT574 (the model), /OE = `rst_rc`, 60 lines, 4 spare bits; **60 pull resistors** (36 up, 24 down: 8 bussed networks, `bom.py`) |
| state, pending port | 74HCT175 (`hw_q`, `cout_q`, P), 74HCT377 + 74HCT541 (C13) |

| Gate | Does | Before / after the 574s |
|---|---|---|
| 00 #1 | `cu_swi_oe_n` = NAND(`abus`) | before |
| 00 #2 | `regs_a_lat_oe_n` = NAND(`regs_cu_bank_n`) | before |
| 00 #3 | `regs_b_lat_oe_n` = NAND(`regs_b_sel` = 11, `regs_const_oe_n`) | before |
| 08 (1) #1, #2 | `addr_lat_oe_n` bit 1 = `abus1` · NOT `abus0`; IE = NOT `abus1` · `abus0` | before |
| 08 (1) #3, #4 | HW = I · INT, SW = I · P | after |
| 08 (2) #1 | `regs_b_sel` = 11 | before |
| 08 (2) #2 | P · `cu_swi_oe_n` (P kept) | after |
| 14 #1–#3 | NOT `alu_op3` (`regs_const_0`), NOT `abus0`, NOT `abus1` | before |
| 14 #4, #5 | NOT `cu_swi_ld_n`; P_next = NOT the 27's NOR | after |
| 27 #1, #2, #3 | `bnd_n` = NOR3(HW, SW, H); `x_n` = NOR3(SW, `cout_q`, `rst_early`); NOR(P kept, NOT `cu_swi_ld_n`) | after |
| 138 | `dbus` → `regs_a_bus_oe_n` and RW (Y1), `regs_res_oe_n` (Y2), `flags_oe_n` (Y3), PC hi / lo (Y4, Y5), SP hi / lo (Y6, Y7) | before |

`addr_lat_oe_n` bit 0 is `abus1` itself (a wire). Spare: one 00, two 08, one 14 gate, the 153's second half, the 157 #3's two last muxes.

| Port | Names |
|---|---|
| inputs | `clk` (2 bits: clkgen branches 2 and 3), `rst_n`, `rst_early`, `rst_rc`, `cu_bus_in` (8), `cu_ir_op` (6), `cu_ir_mode` (4), `cu_flag_c` … `cu_flag_h`, `cu_int`, `cu_addr_cout` |
| outputs | `cu_` + the receiving section's name for each control line (`cu_alu_op`, `cu_regs_we_n`, …, `cu_ie`, `cu_rw`, `cu_inr`: 55 lines in 39 pins), and `cu_bus_addr_lo` (8, the pending port onto the low byte of `bus_addr`). Phase 4's `main` wires each to its section |
| inside only | `cu_force`, `cu_cond_sel`, `cu_swi_ld_n`, `cu_swi_oe_n` (574 outputs, pulled too) |

### 14.3 Tests (`hbc2 vectors cu`, `python designs/cu.py --test`)

The reference is the microcode as built, run by the datapath model (section 13.5, equal to the emulator). Each step gives `cu` what the model's sections give it (`bus_data`, the IR, the flags, INT, `addr_cout`, the reset lines) and expects the decoded word (`word::outputs`) or, while `rst_rc` is high, the pulled levels (`word::pulled`). One step = 2 rows.

| Run | Result |
|---|---|
| 10 sets from Logisim's power-up, each: the reset held 3–8 steps (`rst_rc` high for all but the last 2), the whole boot, a random program with hardware interrupts from the IOD stub and halts woken by an interrupt; when the CPU stops for good (an invalid mode, HLT with I = 0), the button and a new boot, up to 3 per set. Half of the power-ups have the IR's opcode bits 2..0 = 111 (jc = 1 during the reset) | **4,217,898 / 4,217,898** rows on the final circuit (2026-11-10: 2.1 million steps, 30 boots, about 141,000 program steps, 279 of the 1,024 opcode × mode rows), **no bus contention** on `cu_bus_addr_lo` |
| 9 deliberate wiring bugs on the final gate map (153 selects swapped; mode 1..0 bypass on byte 0's load; P kept by the load; the 151's F input wired to C; `x_n` without `rst_early`; `cu_cond_sel` pulled to 00, to 01; the 574 /OE tied low, i.e. no C24; the B latch enable without the constant) | **all 9 caught** (2-set runs, 2 to 19,910 failing rows each). A pull to 10 is not caught, and is harmless: it selects `hw_q`, which the reset clears |

Coverage: the program runs are short (random stores overwrite the program), so 279 rows ran. The ROM contents of every row are checked elsewhere: the `at28c256` test reads images 2–8 in full, and the model runs every opcode × mode against the emulator (section 13.5). This test checks what only the circuit can get wrong: the address (every opcode, mode, step and condition bit through the bypass and the IR), the decode, the pipeline, the sequencing, P, the pending port and the reset with C24.

**The circuit test found a model bug** (fixed, U11): the step register is cleared at the reset's last edge, so the first step after the release still reads step 0, whatever that word's `cu_next` says. The model used `rst_n` of the current step. It only mattered when the power-up IR was on the boot row (3F, B), whose S1 goes on to S2: the model ran S1 once and the hardware twice. `check::boot` now includes that power-up state (65,548 steps, as the others).

### 14.4 Timing (`timing.py --section cu`, `--duty`)

`--duty PCT` (default 50) sets the oscillator's duty cycle: the osc falls (and with them `clk_wr_n`'s rise, `clk_mem_wr_n`'s edges, the write data deadline) move. **Every section passes at 25 °C at 45, 50 and 55 %.** The `cu` paths use the datasheet figures now (74HCT151 CD74HCT151E, 74HCT00 CD74HCT00E, 74HCT157 /E → Y, chips.md); the slowest decode is `abus` (14 → 08, 56 ns at 25 °C), not the 138 (36) of section 6.

| Path (to the 574 setup, or as named) | Max 25 °C | Margin 25 / 85 °C |
|---|---|---|
| step register → EEPROM → decode | 261 | 239 / 211 |
| IR (just loaded) → 157 → EEPROM → decode | 291 | 209 / 173 |
| **bypass** (RAM → `bus_data` → 157 → EEPROM → decode) | **439** | **61 / −2** |
| bypass → `cu_next` → step register | 391 | 109 / 58 |
| `cu_force` → 157 /E | 288 | 212 / 177 |
| jc: IR opcode → 151 → 153 | 341 | 159 / 110 |
| **boundary**: INT valid by 150 (I5) → 08 → 27 → 153 | **459** | **41 / 0** |
| `hw_q` / `cout_q` / `rst_early` → 27 → 153 | 318 | 182 / 139 |
| `cu_cond_sel` → 153 S | 295 | 205 / 168 |
| boot loop: PC + 1 → `addr_cout` → `cout_q` | 278 | 222 / 150 |
| P next | 140 | 360 / 325 |
| pending port: V1 fetched → 377 setup | 198 | 302 / 262 |
| pending port onto `bus_addr` (by the IOD's deadline) | 74 | 163 / 76 |

| Clock generator, max 25 °C | 45 % | 50 % | 55 % |
|---|---|---|---|
| /WE pulse, shortest (≥ 20) | 36.5 | 49 | 61.5 |
| /WE rise + hold, latest (≤ 500) | 415.5 | 428 | 440.5 |
| write data on `bus_data` by (C10) | 322.5 | 335 | 347.5 |
| RAM WE# pulse (≥ 45) | 174 | 174 | 174 |
| address before RAM WE# falls (margin) | 12.5 | 25 | 37.5 |

At 85 °C (information only): the bypass is 2 ns over, the boundary has 0, the /WE pulse 7.5 ns short at 45 %; the others as in sections regs, addr, mem.

### 14.5 Decisions taken in phase 3

| # | Decision | Why |
|---|---|---|
| U7 | `cu` has its own 74HCT14 (+1 chip: `cu` 30, the control unit 35, the CPU 146) instead of the reset 74HCT14's spare gates in `clkgen` | phase 1's plan would send 4 signals out of `cu` and back; and the B-latch NAND3 had no gate in it (U8) |
| U8 | `regs_b_lat_oe_n` = NAND(AND(`regs_b_sel`), `regs_const_oe_n`) (08 → 00, 44 ns); P_next moved to spare 08, 14, 27 gates (140 ns, 360 ns of slack) to free the NAND | section 2 listed a NAND3 no chip provided; AND-AND-inverter would have been 80 ns on the bypass path |
| U9 | Six balanced clock branches; `cu` takes two (its `clk` is 2 bits) | one branch for `cu` had 11 loads: about 9 ns of skew against the others, on hold times section 6 already can't prove |
| U10 | The clock module is `clkgen` | Logisim-evolution refuses a pin named like its circuit (`clk`) |
| U11 | Model fixes: the step register reads 0 in the first step after the reset (above); AND with 00 gives 00 even from an unloaded A latch | the first was a model bug the circuit test found; the second is what the 74HCT08s do, and the boot-row power-up needed it |
| U12 | `cu`'s outputs are `cu_<the receiver's name>` (`cu_regs_we_n` → `regs_we_n` in `main`) | the module prefix rule (conventions.md); the receiver's name keeps the mapping obvious |
| U13 | The RC-level reset is `rst_rc` (C24 called it `rst_oe`) | it is the reset itself (high while the RC is low), used as an /OE |

### 14.6 For phase 4

- `cpu.circ` `main`: `clkgen`, `cu` and the sections; `cu_<x>` → `<x>`, `clk` bits per section 14.1, `cu_bus_addr_lo` onto the low byte of `bus_addr`.
- `hbc2 compare`: the whole CPU against the emulator; the internal buses as output pins (conventions.md).
- The IOD stub's clock is branch 5; INT must come from a `clk` flip-flop by 150 ns (I5): the boundary path has 0 ns left at 85 °C.

### 14.7 Status (2026-11-10)

Phase 3 is complete: the full test and the 9 wiring bugs ran on the final circuit (14.3). Phase 4 (`cpu.circ`): section 15.

## 15. Phase 4: the whole CPU (2026-11-10)

Gianni's go-ahead (2026-11-10): `cpu.circ` `main` from every section, `cu_<receiver>` wired to the receivers (U12); **the Rust crates are not edited** (an emulator branch waits to be merged: an emulator bug is reported, not fixed); the IOD stays a stub; validation: no contention anywhere, a full power-up, `hbc2 compare` against `hbc2 trace` (test programs, interrupts, halt, random programs), cycle counts against cu-cycles.md, wiring bugs between sections, timing across sections, the final part list.

### 15.1 `main` (logisim/cpu.circ, designs/cpu.py)

| Item | As built |
|---|---|
| Instances | `clkgen`, `cu`, `alu`, `flags`, `regs`, `addr`, `ir`, `mem`: every port to a tunnel named after its net (`main` holds no logic; conventions.py: 0 errors, 0 warnings) |
| Nets | `cu_<x>` → `<x>` (U12); `bus_data` = every `*_bus_in` and `*_bus_out` (`flags`, `regs`, `addr`, `mem`) plus the IOD; `bus_addr` = `bus_addr_hi` : `bus_addr_lo` (`addr`'s drivers, `cu`'s pending port, the IOD's port in the vector step); bus A / bus B = `regs_a` / `regs_b` → `alu_a` / `alu_b`, `addr_bus_a` / `addr_bus_b`; `alu_cin` = `flag_c`; the ALU's flags → `flags_alu_*`; `alu_r` → `regs_alu_r`; IR fields → `regs_r*`, `addr_v1` / `addr_v2`, `cu_ir_op` / `cu_ir_mode`; the flags → `cu_flag_*`; `addr_cout` → `cu_addr_cout` |
| Clocks (14.1) | `clk_0` ir, `clk_1` addr, `clk_2` + `clk_3` cu, `clk_4` regs + flags, `clk_5` the reset flip-flops (inside clkgen) and the IOD; `clk_wr_n` regs, `clk_mem_wr_n` mem |
| Pins (U14) | in: `osc`, `rc` (the RC node), the IOD connector: `int`, `iod_bus_data`, `iod_bus_addr_lo`. out: the connector (`bus_data`, `bus_addr`, `rw`, `ie`, `inr`, `clk`, `clk_wr_n`, `rst_n`), the internal buses (`bus_a`, `bus_b`, `mem_d`), and test points: every control line, the IR fields, `alu_r`, `flags` (8, bit 0 = C … 7 = H), `addr_cout`, `rst_early`, `rst_rc`, `clk_mem_wr_n` |
| Chips | **146** in 22 part numbers + 8 resistor networks (notes/bom-cpu.csv, chips.md "Whole CPU"): alu 29, flags 15, regs 18, addr 34, ir 7, mem 8 (the boot ROM's 2 AT28C256 included), clkgen 5, cu 30 (the 7 microcode AT28C256 included) |

### 15.2 The trace bench (circuits/cpu/test_cpu.circ, designs/test_cpu.py) and tools/cputest.py

| Part | What it does |
|---|---|
| Clock | a Logisim Clock on `osc`: one tick = one oscillator half period (4 rows per 500 ns step) |
| Reset | `rc` low from Logisim's power-up, high after 15 oscillator periods (a 74161 and a 7474: the bench's RC) |
| IOD stub (U16) | port latches: a 256-byte RAM, 00 at power-up (the emulator's IOD without peripherals); OUT writes it while `clk_wr_n` is low (captured at its rise, I4), IN reads it (I3). Interrupt source: OUT to a port F0–FF with v > 0 raises INT v steps later for port p = the low 4 bits, from a flip-flop on `clk_5` (I5), cleared when INR is seen; the vector step drives 00:p (low byte) and the port latch of p (I6). |
| Logisim run | `logisim-evolution -tty table` on a copy of logisim/ whose boot ROM (the at28c256 model's images 0–1) holds the program; every run starts at Logisim's power-up: reset, register sweep, the 64 KiB boot copy, then the program. About 650 steps a second alone, 2 to 3 minutes per run (the boot is 65,548 steps) |
| Checks on every row | no E on any output pin (all buses: `bus_data`, `bus_addr`, bus A, bus B, `mem_d`, and every control line); while `rst_rc` is high, every control line at its pulled level (C24); every boot copy write (RAM write with the ROM strobes) carries the image's byte for its address |
| Trace (U15) | from the pins: `pc` and the instruction bytes at F0–F3 (consecutive fetch steps at PC, PC + 1 …), `mem_w` from the RAM writes (address, `mem_d` while the strobe is low), `io_w` from the OUT steps, `flags` from the flag pins, an `INT` line at each vector step (the step that loads PC from the IVT: `addr_base_sel` = 11 with `addr_pc_ld_n` = 0), `pc` of an INT line from its pushes. The registers and SP can't be read from Logisim (74670 / 74377 contents): cputest keeps them from their write events on the section ports (the register write index from `regs_a_sel` and its sources, `bus_data` at `clk_wr_n`'s rise; the address section's sum from its ports, loaded at the edge) and **checks them against the hardware every time it shows them**: every port A / port B read on bus A / bus B, the address latch on `bus_addr`, PC / SP on `bus_data`, and `addr_cout` against the computed carry. Any mismatch fails the run. |
| Reference | `hbc2 trace` on the same RAM image; when the stub raised hardware interrupts, `emutrace` (logisim/tools/emutrace: a standalone package using hbc2-core as a library, nothing in the crates changed): the same trace with each interrupt queued in the emulator's IOD just before the line during which INT rose in Logisim, with the port and data the stub put on the buses. Without interrupts emutrace and `hbc2 trace` give identical files. Then `hbc2 compare`. |
| End of a run | H set with I clear for 20 steps (an entry from the halt loop clears I one step before H, so the bench's halt pin is tied low: U18), halted 300 steps with INT low, or a step limit |

### 15.3 Results

| Run | Result |
|---|---|
| Test programs (logisim/tests/cpu): `loop` 25 lines, `allops` 202, `swint` 33 (3 software interrupts, POF setting H), `hwint` 903 (45 hardware interrupts: INT's rise on every step of a block, while I = 0, right after STI, inside a handler, in the halt loop, against a software interrupt; 1 software), `invalid` 8 (invalid opcodes, then an invalid mode) | **5 / 5 identical** to the emulator (`hbc2 trace`; `emutrace` for hwint) |
| Random programs (`emutrace random 0..49`, the phase 2 generator) | **50 / 50 identical** (60 counting a first batch of seeds 0–9); 21 of them took hardware interrupts (random OUTs to ports F0–FF), most ended on an invalid mode or CLI + HLT |
| Every run (55) | a **full power-up**: reset, register sweep, **65,537 boot copy writes all equal to the image**, first fetch **65,548 steps** after the reset's release (cu-cycles.md); 3.85 million steps, 15.4 million rows; **no E in any row** (reset, sweep and copy included), **no C24 error**, **no cross-check error** (registers, latch, PC / SP, `addr_cout`). 56,997 trace lines, 86 hardware and 1,786 software interrupt entries |
| Cycle counts (`cputest.py cycles`, 62,240 instructions of the matching runs) | **all equal to cu-cycles.md**; **106 / 106** instruction × mode cells reached; the stop column (valid opcode, invalid mode) 29 / 54 opcodes (each stop ends a run: only a reset restarts the CPU, so a dedicated program per opcode would cost a boot each); invalid opcodes 23 |
| Wiring bugs between sections (`cputest.py mutants`, `allops` on each) | **10 / 10 caught**: ALU carry in from Z; R1 / R2 swapped into regs (also E on bus A); V1 / V2 swapped into addr (also the latch check); ALU N / S swapped into flags; cu's Z condition from C; the pending port not on `bus_addr`; the IVT port read from `bus_data` (also the latch check); bus A into the ALU's B input (also E on `alu_r`); the flags' bus input from `alu_r`; INT not wired to cu (the CPU hangs after its first fetch: no line). `python designs/cpu.py` then rewrites cpu.circ identical to the committed one |
| Emulator bugs found | none |

**Timing** (`timing.py --section all` and `--section cpu`, 45 / 50 / 55 % duty): **every section passes at max 25 °C** at the three duties. The new paths across sections (`--section cpu`, 50 %), max 25 °C, margin 25 / 85 °C:

| Path | Max 25 °C | Margin 25 / 85 °C |
|---|---|---|
| edge → control → bus A / bus B valid | 158 | 342 / 298 |
| edge → regs → ALU → flags (alu_z) → 175 setup | 417 | 83 / **−27** |
| edge → regs → ALU (alu_r) → result latch setup | 324 | 176 / 90 |
| edge → alu_op → ALU → flags setup | 292 | 208 / 134 |
| edge → regs → address adder → PC / SP / latch setup | 421 | 79 / **−32** |
| edge → regs → bus_data → write buffer → RAM data (by 322) | 244 | 78 / 3 |
| edge → regs → bus_data: OUT data for the IOD (by 242, I4) | 217 | **25** / **−36** |
| edge → latch → RAM → bus_data → IR setup | 230 | 270 / 222 |
| edge → latch → RAM → bus_data (register write data, by 335) | 186 | 149 / 102 |
| edge → RAM → bus_data → cu bypass → 574 setup | 439 | 61 / **−2** |
| edge → flags → jump condition → EEPROM → 574 setup | 333 | 167 / 120 |
| IOD port → IVT → adder → PC setup | 263 | 237 / 170 |
| IOD INT (by 150) → boundary → 574 setup | 459 | 41 / 0 |
| edge → PC → carry out → `cout_q` (boot loop) | 278 | 222 / 150 |

Worst at 25 °C over all sections: 12.5 ns at 45 % duty (the address before the RAM's WE# falls, section mem), then 16.5 (the /WE pulse at 45 %) and 25 (OUT data, above). At 85 °C (information): the five paths in bold above, the /WE pulse (−7.5 at 45 %) and the memory write timing at 45 % (−14.5 address before WE#, −9.5 / −13.5 write data), as in sections 14.4 and the mem / regs reports.

### 15.4 Pull resistors while `rst_rc` holds the pipeline off (C24)

Checked on every row with `rst_rc` high (29 rows per power-up): **no line off its pulled level**, no E. The levels are the idle word's (cu.py `PIPE`), so each bus keeps exactly one driver during the reset: the address latch drives `bus_addr`, the operand latches drive bus A and bus B, nothing drives `bus_data`.

| Pulled | Lines |
|---|---|
| **up (36)** | `flags_oe_n`, `regs_we_n`, `regs_a_sel` (2: port A off with `regs_cu_bank_n` 11), `regs_cu_bank_n` (2), `regs_b_sel` (2: port B off), `regs_const_oe_n`, `regs_const_0` (don't care while the constant is off), `regs_a_bus_oe_n`, `regs_res_oe_n`, `addr_base_oe_n` (2), `addr_off_oe_n` (2), `addr_lat_ld_n`, `addr_pc_ld_n`, `addr_sp_ld_n`, `addr_pc_oe_n` (2), `addr_sp_oe_n` (2), `ir_b0_ld_n`, `ir_b1_ld_n`, `ir_v1_ld_n`, `ir_v2_ld_n`, `mem_oe_n`, `mem_we_n`, `mem_wbuf_oe_n`, `mem_rom_oe_n`, `rw` (read: the IOD writes nothing), inside `cu`: `swi_ld_n`, `swi_oe_n`, `cond_sel` (2: 11, `x_n`) |
| **down (24)** | `alu_op` (4), `flags_op` (4: hold), `flags_bit_val`, `regs_cu_idx` (2), `regs_a_lat_oe_n` and `regs_b_lat_oe_n` (**on**: the operand latches hold bus A / bus B), `addr_base_sel` (2), `addr_off_sel`, `addr_inv`, `addr_cin`, `addr_lat_oe_n` (2, **on**: the address latch holds `bus_addr`), `mem_re`, `ie`, `inr`, inside `cu`: `force` |

Every strobe and load is inactive (no register, flag, PC / SP / latch, IR, RAM or IOD write; no driver onto `bus_data`). The three lines pulled to an enable keep the buses that the idle word drives driven, so no input floats; their latches hold whatever Logisim (or the board) powers up with, which nothing loads during the reset.

### 15.5 Decisions taken in phase 4

| # | Decision | Why |
|---|---|---|
| U14 | `main`'s pins: the IOD connector, with the bidirectional lines split into an input joined to the bus net (`iod_bus_data`, `iod_bus_addr_lo`) and the bus as an output; plus test points (every control line, the IR fields, the flags, the internal buses) | a Logisim subcircuit pin is one-way (as the RAM model's DQ pins, M2); the trace bench can only see `main`'s pins, and conventions.md asks for the internal buses as outputs |
| U15 | The trace's registers and SP are kept by cputest from the write events on the section ports, and checked against every read the hardware shows (bus A / B, the latch, PC / SP on `bus_data`, `addr_cout`) | Logisim gives no access to the 74670s' and 377s' contents; a module port for it would be a simulation-only port on a module. A wrong write shows at the next read of that register, and in the trace |
| U16 | The IOD stub in the bench: port latches (a 256-byte RAM, 00 at power-up, as the emulator's IOD without peripherals) and an interrupt source armed by the program (OUT F0–FF, v); hardware interrupts reach the emulator through `emutrace`, queued just before the line during which INT rose in Logisim | `hbc2 trace` has no way to raise a hardware interrupt without peripherals; the stub is not the real IOD (out of scope, iod-requirements.md) |
| U17 | Test programs in logisim/tests/cpu: `loop` (hbc2-asm's trace test), `allops` (every instruction in every mode, every jump condition both ways), `swint`, `hwint`, `invalid`; random programs from `check::random_program` with the cu vectors' settings (`emutrace random k`) | the existing trace tests, plus coverage of every row of cu-cycles.md that a run can reach |
| U18 | The bench's `halt` pin is tied low; cputest ends a run (H set with I clear for 20 steps, halted 300 steps with no INT, or limits) and kills Logisim | an entry from the halt loop shows H = 1, I = 0 for one step (IE1 clears I, IE2 clears H) |
| U19 | Cycle counts: F0 to the next F0, or to W1 when an interrupt entry follows; for a halt, to the step where H shows, + 1 for POF and IRT | H is loaded by POF / IRT one step before their boundary step, by HLT and the stop sequence in their last step |
| U20 | timing.py `--section cpu`: the paths across sections from the clock edge (the later of the IR's 377s and the pipeline's 574s), and `--section all` | the sections' own paths start from "control valid" and stop at their ports |

### 15.6 Status (2026-11-10)

Phase 4 is complete: the whole CPU runs from power-up through the boot and matches the emulator on every program tried (5 test programs, 60 random programs), with no contention, the pulls of C24 holding, every cycle count as cu-cycles.md, and every wiring bug tried caught. Open: the stop column covered for 29 of 54 opcodes; the 85 °C margins above (information); the IOD and its peripherals (a stub here); M4's re-check on the new datasheets; INT's 0 ns at 85 °C (I5).
