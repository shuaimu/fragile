// Fragile stub header for <netinet/ip.h>
// IP protocol definitions

#ifndef _NETINET_IP_H
#define _NETINET_IP_H

#include <netinet/in.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// IP header structure
struct iphdr {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    unsigned int ihl:4;
    unsigned int version:4;
#else
    unsigned int version:4;
    unsigned int ihl:4;
#endif
    uint8_t  tos;
    uint16_t tot_len;
    uint16_t id;
    uint16_t frag_off;
    uint8_t  ttl;
    uint8_t  protocol;
    uint16_t check;
    uint32_t saddr;
    uint32_t daddr;
};

// Fragment offset mask
#define IP_RF      0x8000      // Reserved fragment flag
#define IP_DF      0x4000      // Don't fragment flag
#define IP_MF      0x2000      // More fragments flag
#define IP_OFFMASK 0x1fff      // Mask for fragmenting bits

// IP protocols
#define IPPROTO_IP       0     // Dummy protocol for TCP
#define IPPROTO_ICMP     1     // ICMP
#define IPPROTO_TCP      6     // TCP
#define IPPROTO_UDP     17     // UDP
#define IPPROTO_RAW    255     // Raw IP packets

// IP type of service
#define IPTOS_LOWDELAY      0x10
#define IPTOS_THROUGHPUT    0x08
#define IPTOS_RELIABILITY   0x04

// IP options
#define IPOPT_EOL    0   // End of option list
#define IPOPT_NOP    1   // No-operation
#define IPOPT_RR     7   // Record route
#define IPOPT_TS    68   // Timestamp
#define IPOPT_LSRR 131   // Loose source route
#define IPOPT_SSRR 137   // Strict source route

// Maximum IP header length
#define MAXTTL      255
#define IPDEFTTL     64  // Default TTL
#define IP_MSS      576  // Default maximum segment size

#ifdef __cplusplus
}
#endif

#endif // _NETINET_IP_H
