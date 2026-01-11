# Fragile Language Specification

**Fragile** is a unified programming language with three syntaxes: Rust, C++20, and Go.

---

# Part I: Introduction and Getting Started

## Introduction

Fragile is not three languages glued together. It is **one language** that accepts three different syntaxes. All code, regardless of which syntax it was written in, compiles to the same intermediate representation (IR) with identical semantics.

```
┌─────────────────────────────────────────┐
│  User Code (.rs, .cpp, .go)             │  ← Any syntax
├─────────────────────────────────────────┤
│  Fragile HIR (Unified IR)               │  ← One representation
├─────────────────────────────────────────┤
│  LLVM IR                                │  ← One backend
├─────────────────────────────────────────┤
│  Native Code                            │  ← One binary
└─────────────────────────────────────────┘
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

1. **Unified Memory Model**: All three syntaxes use ownership-based memory management
2. **Unified Type System**: Types are shared across syntaxes
3. **Zero-Cost Interop**: Cross-syntax function calls have no overhead
4. **Full Ecosystem Access**: Use crates.io, Go modules, and C++ libraries
5. **Single ABI**: Only C ABI is needed (for libc)

### Language Detection

| Extension | Syntax |
|-----------|--------|
| `.rs` | Rust |
| `.cpp`, `.cc`, `.cxx`, `.hpp` | C++ |
| `.go` | Go |

## Getting Started

### Installation

Prerequisites: LLVM 19, Rust 1.75+

```bash
git clone https://github.com/fragile-lang/fragile
cd fragile
cargo build --release
```

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

### Semantic Unification

| Concept | Fragile Choice | Rationale |
|---------|---------------|-----------|
| Memory | Ownership (Rust) | Compile-time safety, no GC |
| Errors | `Result<T, E>` | Explicit, composable |
| Null | `Option<T>` | No null pointer exceptions |
| Generics | Monomorphization | Zero-cost abstractions |

### Source-First Ecosystem

Fragile compiles everything from source:
- **Full ecosystem access**: Use any Rust crate, Go module, or C++ library
- **No ABI concerns**: No stable Rust ABI needed
- **Maximum optimization**: Compiler sees all code

## Memory Model

Fragile uses **ownership-based memory management** across all three syntaxes.

### Ownership

Every value has exactly one owner. When the owner goes out of scope, the value is dropped.

**Rust** (native):
```rust
let s = String::from("hello");
let t = s;  // ownership moves to t
// s is no longer valid
```

**C++** (move semantics):
```cpp
std::string s = "hello";
std::string t = std::move(s);  // In Fragile: implicit move
// s is no longer valid
```

**Go** (implicit moves):
```go
s := "hello"
t := s  // ownership moves (Fragile behavior)
// s is no longer valid
```

### Key Difference: Assignment Always Moves

| Language | Default Assignment | Fragile Behavior |
|----------|-------------------|------------------|
| Rust | Move | Move (same) |
| C++ | Copy | Move (different) |
| Go | Copy/Share | Move (different) |

To copy, use `.clone()`:
```rust
let t = s.clone();  // Explicit copy
```

### Borrowing

**Immutable borrows** (`&T`): Multiple readers, no writers
```rust
fn print_length(s: &String) { println!("{}", s.len()); }
```

**Mutable borrows** (`&mut T`): One writer, no readers
```rust
fn append_world(s: &mut String) { s.push_str(" world"); }
```

### Borrowing Rules

1. One mutable borrow OR multiple immutable borrows (not both)
2. References must not outlive the data
3. Data cannot be modified while immutably borrowed

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

Fragile uses `Result<T, E>` for all error handling.

### Rust (Native)
```rust
fn parse(s: &str) -> Result<i32, ParseError> {
    s.parse().map_err(|_| ParseError::InvalidFormat)
}
let n = parse("42")?;  // Propagate with ?
```

### C++ (Exceptions → Result)
```cpp
int parse(const std::string& s) {
    if (s.empty()) throw ParseError{"empty"};  // → Err(...)
    return std::stoi(s);  // → Ok(...)
}
// try-catch becomes match on Result
```

### Go (Error Returns → Result)
```go
func parse(s string) (int, error) {
    if s == "" {
        return 0, errors.New("empty")  // → Err(...)
    }
    return strconv.Atoi(s)  // → Ok(...)
}
// if err != nil → match on Result
```

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

C++20 syntax with semantic modifications.

### Key Differences from Standard C++

| Standard C++ | Fragile C++ |
|--------------|-------------|
| Copy by default | Move by default |
| Exceptions | Result types |
| Multiple inheritance | Single + traits |

### Ownership
```cpp
std::string s1 = "hello";
std::string s2 = s1;  // MOVES in Fragile
std::string s3 = s1.clone();  // Explicit copy
```

### Templates → Generics
```cpp
template<typename T>
requires std::totally_ordered<T>
T max(T a, T b) { return a > b ? a : b; }
```

### Imports
```cpp
#include "math.rs"
#include "utils.go"
```

## Go Syntax

Go syntax with ownership-based memory model.

### Key Differences from Standard Go

| Standard Go | Fragile Go |
|-------------|------------|
| Garbage collected | Ownership-based |
| Copy/share | Move by default |
| `nil` | `Option<T>` |
| Goroutines | Async/await (planned) |

### Ownership
```go
s1 := "hello"
s2 := s1  // MOVES in Fragile
s3 := s1.Clone()  // Explicit copy
```

### Error Handling
```go
func readFile(path string) (string, error) {
    // (T, error) becomes Result<T, Error>
}
```

### Imports
```go
import "math.rs"
import "physics.cpp"
```

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

| Feature | Rust (rustc) | Fragile Rust | C++ (g++) | Fragile C++ | Go (gc) | Fragile Go |
|---------|--------------|--------------|-----------|-------------|---------|------------|
| Memory | Ownership | Same | Manual/RAII | Ownership | GC | Ownership |
| Errors | Result | Same | Exceptions | Result | Error returns | Result |
| Null | Option | Same | nullptr | Option | nil | Option |
| Assignment | Move | Same | Copy | Move | Copy | Move |

### What's Changed

- **C++ exceptions** → Result types
- **C++ multiple inheritance** → Single inheritance + traits
- **Go goroutines** → Async/await (planned)
- **Go nil** → Option<T>
- **Go GC** → Ownership

## Migration Guide

### Migrating Rust Code
Most Rust code works unchanged. Check:
- Procedural macros (run via rustc)
- Async runtime (different from tokio)

### Migrating C++ Code
```cpp
// Before: copy by default
std::string s1 = "hello";
std::string s2 = s1;  // Copies

// After: move by default
std::string s1 = "hello";
std::string s2 = s1;  // MOVES
std::string s3 = s2.clone();  // Explicit copy
```

### Migrating Go Code
```go
// Before: GC handles memory
s := "hello"
t := s  // Both valid

// After: ownership
s := "hello"
t := s  // s invalid
u := t.Clone()  // Explicit copy
```

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

---

*Fragile: One language, three syntaxes, zero overhead.*
