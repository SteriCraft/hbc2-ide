/* printf: the minimal set (ROADMAP decision 14): %d %u %x %c %s %%,
   l for long, a width, 0 for zero padding. */

#include <stdio.h>
#include <stdarg.h>

/* Prints s (len characters) after its sign, padded to width. Returns
   the number of characters printed. */
static int field(const char *s, int len, char neg, int width, char pad)
{
    int count = len + neg;
    int n;
    if (neg && pad == '0')
        putchar('-');                       /* -0042: the sign before the zeros */
    for (n = count; n < width; n++)
        putchar(pad);
    if (neg && pad != '0')
        putchar('-');
    for (n = 0; n < len; n++)
        putchar(s[n]);
    return count < width ? width : count;
}

int printf(const char *fmt, ...)
{
    va_list ap;
    char digits[11];                        /* a long: 10 digits at most */
    char *end = digits + sizeof(digits);
    char *s;
    int count = 0;
    int width;
    unsigned base;
    unsigned u;
    unsigned long lu;
    char c, pad, is_long, neg;

    va_start(ap, fmt);
    while ((c = *fmt++) != 0) {
        if (c != '%') {
            putchar(c);
            count++;
            continue;
        }
        pad = ' ';
        if (*fmt == '0') {
            pad = '0';
            fmt++;
        }
        width = 0;
        while (*fmt >= '0' && *fmt <= '9')
            width = width * 10 + (*fmt++ - '0');
        is_long = 0;
        if (*fmt == 'l') {
            is_long = 1;
            fmt++;
        }
        c = *fmt++;
        neg = 0;
        s = end;                            /* digits go backwards from the end */
        switch (c) {
        case 'd':
        case 'u':
        case 'x':
            base = c == 'x' ? 16 : 10;
            if (is_long) {
                lu = va_arg(ap, unsigned long);
                if (c == 'd' && (long)lu < 0) {
                    neg = 1;
                    lu = -lu;
                }
                do {
                    *--s = "0123456789abcdef"[(int)(lu % base)];
                    lu /= base;
                } while (lu);
            } else {
                u = va_arg(ap, unsigned);
                if (c == 'd' && (int)u < 0) {
                    neg = 1;
                    u = -u;
                }
                do {
                    *--s = "0123456789abcdef"[u % base];
                    u /= base;
                } while (u);
            }
            count += field(s, end - s, neg, width, pad);
            break;
        case 'c':
            *--s = va_arg(ap, int);
            count += field(s, 1, 0, width, ' ');
            break;
        case 's':
            s = va_arg(ap, char *);
            for (u = 0; s[u]; u++)
                ;
            count += field(s, u, 0, width, ' ');
            break;
        case 0:
            fmt--;                          /* a % at the end: stop there */
            break;
        default:                            /* %% and unknown ones: the character */
            putchar(c);
            count++;
            break;
        }
    }
    va_end(ap);
    return count;
}
