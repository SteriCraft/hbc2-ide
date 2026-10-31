/* stdlib.h - HBC-2 C library */

#ifndef __STDLIB_H
#define __STDLIB_H

#ifndef NULL
#define NULL ((void *)0)
#endif

/* Ends the program: status goes where main's result goes ([a:b]). */
void exit(int status);

/* Absolute value. abs(-32768) and labs(-2147483648) stay negative (the
   positive value does not fit). */
int abs(int n);
long labs(long n);

/* --- Random numbers ---------------------------------------------------
   xorshift32: good for games and simulations, not for cryptography.
   Without srand, every run gives the same numbers. Assembly in
   asm/mathlib/misc/random.has (the math library). */

#define RAND_MAX 32767

int rand(void);                         /* 0 to RAND_MAX */
void srand(unsigned seed);              /* same seed, same numbers */

/* Not standard C (HBC-2 extras): */
int srand_rtc(void);        /* seeds from the clock: 0, or 1 (no RTC) */
/* lo to hi (both included), every value equally likely; lo when hi < lo */
int rand_range(int lo, int hi);

/* --- Number <-> text --------------------------------------------------
   Assembly in asm/mathlib/misc/text.has (the math library). */

/* Spaces first, then an optional sign and decimal digits, or "0x" and
   hexadecimal digits (not standard C; any case, up to 32 bits:
   atoi("0xFFFF") and atol("0xFFFFFFFF") are -1). 0 when there is no
   digit or the number does not fit in a long; atoi gives the low 16 bits
   of atol's result. */
int atoi(const char *s);
long atol(const char *s);

/* Not standard C (HBC-2 extras): value written at buf, 0-terminated.
   They return the end of the text (the 0), so the length is end - buf.
   Hexadecimal: upper case, fixed width (leading zeros kept), no "0x".
   Sizes of buf, with the 0: */
char *itoa(int value, char *buf);               /* 7  "-32768"      */
char *utoa(unsigned value, char *buf);          /* 6  "65535"       */
char *ltoa(long value, char *buf);              /* 12 "-2147483648" */
char *ultoa(unsigned long value, char *buf);    /* 11 "4294967295"  */
char *htoa(unsigned value, char *buf);          /* 5  "FFFF"        */
char *lhtoa(unsigned long value, char *buf);    /* 9  "FFFFFFFF"    */

#endif
