# RPC Server Infrastructure Plan

## Overview

This plan documents the requirements for implementing RPC server infrastructure
to unblock the remaining Mako test executables and core executables.

## Current Status

- 50 test/benchmark executables built
- 802+ tests passing
- Blocked on RPC server infrastructure for:
  - Core executables (dbtest, simpleTransaction, simplePaxos, simpleRaft)
  - Additional test executables (test_rpc_extended, rpcbench, bench_future)
  - Integration tests (ci.sh tests)

## Key Files

### RPC Server Headers
- `vendor/mako/src/rrr/rpc/server.hpp` (684 lines)
- `vendor/mako/src/rrr/rpc/server.cpp` (714 lines)
- `vendor/mako/test/benchmark_service.h` (478 lines)

### Dependencies
- `rrr.hpp` - Main umbrella header including rpc/server.hpp
- `rpc/client.hpp` - Already parsing successfully
- `reactor/` - Event loop infrastructure (already working)
- `misc/marshal.hpp` - Serialization (already working)

## RPC Server Interface

### Core Classes

1. **Service** (Abstract Base)
   ```cpp
   class Service {
   public:
       virtual ~Service() = default;
       virtual int __reg_to__(Server&, size_t svc_index) = 0;
       virtual void __dispatch__(i32 rpc_id, rusty::Box<Request> req,
                                 WeakServerConnection sconn) = 0;
   };
   ```

2. **Server**
   - Manages listening socket
   - Registers services with RPC ID mapping
   - Handles connection lifecycle
   - Supports graceful shutdown

3. **ServerConnection**
   - Per-client connection handling
   - Request/response marshaling
   - Connection state management

4. **Request/Response**
   ```cpp
   struct Request {
       Marshal m;
       i64 xid;
   };
   ```

### Shutdown Phases
```cpp
enum class ShutdownPhase {
    RUNNING,
    STOP_ACCEPTING,
    DRAINING,
    CLOSING,
    STOPPED
};
```

## Implementation Approach

### Option A: Full Implementation (Recommended)
1. Parse server.hpp with fragile-clang
2. Generate MIR for server components
3. Integrate with rustc driver
4. Link with existing reactor/event infrastructure

**Pros:**
- Full functionality
- Matches original Mako behavior
- Enables all tests

**Cons:**
- Significant implementation effort
- Complex socket/threading code

### Option B: Stub Implementation
1. Create minimal server.hpp stub with interface declarations
2. Provide no-op implementations
3. Use for compile-only verification

**Pros:**
- Quick to implement
- Unblocks parsing of dependent files

**Cons:**
- Tests won't actually work
- Only useful for compilation, not execution

### Option C: Mock Server
1. Create MockServer implementing Service interface
2. Simulate RPC without actual networking
3. Enable unit tests without network I/O

**Pros:**
- Tests can run in isolation
- No networking complexity

**Cons:**
- Different behavior from production
- May miss integration issues

## Recommended Path

1. **Phase 1**: Parse server.hpp with existing fragile-clang
   - Verify no new STL stubs needed
   - Document any parsing issues

2. **Phase 2**: Analyze what minimal server functionality is needed
   - Check which tests actually need server
   - Identify mock-able components

3. **Phase 3**: Implement minimal server stub
   - Interface matching only
   - Allow compilation of test files

4. **Phase 4**: Full server implementation
   - Complete functionality
   - Integration tests passing

## Estimated Effort

- Phase 1: Small (1-2 hours)
- Phase 2: Small (1-2 hours)
- Phase 3: Medium (4-8 hours)
- Phase 4: Large (16-32 hours)

## Parsing Status

**server.cpp parses successfully!** (tested 26:01:17)
- 4688 functions extracted
- No parsing errors
- Uses same include paths as client.cpp

## Next Steps

1. ~~Try parsing server.hpp with fragile-clang~~ ✅ Done
2. ~~Document any parsing errors~~ ✅ None found
3. Add server.cpp to the Mako build configuration
4. Test with benchmark_service.h
5. Enable additional RPC-dependent tests

## Related Documentation

- `docs/dev/plan_m6_mako_tests.md` - Test infrastructure
- `docs/dev/plan_g3_build_system.md` - Build system integration
