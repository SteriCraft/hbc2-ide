/* Expected: main returns 0 (common variables: int n; in two files) */
/* Returns the number of the first failed check. hits and table have no
   value in both files (C "tentative definitions"): the two files share
   one variable. level has a value in level.c: that definition is used. */

int hits;
int table[4];
int level;

void bump(void);
int read_level(void);

int main(void)
{
    if (hits != 0)
        return 1;
    hits = 5;
    bump();
    if (hits != 6)
        return 2;
    table[3] = 40;
    bump();
    if (table[3] != 41)
        return 3;
    if (level != 7 || read_level() != 7)
        return 4;
    level = 9;
    if (read_level() != 9)
        return 5;
    return 0;
}
