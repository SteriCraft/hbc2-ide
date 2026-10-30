/* Expected: main returns 0 (printf) */
/* Returns the number of the first wrong count; printf.screen is what the
   lines print. */

#include <stdio.h>

int main(void)
{
    if (printf("plain\n") != 6)
        return 1;
    if (printf("%d %d %d|\n", 0, 42, -42) != 10)
        return 2;
    if (printf("%d %d\n", 32767, (int)0x8000) != 13)
        return 3;
    if (printf("%u %x %x\n", 65535u, 0xBEEFu, 0) != 13)
        return 4;
    if (printf("%ld %lu %lx\n", -123456789L, 4000000000UL, 0xDEADBEEFUL) != 31)
        return 5;
    if (printf("%ld %ld %lu\n", 2147483647L, (long)0x80000000UL, 0UL) != 25)
        return 6;
    if (printf("[%5d][%05d][%05d][%2d]\n", 42, 42, -42, 12345) != 29)
        return 7;
    if (printf("[%04x][%8lx][%3s][%3c][%c]\n", 0xab, 0x1234L, "ab", 'z', 'y') != 30)
        return 8;
    if (printf("100%% %s=%d%c\n", "x", 5, '!') != 10)
        return 9;
    if (printf("[%s][%1s]\n", "", "long") != 9)
        return 10;
    if (printf("end%") != 3)                /* a % at the end is dropped */
        return 11;
    return 0;
}
