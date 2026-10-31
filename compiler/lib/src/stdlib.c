/* stdlib.h functions written in C. */

#include <stdlib.h>

/* --- Number to text: itoa, utoa, ltoa, ultoa ----------------------------
   Radix 10 and 16 take their digits from the math library's routines
   (asm/stdlib.has: no division), radix 2 and 8 from shifts (bits below). */

/* asm/stdlib.has: they return the end of the text (the 0). Hexadecimal:
   upper case, leading zeros kept. */
char *__itoa_dec(int value, char *buf);
char *__utoa_dec(unsigned value, char *buf);
char *__utoa_hex(unsigned value, char *buf);
char *__ltoa_dec(long value, char *buf);
char *__ultoa_dec(unsigned long value, char *buf);
char *__ultoa_hex(unsigned long value, char *buf);

/* The hexadecimal text from buf to end, without its leading zeros (the
   last digit kept) and in lower case, moved to buf. */
static void hex(char *buf, char *end)
{
    char *s = buf;
    while (*s == '0' && s + 1 < end)
        s++;
    while (s < end)
        *buf++ = *s++ | 0x20;   /* 'A'-'F' -> 'a'-'f'; '0'-'9' have the bit */
    *buf = 0;
}

/* Radix 2 or 8: each digit is 1 or 3 bits, taken from the lowest ones.
   Any other radix: an empty text. */
static void bits(unsigned long value, char *buf, int radix)
{
    char tmp[32];
    unsigned char n = 0;
    unsigned char shift;
    if (radix == 2)
        shift = 1;
    else if (radix == 8)
        shift = 3;
    else {
        *buf = 0;
        return;
    }
    do {
        tmp[n++] = '0' + ((unsigned char)value & (radix - 1));
        value >>= shift;
    } while (value);
    while (n)
        *buf++ = tmp[--n];
    *buf = 0;
}

char *itoa(int value, char *buf, int radix)
{
    if (radix == 10)
        __itoa_dec(value, buf);
    else
        utoa(value, buf, radix);
    return buf;
}

char *utoa(unsigned value, char *buf, int radix)
{
    if (radix == 10)
        __utoa_dec(value, buf);
    else if (radix == 16)
        hex(buf, __utoa_hex(value, buf));
    else
        bits(value, buf, radix);
    return buf;
}

char *ltoa(long value, char *buf, int radix)
{
    if (radix == 10)
        __ltoa_dec(value, buf);
    else
        ultoa(value, buf, radix);
    return buf;
}

char *ultoa(unsigned long value, char *buf, int radix)
{
    if (radix == 10)
        __ultoa_dec(value, buf);
    else if (radix == 16)
        hex(buf, __ultoa_hex(value, buf));
    else
        bits(value, buf, radix);
    return buf;
}


/* --- Text to number: strtol, strtoul, atoi, atol -------------------------
   Both read the text with parse, which returns the value without its sign
   and leaves the sign and the overflow in parse_neg and parse_over. */

static unsigned char parse_neg;     /* 1: there was a '-' */
static unsigned char parse_over;    /* 1: past 32 bits */

/* The value of digit c (0-9, then a-z or A-Z for 10-35), 255 if none */
static unsigned char digit(unsigned char c)
{
    if ((unsigned char)(c - '0') < 10)
        return c - '0';
    c |= 0x20;                      /* 'A'-'Z' -> 'a'-'z' */
    if ((unsigned char)(c - 'a') < 26)
        return c - 'a' + 10;
    return 255;
}

static unsigned long parse(const char *s, char **end, int base)
{
    const char *start = s;
    unsigned long v = 0;
    unsigned long lo, hi;
    unsigned char d, x;
    unsigned char any = 0;
    parse_neg = 0;
    parse_over = 0;
    if (base < 0 || base == 1 || base > 36) {
        if (end)
            *end = (char *)start;
        return 0;
    }
    while (*s == ' ' || (unsigned char)(*s - '\t') < 5)    /* \t \n \v \f \r */
        s++;
    if (*s == '-') {
        parse_neg = 1;
        s++;
    } else if (*s == '+')
        s++;
    if (*s == '0') {
        /* "0x" or "0b" only when a digit follows: otherwise, just the 0 */
        x = s[1] | 0x20;
        if ((base == 0 || base == 16) && x == 'x' && digit(s[2]) < 16) {
            s += 2;
            base = 16;
        } else if ((base == 0 || base == 2) && x == 'b' && digit(s[2]) < 2) {
            s += 2;
            base = 2;
        } else if (base == 0)
            base = 8;
    } else if (base == 0)
        base = 10;
    while ((d = digit(*s)) < base) {
        if (v < 0x04000000UL)       /* below 2^26: v * 36 + 35 fits */
            v = v * base + d;
        else if (!parse_over) {
            /* v * base + d in two 16-bit halves, to see the overflow */
            lo = (unsigned)v * (unsigned long)base + d;
            hi = (v >> 16) * base + (lo >> 16);
            if (hi > 0xFFFF)
                parse_over = 1;
            else
                v = (hi << 16) | (unsigned)lo;
        }
        any = 1;
        s++;
    }
    if (end)
        *end = (char *)(any ? s : start);
    return v;
}

long strtol(const char *str, char **str_end, int base)
{
    unsigned long v = parse(str, str_end, base);
    if (parse_neg) {
        if (parse_over || v > 0x80000000UL)
            return (long)0x80000000UL;  /* -2147483648 */
        return (long)(0 - v);
    }
    if (parse_over || v > 0x7FFFFFFFUL)
        return 0x7FFFFFFFL;
    return (long)v;
}

unsigned long strtoul(const char *str, char **str_end, int base)
{
    unsigned long v = parse(str, str_end, base);
    if (parse_over)
        return 0xFFFFFFFFUL;
    return parse_neg ? 0 - v : v;
}

int atoi(const char *s)
{
    return (int)strtol(s, NULL, 10);
}

long atol(const char *s)
{
    return strtol(s, NULL, 10);
}
