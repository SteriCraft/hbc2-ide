#include <string.h>
#include <stddef.h>									// NULL

char *strstr(const char *str, const char *substr)
{
    size_t len = strlen(substr);

    if (len == 0)                       			// Empty substring: str
        return (char *)str;

    while ((str = strchr(str, *substr)) != NULL)
    {
        if (strncmp(str, substr, len) == 0)
            return (char *)str;         			// Found: where it starts

        str++;                          			// Try after this one
    }

    return NULL;
}
