/* Expected: main returns 0
   Bit tests in conditions ("x & k", k with one non-zero byte: only that
   byte is tested) and <<= / >>= by a constant done in place. */

unsigned int gu = 0x8001;
long gl = 0x40000000L;
static unsigned char sc = 0x81;
int gi = -32768;

struct rec {
    unsigned char tag;
    unsigned int word;
    long big;
};

/* Each test gives 1 when the condition is true */
int top(unsigned int x) { return (x & 0x8000) ? 1 : 0; }
int low(unsigned int x) { if (x & 0x01) return 1; return 0; }
int full(unsigned int x) { if (x & 0xFF00) return 1; return 0; }
int eq0(unsigned int x) { return (x & 0x0400) == 0; }
int ne0(unsigned int x) { return (x & 0x0400) != 0; }
int lbyte(long x, int n)
{
    switch (n) {
    case 0: return (x & 0x80000000L) ? 1 : 0;
    case 1: return (x & 0x00010000L) ? 1 : 0;
    case 2: return (x & 0x00000200L) ? 1 : 0;
    }
    return (x & 0x00000004L) ? 1 : 0;
}
int cbit(unsigned char c) { return (c & 0x80) ? 1 : 0; }
int cff(unsigned char c) { return (c & 0xFF) ? 1 : 0; }
int both(unsigned int x) { return (x & 0x8001) ? 1 : 0; }     /* two bytes: not the shortcut */
int expr(unsigned int a, unsigned int b) { return ((a + b) & 0x0100) ? 1 : 0; }
int zext(unsigned char c) { return ((unsigned int)c & 0x0100) ? 1 : 0; }

int and_chain(unsigned int x, unsigned int y)
{
    if ((x & 0x8000) && !(y & 0x0001))
        return 1;
    return 0;
}

int test_bits(void)
{
    unsigned int local = 0x0400;
    struct rec r;
    struct rec *p = &r;
    if (top(0x8000) != 1 || top(0x7FFF) != 0) return 1;
    if (low(3) != 1 || low(2) != 0) return 2;
    if (full(0x0100) != 1 || full(0x00FF) != 0) return 3;
    if (eq0(0x0400) != 0 || eq0(0xFBFF) != 1) return 4;
    if (ne0(0x0400) != 1 || ne0(0xFBFF) != 0) return 5;
    if (lbyte(0x80000000L, 0) != 1 || lbyte(0x7FFFFFFFL, 0) != 0) return 6;
    if (lbyte(0x00010000L, 1) != 1 || lbyte(0xFFFEFFFFL, 1) != 0) return 7;
    if (lbyte(0x00000200L, 2) != 1 || lbyte(0xFFFFFDFFL, 2) != 0) return 8;
    if (lbyte(0x00000004L, 3) != 1 || lbyte(0xFFFFFFFBL, 3) != 0) return 9;
    if (cbit(0x80) != 1 || cbit(0x7F) != 0) return 10;
    if (cff(1) != 1 || cff(0) != 0) return 11;
    if (both(0x0001) != 1 || both(0x8000) != 1 || both(0x7FFE) != 0) return 12;
    if (expr(0x00FF, 1) != 1 || expr(0x00FE, 1) != 0) return 13;
    if (zext(0xFF) != 0) return 14;
    if (and_chain(0x8000, 2) != 1 || and_chain(0x8000, 1) != 0 || and_chain(1, 2) != 0) return 15;
    /* variables of each kind */
    if (!(gu & 0x8000) || (gu & 0x4000)) return 16;
    if (!(gl & 0x40000000L) || (gl & 0x00000001L)) return 17;
    if (!(sc & 0x80) || (sc & 0x40)) return 18;
    if (!(local & 0x0400) || (local & 0x0800)) return 19;
    p->word = 0x2000;
    p->big = 0x00800000L;
    if (!(p->word & 0x2000) || (p->word & 0x0020)) return 20;
    if (!(p->big & 0x00800000L) || (p->big & 0x01000000L)) return 21;
    if (!(gi & 0x8000)) return 22;
    /* value context: the whole value */
    if ((gu & 0x8000) != 0x8000) return 23;
    /* 0 / 1 values */
    {
        int t = (gu & 0x8000) != 0;
        int f = (gu & 0x4000) != 0;
        int z = (gu & 0x4000) == 0;
        int b = !!(gu & 0x0001);
        long w = 0x00010000L;
        int lt = (local < 0x0500) ? 7 : 9;
        int ge = (local >= 0x0500) ? 7 : 9;
        if (t != 1 || f != 0 || z != 1 || b != 1) return 24;
        if (((w & 0x00010000L) ? 5 : 6) != 5) return 25;
        if (lt != 7 || ge != 9) return 26;
    }
    return 0;
}

int test_shifts(void)
{
    unsigned int u = 0x1234;
    int s = -256;
    unsigned char c = 0x81;
    signed char sch = -64;
    long l = 0x12345678L;
    unsigned long ul = 0x80000001UL;
    long sl = -65536L;
    struct rec r;
    struct rec *p = &r;
    unsigned int v;

    u <<= 4;
    if (u != 0x2340) return 31;
    u >>= 8;
    if (u != 0x0023) return 32;
    u <<= 0;
    if (u != 0x0023) return 33;
    s >>= 4;                        /* signed: sign kept */
    if (s != -16) return 34;
    s <<= 1;
    if (s != -32) return 35;
    c <<= 1;
    if (c != 0x02) return 36;
    c = 0x81;
    c >>= 7;
    if (c != 1) return 37;
    sch >>= 2;
    if (sch != -16) return 38;
    l <<= 8;
    if (l != 0x34567800L) return 39;
    l <<= 1;
    if (l != 0x68ACF000L) return 40;
    ul >>= 16;
    if (ul != 0x8000UL) return 41;
    sl >>= 8;
    if (sl != -256L) return 42;
    gu <<= 1;                       /* global, 0x8001 */
    if (gu != 0x0002) return 43;
    sc >>= 4;                       /* static, 0x81 */
    if (sc != 0x08) return 44;
    gl >>= 24;                      /* 0x40000000 */
    if (gl != 0x40) return 45;
    p->word = 0x0101;
    p->word <<= 3;                  /* through a pointer */
    if (r.word != 0x0808) return 46;
    p->tag = 0x90;
    p->tag >>= 4;
    if (r.tag != 0x09) return 47;
    p->big = 1;
    p->big <<= 1;
    if (r.big != 2) return 48;
    v = (u <<= 2);                  /* the value of the assignment */
    if (v != 0x008C || u != 0x008C) return 49;
    u = 0xFFFF;
    u >>= 15;
    if (u != 1) return 50;
    u = 1;
    u <<= 15;
    if (u != 0x8000) return 51;
    return 0;
}

int main(void)
{
    int e = test_bits();
    if (e)
        return e;
    return test_shifts();
}
