# CLAUDE.md

Guidance for working in this repository.

## Project Overview

**Fragile** is a C++ → Rust transpiler (with future Go support). It parses C++ with Clang and **generates Rust source code** (often `unsafe`) which is then compiled by `rustc`.

### Vision
- Generate readable, debuggable Rust from real-world C++
- Preserve C++ semantics via explicit `unsafe` blocks and runtime helpers
- Keep the pipeline stable and toolchain-friendly (no rustc internals)

### Current Status
- **C++ Support**: Primary focus (see `TODO.md` and `docs/transpiler-status.md`)
- **Go Support**: Planned via transpiling Go SSA → Rust source (no MIR injection)

---

## Goal and Development Philosophy

The goal is to build a **correct, general-purpose C++ compiler** that works by transpiling to Rust. Every feature should be implemented in the **transpiler itself** (parsing, type resolution, code generation), not by hand-writing target-language code for specific types or libraries.

### Anti-patterns to avoid
- **Hand-writing Rust stubs** for specific C++ types (e.g., hand-writing `__emplace_unique` for `std::map`). If a type doesn't transpile, fix the transpiler bug that blocks it.
- **Treating test cases as the goal**. Tests are drivers to find transpiler bugs, not targets to make pass by any means necessary.
- **Skip-listing instead of fixing**. When generated code is wrong, understand WHY the transpiler produces it and fix the root cause. Rollback patterns are safety nets, not solutions.

### Development approach
- Use real C++ programs as **drivers** to discover transpiler bugs
- Start with simple programs (no STL) and work up to complex ones
- When a driver fails, diagnose which transpiler phase is wrong (parsing, type resolution, code generation) and fix that phase
- Each fix should be general — it should improve transpilation of ALL C++ code, not just the current driver

---

## 🚫 Do NOT use rustc MIR injection

We are **not** pursuing rustc MIR injection or any rustc-private integration. The only supported compilation path is:

```
C++ Source ─► Clang AST ─► Rust Source (unsafe) ─► rustc ─► Binary
```

Avoid:
- rustc private crates (`rustc_driver`, `rustc_interface`, etc.)
- MIR conversion/injection plans
- custom rustc drivers or query overrides

---

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `fragile-clang` | Clang AST → Rust source code generation |
| `fragile-cli` | Command-line interface |
| `fragile-build` | Build config parsing |
| `fragile-common` | Shared utilities |
| `fragile-runtime` | Runtime support (allocation helpers, etc.) |

---

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

# Run with output
cargo test -- --nocapture
```

### Using the CLI

```bash
# Transpile C++ to Rust
fragile transpile file.cpp -o output.rs

# Transpile with include paths
fragile transpile file.cpp -I /path/to/headers -o output.rs

# Transpile with libc++ (recommended for STL code)
fragile transpile file.cpp --use-libcxx -o output.rs
```

### Using libc++ (Optional)

For transpiling code that uses STL containers (`std::vector`, `std::string`, etc.),
we recommend using LLVM's libc++ instead of GCC's libstdc++ because libc++ has
cleaner, more transpiler-friendly code.

```bash
# Install libc++ (Ubuntu/Debian)
sudo apt install libc++-dev libc++abi-dev

# Use libc++ for transpilation
fragile transpile file.cpp --use-libcxx -o output.rs
```

**Why libc++?**
- Designed for Clang (better compatibility)
- Clean, modern code structure
- Minimal compiler intrinsics
- Simpler header organization

---

## Key Files

- `crates/fragile-clang/src/parse.rs` - Clang AST parsing via libclang
- `crates/fragile-clang/src/ast.rs` - Clang AST representation
- `crates/fragile-clang/src/types.rs` - C++ type mappings
- `crates/fragile-clang/src/ast_codegen.rs` - AST → Rust source code generation
- `crates/fragile-cli/src/main.rs` - CLI entry point
- `TODO.md` - Current task list
