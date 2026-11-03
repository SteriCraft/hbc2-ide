// stdlib.h - HBC-2 C library

#ifndef __STDLIB_H
#define __STDLIB_H

#ifndef NULL
#define NULL ((void *)0)
#endif

#ifndef __SIZE_T
#define __SIZE_T
typedef unsigned size_t;
#endif


// Ends the program: status goes where main's result goes ([a:b]).
void           exit(int status);

// Absolute value. abs(-32768) and labs(-2147483648) stay negative (the
// positive value does not fit).
int            abs(int n);
long           labs(long n);


// --- Random numbers ---------------------------------------------------
// xorshift32: good for games and simulations, not for cryptography.
// Without srand, every run gives the same numbers.

#define RAND_MAX 32767

int            rand(void);                         // 0 to RAND_MAX
void           srand(unsigned seed);              // same seed, same numbers


// --- Number <-> text --------------------------------------------------

// Spaces first (' ', \t \n \v \f \r), an optional sign, then the digits
// of base (2 to 36: 0-9 then a-z or A-Z). Base 16 may start with "0x" or
// "0X", base 2 with "0b" or "0B" (C23). Base 0 picks it from the text:
// "0x" hexadecimal, "0b" binary, a leading 0 octal, else decimal.
// *str_end (when str_end is not NULL) = the first character not read,
// or str when there is no digit (then 0 is returned); 0 and str for a
// base out of range too. When the number does not fit: strtol gives
// 2147483647 or -2147483648, strtoul 4294967295. strtoul("-1", ...) is
// 4294967295: a '-' negates the unsigned value. errno = ERANGE
// when the number does not fit (errno.h).
long           strtol(const char *str, char **str_end, int base);
unsigned long  strtoul(const char *str, char **str_end, int base);

// strtol(s, NULL, 10): decimal only. atoi gives the low 16 bits.
int            atoi(const char *s);
long           atol(const char *s);

// Not standard C, but common: value written at buf in base radix, 0-terminated.
// They return buf. Radix is 2, 8, 10 or 16 (an empty text for any other).
// Hexadecimal digits are lower case; no leading zeros, no "0x".
// Only radix 10 writes a '-': in the others, a negative value is written
// as its bits (itoa(-1, buf, 16) is "ffff").
char*          itoa(int value, char *buf, int radix);            // 17 (radix 2)
char*          utoa(unsigned value, char *buf, int radix);       // 17 (radix 2)
char*          ltoa(long value, char *buf, int radix);           // 33 (radix 2)
char*          ultoa(unsigned long value, char *buf, int radix); // 33 (radix 2)


// --- Memory --------------------------------------------------------------
// The heap: from the end of the program's variables up toward the stack,
// keeping 256 bytes free below it (the stack can still grow further
// down into the heap: nothing checks). 2 bytes taken per block, sizes
// rounded up to even. NULL when there is no room.

void*          malloc(size_t n);                   // malloc(0): a block of 0 bytes
void           free(void *p);                      // free(NULL): nothing
void*          calloc(size_t n, size_t size);      // set to 0; NULL if n * size > 65535
// The contents are kept up to the smaller size. In place when it can
// (shrinking, the free room after it); else moved, and p freed. NULL p:
// malloc(n). n == 0: free(p), returns NULL. NULL on failure: p is kept.
void*          realloc(void *p, size_t n);


// --- Sorting and searching --------------------------------------------
// cmp(a, b) compares two elements: below 0 when a goes first, 0 when they
// are equal, above 0 when b goes first. For ints:
//     int cmp_int(const void *a, const void *b)
//     {
//         int x = *(const int *)a, y = *(const int *)b;
//         return x < y ? -1 : x > y;
//     }
// (x - y overflows for values far apart, such as 30000 and -30000.)

// Sorts the n elements of size bytes at base (Shell sort: no recursion,
// no memory taken). Equal elements may change order.
void           qsort(void *base, size_t n, size_t size,
                     int (*cmp)(const void *, const void *));

// An element of base equal to key, NULL if none: base must be sorted in
// the order of cmp, which gets key first. At most 17 comparisons.
void*          bsearch(const void *key, const void *base, size_t n, size_t size,
                       int (*cmp)(const void *, const void *));

#endif
