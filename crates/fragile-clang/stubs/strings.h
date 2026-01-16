// Fragile stub header for <strings.h>
// String operations (BSD)

#ifndef _STRINGS_H
#define _STRINGS_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// String comparison (case-insensitive)
int strcasecmp(const char* s1, const char* s2);
int strncasecmp(const char* s1, const char* s2, size_t n);

// Bit operations
int ffs(int i);
int ffsl(long i);
int ffsll(long long i);

// Legacy functions (deprecated, use memset/memcpy instead)
void bcopy(const void* src, void* dest, size_t n);
void bzero(void* s, size_t n);
int bcmp(const void* s1, const void* s2, size_t n);

// Find first set bit
char* index(const char* s, int c);
char* rindex(const char* s, int c);

#ifdef __cplusplus
}
#endif

#endif // _STRINGS_H
