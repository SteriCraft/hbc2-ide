# Real-Time Clock (RTC) reference

Keeps the date and time, and interrupts the CPU once per second.

| | |
|---|---|
| ID | `0x95` |
| Ports | 7 |
| Interrupts | once per second, on its first port (`YEAR`) |
| Time | the host computer's local time (its time zone, daylight saving time included) |
| Range | 1900-01-01 00:00:00 to 2155-12-31 23:59:59, then back to 1900 |

See the [peripherals overview](../README.md) for ports, interrupts and how
to find the RTC's first port (`rtcPort` in the project template).

## Ports

| Offset | Name | Values |
|---|---|---|
| 0 | `YEAR` | Years since 1900: 0 = 1900, 126 = 2026, 255 = 2155 |
| 1 | `MONTH` | 1-12 |
| 2 | `DAY` | 1-31 |
| 3 | `HOUR` | 0-23 |
| 4 | `MINUTE` | 0-59 |
| 5 | `SECOND` | 0-59 |
| 6 | `CMD` | Writing it runs a command |

The date and time ports are refreshed **every second**, whether or not the
interrupt is sent, and by `GET_TIME`. Between two refreshes they keep what
was last put there, including values the program wrote itself. Right after
power-on they hold 0 until the first refresh: use `GET_TIME` to read the time
at once.

## Commands

| Value | Name | What it does |
|---|---|---|
| 0 | `NOP` | Nothing |
| 1 | `GET_TIME` | Puts the current date and time on ports 0-5 |
| 2 | `SET_TIME` | Sets the clock from ports 0-5; `YEAR` then holds the result |
| other | | Nothing |

A command runs each time the CPU writes `CMD`. After `GET_TIME` and
`SET_TIME`, `CMD` goes back to `NOP` (0); another value stays as written.

### SET_TIME

1. Write the date and time on ports 0-5 (`YEAR` as years since 1900).
2. Write 2 on `CMD`.
3. Read `YEAR`: **1** = the clock was set, **0** = the date or time was
   invalid and the clock did not change.

A date is valid when the month is 1-12, the day exists in that month (leap
years included: February 29 only in leap years), the hour is 0-23 and the
minutes and seconds are 0-59.

The clock does not stop following the host: `SET_TIME` keeps the difference
between the time set and the host's time, and the clock keeps counting from
there. Power-on goes back to the host's time.

## The interrupt

Every second, the RTC puts the current date and time on ports 0-5, then
sends an interrupt on its first port. The interrupt's data (in `I` in the
handler) is the `YEAR` value; the handler reads the other ports with `in`.

The project template points this vector to a handler that does nothing, so
the RTC can be left alone. To use it, install a handler on `rtcPort` (see
[Installing a handler](../README.md#installing-a-handler)).

In the IDE, the interrupt is only sent while the program **runs at 100 Hz or
more**: not while paused, stepping, or running at 1 or 10 Hz, where one
interrupt per second would arrive faster than the CPU could handle it. The
date and time ports still change every second in those cases.

The command line (`hbc2 run`) does not follow the host's clock: there the
RTC stays at 2000-01-01 00:00:00 (plus any `SET_TIME`) and sends no
interrupt.

## Examples

These examples use `rtcPort` from the project template.

### Read the time

```
; a = hour, b = minute, c = second.
    lod d, rtcPort
    add d, 6                ; d = CMD
    mov a, 1                ; GET_TIME
    out d, a
    sub d, 3                ; d = HOUR
    in a, d
    inc d                   ; d = MINUTE
    in b, d
    inc d                   ; d = SECOND
    in c, d
```

### Set the time

```
; Sets the clock to 2026-10-30 14:05:00. Carry set if refused.
set_clock:
    lod d, rtcPort          ; d = YEAR
    mov a, 126              ; 2026 - 1900
    out d, a
    inc d                   ; d = MONTH
    mov a, 10
    out d, a
    inc d                   ; d = DAY
    mov a, 30
    out d, a
    inc d                   ; d = HOUR
    mov a, 14
    out d, a
    inc d                   ; d = MINUTE
    mov a, 5
    out d, a
    inc d                   ; d = SECOND
    mov a, 0
    out d, a
    inc d                   ; d = CMD
    mov a, 2                ; SET_TIME
    out d, a
    lod d, rtcPort
    in a, d                 ; a = 1 if set, 0 if refused
    clc
    cmp a, 1
    jme set_clock_done
    stc
set_clock_done:
    ret
```

### Count the seconds

A handler that copies the `SECOND` port into a variable, and the code that
installs it.

```
.data seconds 0

rtc_tick:                   ; once per second
    psf
    psh a, b
    lod b, rtcPort
    add b, 5                ; b = SECOND
    in a, b
    str seconds, a
    pop a, b
    pof
    irt

rtc_install:
    psh a, x, y
    psf
    cli                     ; no interrupt while the vector is half written
    lod a, rtcPort
    mov x, 0xFE             ; [x:y] = $0xFE00 + 2 * rtcPort
    mov y, a
    shl y
    jmc rtc_install_high
    jmp rtc_install_write
rtc_install_high:
    inc x                   ; port 128 or more: $0xFFxx
rtc_install_write:
    mov a, rtc_tick.msb
    str [x:y], a
    mov a, rtc_tick.lsb
    str [x:y + 1], a
    pof
    pop a, x, y
    ret
```
