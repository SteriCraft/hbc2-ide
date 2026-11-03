/* Expected: main returns 0 (sprintf, vsprintf, vprintf) */
/* Returns the number of the first failed check. The program's putchar
   replaces the library's: it keeps what printf and vprintf print, and can
   call sprintf (a sprintf inside a printf). */

#include <stdio.h>
#include <stdarg.h>
#include <string.h>

static char buf[64];
static char out[64];            /* what putchar got */
static unsigned out_len;
static char inner[8];
static int nest;                /* 1: putchar calls sprintf */

int putchar(int c)
{
    if (nest)
        sprintf(inner, "<%c>", c);
    if (out_len < sizeof out - 1)
        out[out_len++] = c;
    return c & 0xFF;
}

/* 1 when n is the length of t and buf holds t */
static int is(int n, const char *t)
{
    return n == strlen(t) && strcmp(buf, t) == 0;
}

/* 1 when n is the length of t and putchar got t; starts again */
static int printed(int n, const char *t)
{
    int ok = n == strlen(t) && out_len == strlen(t) && memcmp(out, t, out_len) == 0;
    out_len = 0;
    return ok;
}

static int my_sprintf(char *b, const char *fmt, ...)
{
    va_list ap;
    int n;
    va_start(ap, fmt);
    n = vsprintf(b, fmt, ap);
    va_end(ap);
    return n;
}

static int my_printf(const char *fmt, ...)
{
    va_list ap;
    int n;
    va_start(ap, fmt);
    n = vprintf(fmt, ap);
    va_end(ap);
    return n;
}

int main(void)
{
    /* sprintf: every format of printf.c */
    if (!is(sprintf(buf, "plain"), "plain"))
        return 1;
    if (!is(sprintf(buf, "%d %d %d|", 0, 42, -42), "0 42 -42|"))
        return 2;
    if (!is(sprintf(buf, "%d %d", 32767, (int)0x8000), "32767 -32768"))
        return 3;
    if (!is(sprintf(buf, "%u %x %x", 65535u, 0xBEEFu, 0), "65535 beef 0"))
        return 4;
    if (!is(sprintf(buf, "%ld %lu %lx", -123456789L, 4000000000UL, 0xDEADBEEFUL),
            "-123456789 4000000000 deadbeef"))
        return 5;
    if (!is(sprintf(buf, "[%5d][%05d][%05d][%2d]", 42, 42, -42, 12345),
            "[   42][00042][-0042][12345]"))
        return 6;
    if (!is(sprintf(buf, "[%04x][%8lx][%3s][%3c][%c]", 0xab, 0x1234L, "ab", 'z', 'y'),
            "[00ab][    1234][ ab][  z][y]"))
        return 7;
    if (!is(sprintf(buf, "100%% %s=%d%c", "x", 5, '!'), "100% x=5!"))
        return 8;
    if (!is(sprintf(buf, "%X %lX %04X|", 0xBEEFu, 0xABCDEF0UL, 0xa), "BEEF ABCDEF0 000A|"))
        return 9;
    if (!is(sprintf(buf, "[%-5s][%-5d][%-05d][%-2s][%-3c]", "ab", 42, -7, "long", 'z'),
            "[ab   ][42   ][-7   ][long][z  ]"))
        return 10;
    if (!is(sprintf(buf, "end%"), "end"))
        return 11;

    /* the 0 after the text, nothing written past it */
    memset(buf, 'z', sizeof buf);
    if (sprintf(buf, "ab") != 2 || buf[2] != 0 || buf[3] != 'z')
        return 12;
    memset(buf, 'z', sizeof buf);
    if (sprintf(buf, "") != 0 || buf[0] != 0 || buf[1] != 'z')
        return 13;
    if (sprintf(buf + 2, "%s", "cd") != 2 || buf[1] != 'z' || strcmp(buf + 2, "cd") != 0)
        return 14;

    /* sprintf of a text made by sprintf */
    sprintf(inner, "%03d", 7);
    if (!is(sprintf(buf, "[%s|%5s]", inner, inner), "[007|  007]"))
        return 15;

    /* vsprintf, through a function with ... */
    if (!is(my_sprintf(buf, "%s-%d-%lx-%c", "v", -1, 0x10000L, 'q'), "v--1-10000-q"))
        return 16;
    if (!is(my_sprintf(buf, "none"), "none"))
        return 17;

    /* printf and vprintf still go to putchar, nothing to buf */
    memset(buf, 'z', sizeof buf);
    out_len = 0;
    if (!printed(printf("p%d", 12), "p12") || buf[0] != 'z')
        return 20;
    if (!printed(my_printf("%s=%lu", "v", 70000UL), "v=70000") || buf[0] != 'z')
        return 21;

    /* sprintf inside printf: printf goes on to putchar after it */
    nest = 1;
    if (!printed(printf("ab%d", 5), "ab5") || strcmp(inner, "<5>") != 0)
        return 22;
    if (!printed(my_printf("%c%c", 'x', 'y'), "xy") || strcmp(inner, "<y>") != 0)
        return 23;
    nest = 0;
    if (!is(sprintf(buf, "%d", 99), "99") || out_len != 0)
        return 24;
    return 0;
}
