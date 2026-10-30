/* Builds the Fuzix Compiler Kit with MSVC. Forced in front of every file (/FI).
   - POSIX I/O names come from <io.h>.
   - Files are opened in binary mode (the stages exchange binary data).
   - stdin/stdout/stderr are switched to binary mode at startup (compat.c). */
#ifndef FCC_COMPAT_H
#define FCC_COMPAT_H
#include <io.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <process.h>

int compat_open(const char *path, int flags, ...);
#define open compat_open

/* The kit has its own itoa / _itoa: rename them past the CRT declarations. */
#define itoa fcc_itoa
#define _itoa fcc__itoa

#define strcasecmp _stricmp
#define strncasecmp _strnicmp

size_t strlcpy(char *dst, const char *src, size_t size);
size_t strlcat(char *dst, const char *src, size_t size);
#endif
