# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Fragile** is a polyglot compiler that unifies Rust, C++, and Go at the MIR (Mid-level Intermediate Representation) level. The goal is seamless interoperability between these languages with zero FFI overhead.

### Vision
- All three syntaxes compile to rustc MIR
- No marshalling, no FFI boundaries
- Direct function calls across languages
- Shared memory model

### Current Status
- **C++ Support**: Primary focus, targeting [Mako](https://github.com/makodb/mako) (C++23 database)
- **Rust Support**: Basic support via tree-sitter (legacy) or rustc integration (new)
- **Go Support**: Planned with conservative GC

---

## ⚠️ CRITICAL: Compile-Time vs Link-Time Unification

**The entire point of Fragile is COMPILE-TIME unification, NOT link-time.**

### Link-Time Solutions are WRONG ❌

Any approach where Rust and C++ are compiled separately and only meet at link time is **fundamentally wrong** for this project:

```
WRONG: Link-time approaches
┌─────────────────────────────────────────────────────────────┐
│  C++ ──► [any compiler] ──► .o ──┐                          │
│                                   ├──► linker ──► exe       │
│  Rust ──► rustc ──► .o ──────────┘                          │
│                                                             │
│  Problem: Two separate compilers, code only meets at link   │
│  - No cross-language inlining                               │
│  - No shared optimization passes                            │
│  - No unified analysis (borrow checking C++, etc.)          │
│  - This is just FFI with extra steps!                       │
└─────────────────────────────────────────────────────────────┘
```

**Examples of WRONG approaches:**
1. **clang++ for C++ codegen** - Separate compiler, link-time only
2. **inkwell/LLVM for C++ codegen** - Still a separate compiler! Just because it's written in Rust and uses LLVM doesn't make it "unified"
3. **Any external tool generating .o files** - Same problem

### Compile-Time Solution is CORRECT ✅

```
CORRECT: Compile-time unification via rustc MIR
┌─────────────────────────────────────────────────────────────┐
│  C++ ──► libclang ──► Fragile MIR ──┐                       │
│                                      ├──► rustc MIR ──►     │
│  Rust ──► rustc frontend ───────────┘     rustc backend     │
│                                              │              │
│  Benefits:                                   ▼              │
│  - Single compiler (rustc)              executable          │
│  - Cross-language inlining possible                         │
│  - Shared optimization pipeline                             │
│  - Could extend borrow checker to C++                       │
│  - TRUE unification at IR level                             │
└─────────────────────────────────────────────────────────────┘
```

### Crates Status

| Crate | Approach | Status |
|-------|----------|--------|
| `fragile-codegen` | inkwell/LLVM | ❌ **WRONG** - Deprecated |
| `fragile-frontend-*` | tree-sitter → HIR | ⚠️ Prototyping only |
| `fragile-clang` | libclang → MIR | ✅ **CORRECT** path |
| `fragile-rustc-driver` | MIR injection | ✅ **CORRECT** path |

**When working on this project, focus on `fragile-clang` and `fragile-rustc-driver`, NOT on `fragile-codegen` or the tree-sitter frontends.**

---

## Architecture

### New Architecture (Clang + rustc Integration)

```
C++ Source              Rust Source              Go Source
    │                       │                        │
    ▼                       ▼                        ▼
┌─────────┐           ┌─────────┐              ┌─────────┐
│  Clang  │           │  rustc  │              │ go/ssa  │
│Frontend │           │Frontend │              │ (TBD)   │
└────┬────┘           └────┬────┘              └────┬────┘
     │                     │                        │
     └─────────────────────┼────────────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  rustc MIR  │  ← Unified representation
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │rustc codegen│
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   Binary    │
                    └─────────────┘
```

### Crate Structure

| Crate | Purpose | Status |
|-------|---------|--------|
| `fragile-clang` | Clang AST → MIR conversion | ✅ **CORRECT PATH** |
| `fragile-rustc-driver` | Custom rustc driver with MIR injection | ✅ **CORRECT PATH** |
| `fragile-runtime` | C++/Go runtime support | ✅ Active |
| `fragile-cli` | Command-line interface | ⚠️ Needs update for rustc path |
| `fragile-driver` | Compilation orchestration | ⚠️ Needs update for rustc path |
| `fragile-hir` | HIR definitions | ❌ **WRONG** - Link-time approach |
| `fragile-frontend-rust` | Tree-sitter Rust parser | ❌ **WRONG** - Link-time approach |
| `fragile-frontend-cpp` | Tree-sitter C++ parser | ❌ **WRONG** - Link-time approach |
| `fragile-frontend-go` | Tree-sitter Go parser | ❌ **WRONG** - Link-time approach |
| `fragile-codegen` | HIR → LLVM IR via inkwell | ❌ **WRONG** - Link-time approach |

**The tree-sitter + inkwell path (`fragile-frontend-*` → `fragile-hir` → `fragile-codegen`) is fundamentally wrong because it's a separate compiler that only meets Rust at link time.**

## Build Commands

### Prerequisites
- Rust 1.75+
- LLVM 19
- Clang/libclang (for C++ support)

```bash
# Install libclang (Ubuntu/Debian)
sudo apt install libclang-dev llvm-dev

# Set libclang path for builds
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
```

### Building

```bash
# Build all crates
cargo build

# Build with libclang path (required for fragile-clang)
LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu cargo build

# Build release
cargo build --release
```

### Testing

```bash
# Run all tests
LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu cargo test

# Run tests for specific crate
cargo test --package fragile-clang
cargo test --package fragile-rustc-driver

# Run with output
cargo test -- --nocapture
```

### Using the CLI

```bash
# Build a Rust file (legacy architecture)
cargo run -- build tests/std/01_primitive.rs -o output

# Check a file
cargo run -- check tests/std/01_primitive.rs

# Dump HIR
cargo run -- dump tests/std/01_primitive.rs --format hir
```

## Key Files

### Plans and Documentation
- `TODO.md` - Hierarchical task list with current focus
- `PLAN_CPP20_MAKO.md` - C++20/23 support plan (32 iterations)
- `PLAN_CLANG_RUSTC_INTEGRATION.md` - Clang + rustc architecture
- `PLAN_GO_SUPPORT.md` - Go support with conservative GC

### Source Code

**New Architecture (Clang + rustc):**
- `crates/fragile-clang/src/parse.rs` - Clang AST parsing via libclang
- `crates/fragile-clang/src/convert.rs` - Clang AST → MIR conversion
- `crates/fragile-clang/src/ast.rs` - Clang AST representation
- `crates/fragile-clang/src/types.rs` - C++ type mappings
- `crates/fragile-rustc-driver/src/driver.rs` - Custom rustc driver
- `crates/fragile-rustc-driver/src/queries.rs` - MIR registry
- `crates/fragile-rustc-driver/src/stubs.rs` - Rust stub generation
- `crates/fragile-runtime/src/exceptions.rs` - C++ exception support
- `crates/fragile-runtime/src/memory.rs` - new/delete support
- `crates/fragile-runtime/src/vtable.rs` - Virtual dispatch

**❌ WRONG Architecture (Tree-sitter + inkwell) - DO NOT EXTEND:**
- `crates/fragile-frontend-rust/src/lower.rs` - Rust → HIR (wrong: link-time)
- `crates/fragile-frontend-cpp/src/lower.rs` - C++ → HIR (wrong: link-time)
- `crates/fragile-codegen/src/codegen.rs` - HIR → LLVM IR via inkwell (wrong: separate compiler)

### Test Files
- `tests/std/` - Language feature tests (Rust and C++)
- `tests/clang_integration/` - Clang integration tests
- `vendor/mako/` - Mako project (C++23 test target)

## Development Guidelines

### Current Focus
The primary goal is **C++20/23 support** to compile the Mako project. See `TODO.md` for the current task list.

### Priority Order
1. **C++20/23 Features** - namespaces, classes, templates, coroutines
2. **Mako Compatibility** - compile Mako source files
3. **Go Support** - deferred until C++ is complete

### Code Style
- Follow Rust conventions (rustfmt, clippy)
- Document public APIs with rustdoc
- Add tests for new features

### Adding New C++ Features
1. Check `PLAN_CPP20_MAKO.md` for the feature's phase
2. Add parsing support in `fragile-clang/src/parse.rs`
3. Add AST node in `fragile-clang/src/ast.rs`
4. Add MIR conversion in `fragile-clang/src/convert.rs`
5. Add tests in `tests/clang_integration/`
6. Update `TODO.md` to mark progress

### Testing Against Mako
```bash
# Try to compile a Mako file (goal)
cargo run -- build vendor/mako/src/rrr/misc/rand.cpp

# Milestones:
# M1: rand.cpp (minimal deps)
# M2: rrr/misc/*.cpp (templates, STL)
# M3: rrr/rpc/*.cpp (OOP, threads)
# M4: mako/vec/*.cpp (coroutines)
# M5: Full Mako build
# M6: Mako tests pass
```

## Common Tasks

### Add a new C++ AST node type
1. Add variant to `ClangNodeKind` in `ast.rs`
2. Handle in `convert_cursor_kind()` in `parse.rs`
3. Add MIR conversion in `convert.rs`

### Add a new MIR construct
1. Add to `MirStatement`, `MirTerminator`, or `MirRvalue` in `lib.rs`
2. Update conversion logic in `convert.rs`
3. Ensure rustc driver can handle it

### Debug Clang parsing
```bash
# Use clang to dump AST for reference
clang -Xclang -ast-dump -fsyntax-only file.cpp

# Run fragile-clang tests with output
LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu cargo test --package fragile-clang -- --nocapture
```

## Dependencies

### External
- **clang-sys**: libclang bindings for C++ parsing
- **inkwell**: LLVM bindings for codegen (legacy)
- **tree-sitter**: Parsing (legacy architecture)
- **miette**: Error diagnostics

### Vendored
- `vendor/mako/` - Mako C++23 project (test target)
- `vendor/rust/` - Rust stdlib (planned)
- `vendor/libcxx/` - libc++ (planned)

## Troubleshooting

### libclang not found
```bash
# Find libclang
find /usr -name "libclang*.so" 2>/dev/null

# Set path
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
```

### Tests fail with "libclang not loaded"
```bash
# Always set LIBCLANG_PATH when running tests
LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu cargo test
```

### Build errors with inkwell/LLVM
```bash
# Ensure LLVM 19 is installed
llvm-config --version  # Should show 19.x

# May need to set LLVM path
export LLVM_SYS_191_PREFIX=/usr/lib/llvm-19
```

## Architecture Notes

### Why Clang + rustc?
- **Full C++ support**: Clang handles all C++ complexity
- **Borrow checking**: rustc provides memory safety for Rust
- **Single codegen**: rustc's LLVM backend for everything
- **No fork needed**: Query system override instead of forking rustc

### Why not continue with tree-sitter + inkwell?
**Fundamental problem**: It's a SEPARATE COMPILER that only meets Rust at link time!
- Even if tree-sitter parsing was perfect, inkwell generates LLVM IR independently
- The resulting .o files are combined by the linker, NOT the compiler
- No cross-language optimization, inlining, or analysis possible
- This is just FFI with extra steps - NOT true unification

Additional issues with tree-sitter:
- Can't handle all C++ edge cases
- No type checking, just syntax parsing
- Missing semantic information for templates

### C++ to MIR Flow
```
C++ Source
    │
    ▼ (libclang)
Clang AST
    │
    ▼ (fragile-clang)
Intermediate MIR representation
    │
    ▼ (fragile-rustc-driver)
rustc MIR (injected via query override)
    │
    ▼ (rustc)
Binary
```

### C++ features handled by runtime
Some C++ features need runtime support:
- **Exceptions**: `fragile_rt_throw()`, `fragile_rt_catch()`
- **RAII**: Destructor call helpers
- **Virtual dispatch**: Vtable lookup functions
- **new/delete**: Memory allocation
