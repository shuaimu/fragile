// Fragile stub header for <sys/ioctl.h>
// I/O control operations

#ifndef _SYS_IOCTL_H
#define _SYS_IOCTL_H

#ifdef __cplusplus
extern "C" {
#endif

// ioctl request codes (common ones)
#define FIONREAD    0x541B
#define FIONBIO     0x5421
#define FIOCLEX     0x5451
#define FIONCLEX    0x5450
#define FIOASYNC    0x5452
#define FIOQSIZE    0x5460

// Terminal I/O
#define TCGETS      0x5401
#define TCSETS      0x5402
#define TCSETSW     0x5403
#define TCSETSF     0x5404
#define TIOCGWINSZ  0x5413
#define TIOCSWINSZ  0x5414

// Network I/O
#define SIOCGIFADDR     0x8915
#define SIOCSIFADDR     0x8916
#define SIOCGIFFLAGS    0x8913
#define SIOCSIFFLAGS    0x8914
#define SIOCGIFNETMASK  0x891b
#define SIOCSIFNETMASK  0x891c
#define SIOCGIFHWADDR   0x8927
#define SIOCGIFINDEX    0x8933

// Window size structure
struct winsize {
    unsigned short ws_row;
    unsigned short ws_col;
    unsigned short ws_xpixel;
    unsigned short ws_ypixel;
};

// ioctl function
int ioctl(int fd, unsigned long request, ...);

#ifdef __cplusplus
}
#endif

#endif // _SYS_IOCTL_H
