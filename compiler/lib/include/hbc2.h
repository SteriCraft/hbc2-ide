// hbc2.h - the HBC-2 hardware from C: I/O ports, interrupts, monitor,
// terminal, real-time clock, EEPROM, random numbers, keyboard.
// Assembly in compiler/lib/asm/hbc2.has.
//
// Functions that can fail return 0 when they worked, and a non-zero
// value when the device is missing or a value is refused.

#ifndef __HBC2_H
#define __HBC2_H


// --- Devices ---------------------------------------------------------
// The first port of each device, 0 when it is not plugged. Set at
// startup (compiler/runtime/devices.has).

#define MONITOR_ID               0x4A
#define RTC_ID                   0x95
#define KEYBOARD_ID              0xAA
#define EEPROM_ID                0xE1

#define MONITOR_PORTS_COUNT      5
#define RTC_PORTS_COUNT          7
#define KEYBOARD_PORTS_COUNT     2
#define EEPROM_PORTS_COUNT       5

extern unsigned char monitor_port;
extern unsigned char rtc_port;
extern unsigned char keyboard_port;
extern unsigned char eeprom_port;


// --- I/O ports and interrupts ---------------------------------------

unsigned char in(unsigned char port);           // the value port holds
void out(unsigned char port, unsigned char value);

void interrupts_off(void);                      // cli
void interrupts_on(void);                       // sti
void wait_interrupt(void);  // sti + hlt: sleeps until the next interrupt (enables them)

// Points the interrupt vector of port to handler, with interrupts off
// while it is written. Returns a non-zero value if port is 0 (refused:
// device detection, kept by the startup code). The handler is an
// assembly routine that ends with irt, declared in C as "void name(void);" (label _name): see
// compiler/ABI.md, section 10. Not a C function: those end with ret.
// Call it from main, once the device is detected (rtc_port etc. not 0):
// until then the startup code points the RTC and keyboard vectors to a
// handler that does nothing. The keyboard has 2 ports: keyboard_port for
// keys pressed, keyboard_port + 1 for keys released. The terminal and
// key_event install their own keyboard handlers.
int set_vector(unsigned char port, void (*handler)(void));


// --- Monitor ---------------------------------------------------------
// Text mode: 42 columns x 24 rows, characters 0x20 (space) to 0x7E (~).
// Pixel mode: 256 x 192 pixels. Each mode has its own picture; the
// text routines work in text mode, the pixel routines in pixel mode.

#define SCREEN_COLUMNS 42
#define SCREEN_ROWS 24
#define SCREEN_WIDTH 256
#define SCREEN_HEIGHT 192

#define BLACK 0
#define BLUE 1
#define GREEN 2
#define CYAN 3
#define RED 4
#define MAGENTA 5
#define BROWN 6
#define LIGHT_GRAY 7
#define DARK_GRAY 8
#define BRIGHT_BLUE 9
#define BRIGHT_GREEN 10
#define BRIGHT_CYAN 11
#define BRIGHT_RED 12
#define BRIGHT_MAGENTA 13
#define BRIGHT_YELLOW 14
#define WHITE 15

// Colors of a text cell: COLORS(WHITE, BLUE) = white on blue
#define COLORS(character, background) (((background) << 4) | (character))

#define PIXEL_MODE 0
#define TEXT_MODE 1

// The mode last set by screen_set_mode or the terminal (TEXT_MODE at
// power-on). Read it, don't write it: use screen_set_mode.
extern unsigned char monitor_mode;

// mode: PIXEL_MODE or TEXT_MODE. Returns a non-zero value if mode is
// neither, or there is no monitor. The terminal switches back to text
// mode on its first output, and on term_clear.
int screen_set_mode(unsigned char mode);
unsigned char screen_get_mode(void);            // PIXEL_MODE or TEXT_MODE

// Text mode

// Returns a non-zero value if x, y is off the screen, c is not
// displayable, or there is no monitor.
int screen_putc(char c, unsigned char x, unsigned char y, unsigned char colors);
// The cell at x, y: character in the low byte, colors in the high byte;
// -1 when off the screen or no monitor
int screen_getc(unsigned char x, unsigned char y);
// Draws length characters from x, y, going on at the start of the next
// row. Returns how many were not drawn (0 = all).
unsigned screen_write(const char *text, unsigned length,
                      unsigned char x, unsigned char y, unsigned char colors);
// Spaces everywhere. Returns a non-zero value if there is no monitor.
int screen_clear(unsigned char colors);
// Background / text color of every cell. Returns a non-zero value if
// color > 15, or there is no monitor.
int screen_set_bg(unsigned char color);
int screen_set_txt(unsigned char color);

// Pixel mode

// Returns a non-zero value if y > 191, color > 15, or there is no monitor.
int screen_put_pixel(unsigned char x, unsigned char y, unsigned char color);
// Returns the color, or -1 if y > 191 or there is no monitor.
int screen_get_pixel(unsigned char x, unsigned char y);
// Returns a non-zero value if color > 15, or there is no monitor.
int screen_fill(unsigned char color);


// --- Terminal ---------------------------------------------------------
// The text console used by putchar, puts and printf (stdio.h): text at
// a cursor, a new row at the end of a row or on '\n', the screen scrolls
// on the last row. The first output switches the monitor to text mode
// and clears the screen.

void term_clear(void);              // clears the screen, cursor at the top left

// If fill = 0, colors affect only the following characters.
// If fill = 1, the whole screen is updating (no data loss).
void term_set_colors(unsigned char colors, unsigned char fill);
void term_prompt(void);             // '>' at the start of the row, cursor after it

// Reads a line typed on the keyboard until Enter (Backspace erases):
// the text, 0-terminated, at most 127 characters, kept until the next
// call. Waits for a keyboard.
char *term_readline(void);
void term_update(void);             // cursor blink and keys; called by term_readline

// Cursor position
void term_get_cursor_pos(char *row, char *column);   // 0, 0 is top left
char term_set_cursor_pos(char row, char column);     // 0, 0 is top left, returns 0 if values are in bound


// --- Real-time clock --------------------------------------------------

struct rtc_time {
    unsigned char year;             // years since 1900: 126 = 2026
    unsigned char month;            // 1-12
    unsigned char day;              // 1-31
    unsigned char hour;             // 0-23
    unsigned char minute;           // 0-59
    unsigned char second;           // 0-59
};

// Returns a non-zero value if there is no RTC.
int rtc_get(struct rtc_time *time);
// Returns a non-zero value if there is no RTC, or the date is invalid.
int rtc_set(const struct rtc_time *time);


// --- EEPROM ------------------------------------------------------------
// 1 MiB of storage that keeps its content when the power is off:
// addresses 0 to 0xFFFFF, all free for the program's data. On the
// hardware it is a file on the I/O processor's SD card. The functions
// return -1 or 1 when there is no EEPROM or the addresses go past its end.

#define EEPROM_SIZE 0x100000L

// Returns the byte, or -1 if address is past the end or there is no EEPROM.
int eeprom_read(unsigned long address);
// Returns a non-zero value if address is past the end or there is no EEPROM.
int eeprom_write(unsigned long address, unsigned char value);
// Copies length bytes from the EEPROM to RAM, or from RAM to the EEPROM.
// Returns a non-zero value if nothing was copied (past the end, no EEPROM).
int eeprom_load(void *ram, unsigned long address, unsigned length);
int eeprom_save(unsigned long address, const void *ram, unsigned length);


// --- Random numbers ----------------------------------------------------
// Extras of rand and srand (stdlib.h): same generator.

// Seeds from the clock. Returns a non-zero value if there is no RTC.
int srand_rtc(void);
// lo to hi (both included), every value equally likely; lo when hi < lo
unsigned long rand_range(unsigned long lo, unsigned long hi);


// --- Keyboard ----------------------------------------------------------
// Key events are kept (up to 15) from the first call to key_event or to
// a terminal function on. Scan codes: Documentation/Peripherals/Keyboard.

#define KEY_RELEASED 0x80   // added to the scan code of a released key

// The oldest key event: its scan code, + KEY_RELEASED when the key was
// released; -1 when there is none. term_readline takes the events too.
int key_event(void);

#endif
