# Fragile Compiler - TODO

## Overview

Fragile is a polyglot compiler unifying Rust, C++, and Go at the **rustc MIR level**.

**Architecture Goal**: C++ code compiles through rustc's codegen, NOT through clang++.

```
C++ Source
    │
    ▼ (libclang - parsing)
Clang AST
    │
    ▼ (fragile-clang - conversion)
Fragile MIR
    │
    ▼ (mir_built query override)
rustc MIR  ◄── Unified with Rust code
    │
    ▼ (rustc LLVM backend)
Object file (.o)
    │
    ▼ (linker)
Executable
```

**Key Principle**: NO separate clang++ compilation. ALL codegen goes through rustc.

```
Status Legend:
  [x] Completed
  [-] In Progress
  [ ] Not Started
  [!] Wrong approach (to be removed/redone)
```

---

## Test Targets (Progressive Complexity)

### Target 1: Simple Test Files
Hand-written C++ files for testing each feature incrementally.
- Location: `tests/cpp/`
- Examples: add.cpp, fibonacci.cpp, struct.cpp, class.cpp

### Target 2: doctest (Single-Header Testing Framework)
- Repository: https://github.com/doctest/doctest
- Size: ~5000 lines, single header
- Why: Popular, exercises templates/macros, no external deps

### Target 3: fmt (Formatting Library)
- Repository: https://github.com/fmtlib/fmt
- Size: Small library, high-quality modern C++
- Why: Popular, real-world usage

### Future: Mako (Deferred)
- The original target, deferred until core infrastructure is solid

---

## Phase 1: Core MIR Pipeline (Current Focus)

### 1.1 Validate Existing Infrastructure
- [x] Clang AST parsing works (fragile-clang, 571 tests)
- [x] MIR representation exists (MirBody, MirStatement, etc.)
- [x] rustc query override infrastructure exists
- [!] **WRONG**: Current builds use clang++ for codegen
- [ ] **TODO**: Remove clang++ codegen path entirely

### 1.2 MIR-to-rustc Conversion (mir_convert.rs)
- [x] Basic structure exists
- [x] **1.2.1** Primitive types (i32, f64, bool, char) [26:01:17] - convert_type() handles all primitive types (void, bool, char, short, int, long, long long, float, double with signed/unsigned variants)
- [x] **1.2.2** Arithmetic operations (add, sub, mul, div) [26:01:17] - convert_binop() maps Add, Sub, Mul, Div, Rem + bit ops
- [x] **1.2.3** Comparison operations (eq, lt, gt) [26:01:17] - convert_binop() maps Eq, Ne, Lt, Le, Gt, Ge
- [x] **1.2.4** Local variables and assignments [26:01:17] - convert_statement() handles Assign, convert_local() handles locals
- [-] **1.2.5** Function calls - convert_terminator() handles Call but uses placeholder func operand (needs resolution)
- [x] **1.2.6** Control flow (if/else, loops) [26:01:17] - convert_terminator() handles Goto, SwitchInt
- [x] **1.2.7** Return statements [26:01:17] - convert_terminator() handles Return terminator

### 1.3 End-to-End Test: add.cpp
First complete test through the correct pipeline:

```cpp
// tests/cpp/add.cpp
int add(int a, int b) {
    return a + b;
}
```

**Milestone**: `add.cpp` compiles to `.o` via rustc codegen (not clang++)

**Critical Discovery [26:01:17]**: extern "C" functions don't have MIR!
- `extern "C" { fn foo(); }` creates ForeignItem, not Function
- ForeignItems are resolved by linker, `mir_built` is never called
- **Solution**: Generate regular Rust functions with stub bodies, then inject MIR
- See: `docs/dev/plan_1_3_end_to_end_add_cpp.md`

- [x] **1.3.1** Parse add.cpp → Clang AST - Already works (fragile-clang)
- [x] **1.3.2** Convert Clang AST → Fragile MIR - Already works (MirConverter)
- [x] **1.3.3** Generate Rust function stubs (NOT extern "C") for MIR injection [26:01:17]
  - Modified `stubs.rs` to generate regular Rust functions with stub bodies
  - Functions use `#[export_name = "mangled"]` instead of extern "C"
  - Stub body: `unreachable!("Fragile: C++ MIR should be injected")`
  - Added `generate_rust_stubs_extern()` for backwards compatibility
  - Updated `rustc_integration.rs` to detect regular functions (not just ForeignItems)
- [ ] **1.3.4** Compile via rustc (mir_built override active)
- [ ] **1.3.5** Link and run: `add(2, 3) == 5`

### 1.4 Verification: No clang++ in Pipeline
- [ ] Remove `cpp_compiler.rs` (clang++ wrapper)
- [ ] Remove `compile_cpp_objects()` function
- [ ] All `.o` files come from rustc only
- [ ] Add CI check that clang++ is never invoked for codegen

---

## Phase 2: Basic C++ Features via MIR

Each feature must work through the MIR pipeline, not clang++.

### 2.1 Expressions
- [ ] Integer literals
- [ ] Float literals
- [ ] Boolean literals
- [ ] String literals
- [ ] Binary operators (+, -, *, /, %, &, |, ^)
- [ ] Unary operators (-, !, ~)
- [ ] Comparison operators (==, !=, <, <=, >, >=)
- [ ] Logical operators (&&, ||)

### 2.2 Statements
- [ ] Variable declarations
- [ ] Assignment
- [ ] If/else
- [ ] While loops
- [ ] For loops
- [ ] Return
- [ ] Break/continue

### 2.3 Functions
- [ ] Function definitions
- [ ] Function calls
- [ ] Parameters (by value)
- [ ] Return values
- [ ] Recursion

### 2.4 Basic Types
- [ ] Primitive types (int, float, bool, char)
- [ ] Pointers (*T)
- [ ] References (&T)
- [ ] Arrays ([T; N])

---

## Phase 3: Structs and Classes via MIR

### 3.1 Structs
- [ ] Struct definitions
- [ ] Field access
- [ ] Struct literals
- [ ] Nested structs

### 3.2 Classes
- [ ] Class definitions
- [ ] Constructors
- [ ] Destructors
- [ ] Member functions
- [ ] Access specifiers (public/private)

### 3.3 Inheritance
- [ ] Single inheritance
- [ ] Virtual functions
- [ ] Vtable generation via MIR
- [ ] Dynamic dispatch

---

## Phase 4: Templates via MIR

### 4.1 Function Templates
- [ ] Basic function templates
- [ ] Template instantiation
- [ ] Type deduction

### 4.2 Class Templates
- [ ] Basic class templates
- [ ] Template specialization

---

## Phase 5: Test Target - doctest

### 5.1 Setup
- [ ] Clone doctest as submodule
- [ ] Create test file using doctest

### 5.2 Compile doctest Tests
- [ ] Parse doctest.h header
- [ ] Compile simple test file
- [ ] Run tests successfully

---

## Cleanup: Remove Wrong Approach

### Files to Remove/Refactor
- [ ] `crates/fragile-rustc-driver/src/cpp_compiler.rs` - DELETE
- [ ] `crates/fragile-rustc-driver/src/stubs.rs` - REFACTOR (no extern "C" stubs)
- [ ] Remove all `compile_cpp_objects()` calls
- [ ] Remove `CppCompilerConfig`, `CppCompiler` types

### Tests to Update
- [ ] Update all tests to use MIR pipeline
- [ ] Remove tests that rely on clang++ codegen

---

## What We Keep (Correct Parts)

### fragile-clang crate
- [x] Clang AST parsing via libclang
- [x] CppModule, CppFunction, CppStruct types
- [x] MirBody, MirStatement, MirTerminator types
- [x] Type conversion (CppType)

### fragile-rustc-driver crate
- [x] mir_convert.rs - MIR conversion (needs completion)
- [x] rustc_integration.rs - Query override infrastructure
- [x] CppMirRegistry - Function registry

### fragile-runtime crate
- [x] Runtime support functions (keep for later)

---

## Current Status

**We took a wrong turn**: Built a working hybrid system using clang++ for C++ codegen.

**Correction needed**: Implement true MIR injection where ALL code goes through rustc.

**Next action**:
1. Create simple `add.cpp` test
2. Make it compile through MIR pipeline only
3. Verify no clang++ is invoked

---

## Architecture Reference

### Correct Flow (Goal)
```
add.cpp ──► libclang ──► Clang AST ──► Fragile MIR ──► rustc MIR ──► LLVM ──► add.o
```

### Wrong Flow (Current - to be removed)
```
add.cpp ──► libclang ──► Clang AST ──► Rust stubs (extern "C")
                                              │
add.cpp ──► clang++ ─────────────────────────►├──► linker ──► executable
                                              │
main.rs ──► rustc ────────────────────────────┘
```

---

## References

- [awesome-hpp](https://github.com/p-ranav/awesome-hpp) - Header-only C++ libraries
- [doctest](https://github.com/doctest/doctest) - Testing framework (Target 2)
- [fmt](https://github.com/fmtlib/fmt) - Formatting library (Target 3)
