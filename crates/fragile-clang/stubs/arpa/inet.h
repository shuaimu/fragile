// Fragile stub header for <arpa/inet.h>
// Network byte order conversion functions

#ifndef _ARPA_INET_H
#define _ARPA_INET_H

#include <stdint.h>
#include <netinet/in.h>  // For in_addr

#ifdef __cplusplus
extern "C" {
#endif

// Convert 16-bit value from host to network byte order
inline uint16_t htons(uint16_t hostshort) {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return ((hostshort & 0xFF00) >> 8) | ((hostshort & 0x00FF) << 8);
#else
    return hostshort;
#endif
}

// Convert 32-bit value from host to network byte order
inline uint32_t htonl(uint32_t hostlong) {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return ((hostlong & 0xFF000000) >> 24) |
           ((hostlong & 0x00FF0000) >> 8) |
           ((hostlong & 0x0000FF00) << 8) |
           ((hostlong & 0x000000FF) << 24);
#else
    return hostlong;
#endif
}

// Convert 16-bit value from network to host byte order
inline uint16_t ntohs(uint16_t netshort) {
    return htons(netshort);
}

// Convert 32-bit value from network to host byte order
inline uint32_t ntohl(uint32_t netlong) {
    return htonl(netlong);
}

// in_addr is now defined in netinet/in.h

// Convert IP address from text to binary form
int inet_pton(int af, const char* src, void* dst);

// Convert IP address from binary to text form
const char* inet_ntop(int af, const void* src, char* dst, uint32_t size);

// Convert IP address from text to binary (deprecated)
uint32_t inet_addr(const char* cp);

// Convert IP address from binary to text (deprecated)
char* inet_ntoa(struct in_addr in);

#ifdef __cplusplus
}
#endif

#endif // _ARPA_INET_H
