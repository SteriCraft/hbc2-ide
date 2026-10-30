/* counter.c - the other C file of the program (see main.c) */

#include "shared.h"

static int count;
int table[3] = { 10, 20, 30 };

/* main.c has its own twice(), which doubles */
static int twice(int x)
{
    return 3 * x;
}

int counter_next(void)
{
    return ++count;
}

int counter_triple(int x)
{
    return twice(x);
}

int counter_plus_total(int x)
{
    return total + x;
}

int counter_apply(int (*f)(int), int x)
{
    return f(x);
}

/* global here, static in main.c */
int half(int x)
{
    return x / 2;
}

int counter_half(int x)
{
    return half(x);
}

int counter_calls(void)
{
    static int n;
    return ++n;
}
