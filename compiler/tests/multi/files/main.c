/* Expected: main returns 0 (several files: shared names, statics, an assembly file) */
/* Returns the number of the first failed check. counter.c has a static
   count, a static twice() and a static n in a function too: each file
   keeps its own. */

#include "shared.h"

static int count = 100;
int total;

static int twice(int x)
{
    return 2 * x;
}

/* counter.c has a global half(): this file uses its own */
static int half(int x)
{
    return 100 * x;
}

static int calls(void)
{
    static int n = 50;
    return ++n;
}

int main(void)
{
    if (counter_next() != 1 || counter_next() != 2)
        return 1;
    if (count != 100)
        return 2;
    if (twice(5) != 10 || counter_triple(5) != 15)
        return 3;
    total = 7;
    if (counter_plus_total(1) != 8)
        return 4;
    if (table[2] != 30)
        return 5;
    if (add3(1000, 200, 34) != 1234)
        return 6;
    /* a static function, called from another file through a pointer */
    if (counter_apply(twice, 4) != 8)
        return 7;
    if (calls() != 51 || counter_calls() != 1 || calls() != 52 || counter_calls() != 2)
        return 8;
    if (half(4) != 400 || counter_half(4) != 2)
        return 9;
    return 0;
}
