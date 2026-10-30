/* Expected: main returns 1196 (0x04AC). Uses no runtime helper. */
struct point { int x; char tag; int y; };
struct point pts[3];
int total;
char msg[] = "Hi!";
char *names[] = { "zero", "one" };
long big = 0x12345678;

int add(int a, int b) { return a + b; }

int fib(int n)
{
    if (n == 0 || n == 1)
        return n;
    return add(fib(n - 1), fib(n - 2));
}

void fill(struct point *p, int x, int y, char tag)
{
    p->x = x;
    p->y = y;
    p->tag = tag;
}

int length(char *s)
{
    int n = 0;
    while (*s != 0) {
        s = s + 1;
        n = n + 1;
    }
    return n;
}

int main(void)
{
    int i = 0;
    struct point *p = pts;
    int (*fp)(int, int) = add;
    while (i != 3) {
        fill(p, i + 10, i + 20, 'a' + i);
        p = p + 1;
        i = i + 1;
    }
    total = pts[0].x + pts[1].y + pts[2].tag;   /* 10 + 21 + 'c' = 130 */
    total = total + fp(1000, 7);                /* 1137 */
    total = total + fib(10);                    /* 1192 */
    total = total + length(msg) + length(names[1]); /* 1198 */
    total = total - (~0 & 5) - -2;              /* 1195 */
    if (!((int)big != 0x5678) && (char)big == 0x78)
        total = total + 1;                      /* 1196 = 0x04AC */
    return total;
}
