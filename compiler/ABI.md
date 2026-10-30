# HBC-2 C ABI

How C code compiled for the HBC-2 uses memory, registers and the stack, and how
it works together with hand-written assembly. The compiler is the Fuzix
Compiler Kit with an HBC-2 backend (see `ROADMAP.md`).

The assembly examples in sections 5, 9 and 10 were run on the emulator.
Section 11 describes the runtime helpers, in `compiler/runtime`.

## 1. Types

| C type | Size | Notes |
|---|---|---|
| `char`, `unsigned char` | 1 | `char` is **unsigned** |
| `signed char` | 1 | |
| `short`, `int`, `unsigned` | 2 | |
| `long`, `unsigned long` | 4 | |
| pointers | 2 | function pointers too |
| `enum` | 2 | same as `int` |
| `float`, `double` | — | not supported: the compiler stops with an error |

* Every value is stored **big-endian**: the most significant byte at the
  lowest address, like the assembler's `.data16` / `.data32` and the return
  address pushed by `cal`.
* No alignment: struct members and variables can start at any address, and
  structs have no padding.
* Signed values are in two's complement.

## 2. Memory map

| Address | Content |
|---|---|
| `$0x0000` | `_start`, the startup code (section 8) |
| after it | code and `.data` (initialized variables, strings), placed by the assembler |
| after that | the `.bss` area: variables without an initial value, cleared at startup |
| up to `$0xFDFF` | the stack, growing down from `$0xFE00` |
| `$0xFE00`-`$0xFFFF` | interrupt vector table |

Nothing stops the stack from growing into the `.bss` area.

## 3. Registers

| Register | Role in compiled code |
|---|---|
| `[a:b]` | working value: `int` and pointers (a = most significant byte); `char` in `b` |
| `[c:d]` | second operand; high half of a `long` (`long` = `[c:d]:[a:b]`, c = most significant byte) |
| `[x:y]` | pointer register: `*p` and `p->field` become `[x:y + n]` |
| `i`, `j` | scratch, or a second pointer `[i:j]` |
| SP | stack pointer |

* A function may change **every register except SP**: the caller saves what
  it still needs.
* SP is the same before the call and after it returns (once the caller has
  removed the arguments).
* Flags are not kept and carry no result.
* When a `char` is returned or loaded, only `b` counts: `a` can hold anything.
* There are no register variables: `register` is accepted and ignored.

## 4. Names

| Thing | Name in assembly | Example |
|---|---|---|
| C function or global variable | `_` + C name | `main` → `_main` |
| `static` function or variable | `__S<unit>_` + C name | `static int count;` in the 2nd file → `__S2_count` |
| compiler label | `__L<unit>_<n><tail>` | `__L1_12_r` |
| string literal | `__T<unit>_<n>` | `__T1_3` |
| switch table | `__W<unit>_<n>` | `__W1_1` |
| runtime helper | `__` + name | `__mul`, `__cceq` |
| startup code | `_start`, `__exit` | |
| `.bss` bounds (assembler) | `__bss_start`, `__bss_end` | |

* `<unit>` is the number of the C file in the build (1, 2, …), so that labels
  and statics of different files don't clash when the files are assembled
  together.
* C keeps 31 characters of a name; longer names are cut.
* C names can't be `start` (it would be `_start`, the program entry).
* C reserves names starting with `__`, so compiler and runtime names never
  clash with C names.
* **Assembly names starting with `_` share the namespace of C names**:
  `_loop` in assembly and a C function `loop` are the same name. The assembler
  reports it ("already defined"), so a clash can't go unnoticed. Assembly that
  is not meant to be seen from C should use names without a leading `_`.
* For C to use an assembly function or variable, it needs the `_` name:
  `extern unsigned char monitor_port;` reads `_monitor_port`. An assembly
  variable named `monitorPort` is not visible from C.

## 5. Calls

### The caller

1. Pushes the arguments **from the last one to the first**. Each argument
   takes its own size (a `char` is one byte) and is pushed **low byte first**,
   so that it is big-endian in memory. In a variadic call (`printf`), C's
   promotions apply: a `char` is passed as an `int`.
2. `cal _function` (or `cal [x:y]` for a function pointer). This pushes the
   return address.
3. After the return: `asp n` removes the `n` bytes of arguments.

| Value in | Push it with |
|---|---|
| `b` (`char`) | `psh b` |
| `[a:b]` (`int`, pointer) | `psh b, a` |
| `[c:d]:[a:b]` (`long`) | `psh b, a, d, c` |

### The function

```
_f:
    ssp L               ; reserve L bytes of locals (omitted when L = 0)
    ...                 ; locals at [sp + 0] .. [sp + L - 1]
                        ; return address at [sp + L], [sp + L + 1]
                        ; first argument from [sp + L + 2]
    asp L               ; free the locals
    ret                 ; result in b, [a:b] or [c:d]:[a:b]
```

There is no frame pointer. When the function pushes temporary values, every
offset grows by the bytes pushed; the compiler keeps track of it.

| Return type | Result in |
|---|---|
| `char` | `b` |
| `int`, pointer | `[a:b]` |
| `long` | `[c:d]:[a:b]` |
| `void` | nothing |

Structures can't be passed or returned by value (the compiler refuses it): use
pointers.

### Example

`int add3(int a, char b, long c)` returning `a + b +` the low 16 bits of `c`,
with 2 bytes of locals, called as `add3(1000, 7, 0x12345)`:

```
    mov c, 0x00                 ; [c:d]:[a:b] = 0x00012345 (third argument)
    mov d, 0x01
    mov a, 0x23
    mov b, 0x45
    psh b, a, d, c              ; last argument first, low byte first
    mov b, 7
    psh b                       ; char: one byte
    mov a, 0x03                 ; 1000 = 0x03E8
    mov b, 0xE8
    psh b, a
    cal _add3
    asp 7                       ; the caller removes the 7 bytes of arguments
                                ; [a:b] = 10036 = 0x2734

_add3:
    ssp 2                       ; locals: [sp], [sp + 1]
                                ; [sp + 2], [sp + 3] = return address
                                ; a = [sp + 4] (MSB), [sp + 5]
                                ; b = [sp + 6]
                                ; c = [sp + 7] (MSB) .. [sp + 10]
    lod a, [sp + 4]
    lod b, [sp + 5]
    add b, [sp + 6]
    adc a, 0
    add b, [sp + 10]
    adc a, [sp + 9]
    asp 2
    ret
```

### Locals and arguments further than 255 bytes

`[sp + n]` reaches 255 bytes. Further, the compiler computes the address:

```
    rsp [x:y]                   ; [x:y] = SP
    add x, n >> 8               ; high part of the offset
    lod a, [x:y + (n & 255)]    ; the CPU adds the low part on 16 bits
```

The address of a local (`&local`) is computed the same way, with a 16-bit add:
`rsp [x:y]`, `add y, n`, `adc x, 0`.

## 6. Variables in assembly

| C | Assembly |
|---|---|
| `int count = 5;` | `.data _count 0x00, 0x05` |
| `char table[] = {1, 2};` | `.data _table 1, 2` |
| `int total;` (no initial value) | `.bss _total 2` |
| `"Hello"` | `.data __T1_4 "Hello", 0` |

Each C object is **one** `.data` line, so that its bytes stay together.
Constants (`const`) are ordinary variables: everything is in RAM.

## 7. Building several files

A program is a list of C files (`.c`) and assembly files (`.has`)
(`hbc2_cc::build_program`). Each C file is compiled to its own `.has` file,
with its unit number (1, 2, … in the order of the list). One `main.has`
then includes, in this order: the startup code, the runtime, the C library
(`compiler/lib`, section 13) and the files of the list (the assembly files
as they are), and is assembled as one program. Unused functions are kept:
a linker that removes them comes later.

* **`static` names.** The compiler writes a `; export _name` line before
  each name that is not `static`. After compiling a file, the driver
  renames every `_name` the file defines (label, `.data`, `.bss`) without
  that line to `__S<unit>_name`, everywhere in the file (not in strings or
  comments). So each file keeps its own `static` names, and a `static`
  name can't be reached from another file. `static` variables inside a
  function are `__T<unit>_<n>` labels already.
* **`#include "x.h"`** looks in the folder of the C file first, then in the
  library headers.
* **Errors.** A name defined twice (two files, or a file and the library)
  and a name used but defined nowhere are reported by the assembler; the
  driver gives the C file instead of its compiled file, and the C name:
  `b.c: error: 'n' is already defined (a.c)`,
  `main.c: error: not defined: 'f'`. Two `int n;` without a value in two
  files are a duplicate too (no "common" variables).
* Defining a library function in the program (`puts`) is a duplicate for
  now (the linker could let the program's function replace the library's).

## 8. Startup and exit

The startup code is at `$0x0000` (`compiler/runtime/crt0.has`):

```
_start:
    wsp $0xFE00                 ; stack (the reset value, set again for a restart)
    ...                         ; clears the .bss area (__bss_start to __bss_end)
    ...                         ; waits for the devices to be detected
    cal _main
__exit:                         ; [a:b] = main's return value, or exit's argument
    hlt
    jmp __exit
```

* Interrupts stay as the reset leaves them: enabled.
* **Device detection:** `crt0.has` includes `devices.has`, a copy of the
  port-0 handler of the project template, with its vector at `$0xFE00`. It
  stores each device's first port in `monitor_port`, `rtc_port`,
  `keyboard_port`, `eeprom_port` (`hbc2.h`; 0 = not plugged), and points the
  RTC and keyboard vectors to a handler that does nothing. The interrupts of
  the devices plugged at power-on are already waiting in the IOD queue: the
  CPU takes them between the first instructions, so `crt0` only waits a few
  loop turns (one was enough in tests; it waits 32) before calling `main`.
  A C program therefore must not set the port-0 vector itself (no
  `interruptHandler.has` of the project template).
* Other interrupt vectors come from the project's assembly files (fixed
  `.data16` at `$0xFE00 + 2n`, or written at run time).
* `hlt` waits for an interrupt (it enables them), so `__exit` loops: once
  the program has finished, only the interrupt handlers run.
* `exit(n)` (`stdlib.h`) puts `n` in `[a:b]`, resets SP and jumps to
  `__exit`. Tests read the result there.

## 9. Assembly functions for C (wrappers)

The assembly libraries take their values in registers. A wrapper with the `_`
name reads the C arguments from the stack, calls the routine, and turns its
result into a C value:

```
; int screen_putc(char c, unsigned char x, unsigned char y, unsigned char colors)
; returns 0, or 1 when nothing was drawn
_screen_putc:
    lod a, [sp + 2]             ; c (first argument)
    lod x, [sp + 3]             ; x
    lod y, [sp + 4]             ; y
    lod b, [sp + 5]             ; colors
    cal screen_putc             ; C = 1 when nothing was drawn
    mov a, 0                    ; mov does not change the flags
    mov b, 0
    jmc screen_putc_c_failed
    ret
screen_putc_c_failed:
    mov b, 1
    ret
```

The wrapper has no locals, so the first argument is at `[sp + 2]`, just above
the return address. It may change every register. Its inner label doesn't
start with `_`, so it can't clash with a C name.

## 10. Interrupt handlers calling C

Interrupt handlers are written in assembly. When an interrupt starts, the CPU
saves only `i` (which then holds the data byte) and the return address. C code
changes every register and the flags, so a handler that calls C saves them
all:

```
tick:
    psf                         ; flags
    psh a, b, c, d, j, x, y     ; i is restored by irt
    psh i                       ; argument: the data byte
    cal _on_tick                ; void on_tick(unsigned char data)
    asp 1
    pop a, b, c, d, j, x, y
    pof
    irt
```

The runtime helpers keep no state in global variables, so compiled code can
be interrupted anywhere. C variables shared with a handler should be
`volatile`, although the compiler mostly ignores the keyword.

## 11. Runtime helpers

Most operations are compiled inline: + - & | ^ ~ on every size, all the
comparisons, shifts by a constant, multiplies by small constants, unsigned
/ and % by a power of two, and ++ -- += -= &= |= ^= on variables. The
others become calls to helpers named
`__<operation><type>`: the type suffix is none for `int`, `u` for
`unsigned`, `c`/`uc` for `char`, `l`/`ul` for `long` (`__mul`, `__divu`,
`__cclt`, `__pluseql`, …). Before the call, the caller pops the left operand
into registers. The right operand is in the working register.

| Left size | Right size | Left operand in |
|---|---|---|
| 1 | any | `d` |
| 2 | 1 or 2 | `[c:d]` |
| 2 | 4 | `[x:y]` |
| 4 | any | `[x:y]:[i:j]` |

* For assignment operators (`+=`, `++`, `<<=`, …) the left operand is the
  address of the variable: `[c:d]`, or `[x:y]` when the value is a `long`.
* The result is in the working register: `b`, `[a:b]` or `[c:d]:[a:b]`.
* Comparisons give 0 or 1 in `[a:b]` (the compiler no longer calls them: it
  compares inline; they stay for assembly code).
* Helpers may change every register; they keep SP, and they use only
  registers and the stack (no global variables).
* Signed division rounds toward zero, the remainder has the sign of the
  left operand. Division by zero gives all bits set (the remainder: the
  left operand).
* Shifts use the low byte of the count; a count of the width or more gives
  0 (or -1 for a negative value shifted right).
* The helpers are written for this convention (they don't keep registers,
  unlike the math library), in `compiler/runtime`: `compare.has`,
  `long.has`, `mul.has`, `div.has`, `shift.has`, `assign.has`,
  `switch.has`, all included by `runtime.has` (about 3.9 KB).

`switch` jumps to `__switch<type>` with the value in the working register and
`[x:y]` = the address of the table: the number of cases (2 bytes), then for
each case its value (the size of the type) and the address of its code, then
the address of the `default` code (or of the end of the switch).

## 12. C supported by the compiler

The Fuzix Compiler Kit compiles C89, without:

* `float` and `double` (HBC-2 choice),
* bitfields,
* structures passed or returned by value,
* K&R function definitions (`int f(a) int a; { … }`).

Other limits:

* Local variables have function-wide scope, not block scope.
* `const` and `volatile` are accepted but mostly ignored.
* There are at most 32 parameters per function, 128 `case` labels per
  `switch`, 30 members per struct and 50 `enum` constants.
* The constant `-32768` can get the wrong type.
* A decimal constant from 32768 to 65535 is an `unsigned int` (standard C
  makes it a `long`): `x / 40000` is an unsigned division. Write `40000L`
  for a `long`.
* A cast to `(void)` to drop a result is refused.

## 13. The C library

`compiler/lib`: headers in `include` (always on the include path), the
assembly in `lib.has`, C functions in `src`. See `compiler/lib/README.md`.
Library names follow section 4: C functions are `_name`; the inner labels
of the assembly start with `lib_`, `screen_`, `term_` or `devices_`:
they don't start with `_`, so they are not C names and can't clash with
the program's names.
