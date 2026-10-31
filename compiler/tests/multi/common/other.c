int hits;
int table[4];
int level = 7;

void bump(void)
{
    hits++;
    table[3]++;
}

int read_level(void)
{
    return level;
}
