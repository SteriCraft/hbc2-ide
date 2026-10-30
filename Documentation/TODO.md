# Documentation to-do

## HBC-2 Monitor Instruction Manual

- **Displayable characters, "Colored text mode" and "ASCII Table" sections:** the
  manual says ASCII codes "32 to 127 (included)" are displayable. That is 96 codes,
  but the manual and `Peripherals/Screen/Specifications.txt` both say 95 characters,
  and 127 is DEL (not displayable) in `Peripherals/Screen/ASCII table.png`.
  Should read: **32 to 126** (0x20 space to 0x7E `~`).
- **Character list in the "ASCII Table" section** (also in `Specifications.txt`): the
  character after `_` is written `'` (apostrophe, 39), but code 96 is the backtick
  `` ` ``.
