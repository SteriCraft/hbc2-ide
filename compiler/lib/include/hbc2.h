/* hbc2.h - the HBC-2 hardware from C: I/O ports, interrupts, monitor,
   terminal, real-time clock, keyboard, EEPROM. Assembly in compiler/lib/asm/hbc2.has.

   Functions returning "0, or 1" give 1 when the device is missing or the
   values are refused. */

#ifndef __HBC2_H
#define __HBC2_H


/* --- Devices ---------------------------------------------------------
   The first port of each device, 0 when it is not plugged. Set at
   startup (compiler/runtime/devices.has). */

extern unsigned char monitor_port;
extern unsigned char rtc_port;
extern unsigned char keyboard_port;
extern unsigned char eeprom_port;


/* --- I/O ports and interrupts --------------------------------------- */

unsigned char in(unsigned char port);           /* the value port holds */
void out(unsigned char port, unsigned char value);

void interrupts_off(void);                      /* cli */
void interrupts_on(void);                       /* sti */
void wait_interrupt(void);  /* hlt: sleeps until the next interrupt (enables them) */

/* Points the interrupt vector of port to handler, with interrupts off
   while it is written. Returns 0, or 1 for port 0 (device detection, kept
   by the startup code). The handler is an assembly routine that ends with
   irt, declared in C as "void name(void);" (label _name): see
   compiler/ABI.md, section 10. Not a C function: those end with ret.
   Call it from main, once the device is detected (rtc_port etc. not 0):
   until then the startup code points the RTC and keyboard vectors to a
   handler that does nothing. The keyboard has 2 ports: keyboard_port for
   keys pressed, keyboard_port + 1 for keys released. The terminal and
   key_event install their own keyboard handlers. */
int set_vector(unsigned char port, void (*handler)(void));


/* --- Monitor ---------------------------------------------------------
   Text mode: 42 columns x 24 rows, characters 0x20 (space) to 0x7E (~).
   Pixel mode: 256 x 192 pixels. Each mode has its own picture; the
   text routines work in text mode, the pixel routines in pixel mode. */

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

/* Colors of a text cell: COLORS(WHITE, BLUE) = white on blue */
#define COLORS(character, background) (((background) << 4) | (character))

int screen_text_mode(void);                     /* 0, or 1 */
int screen_pixel_mode(void);                    /* 0, or 1 */

/* Text mode */
int screen_putc(char c, unsigned char x, unsigned char y, unsigned char colors);
/* The cell at x, y: character in the low byte, colors in the high byte;
   -1 when off the screen or no monitor */
int screen_getc(unsigned char x, unsigned char y);
/* Draws length characters from x, y, going on at the start of the next
   row. Returns how many were not drawn (0 = all). */
unsigned screen_write(const char *text, unsigned length,
                      unsigned char x, unsigned char y, unsigned char colors);
int screen_clear(unsigned char colors);         /* spaces everywhere; 0, or 1 */
int screen_set_bg(unsigned char color);         /* background of every cell; 0, or 1 */

/* Pixel mode */
int screen_put_pixel(unsigned char x, unsigned char y, unsigned char color);
int screen_get_pixel(unsigned char x, unsigned char y);   /* color, or -1 */
int screen_fill(unsigned char color);           /* 0, or 1 */


/* --- Terminal ---------------------------------------------------------
   The text console used by putchar, puts and printf (stdio.h): text at
   a cursor, a new row at the end of a row or on '\n', the screen scrolls
   on the last row. The first output switches the monitor to text mode
   and clears the screen. */

void term_clear(void);              /* clears the screen, cursor at the top left */
void term_set_colors(unsigned char colors);     /* for the text printed next */
void term_prompt(void);             /* '>' at the start of the row, cursor after it */
/* Reads a line typed on the keyboard until Enter (Backspace erases):
   the text, 0-terminated, at most 127 characters, kept until the next
   call. Waits for a keyboard. */
char *term_readline(void);
void term_update(void);             /* cursor blink and keys; called by term_readline */


/* --- Real-time clock -------------------------------------------------- */

struct rtc_time {
    unsigned char year;             /* years since 1900: 126 = 2026 */
    unsigned char month;            /* 1-12 */
    unsigned char day;              /* 1-31 */
    unsigned char hour;             /* 0-23 */
    unsigned char minute;           /* 0-59 */
    unsigned char second;           /* 0-59 */
};

int rtc_get(struct rtc_time *time);             /* 0, or 1 */
int rtc_set(const struct rtc_time *time);       /* 0, or 1 (no RTC, invalid date) */


/* --- EEPROM ------------------------------------------------------------
   1 MiB that keeps its content when the power is off: addresses 0 to
   0xFFFFF. Plugged when the program is built for the EEPROM (see
   compiler/ABI.md, section 8): the program itself is at the start of
   the EEPROM, at the same addresses as in RAM ($0x0000 to $0xFFFF), so
   keep your data from EEPROM_FREE on. The functions return -1 or 1 when
   there is no EEPROM or the addresses go past its end. */

#define EEPROM_SIZE 0x100000L
#define EEPROM_FREE 0x10000L        /* the first address after a program */

int eeprom_read(unsigned long address);         /* the byte, or -1 */
int eeprom_write(unsigned long address, unsigned char value);   /* 0, or 1 */
/* Copies length bytes from the EEPROM to RAM, or from RAM to the EEPROM:
   0, or 1 (nothing copied). */
int eeprom_load(void *ram, unsigned long address, unsigned length);
int eeprom_save(unsigned long address, const void *ram, unsigned length);


/* --- Keyboard ----------------------------------------------------------
   Key events are kept (up to 15) from the first call to key_event or to
   a terminal function on. Scan codes: Documentation/Peripherals/Keyboard. */

#define KEY_RELEASED 0x80   /* added to the scan code of a released key */

/* The oldest key event: its scan code, + KEY_RELEASED when the key was
   released; -1 when there is none. term_readline takes the events too. */
int key_event(void);

#endif
