/* ctype.h - HBC-2 C library: character classes (assembly in
   compiler/lib/asm/ctype.has)

   c is a character (0 to 255) or EOF. The is... functions return 1 when
   c is in the class, 0 otherwise (always 0 for EOF and for values out of
   0-255). Classes follow ASCII; characters 128-255 are in none. */

#ifndef __CTYPE_H
#define __CTYPE_H

int isalnum(int c);     /* a letter or a digit */
int isalpha(int c);     /* a letter: 'A'-'Z', 'a'-'z' */
int islower(int c);     /* 'a'-'z' */
int isupper(int c);     /* 'A'-'Z' */
int isdigit(int c);     /* '0'-'9' */
int isxdigit(int c);    /* a hexadecimal digit: '0'-'9', 'A'-'F', 'a'-'f' */
int isspace(int c);     /* ' ', '\t', '\n', '\v', '\f', '\r' */
int isblank(int c);     /* ' ', '\t' */
int ispunct(int c);     /* printable, not a letter, a digit or a space */
int iscntrl(int c);     /* a control character: 0x00-0x1F, 0x7F */
int isprint(int c);     /* printable, space included: 0x20-0x7E (what the monitor shows) */
int isgraph(int c);     /* printable, space excluded: 0x21-0x7E */

/* The letter in the other case; c unchanged when it is not a letter of
   that case (EOF included). */
int tolower(int c);
int toupper(int c);

#endif
