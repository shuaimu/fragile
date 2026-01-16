# Fragile Language Specification

**Fragile** is a polyglot compiler that unifies Rust, C++20/23, and Go at the MIR level for seamless interoperability.

---

## Table of Contents

- [Part I: Introduction and Getting Started](#part-i-introduction-and-getting-started)
  - [Introduction](#introduction)
  - [Getting Started](#getting-started)
- [Part II: The Unified Model](#part-ii-the-unified-model)
  - [Philosophy](#philosophy)
  - [Memory Model](#memory-model)
  - [Type System](#type-system)
  - [Error Handling](#error-handling)
- [Part III: Syntax Mapping](#part-iii-syntax-mapping)
  - [Rust Syntax](#rust-syntax)
  - [C++ Syntax](#c-syntax)
  - [Go Syntax](#go-syntax)
- [Part IV: Cross-Language Programming](#part-iv-cross-language-programming)
  - [Calling Functions](#calling-functions)
  - [Sharing Types](#sharing-types)
  - [Generics](#generics)
- [Part V: Reference](#part-v-reference)
  - [Built-in Types](#built-in-types)
  - [Operators](#operators)
  - [Attributes](#attributes)
  - [C ABI](#c-abi)
- [Part VI: Appendix](#part-vi-appendix)
  - [Syntax Comparison](#syntax-comparison)
  - [Differences from Native Compilers](#differences-from-native-compilers)
  - [Migration Guide](#migration-guide)
  - [Ecosystem Compatibility](#ecosystem-compatibility)
  - [Architecture Details](#architecture-details)

---

# Part I: Introduction and Getting Started

## Introduction

Fragile is a polyglot compiler that compiles Rust, C++, and Go sources into a single binary. Rather than being "three languages glued together," Fragile leverages established compiler infrastructure (Clang for C++, rustc for Rust) and unifies them at the MIR (Mid-level Intermediate Representation) level.

```
┌─────────────────────────────────────────────────────────────┐
│                    User Code                                │
│    .rs files      .cpp/.cc files       .go files           │
└─────────┬─────────────┬─────────────────┬──────────────────┘
          │             │                 │
          ▼             ▼                 ▼
    ┌──────────┐  ┌──────────┐     ┌──────────┐
    │  rustc   │  │  Clang   │     │  go/ssa  │
    │ Frontend │  │ Frontend │     │  (TBD)   │
    └────┬─────┘  └────┬─────┘     └────┬─────┘
         │             │                 │
         └─────────────┼─────────────────┘
                       ▼
              ┌─────────────────┐
              │    rustc MIR    │  ← Unified representation
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │  rustc codegen  │
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │     Binary      │
              └─────────────────┘
```

### Why Fragile?

Modern software projects often need to combine code from different ecosystems. Today, combining these requires FFI, manual memory management at boundaries, type marshalling, and runtime overhead. Fragile eliminates these barriers:

```rust
// math.rs
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```go
// main.go
import "math.rs"

func main() {
    result := add(2, 3)  // Direct call, no FFI
}
```

**No overhead. No marshalling. Just code.**

### Key Features

1. **Production-Grade Frontends**: Uses Clang for C++ and rustc for Rust (not custom parsers)
2. **Unified MIR**: All languages compile to rustc MIR for a single codegen path
3. **Zero-Cost Interop**: Cross-language function calls have no FFI overhead
4. **Full Ecosystem Access**: Use crates.io, C++ libraries, and Go modules (planned)
5. **C++20/23 Support**: Full modern C++ including templates, concepts, and coroutines
6. **Memory Model Per Language**: Rust uses ownership, C++ uses RAII, Go uses conservative GC

### Language Detection

| Extension | Syntax |
|-----------|--------|
| `.rs` | Rust |
| `.cpp`, `.cc`, `.cxx`, `.hpp` | C++ |
| `.go` | Go |

## Getting Started

### Installation

**Prerequisites:**
- Rust 1.75+ (nightly recommended for rustc-dev features)
- LLVM 19
- Clang/libclang (for C++ support)

```bash
# Ubuntu/Debian: Install dependencies
sudo apt install libclang-dev llvm-dev

# Clone and build
git clone https://github.com/fragile-lang/fragile
cd fragile

# Set libclang path (required)
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu

# Build
cargo build --release
```

**Note:** The `LIBCLANG_PATH` environment variable must be set for C++ compilation support.

### Your First Program

```rust
// hello.rs
fn main() {
    print("Hello, Fragile!");
}
```

Or Go syntax:
```go
package main

func main() {
    print("Hello, Fragile!")
}
```

Or C++:
```cpp
int main() {
    print("Hello, Fragile!");
    return 0;
}
```

Compile: `fragile hello.rs -o hello`

### Mixed-Language Project

**math.rs:**
```rust
pub fn square(x: i32) -> i32 {
    x * x
}
```

**utils.cpp:**
```cpp
#include "math.rs"

int sum_of_squares(int a, int b) {
    return square(a) + square(b);
}
```

**main.go:**
```go
package main

import "utils.cpp"

func main() {
    result := sum_of_squares(3, 4)
    print(result)  // Prints: 25
}
```

Compile: `fragile main.go math.rs utils.cpp -o program`

---

# Part II: The Unified Model

## Philosophy

### One Language, Three Syntaxes

Fragile is built on a simple idea: **syntax is just notation**. The same program can be written in any syntax:

```rust
// Rust syntax
fn factorial(n: i32) -> i32 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

```cpp
// C++ syntax
int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}
```

```go
// Go syntax
func factorial(n int32) int32 {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
```

All three compile to identical IR and produce identical machine code.

### Semantic Preservation

Fragile preserves each language's native semantics rather than forcing a single model:

| Concept | Rust | C++ | Go |
|---------|------|-----|-----|
| Memory | Ownership/borrow | RAII | Conservative GC |
| Errors | `Result<T, E>` | Exceptions | `(T, error)` |
| Null | `Option<T>` | `nullptr`/`optional` | `nil` |
| Generics | Monomorphization | Templates | Type parameters |

**Key Insight:** Rather than imposing one language's model on others, Fragile compiles each language with its native semantics. Interoperability is achieved at the MIR level.

### Source-First Ecosystem

Fragile compiles everything from source:
- **Full ecosystem access**: Use any Rust crate, Go module, or C++ library
- **No ABI concerns**: No stable Rust ABI needed
- **Maximum optimization**: Compiler sees all code

## Memory Model

Fragile supports **multiple memory management strategies** depending on the source language.

### Rust: Ownership

Rust code uses ownership-based memory management with compile-time borrow checking.

```rust
let s = String::from("hello");
let t = s;  // ownership moves to t
// s is no longer valid

let u = t.clone();  // Explicit copy
```

**Borrowing Rules:**
1. One mutable borrow OR multiple immutable borrows (not both)
2. References must not outlive the data
3. Data cannot be modified while immutably borrowed

### C++: RAII

C++ code uses standard RAII (Resource Acquisition Is Initialization) semantics. Destructors are called when objects go out of scope, just like in standard C++.

```cpp
std::string s1 = "hello";
std::string s2 = s1;  // Copy (standard C++ behavior)
std::string s3 = std::move(s1);  // Move (standard C++ behavior)
// s1 is in moved-from state
```

**Key Point:** Fragile does not modify C++ copy/move semantics. C++ code behaves as standard C++ compilers would compile it.

### Go: Conservative GC

Go code uses a **conservative garbage collector** for memory management. This preserves Go's programming model where values can be freely copied and shared.

```go
s := "hello"
t := s  // Both valid (Go semantics preserved)
```

**Note:** The conservative GC scans the stack and heap for potential pointers, enabling Go code to work naturally without ownership annotations.

### Cross-Language Boundaries

When calling between languages, memory ownership follows the callee's conventions:
- Passing to Rust: Caller transfers ownership (Rust may borrow check)
- Passing to C++: Standard C++ semantics apply
- Passing to Go: GC will track the memory

## Type System

Fragile has a unified type system shared across all syntaxes.

### Primitive Types

| Fragile | Rust | C++ | Go | Size |
|---------|------|-----|----|----|
| `i32` | `i32` | `int` | `int32` | 32-bit signed |
| `i64` | `i64` | `long` | `int64` | 64-bit signed |
| `f64` | `f64` | `double` | `float64` | 64-bit float |
| `bool` | `bool` | `bool` | `bool` | Boolean |
| `char` | `char` | `char32_t` | `rune` | Unicode scalar |

### Compound Types

**Arrays**: `[T; N]` / `std::array<T, N>` / `[N]T`

**Slices**: `&[T]` / `std::span<T>` / `[]T`

**Tuples**: `(T1, T2)` / `std::tuple<T1, T2>` / Use struct

### Structs

These three definitions create **the same type**:

```rust
struct Point { x: f64, y: f64 }
```
```cpp
struct Point { double x; double y; };
```
```go
type Point struct { x, y float64 }
```

### Option and Result

| Type | Rust | C++ | Go |
|------|------|-----|-----|
| `Option<T>` | `Option<T>` | `std::optional<T>` | `*T` (nil-able) |
| `Result<T,E>` | `Result<T,E>` | exceptions convert | `(T, error)` |

## Error Handling

Each language uses its **native error handling mechanism**.

### Rust: Result<T, E>
```rust
fn parse(s: &str) -> Result<i32, ParseError> {
    s.parse().map_err(|_| ParseError::InvalidFormat)
}
let n = parse("42")?;  // Propagate with ?
```

### C++: Exceptions
```cpp
int parse(const std::string& s) {
    if (s.empty()) throw ParseError{"empty"};
    return std::stoi(s);
}

// Standard try-catch works
try {
    int n = parse(input);
} catch (const ParseError& e) {
    // Handle error
}
```

### Go: Error Returns
```go
func parse(s string) (int, error) {
    if s == "" {
        return 0, errors.New("empty")
    }
    return strconv.Atoi(s)
}

// Standard Go error handling
n, err := parse(input)
if err != nil {
    // Handle error
}
```

### Cross-Language Error Handling

When calling between languages, errors follow the callee's conventions:
- Rust → C++: Result::Err can be caught as exception (planned)
- C++ → Rust: Exceptions convert to Result::Err (planned)
- Go → Rust: (T, error) converts to Result<T, E> (planned)

---

# Part III: Syntax Mapping

## Rust Syntax

Rust syntax is the most direct mapping to Fragile's internal representation.

### Supported Features

- Functions, methods, closures
- Structs, enums, traits
- Pattern matching
- Ownership and borrowing
- Error handling with Result/?
- Modules and use declarations
- Generics with trait bounds

### Imports from Other Syntaxes
```rust
mod physics;  // looks for physics.cpp or physics.rs
use physics::Vector3;
```

## C++ Syntax

Fragile supports **C++20/23** with full semantic compatibility. C++ code is parsed using Clang, ensuring complete language support.

### Supported Features

| Category | Features |
|----------|----------|
| Core | Classes, inheritance, virtual functions, RTTI |
| Templates | Function/class templates, SFINAE, concepts, variadic templates |
| Memory | RAII, smart pointers, new/delete, placement new |
| Modern | Coroutines (co_await), lambdas, ranges, modules |
| Concurrency | std::thread, std::mutex, atomics, futures |

### Standard C++ Behavior

C++ code in Fragile behaves exactly like standard C++:

```cpp
std::string s1 = "hello";
std::string s2 = s1;  // Copy (standard behavior)
std::string s3 = std::move(s1);  // Move

// Exceptions work normally
try {
    throw std::runtime_error("error");
} catch (const std::exception& e) {
    // ...
}
```

### Templates and Concepts
```cpp
template<typename T>
requires std::totally_ordered<T>
T max(T a, T b) { return a > b ? a : b; }

// Variadic templates
template<typename... Args>
void print_all(Args... args) { (std::cout << ... << args); }
```

### Coroutines (C++20)
```cpp
task<int> fetch_data() {
    auto result = co_await async_fetch();
    co_return result.value();
}
```

### Cross-Language Imports
```cpp
#include "math.rs"    // Import Rust module
#include "utils.go"   // Import Go module (planned)
```

### Test Target: Mako

Fragile's C++ support is validated against [Mako](https://github.com/makodb/mako), a C++23 distributed transactional database. This ensures real-world C++ code compiles correctly.

## Go Syntax

Go syntax with **conservative garbage collection**, preserving Go's programming model.

### Key Design: Standard Go Semantics

| Feature | Standard Go | Fragile Go |
|---------|-------------|------------|
| Memory | GC | Conservative GC |
| Assignment | Copy/share | Same (preserved) |
| `nil` | Nil pointer | Same (preserved) |
| Goroutines | Runtime scheduler | Planned |

### Standard Go Behavior
```go
s1 := "hello"
s2 := s1  // Both valid (standard Go behavior)

// Nil works normally
var p *int
if p == nil {
    // ...
}
```

### Error Handling
```go
func readFile(path string) (string, error) {
    // Standard Go error handling preserved
    if err != nil {
        return "", err
    }
    return content, nil
}
```

### Cross-Language Imports
```go
import "math.rs"       // Import Rust module
import "physics.cpp"   // Import C++ module
```

### Goroutines (Planned)

Goroutine support is planned with integration into Fragile's async runtime. Standard `go` syntax will be supported:

```go
go func() {
    // Concurrent execution
}()
```

**Note:** Go support is currently under development. See the roadmap for status.

---

# Part IV: Cross-Language Programming

## Calling Functions

All syntaxes compile to the same IR, so cross-syntax calls are just regular function calls.

### Basic Example

**math.rs:**
```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }
```

**calc.cpp:**
```cpp
#include "math.rs"
int compute(int x, int y) {
    return add(x, y) + multiply(x, y);
}
```

**main.go:**
```go
import "calc.cpp"
func main() {
    result := compute(10, 20)
}
```

### Ownership Transfer Across Boundaries
```rust
pub fn consume(s: String) { println!("{}", s); }
```
```cpp
void example() {
    std::string s = "hello";
    consume(std::move(s));  // Transfer ownership
}
```

## Sharing Types

Types defined in one syntax can be used in others.

### Structural Equivalence

These define **the same type**:
```rust
pub struct Point { pub x: f64, pub y: f64 }
```
```cpp
struct Point { double x; double y; };
```
```go
type Point struct { x, y float64 }
```

### Using Types Across Files

**geometry.rs:**
```rust
pub struct Rectangle { pub origin: Point, pub width: f64, pub height: f64 }
```

**renderer.cpp:**
```cpp
#include "geometry.rs"
void draw_rectangle(const Rectangle& rect) { /* use rect.origin.x, etc. */ }
```

**main.go:**
```go
import "geometry.rs"
import "renderer.cpp"
func main() {
    rect := Rectangle{origin: Point{10.0, 20.0}, width: 100.0, height: 50.0}
    draw_rectangle(&rect)
}
```

## Generics

Fragile has a unified generics system. All generic code is monomorphized.

### Rust
```rust
fn max<T: Ord>(a: T, b: T) -> T { if a > b { a } else { b } }
```

### C++
```cpp
template<typename T>
requires std::totally_ordered<T>
T max(T a, T b) { return a > b ? a : b; }
```

### Go
```go
func max[T Ordered](a, b T) T {
    if a > b { return a }
    return b
}
```

All three compile to the same monomorphized code.

---

# Part V: Reference

## Built-in Types

### Integer Types

| Type | Size | Rust | C++ | Go |
|------|------|------|-----|-----|
| `i8` | 8-bit | `i8` | `int8_t` | `int8` |
| `i16` | 16-bit | `i16` | `int16_t` | `int16` |
| `i32` | 32-bit | `i32` | `int` | `int32` |
| `i64` | 64-bit | `i64` | `long` | `int64` |
| `u8` | 8-bit | `u8` | `uint8_t` | `uint8` |
| `u32` | 32-bit | `u32` | `uint32_t` | `uint32` |
| `u64` | 64-bit | `u64` | `uint64_t` | `uint64` |
| `f32` | 32-bit | `f32` | `float` | `float32` |
| `f64` | 64-bit | `f64` | `double` | `float64` |

### Standard Library Types

| Type | Rust | C++ | Go |
|------|------|-----|-----|
| `String` | `String` | `std::string` | `string` |
| `Vec<T>` | `Vec<T>` | `std::vector<T>` | `[]T` |
| `Box<T>` | `Box<T>` | `std::unique_ptr<T>` | `*T` |
| `Option<T>` | `Option<T>` | `std::optional<T>` | `*T` (nil) |
| `HashMap<K,V>` | `HashMap<K,V>` | `std::unordered_map` | `map[K]V` |

## Operators

### Arithmetic
`+`, `-`, `*`, `/`, `%` - same across all syntaxes

### Comparison
`==`, `!=`, `<`, `<=`, `>`, `>=` - same across all syntaxes

### Logical
`&&`, `||`, `!` - short-circuit evaluated

### Error Propagation
| Rust | C++ | Go |
|------|-----|-----|
| `result?` | `TRY(result)` | `if err != nil { return }` |

## Attributes

| Attribute | Rust | C++ | Purpose |
|-----------|------|-----|---------|
| derive | `#[derive(...)]` | `[[derive(...)]]` | Auto-implement traits |
| inline | `#[inline]` | `inline` | Inline hint |
| must_use | `#[must_use]` | `[[nodiscard]]` | Warn on unused |
| deprecated | `#[deprecated]` | `[[deprecated]]` | Mark deprecated |
| repr(C) | `#[repr(C)]` | Default | C ABI layout |

## C ABI

Fragile supports C ABI for interfacing with the OS and external C libraries.

### Declaring External Functions

```rust
extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}
```

### Type Mappings

| C Type | Fragile Type |
|--------|--------------|
| `int` | `i32` |
| `long` | `i64` |
| `size_t` | `usize` |
| `void*` | `*mut u8` |
| `const char*` | `*const i8` |

### Exporting Functions
```rust
#[no_mangle]
pub extern "C" fn my_function(x: i32) -> i32 { x * 2 }
```

---

# Part VI: Appendix

## Syntax Comparison

| Feature | Rust | C++ | Go |
|---------|------|-----|-----|
| Entry point | `fn main()` | `int main()` | `func main()` |
| Print | `println!("{}", x)` | `std::cout << x` | `fmt.Println(x)` |
| Null | `None` | `nullptr` | `nil` |
| This/self | `self` | `this` | receiver name |
| Lambda | `\|x\| x + 1` | `[](auto x) { return x+1; }` | `func(x int) int { return x+1 }` |

## Differences from Native Compilers

| Feature | rustc | Fragile Rust | g++/clang++ | Fragile C++ | Go gc | Fragile Go |
|---------|-------|--------------|-------------|-------------|-------|------------|
| Memory | Ownership | Same | RAII | Same | GC | Conservative GC |
| Errors | Result | Same | Exceptions | Same | Error returns | Same |
| Null | Option | Same | nullptr | Same | nil | Same |
| Assignment | Move | Same | Copy | Same | Copy | Same |

### Design Philosophy: Semantic Preservation

Unlike earlier designs that unified semantics, Fragile now **preserves each language's native behavior**:

- **Rust**: Full ownership/borrowing with rustc's borrow checker
- **C++**: Standard C++ semantics (RAII, exceptions, copy/move)
- **Go**: Conservative GC preserving Go's memory model

### What's Different

The key difference is the **compilation target**: all three frontends emit rustc MIR, enabling:
- Unified binary output
- Cross-language function calls without FFI
- Shared optimizations through rustc's backend

### Frontend Implementation

| Language | Frontend | Parser |
|----------|----------|--------|
| Rust | rustc | Native rustc |
| C++ | Clang | libclang |
| Go | go/ssa | Planned |

## Migration Guide

### Migrating Rust Code

Most Rust code works unchanged since Fragile uses rustc as the Rust frontend.

**Fully supported:**
- All stable Rust features
- Ownership and borrowing
- Traits and generics
- Async/await

**Considerations:**
- Procedural macros: Supported (run via rustc)
- Build scripts: Supported (standard cargo integration)
- Inline assembly: Supported (target-dependent)

### Migrating C++ Code

C++ code compiles with **standard C++ semantics**. No code changes required for most projects.

```cpp
// Standard C++ code works as-is
std::string s1 = "hello";
std::string s2 = s1;  // Copy (unchanged)
std::string s3 = std::move(s1);  // Move (unchanged)
```

**Fully supported:**
- C++20/23 features (requires compatible Clang)
- Templates, concepts, coroutines
- Exceptions, RTTI
- STL containers

**Considerations:**
- Compiler-specific extensions: May need adjustments
- Inline assembly: Target-dependent

### Migrating Go Code

Go code compiles with **standard Go semantics** using conservative GC.

```go
// Standard Go code works as-is
s := "hello"
t := s  // Both valid (unchanged)
```

**Supported (planned):**
- Standard Go features
- Goroutines and channels
- Interface-based polymorphism

**Considerations:**
- CGo: Replaced by direct Fragile C++ integration
- Runtime introspection: May have limitations

## Ecosystem Compatibility

| Ecosystem | Support | Method |
|-----------|---------|--------|
| crates.io (Rust) | Full | Compile from source |
| Go modules | Full | Compile from source |
| C++ (source) | Full | Compile from source |
| Header-only C++ | Full | Include and compile |
| C libraries | Via C ABI | Link against compiled library |
| Pre-compiled Rust/Go | No | Source required |

### Fragile.toml Example
```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }

[go-dependencies]
"github.com/gin-gonic/gin" = "v1.9"

[cpp-dependencies]
abseil = { git = "https://github.com/abseil/abseil-cpp" }
```

## Architecture Details

### Why Clang + rustc?

Fragile uses **production-grade frontends** rather than custom parsers:

| Choice | Rationale |
|--------|-----------|
| Clang for C++ | Handles all C++ complexity (templates, SFINAE, concepts) |
| rustc for Rust | Full borrow checking and Rust semantics |
| go/ssa for Go | Planned: Leverages Go's type-checked SSA representation |

### MIR as Unified IR

All frontends emit **rustc MIR** (Mid-level Intermediate Representation):

```
C++ AST (Clang)  ─┐
                  ├──▶ rustc MIR ──▶ LLVM IR ──▶ Binary
Rust AST (rustc) ─┘
```

**Benefits:**
- Single optimization pipeline (rustc → LLVM)
- No ABI boundaries between languages
- Unified debug info and profiling

### Query System Override

Fragile injects C++ MIR into rustc using query overrides:

```rust
// Simplified: how C++ MIR is injected
fn mir_built(tcx, def_id) -> &Mir {
    if is_cpp_function(def_id) {
        cpp_mir_registry.get_mir(def_id)  // Return C++ MIR
    } else {
        original_mir_built(tcx, def_id)   // Normal Rust path
    }
}
```

### Runtime Support

Some language features require runtime support:

| Feature | Implementation |
|---------|---------------|
| C++ exceptions | `fragile_rt_throw()`, `fragile_rt_catch()` |
| C++ RTTI | Type info structures compatible with Itanium ABI |
| C++ vtables | Virtual dispatch via standard vtable layout |
| Go GC | Conservative mark-and-sweep collector |

### Build Flow

```
fragile build main.rs utils.cpp helpers.go
    │
    ├── Rust files ──▶ rustc frontend ──▶ Rust MIR
    │
    ├── C++ files ──▶ Clang parse ──▶ Convert to MIR ──▶ Inject into rustc
    │
    └── Go files ──▶ go/ssa (planned) ──▶ Convert to MIR ──▶ Inject into rustc
    │
    ▼
rustc codegen (all MIR) ──▶ LLVM IR ──▶ Native binary
```

---

*Fragile: Three languages, one compiler, zero FFI overhead.*
