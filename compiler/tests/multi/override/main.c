/* Expected: main returns 0
   A program's own definitions replace the library's (linker, step 11d):
   puts here, putchar in putchar.has. The library's printf calls putchar:
   it gets the program's. */
#include <stdio.h>
#include <string.h>

char out[16];                   /* filled by putchar.has */
unsigned char out_len;
int puts_calls;

int puts(const char *s)
{
    puts_calls++;
    return s[0];
}

int main(void)
{
    if (puts("not printed") != 'n')
        return 1;
    printf("n=%d", 42);
    if (puts_calls != 1)
        return 2;
    if (out_len != 4)
        return 3;
    if (memcmp(out, "n=42", 4))
        return 4;
    return 0;
}
