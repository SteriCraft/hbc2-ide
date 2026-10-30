#include <stdarg.h>
#include "compat.h"
#undef open

int compat_open(const char *path, int flags, ...)
{
	int mode = 0;
	if (flags & O_CREAT) {
		va_list ap;
		va_start(ap, flags);
		mode = va_arg(ap, int);
		va_end(ap);
	}
	return _open(path, flags | _O_BINARY, mode ? _S_IREAD | _S_IWRITE : 0);
}

size_t strlcpy(char *dst, const char *src, size_t size)
{
	size_t len = strlen(src);
	if (size) {
		size_t n = len < size - 1 ? len : size - 1;
		memcpy(dst, src, n);
		dst[n] = 0;
	}
	return len;
}

size_t strlcat(char *dst, const char *src, size_t size)
{
	size_t d = strnlen(dst, size);
	if (d == size)
		return size + strlen(src);
	return d + strlcpy(dst + d, src, size - d);
}

static void compat_binary_stdio(void)
{
	_setmode(0, _O_BINARY);
	_setmode(1, _O_BINARY);
	_setmode(2, _O_BINARY);
}

/* Runs before main (C initializer table). */
#pragma section(".CRT$XCU", read)
__declspec(allocate(".CRT$XCU")) void (*compat_init)(void) = compat_binary_stdio;
#pragma comment(linker, "/include:compat_init")
