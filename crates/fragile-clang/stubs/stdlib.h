// Minimal stdlib.h stub for fragile parsing
#ifndef _FRAGILE_STDLIB_H_
#define _FRAGILE_STDLIB_H_

#include "cstdlib"

// Make all std:: functions available in global namespace
using std::malloc;
using std::calloc;
using std::realloc;
using std::free;
using std::abort;
using std::exit;
using std::atexit;
using std::getenv;
using std::system;
using std::atoi;
using std::atol;
using std::atof;
using std::strtol;
using std::strtoul;
using std::strtod;
using std::rand;
using std::srand;

// BSD random number functions
extern "C" {
long random(void);
void srandom(unsigned int seed);
char* initstate(unsigned int seed, char* state, size_t n);
char* setstate(char* state);
int rand_r(unsigned int* seedp);  // POSIX thread-safe rand
}

using std::qsort;
using std::bsearch;
using std::abs;
using std::labs;

#endif // _FRAGILE_STDLIB_H_
