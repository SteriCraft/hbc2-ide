/* string.h functions. Characters are compared as unsigned char (char is
   unsigned on the HBC-2). */

#include <string.h>

size_t strlen(const char *s)
{
    const char *p = s;
    while (*p)
        p++;
    return p - s;
}

char *strcpy(char *dest, const char *src)
{
    char *d = dest;
    while ((*d++ = *src++) != 0)
        ;
    return dest;
}

/* Copies at most n characters; pads with zeros up to n */
char *strncpy(char *dest, const char *src, size_t n)
{
    char *d = dest;
    while (n && (*d = *src) != 0) {
        d++;
        src++;
        n--;
    }
    while (n) {
        *d++ = 0;
        n--;
    }
    return dest;
}

char *strcat(char *dest, const char *src)
{
    strcpy(dest + strlen(dest), src);
    return dest;
}

/* Appends at most n characters, then a 0 */
char *strncat(char *dest, const char *src, size_t n)
{
    char *d = dest + strlen(dest);
    while (n && *src) {
        *d++ = *src++;
        n--;
    }
    *d = 0;
    return dest;
}

int strcmp(const char *a, const char *b)
{
    while (*a && *a == *b) {
        a++;
        b++;
    }
    return *a - *b;
}

int strncmp(const char *a, const char *b, size_t n)
{
    while (n && *a && *a == *b) {
        a++;
        b++;
        n--;
    }
    if (n == 0)
        return 0;
    return *a - *b;
}

/* The terminating 0 counts: strchr(s, 0) is the end of s */
char *strchr(const char *s, int c)
{
    char ch = c;
    for (;;) {
        if (*s == ch)
            return (char *)s;
        if (*s == 0)
            return NULL;
        s++;
    }
}

char *strrchr(const char *s, int c)
{
    char ch = c;
    const char *found = NULL;
    for (;;) {
        if (*s == ch)
            found = s;
        if (*s == 0)
            return (char *)found;
        s++;
    }
}

void *memcpy(void *dest, const void *src, size_t n)
{
    char *d = dest;
    const char *s = src;
    while (n) {
        *d++ = *s++;
        n--;
    }
    return dest;
}

/* Like memcpy, also when the areas overlap */
void *memmove(void *dest, const void *src, size_t n)
{
    char *d = dest;
    const char *s = src;
    if (d <= s || d >= s + n)
        return memcpy(dest, src, n);
    d += n;
    s += n;
    while (n) {
        *--d = *--s;
        n--;
    }
    return dest;
}

void *memset(void *s, int c, size_t n)
{
    char *p = s;
    char ch = c;
    while (n) {
        *p++ = ch;
        n--;
    }
    return s;
}

int memcmp(const void *a, const void *b, size_t n)
{
    const unsigned char *p = a;
    const unsigned char *q = b;
    while (n) {
        if (*p != *q)
            return *p - *q;
        p++;
        q++;
        n--;
    }
    return 0;
}

void *memchr(const void *s, int c, size_t n)
{
    const char *p = s;
    char ch = c;
    while (n) {
        if (*p == ch)
            return (void *)p;
        p++;
        n--;
    }
    return NULL;
}
