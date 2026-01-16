# Plan: M5 - Full Mako Build

## Overview

Milestone M5 is "Full Mako Build" - the goal is to compile all mako C++ source files through the Fragile compiler and produce a working binary.

## Current Status

### Completed
- **Parsing**: 338/338 mako files successfully parsed through fragile-clang
- **MIR Conversion**: All MIR conversion infrastructure in place (convert.rs, mir_convert.rs)
- **rustc Integration**: Query override infrastructure complete with TLS wiring
- **Stub Headers**: Comprehensive stub headers for all standard library and mako dependencies

### Blocked
- `mongodb/server.cc` - Requires external mongocxx/bsoncxx drivers (optional feature)
- `persist_test.cc` - Has undefined template in mako source
- `mtd.cc` - Has epoll.h conflicts

## Prerequisites

1. **Nightly Rust + rustc-dev**: Required for the `rustc-integration` feature
2. **LLVM 19**: For codegen backend
3. **libclang**: For C++ parsing (already set up)

## Implementation Plan

### Phase 1: Simple End-to-End Test (Est: 200 LOC)

1. **M5.1 Simple Add Function Test**
   - Create a simple C++ file: `add.cpp` with `int add(int a, int b) { return a + b; }`
   - Parse through fragile-clang
   - Generate Rust stubs
   - Compile with rustc-integration feature
   - Link and run test

2. **M5.2 Enable CI with Nightly Rust**
   - Update CI to install nightly rust
   - Add `rustup component add rustc-dev llvm-tools-preview`
   - Run tests with `rustc-integration` feature

### Phase 2: Basic Mako Components (Est: 300 LOC)

3. **M5.3 Compile rand.cpp**
   - First real mako file
   - Simple utility functions
   - Test that RandomGenerator works

4. **M5.4 Compile rrr/misc/*.cpp**
   - Basic infrastructure files
   - marshal.cpp, rand.cpp, recorder.cpp

### Phase 3: OOP and Templates (Est: 400 LOC)

5. **M5.5 Compile rrr/rpc/*.cpp**
   - RPC server/client
   - Tests networking infrastructure

6. **M5.6 Compile mako/vec/*.cpp**
   - Coroutine support
   - More complex templates

### Phase 4: Full Build (Est: 500 LOC)

7. **M5.7 Link All Components**
   - Build static library from all parsed files
   - Link with main.cc entry point
   - Produce executable

8. **M5.8 Run Mako Tests**
   - Execute mako's test suite
   - Verify correctness

## Testing Strategy

Each phase will have:
1. Unit tests for new functionality
2. Integration tests that verify parsing + MIR + codegen
3. Runtime tests that execute compiled code

## Timeline Estimate

- Phase 1: 2-3 iterations
- Phase 2: 3-4 iterations
- Phase 3: 3-4 iterations
- Phase 4: 4-5 iterations

Total: ~15 iterations

## Risks and Mitigations

1. **Risk**: Nightly rust breakage
   - Mitigation: Pin to specific nightly version

2. **Risk**: Missing MIR constructs
   - Mitigation: Add constructs incrementally as needed

3. **Risk**: Linker errors from missing symbols
   - Mitigation: Generate comprehensive stubs

## First Steps

1. Create simple `add.cpp` test case
2. Verify rustc-integration feature compiles
3. Run simple end-to-end test
4. Document any missing infrastructure
