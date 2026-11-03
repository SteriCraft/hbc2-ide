/* Expected: main returns 0 (sscanf, scanf) */
/* Returns the number of the first failed check. scanf.keys types "12 ab",
   "7" and "xy" (each followed by Enter). */

#include <stdio.h>
#include <string.h>

int main(void)
{
    int a, b, c, n, m;
    unsigned u;
    long l, l2;
    char s[20], t[20], ch[4];
    void *p;

    /* decimal, signs, spaces */
    a = b = c = 0;
    if (sscanf("  12 -34\t+5", "%d%d%d", &a, &b, &c) != 3 || a != 12 || b != -34 || c != 5)
        return 1;
    if (sscanf("65535 -1", "%u %d", &u, &a) != 2 || u != 65535u || a != -1)
        return 2;
    if (sscanf("2147483647 -2147483648", "%ld %ld", &l, &l2) != 2 || l != 2147483647L || l2 != -2147483647L - 1)
        return 3;
    if (sscanf("70000", "%ld", &l) != 1 || l != 70000L)
        return 4;
    if (sscanf("70000", "%d", &a) != 1 || a != (int)(70000L & 0xFFFF))
        return 5;                       /* too big: the low bits */
    if (sscanf("12", "%hd", &a) != 1 || a != 12)
        return 6;

    /* other bases */
    if (sscanf("ff 0x1A 0XbC 777 101", "%x %x %X %o %o", &a, &b, &c, &n, &m) != 5 ||
        a != 0xff || b != 0x1a || c != 0xbc || n != 0777 || m != 0101)
        return 7;
    if (sscanf("0x10 010 10 -0x10 0", "%i %i %i %i %i", &a, &b, &c, &n, &m) != 5 ||
        a != 16 || b != 8 || c != 10 || n != -16 || m != 0)
        return 8;
    if (sscanf("deadBEEF", "%lx", &l) != 1 || l != 0xdeadbeefL)
        return 9;
    if (sscanf("1234", "%p", &p) != 1 || p != (void *)0x1234)
        return 10;
    if (sscanf("789", "%o", &a) != 1 || a != 7)   /* stops at 8 */
        return 11;

    /* widths */
    if (sscanf("123456", "%2d%3d%d", &a, &b, &c) != 3 || a != 12 || b != 345 || c != 6)
        return 12;
    if (sscanf("abcdef", "%3s%s", s, t) != 2 || strcmp(s, "abc") || strcmp(t, "def"))
        return 13;

    /* strings, characters */
    if (sscanf("  hello world", "%s %s", s, t) != 2 || strcmp(s, "hello") || strcmp(t, "world"))
        return 14;
    memset(ch, '.', 4);
    if (sscanf(" xyz", "%c%2c", &ch[0], &ch[1]) != 2 || ch[0] != ' ' || ch[1] != 'x' || ch[2] != 'y' || ch[3] != '.')
        return 15;                      /* %c: no space skipped, no 0 */
    if (sscanf("a b", "%c %c", &ch[0], &ch[1]) != 2 || ch[0] != 'a' || ch[1] != 'b')
        return 16;

    /* sets */
    if (sscanf("abc123def", "%[a-z]%[0-9]%s", s, t, ch) != 3 || strcmp(s, "abc") || strcmp(t, "123") || strcmp(ch, "def"))
        return 17;
    if (sscanf("key = value;rest", "%[^ =] = %[^;]", s, t) != 2 || strcmp(s, "key") || strcmp(t, "value"))
        return 18;
    if (sscanf("]]-x", "%[]-]", s) != 1 || strcmp(s, "]]-"))
        return 19;
    if (sscanf("xyz", "%[0-9]", s) != 0)
        return 20;                      /* none: nothing stored */
    if (sscanf("aaab", "%2[a]", s) != 1 || strcmp(s, "aa"))
        return 21;

    /* literals, %%, *, %n */
    if (sscanf("12:34:56", "%d:%d:%d", &a, &b, &c) != 3 || a != 12 || b != 34 || c != 56)
        return 22;
    if (sscanf("12-34", "%d:%d", &a, &b) != 1 || a != 12)
        return 23;                      /* ':' expected: stops */
    if (sscanf("50 %", "%d %%", &a) != 1 || a != 50)
        return 24;
    if (sscanf("1 2 3", "%*d %d %*d", &a) != 1 || a != 2)
        return 25;
    if (sscanf("abc  42", "%*s%n %d%n", &n, &a, &m) != 1 || n != 3 || a != 42 || m != 7)
        return 26;                      /* %n is not counted */
    if (sscanf("ab", "ab%ln", &l) != 0 || l != 2)
        return 27;

    /* failures */
    if (sscanf("", "%d", &a) != EOF || sscanf("   ", "%s", s) != EOF)
        return 28;
    if (sscanf("x", "%d", &a) != 0)
        return 29;
    if (sscanf("-", "%d", &a) != 0 || sscanf("+x", "%d", &a) != 0)
        return 30;
    if (sscanf("1", "%d %d", &a, &b) != 1)
        return 31;                      /* the end after one field: 1, not EOF */
    if (sscanf("abc", "abd") != 0 || sscanf("", "") != 0)
        return 32;
    if (sscanf("5", "%d%", &a) != 1)
        return 33;

    /* the keyboard: "12 ab", "7", "xy" */
    if (scanf("%d %s", &a, s) != 2 || a != 12 || strcmp(s, "ab"))
        return 34;
    if (getchar() != '\n')              /* left by scanf */
        return 35;
    if (scanf("%d", &b) != 1 || b != 7) /* on the next line */
        return 36;
    if (scanf("%c", &ch[0]) != 1 || ch[0] != '\n')
        return 37;
    if (gets(t) != t || strcmp(t, "xy"))
        return 38;
    return 0;
}
