/* Expected: main returns 0 (math.h on float, float.h, fenv.h, strtod) */
/* Returns the number of the first failed check. The float library itself
   is tested much harder by compiler/tools/ftest: this checks that every C
   function reaches it with the right arguments and gives the right result
   back. Transcendental results may be 1 ulp off (the f64 function rounded to
   float is the reference). */

#include <math.h>
#include <float.h>
#include <fenv.h>
#include <stdlib.h>
#include <errno.h>

static int fail;
static int n;
static float k_inf = INFINITY;
static float k_six = 2.0f * 3.0f;
static int k_cmp = 1.5f < 2.5f;

static unsigned long bits(float f)
{
    return *(unsigned long *)&f;
}

static void check(int ok)
{
    n++;
    if (!ok && fail == 0)
        fail = n;
}

static void is(float f, unsigned long b)
{
    check(bits(f) == b);
}

/* within 1 ulp */
static void near(float f, unsigned long b)
{
    unsigned long v = bits(f);
    check(v == b || v == b + 1 || v == b - 1);
}

int main(void)
{
    float ip, z = 0, one = 1;
    int e;
    char *end;
    char *s;

    /* 1-20: trigonometry, hyperbolic, exponentials, logarithms */
    near(sin(1), 0x3F576AA4UL);
    near(cos(1), 0x3F0A5140UL);
    near(tanf(1), 0x3FC75923UL);
    near(asin(.5), 0x3F060A92UL);
    near(acos(.5), 0x3F860A92UL);
    near(atan(1), 0x3F490FDBUL);
    near(atan2(1, -1), 0x4016CBE4UL);
    near(sinh(1), 0x3F966CFEUL);
    near(cosh(1), 0x3FC583ABUL);
    near(tanh(.5), 0x3EEC9A9FUL);
    near(asinh(1), 0x3F61A1B3UL);
    near(acosh(2), 0x3FA89214UL);
    near(atanhf(.5f), 0x3F0C9F54UL);
    near(exp(1), 0x402DF854UL);
    near(expm1(1e-5), 0x3727C5E3UL);
    near(log(10), 0x40135D8EUL);
    near(log1p(1e-5), 0x3727C575UL);
    is(exp2(10), 0x44800000UL);
    is(log2(8), 0x40400000UL);
    is(log10(1000), 0x40400000UL);

    /* 21-40: powers, roots, rounding, remainder, sign */
    is(pow(2, 10), 0x44800000UL);
    near(pow(1.5, 2.5), 0x40305CFFUL);
    is(sqrt(2), 0x3FB504F3UL);
    is(sqrtf(2), 0x3FB504F3UL);
    is(cbrt(-27), 0xC0400000UL);
    is(hypot(3, 4), 0x40A00000UL);
    is(ceil(-1.5), 0xBF800000UL);
    is(floor(-1.5), 0xC0000000UL);
    is(trunc(-1.7), 0xBF800000UL);
    is(round(2.5), 0x40400000UL);
    is(rint(2.5), 0x40000000UL);
    is(nearbyint(3.5), 0x40800000UL);
    is(fmod(7, -3), 0x3F800000UL);
    is(fabs(-2), 0x40000000UL);
    is(copysign(3, -0.0), 0xC0400000UL);
    is(fmin(1, NAN), 0x3F800000UL);
    is(fmax(-0.0, 0), 0x00000000UL);
    is(fdim(5, 3), 0x40000000UL);
    is(fdim(3, 5), 0x00000000UL);
    is(ldexp(1, -149), 0x00000001UL);

    /* 41-48: frexp, modf */
    is(frexp(8, &e), 0x3F000000UL);
    check(e == 4);
    is(frexpf(-0.375f, &e), 0xBF400000UL);
    check(e == -1);
    is(modf(-3.25, &ip), 0xBE800000UL);
    is(ip, 0xC0400000UL);
    is(modf(-3, &ip), 0x80000000UL);
    is(modf(k_inf, &ip), 0x00000000UL);

    /* 49-62: classification, constants */
    check(isnan(NAN) && !isnan(1) && isinf(-INFINITY) && !isinf(FLT_MAX));
    check(isfinite(FLT_MAX) && !isfinite(HUGE_VAL) && isnormal(FLT_MIN));
    check(!isnormal(FLT_TRUE_MIN) && signbit(-0.0) && !signbit(0.0));
    check(fpclassify(1e-40) == FP_SUBNORMAL && fpclassify(0) == FP_ZERO);
    check(fpclassify(NAN) == FP_NAN && fpclassify(-INFINITY) == FP_INFINITE);
    check(fpclassify(-1) == FP_NORMAL);
    is(k_inf, 0x7F800000UL);
    is(k_six, 0x40C00000UL);
    check(k_cmp == 1);
    is(NAN, 0x7FC00000UL);
    is(FLT_EPSILON, 0x34000000UL);
    is(FLT_TRUE_MIN, 0x00000001UL);
    is(DBL_MAX, 0x7F7FFFFFUL);
    check(FLT_MANT_DIG == 24 && FLT_DIG == 6 && DBL_DIG == 6);

    /* 63-76: exception flags */
    feclearexcept(FE_ALL_EXCEPT);
    check(fetestexcept(FE_ALL_EXCEPT) == 0);
    ip = one / z;
    check(fetestexcept(FE_ALL_EXCEPT) == FE_DIVBYZERO);
    feclearexcept(FE_DIVBYZERO);
    ip = sqrt(-one);
    check(fetestexcept(FE_ALL_EXCEPT) == FE_INVALID && isnan(ip));
    feclearexcept(FE_ALL_EXCEPT);
    ip = FLT_MAX * (one + one);
    check(fetestexcept(FE_ALL_EXCEPT) == (FE_OVERFLOW | FE_INEXACT));
    feclearexcept(FE_ALL_EXCEPT);
    check(isless(NAN, one) == 0 && fetestexcept(FE_INVALID) == 0);
    check((NAN < one) == 0 && fetestexcept(FE_INVALID) == FE_INVALID);
    check(isunordered(NAN, one) && islessgreater(one, 2) && !islessgreater(one, one));
    {
        fenv_t env;
        fexcept_t fl;
        feholdexcept(&env);
        check(fetestexcept(FE_ALL_EXCEPT) == 0);
        feraiseexcept(FE_UNDERFLOW);
        feupdateenv(&env);
        check(fetestexcept(FE_ALL_EXCEPT) == (FE_INVALID | FE_UNDERFLOW));
        fegetexceptflag(&fl, FE_INVALID);
        fesetenv(FE_DFL_ENV);
        check(fetestexcept(FE_ALL_EXCEPT) == 0);
        fesetexceptflag(&fl, FE_ALL_EXCEPT);
        check(fetestexcept(FE_ALL_EXCEPT) == FE_INVALID);
    }
    check(fegetround() == FE_TONEAREST && fesetround(FE_TONEAREST) == 0 && fesetround(3) != 0);
    feclearexcept(FE_ALL_EXCEPT);
    ip = one / 3;
    check(fetestexcept(FE_INEXACT) == FE_INEXACT);

    /* 77-90: strtod, atof */
    s = "  -12.5e1xyz";
    is(strtod(s, &end), 0xC2FA0000UL);
    check(end == s + 9);
    s = "abc";
    is(strtod(s, &end), 0x00000000UL);
    check(end == s);
    errno = 0;
    is(strtod("1e40", &end), 0x7F800000UL);
    check(errno == ERANGE && *end == 0);
    errno = 0;
    is(strtof("1e-46", NULL), 0x00000000UL);
    check(errno == ERANGE);
    errno = 0;
    is(atof("0.1"), 0x3DCCCCCDUL);
    check(errno == 0);
    is(atof("-INFINITY"), 0xFF800000UL);
    check(isnan(atof("nan")));
    is(atof("\t1.17549435e-38"), 0x00800000UL);
    feclearexcept(FE_ALL_EXCEPT);
    atof("0.5");
    check(fetestexcept(FE_ALL_EXCEPT) == 0);

    return fail;
}
