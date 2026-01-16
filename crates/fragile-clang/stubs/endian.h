// Minimal endian.h stub for fragile parsing
#ifndef _ENDIAN_H
#define _ENDIAN_H

#include "cstdint"

// Byte order definitions
#define __LITTLE_ENDIAN 1234
#define __BIG_ENDIAN    4321
#define __PDP_ENDIAN    3412

// Assume little endian (x86_64)
#define __BYTE_ORDER __LITTLE_ENDIAN

#define LITTLE_ENDIAN __LITTLE_ENDIAN
#define BIG_ENDIAN    __BIG_ENDIAN
#define PDP_ENDIAN    __PDP_ENDIAN
#define BYTE_ORDER    __BYTE_ORDER

// Byte swap builtins
#define __bswap_16(x) __builtin_bswap16(x)
#define __bswap_32(x) __builtin_bswap32(x)
#define __bswap_64(x) __builtin_bswap64(x)

// Host to big-endian conversions
#if __BYTE_ORDER == __LITTLE_ENDIAN
#define htobe16(x) __bswap_16(x)
#define htobe32(x) __bswap_32(x)
#define htobe64(x) __bswap_64(x)
#define htole16(x) (x)
#define htole32(x) (x)
#define htole64(x) (x)
#define be16toh(x) __bswap_16(x)
#define be32toh(x) __bswap_32(x)
#define be64toh(x) __bswap_64(x)
#define le16toh(x) (x)
#define le32toh(x) (x)
#define le64toh(x) (x)
#else
#define htobe16(x) (x)
#define htobe32(x) (x)
#define htobe64(x) (x)
#define htole16(x) __bswap_16(x)
#define htole32(x) __bswap_32(x)
#define htole64(x) __bswap_64(x)
#define be16toh(x) (x)
#define be32toh(x) (x)
#define be64toh(x) (x)
#define le16toh(x) __bswap_16(x)
#define le32toh(x) __bswap_32(x)
#define le64toh(x) __bswap_64(x)
#endif

#endif // _ENDIAN_H
