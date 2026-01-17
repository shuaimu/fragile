// Minimal netinet/in.h stub for fragile parsing (Internet address structures)
#ifndef _FRAGILE_NETINET_IN_H_
#define _FRAGILE_NETINET_IN_H_

#include "../sys/socket.h"
#include "../cstdint"

// IP protocols
#define IPPROTO_IP 0
#define IPPROTO_ICMP 1
#define IPPROTO_IGMP 2
#define IPPROTO_TCP 6
#define IPPROTO_UDP 17
#define IPPROTO_IPV6 41
#define IPPROTO_ICMPV6 58
#define IPPROTO_RAW 255

// IPv4 address structure
typedef uint32_t in_addr_t;
typedef uint16_t in_port_t;

struct in_addr {
    in_addr_t s_addr;
};

// IPv4 socket address structure
struct sockaddr_in {
    sa_family_t sin_family;
    in_port_t sin_port;
    struct in_addr sin_addr;
    unsigned char sin_zero[8];
};

// IPv6 address structure
struct in6_addr {
    union {
        uint8_t __u6_addr8[16];
        uint16_t __u6_addr16[8];
        uint32_t __u6_addr32[4];
    } __in6_u;
};

#define s6_addr __in6_u.__u6_addr8
#define s6_addr16 __in6_u.__u6_addr16
#define s6_addr32 __in6_u.__u6_addr32

// IPv6 socket address structure
struct sockaddr_in6 {
    sa_family_t sin6_family;
    in_port_t sin6_port;
    uint32_t sin6_flowinfo;
    struct in6_addr sin6_addr;
    uint32_t sin6_scope_id;
};

// Address string length constants
#define INET_ADDRSTRLEN 16
#define INET6_ADDRSTRLEN 46

// Multicast group membership structures
struct ip_mreq {
    struct in_addr imr_multiaddr;  // Multicast group address
    struct in_addr imr_interface;  // Local interface address
};

struct ip_mreqn {
    struct in_addr imr_multiaddr;  // Multicast group address
    struct in_addr imr_address;    // Local interface address
    int imr_ifindex;               // Interface index
};

struct ipv6_mreq {
    struct in6_addr ipv6mr_multiaddr;  // IPv6 multicast address
    unsigned int ipv6mr_interface;     // Interface index
};

// IP socket options
#define IP_OPTIONS 4
#define IP_HDRINCL 3
#define IP_TOS 1
#define IP_TTL 2
#define IP_RECVOPTS 6
#define IP_RETOPTS 7
#define IP_PKTINFO 8
#define IP_PKTOPTIONS 9
#define IP_MTU_DISCOVER 10
#define IP_RECVERR 11
#define IP_RECVTTL 12
#define IP_RECVTOS 13

// Socket options for multicast
#define IP_MULTICAST_IF 32
#define IP_MULTICAST_TTL 33
#define IP_MULTICAST_LOOP 34
#define IP_ADD_MEMBERSHIP 35
#define IP_DROP_MEMBERSHIP 36

#define IPV6_JOIN_GROUP 20
#define IPV6_LEAVE_GROUP 21
#define IPV6_MULTICAST_IF 17
#define IPV6_MULTICAST_HOPS 18
#define IPV6_MULTICAST_LOOP 19
#define IPV6_V6ONLY 26
#define IPV6_UNICAST_HOPS 16
#define IPV6_CHECKSUM 7
#define IPV6_NEXTHOP 9
#define IPV6_RTHDR 57
#define IPV6_HOPOPTS 54
#define IPV6_DSTOPTS 60

// Special addresses
#define INADDR_ANY ((in_addr_t)0x00000000)
#define INADDR_BROADCAST ((in_addr_t)0xffffffff)
#define INADDR_LOOPBACK ((in_addr_t)0x7f000001)
#define INADDR_NONE ((in_addr_t)0xffffffff)

extern const struct in6_addr in6addr_any;
extern const struct in6_addr in6addr_loopback;

#define IN6ADDR_ANY_INIT {{0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0}}
#define IN6ADDR_LOOPBACK_INIT {{0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1}}

// Byte order conversion
extern "C" {
uint32_t htonl(uint32_t hostlong);
uint16_t htons(uint16_t hostshort);
uint32_t ntohl(uint32_t netlong);
uint16_t ntohs(uint16_t netshort);
}

#endif // _FRAGILE_NETINET_IN_H_
