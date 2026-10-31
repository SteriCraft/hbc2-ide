# HBC-2 Integer Math Library — Reference

Integer arithmetic, logic, bit counting, random numbers and number/text
conversions for the HBC-2, in plain assembly.

```
.include "lib/math/math.has"
```

`math.has` includes every file of the library, so this one line is enough.
Nothing runs at include time: the files only add routines and a few variables.

---

## Contents

1. [Conventions](#1-conventions)
2. [Quick index](#2-quick-index)
3. [Memory helpers](#3-memory-helpers)
4. [Unsigned 16-bit arithmetic](#4-unsigned-16-bit-arithmetic)
5. [Unsigned 32-bit arithmetic](#5-unsigned-32-bit-arithmetic)
6. [Signed 16-bit arithmetic](#6-signed-16-bit-arithmetic)
7. [Signed 32-bit arithmetic](#7-signed-32-bit-arithmetic)
8. [Logic operations and shifts](#8-logic-operations-and-shifts)
9. [Bit counting](#9-bit-counting)
10. [Random numbers](#10-random-numbers)
11. [Number ↔ text](#11-number--text)
12. [Interrupts and work areas](#12-interrupts-and-work-areas)
13. [Speed](#13-speed)
14. [Files](#14-files)

---

## 1. Conventions

### Where the values are

| Size | 1st operand / result | 2nd operand |
|---|---|---|
| 8-bit | `a` | `b` |
| 16-bit | `[a:b]` | `[c:d]` |
| 32-bit | 4 bytes in memory at `[x:y]` | 4 bytes in memory at `[i:j]` |

* A register pair `[a:b]` holds a 16-bit value with the first register as
  the MSB: `[a:b] = 0x1234` means `a = 0x12`, `b = 0x34`.
* 32-bit values live in memory, **big-endian** (MSB at the lowest address),
  exactly as `.data32` writes them. The routines take their **address**:
  `[x:y]` points to the 1st operand, which usually receives the result,
  and `[i:j]` to the 2nd.

```
.data32 total 100000
.data32 step  250

    mov x, total.msb        ; [x:y] -> total
    mov y, total.lsb
    mov i, step.msb         ; [i:j] -> step
    mov j, step.lsb
    cal add32               ; total = 100250
```

### Registers

Every routine **restores every register except its outputs**. The outputs
are listed for each routine. The few that also move a pointer (the text
routines) say so.

### Flags

Each routine lists the flags it sets. **A flag that is not listed is
undefined** after the call: don't test it.

| Flag | Meaning in this library | Jump |
|---|---|---|
| C | carry, borrow, overflow or error (see each routine) | `jmc` |
| Z | the result is 0 | `jmz` |
| N | the top bit of the result (negative, if signed) | `jmn` |
| S / E / F | comparisons: greater / equal / smaller, like `cmp` | `jms` / `jme` / `jmf` |

### Errors

A routine that can fail (division by zero, empty text, wrong range…)
sets **C** and, unless said otherwise, **changes nothing**: the operands and
the result stay as they were.

### Interrupts

Every routine except the random ones only uses registers and the stack,
so it can be called from anywhere, interrupt handlers included. The random
routines share the generator's state: see
[section 12](#12-interrupts-and-work-areas) before calling them from an
interrupt handler.

---

## 2. Quick index

| Routine | Does | Section |
|---|---|---|
| `load16` `store16` `copy16` | move a 16-bit value between registers and memory | [3](#3-memory-helpers) |
| `load32` `store32` `copy32` | move a 32-bit value between registers and memory | [3](#3-memory-helpers) |
| `clr32` `tst32` | set to 0, test for 0 | [3](#3-memory-helpers) |
| `add16` `sub16` `cmp16` | add, subtract, compare (unsigned) | [4](#4-unsigned-16-bit-arithmetic) |
| `inc16` `dec16` | +1, −1 | [4](#4-unsigned-16-bit-arithmetic) |
| `min16` `max16` | smaller, larger (unsigned) | [4](#4-unsigned-16-bit-arithmetic) |
| `mul16` `div16` `pow16` | multiply (32-bit product), divide, power | [4](#4-unsigned-16-bit-arithmetic) |
| `gcd16` `isqrt16` | greatest common divisor, square root | [4](#4-unsigned-16-bit-arithmetic) |
| `add32` `sub32` `cmp32` | add, subtract, compare (unsigned) | [5](#5-unsigned-32-bit-arithmetic) |
| `inc32` `dec32` | +1, −1 | [5](#5-unsigned-32-bit-arithmetic) |
| `min32` `max32` | smaller, larger (unsigned) | [5](#5-unsigned-32-bit-arithmetic) |
| `mul32` `div32` `pow32` | multiply, divide, power | [5](#5-unsigned-32-bit-arithmetic) |
| `gcd32` `isqrt32` | greatest common divisor, square root | [5](#5-unsigned-32-bit-arithmetic) |
| `add16s` `sub16s` `cmp16s` | add, subtract, compare (signed) | [6](#6-signed-16-bit-arithmetic) |
| `mul16s` `div16s` | multiply, divide (signed) | [6](#6-signed-16-bit-arithmetic) |
| `neg16` `abs16` `min16s` `max16s` `sext8` | negate, absolute value, min/max, sign-extend | [6](#6-signed-16-bit-arithmetic) |
| `add32s` `sub32s` `cmp32s` | add, subtract, compare (signed) | [7](#7-signed-32-bit-arithmetic) |
| `mul32s` `div32s` | multiply, divide (signed) | [7](#7-signed-32-bit-arithmetic) |
| `neg32` `abs32` `min32s` `max32s` `sext16` | negate, absolute value, min/max, sign-extend | [7](#7-signed-32-bit-arithmetic) |
| `and16` `or16` `xor16` `not16` | bitwise logic, 16-bit | [8](#8-logic-operations-and-shifts) |
| `and32` `or32` `xor32` `not32` | bitwise logic, 32-bit | [8](#8-logic-operations-and-shifts) |
| `shl16` `shr16` `asr16` / `shl32` `shr32` `asr32` | shift by 1 bit | [8](#8-logic-operations-and-shifts) |
| `shl16n` `shr16n` `asr16n` / `shl32n` `shr32n` `asr32n` | shift by any number of bits | [8](#8-logic-operations-and-shifts) |
| `popcnt8/16/32` `lzcnt8/16/32` `tzcnt8/16/32` | count 1 bits, leading zeros, trailing zeros | [9](#9-bit-counting) |
| `srand` `srand_rtc` | seed the random generator | [10](#10-random-numbers) |
| `rand8` `rand16` `rand32` | random number | [10](#10-random-numbers) |
| `randr8` `randr16` `randr32` | random number in a range | [10](#10-random-numbers) |
| `utoa8/16/32` `itoa8/16/32` | number → decimal text | [11](#11-number--text) |
| `htoa8/16/32` | number → hexadecimal text | [11](#11-number--text) |
| `atou` `atoi` | decimal text → number | [11](#11-number--text) |

---

## 3. Memory helpers

| Routine | Does | Flags |
|---|---|---|
| `load16` | `[a:b]` = 16-bit value at `[x:y]` | — |
| `store16` | 16-bit value at `[x:y]` = `[a:b]` | — |
| `copy16` | 16-bit value at `[x:y]` = 16-bit value at `[i:j]` | — |
| `load32` | `[a:b:c:d]` = 32-bit value at `[x:y]` | — |
| `store32` | 32-bit value at `[x:y]` = `[a:b:c:d]` | — |
| `copy32` | 32-bit value at `[x:y]` = 32-bit value at `[i:j]` | — |
| `clr32` | 32-bit value at `[x:y]` = 0 | — |
| `tst32` | tests the 32-bit value at `[x:y]` | Z = the value is 0 |

`load32` / `store32` are the way to work on a 32-bit value in registers:
`[a:b:c:d]` holds it MSB first.

---

## 4. Unsigned 16-bit arithmetic

Values from 0 to 65535 in `[a:b]` (1st operand, result) and `[c:d]`
(2nd operand).

| Routine | Does | Flags |
|---|---|---|
| `add16` | `[a:b]` = `[a:b]` + `[c:d]` | C = overflow (past 65535), Z |
| `sub16` | `[a:b]` = `[a:b]` − `[c:d]` | C = borrow (`[a:b]` < `[c:d]`), Z |
| `cmp16` | compares `[a:b]` with `[c:d]`, like `cmp` | S = greater, E = equal, F = smaller |
| `inc16` | `[a:b]` = `[a:b]` + 1 | C = wrap-around (0xFFFF + 1 = 0), Z, N = bit 15 |
| `dec16` | `[a:b]` = `[a:b]` − 1 | C = wrap-around (0 − 1 = 0xFFFF), Z, N = bit 15 |
| `min16` | `[a:b]` = the smaller of `[a:b]` and `[c:d]` | — |
| `max16` | `[a:b]` = the larger of `[a:b]` and `[c:d]` | — |
| `mul16` | `[a:b:c:d]` = `[a:b]` × `[c:d]` (full 32-bit product) | — |
| `div16` | `[a:b]` = `[a:b]` / `[c:d]`, `[c:d]` = remainder | C = division by zero |
| `pow16` | `[a:b]` = `[a:b]` ^ `c` (exponent 0 to 255) | C = overflow |
| `gcd16` | `[a:b]` = greatest common divisor of `[a:b]` and `[c:d]` | — |
| `isqrt16` | `[a:b]` = square root of `[a:b]` (rounded down), `[c:d]` = remainder | Z = perfect square |

Details:

* **`mul16`** never overflows: the product of two 16-bit numbers always fits
  in the 32 bits of `[a:b:c:d]` (`[a:b]` = high half, `[c:d]` = low half).
  If you only need 16 bits, use `[c:d]` and check that `[a:b]` is 0.
* **`div16`**: the quotient is rounded down. On division by zero, C is set
  and nothing changes.
* **`pow16`**: 0^0 = 1. On overflow, C is set and `[a:b]` holds the low 16
  bits of the true result.
* **`gcd16`**: gcd(n, 0) = gcd(0, n) = n, and gcd(0, 0) = 0.
* **`isqrt16`**: the root is 0 to 255, the remainder (`[a:b]` − root²)
  is 0 to 510.

```
    mov a, 0x04             ; [a:b] = 1234
    mov b, 0xD2
    mov c, 0
    mov d, 10               ; [c:d] = 10
    cal div16               ; [a:b] = 123, [c:d] = 4
```

---

## 5. Unsigned 32-bit arithmetic

Values from 0 to 4294967295, in memory. `[x:y]` points to the 1st operand,
which receives the result; `[i:j]` points to the 2nd operand.

| Routine | Does | Flags |
|---|---|---|
| `add32` | value at `[x:y]` += value at `[i:j]` | C = overflow, Z |
| `sub32` | value at `[x:y]` −= value at `[i:j]` | C = borrow (1st < 2nd), Z |
| `cmp32` | compares the values at `[x:y]` and `[i:j]`, like `cmp` | S = greater, E = equal, F = smaller |
| `inc32` | value at `[x:y]` + 1 | C = wrap-around, Z |
| `dec32` | value at `[x:y]` − 1 | C = wrap-around, Z |
| `min32` | value at `[x:y]` = the smaller of the two values | — |
| `max32` | value at `[x:y]` = the larger of the two values | — |
| `mul32` | value at `[x:y]` ×= value at `[i:j]` | C = overflow |
| `div32` | value at `[x:y]` /= value at `[i:j]`, and the value at `[i:j]` = remainder | C = division by zero |
| `pow32` | value at `[x:y]` = value ^ `c` (exponent 0 to 255) | C = overflow |
| `gcd32` | value at `[x:y]` = greatest common divisor of the two values | — |
| `isqrt32` | value at `[x:y]` = square root (rounded down), value at `[i:j]` = remainder | Z = perfect square |

Details:

* **`mul32`** keeps only 32 bits. On overflow, C is set and the low 32 bits
  of the true product are stored.
* **`div32` overwrites the divisor with the remainder.** If you still need
  the divisor, `copy32` it first. On division by zero, C is set and
  nothing changes.
* **`pow32`**: 0^0 = 1. On overflow, C is set and the low 32 bits are stored.
* **`gcd32`** does not change the value at `[i:j]`.
* **`isqrt32`**: the root is 0 to 65535, the remainder 0 to 131070.
  `[x:y]` and `[i:j]` must be **different addresses**.
* `inc32` / `dec32` usually change only the last byte: they stop at the
  first byte that does not wrap around.

```
.data32 price 1999
.data32 count 250

    mov x, price.msb
    mov y, price.lsb
    mov i, count.msb
    mov j, count.lsb
    cal mul32               ; price = 499750
    jmc too_big             ; C = the product needed more than 32 bits
```

---

## 6. Signed 16-bit arithmetic

Two's complement values from −32768 to 32767, in the same registers as the
unsigned routines.

| Routine | Does | Flags |
|---|---|---|
| `add16s` | `[a:b]` = `[a:b]` + `[c:d]` | C = signed overflow, Z |
| `sub16s` | `[a:b]` = `[a:b]` − `[c:d]` | C = signed overflow, Z |
| `cmp16s` | compares `[a:b]` with `[c:d]` (signed), like `cmp` | S = greater, E = equal, F = smaller |
| `mul16s` | `[a:b:c:d]` = `[a:b]` × `[c:d]` (full 32-bit signed product) | — |
| `div16s` | `[a:b]` = `[a:b]` / `[c:d]`, `[c:d]` = remainder | C = division by zero, or −32768 / −1 |
| `neg16` | `[a:b]` = −`[a:b]` | C = the input was −32768, Z |
| `abs16` | `[a:b]` = \|`[a:b]`\| | C = the input was −32768, Z |
| `min16s` | `[a:b]` = the smaller of `[a:b]` and `[c:d]` | — |
| `max16s` | `[a:b]` = the larger of `[a:b]` and `[c:d]` | — |
| `sext8` | `[a:b]` = `b` sign-extended to 16 bits | — |

Details:

* **Signed overflow** (`add16s`, `sub16s`): the true result is outside
  −32768..32767. `[a:b]` then holds the wrapped result.
* **`div16s`** works like C: the quotient is rounded toward 0 and the
  remainder has the sign of the dividend (−7 / 2 = −3, remainder −1).
  −32768 / −1 = 32768 does not fit: C is set and nothing changes, as for
  division by zero.
* **`neg16`**: −(−32768) does not fit; the value stays −32768 and C is set.
* **`abs16`**: \|−32768\| gives 0x8000, which is right if you read it as
  unsigned (32768); C is set to warn that it is not a valid signed value.
* **`sext8`** reads `b` and overwrites `a`: 0x80 → 0xFF80, 0x7F → 0x007F.

---

## 7. Signed 32-bit arithmetic

Two's complement values from −2147483648 to 2147483647, in memory, with the
same pointers as the unsigned routines.

| Routine | Does | Flags |
|---|---|---|
| `add32s` | value at `[x:y]` += value at `[i:j]` | C = signed overflow, Z |
| `sub32s` | value at `[x:y]` −= value at `[i:j]` | C = signed overflow, Z |
| `cmp32s` | compares the values at `[x:y]` and `[i:j]` (signed), like `cmp` | S = greater, E = equal, F = smaller |
| `mul32s` | value at `[x:y]` ×= value at `[i:j]` | C = overflow |
| `div32s` | value at `[x:y]` /= value at `[i:j]`, and the value at `[i:j]` = remainder | C = division by zero, or −2³¹ / −1 |
| `neg32` | value at `[x:y]` = −value | C = the input was −2147483648, Z |
| `abs32` | value at `[x:y]` = \|value\| | C = the input was −2147483648, Z |
| `min32s` | value at `[x:y]` = the smaller of the two values | — |
| `max32s` | value at `[x:y]` = the larger of the two values | — |
| `sext16` | value at `[x:y]` = `[a:b]` sign-extended to 32 bits | — |

Details:

* **`mul32s`**: overflow = the true product is outside −2³¹..2³¹−1. The low
  32 bits are stored.
* **`div32s`** follows the same rules as `div16s` (rounded toward 0,
  remainder with the sign of the dividend) and, like `div32`,
  **overwrites the divisor with the remainder**.
* **`sext16`**: 0x8000 → 0xFFFF8000. `[a:b]` is not changed.

---

## 8. Logic operations and shifts

### Bitwise logic

| Routine | Does | Flags |
|---|---|---|
| `and16` | `[a:b]` = `[a:b]` AND `[c:d]` | Z, N = bit 15, **C unchanged** |
| `or16` | `[a:b]` = `[a:b]` OR `[c:d]` | Z, N = bit 15, **C unchanged** |
| `xor16` | `[a:b]` = `[a:b]` XOR `[c:d]` | Z, N = bit 15, **C unchanged** |
| `not16` | `[a:b]` = NOT `[a:b]` | Z, N = bit 15, **C unchanged** |
| `and32` | value at `[x:y]` = value AND value at `[i:j]` | Z, N = bit 31 |
| `or32` | value at `[x:y]` = value OR value at `[i:j]` | Z, N = bit 31 |
| `xor32` | value at `[x:y]` = value XOR value at `[i:j]` | Z, N = bit 31 |
| `not32` | value at `[x:y]` = NOT value | Z, N = bit 31 |

Z and N describe the **whole** result. (Two plain instructions such as
`and a, c` / `and b, d` would leave Z and N about `b` only.)

### Shifts by 1 bit

| Routine | Does | Flags |
|---|---|---|
| `shl16` | `[a:b]` = `[a:b]` << 1 | C = bit shifted out |
| `shr16` | `[a:b]` = `[a:b]` >> 1, a 0 comes in at the top (unsigned) | C = bit shifted out |
| `asr16` | `[a:b]` = `[a:b]` >> 1, the sign bit is kept (signed) | C = bit shifted out |
| `shl32` | value at `[x:y]` << 1 | C = bit shifted out |
| `shr32` | value at `[x:y]` >> 1 (unsigned) | C = bit shifted out |
| `asr32` | value at `[x:y]` >> 1 (signed) | C = bit shifted out |

### Shifts by any number of bits

The number of bits is in **`c`** (0 to 255) and is not changed.

| Routine | Does | Flags |
|---|---|---|
| `shl16n` | `[a:b]` = `[a:b]` << `c` | C = last bit shifted out |
| `shr16n` | `[a:b]` = `[a:b]` >> `c` (unsigned) | C = last bit shifted out |
| `asr16n` | `[a:b]` = `[a:b]` >> `c` (signed) | C = last bit shifted out |
| `shl32n` | value at `[x:y]` << `c` | C = last bit shifted out |
| `shr32n` | value at `[x:y]` >> `c` (unsigned) | C = last bit shifted out |
| `asr32n` | value at `[x:y]` >> `c` (signed) | C = last bit shifted out |

* C is the same as after `c` shifts by 1 bit; it is 0 when `c` = 0.
* Shifting by the width (16 or 32) or more gives 0, or for `asr16n` /
  `asr32n` all copies of the sign bit (0 or −1).
* Shifts by a multiple of 8 are fast: they are byte moves.

```
    mov c, 4
    cal shr16n              ; [a:b] = [a:b] / 16 (unsigned)
```

---

## 9. Bit counting

| Routine | Counts | Value |
|---|---|---|
| `popcnt8` / `popcnt16` / `popcnt32` | the 1 bits | `a` / `[a:b]` / 4 bytes at `[x:y]` |
| `lzcnt8` / `lzcnt16` / `lzcnt32` | the 0 bits above the highest 1 (leading zeros) | same |
| `tzcnt8` / `tzcnt16` / `tzcnt32` | the 0 bits below the lowest 1 (trailing zeros) | same |

* The count is returned in **`a`**. Every other register is restored, so
  for the 16-bit versions `b` keeps its value, and the 32-bit value in
  memory is not changed.
* **Flags:** Z = the count is 0.
* A value of 0 gives the width: `lzcnt16(0)` = `tzcnt16(0)` = 16.

Example, with the 16-bit value 0x0B00 (0000 1011 0000 0000):
`popcnt16` = 3, `lzcnt16` = 4, `tzcnt16` = 8.

Uses:

* **Highest bit set** (integer log₂): 15 − `lzcnt16`, for a value that is
  not 0.
* **First free slot in a bitmap**: `tzcnt` of the inverted bitmap.
* **Parity**: bit 0 of `popcnt`.
* **Quick tests with Z alone**: after `popcnt`, Z = the value was 0; after
  `lzcnt`, Z = the top bit is set; after `tzcnt`, Z = bit 0 is set.

---

## 10. Random numbers

The generator is **xorshift32** (George Marsaglia, 2003). Its state is a
32-bit number, never 0; it goes through all 4294967295 non-zero values
before repeating. It is good for games and simulations, **not for
cryptography**.

### Seeding

| Routine | Does |
|---|---|
| `srand` | state = 32-bit value at `[x:y]` (0 is replaced by 1) |
| `srand_rtc` | seeds from the real-time clock; `d` = the RTC's first port |

* Without seeding, the state starts at a fixed value, so a program gets the
  same numbers every time it runs. That is useful for tests.
* The same seed always gives the same sequence.
* `srand_rtc` reads the day, hour, minute and second, so the sequence
  changes every second.

### Random numbers

| Routine | Does |
|---|---|
| `rand8` | `a` = random number, 0 to 255 |
| `rand16` | `[a:b]` = random number, 0 to 65535 |
| `rand32` | 32-bit value at `[x:y]` = random number |

### Random numbers in a range

| Routine | lo | hi | Result | Flags |
|---|---|---|---|---|
| `randr8` | `a` | `b` | `a` | C = `b` < `a` |
| `randr16` | `[a:b]` | `[c:d]` | `[a:b]` | C = `[c:d]` < `[a:b]` |
| `randr32` | value at `[x:y]` | value at `[i:j]` | value at `[x:y]` | C = hi < lo |

* The result is from lo to hi, **both included**, unsigned.
* Every value of the range is equally likely: there is no bias like with
  `rand % n`, and no division.
* The full range (0 to 255, 0 to 65535, 0 to 4294967295) works too.
* If hi < lo, C is set and nothing changes.

```
    mov a, 1
    mov b, 6
    cal randr8              ; a = die roll, 1 to 6
```

---

## 11. Number ↔ text

Text is ASCII and ends with a 0 byte, like `.data msg "Hi", 0`.

### Number → text

| Routine | Value | Writes at `[x:y]` |
|---|---|---|
| `utoa8` / `utoa16` / `utoa32` | `a` / `[a:b]` / `[i:j]` → 4 bytes | unsigned decimal: `"1234"` |
| `itoa8` / `itoa16` / `itoa32` | same | signed decimal: `"-1234"` |
| `htoa8` / `htoa16` / `htoa32` | same | hexadecimal, fixed width: `"04D2"` |

* **`[x:y]` moves forward**: after the call it points to the 0 at the end
  of the text. The next text can be written straight after it, which builds
  a line piece by piece. Save `[x:y]` first (`psh x, y`) if you need the
  start of the text again.
* Decimal text has no leading zeros (0 is `"0"`). Hexadecimal text always
  has 2, 4 or 8 digits, upper case, with no `0x`.
* **Buffer sizes**, with the final 0:

  | | 8-bit | 16-bit | 32-bit |
  |---|---|---|---|
  | `utoa` | 4 | 6 | 11 |
  | `itoa` | 5 | 7 | 12 |
  | `htoa` | 3 | 5 | 9 |

* Flags are changed. Every register except `[x:y]` is restored.

```
.data line 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

    mov x, line.msb
    mov y, line.lsb
    mov a, 0x04             ; [a:b] = 1234
    mov b, 0xD2
    cal utoa16              ; line = "1234", [x:y] -> its 0
```

### Text → number

| Routine | Reads at `[i:j]` | Writes at `[x:y]` | Flags |
|---|---|---|---|
| `atou` | decimal digits | 4-byte unsigned value | C = error |
| `atoi` | an optional `-` or `+`, then decimal digits | 4-byte signed value | C = error |

* Reading stops at the first character that is not a digit, and **`[i:j]`
  moves to it**. With `"12,34"`, `atou` gives 12 and leaves `[i:j]` on the
  comma, so you can skip it and call `atou` again for the 34.
* **Errors**: no digit at all (`""`, `"x"`, `"-"`), or a number that does
  not fit (above 4294967295 for `atou`, outside −2147483648..2147483647 for
  `atoi`). Then C is set and nothing changes: `[i:j]` stays where it was
  and the result is not written.
* Spaces are not skipped: `" 1"` is an error.
* Leading zeros are fine: `"0042"` gives 42.

---

## 12. Interrupts and work areas

The routines that need work space (`mul32`, `div32`, `pow32`, `gcd32`,
`isqrt32`, `mul32s`, `randr32`) keep it on the stack, so they are safe
anywhere, interrupt handlers included. The deepest, `gcd32`, uses 29 bytes of
stack (return address included).

Only the random routines keep a value in a fixed variable: `rand_s`, the
generator's state, shared by `srand`, `srand_rtc`, `rand8`, `rand16`,
`rand32`, `randr8`, `randr16` and `randr32`. If an interrupt handler calls
one of them while the main program is in the middle of another, the
sequence of numbers gets mixed up. Either don't call them from interrupt
handlers, or disable interrupts around the calls in the main program
(`cli` … `sti`).

---

## 13. Speed

Cycles for one call, `cal` and `ret` included, measured on the emulator.
They depend on the values: the ranges below go from small values to the
worst case.

| Routine | Cycles |
|---|---|
| `mul16` | 75 (7 × 1000) to 164 (65535 × 65535) |
| `div16` | 86 (100 / 7) to 190 (12345 / 1000) |
| `mul32` | 155 (7 × 100000) to 446 (max × max) |
| `div32` | 359 (1000 / 7) to 637 (123456789 / 100000) |
| `mul16s` / `div16s` | the unsigned routine + 32 to 81 |
| `mul32s` / `div32s` | the unsigned routine + 50 to 126 |
| `and16` … `not16` | 7 |
| `and32` … `xor32` / `not32` | 26 / 25 |
| `add32` / `sub32` / `copy32` / `neg32` | 22 / 22 / 12 / 25 |
| `shr16` / `asr16` | 8 / 6 |
| `shr32` / `asr32` | 33 / 31 |
| `shl16n` / `shr16n` / `asr16n` | up to 62 / 121 / 111 |
| `shl32n` / `shr32n` / `asr32n` | up to 115 / 193 / 183 |
| `popcnt8` / `16` / `32` | 25 / 44 / 83 |
| `lzcnt` / `tzcnt` | 14 to 42 (8-bit), 15 to 48 (16-bit), 14 to 55 (32-bit) |
| `utoa8` / `utoa16` / `utoa32` | up to 111 / 253 / 635 |
| `htoa8` / `htoa16` / `htoa32` | 36 / 62 / 115 |
| `atou` / `atoi` | up to 429 / 454 (10 digits) |
| `isqrt16` / `isqrt32` | 192 / about 490 |
| `randr8` / `randr16` / `randr32` | about 170 / 150 to 240 / 190 to 300, on average |

---

## 14. Files

| File | Routines |
|---|---|
| `math.has` | includes everything; `load16`, `store16`, `copy16`, `load32`, `store32`, `copy32` |
| `16/uint16_arithmetic.has` | `add16`, `sub16`, `cmp16`, `inc16`, `dec16`, `min16`, `max16`, `gcd16`, `isqrt16` |
| `16/uint16_mul_div.has` | `mul16`, `div16`, `pow16` |
| `16/int16_arithmetic.has` | `cmp16s`, `add16s`, `sub16s` |
| `16/int16_mul_div.has` | `mul16s`, `div16s` |
| `16/int16_logic_ops.has` | `and16`, `or16`, `xor16`, `not16`, `shl16`, `shr16`, `asr16`, `shl16n`, `shr16n`, `asr16n` |
| `16/int16_misc.has` | `neg16`, `abs16`, `min16s`, `max16s`, `sext8` |
| `32/uint32_arithmetic.has` | `add32`, `sub32`, `cmp32`, `inc32`, `dec32`, `clr32`, `tst32`, `min32`, `max32`, `gcd32`, `isqrt32` |
| `32/uint32_mul_div.has` | `mul32`, `div32`, `pow32` |
| `32/int32_arithmetic.has` | `cmp32s`, `add32s`, `sub32s` |
| `32/int32_mul_div.has` | `mul32s`, `div32s` |
| `32/int32_logic_ops.has` | `and32`, `or32`, `xor32`, `not32`, `shl32`, `shr32`, `asr32`, `shl32n`, `shr32n`, `asr32n` |
| `32/int32_misc.has` | `neg32`, `abs32`, `min32s`, `max32s`, `sext16` |
| `misc/bit_counting.has` | `popcnt8/16/32`, `lzcnt8/16/32`, `tzcnt8/16/32` |
| `misc/random.has` | `srand`, `srand_rtc`, `rand8`, `rand16`, `rand32` |
| `misc/random_range.has` | `randr8`, `randr16`, `randr32` |
| `misc/text.has` | `utoa8/16/32`, `itoa8/16/32`, `htoa8/16/32`, `atou`, `atoi` |

The files call each other (for example `div16s` calls `div16`), so include
`math.has` rather than single files.
