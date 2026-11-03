/* Expected: main returns 0 (limits.h, errno.h, strerror, perror) */
/* Returns the number of the first failed check; errno.screen is what
   perror prints. */

#include <limits.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

int main(void)
{
    char c = 255;
    signed char sc = -128;
    int i = INT_MIN;
    long l = LONG_MIN;

    /* limits.h */
    if (CHAR_BIT != 8 || MB_LEN_MAX != 1)
        return 1;
    if (c != CHAR_MAX || CHAR_MIN != 0 || (unsigned char)c != UCHAR_MAX)
        return 2;
    if (sc != SCHAR_MIN || (signed char)(sc - 1) != SCHAR_MAX)
        return 3;
    if (i != (int)0x8000 || i - 1 != INT_MAX || SHRT_MIN != INT_MIN || SHRT_MAX != INT_MAX)
        return 4;
    if (UINT_MAX != 0xFFFF || USHRT_MAX != UINT_MAX || (unsigned)(INT_MAX + 1) != 0x8000)
        return 5;
    if (l != (long)0x80000000UL || l - 1 != LONG_MAX || ULONG_MAX != 0xFFFFFFFFUL)
        return 6;
    if (sizeof(INT_MIN) != 2 || sizeof(LONG_MIN) != 4 || sizeof(UINT_MAX) != 2 || sizeof(ULONG_MAX) != 4)
        return 7;
    if (INT_MIN >= 0 || LONG_MIN >= 0 || UINT_MAX < 0 || ULONG_MAX < 0)
        return 8;

    /* errno: 0 at the start, set by strtol / strtoul, never cleared */
    if (errno != 0)
        return 10;
    if (strtol("2147483647", NULL, 10) != LONG_MAX || strtol("-2147483648", NULL, 10) != LONG_MIN)
        return 11;
    if (strtoul("4294967295", NULL, 10) != ULONG_MAX || errno != 0)
        return 12;
    if (strtol("2147483648", NULL, 10) != LONG_MAX || errno != ERANGE)
        return 13;
    if (atoi("12") != 12 || errno != ERANGE)    /* not cleared */
        return 14;
    errno = 0;
    if (strtol("-2147483649", NULL, 10) != LONG_MIN || errno != ERANGE)
        return 15;
    errno = 0;
    if (strtoul("4294967296", NULL, 10) != ULONG_MAX || errno != ERANGE)
        return 16;
    errno = 0;
    if (strtol("ffffffff", NULL, 16) != LONG_MAX || errno != ERANGE)
        return 17;

    /* strerror */
    if (strcmp(strerror(0), "No error") != 0)
        return 20;
    if (strcmp(strerror(EDOM), "Domain error") != 0)
        return 21;
    if (strcmp(strerror(ERANGE), "Result out of range") != 0)
        return 22;
    if (strcmp(strerror(3), "Unknown error") != 0 || strcmp(strerror(255), "Unknown error") != 0)
        return 23;
    if (strcmp(strerror(256), "Unknown error") != 0 || strcmp(strerror(-1), "Unknown error") != 0)
        return 24;
    if (strcmp(strerror(258), "Unknown error") != 0)   /* low byte 2 */
        return 25;
    if (strerror(ERANGE) != strerror(ERANGE))
        return 26;

    /* perror */
    errno = ERANGE;
    perror("strtol");
    errno = 0;
    perror(NULL);
    errno = EDOM;
    perror("");
    errno = 1000;
    perror("x");
    return 0;
}
