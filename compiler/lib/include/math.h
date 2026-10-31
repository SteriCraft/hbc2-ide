/* math.h - HBC-2 C library: integer math

   The HBC-2 has no floating point, so standard C's math.h (sqrt, pow...
   on double) is not here. These are integer functions with their own
   names (isqrt, ipow...), so that the float ones can be added one day
   without a clash. Not standard C. Assembly in asm/math.has, on the math
   library (asm/mathlib). abs and labs are in stdlib.h.

   The l versions take an (unsigned) long. */

#ifndef __MATH_H
#define __MATH_H

/* Square root, rounded down: isqrt(10) = 3. */
unsigned isqrt(unsigned n);
unsigned lsqrt(unsigned long n);

/* base to the power exp (exp 0 to 255; ipow(x, 0) = 1, 0^0 too). When the
   result does not fit, its low bits are kept, as with *. */
int ipow(int base, unsigned char exp);
long lpow(long base, unsigned char exp);

/* Greatest common divisor: gcd(12, 18) = 6. gcd(n, 0) = n, gcd(0, 0) = 0. */
unsigned gcd(unsigned a, unsigned b);
unsigned long lgcd(unsigned long a, unsigned long b);

/* Bits: popcount = how many 1 bits; clz = 0 bits above the highest 1
   (leading zeros); ctz = 0 bits below the lowest 1 (trailing zeros).
   clz(0) and ctz(0) give the width: 16 (32 for lclz, lctz). */
int popcount(unsigned n);
int clz(unsigned n);
int ctz(unsigned n);
int lpopcount(unsigned long n);
int lclz(unsigned long n);
int lctz(unsigned long n);

/* Smaller, larger: min / max for int, umin / umax for unsigned. */
int min(int a, int b);
int max(int a, int b);
unsigned umin(unsigned a, unsigned b);
unsigned umax(unsigned a, unsigned b);

#endif
