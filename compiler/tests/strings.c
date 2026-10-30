/* Expected: main returns 0 (string.h) */
/* Returns the number of the first failed check. */

#include <string.h>
#include <stddef.h>

static char a[400];
static char b[400];

struct pair {
    char c;
    int n;
};

int lengths(void)
{
    if (strlen("") != 0 || strlen("abc") != 3)
        return 1;
    memset(a, 'x', 300);
    a[300] = 0;
    if (strlen(a) != 300)                   /* more than 255 */
        return 2;
    if (offsetof(struct pair, n) != 1)
        return 3;
    return 0;
}

int copies(void)
{
    if (strcpy(b, "hello") != b || strcmp(b, "hello"))
        return 10;
    /* strncpy: cut at n, or padded with zeros up to n */
    memset(b, '#', 10);
    if (strncpy(b, "abcdef", 3) != b || b[2] != 'c' || b[3] != '#')
        return 11;
    if (strncpy(b, "ab", 5) != b || b[1] != 'b' || b[2] || b[4] || b[5] != '#')
        return 12;
    strcpy(b, "foo");
    if (strcat(b, "bar") != b || strcmp(b, "foobar"))
        return 13;
    if (strncat(b, "bazzz", 3) != b || strcmp(b, "foobarbaz"))
        return 14;
    strncat(b, "!", 5);
    if (strcmp(b, "foobarbaz!"))
        return 15;
    return 0;
}

int compares(void)
{
    if (strcmp("abc", "abc") || strcmp("", ""))
        return 20;
    if (strcmp("abc", "abd") >= 0 || strcmp("abd", "abc") <= 0)
        return 21;
    if (strcmp("ab", "abc") >= 0 || strcmp("abc", "ab") <= 0)
        return 22;
    if (strcmp("\x80", "a") <= 0)           /* chars are unsigned */
        return 23;
    if (strncmp("abcx", "abcy", 3) || strncmp("abcx", "abcy", 4) >= 0)
        return 24;
    if (strncmp("a", "b", 0) || strncmp("ab", "ab", 10))
        return 25;
    if (memcmp("abc", "abd", 2) || memcmp("abc", "abd", 3) >= 0)
        return 26;
    if (memcmp("\xff", "\x01", 1) <= 0 || memcmp("x", "y", 0))
        return 27;
    return 0;
}

int searches(void)
{
    char *s = "hello world";
    if (strchr(s, 'o') != s + 4 || strrchr(s, 'o') != s + 7)
        return 30;
    if (strchr(s, 'z') != NULL || strrchr(s, 'z') != NULL)
        return 31;
    if (strchr(s, 0) != s + 11 || strrchr(s, 0) != s + 11)
        return 32;
    if (strchr(s, 'h') != s || strrchr(s, 'h') != s)
        return 33;
    if (memchr(s, 'w', 11) != s + 6 || memchr(s, 'w', 6) != NULL)
        return 34;
    if (strchr(s, 0x100 + 'e') != s + 1)    /* c is used as a char */
        return 35;
    return 0;
}

int blocks(void)
{
    int i;
    for (i = 0; i < 400; i++)
        a[i] = i;
    /* 300 bytes: more than 255 */
    if (memcpy(b, a, 300) != b || memcmp(a, b, 300))
        return 40;
    if (memset(b, 0x1AB, 5) != b || b[0] != (char)0xAB || b[4] != (char)0xAB || b[5] != 5)
        return 41;
    /* overlapping, forwards and backwards */
    for (i = 0; i < 400; i++)
        a[i] = i;
    if (memmove(a + 1, a, 299) != a + 1)
        return 42;
    if (a[0] != 0 || a[1] != 0 || a[2] != 1 || a[299] != (char)298 || a[300] != (char)300)
        return 43;
    for (i = 0; i < 400; i++)
        a[i] = i;
    memmove(a, a + 1, 299);
    if (a[0] != 1 || a[298] != (char)299 || a[299] != (char)299)
        return 44;
    memmove(a, a, 10);
    memset(a, 0, 0);
    memcpy(a, b, 0);
    if (a[0] != 1)
        return 45;
    return 0;
}

int main(void)
{
    int r;
    if ((r = lengths()) != 0)
        return r;
    if ((r = copies()) != 0)
        return r;
    if ((r = compares()) != 0)
        return r;
    if ((r = searches()) != 0)
        return r;
    return blocks();
}
