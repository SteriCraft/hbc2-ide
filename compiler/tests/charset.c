/* Expected: main returns 0 (UTF-8 literals become Windows-1252) */
/* charset.screen: what puts shows (the font has the accents). */

#include <stdio.h>
#include <string.h>

static const char text[] = "Noël, cœur, 5 €";

int main(void)
{
    if (strlen(text) != 15)                     /* one byte per character */
        return 1;
    if ((unsigned char)text[2] != 0xEB)         /* ë */
        return 2;
    if ((unsigned char)text[7] != 0x9C)         /* œ: Windows-1252 only */
        return 3;
    if ((unsigned char)text[14] != 0x80)        /* € */
        return 4;
    if ((unsigned char)'é' != 0xE9 || 'é' != '\xE9')
        return 5;
    if (strcmp("à", "\xE0") != 0)
        return 6;
    puts("Été : à l'œil, 10 €");
    return 0;
}
