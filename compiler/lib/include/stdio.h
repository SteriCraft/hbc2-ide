/* stdio.h - HBC-2 C library: text on the terminal (see hbc2.h).
   The first output switches the monitor to text mode and clears the
   screen. Without a monitor, output does nothing. */

#ifndef __STDIO_H
#define __STDIO_H

#ifndef NULL
#define NULL ((void *)0)
#endif

#define EOF (-1)

/* Prints c at the cursor; '\n' starts a new row. Characters other than
   '\n' and 0x20-0x7E are not shown. Returns c (as an unsigned char). */
int putchar(int c);

/* Prints s, then '\n'. Returns 0. */
int puts(const char *s);

/* Prints fmt, with the arguments in place of:
   %d (int), %u (unsigned), %x (unsigned, in hexadecimal), %c (char),
   %s (string), %% (a %). l before d, u, x: a long (%ld, %lu, %lx).
   A width before it gives the least number of characters, padded with
   spaces on the left, or with zeros when the width starts with 0
   (%5d, %04x); a - before the width pads on the right instead (%-6s).
   No floating point.
   Returns the number of characters printed. */
int printf(const char *fmt, ...);

/* The next character of the line typed on the keyboard: a whole line is
   read (term_readline) when the last one is used up, and its '\n' comes
   after its characters. Waits for a keyboard. */
int getchar(void);

/* Reads a line typed into s, without the '\n' (at most 127 characters:
   s needs 128 bytes). Returns s. */
char *gets(char *s);

#endif
