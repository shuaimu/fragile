// Stub header for boost/foreach.hpp
#pragma once

// BOOST_FOREACH is typically used like:
// BOOST_FOREACH(item, container) { ... }
// We can implement it using a range-based for loop wrapper

#define BOOST_FOREACH(VAR, COL) for(VAR : COL)

// Reverse version
#define BOOST_REVERSE_FOREACH(VAR, COL) for(VAR : COL)
