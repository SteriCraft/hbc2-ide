/* stdlib.h - HBC-2 C library */

#ifndef __STDLIB_H
#define __STDLIB_H

#ifndef NULL
#define NULL ((void *)0)
#endif


/* Ends the program: status goes where main's result goes ([a:b]). */
void           exit(int status);

/* Absolute value. abs(-32768) and labs(-2147483648) stay negative (the
   positive value does not fit). */
int            abs(int n);
long           labs(long n);


/* --- Random numbers ---------------------------------------------------
   xorshift32: good for games and simulations, not for cryptography.
   Without srand, every run gives the same numbers. */

#define RAND_MAX 32767

int            rand(void);                         // 0 to RAND_MAX
void           srand(unsigned seed);              // same seed, same numbers


// --- Number <-> text --------------------------------------------------

/* Spaces first (' ', \t \n \v \f \r), an optional sign, then the digits
   of base (2 to 36: 0-9 then a-z or A-Z). Base 16 may start with "0x" or
   "0X", base 2 with "0b" or "0B" (C23). Base 0 picks it from the text:
   "0x" hexadecimal, "0b" binary, a leading 0 octal, else decimal.
   *str_end (when str_end is not NULL) = the first character not read,
   or str when there is no digit (then 0 is returned); 0 and str for a
   base out of range too. When the number does not fit: strtol gives
   2147483647 or -2147483648, strtoul 4294967295. strtoul("-1", ...) is
   4294967295: a '-' negates the unsigned value. No errno. */
long           strtol(const char *str, char **str_end, int base);
unsigned long  strtoul(const char *str, char **str_end, int base);

/* strtol(s, NULL, 10): decimal only. atoi gives the low 16 bits. */
int            atoi(const char *s);
long           atol(const char *s);

/* Not standard C, but common: value written at buf in base radix, 0-terminated.
   They return buf. Radix is 2, 8, 10 or 16 (an empty text for any other).
   Hexadecimal digits are lower case; no leading zeros, no "0x".
   Only radix 10 writes a '-': in the others, a negative value is written
   as its bits (itoa(-1, buf, 16) is "ffff"). */
char*          itoa(int value, char *buf, int radix);            // 17 (radix 2)
char*          utoa(unsigned value, char *buf, int radix);       // 17 (radix 2)
char*          ltoa(long value, char *buf, int radix);           // 33 (radix 2)
char*          ultoa(unsigned long value, char *buf, int radix); // 33 (radix 2)

#endif
