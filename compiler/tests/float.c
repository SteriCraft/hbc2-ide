/* Expected: main returns 0 (float and double: constants, operators, conversions) */
/* Returns the number of the first failed check. The expected values are the
   bits of IEEE 754 binary32 results (checked with Rust's f32). */

float g = 1.5;
double h = -2.25e-3;
float arr[4] = { 1, 2.5f, -0.0, .5e1 };
int gi = 3.7;
long gl = -2.9e5;
float gneg = -.5;

struct pt { char tag; float x; float y; };
struct pt gp = { 'p', 0.25, -8 };

static int fail;
static int n;

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

float add(float a, float b) { return a + b; }
double twice(double d) { return d * 2; }
float sum(float *p, int k)
{
    float s = 0;
    int i;
    for (i = 0; i < k; i++)
        s += p[i];
    return s;
}

int main(void)
{
    float a = 3, b = .5, c, z, nan, inf;
    float *p = &a;
    int i = 7;
    unsigned u = 65535;
    long l = 123456789;
    unsigned long ul = 0xFFFFFFFFUL;
    unsigned char uc = 200;
    signed char sc = -5;
    struct pt q;
    static float st = 1e-45f;

    /* 1-16: constants (cc0 converts them, correctly rounded) */
    is(g, 0x3FC00000UL);
    is(h, 0xBB1374BCUL);
    is(arr[0], 0x3F800000UL);
    is(arr[1], 0x40200000UL);
    is(arr[2], 0x80000000UL);
    is(arr[3], 0x40A00000UL);
    check(gi == 3);
    check(gl == -290000L);
    is(gneg, 0xBF000000UL);
    is(0.1f, 0x3DCCCCCDUL);
    is(1E-3f, 0x3A83126FUL);
    is(3.14159265, 0x40490FDBUL);
    is(st, 0x00000001UL);
    is(1.17549435e-38, 0x00800000UL);
    is(3.40282347e38, 0x7F7FFFFFUL);
    is(16777217.0, 0x4B800000UL);

    /* 17-26: arithmetic */
    is(0.1f + 0.2f, 0x3E99999AUL);
    c = 1;
    c = c / 3;
    is(c, 0x3EAAAAABUL);
    is(a + b, 0x40600000UL);
    is(a - b, 0x40200000UL);
    is(a * b, 0x3FC00000UL);
    is(a / b, 0x40C00000UL);
    is(-a, 0xC0400000UL);
    z = a - a;
    is(-z, 0x80000000UL);
    is(add(a, 2), 0x40A00000UL);
    is(twice(gp.x), 0x3F000000UL);

    /* 27-34: assignment operators, ++ and -- */
    c = 1.5;
    c += a;
    is(c, 0x40900000UL);
    c -= b;
    is(c, 0x40800000UL);
    c *= a;
    is(c, 0x41400000UL);
    c /= 4;
    is(c, 0x40400000UL);
    c = 1.5;
    is(c++, 0x3FC00000UL);
    is(c, 0x40200000UL);
    is(--c, 0x3FC00000UL);
    c--;
    ++c;
    *p += 1;
    is(a + c, 0x40B00000UL);

    /* 35-46: comparisons, -0, NaN, infinities */
    check(a > b && b < a && a >= 4 && a <= 4 && a == 4 && a != b);
    check(!(a < b) && !(b > a) && !(a == b));
    z = -z;
    check(!z && z == 0 && !(z < 0));
    i = 0;
    if (z)
        i = 1;
    check(i == 0 && !z == 1);
    nan = z / z;
    is(nan, 0x7FC00000UL);
    check(nan != nan && !(nan == nan) && !(nan < 1) && !(nan >= 1));
    i = 0;
    if (nan)
        i = 1;
    check(i == 1);
    z = 0;
    inf = 1 / z;
    is(inf, 0x7F800000UL);
    is(-1 / z, 0xFF800000UL);
    check(inf > 3.40282347e38 && -inf < -3.40282347e38);
    c = a > b ? a : b;
    is(c, 0x40800000UL);
    i = 0;
    while (c > 0.5f) {
        c = c / 2;
        i++;
    }
    check(i == 3);

    /* 47-58: integers to float */
    i = -32768;
    is(i, 0xC7000000UL);
    is(u, 0x477FFF00UL);
    is(l, 0x4CEB79A3UL);
    is(ul, 0x4F800000UL);
    is(0x7FFFFFFFL, 0x4F000000UL);
    is(uc, 0x43480000UL);
    is(sc, 0xC0A00000UL);
    l = -16777217L;
    is(l, 0xCB800000UL);
    c = 7;
    is((float)i / 4096, 0xC1000000UL);
    check(sizeof(float) == 4 && sizeof(double) == 4 && sizeof c == 4);
    c = 'A';
    is(c, 0x42820000UL);
    is(i + 0.5f, 0xC6FFFF00UL);

    /* 59-68: float to integers (toward zero) */
    c = -2.7f;
    i = c;
    check(i == -2);
    l = c;
    check(l == -2);
    c = 3e9;
    ul = c;
    check(ul == 3000000000UL);
    c = 65535.9f;
    u = c;
    check(u == 65535U);
    c = 200.9f;
    uc = c;
    check(uc == 200);
    c = -5.5f;
    sc = c;
    check(sc == -5);
    c = 123456792.0f;
    l = c;
    check(l == 123456792L);
    c = 0.999f;
    i = c;
    check(i == 0);
    check((int)-0.5f == 0);
    check((long)1e9f == 1000000000L);

    /* 69-72: structs, arrays, pointers */
    q = gp;
    q.y *= q.x;
    is(q.y, 0xC0000000UL);
    check(q.tag == 'p');
    is(sum(arr, 4), 0x41080000UL);
    p = &arr[1];
    *p = *p * 2;
    is(arr[1], 0x40A00000UL);

    /* 73-84: more constants (cc0), folding (cc1) */
    is(1.f, 0x3F800000UL);
    is(5., 0x40A00000UL);
    is(0e0, 0x00000000UL);
    is(0123.5, 0x42F70000UL);
    is(0x1.8p1, 0x40400000UL);
    is(0x1p-149, 0x00000001UL);
    is(2.5L, 0x40200000UL);
    c = 2;
    is(c-.5, 0x3FC00000UL);
    is(-.25, 0xBE800000UL);
    is(1.0f / 3.0f, 0x3EAAAAABUL);
    check(017 == 15 && 1.5f < 2 && !(0.0f / 0.0f == 0.0f / 0.0f));
    is(0.0f / 0.0f, 0x7FC00000UL);

    return fail;
}
