// Minimal string.h stub for fragile parsing (C-style string functions)
// Simply includes cstring to avoid duplicate declarations
#ifndef _FRAGILE_STRING_H_
#define _FRAGILE_STRING_H_

#include "cstring"
#include "strings.h"  // For bzero, bcopy, etc.

// POSIX extensions not in standard cstring
extern "C" {
char* strdup(const char* str);
char* strndup(const char* str, size_t size);
size_t strerror_r(int errnum, char* buf, size_t buflen);
}

#endif // _FRAGILE_STRING_H_
