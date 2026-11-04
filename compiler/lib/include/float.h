// float.h - HBC-2 C library: the limits of float and double
//
// float, double and long double are the same type: IEEE 754 binary32,
// rounding to nearest even, with subnormals, infinities and NaNs.

#ifndef __FLOAT_H
#define __FLOAT_H

#define FLT_RADIX        2
#define FLT_ROUNDS       1              // to nearest
#define FLT_EVAL_METHOD  0              // every operation in its own type
#define DECIMAL_DIG      9              // digits that give a float back

#define FLT_MANT_DIG     24
#define FLT_DIG          6
#define FLT_MIN_EXP      (-125)
#define FLT_MAX_EXP      128
#define FLT_MIN_10_EXP   (-37)
#define FLT_MAX_10_EXP   38
#define FLT_EPSILON      1.19209290e-7F     // 2^-23
#define FLT_MIN          1.17549435e-38F    // smallest normal, 2^-126
#define FLT_MAX          3.40282347e+38F
#define FLT_TRUE_MIN     1.40129846e-45F    // smallest subnormal, 2^-149
#define FLT_DECIMAL_DIG  9
#define FLT_HAS_SUBNORM  1

#define DBL_MANT_DIG     FLT_MANT_DIG
#define DBL_DIG          FLT_DIG
#define DBL_MIN_EXP      FLT_MIN_EXP
#define DBL_MAX_EXP      FLT_MAX_EXP
#define DBL_MIN_10_EXP   FLT_MIN_10_EXP
#define DBL_MAX_10_EXP   FLT_MAX_10_EXP
#define DBL_EPSILON      FLT_EPSILON
#define DBL_MIN          FLT_MIN
#define DBL_MAX          FLT_MAX
#define DBL_TRUE_MIN     FLT_TRUE_MIN
#define DBL_DECIMAL_DIG  FLT_DECIMAL_DIG
#define DBL_HAS_SUBNORM  1

#define LDBL_MANT_DIG    FLT_MANT_DIG
#define LDBL_DIG         FLT_DIG
#define LDBL_MIN_EXP     FLT_MIN_EXP
#define LDBL_MAX_EXP     FLT_MAX_EXP
#define LDBL_MIN_10_EXP  FLT_MIN_10_EXP
#define LDBL_MAX_10_EXP  FLT_MAX_10_EXP
#define LDBL_EPSILON     FLT_EPSILON
#define LDBL_MIN         FLT_MIN
#define LDBL_MAX         FLT_MAX
#define LDBL_TRUE_MIN    FLT_TRUE_MIN
#define LDBL_DECIMAL_DIG FLT_DECIMAL_DIG
#define LDBL_HAS_SUBNORM 1

#endif
