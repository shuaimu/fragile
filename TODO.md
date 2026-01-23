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
- [x] **12.1** Coroutine detection and parsing ✅
  - [x] **12.1.1** Detect `co_await`, `co_yield`, `co_return` keywords in function bodies ✅ 2026-01-22 (parsing done in parse.rs)
  - [x] **12.1.2** Parse coroutine promise types from return type ✅ [26:01:23, 01:45] [docs/dev/plan_12_1_2_coroutine_promise_types.md]
  - [x] **12.1.3** Identify coroutine frame state variables - SKIPPED (Rust async fn handles frame state automatically; libclang doesn't expose frame internals) ✅ [26:01:23, 02:15]
- [ ] **12.2** Generator coroutines (co_yield)
  - [x] **12.2.1** Map generator functions to Rust `Iterator` return type ✅ [26:01:23, 02:20] (return type: `impl Iterator<Item=T>`, body uses unstable `yield`)
  - [x] **12.2.2** `co_yield value` → `yield value` in generator context (~50 LOC) ✅ 2026-01-22
  - [ ] **12.2.3** Generate state machine struct for generator (~200 LOC) - needed for stable Rust support
- [x] **12.3** Async coroutines (co_await) ✅
  - [x] **12.3.1** Mark coroutine functions as `async fn` (~40 LOC) ✅ 2026-01-22
  - [x] **12.3.2** `co_await expr` → `expr.await` (~50 LOC) ✅ 2026-01-22
  - [x] **12.3.3** `co_return value` → `return value` in async context (~30 LOC) ✅ 2026-01-22
  - [x] **12.3.4** Handle awaitable types (map to Future trait) ✅ 2026-01-23 - Awaitable types pass through as regular types per Section 22 approach; co_await → .await transformation handles runtime behavior
- [x] **12.4** Task/Promise types ✅ 2026-01-23 - Types transpiled from implementation per Section 22 approach
  - [x] **12.4.1** Map `std::coroutine_handle<>` - Type passes through; will be transpiled from libc++ implementation
  - [x] **12.4.2** Map common task types (cppcoro::task, etc.) - Types pass through; will be transpiled from their headers

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
- [x] **19.1** Range adaptors ✅ 2026-01-23
  - [x] **19.1.1** Map `std::views::filter` → `.filter()` (~50 LOC) ✅ 2026-01-23
  - [x] **19.1.2** Map `std::views::transform` → `.map()` (~50 LOC) ✅ 2026-01-23
  - [x] **19.1.3** Map `std::views::take` → `.take()` (~40 LOC) ✅ 2026-01-23
  - [x] **19.1.4** Map `std::views::drop` → `.skip()` (~40 LOC) ✅ 2026-01-23
  - [x] **19.1.5** Map `std::views::reverse` → `.rev()` (~40 LOC) ✅ 2026-01-23
- [x] **19.2** Range algorithms ✅ 2026-01-23
  - [x] **19.2.1** Map `std::ranges::for_each` → `.for_each()` (~50 LOC) ✅ 2026-01-23
  - [x] **19.2.2** Map `std::ranges::find` → `.find()` (~50 LOC) ✅ 2026-01-23
  - [x] **19.2.3** Map `std::ranges::sort` → `.sort()` / `.sort_by()` (~60 LOC) ✅ 2026-01-23
  - [x] **19.2.4** Map `std::ranges::copy` → `.collect()` / iterator consumption (~60 LOC) ✅ 2026-01-23
- [ ] **19.3** Range concepts
  - [ ] **19.3.1** Map range concepts to Rust Iterator trait bounds (~80 LOC)

### 20. Anonymous Namespaces (Priority: Low)
- [x] **20.1** Anonymous namespace handling ✅ 2026-01-22
  - [x] **20.1.1** Detect anonymous namespace declarations (~30 LOC) ✅ (already implemented)
  - [x] **20.1.2** Generate private module with synthetic name (~40 LOC) ✅ 2026-01-22
  - [x] **20.1.3** Auto-use contents in parent scope (~50 LOC) ✅ 2026-01-22 (combined with 20.1.2)
  - [ ] **20.1.4** Mark all items as `pub(super)` for parent access only (~40 LOC)

### 21. Code Quality Improvements (Priority: Low)
- [ ] **21.1** Dead code elimination
  - [ ] **21.1.1** Track unused functions during transpilation (~80 LOC)
  - [ ] **21.1.2** Track unused types during transpilation (~80 LOC)
  - [ ] **21.1.3** Optionally omit unreachable code from output (~60 LOC)
- [ ] **21.2** Private field enforcement
  - [x] **21.2.1** Parse access specifiers (public/private/protected) (~50 LOC) ✅ (already implemented)
  - [ ] **21.2.2** Generate `pub(crate)` for protected, no `pub` for private (~60 LOC)
  - [ ] **21.2.3** Generate accessor methods for private fields when needed (~100 LOC)

---

## C++ Standard Library Transpilation (Major Initiative)

**Goal**: Remove special-case STL type mappings. The C++ standard library should be transpiled exactly like any other C++ code - no special treatment.

**Key Principle**: We require **all C++ source code** to transpile - we cannot link against system libraries. The STL must be vendored and transpiled along with user code.

**STL Implementation**: We vendor **libc++ (LLVM)** source code because:
- Designed to work with Clang (which we use for parsing)
- Cleaner, more readable codebase than libstdc++
- Fewer GCC-specific compiler intrinsics
- Fully open source (Apache 2.0 with LLVM exception)
- Modern C++ standards adopted quickly and cleanly

**Why we need full source (not just headers)**:
- We transpile to Rust source, not object code - cannot link against `libc++.so`
- Header-only parts (most containers, algorithms) work directly
- Non-header parts (`iostream`, `locale`, some `string` functions) need `src/` transpilation
- OS interface code (I/O, threading) needs Rust implementations

**libc++ source**: https://github.com/llvm/llvm-project/tree/main/libcxx
- `include/` - headers (templates, inline functions)
- `src/` - compiled library source (must also be transpiled)

### Current State (to be removed)
The current approach in `crates/fragile-clang/src/types.rs:183-580` has special-case mappings:
- `std::vector<T>` → `Vec<T>`
- `std::unordered_map<K,V>` → `HashMap<K,V>`
- `std::string` → `String`
- etc.

**Problems with this approach**:
- Semantic differences (Rust HashMap vs C++ unordered_map have different guarantees)
- Missing methods (not all STL methods have Rust equivalents)
- Iterator model differences (C++ iterators vs Rust iterators)
- Treating STL as "special" when it's just C++ code
- Maintenance burden of mapping tables

### 22. Remove STL Type Mappings (Priority: Critical)

#### Phase 1: Remove Special-Case Mappings
- [x] **22.1** Remove STL → Rust std mappings from `types.rs`
  - [x] **22.1.1** Remove `std::vector<T>` → `Vec<T>` mapping (lines 267-280)
  - [x] **22.1.2** Remove `std::string` → `String` mapping (lines 255-265)
  - [x] **22.1.3** Remove `std::map`/`std::unordered_map` → `BTreeMap`/`HashMap` (lines 336-352)
  - [x] **22.1.4** Remove smart pointer mappings `unique_ptr`/`shared_ptr`/`weak_ptr` (lines 353-395)
  - [x] **22.1.5** Remove `std::optional` → `Option` mapping (lines 282-287)
  - [x] **22.1.6** Remove `std::array`/`std::span` mappings (lines 289-335)
  - [x] **22.1.7** Remove `std::variant` → enum mapping (lines 468-499)
  - [x] **22.1.8** Remove I/O stream mappings (lines 396-467)

- [x] **22.2** STL types pass through as regular C++ types ✅ 2026-01-23
  - [x] **22.2.1** `std::vector<T>` stays as `std_vector<T>` (awaiting libc++ transpilation)
  - [x] **22.2.2** `std::string` stays as `std_string` (awaiting libc++ transpilation)
  - [x] **22.2.3** All STL types pass through - full transpilation depends on Phase 2-4

#### Phase 2: Vendor libc++ Source Code
- [x] **22.3** Vendor libc++ from LLVM project ✅ [26:01:23, 11:15] [docs/dev/plan_22_3_libc++_setup.md]
  - [x] **22.3.1** Add libc++ as git submodule at `vendor/llvm-project/libcxx` (sparse checkout)
  - [x] **22.3.2** Include both `include/` (headers) and `src/` (library source)
  - [x] **22.3.3** Submodule tracks LLVM main branch (commit f091be6d5, Jan 2026)
  - [x] **22.3.4** Document license (Apache 2.0 with LLVM exception)

- [x] **22.4** Configure build to use vendored libc++ ✅ [26:01:23, 11:35] [docs/dev/plan_22_4_vendored_libcxx_config.md]
  - [x] **22.4.1** Point Clang to vendored `include/` directory - Added `--use-vendored-libcxx` CLI flag
  - [ ] **22.4.2** Transpile `src/*.cpp` files as part of STL support (deferred - not needed until STL feature implementation)
  - [x] **22.4.3** Handle libc++ build configuration macros (`_LIBCPP_*`) - Added `_LIBCPP_HAS_NO_PRAGMA_SYSTEM_HEADER`

- [x] **22.5** Categorize libc++ components by transpilation complexity ✅ [26:01:23, 11:45] [docs/dev/plan_22_5_libcxx_categorization.md]
  - [x] **22.5.1** Header-only (easy): `<vector>`, `<map>`, `<algorithm>`, `<memory>`, etc. - ~40 headers categorized
  - [x] **22.5.2** Partial src (medium): `<string>`, `<locale>`, `<regex>` - ~20 components with src files
  - [x] **22.5.3** OS interface (hard): `<iostream>`, `<fstream>`, `<thread>`, `<mutex>` - ~15 OS-dependent components
  - [x] **22.5.4** Create priority list based on common usage - 4-tier priority list created

#### Phase 3: Handle libc++ Implementation Patterns
- [x] **22.6** Handle libc++ code patterns ✅ 2026-01-23
  - [x] **22.6.1** Handle `_LIBCPP_*` macros and conditionals - Handled by Clang preprocessing ✅
  - [x] **22.6.2** Handle `__` prefixed internal identifiers - Preserved as valid Rust identifiers ✅
  - [x] **22.6.3** Handle inline namespaces (`std::__1::`) - Strip ABI versioning namespaces ✅

#### Phase 4: Fix Transpiler Gaps Exposed by STL Code
- [x] **22.7** Handle Unknown AST nodes that appear in libc++ code ✅ [26:01:23, 14:30]
  - NOTE: "Discriminant(72)" in output is `ClangNodeKind::Unknown` - unhandled Clang AST node kinds
  - [x] **22.7.1** Identify specific Clang AST node kinds producing Unknown (analyze libc++ transpilation output)
    - Common Unknown types: TemplateRef, NamespaceRef, OverloadedDeclRef, NonTypeTemplateParameter, MacroExpansion
  - [x] **22.7.2** Add descriptive handlers for common Unknown nodes to preserve information
    - TemplateRef → "TemplateRef:name", NamespaceRef → "NamespaceRef:name", etc.
    - Helps with debugging and future improvements
  - [ ] **22.7.3** Handle static initialization of function objects (deferred - complex and libc++-specific)

- [x] **22.8** Implement compiler builtin functions ✅ [26:01:23, 12:05]
  - [x] **22.8.1** `__builtin_is_constant_evaluated()` → `false` (runtime always)
  - [x] **22.8.2** `__builtin_memset` → `std::ptr::write_bytes`
  - [x] **22.8.3** `__builtin_memcpy` → `std::ptr::copy_nonoverlapping`
  - [x] **22.8.4** `__builtin_memmove` → `std::ptr::copy`
  - [x] **22.8.5** Other builtins: clz/ctz/popcount, bswap, expect, unreachable, trap, abort, strlen, memcmp

- [x] **22.9** Fix duplicate struct definitions from template specializations ✅ [26:01:23, 13:45]
  - [x] **22.9.1** Generate unique names for each template instantiation - Using type spelling from Clang for template instantiations
  - [x] **22.9.2** Use mangled names or type parameters in struct names - e.g., MyPair<int> → MyPair_int
  - [x] **22.9.3** Deduplicate identical instantiations - Added generated_structs HashSet tracking

- [x] **22.10** Fix invalid type names in generated code ✅ [26:01:23, 12:20]
  - [x] **22.10.1** Convert `long` → `i64`, `unsigned_long_long` → `u64`, etc. (already handled)
  - [x] **22.10.2** Convert `float`/`double` in generic contexts to valid Rust (already handled - f32/f64)
  - [x] **22.10.3** Handle `__int128` and other extended types → i128/u128

- [x] **22.11** Fix template syntax in generated Rust ✅ [26:01:23, 12:50]
  - [x] **22.11.1** Convert `std_vector<int>` to monomorphized name `std_vector_int` - Done via to_rust_type_str()
  - [x] **22.11.2** Or convert to Rust generics - Using monomorphized names for compatibility
  - [x] **22.11.3** Handle nested templates correctly - e.g., `std_array_std_vector_int__2`

- [x] **22.12** Template instantiation (no special handling - Clang does the work) ✅ 2026-01-23
  - [x] **22.12.1** Clang instantiates templates when used; we transpile the result ✅
  - [x] **22.12.2** Verify explicit instantiations work correctly ✅

#### Phase 5: OS Interface Layer (for I/O, threading)
- [ ] **22.13** Implement Rust backends for OS-dependent STL components
  - [ ] **22.13.1** File I/O: map libc++ `<fstream>` to Rust `std::fs`
  - [ ] **22.13.2** Console I/O: map `std::cout`/`std::cin` to Rust `std::io`
  - [ ] **22.13.3** Threading: map `<thread>` to Rust `std::thread`
  - [ ] **22.13.4** Mutexes: map `<mutex>` to Rust `std::sync`
  - [ ] **22.13.5** Atomics: map `<atomic>` to Rust `std::sync::atomic`

#### Phase 6: Update Tests
- [x] **22.14** Update existing tests ✅ 2026-01-23
  - [x] **22.14.1** Update tests expecting `Vec<T>` to expect pass-through type ✅
  - [x] **22.14.2** Update tests expecting `String` to expect pass-through type ✅
  - [x] **22.14.3** Update tests expecting `HashMap`/`BTreeMap` to expect pass-through types ✅
  - [x] **22.14.4** Update tests expecting `Box`/`Arc`/`Weak` to expect pass-through types ✅

- [ ] **22.15** Add new E2E tests for STL usage
  - [ ] **22.15.1** Test `std::vector` operations (push_back, iterator, etc.)
  - [ ] **22.15.2** Test `std::string` operations
  - [ ] **22.15.3** Test `std::map`/`std::unordered_map` operations
  - [ ] **22.15.4** Test smart pointer usage
  - [ ] **22.15.5** Test STL algorithms (std::sort, std::find, etc.)

### Why libc++ Over libstdc++

| Aspect | libc++ (LLVM) | libstdc++ (GNU) |
|--------|---------------|-----------------|
| Clang compatibility | Designed for Clang | GCC-specific extensions |
| Code readability | Clean, modern | Complex `bits/` maze |
| Compiler intrinsics | Minimal | Heavy `__builtin_*` usage |
| Internal namespaces | `std::__1::` | `std::__cxx11::` etc. |
| Header structure | Flat, organized | Nested `bits/` headers |

### Why This Approach is Better

1. **Simpler**: No mapping tables to maintain
2. **Correct**: Exact C++ semantics preserved
3. **Complete**: All STL methods work, not just mapped ones
4. **Consistent**: STL code treated the same as any other C++ code
5. **Self-improving**: Fixes for STL transpilation improve all C++ transpilation

### What This Means for Output

Before (with mappings):
```rust
let v: Vec<i32> = Vec::new();
v.push(42);
```

After (transpiled from libc++):
```rust
let v: std::vector<i32> = std::vector::new();
v.push_back(42);
```

The output is more verbose but semantically identical to the original C++

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
