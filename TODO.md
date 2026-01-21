# Fragile - C++ to Rust Transpiler

## Overview

Fragile transpiles C++ source code to Rust source code.

```
C++ Source → libclang → Clang AST → Rust Source → rustc → Binary
```

**Why this works**: Clang handles all the hard C++ stuff (templates, overloads, SFINAE).
We just convert the fully-resolved AST to equivalent Rust code.

## Current Status

**Working**:
- Simple functions with control flow (if/else, loops, recursion)
- Structs with fields and methods
- Constructors (default and parameterized)
- Primitive types (int, float, bool, char, pointers, references)
- Binary/unary operators, comparisons, logical ops
- 6/7 test files transpile to compilable Rust

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

### 1. End-to-End Verification
- [ ] **1.1** Verify transpiled code compiles with rustc
- [ ] **1.2** Verify transpiled code runs correctly
- [ ] **1.3** Add integration test that runs full pipeline

### 2. Improve Transpiler Quality
- [ ] **2.1** Reduce temporary variables in generated code
- [ ] **2.2** Handle `nullptr` → `std::ptr::null()` / `std::ptr::null_mut()`
- [ ] **2.3** Handle C++ casts (`static_cast`, `reinterpret_cast`)
- [ ] **2.4** Map C++ namespaces to Rust modules

### 3. OOP Features
- [ ] **3.1** Single inheritance (embed base as first field)
- [ ] **3.2** Virtual methods (manual vtable)
- [ ] **3.3** Destructor → `Drop` trait
- [ ] **3.4** Copy/move constructors

### 4. Memory Management
- [ ] **4.1** `new`/`delete` → `Box::new()` / drop
- [ ] **4.2** `new[]`/`delete[]` → `Vec`
- [ ] **4.3** Smart pointers (`unique_ptr` → `Box`, `shared_ptr` → `Arc`)

### 5. STL Type Mappings
- [ ] **5.1** `std::string` → `String`
- [ ] **5.2** `std::vector<T>` → `Vec<T>`
- [ ] **5.3** `std::map<K,V>` → `BTreeMap<K,V>`
- [ ] **5.4** `std::unordered_map<K,V>` → `HashMap<K,V>`
- [ ] **5.5** `std::optional<T>` → `Option<T>`

### 6. Error Handling
- [ ] **6.1** `throw` → `panic!()` or `Result`
- [ ] **6.2** `try`/`catch` → `catch_unwind` or `Result`

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
- Pointers and references
- Structs and classes
- Methods (instance and const)
- Constructors (default, parameterized)
- Control flow (if, while, for, switch)
- Operators (arithmetic, comparison, logical, bitwise)
- Function templates (via Clang instantiation)

### Partial Support
- Rvalue references (parsed, codegen incomplete)
- Virtual methods (parsed, vtable not generated)
- Namespaces (parsed, not mapped to modules)

### Not Yet Supported
- Inheritance
- Operator overloading
- Exceptions
- STL types
- `new`/`delete`

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
