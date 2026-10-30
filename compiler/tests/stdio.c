/* Expected: main returns 0 (stdio.h: putchar, puts, getchar, gets; scrolling) */
/* Returns the number of the first failed check. stdio.keys types "hi" and
   "ok" (each followed by Enter); stdio.screen is the screen at the end. */

#include <stdio.h>

/* "line n" in s */
void line(char *s, int n)
{
    char *p = s;
    *p++ = 'l';
    *p++ = 'i';
    *p++ = 'n';
    *p++ = 'e';
    *p++ = ' ';
    if (n >= 10)
        *p++ = '0' + n / 10;
    *p++ = '0' + n % 10;
    *p = 0;
}

int main(void)
{
    static char s[128];
    int i;
    if (putchar('A') != 'A')
        return 1;
    if (putchar(0x141) != 0x41)             /* an unsigned char: 'A' */
        return 2;
    putchar(7);                             /* not shown */
    putchar('\n');
    if (puts("Hello") != 0)
        return 3;
    /* 30 more rows: the screen scrolls */
    for (i = 1; i <= 30; i++) {
        line(s, i);
        puts(s);
    }
    /* "hi" + Enter: getchar gives the characters, then '\n' */
    if (getchar() != 'h' || getchar() != 'i' || getchar() != '\n')
        return 4;
    /* "ok" + Enter */
    if (gets(s) != s || s[0] != 'o' || s[1] != 'k' || s[2] != 0)
        return 5;
    puts("end");
    return 0;
}
