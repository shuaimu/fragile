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
- [x] **1.2.5** Function calls [26:01:17] - resolve_function_call() resolves display names to DefIds via registry lookup; fixed CXToken_Punctuation constant (0 not 1)
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
- [x] **1.3.3b** Implement proper C++ name mangling [26:01:17]
  - Added `cursor_mangled_name()` using `clang_Cursor_getMangling` from libclang
  - Added `mangled_name` field to `ClangNodeKind::FunctionDecl`
  - Updated `convert.rs` to pass mangled_name through to CppFunction
  - Added test `test_mangled_name_for_simple_function` verifying `_Z7add_cppii`
- [x] **1.3.4** Compile via rustc (mir_built override active) [26:01:17]
  - Fixed threading issue: use global statics instead of TLS for C++ registry
  - Fixed MirSource: pass correct DefId instead of CRATE_DEF_ID
  - Fixed arg_count: count locals with is_arg=true
  - Created wrapper source to include stubs as module
- [x] **1.3.5** Link and run: `add(2, 3) == 5` [26:01:17] ✓
  - End-to-end test passes: binary executes and outputs "add_cpp(2, 3) = 5"
  - **MILESTONE ACHIEVED**: C++ compiles through rustc codegen, not clang++!

### 1.4 Verification: No clang++ in Pipeline (DEFERRED)

**Status**: Deferred until MIR injection handles more complex cases.

**Rationale**: While MIR injection works for simple leaf functions (like `add_cpp`),
the full Fragile pipeline (CLI, Mako compilation) still requires clang++ for:
- Complex C++ with function calls, templates, STL
- Object file generation for linking with Rust
- The CLI build system integration

**Prerequisite**: Complete Phase 2 (Basic C++ Features) to enable pure MIR path.

- [ ] Remove `cpp_compiler.rs` (clang++ wrapper) - AFTER Phase 2
- [ ] Remove `compile_cpp_objects()` function - AFTER Phase 2
- [ ] All `.o` files come from rustc only - AFTER Phase 2
- [ ] Add CI check that clang++ is never invoked for codegen - AFTER Phase 2

---

## Phase 2: Basic C++ Features via MIR

Each feature must work through the MIR pipeline, not clang++.

### 2.1 Expressions
- [x] **2.1.1** Integer literals [26:01:17] (see `docs/dev/plan_2_1_integer_literals.md`)
  - [x] 2.1.1a Add `signed` field to `MirConstant::Int`
  - [x] 2.1.1b Add `bit_width()` method to `CppType`
  - [x] 2.1.1c Add `cpp_type` field to `IntegerLiteral` AST node
  - [x] 2.1.1d Update parser to capture type from Clang (uses getAsUnsigned/getAsLongLong for proper values)
  - [x] 2.1.1e Update converter to use actual type info
  - [x] 2.1.1f Update rustc mir_convert.rs for unsigned types
  - [x] 2.1.1g Update all `MirConstant::Int` call sites
  - [x] 2.1.1h Add tests for integer literals (4 tests in parse.rs, 4 tests in types.rs)
- [x] **2.1.2** Float literals [26:01:17] (see `docs/dev/plan_2_1_2_float_literals.md`)
- [x] **2.1.3** Boolean literals [26:01:17] (added CXXBoolLiteralExpr parsing)
- [x] **2.1.4** String literals [26:01:17] (added CXCursor_StringLiteral parsing)
- [x] **2.1.5** Binary operators (+, -, *, /, %, &, |, ^) [already implemented]
- [x] **2.1.6** Unary operators (-, !, ~) [already implemented]
- [x] **2.1.7** Comparison operators (==, !=, <, <=, >, >=) [already implemented]
- [x] **2.1.8** Logical operators (&&, ||) [already implemented]

### 2.2 Statements
- [x] **2.2.1** Variable declarations [already implemented - DeclStmt/VarDecl]
- [x] **2.2.2** Assignment [already implemented - via BinaryOperator::Assign]
- [x] **2.2.3** If/else [already implemented - IfStmt]
- [x] **2.2.4** While loops [already implemented - WhileStmt]
- [x] **2.2.5** For loops [already implemented - ForStmt]
- [x] **2.2.6** Return [already implemented - ReturnStmt]
- [x] **2.2.7** Break/continue [already implemented - BreakStmt/ContinueStmt]

### 2.3 Functions
- [x] **2.3.1** Function definitions [already implemented - FunctionDecl]
- [x] **2.3.2** Function calls [already implemented - CallExpr]
- [x] **2.3.3** Parameters (by value) [already implemented - ParmVarDecl]
- [x] **2.3.4** Return values [already implemented - via ReturnStmt]
- [x] **2.3.5** Recursion [already implemented - via CallExpr]

### 2.4 Basic Types
- [x] **2.4.1** Primitive types (int, float, bool, char) [already implemented - CppType]
- [x] **2.4.2** Pointers (*T) [26:01:17] - Fixed address-of (`&x`) and dereference (`*ptr`) in MIR conversion
  - Address-of now generates `MirRvalue::Ref { place, mutability }` instead of incorrect `MirUnaryOp::Neg`
  - Dereference now adds `MirProjection::Deref` to place projections
  - Added `get_node_type()` helper method for extracting C++ type from AST nodes
  - Added 3 tests: `test_convert_address_of`, `test_convert_dereference`, `test_convert_pointer_ops_combined`
  - See: `docs/dev/plan_2_4_2_pointers.md`
- [ ] **2.4.3** References (&T)
- [ ] **2.4.4** Arrays ([T; N])

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
