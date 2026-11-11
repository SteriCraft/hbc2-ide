/* Expected: main returns 0 (delay keeps the I flag as it was) */
#include <hbc2.h>
#include <time.h>

unsigned char iflag(void);      /* iflag.has: 1 when interrupts are on */

int main(void)
{
    clock_t c;
    interrupts_off();
    c = clock();
    delay(3);                   /* waits, interrupts off before and after */
    if (iflag() != 0)
        return 1;
    if (clock() - c < 2)
        return 2;
    interrupts_on();
    delay(2);
    if (iflag() != 1)
        return 3;
    interrupts_off();
    screen_write("abc", 3, 0, 0, DEFAULT_COLORS);   /* the screen keeps it too */
    if (iflag() != 0)
        return 4;
    interrupts_on();
    screen_write("abc", 3, 0, 0, DEFAULT_COLORS);
    if (iflag() != 1)
        return 5;
    return 0;
}
