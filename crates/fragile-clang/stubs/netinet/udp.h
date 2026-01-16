// Fragile stub header for <netinet/udp.h>
// UDP protocol definitions

#ifndef _NETINET_UDP_H
#define _NETINET_UDP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// UDP header structure
struct udphdr {
    uint16_t uh_sport;    // Source port
    uint16_t uh_dport;    // Destination port
    uint16_t uh_ulen;     // UDP length
    uint16_t uh_sum;      // UDP checksum
};

// Alternative names (BSD style)
#define uh_src  uh_sport
#define uh_dst  uh_dport
#define uh_len  uh_ulen

#ifdef __cplusplus
}
#endif

#endif // _NETINET_UDP_H
