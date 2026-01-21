# Fragile

A unified compiler for **Rust**, **C++20/23**, and **Go** that serves as:

1. **A drop-in replacement** for g++, clang++, and `go build`
2. **A polyglot compiler** for mixing languages with zero-cost interop

## Quick Start

```bash
# As a C++ compiler (replaces g++)
fragile++ main.cpp -o main
./main

# As a Go compiler (replaces go build)
fragile-go build main.go
./main

# As a polyglot compiler
fragile build main.rs utils.cpp helpers.go -o program
./program
```

## Cross-Language Interop

Use C++ STL in Rust:
```rust
use fragile::cpp::std::vector::Vector;
use fragile::cpp::std::map::Map;

fn main() {
    let mut v: Vector<i32> = Vector::new();
    v.push_back(42);

    let mut m: Map<String, i32> = Map::new();
    m.insert("key".into(), 100);
}
```

Use Rust types in C++:
```cpp
#include <fragile/rust/std/vec.hpp>

int main() {
    fragile::rust::std::Vec<int> v;
    v.push(42);
    return v.len();
}
```

**No FFI. No marshalling. Zero overhead.**

## How It Works

Fragile transpiles C++ and Go to Rust, then compiles with rustc:

```
C++ Source ──► Clang ──► AST ──► Transpiler ──► Rust Code ──┐
                                                            ├──► rustc ──► Binary
Rust Source ────────────────────────────────────────────────┘
```

This means:
- **Full C++ support**: Clang handles templates, SFINAE, concepts, coroutines
- **Debuggable output**: Generated `.rs` files can be inspected
- **Stable toolchain**: No dependency on unstable compiler internals

## Building

Prerequisites: LLVM 19, Rust 1.75+, libclang

```bash
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu  # Adjust for your system
cargo build --release
```

## Documentation

See [docs/fragile-book.md](docs/fragile-book.md) for the full specification.

## License

MIT
