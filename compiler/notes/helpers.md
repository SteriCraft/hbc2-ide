# Helpers (step 4)

(Moved out of ROADMAP.md 2026-10-31. Read only when working on this part.)

The probe (every operator × every type, scratchpad probe/) gave exactly 89 helper names: chars are always widened to int except in assignment operators (`__pluseqc`…); long assignment operators get a long right operand (even shift counts). Helper conventions: ABI.md section 11. Runtime layout: comments at the top of each runtime file.

Old list:

From a program using every operator (scratchpad th.c): `__mul __div __divu __rem __remu __shl __shr __shru __cclt __ccltu __ccgt __ccgtu __cclteq __ccltequ __ccgteq __ccgtequ __pluseq __pluseqc __minuseq __muleq __divequ __remeq __shleq __shreq __andeq __oreq __xoreq __switch` and the long forms `__plusl __minusl __mull __divl __divul __reml __bandl __orl __xorl __negatel __cpll __shll __shrl __shrul __cceql __ccnel __ccltl __ccgtul __pluseql __minuseql __switchl` (+ the missing combinations: other compare/assign-op variants for c/uc/l/ul — generate the list systematically from backend.c make_node + helper_type). Also postinc/postdec variants when the result is used.

