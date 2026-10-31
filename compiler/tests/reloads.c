/* Expected: main returns 0
   Loads removed by the driver when the register already holds the value
   (hbc2-cc reloads.rs), and the cases where it must load again. */

int g;
int h;
unsigned char bytes[4];

int alias(int *p)
{
    int a, b;
    a = *p;         /* p points to g: [x:y] holds g */
    g = 5;          /* a store to g changes *p */
    b = *p;
    return a * 10 + b;
}

unsigned char pair[2];

/* a byte store leaves a (high byte of *p) in place */
int alias_byte(void)
{
    int *p = (int *)pair;
    int a, b;
    pair[0] = 1;
    pair[1] = 2;
    a = *p;
    pair[0] = 9;
    b = *p;
    return b - a;   /* 0x0902 - 0x0102 */
}

int alias_local(void)
{
    int v = 1;
    int *p = &v;
    int a = *p;
    v = 7;          /* a store to the stack changes *p too */
    return a * 10 + *p;
}

int through_pointer(int *p)
{
    int a = g;      /* a register holds g */
    *p = 9;         /* p points to g */
    return a * 10 + g;
}

int same_statement(void)
{
    h = 3;
    g = h;          /* h is still in the registers */
    h = h + g;
    return h;       /* 6 */
}

int bytes_test(void)
{
    unsigned char *p = bytes;
    p[0] = 1;
    p[1] = p[0] + 1;
    p[2] = p[1] + p[0];
    p++;
    p[2] = p[0] + p[1];     /* 2 + 3: the pointer moved */
    return bytes[3];
}

int main(void)
{
    g = 1;
    if (alias(&g) != 15) return 1;
    if (alias_local() != 17) return 2;
    g = 4;
    if (through_pointer(&g) != 49) return 3;
    if (same_statement() != 6) return 4;
    if (bytes_test() != 5) return 5;
    if (alias_byte() != 0x0800) return 6;
    return 0;
}
