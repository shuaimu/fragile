# Plan: G.5 Build Mako Executables

## Overview

Building Mako executables requires compiling 338 source files and linking them into working binaries. This is a significant task that requires:

1. **Compilation Pipeline**: Parse C++ → Generate stubs → Build objects
2. **Link Pipeline**: Link C++ objects with Rust runtime
3. **Runtime Support**: Ensure fragile-runtime provides necessary symbols

## Current State

- **Parsing**: 338/338 files parse successfully (100%)
- **Stub Generation**: Rust stubs generated for all files
- **Object Compilation**: CppCompiler can build individual .o files
- **Linking**: Basic linking demonstrated in M5.7.3 test harness

## Task Breakdown

### G.5.1 Core Executables (~500 LOC estimated)

#### Subtask G.5.1.1: simpleTransaction Executable

**Dependencies** (from CMakeLists.txt):
- examples/simpleTransaction.cc
- Mako library (libmako)
- RRR library (librrr)
- Third-party: pthread, numa, dpdk (optional)

**Source files needed**:
- src/mako/*.cc (core mako)
- src/rrr/**/*.cc (RRR infrastructure)
- src/deptran/*.cc (transaction protocols)
- src/bench/*.cc (benchmark infrastructure)

**Approach**:
1. Create fragile.toml for simpleTransaction target
2. Use CompilationJob to gather all source files
3. Parse each file with ClangParser
4. Generate combined Rust stubs
5. Compile C++ objects with CppCompiler
6. Link with rustc (extern stubs + C++ objects)

#### Subtask G.5.1.2: dbtest Executable

Similar to simpleTransaction but with more dependencies.

#### Subtask G.5.1.3: simplePaxos Executable

Requires consensus protocol files in addition to base mako.

#### Subtask G.5.1.4: simpleRaft Executable

Requires Raft protocol files.

### G.5.2 Unit Test Executables (~100 LOC per test)

Each test executable is simpler than the core executables:
- test_marshal, test_config_schema, test_fiber, etc.

### G.5.3 Benchmark Executables (~50 LOC per benchmark)

- rpcbench, bench_future

## Implementation Strategy

### Phase 1: Build Pipeline (G.5.1.1)

1. **Create target configuration** (fragile.toml)
   - Define simpleTransaction target
   - List all source dependencies
   - Configure include paths, defines, libs

2. **Build automation**
   - Add build command to fragile-cli
   - Integrate with fragile-build crate
   - Handle dependency resolution

3. **Test with simpleTransaction**
   - Build and link executable
   - Run basic tests

### Phase 2: Scale to More Targets (G.5.1.2-G.5.1.4, G.5.2, G.5.3)

Once simpleTransaction works:
1. Add remaining core executables
2. Add unit test executables
3. Add benchmark executables

## Risks and Mitigations

1. **Missing runtime symbols**: Add stubs to fragile-runtime as needed
2. **Link order issues**: Use topological sort from fragile-build
3. **ABI compatibility**: Ensure C++ name mangling matches

## Estimated LOC

- G.5.1: ~500 LOC (CLI commands, build integration)
- G.5.2: ~200 LOC (test harness extensions)
- G.5.3: ~100 LOC (benchmark extensions)

Total: ~800 LOC

## Next Steps

1. Create fragile.toml for Mako project
2. Add `fragile build` command to CLI
3. Test with simpleTransaction
