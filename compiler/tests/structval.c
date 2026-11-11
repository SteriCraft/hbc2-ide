/* Expected: main returns 100. Structs and unions passed and returned by value. */
#include <stdarg.h>

struct pt { int x, y; };                 /* 4 bytes: copied inline */
struct big { char name[9]; long n; };    /* 13 bytes: __structcpy */
struct one { char c; };
union u { long l; unsigned char b[4]; };

struct pt g = { 7, 8 };

struct pt make(int x, int y)
{
    struct pt p;
    p.x = x;
    p.y = y;
    return p;
}

struct pt add(struct pt a, struct pt b)
{
    a.x += b.x;                         /* a is the function's own copy */
    a.y += b.y;
    return a;
}

struct pt swap(struct pt a)
{
    struct pt r;
    r.x = a.y;
    r.y = a.x;
    return r;
}

/* the argument is copied when the function starts: changing the global
   afterwards does not change it */
int keeps_copy(struct pt a)
{
    g.x = 100;
    return a.x;
}

struct big grow(struct big b, int k)
{
    b.name[k] = 'z';
    b.n += k;
    return b;
}

/* recursion: each call has its own copy */
long depth(struct big b, int k)
{
    if (k == 0)
        return b.n;
    b.n += 10;
    return depth(b, k - 1) + b.n;
}

struct one next(struct one o) { o.c++; return o; }

union u flip(union u v)
{
    unsigned char t = v.b[0];
    v.b[0] = v.b[3];
    v.b[3] = t;
    return v;
}

/* variadic with a struct result: the arguments move up for the hidden
   pointer */
struct pt sum(int n, ...)
{
    va_list ap;
    struct pt r = { 0, 0 };
    va_start(ap, n);
    while (n--) {
        r.x += va_arg(ap, int);
        r.y++;
    }
    va_end(ap);
    return r;
}

int main(void)
{
    struct pt a, b;
    struct big bg, bg2;
    struct pt (*fp)(int, int) = make;
    struct one o;
    union u v;
    int i;

    a = make(1, 2);
    if (a.x != 1 || a.y != 2)
        return 1;
    b = add(a, make(10, 20));           /* a is not changed by add */
    if (a.x != 1 || b.x != 11 || b.y != 22)
        return 2;
    if (make(3, 4).y != 4)              /* member of a result */
        return 3;
    a = swap(a);                        /* the result goes to a temporary first */
    if (a.x != 2 || a.y != 1)
        return 4;
    if (add(make(1, 2), add(make(3, 4), make(5, 6))).y != 12)
        return 5;
    if (fp(5, 6).x != 5)                /* through a pointer */
        return 6;
    {
        struct pt c = make(7, 9);       /* initial value */
        if (c.y - c.x != 2)
            return 7;
    }
    if (keeps_copy(g) != 7 || g.x != 100)
        return 8;
    make(1, 1);                         /* result not used */
    (void)make(2, 2);
    i = (make(3, 3), 5);
    if (i != 5)
        return 9;

    for (i = 0; i < 9; i++)
        bg.name[i] = 'a' + i;
    bg.n = 70000L;
    bg2 = grow(bg, 3);
    if (bg.name[3] != 'd' || bg2.name[3] != 'z' || bg2.name[8] != 'i' ||
        bg2.n != 70003L)
        return 10;
    /* each level adds its own n after the deeper call: the deeper calls
       did not change it */
    if (depth(bg, 3) != 70010L + 70020L + 70030L + 70030L || bg.n != 70000L)
        return 11;

    o.c = 'A';
    o = next(next(o));
    if (o.c != 'C')
        return 12;

    v.l = 0x11223344L;
    v = flip(v);
    if (v.l != 0x44223311L)
        return 13;

    a = sum(3, 10, 20, 30);
    if (a.x != 60 || a.y != 3)
        return 14;

    a = b = make(40, 60);               /* chained */
    return a.x + b.y;                   /* 100 */
}
