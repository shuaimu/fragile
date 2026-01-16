// Minimal netinet/tcp.h stub for fragile parsing (TCP options)
#ifndef _FRAGILE_NETINET_TCP_H_
#define _FRAGILE_NETINET_TCP_H_

#include "../cstdint"

// TCP socket options (for setsockopt/getsockopt with level IPPROTO_TCP)
#define TCP_NODELAY 1           // Disable Nagle's algorithm
#define TCP_MAXSEG 2            // MSS value
#define TCP_CORK 3              // Cork sending
#define TCP_KEEPIDLE 4          // Idle time before keepalive probes
#define TCP_KEEPINTVL 5         // Interval between keepalive probes
#define TCP_KEEPCNT 6           // Number of keepalive probes
#define TCP_SYNCNT 7            // Number of SYN retransmits
#define TCP_LINGER2 8           // FIN-WAIT2 timeout
#define TCP_DEFER_ACCEPT 9      // Defer accept until data arrives
#define TCP_WINDOW_CLAMP 10     // Clamp receive window
#define TCP_INFO 11             // TCP connection info
#define TCP_QUICKACK 12         // Quick ACK mode
#define TCP_CONGESTION 13       // Congestion control algorithm
#define TCP_MD5SIG 14           // TCP MD5 signature
#define TCP_THIN_LINEAR_TIMEOUTS 16
#define TCP_THIN_DUPACK 17
#define TCP_USER_TIMEOUT 18     // Maximum time without acknowledgment
#define TCP_REPAIR 19
#define TCP_REPAIR_QUEUE 20
#define TCP_QUEUE_SEQ 21
#define TCP_REPAIR_OPTIONS 22
#define TCP_FASTOPEN 23         // TCP Fast Open
#define TCP_TIMESTAMP 24
#define TCP_NOTSENT_LOWAT 25    // Low watermark for not-yet-sent data
#define TCP_CC_INFO 26
#define TCP_SAVE_SYN 27
#define TCP_SAVED_SYN 28
#define TCP_REPAIR_WINDOW 29
#define TCP_FASTOPEN_CONNECT 30
#define TCP_ULP 31
#define TCP_MD5SIG_EXT 32
#define TCP_FASTOPEN_KEY 33
#define TCP_FASTOPEN_NO_COOKIE 34
#define TCP_ZEROCOPY_RECEIVE 35

// TCP states
#define TCP_ESTABLISHED 1
#define TCP_SYN_SENT 2
#define TCP_SYN_RECV 3
#define TCP_FIN_WAIT1 4
#define TCP_FIN_WAIT2 5
#define TCP_TIME_WAIT 6
#define TCP_CLOSE 7
#define TCP_CLOSE_WAIT 8
#define TCP_LAST_ACK 9
#define TCP_LISTEN 10
#define TCP_CLOSING 11

// TCP info structure (for TCP_INFO option)
struct tcp_info {
    uint8_t tcpi_state;
    uint8_t tcpi_ca_state;
    uint8_t tcpi_retransmits;
    uint8_t tcpi_probes;
    uint8_t tcpi_backoff;
    uint8_t tcpi_options;
    uint8_t tcpi_snd_wscale : 4;
    uint8_t tcpi_rcv_wscale : 4;
    uint8_t tcpi_delivery_rate_app_limited : 1;
    uint32_t tcpi_rto;
    uint32_t tcpi_ato;
    uint32_t tcpi_snd_mss;
    uint32_t tcpi_rcv_mss;
    uint32_t tcpi_unacked;
    uint32_t tcpi_sacked;
    uint32_t tcpi_lost;
    uint32_t tcpi_retrans;
    uint32_t tcpi_fackets;
    uint32_t tcpi_last_data_sent;
    uint32_t tcpi_last_ack_sent;
    uint32_t tcpi_last_data_recv;
    uint32_t tcpi_last_ack_recv;
    uint32_t tcpi_pmtu;
    uint32_t tcpi_rcv_ssthresh;
    uint32_t tcpi_rtt;
    uint32_t tcpi_rttvar;
    uint32_t tcpi_snd_ssthresh;
    uint32_t tcpi_snd_cwnd;
    uint32_t tcpi_advmss;
    uint32_t tcpi_reordering;
    uint32_t tcpi_rcv_rtt;
    uint32_t tcpi_rcv_space;
    uint32_t tcpi_total_retrans;
};

#endif // _FRAGILE_NETINET_TCP_H_
