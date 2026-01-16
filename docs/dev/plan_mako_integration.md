# Plan: Mako Integration

## Overview

Integrate the [Mako](https://github.com/makodb/mako) distributed database (C++23) with the Fragile polyglot compiler.

## Status: PARSING COMPLETE, COMPILATION PENDING

### Milestones

| Milestone | Description | Status |
|-----------|-------------|--------|
| **M1** | Parse `rand.cpp` (minimal deps) | ✅ Complete |
| **M2** | Parse `rrr/misc/*.cpp` (templates, STL) | ✅ Complete |
| **M3** | Parse `rrr/rpc/*.cpp` (OOP, threads) | ✅ Complete |
| **M4** | Parse `mako/vec/*.cpp` (coroutines) | ✅ Complete |
| **M5** | Full Mako build | ⏳ Pending |
| **M6** | Mako tests pass | ⏳ Pending |

### Parsing Statistics

- **Total Mako files**: 338
- **Files with tests**: 338 (100%)
- **Integration tests**: 569 passing
- **Blocked files**: 4 (external dependencies)

### Blocked Files

| File | Reason | Resolution |
|------|--------|------------|
| `mongodb/server.cc` | Needs bsoncxx (MongoDB C++ driver) | Stub or skip |
| `thread.cc` | Needs eRPC rpc.h header | Stub or skip |
| `persist_test.cc` | Undefined `one_way_post` template | Bug in mako |
| `mtd.cc` | sys/epoll.h conflicts | Header fix |

## Stub Headers

Created minimal stub headers in `crates/fragile-clang/stubs/` to handle libstdc++ incompatibilities:

| Header | Purpose |
|--------|---------|
| `cstdint` | Basic integer types |
| `inttypes.h` | Format macros |
| `random` | std::mt19937, distributions |
| `gflags/gflags.h` | Command-line flags |
| `event2/event.h` | libevent |
| `gperftools/malloc_extension.h` | Memory profiling |
| `rocksdb/db.h` | RocksDB |
| `boost/smart_ptr/intrusive_ref_counter.hpp` | Boost intrusive_ptr |
| `yaml-cpp/yaml.h` | YAML parsing |
| `pthread.h` | POSIX threads |

---

## M5: Full Mako Build

### Requirements

To achieve M5 (Full Mako build), the following must be completed:

1. **MIR Conversion Integration**
   - Wire up `mir_convert.rs` to rustc query overrides
   - Implement function call resolution (string → DefId)
   - Handle C++ name mangling

2. **Type Resolution**
   - Map C++ types to Rust types
   - Handle struct layouts
   - Support template instantiations

3. **Linking**
   - Link C++ MIR with Rust main
   - Handle symbol visibility
   - Resolve cross-module references

### Architecture

```
C++ Source Files
    │
    ▼ (fragile-clang)
MIR Bodies
    │
    ▼ (CppMirRegistry)
Registry Storage
    │
    ▼ (mir_convert.rs)
rustc MIR
    │
    ▼ (rustc query override)
Injected into compilation
    │
    ▼ (rustc codegen)
Binary
```

### Key Files

| File | Purpose |
|------|---------|
| `fragile-clang/src/parse.rs` | C++ parsing via libclang |
| `fragile-clang/src/convert.rs` | AST → MIR conversion |
| `fragile-rustc-driver/src/mir_convert.rs` | MIR → rustc MIR conversion |
| `fragile-rustc-driver/src/rustc_integration.rs` | rustc callbacks |
| `fragile-rustc-driver/src/queries.rs` | MIR registry |
| `fragile-rustc-driver/src/stubs.rs` | Rust stub generation |

---

## M6: Mako Tests Pass

### Requirements

To achieve M6 (Mako tests pass):

1. **Runtime Support**
   - Implement exception handling
   - Implement new/delete
   - Implement vtable dispatch

2. **Test Harness**
   - Integrate with Mako's test framework
   - Handle test discovery
   - Report results

3. **Debugging**
   - Source mapping for errors
   - Stack traces
   - Breakpoint support

---

## History

### Initial Integration (26:01:16, 04:15)

Solved libstdc++ parsing issues by creating stub headers. Successfully parsed rand.cpp with 225 functions.

### Bulk Parsing (26:01:16, 05:00-20:00)

Added tests for all 338 mako files. Achieved 100% test coverage. Identified 4 blocked files.

### MIR Conversion (26:01:16, 23:00)

Implemented ~290 LOC of MIR conversion code supporting all basic MIR constructs.
