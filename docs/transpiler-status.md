# C++ to Rust Transpiler Status

This document tracks the implementation status of the C++ to Rust transpiler.

## Overview

The transpiler converts C++ source code to Rust source code via:
```
C++ Source → Clang (libclang) → Clang AST → MIR → Rust Source → rustc → Binary
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
| `size_t` | ⚠️ | Needs explicit handling |
| `nullptr_t` | ❌ | |

## Compound Types

| Feature | Status | Notes |
|---------|--------|-------|
| Pointers (`T*`) | ✅ | Maps to `*mut T` / `*const T` |
| References (`T&`) | ✅ | Maps to `&T` / `&mut T` |
| Rvalue references (`T&&`) | ⚠️ | Parsed, codegen incomplete |
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
| Default constructor | ✅ | Generates `new() -> Self { ... }` |
| Parameterized constructor | ✅ | Generates `new_N(...)` with param mapping |
| Copy constructor | ❌ | |
| Move constructor | ❌ | |
| Destructor | ❌ | Should map to `Drop` trait |
| Member initializer lists | ✅ | Positional mapping from params to fields |
| Constructor calls | ⚠️ | AST parsing incomplete |

## Methods

| Feature | Status | Notes |
|---------|--------|-------|
| Instance methods | ✅ | Body generates `(*self).field` access |
| Static methods | ⚠️ | Signature correct |
| Const methods | ✅ | Maps to `&self` |
| Method calls | ⚠️ | AST parsing incomplete |
| Virtual methods | ❌ | Need manual vtable |
| Pure virtual methods | ❌ | |
| Override/final | ❌ | |
| Operator overloading | ❌ | Should map to Rust traits |

## Inheritance

| Feature | Status | Notes |
|---------|--------|-------|
| Single inheritance | ❌ | Need to embed base as first field |
| Multiple inheritance | ❌ | Complex, low priority |
| Virtual inheritance | 🚫 | Out of scope for now |
| `dynamic_cast` | ❌ | |
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
| Default parameters | ❌ | |
| Function overloading | ⚠️ | Clang resolves, but name mangling needed |

## Expressions

| Feature | Status | Notes |
|---------|--------|-------|
| Integer literals | ✅ | With type suffix |
| Float literals | ✅ | With type suffix |
| Bool literals | ✅ | |
| String literals | ❌ | Need `&'static str` or `CStr` |
| Char literals | ❌ | |
| Binary operators (+, -, *, /, %) | ✅ | |
| Comparison operators | ✅ | |
| Logical operators (&&, \|\|, !) | ✅ | |
| Bitwise operators | ✅ | |
| Assignment (=) | ✅ | |
| Compound assignment (+=, etc.) | ⚠️ | Parsed, codegen may be incomplete |
| Increment/decrement (++, --) | ⚠️ | |
| Ternary operator (?:) | ⚠️ | Converted to if/else in MIR |
| Comma operator | ❌ | |
| `sizeof` | ❌ | Should use `std::mem::size_of` |
| `alignof` | ❌ | Should use `std::mem::align_of` |
| Type casts | ⚠️ | Basic casts work |
| `reinterpret_cast` | ❌ | Should use `transmute` |
| `static_cast` | ⚠️ | |
| `const_cast` | ❌ | |

## Statements

| Feature | Status | Notes |
|---------|--------|-------|
| Variable declaration | ✅ | `let mut` |
| If/else | ✅ | Via MIR SwitchInt |
| While loop | ✅ | Via MIR Goto |
| For loop | ✅ | Via MIR Goto |
| Do-while loop | ✅ | Via MIR Goto |
| Switch/case | ⚠️ | Via MIR SwitchInt |
| Break | ✅ | |
| Continue | ✅ | |
| Return | ✅ | |
| Goto | ❌ | Not supported in safe Rust |

## Templates

| Feature | Status | Notes |
|---------|--------|-------|
| Function templates | ⚠️ | Clang instantiates, we transpile result |
| Class templates | ⚠️ | Clang instantiates, we transpile result |
| Template specialization | ⚠️ | Via Clang |
| Partial specialization | ⚠️ | Via Clang |
| Variadic templates | ⚠️ | Via Clang |
| SFINAE | ✅ | Handled by Clang |
| Concepts (C++20) | ✅ | Handled by Clang |

## Namespaces

| Feature | Status | Notes |
|---------|--------|-------|
| Namespace declaration | ⚠️ | Parsed, not reflected in output modules |
| Nested namespaces | ⚠️ | |
| Using directive | ❌ | |
| Using declaration | ❌ | |
| Anonymous namespace | ❌ | |

## Memory Management

| Feature | Status | Notes |
|---------|--------|-------|
| Stack allocation | ✅ | Local variables |
| `new` / `delete` | ❌ | Should use `Box` |
| `new[]` / `delete[]` | ❌ | Should use `Vec` |
| Placement new | ❌ | Should use `ptr::write` |
| Smart pointers | ❌ | Should map to Rust equivalents |

## Error Handling

| Feature | Status | Notes |
|---------|--------|-------|
| Exceptions (`throw`) | ❌ | Should use `panic!` or `Result` |
| `try`/`catch` | ❌ | Should use `catch_unwind` |
| `noexcept` | ⚠️ | Parsed, not enforced |
| Stack unwinding | ❌ | |

## Preprocessor

| Feature | Status | Notes |
|---------|--------|-------|
| `#include` | ✅ | Handled by Clang |
| `#define` (constants) | ✅ | Handled by Clang |
| `#define` (macros) | ✅ | Expanded by Clang |
| `#ifdef` / `#ifndef` | ✅ | Handled by Clang |
| `#pragma` | 🚫 | Ignored |

## C++20/23 Features

| Feature | Status | Notes |
|---------|--------|-------|
| Concepts | ✅ | Handled by Clang |
| Ranges | ❌ | |
| Coroutines | ❌ | Should map to async Rust |
| Modules | ❌ | |
| `constexpr` | ⚠️ | Evaluated by Clang |
| `consteval` | ⚠️ | Evaluated by Clang |
| Three-way comparison (`<=>`) | ❌ | |
| Designated initializers | ❌ | |

## Standard Library

| Feature | Status | Notes |
|---------|--------|-------|
| `std::string` | ❌ | Should map to `String` |
| `std::vector` | ❌ | Should map to `Vec` |
| `std::map` | ❌ | Should map to `BTreeMap` |
| `std::unordered_map` | ❌ | Should map to `HashMap` |
| `std::unique_ptr` | ❌ | Should map to `Box` |
| `std::shared_ptr` | ❌ | Should map to `Arc` |
| `std::optional` | ❌ | Should map to `Option` |
| `std::variant` | ❌ | Should map to enum |
| `std::array` | ❌ | Should map to `[T; N]` |
| `std::span` | ❌ | Should map to `&[T]` |
| I/O streams | ❌ | Should map to `std::io` |

---

## Code Generation Quality

| Feature | Status | Notes |
|---------|--------|-------|
| Minimize temporaries | ❌ | Currently generates many locals |
| Dead code elimination | ❌ | |
| Readable variable names | ⚠️ | Uses MIR names when available |
| Proper indentation | ✅ | |
| Comments | ⚠️ | Doc comments for functions |

## Testing

| Feature | Status | Notes |
|---------|--------|-------|
| Unit tests | ✅ | Basic function and struct tests |
| Integration tests | ⚠️ | factorial.cpp works |
| Compile generated code | ❌ | Not automatically verified |
| Run generated code | ❌ | Not automatically verified |

---

## Priority Implementation Order

### Phase 1: Core Features (Current)
1. ✅ Basic types
2. ✅ Functions
3. ✅ Structs
4. ⚠️ Methods (in progress)
5. ❌ Constructors/Destructors

### Phase 2: OOP Features
1. ❌ Single inheritance
2. ❌ Virtual methods (manual vtable)
3. ❌ Operator overloading

### Phase 3: Memory & Errors
1. ❌ `new`/`delete` → `Box`
2. ❌ Smart pointers
3. ❌ Exceptions → `Result`/`panic`

### Phase 4: Standard Library
1. ❌ `std::string` → `String`
2. ❌ `std::vector` → `Vec`
3. ❌ `std::map` → `BTreeMap`

### Phase 5: Advanced
1. ❌ Coroutines → async
2. ❌ Multiple inheritance (if needed)

---

## Known Issues

1. **Constructor calls**: `Point p1;` becomes `p1 = ()` instead of `Point::new()`
2. **Method calls**: `p.get_x()` becomes `unknown()` instead of proper method call
3. **Redundant locals**: Generated code has many unnecessary temporary variables
4. **No namespace modules**: C++ namespaces don't create Rust modules yet
5. **CXXConstructExpr not handled**: libclang's constructor expression nodes need special handling

## Recent Fixes (2026-01-19)

1. **Method body generation**: Methods now correctly access fields via `(*self).field`
2. **Constructor body generation**: Constructors generate proper `Self { field: value }` initialization
3. **Implicit `this` handling**: Member expressions without explicit base use `this` local
4. **`this` → `self` translation**: C++ `this` is translated to Rust `self` in generated code
5. **Parser bug fix**: Fixed visitor context passing in libclang AST traversal

---

*Last updated: 2026-01-19*
