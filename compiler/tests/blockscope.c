/* Expected: main returns 123. Block scope: inner locals hide outer ones, sibling blocks share stack room. */
int g = 5;

int shadow_arg(int n)
{
    int r = n;                      /* n */
    {
        int n = 7;                  /* hides the argument */
        r += n;                     /* n + 7 */
    }
    return r + n;                   /* 2n + 7 */
}

int (*pick(int x))(int n)           /* argument names of a function type: their own scope */
{
    return x ? shadow_arg : 0;
}

/* Two sibling blocks of 40 bytes each: the frame holds one of them */
int siblings(void)
{
    int total = 0;
    {
        char a[40];
        int i;
        for (i = 0; i < 40; i++)
            a[i] = 1;
        for (i = 0; i < 40; i++)
            total += a[i];          /* 40 */
    }
    {
        char b[40];
        int i;
        for (i = 0; i < 40; i++)
            b[i] = 2;
        for (i = 0; i < 40; i++)
            total += b[i];          /* + 80 = 120 */
    }
    return total;
}

int frame_probe(void)
{
    char here;
    return (int)&here;
}

/* The stack frame of siblings() must be 40 bytes of arrays, not 80:
   compare the address of a local in a function called from inside
   each version. */
int depth_one(void)
{
    char a[40];
    a[0] = 0;
    return frame_probe() + a[0];
}

int depth_sibling(void)
{
    {
        char a[40];
        a[0] = 0;
    }
    {
        char b[40];
        b[0] = 0;
        return frame_probe() + b[0];
    }
}

int main(void)
{
    int r = 0;
    int x = 1;
    {
        int x = 10;
        {
            int x = 100;
            r += x;                 /* 100 */
        }
        r += x;                     /* 110 */
        int g = 1000;               /* hides the global */
        g = 0;
    }
    r += x;                         /* 111 */
    r += g;                         /* 116 (the global) */
    if (shadow_arg(1) != 9)
        return 1;
    if (pick(1)(2) != 11)
        return 2;
    if (siblings() != 120)
        return 3;
    int p1 = depth_one();           /* called at the same stack depth */
    int p2 = depth_sibling();
    if (p1 != p2)
        return 4;
    {
        static int count;           /* a static in a block */
        int k;
        for (k = 0; k < 7; k++)
            count++;
        r += count;                 /* 123 */
    }
    return r;
}
