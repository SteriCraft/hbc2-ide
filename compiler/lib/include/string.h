// string.h - HBC-2 C library (code in compiler/lib/asm/string.has)

#ifndef __STRING_H
#define __STRING_H

#ifndef NULL
#define NULL ((void *)0)
#endif

#ifndef __SIZE_T
#define __SIZE_T
typedef unsigned size_t;
#endif

size_t 	strlen(const char *s);							// Returns the length of a given string
char* 	strcpy(char *dest, const char *src);			// Copies one string to another
char* 	strncpy(char *dest, const char *src, size_t n);	// Copies a certain amount of characters from one string to another
char* 	strcat(char *dest, const char *src);			// Concatenates two string
char* 	strncat(char *dest, const char *src, size_t n);	// Contatenates a certain amount of characters of two strings
int 	strcmp(const char *a, const char *b);			// Compares two strings
int 	strncmp(const char *a, const char *b, size_t n);// Compares a certain amount of characters of two strings
char* 	strchr(const char *s, int c);					// Finds the first occurrence of a character, otherwise NULL
char* 	strrchr(const char *s, int c);					// Finds the last occurrence of a character, otherwise NULL
char* 	strstr(const char* str, const char* substr);	// Finds the first occurence of a substring, otherwise NULL

void* 	memcpy(void *dest, const void *src, size_t n);
void* 	memmove(void *dest, const void *src, size_t n);
void* 	memset(void *s, int c, size_t n);
int 	memcmp(const void *a, const void *b, size_t n);
void* 	memchr(const void *s, int c, size_t n);

#endif
