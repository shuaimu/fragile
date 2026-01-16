// Minimal execinfo.h stub for fragile parsing
#ifndef _FRAGILE_EXECINFO_H_
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

#endif // _FRAGILE_EXECINFO_H_
