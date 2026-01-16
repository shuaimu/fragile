// Minimal string.h stub for fragile parsing (C-style string functions)
#ifndef _FRAGILE_STRING_H_
#define _FRAGILE_STRING_H_

#include "cstdint"

extern "C" {

// String manipulation
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, size_t count);
char* strcat(char* dest, const char* src);
char* strncat(char* dest, const char* src, size_t count);
char* strdup(const char* str);
char* strndup(const char* str, size_t size);

// String examination
size_t strlen(const char* str);
int strcmp(const char* lhs, const char* rhs);
int strncmp(const char* lhs, const char* rhs, size_t count);
int strcoll(const char* lhs, const char* rhs);
size_t strxfrm(char* dest, const char* src, size_t count);
char* strchr(const char* str, int ch);
char* strrchr(const char* str, int ch);
size_t strspn(const char* dest, const char* src);
size_t strcspn(const char* dest, const char* src);
char* strpbrk(const char* dest, const char* breakset);
char* strstr(const char* haystack, const char* needle);
char* strtok(char* str, const char* delim);

// Memory manipulation
void* memcpy(void* dest, const void* src, size_t count);
void* memmove(void* dest, const void* src, size_t count);
void* memset(void* dest, int ch, size_t count);
int memcmp(const void* lhs, const void* rhs, size_t count);
void* memchr(const void* ptr, int ch, size_t count);

// Error string
char* strerror(int errnum);
size_t strerror_r(int errnum, char* buf, size_t buflen);

}

#endif // _FRAGILE_STRING_H_
