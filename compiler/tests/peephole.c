/* Expected: main returns 0. Code shapes for the peephole rules of
   fcc/rules.hbc2 (jumps over labels, stores then loads, the left operand
   of a helper); returns the number of the first wrong result. */

int g, h;
long gl;
struct pair { int a, b; } gp;
char buf[8];

int classify(int a, int b)
{
    int r = 0;
    if (a) {
        if (b)
            r = 1;
    } else if (b)
        r = 2;
    else
        r = 3;
    return r;
}

int first_zero(char *s)
{
    int i;
    for (i = 0; i < 8; i++) {
        if (s[i] == 0)
            break;
        if (s[i] == 1)
            continue;
    }
    return i;
}

int deep(int a, int b, int c)
{
    while (a) {
        if (b) {
            if (c)
                return 7;
        }
        a--;
    }
    return 0;
}

long longs(long x)
{
    long t, u;
    t = x + 1;
    gl = t;
    u = gl * 3;
    return u - t + gl;
}

int fields(int v)
{
    gp.b = v;
    h = gp.b * 5;
    gp.a = h;
    return gp.a / 3;
}

int helpers(int *p, int k)
{
    int r = (int) (p - &g) * 7;
    r += k % (int) &buf[3];
    r += g * h;
    r += (unsigned) k / 3u;
    return r;
}

int main(void)
{
    static char s[8] = { 5, 1, 1, 4, 0, 6, 7, 8 };
    if (classify(1, 1) != 1 || classify(1, 0) != 0 || classify(0, 1) != 2 || classify(0, 0) != 3)
        return 1;
    if (first_zero(s) != 4)
        return 2;
    if (deep(3, 1, 1) != 7 || deep(3, 1, 0) != 0 || deep(0, 1, 1) != 0)
        return 3;
    if (longs(99999L) != 300000L || gl != 100000L)
        return 4;
    if (fields(9) != 15 || h != 45)
        return 5;
    g = 6;
    h = 7;
    if (helpers(&g, 100) != 42 + 100 % (int) &buf[3] + 33)
        return 6;
    return 0;
}
