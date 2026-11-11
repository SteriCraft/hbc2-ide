/* Expected: main returns 0 (hbc2.h: devices, ports, screen, RTC, keyboard, exit) */
/* Returns the number of the first failed check. hardware.keys types "ab";
   hardware.screen is what the screen shows at the end. */

#include <hbc2.h>
#include <stdlib.h>

static char text[] = "Hello, HBC-2!";

int pixels(void)
{
    /* the power-on picture is blue */
    if (screen_get_pixel(11, 20) != BLUE)
        return 8;
    if (screen_put_pixel(10, 20, RED) || screen_put_pixel(255, 191, WHITE))
        return 2;
    if (screen_put_pixel(0, 192, RED) != 1)     /* below the screen */
        return 3;
    if (screen_put_pixel(0, 0, 16) != 1)        /* not a color */
        return 4;
    if (screen_get_pixel(10, 20) != RED || screen_get_pixel(255, 191) != WHITE)
        return 5;
    if (screen_get_pixel(11, 20) != BLUE || screen_get_pixel(0, 192) != -1)
        return 6;
    /* a line: both ends, nothing past them */
    if (screen_line(0, 30, 99, 30, GREEN) || screen_line(0, 0, 9, 9, 16) != 1)
        return 9;
    if (screen_get_pixel(0, 30) != GREEN || screen_get_pixel(99, 30) != GREEN
        || screen_get_pixel(100, 30) != BLUE)
        return 1;
    /* 10 x 5 from 50, 50: the outline, then filled */
    if (screen_rect(50, 50, 10, 5, CYAN))
        return 70;
    if (screen_get_pixel(50, 50) != CYAN || screen_get_pixel(59, 54) != CYAN
        || screen_get_pixel(55, 52) != BLUE || screen_get_pixel(60, 52) != BLUE)
        return 71;
    if (screen_fill_rect(50, 50, 10, 5, MAGENTA) || screen_get_pixel(55, 52) != MAGENTA
        || screen_get_pixel(59, 55) != BLUE)
        return 72;
    /* a size of 0 is 256: a full-width band */
    if (screen_fill_rect(0, 60, 0, 2, RED) || screen_get_pixel(255, 61) != RED
        || screen_get_pixel(0, 62) == RED)
        return 73;
    if (screen_fill(BLUE) || screen_get_pixel(100, 100) != BLUE || screen_fill(16) != 1)
        return 7;
    return 0;
}

/* a glyph for '~': a frame */
static const unsigned char frame[8] = { 0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF };

int cells(void)
{
    unsigned char color = COLORS(WHITE, BLUE);
    if (screen_clear(COLORS(LIGHT_GRAY, BLACK)))
        return 10;
    if (screen_putc('A', 0, 0, color))
        return 11;
    if (screen_getc(0, 0) != (color << 8 | 'A'))
        return 12;
    if (screen_fill(BLACK) || screen_getc(0, 0) != (color << 8 | 'A'))  /* cells kept */
        return 62;
    if (screen_putc('A', 32, 0, color) != 1 || screen_putc('A', 0, 24, color) != 1)
        return 13;
    if (screen_putc(7, 0, 0, color) != 1)       /* a control code */
        return 14;
    if (screen_getc(32, 0) != -1)
        return 15;
    /* 13 characters from column 25 of row 2: 7 on row 2, 6 on row 3 */
    if (screen_write(text, 13, 25, 2, COLORS(LIGHT_GRAY, BLACK)))
        return 16;
    if ((screen_getc(31, 2) & 255) != ' ' || (screen_getc(0, 3) & 255) != 'H')
        return 17;
    if (screen_write(text, 13, 32, 0, color) != 13)     /* off the grid */
        return 18;
    if (screen_set_bg(GREEN) || screen_getc(0, 0) != (COLORS(WHITE, GREEN) << 8 | 'A'))
        return 19;
    if (screen_set_txt(BRIGHT_YELLOW)
        || screen_getc(0, 0) != (COLORS(BRIGHT_YELLOW, GREEN) << 8 | 'A'))
        return 50;
    if (screen_set_bg(16) != 1 || screen_set_txt(16) != 1)
        return 51;
    /* DEFCHAR: '~' becomes a frame, cell 5, 5 is at pixel 40, 40 */
    if (screen_define_char('~', frame) || screen_define_char('\n', frame) != 1)
        return 52;
    if (screen_putc('~', 5, 5, COLORS(WHITE, BLACK)))
        return 53;
    if (screen_get_pixel(40, 40) != WHITE || screen_get_pixel(41, 41) != BLACK
        || screen_get_pixel(47, 47) != WHITE)
        return 54;
    /* the last cell doesn't scroll; the next character does */
    if (screen_write(text, 3, 29, 23, COLORS(LIGHT_GRAY, BLACK))
        || (screen_getc(31, 23) & 255) != 'l' || (screen_getc(0, 0) & 255) != 'A')
        return 55;
    if (screen_write(text + 3, 10, 29, 23, COLORS(LIGHT_GRAY, BLACK))
        || (screen_getc(29, 22) & 255) != 'l' || (screen_getc(1, 23) & 255) != 'H'
        || (screen_getc(0, 2) & 255) != 'H')    /* row 3 moved up */
        return 56;
    if (screen_scroll(2) || (screen_getc(0, 0) & 255) != 'H')
        return 57;
    /* back to the defaults: font, colours, blue screen */
    if (screen_reset() || screen_getc(0, 0) != (DEFAULT_COLORS << 8 | ' '))
        return 58;
    if (screen_putc('~', 5, 5, DEFAULT_COLORS) || screen_get_pixel(40, 40) != BLUE)
        return 59;
    /* Windows-1252: e acute, a grave, c cedilla, oe, euro */
    if (screen_write("\xE9 \xE0 \xE7 \x9C \x80", 9, 0, 1, DEFAULT_COLORS))
        return 60;
    if ((screen_getc(0, 1) & 255) != 0xE9 || (screen_getc(8, 1) & 255) != 0x80)
        return 61;
    return 0;
}

int ports(void)
{
    if (monitor_port == 0 || display_port == 0 || rtc_port == 0 || keyboard_port == 0
        || eeprom_port == 0)
        return 20;
    /* a port is a latch: it holds the last value written */
    out(monitor_port + 2, 17);                  /* the monitor's POS_X */
    if (in(monitor_port + 2) != 17)
        return 21;
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
    if (rtc_set(&t) != 1)                      /* refused */
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
