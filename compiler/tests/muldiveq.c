/* Expected: main returns 100 */
/* x *= k, x /= k, x %= k with a constant: done in place when the
   multiply is short or k is a power of two (unsigned), else by the
   helpers. Every kind of target and the value of the expression. */

int g = 7;
unsigned ug = 1000;
unsigned long lg = 0x12345678;

struct s { int pad; int x; unsigned u; unsigned long l; };

int arg(int a, unsigned u)
{
    a *= 10;
    u /= 8;
    return a + u;            /* 50 + 12 */
}

int main(void)
{
    int x = 3, y;
    unsigned u = 1234;
    unsigned long l = 0xABCDEF01;
    char c = 20;
    static int st = 6;
    struct s v = { 0, 5, 77, 0x10000 };
    struct s *p = &v;
    int a[4] = { 1, 2, 3, 4 };
    int i = 2;

    x *= 10;                         /* 30: shifts and adds */
    if (x != 30) return 1;
    x *= 8;                          /* 240: power of two */
    if (x != 240) return 2;
    x *= -1;                         /* -240: 0xFFFF, helper */
    if (x != -240) return 3;
    x *= 7;                          /* -1680 */
    if (x != -1680) return 4;
    x *= 0;                          /* helper */
    if (x != 0) return 5;
    y = (x = 4, x *= 5);             /* the value of the expression */
    if (y != 20 || x != 20) return 6;

    g *= 3;                          /* global: 21 */
    if (g != 21) return 7;
    st *= 5;                         /* local static: 30 */
    if (st != 30) return 8;
    p->x *= 3;                       /* through a pointer: 15 */
    if (v.x != 15) return 9;
    a[i] *= 5;                       /* computed pointer: 15 */
    if (a[2] != 15 || a[1] != 2 || a[3] != 4) return 10;

    u /= 4;                          /* 308 */
    if (u != 308) return 11;
    u %= 8;                          /* 4 */
    if (u != 4) return 12;
    u = 1234;
    u /= 1;
    if (u != 1234) return 13;
    u %= 1;
    if (u != 0) return 14;
    u = 1000;
    u /= 10;                         /* helper */
    if (u != 100) return 15;
    ug /= 16;                        /* global: 62 */
    if (ug != 62) return 16;
    ug %= 16;                        /* 14 */
    if (ug != 14) return 17;
    p->u %= 32;                      /* 77 % 32 = 13 */
    if (v.u != 13) return 18;
    p->u /= 2;                       /* 6 */
    if (v.u != 6) return 19;
    if ((u /= 4) != 25) return 20;

    l /= 256;                        /* 0xABCDEF */
    if (l != 0xABCDEFUL) return 21;
    l %= 0x1000;                     /* 0xDEF */
    if (l != 0xDEFUL) return 22;
    l = 0xABCDEF01;
    l /= 2;                          /* long, not a multiple of 8: helper */
    if (l != 0x55E6F780UL) return 23;
    lg /= 0x10000;                   /* 0x1234 */
    if (lg != 0x1234UL) return 24;
    p->l /= 0x100;                   /* 0x100 */
    if (v.l != 0x100UL) return 25;
    p->l %= 0x80;                    /* 0 */
    if (v.l != 0) return 26;

    x = -7;
    x /= 4;                          /* signed: helper, -1 */
    if (x != -1) return 27;
    x = -7;
    x %= 4;                          /* -3 */
    if (x != -3) return 28;
    c *= 3;                          /* char: helper, 60 */
    if (c != 60) return 29;
    c /= 4;
    if (c != 15) return 30;

    if (arg(5, 100) != 62) return 31;
    return 100;
}
