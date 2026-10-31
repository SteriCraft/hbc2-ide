/* Expected: main returns 142. C99: declarations after statements, in any block. */
int nested(int n)
{
    int total = 0;
    total += n;                     /* n */
    int twice = n * 2;
    total += twice;                 /* 3n */
    {
        total++;
        int inner = total;          /* 3n + 1 */
        total += inner;             /* 6n + 2 */
    }
    return total;
}

int main(void)
{
    int a = 1;
    a = a + 1;                      /* 2 */
    int length = -1;
    a += nested(5);                 /* 2 + 32 = 34 */
    int i;
    for (i = 0; i < 3; i++) {
        a += i;
        int sq = i * i;
        a += sq;                    /* + 0, + 2, + 6 = 42 */
    }
    if (length < 0)
        a += 100;                   /* 142 */
    return a;
}
