# Fragile Compiler - TODO

## Overview

Fragile is a polyglot compiler unifying Rust, C++, and Go at the **rustc MIR level**.

**Architecture Goal**: C++ code compiles through rustc's codegen, NOT through clang++ or custom LLVM.

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

**Key Principle**: NO separate compilation paths. ALL codegen goes through rustc.

```
Status Legend:
  [x] Completed
  [-] In Progress
  [ ] Not Started
  [!] Wrong approach (to be removed/redone)
```

---

## ⚠️ CRITICAL: Wrong Approaches We've Taken

### Wrong Approach #1: clang++ Hybrid (Link-time)
```
C++ ──► clang++ ──► .o ──┐
                         ├──► linker ──► executable
Rust ──► rustc ──► .o ──┘
```
**Why wrong**: Two separate compilers, no shared optimization, just FFI at link time.

### Wrong Approach #2: inkwell/tree-sitter (Custom LLVM)
```
C++ ──► tree-sitter ──► HIR ──► inkwell ──► LLVM IR ──► .o ──┐
                                                              ├──► linker
Rust ──► rustc ──────────────────────────────────────► .o ──┘
```
**Why wrong**: Still two separate compilers! inkwell generates LLVM independently of rustc.
The code only meets at link time. No cross-language inlining, no shared analysis.

**Status**: The `fragile-codegen` crate using inkwell IS WRONG and should be deprecated.
The `fragile-frontend-*` crates using tree-sitter ARE WRONG for the final architecture.

### Correct Approach: rustc MIR Injection (Compile-time)
```
C++ ──► libclang ──► Clang AST ──► Fragile MIR ──┐
                                                  ├──► rustc MIR ──► rustc backend ──► .o
Rust ──► rustc frontend ─────────────────────────┘
```
**Why correct**: Single compiler (rustc), single optimization pipeline, true unification.
Both languages go through the SAME MIR and SAME codegen.

### What Needs to Happen
1. **Deprecate** `fragile-codegen` (inkwell) - it's a dead end
2. **Deprecate** `fragile-frontend-*` (tree-sitter) for production - use only for prototyping
3. **Complete** `fragile-rustc-driver` with MIR injection via `mir_built` query override
4. **Complete** `fragile-clang` for Clang AST → Fragile MIR conversion

---

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
- [x] **2.4.3** References (&T) [26:01:17] - Already fully implemented in parser and type system
  - CppType::Reference supports lvalue (&T), rvalue (T&&), and const references
  - Parser handles CXType_LValueReference and CXType_RValueReference
  - Added 3 tests: `test_parse_lvalue_reference_parameter`, `test_parse_const_lvalue_reference_parameter`, `test_parse_rvalue_reference_parameter`
  - Integration tests exist: `test_rvalue_reference`, `test_const_reference`, etc.
- [x] **2.4.4** Arrays ([T; N]) [26:01:17] - Added ArraySubscriptExpr handling in convert_expr()
  - Converts `arr[index]` to MirOperand with MirProjection::Index
  - Handles compile-time constant indices properly
  - Falls back to index 0 for runtime variable indices (TODO: future enhancement)
  - Added 3 tests: `test_convert_array_subscript`, `test_convert_array_subscript_variable_index`, `test_convert_array_subscript_nested`
  - See: `docs/dev/plan_2_4_4_arrays.md`

---

## Phase 3: Structs and Classes via MIR

### 3.1 Structs
- [x] **3.1.1** Struct definitions [already implemented] - CppStruct parsing and stub generation
  - Parser handles RecordDecl (struct/class)
  - Fields, methods, constructors, destructors extracted
  - Rust stubs generated with #[repr(C)]
  - Existing tests: test_struct_default_access, test_namespace_struct
- [x] **3.1.2** Field access (MemberExpr → MirProjection::Field) [26:01:17]
  - Added MemberExpr handling in convert_expr() for both dot (`.`) and arrow (`->`) access
  - Updated MirProjection::Field to include field name for later resolution
  - Arrow access generates Deref + Field projections
  - Added 3 tests: test_convert_member_expr_dot, test_convert_member_expr_arrow, test_convert_nested_member_expr
  - See: `docs/dev/plan_3_1_2_field_access.md`
- [x] **3.1.3** Struct literals / aggregate initialization [26:01:17]
  - [x] 3.1.3a Add `InitListExpr` AST node kind in ast.rs
  - [x] 3.1.3b Add parser handling for `CXCursor_InitListExpr` and `CXXFunctionalCastExpr` in parse.rs
  - [x] 3.1.3c Add MIR aggregate initialization support (MirRvalue::Aggregate)
  - [x] 3.1.3d Add conversion from InitListExpr to MIR in convert.rs
  - [x] 3.1.3e Add tests: test_convert_init_list_struct, test_convert_init_list_variable, test_convert_init_list_array
  - Fixed CastExpr to skip TypeRef children when finding expression value
  - See: `docs/dev/plan_3_1_3_init_list.md`
- [x] **3.1.4** Nested structs [26:01:17] - Already implemented, verified with tests
  - Struct fields with nested struct types (CppType::Named) work correctly
  - Nested field access (o.inner.x) generates chained MirProjection::Field
  - Nested aggregate initialization (Outer{{1, 2}, 3}) works via recursive InitListExpr
  - Added 3 tests: test_nested_struct_definition, test_nested_aggregate_initialization, test_nested_struct_assignment

### 3.2 Classes
- [x] **3.2.1** Class definitions [already implemented] - Classes parsed via CXCursor_ClassDecl
  - Classes use same infrastructure as structs (CppStruct with is_class flag)
  - Access specifiers properly handled via libclang's clang_getCXXAccessSpecifier()
  - Default private access for class members works correctly
  - Existing test: test_class_access_specifiers
- [x] **3.2.2** Constructors [already implemented] - CppConstructor with kind detection
  - Default, copy, move, and parameterized constructors parsed
  - Member initializer lists extracted
  - Constructor bodies converted to MIR
  - Existing tests: test_default_constructor, test_copy_constructor, test_move_constructor
- [x] **3.2.3** Destructors [already implemented] - CppDestructor with optional body
  - Destructors parsed and stored in CppStruct
  - Destructor bodies converted to MIR when present
  - Existing test: test_destructor
- [x] **3.2.4** Member functions [already implemented] - CppMethod with MIR body
  - Methods parsed with return type, params, const/static modifiers
  - Method bodies converted to MIR
  - Static methods supported
  - Existing tests in integration_test.rs
- [x] **3.2.5** Access specifiers (public/private) [already implemented]
  - libclang's clang_getCXXAccessSpecifier returns effective access
  - Default access differs between struct (public) and class (private)
  - AccessSpecifier enum: Public, Private, Protected

### 3.3 Inheritance
- [x] **3.3.1** Single inheritance [already implemented] - CppBaseClass in CppStruct.bases
  - CXCursor_CXXBaseSpecifier parsed with access specifier
  - Public/protected/private inheritance supported
  - Virtual inheritance flag tracked
  - Existing tests: test_single_inheritance, test_protected_inheritance, test_private_inheritance, test_virtual_inheritance
- [x] **3.3.2** Virtual functions [already implemented] - CppMethod flags
  - is_virtual, is_pure_virtual, is_override, is_final tracked
  - Virtual function detection via clang_getCursorSemanticParent
  - Existing tests: test_virtual_function, test_pure_virtual_function, test_override_specifier
- [x] **3.3.3** Vtable generation via MIR [26:01:17]
  - Added VtableEntry and CppVtable structures to lib.rs
  - CppModule.vtables stores vtables for all polymorphic classes
  - CppStruct.vtable_name stores mangled vtable name (if polymorphic)
  - CppStruct.is_polymorphic() method checks for virtual methods
  - MirStatement::InitVtable for constructor vtable pointer initialization
  - Vtable generation in convert_struct() for classes with virtual methods
  - 6 new tests: test_vtable_generation_for_polymorphic_class, test_no_vtable_for_non_polymorphic_class, test_is_polymorphic_method, test_vtable_with_pure_virtual, test_constructor_vtable_init, test_constructor_no_vtable_init_for_non_polymorphic
- [ ] **3.3.4** Dynamic dispatch (partial - VirtualCall terminator added [26:01:17])
  - MirTerminator::VirtualCall added with receiver, vtable_index, args
  - Helper functions: try_extract_member_call, unwrap_casts, extract_class_name
  - TODO: Integrate with CallExpr conversion (needs method virtuality lookup)
  - TODO: Update rustc-driver to translate VirtualCall to rustc MIR

---

## Phase 4: Templates via MIR

### 4.1 Function Templates
- [x] **4.1.1** Basic function templates [already implemented] - CppFunctionTemplate
  - CXCursor_FunctionTemplate (cursor kind 30) parsed
  - Template parameters with variadic (typename...) support
  - Return type and params with template types (CppType::TemplateParam)
  - Requires clause support for C++20 constraints
  - Comprehensive test coverage (30+ tests)
- [x] **4.1.2** Template instantiation [already implemented]
  - `instantiate()` method on CppFunctionTemplate
  - `add_specialization()` for explicit specializations
  - Test: test_template_instantiation
- [x] **4.1.3** Type deduction [already implemented] - TypeDeducer in deduce.rs
  - Basic deduction from call arguments
  - Explicit template argument support
  - Conflict detection for incompatible deductions
  - Tests: test_deduce_simple_*, test_explicit_*

### 4.2 Class Templates
- [x] **4.2.1** Basic class templates [already implemented] - CppClassTemplate
  - CXCursor_ClassTemplate (cursor kind 31) parsed
  - Fields, constructors, methods, member templates all preserved
  - Tests: test_class_template_basic, test_class_template_with_methods
- [x] **4.2.2** Template specialization [already implemented]
  - CXCursor_ClassTemplatePartialSpecialization (cursor kind 32)
  - CppClassTemplatePartialSpec with specialization_args pattern
  - Tests: test_partial_specialization_*

---

## Phase 5: Test Target - doctest

### 5.1 Setup
- [x] Clone doctest as submodule [26:01:17]
  - Added vendor/doctest as git submodule from https://github.com/doctest/doctest
  - Header-only library (~323KB doctest.h)
- [x] Create test file using doctest [26:01:17]
  - Created tests/cpp/doctest_simple.cpp with basic tests
  - Tests factorial function, comparisons, and subcases
  - Verified compiles with clang++ -std=c++17

### 5.2 Compile doctest Tests
- [x] Parse doctest.h header [26:01:17]
  - fragile-clang successfully parses doctest_simple.cpp (includes doctest.h)
  - Finds 3,145 functions, 1,905 function templates, 792 structs/classes, 667 class templates
  - Correctly identifies `factorial` function and generated test case functions
  - Created parse_doctest.rs example for testing
- [x] Compile simple test file [26:01:17]
  - [x] Created factorial.cpp standalone test (without doctest) [26:01:17]
    - tests/cpp/factorial.cpp - minimal test with recursion and control flow
    - tests/clang_integration/factorial.cpp - integration test version
    - MIR conversion verified: 4 basic blocks, 6 locals, recursive calls work
    - Added test_end_to_end_factorial_cpp test - passes
    - Stub generation works: factorial function produces correct extern "C" stub
  - [x] Test full MIR injection via rustc-integration feature [26:01:17]
    - Fixed MIR block allocation bug in if/else handling
    - Added reserve_block() method for forward references
    - Added needs_terminator() to avoid spurious Goto blocks after return
    - test_compile_factorial_with_rustc passes: factorial(5) = 120
  - [ ] Test doctest_simple.cpp (blocked on template/STL support)
- [ ] Run tests successfully (blocked on doctest_simple.cpp)

---

## Cleanup: Remove Wrong Approach Code

The following crates and files implement the WRONG approach (link-time unification via
inkwell/tree-sitter or clang++) and must be removed or deprecated.

### Crates to DELETE (Wrong Approach)

These crates use tree-sitter parsing + inkwell LLVM codegen - a parallel compiler that
only meets Rust at link time. They should be REMOVED entirely:

- [ ] `crates/fragile-codegen/` - **DELETE** (inkwell LLVM codegen, wrong approach)
- [ ] `crates/fragile-frontend-cpp/` - **DELETE** (tree-sitter C++ parsing, wrong approach)
- [ ] `crates/fragile-frontend-rust/` - **DELETE** (tree-sitter Rust parsing, wrong approach)
- [ ] `crates/fragile-frontend-go/` - **DELETE** (tree-sitter Go parsing, wrong approach)
- [ ] `crates/fragile-hir/` - **DELETE** (HIR used by above, wrong approach)
- [ ] `crates/fragile-driver/` - **DELETE** (driver for wrong approach crates)

### Files to Remove in Correct Crates

In `fragile-rustc-driver` (the CORRECT crate), remove clang++ codegen remnants:

- [ ] `crates/fragile-rustc-driver/src/cpp_compiler.rs` - DELETE (clang++ wrapper)
- [ ] `crates/fragile-rustc-driver/src/stubs.rs` - REFACTOR (no extern "C" stubs)
- [ ] Remove all `compile_cpp_objects()` calls
- [ ] Remove `CppCompilerConfig`, `CppCompiler` types

### Test Files to Clean Up

- [ ] Review `tests/cpp/` - some tests were for wrong approach
- [ ] `tests/cpp/namespace.cpp` - Created for tree-sitter path, may need to be recreated for fragile-clang
- [ ] `tests/cpp/class.cpp` - Created for tree-sitter path, may need to be recreated for fragile-clang

### Cargo.toml Updates

After removing wrong crates:
- [ ] Update root `Cargo.toml` workspace members
- [ ] Remove wrong crates from any dependency lists
- [ ] Clean up feature flags referencing wrong crates

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

**Major Milestone Achieved [26:01:17]**: C++ code now compiles through rustc MIR injection!

**Proven Working**:
- `add_cpp(2, 3) = 5` - simple addition compiles via rustc
- `factorial(5) = 120` - recursion and control flow work
- No clang++ used for these test cases

**Next Steps**:
1. Add more MIR injection tests (structs, classes)
2. Implement vtable generation (Task 3.3.3) for virtual dispatch
3. Eventually compile doctest through MIR (blocked on template/STL support)

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
