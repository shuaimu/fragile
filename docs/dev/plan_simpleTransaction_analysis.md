# Analysis: simpleTransaction Build Blockers

## Current Status

simpleTransaction is the first of the core executables that need to be built for G.6.2 Integration Tests. It's currently blocked by multiple issues.

## Changes Made

### 1. common2.h - Added missing STL headers

```cpp
#include <iterator>    // For istream_iterator, back_inserter
#include <algorithm>   // For std::copy
#include <sstream>     // For istringstream
```

### 2. fragile.toml - Updated simpleTransaction include paths

```toml
includes = [
    "examples",
    "src/mako/benchmarks",
    "src/mako/benchmarks/sto",
    "src/mako/benchmarks/sto/masstree-beta",
]
```

## Remaining Blockers

### eRPC Infrastructure Issues
The simpleTransaction includes `mako.hh` which pulls in the full eRPC (Embedded RPC) infrastructure. This causes:

1. **asio headers not found**
   - `'asio/ts/internet.hpp' file not found`
   - `'asio/ts/buffer.hpp' file not found`
   - eRPC uses asio for networking

2. **std::array issues in sslot.h**
   - Multiple `no template named 'array' in namespace 'std'` errors
   - Likely missing `<array>` include in eRPC headers

3. **SSlot member issues**
   - `no member named 'client_info_' in 'erpc::SSlot'`
   - `no member named 'server_info_' in 'erpc::SSlot'`
   - These appear to be conditional compilation issues

### MassTrans.hh Issues
1. **Transaction class access**
   - `calling a private constructor of class 'Transaction'`
   - `variable of type 'Transaction' has private destructor`

2. **Tuple conversion issues in mbta_wrapper.hh**
   - `no viable conversion from 'tuple<...>' to 'const tuple<...>'`
   - `no matching function for call to 'make_tuple'`

### Other Issues
1. **realpath undeclared** in examples/common.h:71
   - stdlib.h is included but realpath may need special feature test macros

2. **operator""s issues** in mako.hh:766
   - C++14 string literals need `<string>` with proper namespace

## Recommended Next Steps

1. **Short-term**: Focus on fixing the many gtest-based tests that are already working. The 54 executables with 880+ tests provide good coverage.

2. **Medium-term**: Add stub infrastructure for simpleTransaction:
   - Create a simplified mako.hh that doesn't include eRPC for basic transaction tests
   - Or mock out the eRPC dependencies

3. **Long-term**: Full eRPC support:
   - Add asio headers to include paths
   - Fix all the conditional compilation issues in eRPC headers
   - This is a significant undertaking

## Date
[26:01:17]
