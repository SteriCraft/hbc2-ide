// hbc2.h - the HBC-2 hardware from C: I/O ports, interrupts, monitor,
// terminal, real-time clock, EEPROM, random numbers, keyboard.
// Assembly in compiler/lib/asm/hbc2.has.
//
// Functions that can fail return 0 when they worked, and a non-zero
// value when the device is missing or a value is refused.

#ifndef __HBC2_H
#define __HBC2_H


// --- Devices ---------------------------------------------------------
// At startup the library asks the I/O processor (the ESP32 behind the
// mailbox) for its devices, with INFO: each device has a class (PCI base
// class and subclass), a product ID, a first port and a number of ports.
// It waits at most 2 s for the answer; without one there is no device and
// io_protocol() is 0. Assembly in compiler/lib/asm/devices.has.

// Base classes and subclasses (Peripherals/README.md, "Device classes")
#define DEVICE_STORAGE           0x01       // mass storage
#define DEVICE_NETWORK           0x02
#define DEVICE_DISPLAY           0x03
#define DEVICE_SYSTEM            0x08       // system peripheral
#define DEVICE_INPUT             0x09       // input device
#define DEVICE_VGA               0x00       // display: VGA-compatible
#define DEVICE_RTC               0x03       // system: real-time clock
#define DEVICE_KEYBOARD          0x00       // input: keyboard
#define DEVICE_OTHER             0x80       // storage: other (EEPROM, SD card);
                                            // display: other (the native display)
#define DEVICE_ANY               0xFF       // device_find: any subclass

// Product IDs: which device it is (its ports and commands)
#define PRODUCT_MONITOR          0x0001
#define PRODUCT_RTC              0x0002
#define PRODUCT_KEYBOARD         0x0003
#define PRODUCT_EEPROM           0x0004
#define PRODUCT_SD_CARD          0x0005
#define PRODUCT_DISPLAY          0x0006     // the native display

// One device, as INFO describes it
struct device {
    unsigned char base_class;
    unsigned char subclass;
    unsigned int product;
    unsigned char first_port;
    unsigned char ports;                    // number of ports
};

unsigned char io_protocol(void);    // INFO's protocol version, 0: no I/O processor
unsigned char device_count(void);   // devices found (16 at most)
// The index-th device (0 = the first), 0 when there are fewer.
const struct device *device_get(unsigned char index);
// The n-th device (0 = the first) of a class, 0 when there is none.
// subclass can be DEVICE_ANY. device_find(DEVICE_INPUT, DEVICE_KEYBOARD, 0)
// is the first keyboard.
const struct device *device_find(unsigned char base_class,
                                 unsigned char subclass, unsigned char n);

// The first port of the devices the library's drivers use, 0 when the
// device is missing: the first monitor (DEVICE_DISPLAY, DEVICE_VGA: the
// old monitor ports), the first native display (DEVICE_DISPLAY,
// DEVICE_OTHER: the screen functions below; its port is the VSYNC
// event's), the first RTC and keyboard, and the first EEPROM
// (PRODUCT_EEPROM). Set at startup (compiler/runtime/devices.has).

#define MONITOR_PORTS_COUNT      5
#define DISPLAY_PORTS_COUNT      1
#define RTC_PORTS_COUNT          7
#define KEYBOARD_PORTS_COUNT     2
#define EEPROM_PORTS_COUNT       5

extern unsigned char monitor_port;
extern unsigned char display_port;
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
// while it is written. Returns a non-zero value if port is 0 (refused: no
// device has port 0, a port variable is 0 when the device is missing).
// The handler is an assembly routine that ends with irt, declared in C as
// "void name(void);" (label _name): see compiler/ABI.md, section 10. Not
// a C function: those end with ret. The startup code points the vectors
// of every device's ports to a handler that does nothing, unless the
// program set them already. The keyboard has 2 ports: keyboard_port for
// keys pressed, keyboard_port + 1 for keys released. The terminal and
// key_event install their own keyboard handlers.
int set_vector(unsigned char port, void (*handler)(void));


// --- Screen ------------------------------------------------------------
// The native display (Documentation/Peripherals/Native/Reference.md):
// one picture of 256 x 192 pixels, 16 colours. Text is drawn into it in
// 8 x 8 cells, 32 columns x 24 rows, Windows-1252 characters (0x20-0xFF:
// "é" in a string is 0xE9). Text and graphics share the picture.
// The text functions draw in the colours given, then set the text
// colours back to the terminal's (term_set_colors), and leave the cursor
// (the terminal's too) after what they drew.
// Every function returns a non-zero value when there is no native
// display (display_port is 0).

#define SCREEN_COLUMNS 32
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
// The text colours at power-on, after a reset and screen_reset()
#define DEFAULT_COLORS COLORS(BRIGHT_YELLOW, BLUE)

// Text
// c at x, y. Returns a non-zero value if x, y is off the screen or c is a
// control code (below 0x20).
int screen_putc(char c, unsigned char x, unsigned char y, unsigned char colors);
// The cell at x, y: character in the low byte, colors in the high byte;
// -1 when off the screen
int screen_getc(unsigned char x, unsigned char y);
// length characters from x, y, as the terminal prints them: on at the
// start of the next row after column 31, '\n' starts a new row, the
// screen scrolls after the last row. Returns 0, or length when x, y is
// off the screen.
unsigned screen_write(const char *text, unsigned length,
                      unsigned char x, unsigned char y, unsigned char colors);
// Spaces everywhere, cursor at the top left.
int screen_clear(unsigned char colors);
// Background / text color of every cell (characters and cursor kept):
// one RECOLOR command, the I/O processor draws every cell again. Pixels
// drawn over the text are replaced. Returns a non-zero value if
// color > 15.
int screen_set_bg(unsigned char color);
int screen_set_txt(unsigned char color);
// The screen moves up rows text rows (graphics too); the rows at the
// bottom are cleared in the terminal's background.
int screen_scroll(unsigned char rows);
// The glyph of code (0x20-0xFF): 8 bytes, the top row first, bit 7 the
// leftmost pixel. Characters already drawn don't change. Returns a
// non-zero value for a control code.
int screen_define_char(unsigned char code, const unsigned char *rows);
// The default font and colours, cursor at the top left (hidden), the
// screen cleared in blue.
int screen_reset(void);

// Graphics: x 0-255, y 0-191; what falls off the screen is clipped.
// Each returns a non-zero value if color > 15.
// Also non-zero if y > 191.
int screen_put_pixel(unsigned char x, unsigned char y, unsigned char color);
// Returns the color, or -1 if y > 191.
int screen_get_pixel(unsigned char x, unsigned char y);
// The whole picture (the text cells and the cursor don't change).
int screen_fill(unsigned char color);
// From x0, y0 to x1, y1, both ends drawn.
int screen_line(unsigned char x0, unsigned char y0,
                unsigned char x1, unsigned char y1, unsigned char color);
// width x height pixels from x, y: the outline / filled. A size of 0
// means 256: screen_fill_rect(0, y, 0, 8, c) is a full-width band.
int screen_rect(unsigned char x, unsigned char y, unsigned char width,
                unsigned char height, unsigned char color);
int screen_fill_rect(unsigned char x, unsigned char y, unsigned char width,
                     unsigned char height, unsigned char color);


// --- Terminal ---------------------------------------------------------
// The text console used by putchar, puts and printf (stdio.h), at the
// native display's cursor: a new row after column 31 or on '\n', '\b'
// goes back, '\t' to the next column multiple of 8, the screen scrolls
// after the last row. The first output clears the screen and shows the
// cursor (blinking). Its colours: DEFAULT_COLORS until term_set_colors.

void term_clear(void);              // clears the screen, cursor at the top left

// If fill = 0, colors affect only the following characters.
// If fill = 1, the whole screen is updating (no data loss).
void term_set_colors(unsigned char colors, unsigned char fill);
void term_prompt(void);             // '>' at the start of the row, cursor after it

// Reads a line typed on the keyboard until Enter (Backspace erases):
// the text, 0-terminated, at most 127 characters, kept until the next
// call. Waits for a keyboard.
char *term_readline(void);
void term_update(void);             // keys typed; called by term_readline

// Cursor position
// 0, 0 is top left; column 32 after a character in column 31 (the next
// one goes on the next row)
void term_get_cursor_pos(char *row, char *column);
char term_set_cursor_pos(char row, char column);     // 0, 0 is top left, returns 0 if values are in bound


// --- Tick ------------------------------------------------------------
// The native display's frames, 60 per second (the VSYNC event), counted
// from the first call to delay() or clock() (time.h, CLOCKS_PER_SEC 60)
// on. The library then owns the display port's interrupt vector.

// Waits frames sixtieths of a second. Interrupts are on while it waits
// (the frames come as interrupts); afterwards the I flag is back as it
// was. Returns at once without a native display.
void delay(unsigned frames);


// --- Real-time clock --------------------------------------------------

struct rtc_time {
    unsigned char year;             // years since 1900: 126 = 2026
    unsigned char month;            // 1-12
    unsigned char day;              // 1-31
    unsigned char hour;             // 0-23
    unsigned char minute;           // 0-59
    unsigned char second;           // 0-59
};

// The I/O processor's clock (GET_TIME). Returns a non-zero value if
// there is no clock.
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
