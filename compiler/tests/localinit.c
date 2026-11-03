/* Expected: main returns 0 (initial values of local arrays and structs, (void) casts) */
/* Returns the number of the first failed check. */

#include <string.h>

struct p { int x, y; };
struct rec { char name[6]; struct p at; long id; };

int count;

int next(void)
{
    return ++count;
}

void bump(void)
{
    count += 10;
}

/* a new copy at each call, even in recursion */
int depth(int n)
{
    int t[3] = {1, 2, 3};
    int r;
    t[1] += n;
    r = n ? depth(n - 1) : 0;
    return r + t[1];             /* 2 + n, for every n */
}

int main(void)
{
    int a[3] = {10, 20, 30};
    char s1[] = "hello";
    char s2[8] = "hi";
    char s3[] = {'o', 'k', 0};
    long l[2] = {0x12345678L, -1L};
    struct p pt = {3, -4};
    struct rec r = {"bob", {7, 8}, 99999L};
    struct p ps[3] = {{1, 2}, {3, 4}};
    int grid[2][3] = {{1, 2, 3}, {4, 5, 6}};
    unsigned char one[1] = {200};
    int i, sum;

    if (a[0] != 10 || a[1] != 20 || a[2] != 30)
        return 1;
    if (sizeof s1 != 6 || strcmp(s1, "hello"))
        return 2;
    if (sizeof s2 != 8 || strcmp(s2, "hi") || s2[7] != 0)
        return 3;
    if (sizeof s3 != 3 || strcmp(s3, "ok"))
        return 4;
    if (l[0] != 0x12345678L || l[1] != -1L)
        return 5;
    if (pt.x != 3 || pt.y != -4)
        return 6;
    if (strcmp(r.name, "bob") || r.name[5] != 0 || r.at.x != 7 || r.at.y != 8 || r.id != 99999L)
        return 7;
    if (ps[1].y != 4 || ps[2].x != 0 || ps[2].y != 0)       /* the rest is 0 */
        return 8;
    if (grid[0][2] != 3 || grid[1][0] != 4)
        return 9;
    if (one[0] != 200)
        return 10;

    /* the values come back at each pass of the declaration */
    sum = 0;
    for (i = 0; i < 4; i++) {
        int t[2] = {5, 6};
        char w[] = "ab";
        sum += t[1] + w[0];
        t[1] = 100;
        w[0] = 'z';
    }
    if (sum != 4 * (6 + 'a'))
        return 11;
    if (depth(5) != 6 * 2 + 15)
        return 12;

    /* changing the copy leaves the next one alone */
    s1[0] = 'j';
    a[0] = 0;
    if (strcmp(s1, "jello") || a[0] != 0 || a[1] != 20)
        return 13;

    /* (void) */
    count = 0;
    (void)next();
    (void)bump();
    (void)count;
    (void)0;
    (void)pt;
    if (count != 11)
        return 14;
    return 0;
}
