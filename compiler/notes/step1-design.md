# Design points found in step 1

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

- **Data:** FCC emits a data label then one value at a time (`gen_value` per byte/word/long, `gen_name` = address `name+off`, `gen_text_data` = string literal address, `gen_space` = zeros). hbc2-asm needs ONE `.data` line per object (floating `.data` lines are not guaranteed contiguous). → backend buffers values and prints `.data _name v, v, …` when the object ends. Words → two bytes; `name+off` addresses need a byte form: `.data` only takes `name.msb/.lsb` WITHOUT offset → small assembler addition needed (e.g. allow `name + n` addresses inside `.data` as 2 bytes, or `(name + n).msb`). Decide in step 3 (assembler convenience, not ISA).
- **BSS segment** → `.bss _name size` (gen_space in A_BSS).
- **Labels:** FCC labels `L<n><tail>` (tails `_r` return, `_l` loop, `_b` break, `_c` continue, `_n` …), literals `T<n>`, switch tables `Sw<n>`; numbers restart in every file → print `__L<unit>_<n><tail>`, `__T<unit>_<n>`, `__W<unit>_<n>`; unit number given to cc2 as an extra argument (argv[5], the "codeseg" slot, is free for our backend).
- **Statics:** cc2 prints `_s` for statics, just without `.export` (references don't say static/global). → backend prints a marker for exported names (e.g. `; export _g`) and the DRIVER renames non-exported `_name` symbols of that file to `__S<unit>_name` (done, step 9).
- **Arguments:** FCC's T_ARGUMENT offsets start at 0 for the first argument; `target_argsize` = real size (chars passed as 1 byte — fits `psh`).
- **Helpers:** default backend emits `call __enter`, `__loadl`, `__deref`, `__plus`, `__castuc_`, `__cleanup`… → names `__<op><type suffix>`. Our gen_helpcall prints `cal __`.

