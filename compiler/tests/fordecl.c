/* Expected: main returns 77. C99: declarations in the first clause of for. */
struct pt { int x, y; };

int main(void)
{
    int r = 0;
    int i = 50;                         /* hidden by the loops' i */

    for (int i = 0; i < 4; i++)
        r += i;                         /* 6 */
    if (i != 50)
        return 1;

    for (int i = 0, j = 10; i < j; i++, j--)
        r++;                            /* + 5 = 11 */

    for (unsigned char c = 250; c != 2; c++)
        r++;                            /* + 8 = 19 (wraps at 256) */

    for (int i = 0; i < 2; i++) {
        int i = 20;                     /* the body is a block of its own */
        r += i;                         /* + 40 = 59 */
    }

    struct pt pts[3] = { { 1, 2 }, { 3, 4 }, { 5, 6 } };
    for (struct pt *p = pts; p < pts + 3; p++)
        r += p->y - p->x;               /* + 3 = 62 */

    for (int k = 0; ; k++) {            /* break and continue */
        if (k == 1)
            continue;
        if (k == 5)
            break;
        r += k;                         /* + 0 + 2 + 3 + 4 = 71 */
    }

    for (long big = 70000L; big < 70003L; big++)
        r += 2;                         /* + 6 = 77 */

    return i == 50 ? r : 2;
}
