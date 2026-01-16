// Minimal execinfo.h stub for fragile parsing
// Use system include guard so we're mutually exclusive with system execinfo.h
#ifndef _EXECINFO_H
#define _EXECINFO_H
#define _FRAGILE_EXECINFO_H_

#include "cstdint"

extern "C" {

// backtrace - get backtrace for current thread
int backtrace(void** buffer, int size);

// backtrace_symbols - translate backtrace addresses to symbol names
char** backtrace_symbols(void* const* buffer, int size);

// backtrace_symbols_fd - write backtrace symbols to file descriptor
void backtrace_symbols_fd(void* const* buffer, int size, int fd);

}

#endif // _EXECINFO_H
