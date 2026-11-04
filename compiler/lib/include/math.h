// math.h - HBC-2 C library: floating point and integer math
//
// float and double are the same type here: IEEE 754 binary32 (4 bytes,
// about 7 decimal digits). Every function has its standard name (on
// double) and an f version (on float): sin and sinf are the same. Assembly
// in asm/fmath.has, on the float library (asm/math/float).
//
// Accuracy: within 0.61 ulp (nearly always the correctly rounded float)
// for everything but sin, cos and tan (within 0.85 ulp); sqrt, the
// rounding functions, fmod, fabs... are exact. Special values follow C99
// Annex F (sqrt(-1) is NaN, log(0) is -inf, pow(x, 0) is 1...). Errors are
// reported only through the exception flags of fenv.h (math_errhandling is
// MATH_ERREXCEPT): errno is not set.
//
// The integer functions at the end (isqrt, ipow, gcd...) are not standard
// C: they have their own names so as not to clash. abs and labs are in
// stdlib.h.

#ifndef __MATH_H
#define __MATH_H

typedef float float_t;
typedef float double_t;

#define HUGE_VAL   (1.0 / 0.0)          // +inf (folded by the compiler)
#define HUGE_VALF  (1.0f / 0.0f)
#define INFINITY   (1.0f / 0.0f)
#define NAN        (0.0f / 0.0f)        // the quiet NaN

#define FP_NAN       0                  // fpclassify
#define FP_INFINITE  1
#define FP_ZERO      2
#define FP_SUBNORMAL 3
#define FP_NORMAL    4

#define FP_ILOGB0   (-32767 - 1)
#define FP_ILOGBNAN (-32767 - 1)

#define MATH_ERRNO      1
#define MATH_ERREXCEPT  2
#define math_errhandling MATH_ERREXCEPT

// Classification (on the bits: never an exception). Functions, not macros
// on any type: float and double are the same.
int            fpclassify(double x);
int            isfinite(double x);
int            isinf(double x);
int            isnan(double x);
int            isnormal(double x);
int            signbit(double x);       // 1 for -0, -inf, a NaN with the sign set

// Comparisons that never raise FE_INVALID for a quiet NaN (0 or 1)
int            isgreater(double x, double y);
int            isgreaterequal(double x, double y);
int            isless(double x, double y);
int            islessequal(double x, double y);
int            islessgreater(double x, double y);
int            isunordered(double x, double y);

// Trigonometry (radians)
double         sin(double x);
double         cos(double x);
double         tan(double x);
double         asin(double x);          // [-pi/2, pi/2]
double         acos(double x);          // [0, pi]
double         atan(double x);          // [-pi/2, pi/2]
double         atan2(double y, double x);   // [-pi, pi]

// Hyperbolic
double         sinh(double x);
double         cosh(double x);
double         tanh(double x);
double         asinh(double x);
double         acosh(double x);
double         atanh(double x);

// Exponentials and logarithms
double         exp(double x);
double         exp2(double x);
double         expm1(double x);         // e^x - 1, exact for small x
double         log(double x);
double         log2(double x);
double         log10(double x);
double         log1p(double x);         // log(1 + x), exact for small x
double         frexp(double x, int *exp);   // x = m 2^*exp, |m| in [0.5, 1)
double         ldexp(double x, int exp);    // x 2^exp
double         modf(double x, double *iptr);    // fraction; *iptr = integer part

// Powers and roots
double         pow(double x, double y);
double         sqrt(double x);
double         cbrt(double x);
double         hypot(double x, double y);   // sqrt(x^2 + y^2), no overflow

// Rounding, remainder, sign
double         ceil(double x);
double         floor(double x);
double         trunc(double x);
double         round(double x);         // halves away from 0
double         rint(double x);          // halves to even, FE_INEXACT
double         nearbyint(double x);     // halves to even, no FE_INEXACT
double         fmod(double x, double y);    // exact, sign of x
double         fabs(double x);
double         copysign(double x, double y);
double         fmin(double x, double y);    // a NaN loses
double         fmax(double x, double y);
double         fdim(double x, double y);    // x - y if x > y, else +0

// The same on float
float          sinf(float x);
float          cosf(float x);
float          tanf(float x);
float          asinf(float x);
float          acosf(float x);
float          atanf(float x);
float          atan2f(float y, float x);
float          sinhf(float x);
float          coshf(float x);
float          tanhf(float x);
float          asinhf(float x);
float          acoshf(float x);
float          atanhf(float x);
float          expf(float x);
float          exp2f(float x);
float          expm1f(float x);
float          logf(float x);
float          log2f(float x);
float          log10f(float x);
float          log1pf(float x);
float          frexpf(float x, int *exp);
float          ldexpf(float x, int exp);
float          modff(float x, float *iptr);
float          powf(float x, float y);
float          sqrtf(float x);
float          cbrtf(float x);
float          hypotf(float x, float y);
float          ceilf(float x);
float          floorf(float x);
float          truncf(float x);
float          roundf(float x);
float          rintf(float x);
float          nearbyintf(float x);
float          fmodf(float x, float y);
float          fabsf(float x);
float          copysignf(float x, float y);
float          fminf(float x, float y);
float          fmaxf(float x, float y);
float          fdimf(float x, float y);


// --- Integer math (not standard C) ------------------------------------
// The l versions take an (unsigned) long.

// Square root, rounded down: isqrt(10) = 3.
unsigned       isqrt(unsigned n);
unsigned       lsqrt(unsigned long n);

// base to the power exp (exp 0 to 255; ipow(x, 0) = 1, 0^0 too). When the
// result does not fit, its low bits are kept, as with *.
int            ipow(int base, unsigned char exp);
long           lpow(long base, unsigned char exp);

// Greatest common divisor: gcd(12, 18) = 6. gcd(n, 0) = n, gcd(0, 0) = 0.
unsigned       gcd(unsigned a, unsigned b);
unsigned long  lgcd(unsigned long a, unsigned long b);

// Bits: popcount = how many 1 bits; clz = 0 bits above the highest 1
// (leading zeros); ctz = 0 bits below the lowest 1 (trailing zeros).
// clz(0) and ctz(0) give the width: 16 (32 for lclz, lctz).
int            popcount(unsigned n);
int            clz(unsigned n);
int            ctz(unsigned n);
int            lpopcount(unsigned long n);
int            lclz(unsigned long n);
int            lctz(unsigned long n);

// Smaller, larger: min / max for int, umin / umax for unsigned.
int            min(int a, int b);
int            max(int a, int b);
unsigned       umin(unsigned a, unsigned b);
unsigned       umax(unsigned a, unsigned b);

#endif
