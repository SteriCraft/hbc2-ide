/* stddef.h - HBC-2 C library */

#ifndef __STDDEF_H
#define __STDDEF_H

#ifndef NULL
#define NULL ((void *)0)
#endif

#ifndef __SIZE_T
#define __SIZE_T
typedef unsigned size_t;
#endif

typedef int ptrdiff_t;

#define offsetof(type, member) ((size_t)&((type *)0)->member)

#endif
