// stdio.h - HBC-2 C library: text on the terminal (see hbc2.h).
// The first output switches the monitor to text mode and clears the
// screen. Without a monitor, output does nothing.

#ifndef __STDIO_H
#define __STDIO_H

#ifndef NULL
#define NULL ((void *)0)
#endif

#define EOF (-1)

// Prints c at the cursor; '\n' starts a new row. Characters other than
// '\n' and 0x20-0x7E are not shown. Returns c (as an unsigned char).
int putchar(int c);

// Prints s, then '\n'. Returns 0.
int puts(const char *s);

// Prints fmt, with the arguments in place of:
// %d (int), %u (unsigned), %x (unsigned, in hexadecimal), %c (char),
// %s (string), %% (a %). l before d, u, x: a long (%ld, %lu, %lx).
// A width before it gives the least number of characters, padded with
// spaces on the left, or with zeros when the width starts with 0
// (%5d, %04x); a - before the width pads on the right instead (%-6s).
// No floating point.
// Returns the number of characters printed.
int printf(const char *fmt, ...);

// Like printf, but writes the characters at buf, then a 0 (not counted).
// buf must have room for them all: nothing checks its size.
int sprintf(char *buf, const char *fmt, ...);

// printf and sprintf with the arguments in ap (stdarg.h: va_list is a
// char *), to write functions like printf:
//     void log(const char *fmt, ...)
//     {
//         va_list ap;
//         va_start(ap, fmt);
//         vprintf(fmt, ap);
//         va_end(ap);
//     }
int vprintf(const char *fmt, char *ap);
int vsprintf(char *buf, const char *fmt, char *ap);

// Prints "s: " (nothing when s is NULL or ""), the text of errno
// (strerror), then a new line.
void perror(const char *s);

// The next character of the line typed on the keyboard: a whole line is
// read (term_readline) when the last one is used up, and its '\n' comes
// after its characters. Waits for a keyboard. After scanf: first the
// character scanf did not use (often the '\n' of the line).
int getchar(void);

// Reads a new line typed into s, without the '\n' (at most 127
// characters: s needs 128 bytes); the rest of the line getchar or scanf
// was reading is dropped. Returns s.
char *gets(char *s);

// Reads fields from the input as fmt says, into the variables the
// arguments point to: sscanf from the string s, scanf from the lines
// typed (like getchar: it waits for more lines while it needs input).
// In fmt:
//   a space (' ', \t to \r)  uses up any spaces of the input, '\n' too
//   another character        must come next in the input
//   %d %u  decimal       %o octal       %x %X %p hexadecimal ("0x" allowed)
//   %i     like a C constant: 0x hexadecimal, a leading 0 octal, else decimal
//   %s     characters up to a space, then a 0 (char *)
//   %c     one character, %5c five (no 0 after them, no space skipped)
//   %[abc] %[a-z] %[^,]  the characters of the set (^: all but these), then a 0
//   %n     the characters read so far (nothing read, not counted)
//   %%     a %
// Between % and the letter: * to read the field without storing it, a
// width (the most characters taken, up to 255), l for a long (%ld %lx
// %ln), h (no effect: short is int). Numbers may start with - or +; a
// number too big keeps its low bits. All but %c, %[ and %n skip spaces
// first. No floating point.
// Returns the number of fields stored, stopping at the first field or
// character that does not match; EOF (-1) when the input ends before the
// first field.
//     int day, month;
//     char name[16];
//     if (sscanf("12/05 bob", "%d/%d %15s", &day, &month, name) == 3) ...
int scanf(const char *fmt, ...);
int sscanf(const char *s, const char *fmt, ...);

#endif
