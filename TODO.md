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
**E2E Tests**: 58/59 passing (1 ignored due to STL header limitations)

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
- std::variant type mapping and construction (→ Rust enum)
- std::get<T>/std::get<I> on variants (→ match expression)
- std::visit on variants (→ match expression with lambda/functor/function visitor)

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
- [x] **8.3** `std::variant<T...>` → Rust enum with variants ✅ [26:01:22, 23:55]
  - [x] **8.3.1** Type mapping: Parse `std::variant<T1, T2, ...>` and extract template args (~100 LOC) ✅ [26:01:22, 21:46] [docs/dev/plan_8_3_1_variant_type_mapping.md]
  - [x] **8.3.2** Enum generation: Generate Rust enum definitions for variant types with synthetic names (~150 LOC) ✅ [26:01:22, 21:51] [docs/dev/plan_8_3_2_variant_enum_generation.md]
  - [x] **8.3.3** Construction/assignment: Handle variant initialization and reassignment (~100 LOC) ✅ [26:01:22, 22:00] [docs/dev/plan_8_3_3_variant_construction.md]
  - [x] **8.3.4** std::visit: Map to Rust match statements ✅ [26:01:22, 23:55] [docs/dev/plan_8_3_4_std_visit.md]
    - [x] **8.3.4.1** Detection: Add is_std_visit_call() to detect std::visit calls and extract visitor+variants (~60 LOC) ✅
    - [x] **8.3.4.2** Single variant support: Generate match expression for single variant with lambda visitor (~80 LOC) ✅
    - [x] **8.3.4.3** Multiple variant support: Generate cartesian product match arms for 2+ variants (~80 LOC) ✅
    - [x] **8.3.4.4** Functor/function support: Handle non-lambda visitors (functor op_call, function refs) (~60 LOC) ✅
    - [x] **8.3.4.5** Tests and edge cases: Add E2E tests for std::visit patterns (~50 LOC) ✅
  - [x] **8.3.5** std::get<T>/std::get<I>: Map to pattern matching (~70 LOC) ✅ [26:01:22, 23:30] [docs/dev/plan_8_3_5_std_get.md]

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

### 11. I/O Streams (Priority: Medium)
- [x] **11.1** Basic I/O stream type mappings ✅ 2026-01-22
  - [x] **11.1.1** `std::ostream` → `Box<dyn std::io::Write>` type mapping (~50 LOC) ✅ 2026-01-22
  - [x] **11.1.2** `std::istream` → `Box<dyn std::io::Read>` type mapping (~50 LOC) ✅ 2026-01-22
  - [x] **11.1.2a** `std::iostream` → `Box<dyn std::io::Read + std::io::Write>` type mapping ✅ 2026-01-22
  - [x] **11.1.3** `std::cout` → `std::io::stdout()` global mapping (~30 LOC) ✅ 2026-01-22
  - [x] **11.1.4** `std::cerr` → `std::io::stderr()` global mapping (~30 LOC) ✅ 2026-01-22
  - [x] **11.1.5** `std::cin` → `std::io::stdin()` global mapping (~30 LOC) ✅ 2026-01-22
  - [x] **11.1.5a** `std::clog` → `std::io::stderr()` global mapping ✅ 2026-01-22
- [x] **11.2** Stream operators ✅ 2026-01-22
  - [x] **11.2.1** `operator<<` for ostream → `write!()` / `writeln!()` macro calls (~100 LOC) ✅ 2026-01-22
  - [x] **11.2.2** `operator>>` for istream → `read_line()` + parsing (~100 LOC) ✅ 2026-01-22
  - [x] **11.2.3** Handle chained `<<`/`>>` operators (~50 LOC) ✅ 2026-01-22
  - [x] **11.2.3a** Handle `std::endl` → newline in writeln!() ✅ 2026-01-22
- [x] **11.3** String streams ✅ 2026-01-22
  - [x] **11.3.1** `std::stringstream` → `std::io::Cursor<Vec<u8>>` type mapping ✅ 2026-01-22
  - [x] **11.3.2** `std::ostringstream` → `String` type mapping ✅ 2026-01-22
  - [x] **11.3.3** `std::istringstream` → `std::io::Cursor<String>` type mapping ✅ 2026-01-22
- [x] **11.4** File streams ✅ 2026-01-22
  - [x] **11.4.1** `std::ofstream` → `std::fs::File` type mapping ✅ 2026-01-22
  - [x] **11.4.2** `std::ifstream` → `std::fs::File` type mapping ✅ 2026-01-22
  - [x] **11.4.3** `std::fstream` → `std::fs::File` type mapping ✅ 2026-01-22

### 12. C++20 Coroutines (Priority: Medium)
- [ ] **12.1** Coroutine detection and parsing
  - [x] **12.1.1** Detect `co_await`, `co_yield`, `co_return` keywords in function bodies ✅ 2026-01-22 (parsing done in parse.rs)
  - [ ] **12.1.2** Parse coroutine promise types from return type (~80 LOC)
  - [ ] **12.1.3** Identify coroutine frame state variables (~60 LOC)
- [ ] **12.2** Generator coroutines (co_yield)
  - [ ] **12.2.1** Map generator functions to Rust `Iterator` trait implementation (~150 LOC)
  - [x] **12.2.2** `co_yield value` → `yield value` in generator context (~50 LOC) ✅ 2026-01-22
  - [ ] **12.2.3** Generate state machine struct for generator (~200 LOC)
- [ ] **12.3** Async coroutines (co_await)
  - [ ] **12.3.1** Mark coroutine functions as `async fn` (~40 LOC)
  - [x] **12.3.2** `co_await expr` → `expr.await` (~50 LOC) ✅ 2026-01-22
  - [x] **12.3.3** `co_return value` → `return value` in async context (~30 LOC) ✅ 2026-01-22
  - [ ] **12.3.4** Handle awaitable types (map to Future trait) (~100 LOC)
- [ ] **12.4** Task/Promise types
  - [ ] **12.4.1** Map `std::coroutine_handle<>` to internal state pointer (~60 LOC)
  - [ ] **12.4.2** Map common task types (cppcoro::task, etc.) to async blocks (~100 LOC)

### 13. Anonymous Structs and Unions (Priority: Low)
- [ ] **13.1** Anonymous struct support
  - [x] **13.1.1** Detect anonymous struct declarations in AST (~40 LOC) ✅ 2026-01-22
  - [x] **13.1.2** Generate synthetic name for anonymous struct (e.g., `__anon_LineCol`) (~30 LOC) ✅ 2026-01-22
  - [ ] **13.1.3** Flatten anonymous struct fields into parent when used inline (~80 LOC)
- [ ] **13.2** Anonymous union support
  - [ ] **13.2.1** Detect anonymous union declarations in AST (~40 LOC)
  - [ ] **13.2.2** Generate `#[repr(C)] union` with synthetic name (~50 LOC)
  - [ ] **13.2.3** Handle anonymous union field access (direct member access) (~60 LOC)

### 14. Bit Fields (Priority: Low)
- [ ] **14.1** Bit field parsing
  - [x] **14.1.1** Parse bit field width from FieldDecl (`field : width`) (~50 LOC) ✅ 2026-01-22
  - [ ] **14.1.2** Track bit field offset and packing within struct (~80 LOC)
- [ ] **14.2** Bit field code generation
  - [ ] **14.2.1** Generate getter/setter methods for bit field access (~100 LOC)
  - [ ] **14.2.2** Pack adjacent bit fields into appropriate integer type (~120 LOC)
  - [ ] **14.2.3** Handle bit field assignment and initialization (~80 LOC)
- [ ] **14.3** Alternative: Use `bitflags` crate
  - [ ] **14.3.1** Detect bit field patterns that map to flags (~60 LOC)
  - [ ] **14.3.2** Generate `bitflags!` macro invocations (~80 LOC)

### 15. Variadic Functions (Priority: Low)
- [ ] **15.1** C-style variadic functions
  - [x] **15.1.1** Detect variadic function declarations (`...` parameter) (~30 LOC) ✅ (already implemented)
  - [ ] **15.1.2** Map `va_list` to Rust's `std::ffi::VaList` (~50 LOC)
  - [ ] **15.1.3** Map `va_start`/`va_arg`/`va_end` to VaList methods (~80 LOC)
  - [ ] **15.1.4** Generate `extern "C"` with `...` for variadic functions (~40 LOC)
- [ ] **15.2** Variadic templates (already handled by Clang instantiation)
  - Note: Clang instantiates variadic templates, so we transpile the result

### 16. RTTI (Runtime Type Information) (Priority: Low)
- [ ] **16.1** typeid operator
  - [ ] **16.1.1** Detect `typeid(expr)` and `typeid(Type)` expressions (~50 LOC)
  - [ ] **16.1.2** Generate `std::any::TypeId::of::<T>()` for type queries (~60 LOC)
  - [ ] **16.1.3** Handle `typeid` comparison (`==`, `!=`) (~40 LOC)
- [ ] **16.2** type_info class
  - [ ] **16.2.1** Map `std::type_info` to wrapper struct with TypeId (~80 LOC)
  - [ ] **16.2.2** Implement `name()` method via `std::any::type_name` (~40 LOC)
  - [ ] **16.2.3** Implement `hash_code()` via TypeId hash (~30 LOC)
- [ ] **16.3** dynamic_cast improvements
  - [ ] **16.3.1** Improve trait object-based dynamic_cast for deep hierarchies (~100 LOC)
  - [ ] **16.3.2** Handle `dynamic_cast` to reference types (~60 LOC)

### 17. Placement New (Priority: Low)
- [ ] **17.1** Basic placement new
  - [ ] **17.1.1** Detect placement new syntax `new (ptr) Type(args)` (~50 LOC)
  - [ ] **17.1.2** Generate `std::ptr::write(ptr, T::new(...))` (~60 LOC)
  - [ ] **17.1.3** Handle alignment requirements with `std::alloc::Layout` (~80 LOC)
- [ ] **17.2** Placement new with allocators
  - [ ] **17.2.1** Map placement new with custom allocator (~100 LOC)
  - [ ] **17.2.2** Handle array placement new (~80 LOC)

### 18. C++20 Modules (Priority: Low - Long-term)
- [ ] **18.1** Module detection
  - [ ] **18.1.1** Parse `module` and `export module` declarations (~60 LOC)
  - [ ] **18.1.2** Parse `import` declarations (~50 LOC)
  - [ ] **18.1.3** Track module partitions (~80 LOC)
- [ ] **18.2** Module mapping
  - [ ] **18.2.1** Map C++ modules to Rust modules/crates (~100 LOC)
  - [ ] **18.2.2** Handle `export` visibility → `pub` (~50 LOC)
  - [ ] **18.2.3** Map module partitions to submodules (~80 LOC)
- [ ] **18.3** Module interface units
  - [ ] **18.3.1** Generate Rust module files from module interface units (~100 LOC)
  - [ ] **18.3.2** Handle re-exports from module interface (~60 LOC)

### 19. C++20 Ranges (Priority: Low)
- [ ] **19.1** Range adaptors
  - [ ] **19.1.1** Map `std::views::filter` → `.filter()` (~50 LOC)
  - [ ] **19.1.2** Map `std::views::transform` → `.map()` (~50 LOC)
  - [ ] **19.1.3** Map `std::views::take` → `.take()` (~40 LOC)
  - [ ] **19.1.4** Map `std::views::drop` → `.skip()` (~40 LOC)
  - [ ] **19.1.5** Map `std::views::reverse` → `.rev()` (~40 LOC)
- [ ] **19.2** Range algorithms
  - [ ] **19.2.1** Map `std::ranges::for_each` → `.for_each()` (~50 LOC)
  - [ ] **19.2.2** Map `std::ranges::find` → `.find()` (~50 LOC)
  - [ ] **19.2.3** Map `std::ranges::sort` → `.sort()` / `.sort_by()` (~60 LOC)
  - [ ] **19.2.4** Map `std::ranges::copy` → `.collect()` / iterator consumption (~60 LOC)
- [ ] **19.3** Range concepts
  - [ ] **19.3.1** Map range concepts to Rust Iterator trait bounds (~80 LOC)

### 20. Anonymous Namespaces (Priority: Low)
- [ ] **20.1** Anonymous namespace handling
  - [x] **20.1.1** Detect anonymous namespace declarations (~30 LOC) ✅ (already implemented)
  - [ ] **20.1.2** Generate private module with synthetic name (~40 LOC)
  - [ ] **20.1.3** Auto-use contents in parent scope (~50 LOC)
  - [ ] **20.1.4** Mark all items as `pub(super)` for parent access only (~40 LOC)

### 21. Code Quality Improvements (Priority: Low)
- [ ] **21.1** Dead code elimination
  - [ ] **21.1.1** Track unused functions during transpilation (~80 LOC)
  - [ ] **21.1.2** Track unused types during transpilation (~80 LOC)
  - [ ] **21.1.3** Optionally omit unreachable code from output (~60 LOC)
- [ ] **21.2** Private field enforcement
  - [ ] **21.2.1** Parse access specifiers (public/private/protected) (~50 LOC)
  - [ ] **21.2.2** Generate `pub(crate)` for protected, no `pub` for private (~60 LOC)
  - [ ] **21.2.3** Generate accessor methods for private fields when needed (~100 LOC)

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
- I/O streams (`std::cout`, `std::cin`, file streams)
- C++20 coroutines (`co_await`, `co_yield`, `co_return`)
- Anonymous structs/unions
- Bit fields
- C-style variadic functions (`...`)
- RTTI (`typeid`, `type_info`)
- Placement new
- C++20 modules
- C++20 ranges library

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
