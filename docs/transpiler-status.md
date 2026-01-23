# C++ to Rust Transpiler Status

This document tracks the implementation status of the C++ to Rust transpiler.

## Overview

The transpiler converts C++ source code to Rust source code via:
```
C++ Source → Clang (libclang) → Clang AST → Rust Source → rustc → Binary
```

## Feature Status Legend

- ✅ Implemented and tested
- ⚠️ Partially implemented
- ❌ Not yet implemented
- 🚫 Not planned / Out of scope

---

## Basic Types

| Feature | Status | Notes |
|---------|--------|-------|
| `void` | ✅ | Maps to `()` |
| `bool` | ✅ | Maps to `bool` |
| `char` | ✅ | Maps to `i8` (signed) or `u8` (unsigned) |
| `short` | ✅ | Maps to `i16`/`u16` |
| `int` | ✅ | Maps to `i32`/`u32` |
| `long` | ✅ | Maps to `i64`/`u64` |
| `long long` | ✅ | Maps to `i64`/`u64` |
| `float` | ✅ | Maps to `f32` |
| `double` | ✅ | Maps to `f64` |
| `size_t` | ✅ | Maps to `usize` |
| `nullptr_t` | ✅ | Maps to `std::ptr::null_mut()` |

## Compound Types

| Feature | Status | Notes |
|---------|--------|-------|
| Pointers (`T*`) | ✅ | Maps to `*mut T` / `*const T` |
| References (`T&`) | ✅ | Maps to `&T` / `&mut T` |
| Rvalue references (`T&&`) | ⚠️ | Parsed, basic return-by-value works |
| Arrays (`T[N]`) | ✅ | Maps to `[T; N]` |
| Function pointers | ⚠️ | Parsed, codegen incomplete |

## Structs and Classes

| Feature | Status | Notes |
|---------|--------|-------|
| Struct definition | ✅ | `#[repr(C)]` struct |
| Class definition | ✅ | Same as struct |
| Public fields | ✅ | `pub field: Type` |
| Private fields | ⚠️ | Currently all fields are `pub` |
| Field access (`.`) | ✅ | `obj.field` |
| Arrow access (`->`) | ✅ | `(*ptr).field` |
| Nested structs | ✅ | |
| Anonymous structs | ❌ | |
| Bit fields | ❌ | |

## Constructors and Destructors

| Feature | Status | Notes |
|---------|--------|-------|
| Default constructor | ✅ | Generates `new_0() -> Self { ... }` |
| Parameterized constructor | ✅ | Generates `new_N(...)` with param mapping |
| Copy constructor | ✅ | Maps to `Clone` trait |
| Move constructor | ✅ | Rust's natural move semantics |
| Destructor | ✅ | Maps to `Drop` trait |
| Member initializer lists | ✅ | Positional mapping from params to fields |
| Constructor calls | ✅ | `new T()` → `Box::into_raw(Box::new(T::new()))` |

## Methods

| Feature | Status | Notes |
|---------|--------|-------|
| Instance methods | ✅ | Body generates `self.field` access |
| Static methods | ✅ | `static mut` globals with unsafe access |
| Const methods | ✅ | Maps to `&self`, auto-detected |
| Non-const methods | ✅ | Maps to `&mut self` |
| Method calls | ✅ | Full AST codegen |
| Virtual methods | ✅ | Static dispatch via override resolution |
| Pure virtual methods | ⚠️ | Basic support |
| Override/final | ⚠️ | Parsed, not enforced |
| Operator overloading | ✅ | Full support (see below) |

## Operator Overloading

| Feature | Status | Notes |
|---------|--------|-------|
| Binary operators (+, -, *, /, %) | ✅ | `op_add`, `op_sub`, etc. |
| Comparison operators (==, !=, <, >, <=, >=) | ✅ | `op_eq`, `op_ne`, etc. |
| Assignment operators (=, +=, -=, etc.) | ✅ | `op_assign`, `op_add_assign`, etc. |
| Subscript operator [] | ✅ | Returns `&mut`, correct arg passing |
| Function call operator () | ✅ | `op_call` method |
| Dereference operator * | ✅ | `op_deref` → returns `&mut` |
| Arrow operator -> | ✅ | `op_arrow` → pointer dereference |
| Increment/decrement (++, --) | ✅ | Pre/post semantics |

## Inheritance

| Feature | Status | Notes |
|---------|--------|-------|
| Single inheritance | ✅ | Base embedded as `__base` field |
| Multiple inheritance | ✅ | Multiple `__base_N` fields |
| Virtual inheritance | ✅ | Diamond inheritance via shared pointers |
| `dynamic_cast` | ⚠️ | Via trait objects |
| RTTI (`typeid`) | ❌ | |

## Functions

| Feature | Status | Notes |
|---------|--------|-------|
| Function definitions | ✅ | |
| Function declarations | ✅ | Extern declarations |
| Parameters (by value) | ✅ | |
| Parameters (by reference) | ✅ | |
| Return values | ✅ | |
| Recursion | ✅ | Tested with factorial |
| Variadic functions | ❌ | |
| Default parameters | ✅ | Evaluated at call site via clang |
| Function overloading | ✅ | Clang resolves, name mangled |

## Expressions

| Feature | Status | Notes |
|---------|--------|-------|
| Integer literals | ✅ | With type suffix |
| Float literals | ✅ | With type suffix |
| Bool literals | ✅ | |
| String literals | ✅ | `b"...\0".as_ptr() as *const i8` |
| Char literals | ✅ | |
| Binary operators (+, -, *, /, %) | ✅ | |
| Comparison operators | ✅ | |
| Logical operators (&&, \|\|, !) | ✅ | |
| Bitwise operators | ✅ | |
| Assignment (=) | ✅ | |
| Compound assignment (+=, etc.) | ✅ | Full support |
| Increment/decrement (++, --) | ✅ | Pre/post semantics correct |
| Ternary operator (?:) | ✅ | `if cond { a } else { b }` |
| Comma operator | ✅ | `{ a; b }` block expression |
| `sizeof` | ✅ | Evaluated by Clang at compile time |
| `alignof` | ✅ | Evaluated by Clang at compile time |
| Type casts | ✅ | `static_cast`, `reinterpret_cast`, `const_cast` |
| Implicit casts | ✅ | Detected and generated as `as` casts |
| Pointer arithmetic | ✅ | `.add()`, `.sub()` methods |

## Statements

| Feature | Status | Notes |
|---------|--------|-------|
| Variable declaration | ✅ | `let mut` |
| If/else | ✅ | |
| While loop | ✅ | |
| For loop | ✅ | |
| Do-while loop | ✅ | |
| Range-based for | ✅ | `for x in container.iter()` |
| Switch/case | ✅ | Match expression |
| Break | ✅ | |
| Continue | ✅ | |
| Return | ✅ | |
| Goto | ❌ | Not supported in safe Rust |

## Templates

| Feature | Status | Notes |
|---------|--------|-------|
| Function templates | ✅ | Clang instantiates, we transpile result |
| Class templates | ✅ | Clang instantiates, we transpile result |
| Template specialization | ✅ | Via Clang |
| Partial specialization | ✅ | Via Clang |
| Variadic templates | ✅ | Via Clang |
| SFINAE | ✅ | Handled by Clang |
| Concepts (C++20) | ✅ | Handled by Clang |

## Namespaces

| Feature | Status | Notes |
|---------|--------|-------|
| Namespace declaration | ✅ | Maps to Rust modules |
| Nested namespaces | ✅ | Nested modules |
| Using directive | ⚠️ | Parsed |
| Using declaration | ✅ | `pub type` aliases |
| Anonymous namespace | ❌ | |

## Memory Management

| Feature | Status | Notes |
|---------|--------|-------|
| Stack allocation | ✅ | Local variables |
| `new` / `delete` | ✅ | `Box::into_raw(Box::new())` / `Box::from_raw()` |
| `new[]` / `delete[]` | ✅ | Vec allocation with raw pointer |
| Placement new | ❌ | |
| Smart pointers | ✅ | Type mappings (unique_ptr→Box, shared_ptr→Arc, weak_ptr→Weak) |

## Error Handling

| Feature | Status | Notes |
|---------|--------|-------|
| Exceptions (`throw`) | ✅ | Maps to `panic!("message")` |
| `try`/`catch` | ✅ | Maps to `std::panic::catch_unwind` |
| `noexcept` | ⚠️ | Parsed, not enforced |
| Stack unwinding | ✅ | Via panic unwinding |

## Lambdas

| Feature | Status | Notes |
|---------|--------|-------|
| Basic lambdas | ✅ | Rust closures with type inference |
| Capture by value ([=]) | ✅ | `move` closures |
| Capture by reference ([&]) | ✅ | Borrow closures |
| Generic lambdas (auto params) | ✅ | `_` type inference |

## Preprocessor

| Feature | Status | Notes |
|---------|--------|-------|
| `#include` | ✅ | Handled by Clang |
| `#define` (constants) | ✅ | Handled by Clang |
| `#define` (macros) | ✅ | Expanded by Clang |
| `#ifdef` / `#ifndef` | ✅ | Handled by Clang |
| `#pragma` | 🚫 | Ignored |

## C++11/14/17/20 Features

| Feature | Status | Notes |
|---------|--------|-------|
| Scoped enums (enum class) | ✅ | Rust enums with `#[repr]` |
| Type aliases (using) | ✅ | `pub type` |
| Auto type deduction | ✅ | Via Clang |
| Range-based for | ✅ | |
| Lambdas | ✅ | |
| Concepts | ✅ | Handled by Clang |
| Ranges | ❌ | |
| Coroutines | ❌ | Should map to async Rust |
| Modules | ❌ | |
| `constexpr` | ✅ | Evaluated by Clang |
| `consteval` | ✅ | Evaluated by Clang |
| Three-way comparison (`<=>`) | ❌ | |
| Designated initializers | ❌ | |

## Standard Library Type Mappings

| Feature | Status | Notes |
|---------|--------|-------|
| `std::string` | ✅ | Maps to `String` |
| `std::vector<T>` | ✅ | Maps to `Vec<T>` |
| `std::map<K,V>` | ✅ | Maps to `BTreeMap<K,V>` |
| `std::unordered_map<K,V>` | ✅ | Maps to `HashMap<K,V>` |
| `std::unique_ptr<T>` | ✅ | Maps to `Box<T>` |
| `std::shared_ptr<T>` | ✅ | Maps to `Arc<T>` |
| `std::weak_ptr<T>` | ✅ | Maps to `Weak<T>` |
| `std::optional<T>` | ✅ | Maps to `Option<T>` |
| `std::array<T, N>` | ✅ | Maps to `[T; N]` |
| `std::span<T>` | ✅ | Maps to `&mut [T]` or `&[T]` for const |
| `std::variant` | ❌ | Should map to enum |
| I/O streams | ❌ | Should map to `std::io` |

---

## Code Generation Quality

| Feature | Status | Notes |
|---------|--------|-------|
| Minimize temporaries | ✅ | Removed redundant type suffixes |
| Dead code elimination | ❌ | |
| Readable variable names | ✅ | Preserves source identifiers |
| Proper indentation | ✅ | |
| Comments | ✅ | Doc comments for functions/classes |

## Testing

| Feature | Status | Notes |
|---------|--------|-------|
| Grammar tests | ✅ | 20/20 passing |
| E2E tests | ✅ | 56/56 passing |
| Compile generated code | ✅ | Automatically verified |
| Run generated code | ✅ | Exit codes verified |

---

## Test Coverage

### Grammar Tests (20/20)
- Arithmetic, comparisons, logical/bitwise operators
- Control flow (if/else, while, for, do-while, switch)
- Functions and recursion
- Structs with fields, methods, constructors
- Pointers, references, arrays
- Ternary operator, nested structs

### E2E Tests (56/56)
- Simple functions, factorial, arrays
- Pointers, references
- Constructors, destructors (Drop trait)
- Copy constructors (Clone trait)
- Single and multiple inheritance
- Virtual/diamond inheritance
- Namespaces and modules
- Operator overloading (binary, subscript, call, deref, arrow)
- Assignment operators
- Exception handling (throw/try/catch)
- Enum classes
- Static members
- Lambdas with captures
- Range-based for loops
- Default parameters
- Const/non-const methods
- Increment/decrement operators
- Pointer arithmetic
- Type aliases
- sizeof/alignof operators
- String literals and char literals
- Implicit type casts (char→int, etc.)

---

*Last updated: 2026-01-22*
