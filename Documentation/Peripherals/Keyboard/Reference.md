# Keyboard reference

An AZERTY keyboard that interrupts the CPU each time a key is pressed or
released.

| | |
|---|---|
| ID | `0xAA` |
| Ports | 2 |
| Interrupts | key pressed (first port), key released (second port) |
| Commands | none |

See the [peripherals overview](../README.md) for ports, interrupts and how
to find the keyboard's first port (`keyboardPort` in the project template).

## Ports

| Offset | Name | Holds |
|---|---|---|
| 0 | `PRESSED_SCAN_CODE` | Scan code of the last key pressed |
| 1 | `RELEASED_SCAN_CODE` | Scan code of the last key released |

Writing these ports does nothing.

## Interrupts

* **Key pressed:** the keyboard puts the key's scan code on its first port and
  sends an interrupt on that port. The handler finds the scan code in `I`.
* **Key released:** the same, on the second port.
* **Held key:** the pressed interrupt repeats at the host's key-repeat rate,
  then one released interrupt comes when the key is let go.

Each event is one interrupt in the IOD queue. If the queue is full, the
event is lost. A program that reads keys needs a handler on **both** ports;
an interrupt on a port whose vector was never set restarts the program (see
[Interrupts](../README.md#interrupts)).

A scan code names a **physical key**, not a character: the program decides
what the key types, for example from the state of Shift. Shift, Ctrl and Alt
send their own pressed and released events.

## Scan codes

The value of a scan code is the ASCII code of what the key shows on an
AZERTY keyboard (unshifted, letters in upper case), or a control code for the
other keys.

### Letters

The 26 letter keys send `0x41` (`A`) to `0x5A` (`Z`), by their AZERTY
position: the key left of `Z` on the top row is `A` (`0x41`).

### Main keys

| Code | Key (unshifted / shifted) |
|---|---|
| `0x20` | Space |
| `0x21` | `!` / `§` |
| `0x22` | `"` / `3` |
| `0x24` | `$` / `£` |
| `0x25` | `ù` / `%` |
| `0x26` | `&` / `1` |
| `0x27` | `'` / `4` |
| `0x28` | `(` / `5` |
| `0x29` | `)` / `°` |
| `0x2A` | `*` / `µ` |
| `0x2B` | `=` / `+` |
| `0x2C` | `,` / `?` |
| `0x2D` | `-` / `6` |
| `0x2E` | `;` / `.` |
| `0x3A` | `:` / `/` |
| `0x3C` | `<` / `>` |
| `0x40` | `à` / `0` |
| `0x5B` | `ç` / `9` |
| `0x5E` | `^` / `¨` |
| `0x5F` | `_` / `8` |
| `0x60` | `è` / `7` |
| `0x7E` | `é` / `2` |

### Keypad

| Code | Key |
|---|---|
| `0x30`-`0x39` | `0` to `9` |
| `0x12` | Enter |
| `0x13` | `+` |
| `0x14` | `-` |
| `0x15` | `*` |
| `0x16` | `/` |
| `0x17` | `.` |

### Control keys

| Code | Key |
|---|---|
| `0x01` | Insert |
| `0x02` | Home |
| `0x03` | End |
| `0x04` | Page Up |
| `0x05` | Page Down |
| `0x06` | Pause |
| `0x07` | Left arrow |
| `0x08` | Backspace |
| `0x09` | Tab |
| `0x0A` | Up arrow |
| `0x0B` | Right arrow |
| `0x0C` | Down arrow |
| `0x0D` | Enter |
| `0x0E` | Shift (left or right) |
| `0x0F` | Caps Lock |
| `0x10` | Ctrl (left or right) |
| `0x11` | Alt (left or right, AltGr included) |
| `0x1B` | Escape |
| `0x7F` | Delete |

No other code is ever sent.

## Example

Two handlers that keep the state of Shift and remember the last key pressed,
then the code that installs them (it uses `keyboardPort` from the project
template).

```
.data shift_down 0          ; 1 while Shift is held
.data last_key 0            ; scan code of the last key pressed (0 = none)

key_pressed:
    psf
    psh a
    cmp i, 0x0E             ; Shift?
    jme key_pressed_shift
    str last_key, i
    jmp key_pressed_end
key_pressed_shift:
    mov a, 1
    str shift_down, a
key_pressed_end:
    pop a
    pof
    irt

key_released:
    psf
    psh a
    cmp i, 0x0E
    jme key_released_shift
    jmp key_released_end
key_released_shift:
    mov a, 0
    str shift_down, a
key_released_end:
    pop a
    pof
    irt

; Points the keyboard's two vectors to the handlers above.
keyboard_install:
    psh a, x, y
    psf
    cli                     ; no interrupt while a vector is half written
    lod a, keyboardPort
    mov x, 0xFE             ; [x:y] = $0xFE00 + 2 * keyboardPort
    mov y, a
    shl y
    jmc keyboard_install_high
    jmp keyboard_install_write
keyboard_install_high:
    inc x                   ; port 128 or more: $0xFFxx
keyboard_install_write:
    mov a, key_pressed.msb
    str [x:y], a
    mov a, key_pressed.lsb
    str [x:y + 1], a
    mov a, key_released.msb
    str [x:y + 2], a
    mov a, key_released.lsb
    str [x:y + 3], a
    pof
    pop a, x, y
    ret
```

`[x:y + 2]` and `[x:y + 3]` reach the second port's vector even when it
crosses into the next page: the CPU adds the offset on 16 bits.

## In the IDE

Click the **Monitor** view to type on the HBC-2 keyboard; click elsewhere to
stop. Keys are sent by their position on the host keyboard, as if it were
AZERTY. With a non-AZERTY layout, the keys still send the codes of the
AZERTY key at the same place.

* A host key that types `é`, `è`, `à` or `ù` is sent as the `E`, `E`, `A` or
  `U` letter key instead, so French typed on the host reads well.
* Caps Lock on the host is sent as Shift held down (pressed when Caps Lock
  turns on, released when it turns off).
* The main-row keys that type a digit, `-` or `/` without Shift are sent as
  the keypad keys.
* The host cannot report Pause (`0x06`), Caps Lock (`0x0F`), keypad Enter
  (`0x12`), keypad `*` (`0x15`) and keypad `.` (`0x17`): the IDE never sends
  them.
* When the Monitor view loses focus, every key still held is released.
