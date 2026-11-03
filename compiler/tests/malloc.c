/* Expected: main returns 0 (malloc, free, calloc, realloc) */
/* Returns the number of the first failed check. */

#include <stdlib.h>
#include <string.h>

#define SLOTS 40

static char *slot[SLOTS];
static unsigned len[SLOTS];
static char tag[SLOTS];

#define BIG 70
static char *big[BIG];

/* fills p with n bytes from t on */
static void fill(char *p, unsigned n, char t)
{
    unsigned i;
    for (i = 0; i < n; i++)
        p[i] = t + i;
}

static int same(char *p, unsigned n, char t)
{
    unsigned i;
    for (i = 0; i < n; i++)
        if (p[i] != (char)(t + i))
            return 0;
    return 1;
}

/* every live slot still holds its bytes */
static int all_good(void)
{
    int i;
    for (i = 0; i < SLOTS; i++)
        if (slot[i] && !same(slot[i], len[i], tag[i]))
            return 0;
    return 1;
}

int main(void)
{
    char *a, *b, *c, *d, *e;
    int *t;
    unsigned i, n, total;
    int k, round;

    /* blocks apart from each other */
    a = malloc(10);
    b = malloc(20);
    c = malloc(1);
    if (!a || !b || !c)
        return 1;
    if (b < a + 10 || c < b + 20)
        return 2;
    fill(a, 10, 1);
    fill(b, 20, 50);
    fill(c, 1, 99);
    if (!same(a, 10, 1) || !same(b, 20, 50) || !same(c, 1, 99))
        return 3;

    /* a freed block is used again */
    free(b);
    d = malloc(20);
    if (d != b || !same(a, 10, 1) || !same(c, 1, 99))
        return 4;
    free(d);
    d = malloc(8);                      /* first fit: in b's place, the rest stays free */
    e = malloc(8);
    if (d != b || e != b + 10)          /* 8 bytes + a 2-byte header */
        return 5;
    free(d);
    free(e);
    d = malloc(20);                     /* the two halves joined again */
    if (d != b)
        return 6;

    /* free(NULL), malloc(0) */
    free(NULL);
    e = malloc(0);
    if (!e || e == a || e == c || e == d)
        return 7;
    free(e);

    /* calloc */
    t = calloc(50, sizeof(int));
    if (!t)
        return 8;
    for (i = 0; i < 50; i++)
        if (t[i])
            return 9;
    t[49] = 7;
    free(t);
    t = calloc(50, sizeof(int));        /* the same block: set to 0 again */
    if (!t || t[49] != 0)
        return 10;
    if (calloc(300, 300) || calloc(2, 32768u))
        return 11;                      /* more than 65535 bytes */
    free(t);

    /* too big */
    if (malloc(65535u) || malloc(65534u) || malloc(60000u))
        return 12;

    /* realloc */
    a = realloc(a, 4);                  /* shrinking: in place */
    if (!a || !same(a, 4, 1))
        return 13;
    b = malloc(4);                      /* in the 6 bytes a gave back */
    if (b != a + 6)
        return 14;
    free(b);
    free(d);
    b = realloc(a, 30);                 /* grows into the free blocks after it */
    if (b != a || !same(b, 4, 1))
        return 15;
    fill(b, 30, 3);
    e = malloc(5);                      /* the last block */
    fill(e, 5, 40);
    a = realloc(e, 200);                /* grows at the top */
    if (a != e || !same(a, 5, 40))
        return 16;
    c = realloc(b, 60);                 /* b can't grow there: it moves */
    if (!c || c == b || !same(c, 30, 3) || !same(a, 5, 40))
        return 17;
    d = malloc(30);                     /* b's place is free */
    if (d != b)
        return 18;
    if (realloc(NULL, 3) == NULL)
        return 19;
    if (realloc(d, 0) != NULL)
        return 20;
    if (realloc(c, 60000u) != NULL || !same(c, 30, 3))
        return 21;                      /* failure: c is kept */

    /* all the memory, then all given back */
    free(a);
    free(c);
    n = 0;
    while (n < BIG && (e = malloc(1000)) != NULL) {
        if (e + 1000 > (char *)&k - 256)
            return 32;                  /* 256 bytes kept below the stack */
        memset(e, n, 1000);
        big[n++] = e;
    }
    if (n < 10 || n == BIG)             /* 10 KB of heap or more, less than 64 KB */
        return 22;
    if (malloc(1000))
        return 23;
    for (i = 0; i < n; i++)
        if (big[i][0] != (char)i || big[i][999] != (char)i)
            return 24;
    for (i = 0; i < n; i++)
        free(big[i]);
    total = n * 1000u;
    e = malloc(total);                  /* all joined into one block */
    if (!e)
        return 25;
    free(e);

    /* many blocks of random sizes, freed and grown in any order */
    srand(7);
    for (round = 0; round < 600; round++) {
        k = rand() % SLOTS;
        if (slot[k]) {
            if (rand() & 1) {
                free(slot[k]);
                slot[k] = NULL;
            } else {
                n = rand() % 300;
                e = realloc(slot[k], n);
                if (n == 0) {
                    if (e)
                        return 26;
                    slot[k] = NULL;
                } else if (e) {
                    if (!same(e, n < len[k] ? n : len[k], tag[k]))
                        return 27;
                    slot[k] = e;
                    len[k] = n;
                    fill(e, n, tag[k]);
                }
            }
        } else {
            n = rand() % 300;
            slot[k] = malloc(n);
            if (!slot[k])
                return 29;
            len[k] = n;
            tag[k] = rand();
            fill(slot[k], n, tag[k]);
        }
        if (round % 50 == 0 && !all_good())
            return 28;
    }
    if (!all_good())
        return 30;
    for (i = 0; i < SLOTS; i++)
        free(slot[i]);
    e = malloc(total);                  /* everything came back */
    if (!e)
        return 31;
    return 0;
}
