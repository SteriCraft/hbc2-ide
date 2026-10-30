# Pipeline (from upstream cc.c)

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

`cpp -E [-I…] [-D…] f.c > f.%` → `cc0 f.sym < f.% > f.@` → `cc1 <cpucode> <features> < f.@ > f.#` → `cc2 f.sym <cpucode> <opt 0/1/2/s> <features> [codeseg] < f.# > f.^` → `copt rules < f.^ > f.s` (with opt 0: cc2 writes f.s, no copt).
- cc1 reads back its own stdout (lseek/read on fd 1): the output file must be opened read+write (Rust: `OpenOptions::new().read(true).write(true).create(true).truncate(true)`).
- Trial driver used in step 1: python, `subprocess.run(..., stdin=open(in,'rb'), stdout=open(out,'w+b'))`.

