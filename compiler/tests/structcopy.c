/* Expected: main returns 0 (struct and union assignment, struct s v = w;) */
/* Returns the number of the first failed check. */

#include <string.h>

struct c1 { char a; };
struct c3 { char r, g, b; };
struct p { int x, y; };
struct s { int a; char b; long c; };
struct big { char t[300]; int last; };
struct in { struct p a, b; char name[5]; };
union u { long l; char c[4]; };

struct s gs = { 1, 2, 3 }, gt;
struct p pts[4] = { {1, 2}, {3, 4}, {5, 6}, {7, 8} };
struct big b1, b2;
int calls;

struct p *pick(int i)
{
    calls++;
    return &pts[i];
}

/* the copy goes through the pointers: the caller's structs change */
void swap(struct p *a, struct p *b)
{
    struct p t = *a;
    *a = *b;
    *b = t;
}

int main(void)
{
    struct c1 k1, k2;
    struct c3 m1, m2;
    struct p lp = pts[1], lq;
    struct s ls = gs, lt;
    struct in n1, n2;
    union u u1, u2;
    struct s *q = &gs;
    int i, v;

    /* sizes 1, 3, 4 (inline), 7 (helper) */
    k1.a = 'k';
    k2 = k1;
    if (k2.a != 'k')
        return 1;
    m1.r = 1; m1.g = 2; m1.b = 3;
    m2 = m1;
    if (m2.r != 1 || m2.g != 2 || m2.b != 3)
        return 2;
    if (lp.x != 3 || lp.y != 4)
        return 3;
    if (ls.a != 1 || ls.b != 2 || ls.c != 3)
        return 4;

    /* through pointers, globals and locals */
    gs.c = 0x12345678L;
    lt = *q;
    if (lt.c != 0x12345678L || lt.a != 1)
        return 5;
    gt = lt;
    gt.b = 9;
    if (lt.b != 2 || gt.b != 9 || gt.c != 0x12345678L)
        return 6;

    /* a = b = c, and the value of an assignment */
    lq = lp = pts[3];
    if (lq.x != 7 || lp.y != 8)
        return 7;
    if ((lq = pts[0]).y != 2 || lq.x != 1)
        return 8;

    /* arrays of structs, the address computed once */
    calls = 0;
    i = 0;
    pts[i++] = pts[3];
    if (i != 1 || pts[0].x != 7 || pts[0].y != 8)
        return 9;
    *pick(2) = *pick(1);
    if (calls != 2 || pts[2].x != 3 || pts[2].y != 4)
        return 10;
    swap(&pts[0], &pts[1]);
    if (pts[0].x != 3 || pts[1].x != 7 || pts[1].y != 8)
        return 11;

    /* structs and arrays in a struct */
    n1.a = pts[0];
    n1.b = pts[1];
    strcpy(n1.name, "abcd");
    n2 = n1;
    n2.b = n2.a;
    if (n2.a.x != 3 || n2.b.x != 3 || n1.b.x != 7 || strcmp(n2.name, "abcd"))
        return 12;

    /* unions */
    u1.l = 0x01020304L;
    u2 = u1;
    if (u2.c[0] != 1 || u2.c[3] != 4)
        return 13;

    /* more than 255 bytes */
    for (i = 0; i < 300; i++)
        b1.t[i] = i;
    b1.last = 1234;
    b2 = b1;
    for (i = 0; i < 300; i++)
        if (b2.t[i] != (char)i)
            return 14;
    if (b2.last != 1234)
        return 15;

    /* a field read before the copy is read again after it */
    v = lp.x;
    lp = pts[2];
    if (v != 7 || lp.x != 3)
        return 16;
    lp = lp;
    if (lp.x != 3 || lp.y != 4)
        return 17;
    return 0;
}
