/* Expected: main returns 604 (0x025C). Frames past [sp + 255], sign extension, &local. No helper. */
int big_frame(int base)
{
    char buf[300];
    int k;
    buf[0] = 1;
    buf[299] = 2;
    k = buf[299] + buf[0];          /* 3 */
    return k + base;                /* the argument is past 255 bytes too */
}

int sext(void)
{
    signed char c = -5;
    int x = c;                      /* -5 */
    long l = x;                     /* 0xFFFFFFFB */
    unsigned char *q = (unsigned char *)&l;
    return x + q[0] + q[3];         /* -5 + 255 + 251 = 501 */
}

int main(void)
{
    return big_frame(100) + sext(); /* 103 + 501 = 604 = 0x025C */
}
