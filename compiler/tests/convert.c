/* Expected: main returns 0 (atoi, atol (decimal and 0x hex), itoa, utoa, ltoa, ultoa, htoa, lhtoa) */
/* Returns the number of the first failed check. */

#include <stdlib.h>
#include <string.h>

static char buf[12];

/* 1 when the text at buf is t and end is on its 0 */
static int is(char *end, const char *t)
{
    return strcmp(buf, t) == 0 && end == buf + strlen(t);
}

int main(void)
{
    if (atoi("0") != 0 || atoi("42") != 42 || atoi("-42") != -42)
        return 1;
    if (atoi("+7") != 7 || atoi("32767") != 32767 || atoi("-32768") != (int)0x8000)
        return 2;
    if (atoi(" \t\n\v\f\r 12x") != 12)          /* spaces before, stops at x */
        return 3;
    if (atoi("") != 0 || atoi("x1") != 0 || atoi("-") != 0 || atoi("- 1") != 0)
        return 4;
    if (atoi("65537") != 1)                     /* the low 16 bits */
        return 5;
    if (atol("2147483647") != 2147483647L || atol("-2147483648") != (long)0x80000000UL)
        return 6;
    if (atol("  -123456789;") != -123456789L || atol("00000000000000000099") != 99)
        return 7;
    if (atol("2147483648") != 0 || atol("-2147483649") != 0 || atol("99999999999") != 0)
        return 8;                               /* does not fit: 0 */

    if (!is(itoa(0, buf), "0") || !is(itoa(-1, buf), "-1"))
        return 9;
    if (!is(itoa(32767, buf), "32767") || !is(itoa((int)0x8000, buf), "-32768"))
        return 10;
    if (!is(utoa(65535u, buf), "65535") || !is(utoa(100, buf), "100"))
        return 11;
    if (!is(ltoa(0, buf), "0") || !is(ltoa(-2147483647L - 1, buf), "-2147483648"))
        return 12;
    if (!is(ltoa(1000000000L, buf), "1000000000") || !is(ltoa(-10, buf), "-10"))
        return 13;
    if (!is(ultoa(4294967295UL, buf), "4294967295") || !is(ultoa(65536UL, buf), "65536"))
        return 14;
    if (!is(htoa(0xBEEFu, buf), "BEEF") || !is(htoa(0x0A, buf), "000A"))
        return 15;
    if (!is(lhtoa(0xDEADBEEFUL, buf), "DEADBEEF") || !is(lhtoa(0x1234UL, buf), "00001234"))
        return 16;
    if (atol(ltoa(-98765L, buf) - 6) != -98765L)    /* back and forth */
        return 17;
    if (atoi("0x1F") != 31 || atoi("0Xff") != 255 || atoi(" -0x10") != -16)
        return 18;
    if (atoi("0xFFFF") != -1 || atoi("+0xaBcD") != (int)0xABCD || atoi("0x12g") != 0x12)
        return 19;
    if (atoi("0x") != 0 || atoi("0xg") != 0 || atoi("0 x1") != 0 || atoi("x1F") != 0)
        return 20;                              /* no digit after 0x: the 0 */
    if (atol("0xDEADBEEF") != (long)0xDEADBEEFUL || atol("0xFFFFFFFF") != -1L)
        return 21;
    if (atol("0x000000012345678") != 0x12345678L || atol("0x100000000") != 0)
        return 22;                              /* past 32 bits: 0 */
    if (atoi("0x09") != 9 || atoi("0x@") != 0 || atoi("0x:") != 0 || atoi("0x`G") != 0)
        return 23;                              /* the characters around the digits */
    return 0;
}
