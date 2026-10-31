/* Expected: main returns 0 (rand, srand, srand_rtc, rand_range) */
/* Returns the number of the first failed check. */

#include <stdlib.h>

static int first[20];
static char seen[16];

int sequences(void)
{
    int i, same;

    srand(1);
    for (i = 0; i < 20; i++) {
        first[i] = rand();
        if (first[i] < 0 || first[i] > RAND_MAX)
            return 1;
    }
    srand(1);                               /* same seed, same numbers */
    for (i = 0; i < 20; i++)
        if (rand() != first[i])
            return 2;
    srand(2);                               /* next seed: other numbers */
    same = 0;
    for (i = 0; i < 20; i++)
        if (rand() == first[i])
            same++;
    if (same > 2)
        return 3;
    srand(0);                               /* 0 is a seed like the others */
    if (rand() == rand() && rand() == rand())
        return 4;
    return 0;
}

int ranges(void)
{
    int i, n;

    /* -3..3: every value comes out, nothing else */
    for (i = 0; i < 16; i++)
        seen[i] = 0;
    for (i = 0; i < 300; i++) {
        n = rand_range(-3, 3);
        if (n < -3 || n > 3)
            return 10;
        seen[n + 3] = 1;
    }
    for (i = 0; i < 7; i++)
        if (!seen[i])
            return 11;
    if (rand_range(5, 5) != 5)
        return 12;
    if (rand_range(9, 4) != 9)              /* hi < lo: lo */
        return 13;
    seen[0] = seen[1] = 0;                  /* the whole int range */
    for (i = 0; i < 100; i++)
        seen[rand_range(-32767 - 1, 32767) < 0] = 1;
    if (!seen[0] || !seen[1])
        return 16;
    for (i = 0; i < 100; i++) {
        n = rand_range(1000, 1009);
        if (n < 1000 || n > 1009)
            return 14;
    }
    for (i = 0; i < 100; i++) {
        n = rand_range(-20000, -19990);
        if (n < -20000 || n > -19990)
            return 15;
    }
    return 0;
}

int main(void)
{
    int r;

    r = sequences();
    if (r)
        return r;
    r = ranges();
    if (r)
        return r;
    if (srand_rtc())                        /* the tests run with an RTC */
        return 20;
    r = rand();
    if (r < 0 || r > RAND_MAX)
        return 21;
    return 0;
}
