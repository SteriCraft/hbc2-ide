/* stdio.h functions written in C: input from the terminal. */

#include <hbc2.h>
#include <stdio.h>

/* The next character of the line read by getchar, 0 when a new line
   must be read. */
static char *next;

int getchar(void)
{
    int c;
    if (!next)
        next = term_readline();
    c = *next++;
    if (c == 0) {
        next = 0;
        return '\n';
    }
    return c;
}

char *gets(char *s)
{
    char *line = term_readline();
    char *d = s;
    while ((*d++ = *line++) != 0)
        ;
    next = 0;
    return s;
}
