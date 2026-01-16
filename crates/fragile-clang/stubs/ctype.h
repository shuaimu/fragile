// Minimal ctype.h stub for fragile parsing
#ifndef _FRAGILE_CTYPE_H_
#define _FRAGILE_CTYPE_H_

#include "cctype"

// Make std:: functions available in global namespace
using std::isalnum;
using std::isalpha;
using std::isblank;
using std::iscntrl;
using std::isdigit;
using std::isgraph;
using std::islower;
using std::isprint;
using std::ispunct;
using std::isspace;
using std::isupper;
using std::isxdigit;
using std::tolower;
using std::toupper;

#endif // _FRAGILE_CTYPE_H_
