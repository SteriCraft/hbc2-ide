/* Expected: main returns 0 (ctype.h) */
/* Returns the functions that gave a wrong answer, one bit each: every
   value from EOF (-1) to 255 and a few out of range, against plain C. */

#include <ctype.h>

static int in(int c, int lo, int hi) { return c >= lo && c <= hi; }
/* The is... functions return exactly 1 or 0. */
static int same(int got, int want) { return got == (want != 0); }

/* Bit n set: function n gave a wrong answer somewhere. */
int main(void)
{
    int c, i;
    unsigned bad = 0;
    int extra[4];
    extra[0] = 256; extra[1] = -5; extra[2] = 0x161; extra[3] = 0x141;
    for (i = -1; i < 260; i++) {
        int lower, upper, digit, xdigit, space, punct, cntrl, print, value;
        c = i < 256 ? i : extra[i - 256];
        value = c >= 0 && c < 256;
        lower = value && in(c, 'a', 'z');
        upper = value && in(c, 'A', 'Z');
        digit = value && in(c, '0', '9');
        xdigit = digit || (value && (in(c, 'a', 'f') || in(c, 'A', 'F')));
        space = value && (c == ' ' || in(c, 9, 13));
        cntrl = value && (c < 0x20 || c == 0x7F);
        print = value && in(c, 0x20, 0x7E);
        punct = print && c != ' ' && !lower && !upper && !digit;
        if (!same(isalnum(c), lower || upper || digit)) bad |= 1;
        if (!same(isalpha(c), lower || upper)) bad |= 2;
        if (!same(islower(c), lower)) bad |= 4;
        if (!same(isupper(c), upper)) bad |= 8;
        if (!same(isdigit(c), digit)) bad |= 16;
        if (!same(isxdigit(c), xdigit)) bad |= 32;
        if (!same(isspace(c), space)) bad |= 64;
        if (!same(ispunct(c), punct)) bad |= 128;
        if (!same(iscntrl(c), cntrl)) bad |= 256;
        if (!same(isgraph(c), print && c != ' ')) bad |= 512;
        if (!same(isprint(c), print)) bad |= 1024;
        if (!same(isblank(c), value && (c == ' ' || c == '\t'))) bad |= 2048;
        if (tolower(c) != (upper ? c + 32 : c)) bad |= 4096;
        if (toupper(c) != (lower ? c - 32 : c)) bad |= 8192;
    }
    return bad;
}
