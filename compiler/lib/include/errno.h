// errno.h - HBC-2 C library: error numbers (code in compiler/lib/asm/errno.has).
// errno is 0 when the program starts. Library functions set it when they
// fail and never clear it: set errno = 0 before a call, test it after.
// strerror (string.h) gives the text of a number, perror (stdio.h)
// prints it.

#ifndef __ERRNO_H
#define __ERRNO_H

extern int errno;

#define EDOM    1       // argument out of the function's domain (math.h)
#define ERANGE  2       // result out of range: strtol, strtoul

#endif
