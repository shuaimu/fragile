# Fragile Language Specification

**Fragile** is a unified compiler for Rust, C++20/23, and Go that can serve as a **drop-in replacement** for g++, clang++, and `go build`, while enabling seamless cross-language interoperability.

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
  - [The Library Approach](#the-library-approach)
  - [Using C++ from Rust](#using-c-from-rust)
  - [Using Rust from C++](#using-rust-from-c)
  - [Type Mapping Reference](#type-mapping-reference)
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

Fragile is a **complete compiler** for Rust, C++, and Go that produces native binaries. It can be used in two ways:

1. **As a compiler replacement**: Use `fragile++` instead of `g++` or `clang++`
2. **As a polyglot compiler**: Mix Rust, C++, and Go in the same project with zero-cost interop

```bash
# Use as a C++ compiler (drop-in replacement for g++)
fragile++ main.cpp -o main
./main

# Use as a Go compiler (drop-in replacement for go build)
fragile-go build main.go
./main

# Use as a polyglot compiler (mix languages)
fragile build main.rs utils.cpp helpers.go -o program
./program
```

### Architecture

Fragile transpiles C++ and Go to Rust, then compiles everything with rustc:

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Code                               │
│     .rs files         .cpp/.cc files          .go files         │
└────────┬──────────────────┬───────────────────────┬─────────────┘
         │                  │                       │
         │                  ▼                       ▼
         │           ┌────────────┐          ┌───────────┐
         │           │   Clang    │          │  go/ssa   │
         │           │  (parsing) │          │ (planned) │
         │           └─────┬──────┘          └─────┬─────┘
         │                 │                       │
         │                 ▼                       ▼
         │           ┌────────────┐          ┌───────────┐
         │           │ Transpiler │          │ Transpiler│
         │           │ (C++→Rust) │          │ (Go→Rust) │
         │           └─────┬──────┘          └─────┬─────┘
         │                 │                       │
         │                 ▼                       ▼
         │           ┌────────────┐          ┌───────────┐
         │           │ Generated  │          │ Generated │
         │           │ Rust Code  │          │ Rust Code │
         │           └─────┬──────┘          └─────┬─────┘
         │                 │                       │
         └────────────────►├◄──────────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │    rustc    │
                    └──────┬──────┘
                           ▼
                    ┌─────────────┐
                    │   Binary    │
                    └─────────────┘
```

### Why Fragile?

**Goal 1: Complete Compiler Replacement**

Fragile aims to compile any valid C++ or Go program:

```bash
# Your existing C++ project
fragile++ -std=c++20 -O2 main.cpp utils.cpp -o myapp

# Your existing Go project
fragile-go build ./...
```

This works because:
- **Clang handles all C++ complexity**: Templates, SFINAE, concepts, coroutines
- **go/ssa handles Go**: Type checking, interface resolution
- **We transpile the fully-resolved AST**: No need to reimplement language semantics

**Goal 2: Zero-Cost Cross-Language Interop**

Once code is transpiled to Rust, cross-language calls are just regular function calls:

```rust
// Use C++ std::map in Rust
use fragile::cpp::std::map::Map;

fn main() {
    let mut m: Map<String, i32> = Map::new();
    m.insert("key".into(), 42);
}
```

```cpp
// Use Rust Vec in C++
#include <fragile/rust/std/vec.hpp>

int main() {
    fragile::rust::std::Vec<int> v;
    v.push(42);
}
```

**No FFI. No marshalling. No overhead.**

### Key Features

1. **Complete Compiler**: Can replace g++/clang++/go build for standalone projects
2. **Production-Grade Frontends**: Uses Clang for C++ and go/ssa for Go (not custom parsers)
3. **Transpiles to Rust**: Generated code is debuggable, stable across versions
4. **Zero-Cost Interop**: Cross-language calls compile to direct function calls
5. **Full Ecosystem Access**: Use crates.io, C++ STL, and Go stdlib together
6. **C++20/23 Support**: Full modern C++ including templates, concepts, and coroutines
7. **Memory Model Per Language**: Rust uses ownership, C++ uses RAII, Go uses conservative GC

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

**Key Insight:** Rather than imposing one language's model on others, Fragile compiles each language with its native semantics. Interoperability is achieved by generating compatible Rust APIs and shared runtime helpers.

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

## The Library Approach

Fragile uses a **library approach** for cross-language interop. Instead of special import syntax, foreign libraries appear as native modules in your language.

### Design Principles

1. **No special syntax**: Use standard `use` (Rust), `#include` (C++), or `import` (Go)
2. **Namespace preservation**: C++ namespaces become Rust modules and vice versa
3. **Type safety**: All types are checked at compile time
4. **Zero overhead**: Calls are direct function calls, no FFI marshalling

### Module Structure

```
fragile::cpp::*      # C++ libraries available in Rust
fragile::rust::*     # Rust libraries available in C++/Go
fragile::go::*       # Go libraries available in Rust/C++
```

---

## Using C++ from Rust

C++ libraries are exposed under `fragile::cpp::*`.

### Basic Usage

```rust
// Use C++ std::vector in Rust
use fragile::cpp::std::vector::Vector;

fn main() {
    let mut v: Vector<i32> = Vector::new();
    v.push_back(10);
    v.push_back(20);
    println!("size: {}", v.size());  // Calls C++ vector::size()
}
```

### STL Containers

```rust
use fragile::cpp::std::map::Map;
use fragile::cpp::std::string::String as CppString;
use fragile::cpp::std::unordered_map::UnorderedMap;

fn example() {
    // std::map<std::string, int>
    let mut m: Map<CppString, i32> = Map::new();
    m.insert(CppString::from("key"), 42);

    if let Some(val) = m.get(&CppString::from("key")) {
        println!("value: {}", val);
    }
}
```

### C++ Classes

```rust
use fragile::cpp::my_lib::Point;
use fragile::cpp::my_lib::Rectangle;

fn example() {
    // Construct C++ object
    let p = Point::new(10.0, 20.0);

    // Method calls
    let x = p.get_x();          // Calls Point::getX()
    let dist = p.distance(&Point::new(0.0, 0.0));

    // Nested types
    let rect = Rectangle::new(p, 100.0, 50.0);
}
```

### Templates/Generics

C++ templates are instantiated on first use:

```rust
use fragile::cpp::std::vector::Vector;
use fragile::cpp::std::pair::Pair;

fn example() {
    // Instantiates std::vector<std::pair<i32, f64>>
    let mut v: Vector<Pair<i32, f64>> = Vector::new();
    v.push_back(Pair::new(1, 3.14));
}
```

### Namespace Mapping

| C++ | Rust |
|-----|------|
| `std::vector<T>` | `fragile::cpp::std::vector::Vector<T>` |
| `std::map<K,V>` | `fragile::cpp::std::map::Map<K,V>` |
| `boost::asio::io_context` | `fragile::cpp::boost::asio::io_context::IoContext` |
| `my::nested::Type` | `fragile::cpp::my::nested::Type` |

---

## Using Rust from C++

Rust libraries are exposed under `fragile/rust/*` headers.

### Basic Usage

```cpp
#include <fragile/rust/std/vec.hpp>
#include <fragile/rust/std/string.hpp>

void example() {
    // Use Rust Vec in C++
    fragile::rust::std::Vec<int> v;
    v.push(10);
    v.push(20);
    std::cout << "len: " << v.len() << std::endl;
}
```

### Rust Types in C++

```cpp
#include <fragile/rust/std/option.hpp>
#include <fragile/rust/std/result.hpp>
#include <fragile/rust/std/hashmap.hpp>

void example() {
    using namespace fragile::rust::std;

    // Option<T>
    Option<int> opt = Option<int>::Some(42);
    if (opt.is_some()) {
        int val = opt.unwrap();
    }

    // Result<T, E>
    Result<int, String> res = Result<int, String>::Ok(42);
    if (res.is_ok()) {
        int val = res.unwrap();
    }

    // HashMap<K, V>
    HashMap<String, int> map;
    map.insert(String::from("key"), 42);
}
```

### Namespace Mapping

| Rust | C++ |
|------|-----|
| `Vec<T>` | `fragile::rust::std::Vec<T>` |
| `HashMap<K,V>` | `fragile::rust::std::HashMap<K,V>` |
| `Option<T>` | `fragile::rust::std::Option<T>` |
| `my_crate::MyType` | `fragile::rust::my_crate::MyType` |

---

## Using Go from Rust/C++

Go packages are exposed under `fragile::go::*` (Rust) and `fragile/go/*` (C++).

```rust
// Rust
use fragile::go::fmt;
use fragile::go::strings;

fn example() {
    let s = strings::to_upper("hello");
    fmt::println(&s);
}
```

```cpp
// C++
#include <fragile/go/fmt.hpp>
#include <fragile/go/strings.hpp>

void example() {
    auto s = fragile::go::strings::ToUpper("hello");
    fragile::go::fmt::Println(s);
}
```

---

## User-Defined Libraries

### Exposing Your C++ Library to Rust

**my_math.cpp:**
```cpp
namespace my_math {

struct Point {
    double x, y;

    Point(double x, double y) : x(x), y(y) {}

    double distance(const Point& other) const {
        double dx = x - other.x;
        double dy = y - other.y;
        return std::sqrt(dx*dx + dy*dy);
    }
};

Point midpoint(const Point& a, const Point& b) {
    return Point((a.x + b.x) / 2, (a.y + b.y) / 2);
}

} // namespace my_math
```

**Using from Rust:**
```rust
use fragile::cpp::my_math::Point;
use fragile::cpp::my_math::midpoint;

fn main() {
    let a = Point::new(0.0, 0.0);
    let b = Point::new(10.0, 10.0);
    let mid = midpoint(&a, &b);
    println!("midpoint: ({}, {})", mid.x, mid.y);
}
```

### Exposing Your Rust Library to C++

**my_utils.rs:**
```rust
pub mod my_utils {
    pub struct Counter {
        value: i64,
    }

    impl Counter {
        pub fn new() -> Self {
            Counter { value: 0 }
        }

        pub fn increment(&mut self) {
            self.value += 1;
        }

        pub fn get(&self) -> i64 {
            self.value
        }
    }
}
```

**Using from C++:**
```cpp
#include <fragile/rust/my_utils.hpp>

void example() {
    fragile::rust::my_utils::Counter c;
    c.increment();
    c.increment();
    std::cout << c.get() << std::endl;  // Prints: 2
}
```

---

## Type Mapping Reference

### Primitives

| Rust | C++ | Go |
|------|-----|-----|
| `i32` | `int32_t` | `int32` |
| `i64` | `int64_t` | `int64` |
| `f64` | `double` | `float64` |
| `bool` | `bool` | `bool` |
| `char` | `char32_t` | `rune` |

### Smart Pointers and Ownership

| Rust | C++ Equivalent |
|------|----------------|
| `Box<T>` | `std::unique_ptr<T>` |
| `Rc<T>` | `std::shared_ptr<T>` |
| `Arc<T>` | `std::shared_ptr<T>` (thread-safe) |
| `&T` | `const T&` |
| `&mut T` | `T&` |

| C++ | Rust Equivalent |
|-----|-----------------|
| `std::unique_ptr<T>` | `Box<T>` |
| `std::shared_ptr<T>` | `Arc<T>` |
| `const T&` | `&T` |
| `T&` | `&mut T` |
| `T*` | `*const T` or `*mut T` |

### Strings

| Rust | C++ | Conversion |
|------|-----|------------|
| `String` | `std::string` | Zero-copy when possible |
| `&str` | `std::string_view` | Zero-copy view |

### Containers

| Rust | C++ |
|------|-----|
| `Vec<T>` | `std::vector<T>` |
| `HashMap<K,V>` | `std::unordered_map<K,V>` |
| `BTreeMap<K,V>` | `std::map<K,V>` |
| `HashSet<T>` | `std::unordered_set<T>` |
| `[T; N]` | `std::array<T, N>` |

---

## Memory Management Across Boundaries

### Ownership Rules

1. **Rust owns by default**: When Rust creates an object, Rust manages its lifetime
2. **C++ owns by default**: When C++ creates an object, C++ manages its lifetime
3. **Explicit transfer**: Use `.into_cpp()` or `.into_rust()` to transfer ownership

### Examples

```rust
use fragile::cpp::std::vector::Vector;

fn example() {
    // Rust creates, Rust owns
    let mut v: Vector<i32> = Vector::new();
    v.push_back(42);
    // v is dropped when it goes out of scope (calls C++ destructor)

    // Transfer to C++ ownership
    let cpp_owned = v.into_cpp();  // Rust no longer manages this
}
```

```cpp
#include <fragile/rust/std/vec.hpp>

void example() {
    // C++ creates, C++ owns
    fragile::rust::std::Vec<int> v;
    v.push(42);
    // v destructor called at end of scope

    // Transfer to Rust ownership
    auto rust_owned = v.into_rust();  // C++ no longer manages this
}
```

### Reference Semantics

```rust
use fragile::cpp::my_lib::Point;

fn takes_ref(p: &Point) {
    // Borrows C++ object, does not take ownership
    println!("x = {}", p.get_x());
}

fn takes_mut_ref(p: &mut Point) {
    // Mutable borrow
    p.set_x(100.0);
}
```

---

## Calling Functions (Direct File Import)

For simple cases, direct file imports also work:

### Basic Example

**math.rs:**
```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }
```

**calc.cpp:**
```cpp
#include "math.rs"
int compute(int x, int y) {
    return add(x, y) * 2;
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

Fragile **preserves each language's native behavior**:

- **Rust**: Full ownership/borrowing with rustc's borrow checker
- **C++**: Standard C++ semantics (RAII, exceptions, copy/move)
- **Go**: Conservative GC preserving Go's memory model

### What's Different

The key difference is the **compilation method**: C++ and Go are transpiled to Rust, then compiled with rustc:

- All languages end up as Rust code
- Cross-language calls are just Rust function calls
- LLVM optimizes across language boundaries

### Frontend Implementation

| Language | Frontend | Parser | Transpilation |
|----------|----------|--------|---------------|
| Rust | rustc | Native rustc | None (native) |
| C++ | Clang | libclang | C++ AST → Rust source |
| Go | go/ssa | Planned | Go SSA → Rust source |

## C++ Feature Support

### Fully Supported

| Feature | Status | Notes |
|---------|--------|-------|
| Classes and structs | ✅ | `#[repr(C)]` structs + impl blocks |
| Single inheritance | ✅ | Base embedded as first field |
| Virtual functions | ✅ | Manual vtable structs |
| Templates | ✅ | Clang instantiates, we transpile result |
| Concepts (C++20) | ✅ | Clang evaluates constraints |
| RAII / Destructors | ✅ | `Drop` trait implementation |
| Move semantics | ✅ | Rust move semantics |
| Operator overloading | ✅ | Trait impls (Add, Sub, Index, etc.) |
| Lambdas | ✅ | Closures |
| `const` methods | ✅ | `&self` methods |
| Namespaces | ✅ | Rust modules |
| `static` members | ✅ | Module-level statics |
| `constexpr` | ✅ | Rust `const` |
| References | ✅ | Rust references |
| Pointers | ✅ | Raw pointers |
| Arrays | ✅ | Rust arrays |

### Supported with Caveats

| Feature | Status | Notes |
|---------|--------|-------|
| Multiple inheritance | ⚠️ | Works but complex layout |
| Exceptions | ⚠️ | Use `-fno-exceptions` for best support |
| RTTI | ⚠️ | Basic support, `dynamic_cast` limited |
| Coroutines (C++20) | ⚠️ | Mapped to async Rust |
| Volatile | ⚠️ | `read_volatile`/`write_volatile` |
| Bitfields | ⚠️ | Manual bit manipulation |
| `union` | ⚠️ | Rust unions (no non-trivial members) |

### Not Yet Supported

| Feature | Status | Planned |
|---------|--------|---------|
| `asm` blocks | ❌ | Yes |
| `#pragma` directives | ❌ | Partial |
| Windows SEH | ❌ | No |
| Computed goto | ❌ | No |

### Compiler-Specific Extensions

| Extension | g++ | clang++ | Fragile |
|-----------|-----|---------|---------|
| `__attribute__` | ✅ | ✅ | Partial |
| `__builtin_*` | ✅ | ✅ | Common ones |
| Statement expressions | ✅ | ✅ | ✅ |
| Nested functions | ✅ | ❌ | ❌ |

## Migration Guide

### Migrating Rust Code

Rust code works unchanged—Fragile uses rustc directly.

**Fully supported:**
- All stable Rust features
- Ownership and borrowing
- Traits and generics
- Async/await
- Procedural macros
- Build scripts

### Migrating C++ Code

Most C++ code compiles unchanged:

```bash
# Before
g++ -std=c++20 -O2 main.cpp utils.cpp -o myapp

# After
fragile++ -std=c++20 -O2 main.cpp utils.cpp -o myapp
```

**Best compatibility:**
```bash
# Use -fno-exceptions for cleanest transpilation
fragile++ -fno-exceptions main.cpp -o main
```

**Inspecting generated code:**
```bash
# See what Rust code was generated
fragile++ --emit=rust main.cpp -o main.rs
cat main.rs
```

### Common Migration Issues

**Issue: Exceptions not caught**
```cpp
// May need adjustment if using exceptions heavily
try { throw std::runtime_error("error"); }
catch (...) { /* handler */ }
```
Solution: Use `-fno-exceptions` and error codes, or wait for full exception support.

**Issue: RTTI not working**
```cpp
dynamic_cast<Derived*>(base_ptr)  // Limited support
```
Solution: Use virtual function dispatch instead.

**Issue: Compiler-specific extension**
```cpp
__attribute__((packed))  // May not work
```
Solution: Use standard C++ or `#[repr(packed)]` equivalent.

### Migrating Go Code

Go code compiles with **standard Go semantics** using conservative GC.

```bash
# Before
go build main.go

# After
fragile-go build main.go
```

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

### Transpilation to Rust

Fragile transpiles C++ and Go to **unsafe Rust source code**, which is then compiled by rustc:

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
│  C++ Source │────▶│ Clang (parsing) │────▶│  Clang AST  │
└─────────────┘     └─────────────────┘     └──────┬──────┘
                                                   │
                                                   ▼
                                           ┌──────────────┐
                                           │ Transpiler   │
                                           │ (fragile)    │
                                           └──────┬───────┘
                                                  │
                                                  ▼
                                           ┌──────────────┐
                                           │ Unsafe Rust  │
                                           │ Source Code  │
                                           └──────┬───────┘
                                                  │
                                                  ▼
┌─────────────┐                            ┌──────────────┐
│ Rust Source │───────────────────────────▶│    rustc     │
└─────────────┘                            └──────┬───────┘
                                                  │
                                                  ▼
                                           ┌──────────────┐
                                           │    Binary    │
                                           └──────────────┘
```

**Why transpile to Rust source?**

Rust source is a stable, debuggable interface that keeps the toolchain simple and transparent.

### Generated Code Structure

C++ code transpiles to `#[repr(C)]` structs with `impl` blocks:

```cpp
// Input: my_lib.cpp
namespace my_lib {
    class Point {
        double x, y;
    public:
        Point(double x, double y) : x(x), y(y) {}
        double getX() const { return x; }
    };
}
```

```rust
// Output: fragile/cpp/my_lib.rs (generated)
#[repr(C)]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn get_x(&self) -> f64 {
        self.x
    }
}
```

### Runtime Support

Some language features require runtime support:

| Feature | Implementation |
|---------|---------------|
| C++ exceptions | `fragile_rt_throw()`, `fragile_rt_catch()` |
| C++ RTTI | Type info structures compatible with Itanium ABI |
| C++ vtables | Generated `#[repr(C)]` vtable structs |
| Go GC | Conservative mark-and-sweep collector |

### Virtual Dispatch

Virtual functions generate explicit vtable structs:

```cpp
// Input
class Animal {
public:
    virtual int speak() = 0;
};
```

```rust
// Output (simplified)
#[repr(C)]
pub struct AnimalVtable {
    pub speak: unsafe fn(*const Animal) -> i32,
}

#[repr(C)]
pub struct Animal {
    __vtable: *const AnimalVtable,
}
```

### Build Flow

```
fragile build main.rs utils.cpp helpers.go
    │
    ├── Rust files ──▶ (used directly)
    │
    ├── C++ files ──▶ Clang parse ──▶ Transpile to Rust ──▶ .rs files
    │
    └── Go files ──▶ go/ssa (planned) ──▶ Transpile to Rust ──▶ .rs files
    │
    ▼
All .rs files ──▶ rustc ──▶ LLVM IR ──▶ Native binary
```

### Generated File Layout

```
target/fragile/
├── cpp/
│   ├── std/
│   │   ├── vector.rs      # std::vector<T>
│   │   ├── map.rs         # std::map<K,V>
│   │   └── string.rs      # std::string
│   └── my_lib/
│       └── mod.rs         # User's C++ library
├── go/
│   ├── fmt.rs             # Go fmt package
│   └── strings.rs         # Go strings package
└── lib.rs                 # Re-exports all modules
```

### Advantages of This Approach

1. **Stable API**: Rust source syntax is stable; compiler internals are not
2. **Debuggable**: Can inspect generated `.rs` files
3. **Tooling works**: IDE support, rustfmt, clippy all work on generated code
4. **Incremental**: Only regenerate changed files
5. **LLVM optimization**: rustc → LLVM still optimizes across the generated code

---

*Fragile: Three languages, one compiler, zero FFI overhead.*
