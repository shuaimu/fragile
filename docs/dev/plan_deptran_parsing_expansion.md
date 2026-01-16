# Plan: Expand deptran Directory Parsing

## Goal
Expand Mako integration to parse more files from the `vendor/mako/src/deptran/` directory.

## Implementation Summary (Completed 26:01:16, 12:25)

### Stubs Added
1. **Boost stubs**:
   - `boost/any.hpp` - boost::any class with type-erased storage
   - `boost/foreach.hpp` - BOOST_FOREACH macro (maps to range-for)
   - `boost/filesystem.hpp` - path class and filesystem operations
   - `boost/algorithm/string.hpp` - string algorithms (trim, split, join, etc.)

2. **yaml-cpp stub**:
   - `yaml-cpp/yaml.h` - YAML::Node, Load/LoadFile, Emitter

### Files Successfully Parsed
1. `deptran/txn_reg.cc` - 0 functions (empty file)
2. `deptran/troad/tx.cc` - 4754 functions
3. `deptran/janus/tx.cc` - 4754 functions
4. `deptran/rcc/graph_marshaler.cc` - 4754 functions

### Files Requiring External Dependencies
1. `deptran/mongodb/server.cc` - Needs bsoncxx library and rcc_rpc.h (generated RPC header)

### Test Results
- Added 4 new integration tests
- Total tests: 375 passing
- Mako parsing coverage: 121/338 files (~36%)

## Design Rationale

The boost stubs provide minimal implementations that satisfy the compiler without full functionality:
- `boost::any` uses type-erased storage with holder pattern
- `boost::filesystem::path` provides basic path manipulation
- `boost::algorithm` provides common string operations using STL

The yaml-cpp stub provides Node class with basic YAML operations (sequence, map, scalar access) sufficient for header parsing.

## Remaining Work
- Many deptran files require `rcc_rpc.h` (generated from protobuf)
- MongoDB files require bsoncxx C++ driver stubs
- eRPC integration files need eRPC library stubs
