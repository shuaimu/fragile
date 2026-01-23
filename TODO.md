# Fragile - C++ to Rust Transpiler

## Overview

Fragile transpiles C++ source code to Rust source code.

```
C++ Source → libclang → Clang AST → Rust Source → rustc → Binary
```

**Why this works**: Clang handles all the hard C++ stuff (templates, overloads, SFINAE).
We just convert the fully-resolved AST to equivalent Rust code.

## Current Status

**Grammar Tests**: 20/20 passing
**E2E Tests**: 58/58 passing

**Working**:
- Simple functions with control flow (if/else, while, for, do-while, switch, recursion)
- Structs with fields and methods
- Constructors (default, parameterized, copy)
- Copy constructor → Clone trait
- Destructors → Drop trait
- Primitive types (int, float, bool, char)
- Pointers with unsafe blocks for dereference
- References with Rust borrow semantics (&mut T)
- Arrays with proper initialization and indexing
- Binary/unary operators, comparisons, logical ops, bitwise ops
- Ternary operator
- Nested structs
- nullptr → std::ptr::null_mut()
- C++ casts (static_cast, reinterpret_cast, const_cast)
- new/delete → Box::into_raw/Box::from_raw
- new[]/delete[] → Vec allocation with raw pointer
- Single inheritance (base class embedded as `__base` field)
- Multiple inheritance (multiple `__base` fields)
- Virtual/diamond inheritance (shared virtual base via pointers)
- C++ namespaces → Rust modules (with relative path resolution)
- Virtual method override (static dispatch)
- Inherited field access via `__base`
- Base class constructor delegation in derived constructors
- Operator overloading (binary operators like +, ==, etc.)
- Function call operator (operator() → op_call method with arguments)
- Dynamic dispatch (polymorphism through base pointers via trait objects)
- STL smart pointer type mappings (unique_ptr→Box, shared_ptr→Arc, weak_ptr→Weak)
- Enum class (scoped enums) → Rust enums with #[repr]
- Static class members → `static mut` globals with unsafe access
- Basic lambda expressions → Rust closures with type inference
- Lambda captures ([=] → move, [&] → borrow)
- Generic lambdas (auto params → _ type inference, single-type only)
- Range-based for loops (for x : container → for x in container.iter())
- Increment/decrement operators (++x, x++, --x, x-- with correct pre/post semantics)
- Default function parameters (evaluated at call site via clang_Cursor_Evaluate)
- Const vs non-const methods (auto-detect &self vs &mut self based on modifications)
- Comma operator (C++ (a, b) → Rust block expression { a; b })
- Type aliases (typedef and using declarations → Rust pub type)
- Global variables (static mut with unsafe access)
- Global arrays (const-safe initialization with [0; N])
- Pointer arithmetic (++, --, +=, -= using .add()/.sub())
- Subscript operator [] (returns &mut, correct argument passing, auto-dereference)
- Assignment operators (=, +=, -=, *=, /=, etc. with correct *this return)
- Dereference operator * (op_deref → returns &mut, pointer-to-bool via .is_null())
- Arrow operator -> (op_arrow method → pointer dereference with unsafe block)
- sizeof/alignof (evaluated at compile time by Clang)
- String literals (const char* → b"...\0".as_ptr() as *const i8)
- Character literals ('a' → 65i8 with proper type)
- Implicit type casts (char→int, int→long, etc. via `as` casts)
- std::array<T, N> → [T; N] type mapping
- std::span<T> → &mut [T] / &[T] slice type mapping
- C++20 designated initializers ({ .x = 10, .y = 20 })
- Function pointers (Option<fn(...)> type, Some() wrapping, .unwrap()() calls)
- Three-way comparison operator (<=> → a.cmp(&b) as i8)

**CLI**:
```bash
fragile transpile file.cpp -o output.rs
rustc output.rs -o program
```

## Project Structure

```
crates/
├── fragile-cli      # CLI: fragile transpile
├── fragile-clang    # Core: Clang parsing + Rust codegen
├── fragile-common   # Shared types
├── fragile-runtime  # Runtime support (future)
└── fragile-build    # Build config parsing
```

---

## Next Steps

### 1. End-to-End Verification ✅
- [x] **1.1** Verify transpiled code compiles with rustc
- [x] **1.2** Verify transpiled code runs correctly
- [x] **1.3** Add integration test that runs full pipeline (13 E2E tests)

### 2. Improve Transpiler Quality
- [x] **2.1** Reduce temporary variables in generated code (removed redundant type suffixes)
- [x] **2.2** Handle `nullptr` → `std::ptr::null_mut()`
- [x] **2.3** Handle C++ casts (`static_cast`, `reinterpret_cast`, `const_cast`)
- [x] **2.4** Map C++ namespaces to Rust modules

### 3. OOP Features
- [x] **3.3.1** Parse virtual methods in classes
- [x] **3.3.2** Generate vtable struct for each class with virtuals
- [x] **3.3.3** Add vtable pointer field to class struct
- [x] **3.3.4** Dynamic dispatch via trait objects for polymorphism
- [x] **3.1** Single inheritance (embed base as first field, member access through `__base`)
- [x] **3.2** Virtual method override resolution (static dispatch, inherited field access via `__base`)
- [x] **3.3** Destructor → `Drop` trait
- [x] **3.4** Copy constructor → `Clone` trait (move constructors work via Rust's natural move semantics)

### 4. Memory Management
- [x] **4.1** `new`/`delete` → `Box::into_raw(Box::new())` / `Box::from_raw()` + drop
- [x] **4.2** `new[]`/`delete[]` → Vec allocation with raw pointer (note: delete[] leaks due to size tracking)
- [x] **4.3** Smart pointers (`unique_ptr` → `Box`, `shared_ptr` → `Arc`, `weak_ptr` → `Weak`) - type mapping done

### 5. STL Type Mappings (Type names converted, constructor/operator semantics need work)
- [x] **5.1** `std::string` → `String` (type mapping done)
- [x] **5.2** `std::vector<T>` → `Vec<T>` (type mapping done)
- [x] **5.3** `std::map<K,V>` → `BTreeMap<K,V>` (type mapping done)
- [x] **5.4** `std::unordered_map<K,V>` → `HashMap<K,V>` (type mapping done)
- [x] **5.5** `std::optional<T>` → `Option<T>` (type mapping done)

### 6. Error Handling
- [x] **6.1** `throw` → `panic!("message")`
- [x] **6.2** `try`/`catch` → `std::panic::catch_unwind`

### 7. Advanced Operator Overloading (Priority: High)
- [x] **7.1** Subscript operator [] (returns mutable reference, correct argument passing, auto-dereference) ✅ 2026-01-22
- [x] **7.2** Assignment operators (=, +=, -=, etc.) for custom types ✅ 2026-01-22
- [x] **7.3** Dereference operator * for smart pointer types ✅ 2026-01-22
- [x] **7.4** Arrow operator -> for smart pointer types ✅ 2026-01-22

### 8. Additional STL Type Mappings (Priority: Medium)
- [x] **8.1** `std::array<T, N>` → `[T; N]` with proper type extraction ✅ [26:01:22, 17:15]
- [x] **8.2** `std::span<T>` → `&[T]` slice type mapping ✅ [26:01:22, 17:30]
- [ ] **8.3** `std::variant<T...>` → Rust enum with variants
  - [x] **8.3.1** Type mapping: Parse `std::variant<T1, T2, ...>` and extract template args (~100 LOC) ✅ [26:01:22, 21:46] [docs/dev/plan_8_3_1_variant_type_mapping.md]
  - [x] **8.3.2** Enum generation: Generate Rust enum definitions for variant types with synthetic names (~150 LOC) ✅ [26:01:22, 21:51] [docs/dev/plan_8_3_2_variant_enum_generation.md]
  - [x] **8.3.3** Construction/assignment: Handle variant initialization and reassignment (~100 LOC) ✅ [26:01:22, 22:00] [docs/dev/plan_8_3_3_variant_construction.md]
  - [ ] **8.3.4** std::visit: Map to Rust match statements (complex, may defer) (~200+ LOC)
  - [ ] **8.3.5** std::get<T>/std::get<I>: Map to pattern matching or unwrap (~100 LOC)

### 9. C++20 Features (Priority: Medium)
- [x] **9.1** Designated initializers (`.field = value` syntax) ✅ [26:01:22, 18:15]
- [x] **9.2** Three-way comparison operator (`<=>` spaceship operator) - basic parsing and code gen ✅ [26:01:22, 22:30]
  - Note: Comparing std::strong_ordering result to 0 requires additional std lib support

### 10. Function Pointers (Priority: Medium)
- [x] **10.1** Function pointer support ✅ [26:01:22, 21:50]
  - [x] **10.1.1** Update CppType::Pointer handling for function pointees in to_rust_type_str() ✅ [26:01:22, 18:30]
  - [x] **10.1.2** Handle function-to-pointer decay in assignments (wrap in Some()) ✅ [26:01:22, 21:50]
  - [x] **10.1.3** Handle function pointer calls (use .unwrap()()) ✅ [26:01:22, 21:50]
  - [x] **10.1.4** Handle null initializers (None) and nullptr comparison (.is_none()/.is_some()) ✅ [26:01:22, 22:45]

---

## Grammar Tests (20/20 Passing)

| Test | Feature | Status |
|------|---------|--------|
| 01 | Arithmetic | ✅ |
| 02 | Comparisons | ✅ |
| 03 | Logical operators | ✅ |
| 04 | Bitwise operators | ✅ |
| 05 | If/else | ✅ |
| 06 | While loop | ✅ |
| 07 | For loop | ✅ |
| 08 | Nested loops | ✅ |
| 09 | Break/continue | ✅ |
| 10 | Functions | ✅ |
| 11 | Recursion | ✅ |
| 12 | Struct basic | ✅ |
| 13 | Struct methods | ✅ |
| 14 | Struct constructor | ✅ |
| 15 | Pointers | ✅ |
| 16 | References | ✅ |
| 17 | Arrays | ✅ |
| 18 | Ternary | ✅ |
| 19 | Do-while | ✅ |
| 20 | Nested struct | ✅ |

---

## Test Files

| File | Status | Notes |
|------|--------|-------|
| `tests/cpp/add_simple.cpp` | Compiles | Simple function + struct |
| `tests/cpp/class.cpp` | Compiles | Methods with `(*self).field` |
| `tests/cpp/constructor.cpp` | Compiles | Constructor calls |
| `tests/cpp/namespace.cpp` | Compiles | Namespace handling |
| `tests/cpp/factorial.cpp` | Compiles | Recursion |
| `tests/cpp/doctest_simple.cpp` | 28 errors | STL internals (low priority) |

---

## Feature Support

See `docs/transpiler-status.md` for detailed feature matrix.

### Fully Supported
- Primitive types (int, float, bool, char)
- Pointers (with unsafe blocks)
- References (with Rust borrow semantics)
- Arrays (initialization and indexing)
- Structs and classes
- Methods (instance, const, mutable)
- Constructors (default, parameterized)
- Control flow (if, while, for, do-while, switch)
- Operators (arithmetic, comparison, logical, bitwise)
- Function templates (via Clang instantiation)

### Partial Support
- Rvalue references (parsed, basic return-by-value works)

### Not Yet Supported
- (All major OOP features now supported!)

---

## Commands

```bash
# Transpile C++ to Rust
fragile transpile file.cpp -o output.rs

# Transpile with include paths
fragile transpile file.cpp -I /path/to/headers -o output.rs

# Generate stubs only (no function bodies)
fragile transpile file.cpp --stubs-only

# Build and test
cargo build
cargo test --package fragile-clang
```
