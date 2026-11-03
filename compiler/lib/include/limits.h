// limits.h - HBC-2 C library: the ranges of the integer types.
// char is unsigned (ABI.md, section 1); int is 16 bits, long 32 bits.

#ifndef __LIMITS_H
#define __LIMITS_H

#define CHAR_BIT    8               // bits in a char
#define MB_LEN_MAX  1               // bytes in a multibyte character

#define SCHAR_MIN   (-128)
#define SCHAR_MAX   127
#define UCHAR_MAX   255

#define CHAR_MIN    0               // char is unsigned
#define CHAR_MAX    255

#define SHRT_MIN    (-32767 - 1)    // -32768 written as a constant can get the wrong type
#define SHRT_MAX    32767
#define USHRT_MAX   65535U

#define INT_MIN     (-32767 - 1)
#define INT_MAX     32767
#define UINT_MAX    65535U

#define LONG_MIN    (-2147483647L - 1)
#define LONG_MAX    2147483647L
#define ULONG_MAX   4294967295UL

#endif
