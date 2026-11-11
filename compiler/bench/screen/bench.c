// Screen speed: the cycles between two calls of mark() are printed by
//   cargo run --release -p hbc2-cc --example screen_bench compiler/bench/screen
// Builds with the old library (42 x 24 text mode, PIXEL_MODE) and the
// native one (32 x 24, screen_line).
#include <hbc2.h>
#include <stdio.h>

void mark(void);                    // mark.has: a breakpoint for the bench

static char text[SCREEN_COLUMNS * SCREEN_ROWS];

int main(void)
{
    unsigned i;
    for (i = 0; i < sizeof text; i++)
        text[i] = 'A' + i % 26;
    puts("bench");                             // warm-up: the terminal is set up
    screen_clear(COLORS(WHITE, BLUE));
    mark();
    screen_clear(COLORS(WHITE, BLUE));         // 1. clear screen
    mark();
    screen_write(text, SCREEN_COLUMNS, 0, 0, COLORS(WHITE, BLUE));    // 2. one full row
    mark();
    screen_write(text, sizeof text, 0, 0, COLORS(WHITE, BLUE));      // 3. a full screen
    mark();
#ifdef PIXEL_MODE
    screen_set_mode(PIXEL_MODE);
    mark();
    for (i = 0; i < 100; i++)                  // 4. a 100-pixel line
        screen_put_pixel(i, 10, RED);
#else
    mark();
    screen_line(0, 10, 99, 10, RED);           // 4. a 100-pixel line
#endif
    mark();
    text[SCREEN_COLUMNS - 1] = 0;
    puts(text);                                // 5. one row through the terminal
    mark();
    return 0;
}
