/* Expected: main returns 0 (atoi, atol, strtol, strtoul, itoa, utoa, ltoa, ultoa in radix 2, 8, 10, 16) */
/* Returns the number of the first failed check. */

#include <stdlib.h>
#include <string.h>

static char buf[33];
static char *end;

/* texts whose address the checks of end compare to */
static const char str_x[] = "0x", str_xg[] = "0xg", str_x1f[] = "x1F";
static const char str_p1[] = " + 1", str_12[] = "12";

/* 1 when r is buf and the text at buf is t */
static int is(char *r, const char *t)
{
    return r == buf && strcmp(buf, t) == 0;
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
    if (atol("2147483648") != 2147483647L || atol("-2147483649") != (long)0x80000000UL || atol("99999999999") != 2147483647L)
        return 8;                               /* does not fit: the limits */

    if (!is(itoa(0, buf, 10), "0") || !is(itoa(-1, buf, 10), "-1"))
        return 9;
    if (!is(itoa(32767, buf, 10), "32767") || !is(itoa((int)0x8000, buf, 10), "-32768"))
        return 10;
    if (!is(utoa(65535u, buf, 10), "65535") || !is(utoa(100, buf, 10), "100"))
        return 11;
    if (!is(ltoa(0, buf, 10), "0") || !is(ltoa(-2147483647L - 1, buf, 10), "-2147483648"))
        return 12;
    if (!is(ltoa(1000000000L, buf, 10), "1000000000") || !is(ltoa(-10, buf, 10), "-10"))
        return 13;
    if (!is(ultoa(4294967295UL, buf, 10), "4294967295") || !is(ultoa(65536UL, buf, 10), "65536"))
        return 14;
    if (!is(utoa(0xBEEFu, buf, 16), "beef") || !is(utoa(0x0A, buf, 16), "a") || !is(utoa(0, buf, 16), "0"))
        return 15;
    if (!is(ultoa(0xDEADBEEFUL, buf, 16), "deadbeef") || !is(ultoa(0x1234UL, buf, 16), "1234"))
        return 16;
    if (atol(ltoa(-98765L, buf, 10)) != -98765L)    /* back and forth */
        return 17;
    if (atoi("0x1F") != 0 || atol("12abc") != 12)
        return 18;                              /* atoi, atol: decimal only */
    if (strtol("0x1F", 0, 16) != 31 || strtol("0Xff", 0, 0) != 255 || strtol(" -0x10", 0, 16) != -16)
        return 19;
    if (strtol("FFFF", 0, 16) != 0xFFFF || strtol("+aBcD", 0, 16) != 0xABCD || strtol("12g", 0, 16) != 0x12)
        return 20;
    if (strtol(str_x, &end, 16) != 0 || end != str_x + 1 || strtol(str_xg, &end, 0) != 0 || end != str_xg + 1)
        return 21;                              /* no digit after 0x: the 0 */
    if (strtol(str_x1f, &end, 16) != 0 || end != str_x1f || strtol(str_p1, &end, 10) != 0 || end != str_p1)
        return 22;                              /* no digit: 0, end = the text */
    if (strtoul("0xDEADBEEF", 0, 0) != 0xDEADBEEFUL || strtoul("FFFFFFFF", 0, 16) != 0xFFFFFFFFUL)
        return 23;
    if (strtoul("100000000", 0, 16) != 0xFFFFFFFFUL || strtoul("4294967296", 0, 10) != 0xFFFFFFFFUL)
        return 24;                              /* past 32 bits: the limit */
    if (strtoul("4294967295", 0, 10) != 0xFFFFFFFFUL || strtoul("-1", 0, 10) != 0xFFFFFFFFUL || strtoul("-2", 0, 0) != 0xFFFFFFFEUL)
        return 25;
    if (strtol("0x7FFFFFFF", 0, 0) != 2147483647L || strtol("0x80000000", 0, 0) != 2147483647L || strtol("-0x80000000", 0, 0) != (long)0x80000000UL)
        return 26;
    if (strtol("-0x80000001", 0, 0) != (long)0x80000000UL || strtol("-99999999999999999999", &end, 10) != (long)0x80000000UL || *end != 0)
        return 27;                              /* end past every digit, even too many */
    if (strtol("017", 0, 0) != 15 || strtol("019", &end, 0) != 1 || *end != '9' || strtol("0", &end, 0) != 0 || *end != 0)
        return 28;                              /* base 0: a leading 0 is octal */
    if (strtol("0b101", 0, 0) != 5 || strtol("0B11", 0, 2) != 3 || strtol("0b2", &end, 0) != 0 || *end != 'b' || strtol("0b1", 0, 16) != 0xB1)
        return 29;
    if (strtol("zZ", 0, 36) != 36 * 35 + 35 || strtol("123", 0, 4) != 27 || strtol("1a", 0, 10) != 1 || strtol("09", 0, 8) != 0)
        return 30;
    if (strtol(str_12, &end, 1) != 0 || end != str_12 || strtol(str_12, &end, 37) != 0 || end != str_12 || strtol("12", 0, -1) != 0)
        return 31;                              /* base refused */
    if (strtol("0x09", 0, 16) != 9 || strtol("@", 0, 16) != 0 || strtol(":", 0, 16) != 0 || strtol("`G[", 0, 36) != 0)
        return 32;                              /* the characters around the digits */
    if (strtol("\t\n 42 ", &end, 10) != 42 || *end != ' ' || strtol("2147483647", &end, 10) != 2147483647L || *end != 0)
        return 33;
    if (strtoul("0x4000000", 0, 0) != 0x4000000UL || strtoul("3FFFFFFF", 0, 16) != 0x3FFFFFFFUL || strtoul("123456789", 0, 10) != 123456789UL)
        return 34;                              /* around 2^26, where parse changes way */

    /* radix 16, 2, 8: negative values written as their bits */
    if (!is(itoa(-1, buf, 16), "ffff") || !is(itoa(0x7F, buf, 16), "7f") || !is(ltoa(-1, buf, 16), "ffffffff"))
        return 35;
    if (!is(itoa(0, buf, 2), "0") || !is(itoa(5, buf, 2), "101") || !is(itoa(-1, buf, 2), "1111111111111111"))
        return 36;
    if (!is(ultoa(0x80000001UL, buf, 2), "10000000000000000000000000000001"))
        return 37;
    if (!is(utoa(8, buf, 8), "10") || !is(itoa(-1, buf, 8), "177777") || !is(ultoa(4294967295UL, buf, 8), "37777777777"))
        return 38;
    if (!is(itoa(0, buf, 8), "0") || !is(ltoa(-8, buf, 8), "37777777770"))
        return 39;
    if (!is(itoa(12, buf, 3), "") || !is(ltoa(12, buf, 0), "") || !is(utoa(12, buf, 36), ""))
        return 40;                              /* radix refused: empty */
    return 0;
}
