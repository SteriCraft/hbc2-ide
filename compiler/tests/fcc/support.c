/*
 * HBC-2 stand-in for the output of upstream's test emulators (FCC's
 * test/emu6809.c): printchar and printint write into fcc_out, which
 * hbc2/crates/hbc2-cc/tests/fcc.rs reads at the end and compares with the
 * expected output. Not an upstream file.
 */

char fcc_out[1024];
unsigned fcc_len;

void printchar(int c)
{
    if (fcc_len < sizeof fcc_out)
        fcc_out[fcc_len++] = c;
}

/* As the emulator: the value and a new line, signed except 0x8000 (32768). */
void printint(int x)
{
    unsigned u = x;
    char digits[6];
    int n = 0;

    if (u > 0x8000u) {
        printchar('-');
        u = -u;
    }
    do {
        digits[n++] = '0' + u % 10;
        u /= 10;
    } while (u);
    while (n)
        printchar(digits[--n]);
    printchar('\n');
}
