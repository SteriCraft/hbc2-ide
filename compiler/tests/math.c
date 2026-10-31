/* Expected: main returns 0 (math.h, abs, labs) */
/* Returns the number of the first failed check. Each function is checked
   against plain C on many values: edge values, then pseudo-random ones. */

#include <math.h>
#include <stdlib.h>

static unsigned long seed = 12345;

/* xorshift32, in C (not rand: its own state) */
static unsigned long next(void)
{
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    return seed;
}

/* a value with 0 to 32 significant bits, so every size is tried */
static unsigned long any(void)
{
    unsigned long v = next();
    return v >> (unsigned)(next() & 31);
}

static int sqrt_ok(unsigned long n, unsigned long r)
{
    /* r * r <= n < (r + 1)^2; (r + 1)^2 may need 33 bits when r = 65535 */
    if (r > 65535UL || r * r > n)
        return 0;
    return r == 65535UL || (r + 1) * (r + 1) > n;
}

static unsigned long gcd_c(unsigned long a, unsigned long b)
{
    while (b) {
        unsigned long t = a % b;
        a = b;
        b = t;
    }
    return a;
}

static unsigned long pow_c(unsigned long b, unsigned char e)
{
    unsigned long r = 1;
    while (e--)
        r *= b;
    return r;
}

static int pop_c(unsigned long n)
{
    int c = 0;
    while (n) {
        c += (int)(n & 1);
        n >>= 1;
    }
    return c;
}

static int clz_c(unsigned long n, int width)
{
    int c = 0;
    unsigned long top = 1UL << (width - 1);
    while (c < width && !(n & top)) {
        n <<= 1;
        c++;
    }
    return c;
}

static int ctz_c(unsigned long n, int width)
{
    int c = 0;
    while (c < width && !(n & 1)) {
        n >>= 1;
        c++;
    }
    return c;
}

static const unsigned edges[] = { 0, 1, 2, 3, 4, 255, 256, 32767, 32768u, 65534u, 65535u };
static const unsigned long ledges[] = {
    0, 1, 2, 65535UL, 65536UL, 0x7FFFFFFFUL, 0x80000000UL, 0xFFFFFFFEUL, 0xFFFFFFFFUL
};

/* every function on one pair of values; 0 or the failed check */
static int check(unsigned long la, unsigned long lb)
{
    unsigned a = (unsigned)la, b = (unsigned)lb;
    unsigned char e = (unsigned char)(lb & 15);

    if (!sqrt_ok(a, isqrt(a)))
        return 1;
    if (!sqrt_ok(la, lsqrt(la)))
        return 2;
    if (gcd(a, b) != (unsigned)gcd_c(a, b))
        return 3;
    if (lgcd(la, lb) != gcd_c(la, lb))
        return 4;
    if ((unsigned)ipow((int)a, e) != (unsigned)pow_c(a, e))
        return 5;
    if ((unsigned long)lpow((long)la, e) != pow_c(la, e))
        return 6;
    if (popcount(a) != pop_c(a) || lpopcount(la) != pop_c(la))
        return 7;
    if (clz(a) != clz_c(a, 16) || lclz(la) != clz_c(la, 32))
        return 8;
    if (ctz(a) != ctz_c(a, 16) || lctz(la) != ctz_c(la, 32))
        return 9;
    if (min((int)a, (int)b) != ((int)a < (int)b ? (int)a : (int)b))
        return 10;
    if (max((int)a, (int)b) != ((int)a > (int)b ? (int)a : (int)b))
        return 11;
    if (umin(a, b) != (a < b ? a : b) || umax(a, b) != (a > b ? a : b))
        return 12;
    if (abs((int)a) != ((int)a < 0 ? -(int)a : (int)a))
        return 13;
    if (labs((long)la) != ((long)la < 0 ? -(long)la : (long)la))
        return 14;
    return 0;
}

int main(void)
{
    unsigned i, j;
    int r;

    /* (the loops below also check sizeof a / sizeof a[0], which FCC read
       as sizeof (a / sizeof a[0]) until 2026-10-31) */

    /* known values */
    if (isqrt(10) != 3 || isqrt(65535u) != 255 || lsqrt(0xFFFFFFFFUL) != 65535u)
        return 20;
    if (ipow(2, 10) != 1024 || ipow(-3, 3) != -27 || ipow(0, 0) != 1 || ipow(7, 0) != 1)
        return 21;
    if (lpow(10, 9) != 1000000000L || lpow(-2, 31) != (long)0x80000000UL)
        return 22;
    if (ipow(2, 16) != 0 || lpow(3, 40) != (long)0x291FE821UL)  /* low bits kept */
        return 23;
    if (gcd(12, 18) != 6 || gcd(0, 0) != 0 || gcd(7, 0) != 7 || lgcd(0, 9) != 9)
        return 24;
    if (clz(0) != 16 || ctz(0) != 16 || lclz(0) != 32 || lctz(0) != 32)
        return 25;
    if (clz(1) != 15 || ctz(0x8000u) != 15 || lclz(1) != 31 || lctz(0x80000000UL) != 31)
        return 26;
    if (popcount(0xFFFFu) != 16 || lpopcount(0xFFFFFFFFUL) != 32)
        return 27;
    if (min(-5, 3) != -5 || max(-5, 3) != 3 || umin(65535u, 3) != 3 || umax(65535u, 3) != 65535u)
        return 28;
    if (abs(-7) != 7 || abs(7) != 7 || labs(-100000L) != 100000L || abs(-32767 - 1) != -32767 - 1)
        return 29;

    for (i = 0; i < sizeof edges / sizeof edges[0]; i++)
        for (j = 0; j < sizeof edges / sizeof edges[0]; j++)
            if ((r = check(edges[i], edges[j])) != 0)
                return 100 + r;
    for (i = 0; i < sizeof ledges / sizeof ledges[0]; i++)
        for (j = 0; j < sizeof ledges / sizeof ledges[0]; j++)
            if ((r = check(ledges[i], ledges[j])) != 0)
                return 200 + r;
    for (i = 0; i < 300; i++)
        if ((r = check(any(), any())) != 0)
            return 300 + r;
    return 0;
}
