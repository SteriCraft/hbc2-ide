/* Expected: main returns 0. Small real programs (sieve, sort, strings,
   structs, CRC, long arithmetic); returns the number of the first wrong
   result. Also the benchmark for the code size and speed of the compiler. */

char flags[1000];
unsigned data[50];
char text[] = "The HBC-2 runs C programs now";
char buffer[40];

struct item {
    char kind;
    int count;
    unsigned mask;
};
struct item items[20];

int sieve(void)
{
    int i, j, n = 0;
    for (i = 0; i < 1000; i++)
        flags[i] = 1;
    for (i = 2; i < 1000; i++) {
        if (flags[i]) {
            n++;
            for (j = i + i; j < 1000; j += i)
                flags[j] = 0;
        }
    }
    return n;
}

unsigned seed = 1;

unsigned next_random(void)
{
    seed = seed * 25173 + 13849;
    return seed;
}

void sort(unsigned *a, int n)
{
    int i, j;
    unsigned t;
    for (i = 0; i < n - 1; i++)
        for (j = 0; j < n - 1 - i; j++)
            if (a[j] > a[j + 1]) {
                t = a[j];
                a[j] = a[j + 1];
                a[j + 1] = t;
            }
}

int length(char *s)
{
    char *p = s;
    while (*p)
        p++;
    return p - s;
}

void copy(char *d, char *s)
{
    while ((*d++ = *s++) != 0)
        ;
}

int compare(char *a, char *b)
{
    while (*a && *a == *b) {
        a++;
        b++;
    }
    return *a - *b;
}

void reverse(char *s)
{
    char *e = s + length(s) - 1;
    char t;
    while (s < e) {
        t = *s;
        *s++ = *e;
        *e-- = t;
    }
}

int count_upper(char *s)
{
    int n = 0;
    for (; *s; s++)
        if (*s >= 'A' && *s <= 'Z')
            n++;
    return n;
}

unsigned crc16(char *s)
{
    unsigned crc = 0xFFFF;
    int i;
    while (*s) {
        crc ^= (unsigned) *s++ << 8;
        for (i = 0; i < 8; i++) {
            if (crc & 0x8000)
                crc = (crc << 1) ^ 0x1021;
            else
                crc <<= 1;
        }
    }
    return crc;
}

int digit_sum(unsigned v)
{
    int s = 0;
    do {
        s += v % 10;
        v /= 10;
    } while (v);
    return s;
}

long fib(int n)
{
    long a = 0, b = 1, t;
    while (n-- > 0) {
        t = a + b;
        a = b;
        b = t;
    }
    return a;
}

int fill_items(void)
{
    struct item *p;
    int i, total = 0;
    for (i = 0, p = items; i < 20; i++, p++) {
        p->kind = 'a' + i % 3;
        p->count = i * 3;
        p->mask = 1 << (i & 15);
    }
    for (p = items; p < items + 20; p++) {
        if (p->kind == 'b')
            p->count += 100;
        p->count++;
        p->mask |= 0x8000;
        total += p->count;
    }
    return total;
}

int main(void)
{
    int i;
    unsigned sum = 0;
    if (sieve() != 168)
        return 1;
    for (i = 0; i < 50; i++)
        data[i] = next_random();
    sort(data, 50);
    for (i = 0; i < 49; i++)
        if (data[i] > data[i + 1])
            return 2;
    for (i = 0; i < 50; i++)
        sum += data[i] * (i + 1);
    if (sum != 29467u)
        return 3;
    if (length(text) != 29)
        return 4;
    copy(buffer, text);
    if (compare(buffer, text) != 0 || compare(buffer, "The") <= 0)
        return 5;
    reverse(buffer);
    if (buffer[0] != 'w' || buffer[28] != 'T' || count_upper(buffer) != 5)
        return 6;
    if (crc16(text) != 0xDB19u)
        return 7;
    if (digit_sum(65535u) != 24 || digit_sum(0) != 0)
        return 8;
    if (fib(40) != 102334155L)
        return 9;
    if (fill_items() != 1290)
        return 10;
    if (items[19].mask != 0x8008u || items[4].kind != 'b')
        return 11;
    return 0;
}
