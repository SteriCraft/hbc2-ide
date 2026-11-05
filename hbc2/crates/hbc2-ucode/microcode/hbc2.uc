# HBC-2 microcode (control unit, logisim/notes/cu-decisions.md).
#
# One line = one clock step (500 ns): a list of actions, separated by commas,
# all done in that step. `hbc2 ucode` compiles this file into the 7 AT28C256
# images, a listing and the cycle-count table (logisim/notes/cu-decisions.md
# section 13 has the design; this header is the language's reference):
#
#   bus<-X        X drives bus_data: ram, a (bus A), res (result latch), flags,
#                 pchi, pclo, sphi, splo, io (the IOD, IN)
#   ram<-bus      RAM write (data from bus_data); ram<-rom: boot copy
#   io<-bus       OUT (with bus<-a)
#   ir0<-bus ir1<-bus v1<-bus v2<-bus   load an IR byte from bus_data
#   swi<-bus      load the pending software interrupt port latch (sets P)
#   a=X           bus A: r1 r2 r3 (IR fields), i, reg0..reg7 (fixed index);
#                 default: the A operand latch
#   b=X           bus B: r2 r1 r3, one (01, INC/DEC), zero (00); default: the
#                 B operand latch
#   X<-bus        write register X (r1 r2 r3 i reg0..reg7) from bus_data
#   alu=OP        add sub adc sbb and or xor not shl shr asr
#   flags<-G      store flags from the ALU (czn, zn, sef) or bus_data (bus)
#   C<-0 ... H<-1 single flag write (C E I N S Z F H)
#   lat<-E pc<-E sp<-E   load the address latch / PC / SP with the adder's
#                 sum E (one sum per step): pc pc+1 pc-1 sp sp+1 sp-1 sp+v1
#                 sp-v1 sp+vx sp-vx a:b a:b+v2 0:b vx ivt ivt.hi 0
#   io            the latch shows the I/O port (IE = 1): implied by bus<-io
#                 and io<-bus
#   hwvec swvec   vector step of a hardware / software interrupt
#   inr           INR = 1
#
# How a step ends (default: the next line):
#   next L        go to line L
#   if C L1 else L0   C = jc (jump condition), noint (no interrupt, not
#                 halted), hwq (hardware interrupt sampled), xn (no software
#                 interrupt, no adder carry, not in reset)
#   end           instruction boundary: the next fetch, or an interrupt / halt
#   end nocheck   the next fetch, without the interrupt check
#   end halt      the halt loop (W1)
#   dispatch      (F0 only) step 1 of the instruction just fetched
#   restart       (S1 only) the boot row
#
# A bare name is a macro (below). $name is a per-opcode parameter (param).
# F0 (the fetch of byte 0) is the same for every instruction: the blocks
# start with the step after it.

macro f1  = bus<-ram, ir1<-bus, lat<-pc+1, pc<-pc+1
macro fv1 = bus<-ram, v1<-bus, lat<-pc+1, pc<-pc+1
macro fv2 = bus<-ram, v2<-bus, lat<-pc+1, pc<-pc+1

param alu add=add adc=adc sub=sub sbb=sbb and=and or=or xor=xor inc=add dec=sub not=not shl=shl asr=asr shr=shr
param fl  add=czn adc=czn sub=czn sbb=czn and=zn or=zn xor=zn not=zn shl=czn asr=czn shr=zn
param f   clc=C cle=E cln=N cls=S clz=Z clf=F stc=C ste=E stn=N sts=S stz=Z stf=F
param sp8  asp=sp+v1 ssp=sp-v1
param sp16 asp=sp+vx ssp=sp-vx

# ----- Interrupt entry, halt loop, fetch: the forced row (00, 0)

seq forced
F0:   bus<-ram, ir0<-bus, lat<-pc+1, pc<-pc+1, dispatch
W1:   if hwq IE1h else W2
W2:   if xn W1 else IE1s
IE1h: lat<-sp-1, sp<-sp-1, I<-0
IE2h: a=i, bus<-a, ram<-bus, lat<-sp-1, sp<-sp-1, H<-0
IE3h: bus<-pclo, ram<-bus, lat<-sp-1, sp<-sp-1
IE4h: bus<-pchi, ram<-bus, lat<-0, inr
IE5h: hwvec, i<-bus, lat<-ivt, pc<-ivt, next IE6
IE1s: lat<-sp-1, sp<-sp-1, I<-0
IE2s: a=i, bus<-a, ram<-bus, lat<-sp-1, sp<-sp-1, H<-0
IE3s: bus<-pclo, ram<-bus, lat<-sp-1, sp<-sp-1
IE4s: bus<-pchi, ram<-bus, lat<-0
IE5s: swvec, lat<-ivt, pc<-ivt
IE6:  bus<-ram, v1<-bus, lat<-pc+1
IE7:  bus<-ram, v2<-bus
IE8:  lat<-vx, pc<-vx, end

# ----- Reset (step 0 of every row) and boot (row 3F, B)

seq reset
reset: pc<-ivt.hi, sp<-ivt.hi, alu=and, b=zero, if xn S1 else reset
S1:    bus<-sphi, ir0<-bus, ir1<-bus, alu=and, b=zero, restart

seq boot
S2:  bus<-res, reg0<-bus, alu=and, b=zero
S3:  bus<-res, reg1<-bus, alu=and, b=zero
S4:  bus<-res, reg2<-bus, alu=and, b=zero
S5:  bus<-res, reg3<-bus, alu=and, b=zero
S6:  bus<-res, reg4<-bus, alu=and, b=zero
S7:  bus<-res, reg5<-bus, alu=and, b=zero
S8:  bus<-res, reg6<-bus, alu=and, b=zero
S9:  bus<-res, reg7<-bus, lat<-pc
C:   ram<-rom, lat<-pc+1, pc<-pc+1, if xn C else FIX
FIX: lat<-pc-1, pc<-pc-1, end nocheck

# ----- Invalid mode: undo the fetch's PC increments, then stop (I, then H).
# stop1: opcode without operands, mode bits of byte 0 not 00 (1 byte read).
# stop2: opcode with operands, mode not listed for it (2 bytes read).

seq stop1
     I<-0, pc<-pc-1
     H<-1, end halt

seq stop2
     f1
     I<-0, pc<-pc-1
     H<-1, pc<-pc-1, end halt

# ----- ALU instructions: R1 (R3 in mode 4) := R1 op operand

instr add adc sub sbb and or xor : reg
     f1
     alu=$alu, a=r1, b=r2, flags<-$fl
     r1<-bus, bus<-res, end

instr add adc sub sbb and or xor : regimm8
     f1
     fv1
     alu=$alu, a=r1, flags<-$fl
     r1<-bus, bus<-res, end

instr add adc sub sbb and or xor : regram
     f1
     fv1
     fv2
     lat<-vx
     bus<-ram, lat<-pc
     alu=$alu, a=r1, flags<-$fl
     r1<-bus, bus<-res, end

instr add adc sub sbb and or xor : mode4
     f1
     fv1
     fv2
     a=r1, b=r2, lat<-a:b+v2
     bus<-ram, lat<-pc
     alu=$alu, a=r3, flags<-$fl
     r3<-bus, bus<-res, end

instr add adc sub sbb and or xor : stack
     f1
     fv1
     lat<-sp+v1
     bus<-ram, lat<-pc
     alu=$alu, a=r1, flags<-$fl
     r1<-bus, bus<-res, end

# CMP: flags S E F only, no write (no RegRam mode)

instr cmp : reg
     f1
     alu=sub, a=r1, b=r2, flags<-sef, end

instr cmp : regimm8
     f1
     fv1
     alu=sub, a=r1, flags<-sef, end

instr cmp : mode4
     f1
     fv1
     fv2
     a=r1, b=r2, lat<-a:b+v2
     bus<-ram, lat<-pc
     alu=sub, a=r3, flags<-sef, end

instr cmp : stack
     f1
     fv1
     lat<-sp+v1
     bus<-ram, lat<-pc
     alu=sub, a=r1, flags<-sef, end

# INC / DEC (b = 01), NOT, shifts

instr inc dec : reg
     f1
     alu=$alu, a=r1, b=one, flags<-czn
     r1<-bus, bus<-res, end

instr inc dec : imm16
     f1
     fv1
     fv2
     lat<-vx
     bus<-ram
     alu=$alu, b=one, flags<-czn
     ram<-bus, bus<-res, lat<-pc, end

instr inc dec : mode4
     f1
     fv1
     fv2
     a=r1, b=r2, lat<-a:b+v2
     bus<-ram
     alu=$alu, b=one, flags<-czn
     ram<-bus, bus<-res, lat<-pc, end

instr not shl asr shr : reg
     f1
     alu=$alu, a=r1, flags<-$fl
     r1<-bus, bus<-res, end

instr not : imm16
     f1
     fv1
     fv2
     lat<-vx
     bus<-ram
     alu=not, flags<-zn
     ram<-bus, bus<-res, lat<-pc, end

# ----- Moves, memory, stack, I/O

instr mov : reg
     f1
     alu=and, a=r2, b=r2
     r1<-bus, bus<-res, end

instr mov : regimm8
     f1
     fv1, r1<-bus, end

instr lod : regram
     f1
     fv1
     fv2
     lat<-vx
     bus<-ram, r1<-bus, lat<-pc, end

instr lod : mode4
     f1
     fv1
     fv2
     a=r1, b=r2, lat<-a:b+v2
     bus<-ram, r3<-bus, lat<-pc, end

instr lod : stack
     f1
     fv1
     lat<-sp+v1
     bus<-ram, r1<-bus, lat<-pc, end

instr str : regram
     f1
     fv1
     fv2
     lat<-vx
     a=r1, bus<-a, ram<-bus, lat<-pc, end

instr str : mode4
     f1
     fv1
     fv2
     a=r1, b=r2, lat<-a:b+v2
     a=r3, bus<-a, ram<-bus, lat<-pc, end

instr str : stack
     f1
     fv1
     lat<-sp+v1
     a=r1, bus<-a, ram<-bus, lat<-pc, end

instr psh : reg
     f1
     lat<-sp-1, sp<-sp-1
     a=r1, bus<-a, ram<-bus, lat<-pc, end

instr pop : reg
     f1
     lat<-sp
     bus<-ram, r1<-bus, sp<-sp+1
     lat<-pc, end

instr psf : none
     lat<-sp-1, sp<-sp-1
     bus<-flags, ram<-bus, lat<-pc, end

instr pof : none
     lat<-sp
     bus<-ram, flags<-bus, sp<-sp+1
     lat<-pc, end

instr in : reg
     f1
     b=r2, lat<-0:b
     bus<-io, r1<-bus, lat<-pc, end

instr out : reg
     f1
     b=r1, lat<-0:b
     a=r2, bus<-a, io<-bus, lat<-pc, end

# ----- Stack pointer

instr asp ssp : imm8
     f1
     fv1
     sp<-$sp8, end

instr asp ssp : imm16
     f1
     fv1
     fv2
     sp<-$sp16, end

instr wsp : reg16
     f1
     a=r1, b=r2, sp<-a:b, end

instr wsp : imm16
     f1
     fv1
     fv2
     sp<-vx, end

instr rsp : reg16
     f1
     bus<-sphi, r1<-bus
     bus<-splo, r2<-bus, end

# ----- Jumps (the 74HCT151 picks the flag from opcode bits 2..0), calls

instr jmp jmc jme jmn jms jmz jmf : reg16
     f1, if jc taken else skip
taken: a=r1, b=r2, lat<-a:b, pc<-a:b, end
skip:  end

instr jmp jmc jme jmn jms jmz jmf : imm16
     f1
     fv1
     fv2, if jc taken else skip
taken: lat<-vx, pc<-vx, end
skip:  end

instr cal : reg16
     f1
     lat<-sp-1, sp<-sp-1
     bus<-pclo, ram<-bus, lat<-sp-1, sp<-sp-1
     bus<-pchi, ram<-bus, a=r1, b=r2, lat<-a:b, pc<-a:b, end

instr cal : imm16
     f1
     fv1
     fv2
     lat<-sp-1, sp<-sp-1
     bus<-pclo, ram<-bus, lat<-sp-1, sp<-sp-1
     bus<-pchi, ram<-bus, lat<-vx, pc<-vx, end

instr ret : none
     lat<-sp
     bus<-ram, v1<-bus, lat<-sp+1, sp<-sp+1
     bus<-ram, v2<-bus, sp<-sp+1
     lat<-vx, pc<-vx, end

instr irt : none
     lat<-sp
     bus<-ram, v1<-bus, lat<-sp+1, sp<-sp+1
     bus<-ram, v2<-bus, lat<-sp+1, sp<-sp+1
     bus<-ram, i<-bus, sp<-sp+1, I<-1
     lat<-vx, pc<-vx, end

# INT: the pending port latch takes V1 while it is fetched; the boundary
# step after it sees P = 1.
instr int : imm8
     f1
     fv1, swi<-bus
     end

# ----- No operation, flags, halt

instr nop : none
     -

instr clc cle cln cls clz clf : none
     $f<-0, end

instr stc ste stn sts stz stf : none
     $f<-1, end

# CLI: no interrupt check (it would still see I = 1). STI: interrupts wait
# until the next instruction has run.
instr cli : none
     I<-0, end nocheck

instr sti : none
     I<-1, end nocheck

instr hlt : none
     H<-1, end halt
