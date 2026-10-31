/* Expected: main returns 0 (hbc2.h: devices, ports, monitor, RTC, keyboard, exit) */
/* Returns the number of the first failed check. hardware.keys types "ab";
   hardware.screen is what the monitor shows at the end. */

#include <hbc2.h>
#include <stdlib.h>

static char text[] = "Hello, HBC-2!";

int pixels(void)
{
    if (screen_pixel_mode())
        return 1;
    if (screen_put_pixel(10, 20, RED) || screen_put_pixel(255, 191, WHITE))
        return 2;
    if (screen_put_pixel(0, 192, RED) != 1)     /* below the screen */
        return 3;
    if (screen_put_pixel(0, 0, 16) != 1)        /* not a color */
        return 4;
    if (screen_get_pixel(10, 20) != RED || screen_get_pixel(255, 191) != WHITE)
        return 5;
    if (screen_get_pixel(11, 20) != BLACK || screen_get_pixel(0, 192) != -1)
        return 6;
    if (screen_fill(BLUE) || screen_get_pixel(100, 100) != BLUE)
        return 7;
    return 0;
}

int cells(void)
{
    unsigned char color = COLORS(WHITE, BLUE);
    if (screen_text_mode() || screen_clear(COLORS(LIGHT_GRAY, BLACK)))
        return 10;
    if (screen_putc('A', 0, 0, color))
        return 11;
    if (screen_getc(0, 0) != (color << 8 | 'A'))
        return 12;
    if (screen_putc('A', 42, 0, color) != 1 || screen_putc('A', 0, 24, color) != 1)
        return 13;
    if (screen_putc(7, 0, 0, color) != 1)       /* not displayable */
        return 14;
    if (screen_getc(42, 0) != -1)
        return 15;
    /* 13 characters from column 35 of row 2: 7 on row 2, 6 on row 3 */
    if (screen_write(text, 13, 35, 2, COLORS(LIGHT_GRAY, BLACK)))
        return 16;
    if ((screen_getc(41, 2) & 255) != ' ' || (screen_getc(0, 3) & 255) != 'H')
        return 17;
    /* the last row: 3 characters fit, 10 are left */
    if (screen_write(text, 13, 39, 23, COLORS(LIGHT_GRAY, BLACK)) != 10)
        return 18;
    if (screen_set_bg(GREEN) || screen_getc(0, 0) != (COLORS(WHITE, GREEN) << 8 | 'A'))
        return 19;
    return 0;
}

int ports(void)
{
    if (monitor_port == 0 || rtc_port == 0 || keyboard_port == 0 || eeprom_port != 0)
        return 20;
    /* a port is a latch: it holds the last value written */
    out(monitor_port + 2, 17);                  /* the monitor's POS_X */
    if (in(monitor_port + 2) != 17)
        return 21;
    /* no EEPROM (a RAM program): everything refused */
    if (eeprom_read(0) != -1 || eeprom_write(EEPROM_FREE, 1) != 1)
        return 22;
    if (eeprom_load(&monitor_port, 0, 1) != 1 || eeprom_save(0, &monitor_port, 1) != 1)
        return 23;
    return 0;
}

int clock(void)
{
    struct rtc_time t;
    if (rtc_get(&t))
        return 30;
    /* the emulator's RTC starts at 2000-01-01 00:00:00 (in tests, no time goes by) */
    if (t.year != 100 || t.month != 1 || t.day != 1 || t.hour != 0 || t.minute != 0)
        return 31;
    t.year = 126;
    t.month = 2;
    t.day = 29;                                 /* 2026 is not a leap year */
    if (rtc_set(&t) != 1)
        return 32;
    t.month = 10;
    t.day = 30;
    t.hour = 14;
    t.minute = 5;
    t.second = 0;
    if (rtc_set(&t))
        return 33;
    t.year = 0;
    if (rtc_get(&t) || t.year != 126 || t.month != 10 || t.day != 30 || t.hour != 14)
        return 34;
    return 0;
}

int keys(void)
{
    int k;
    /* 'a' pressed, released, then 'b' (scan codes: the letter in upper case) */
    while ((k = key_event()) == -1)
        ;
    if (k != 'A')
        return 40;
    while ((k = key_event()) == -1)
        ;
    if (k != ('A' | KEY_RELEASED))
        return 41;
    while ((k = key_event()) == -1)
        ;
    if (k != 'B')
        return 42;
    return 0;
}

void finish(int r)
{
    if (r)
        exit(r);
}

int main(void)
{
    finish(ports());
    finish(pixels());
    finish(cells());
    finish(clock());
    finish(keys());
    interrupts_off();
    interrupts_on();
    exit(0);
    return 99;                                  /* not reached */
}
