// Minimal stdio.h stub for fragile parsing
#ifndef _FRAGILE_STDIO_H_
#define _FRAGILE_STDIO_H_

#include "cstdio"

// Make all std:: functions available in global namespace
using std::FILE;
using std::fpos_t;
using std::fopen;
using std::freopen;
using std::fclose;
using std::fflush;
using std::fread;
using std::fwrite;
using std::fgetc;
using std::fgets;
using std::fputc;
using std::fputs;
using std::getc;
using std::getchar;
using std::putc;
using std::putchar;
using std::puts;
using std::ungetc;
using std::scanf;
using std::fscanf;
using std::sscanf;
using std::printf;
using std::fprintf;
using std::sprintf;
using std::snprintf;
using std::vprintf;
using std::vfprintf;
using std::vsprintf;
using std::vsnprintf;
using std::fseek;
using std::ftell;
using std::rewind;
using std::fgetpos;
using std::fsetpos;
using std::clearerr;
using std::feof;
using std::ferror;
using std::perror;
using std::remove;
using std::rename;
using std::tmpfile;
using std::tmpnam;
using std::popen;
using std::pclose;
using std::getline;
using std::getdelim;

#endif // _FRAGILE_STDIO_H_
