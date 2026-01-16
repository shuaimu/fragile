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
using std::qsort;
using std::bsearch;
using std::abs;
using std::labs;

#endif // _FRAGILE_STDLIB_H_
