/* Expected: main returns 0 (qsort, bsearch, strspn, strcspn, strpbrk, strtok) */
/* Returns the number of the first failed check. */

#include <stdlib.h>
#include <string.h>

#define N 300

static int a[N];
static int *lo, *hi;            /* cmp_int checks its pointers are in lo..hi */
static int bad;                 /* 1 when cmp_int got a wrong pointer */
static unsigned calls;

static int cmp_int(const void *p, const void *q)
{
    const int *x = p, *y = q;
    calls++;
    if (x < lo || x >= hi || y < lo || y >= hi || ((char *)x - (char *)lo) % 2 || ((char *)y - (char *)lo) % 2)
        bad = 1;
    return *x < *y ? -1 : *x > *y;
}

/* bsearch: key first, then an element of the table */
static int cmp_key(const void *k, const void *e)
{
    int x = *(const int *)k, y = *(const int *)e;
    if ((const int *)e < lo || (const int *)e >= hi)
        bad = 1;
    return x < y ? -1 : x > y;
}

/* 5-byte records, sorted on a long; the char says where they were */
struct rec {
    long key;
    unsigned char at;
};
static struct rec r[40];

static int cmp_rec(const void *p, const void *q)
{
    long x = ((const struct rec *)p)->key, y = ((const struct rec *)q)->key;
    return x < y ? -1 : x > y;
}

static char *words[6];
static int cmp_str(const void *p, const void *q)
{
    return strcmp(*(char *const *)p, *(char *const *)q);
}

/* sorts the n ints of a; 1 when they end in order, cmp_int saw no wrong pointer */
static int sort(int n)
{
    int i;
    lo = a;
    hi = a + n;
    bad = 0;
    qsort(a, n, sizeof(int), cmp_int);
    for (i = 1; i < n; i++)
        if (a[i - 1] > a[i])
            return 0;
    return !bad;
}

int main(void)
{
    int i, j, n, key, *p;
    unsigned count[10];
    char text[40];
    char *t;

    /* 0, 1, 2, 3 elements */
    a[0] = 5;
    calls = 0;
    if (!sort(0) || !sort(1) || calls != 0 || a[0] != 5)
        return 1;
    a[0] = 2; a[1] = 1;
    if (!sort(2) || a[0] != 1 || a[1] != 2)
        return 2;
    a[0] = 3; a[1] = 1; a[2] = 2;
    if (!sort(3) || a[0] != 1 || a[2] != 3)
        return 3;

    /* a permutation of 0..n-1: sorted, each value at its index */
    for (n = 4; n <= N; n += n < 20 ? 1 : 37) {
        for (i = 0; i < n; i++)
            a[i] = i;
        for (i = n - 1; i > 0; i--) {
            j = rand() % (i + 1);
            key = a[i]; a[i] = a[j]; a[j] = key;
        }
        if (!sort(n))
            return 4;
        for (i = 0; i < n; i++)
            if (a[i] != i)
                return 5;
    }

    /* already sorted, reversed, all equal, negatives */
    for (i = 0; i < N; i++)
        a[i] = i;
    if (!sort(N) || a[0] != 0 || a[N - 1] != N - 1)
        return 6;
    for (i = 0; i < N; i++)
        a[i] = N - i;
    if (!sort(N) || a[0] != 1 || a[N - 1] != N)
        return 7;
    for (i = 0; i < N; i++)
        a[i] = 7;
    if (!sort(N) || a[0] != 7 || a[N - 1] != 7)
        return 8;
    for (i = 0; i < N; i++)
        a[i] = (i * 97) % 201 - 100;     /* -100..100, 30000 and -30000 too */
    a[3] = 30000;
    a[4] = -30000;
    if (!sort(N) || a[0] != -30000 || a[N - 1] != 30000)
        return 9;

    /* many equal values: the same count of each */
    for (i = 0; i < 10; i++)
        count[i] = 0;
    for (i = 0; i < N; i++) {
        a[i] = rand() % 10;
        count[a[i]]++;
    }
    if (!sort(N))
        return 10;
    for (i = 0, j = 0; i < 10; i++)
        while (count[i]--)
            if (a[j++] != i)
                return 11;

    /* bsearch: every element of 0, 2, 4... found, the odd ones not */
    for (n = 0; n <= 40; n++) {
        for (i = 0; i < n; i++)
            a[i] = 2 * i;
        lo = a;
        hi = a + n;
        bad = 0;
        for (key = -1; key <= 2 * n; key++) {
            p = bsearch(&key, a, n, sizeof(int), cmp_key);
            if (key >= 0 && key % 2 == 0 && key < 2 * n) {
                if (p != a + key / 2)
                    return 12;
            } else if (p != NULL)
                return 13;
        }
        if (bad)
            return 14;
    }
    for (i = 0; i < N; i++)
        a[i] = i * 3;
    lo = a;
    hi = a + N;
    calls = 0;
    key = 3 * 123;
    if (bsearch(&key, a, N, sizeof(int), cmp_key) != a + 123)
        return 15;
    key = 4;
    if (bsearch(&key, a, N, sizeof(int), cmp_key) != NULL)
        return 16;

    /* records of 5 bytes, on a long key */
    for (i = 0; i < 40; i++) {
        r[i].key = (long)((i * 13) % 40) * 100000L - 2000000L;
        r[i].at = i;
    }
    qsort(r, 40, sizeof r[0], cmp_rec);
    for (i = 0; i < 40; i++)
        if (r[i].key != (long)i * 100000L - 2000000L || (r[i].at * 13) % 40 != i)
            return 17;

    /* pointers to strings */
    words[0] = "pear"; words[1] = "apple"; words[2] = "fig";
    words[3] = "banana"; words[4] = "apricot"; words[5] = "a";
    qsort(words, 6, sizeof words[0], cmp_str);
    if (strcmp(words[0], "a") || strcmp(words[1], "apple") || strcmp(words[2], "apricot")
        || strcmp(words[3], "banana") || strcmp(words[4], "fig") || strcmp(words[5], "pear"))
        return 18;
    t = "fig";
    if (bsearch(&t, words, 6, sizeof words[0], cmp_str) != &words[4])
        return 19;

    /* strspn, strcspn */
    if (strspn("  \tab", " \t") != 3 || strspn("abc", "") != 0 || strspn("", "ab") != 0)
        return 20;
    if (strspn("abcabd", "abc") != 5 || strspn("aaa", "a") != 3)
        return 21;
    if (strcspn("hello, world", ", ") != 5 || strcspn("abc", "") != 3 || strcspn("", "a") != 0)
        return 22;
    if (strcspn("abc", "xyc") != 2 || strcspn("abc", "a") != 0)
        return 23;
    t = "\x80\xFF z";                    /* characters above 127 */
    if (strspn(t, "\xFF\x80") != 2 || strcspn(t, "\xFF") != 1)
        return 24;

    /* strpbrk */
    t = "key=value;x";
    if (strpbrk(t, "=;") != t + 3 || strpbrk(t, ";") != t + 9 || strpbrk(t, "#") != NULL)
        return 25;
    if (strpbrk(t, "") != NULL || strpbrk("", "a") != NULL || strpbrk(t, "k") != t)
        return 26;

    /* strtok */
    if (strtok(NULL, " ") != NULL)       /* no text given yet */
        return 27;
    strcpy(text, "  move  10,20 ;go");
    if ((t = strtok(text, " ")) != text + 2 || strcmp(t, "move"))
        return 28;
    if ((t = strtok(NULL, ", ")) != text + 8 || strcmp(t, "10"))
        return 29;
    if ((t = strtok(NULL, ",; ")) != text + 11 || strcmp(t, "20"))
        return 30;
    if ((t = strtok(NULL, " ;")) != text + 15 || strcmp(t, "go"))
        return 31;
    if (strtok(NULL, " ") != NULL || strtok(NULL, " ") != NULL)
        return 32;
    if (memcmp(text, "  move\0 10\00020\0;go", 18))
        return 33;
    strcpy(text, " ,, ");
    if (strtok(text, " ,") != NULL || strtok(NULL, " ,") != NULL)
        return 34;
    strcpy(text, "one");
    if (strtok(text, "") != text || strtok(NULL, "") != NULL)
        return 35;
    strcpy(text, "a b");
    if (strtok(text, " ") != text || strtok(text + 2, " ") != text + 2 || strtok(NULL, " ") != NULL)
        return 36;
    return 0;
}
