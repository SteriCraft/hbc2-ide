/* stdarg.h - HBC-2 C library: variable arguments.
   The arguments are on the stack in order, the first one at the lowest
   address (ABI.md, section 5), so the next one starts right after the
   last named one. char arguments arrive as int: va_arg(ap, int). */

#ifndef __STDARG_H
#define __STDARG_H

typedef char *va_list;

#define va_start(ap, last) ((ap) = (char *)&(last) + sizeof(last))
#define va_arg(ap, type) (*(type *)(((ap) += sizeof(type)) - sizeof(type)))
#define va_end(ap)

#endif
