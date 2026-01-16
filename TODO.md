# Fragile Compiler - Hierarchical TODO

## Overview

Fragile is a polyglot compiler unifying Rust, C++, and Go at the rustc MIR level.

**Primary Goal**: Compile the [Mako](https://github.com/makodb/mako) distributed database (C++23)

```
Status Legend:
  [x] Completed
  [-] In Progress
  [ ] Not Started
```

---

## 1. C++20/23 Support (Target: Mako Project)

See [PLAN_CPP20_MAKO.md](PLAN_CPP20_MAKO.md) for detailed plan.

### 1.0 Infrastructure ✅
- [x] **1.0.1 Clang Integration**
  - [x] `fragile-clang` crate with libclang
  - [x] Clang AST parsing
  - [x] Basic MIR conversion
- [x] **1.0.2 rustc Driver**
  - [x] `fragile-rustc-driver` crate (stub)
  - [x] MIR registry
  - [x] Rust stub generation
- [x] **1.0.3 Runtime**
  - [x] `fragile-runtime` crate
  - [x] Exception/memory/vtable stubs

### 1.1 Phase A: Core C++ Infrastructure
- [x] **A.1 Namespaces**
  - [x] `namespace foo { }` declarations [26:01:15, 23:35] ([docs/dev/plan_namespace_declarations.md](docs/dev/plan_namespace_declarations.md))
  - [x] Nested namespaces `foo::bar` [26:01:15, 23:35] (included in above)
  - [x] `using namespace` [26:01:15, 23:41] ([docs/dev/plan_using_namespace.md](docs/dev/plan_using_namespace.md))
  - [x] Name resolution [26:01:16, 00:53] ([docs/dev/plan_namespace_name_resolution.md](docs/dev/plan_namespace_name_resolution.md))
- [x] **A.2 Classes Complete**
  - [x] Field declarations
  - [x] Access specifiers (public/private/protected) [26:01:15, 23:46] ([docs/dev/plan_access_specifiers.md](docs/dev/plan_access_specifiers.md))
  - [x] Constructors (default, copy, move) [26:01:15] ([docs/dev/plan_constructors.md](docs/dev/plan_constructors.md))
  - [x] Destructors [26:01:15] (included in above)
  - [x] Member initializer lists [26:01:16] ([docs/dev/plan_member_initializer_lists.md](docs/dev/plan_member_initializer_lists.md))
  - [x] Static members [26:01:16] ([docs/dev/plan_static_members.md](docs/dev/plan_static_members.md))
  - [x] Friend declarations [26:01:16] ([docs/dev/plan_friend_declarations.md](docs/dev/plan_friend_declarations.md))
- [x] **A.3 Inheritance**
  - [x] Single inheritance [26:01:16] ([docs/dev/plan_single_inheritance.md](docs/dev/plan_single_inheritance.md))
  - [x] Multiple inheritance [26:01:16] (uses same infrastructure as single inheritance)
  - [x] Virtual functions + vtables [26:01:16] ([docs/dev/plan_virtual_functions.md](docs/dev/plan_virtual_functions.md))
  - [x] Pure virtual (= 0) [26:01:16] (included in above)
  - [x] Override/final specifiers [26:01:16] ([docs/dev/plan_override_final.md](docs/dev/plan_override_final.md))
- [x] **A.4 Operator Overloading**
  - [x] Arithmetic (+, -, *, /) [26:01:16] ([docs/dev/plan_operator_overloading.md](docs/dev/plan_operator_overloading.md))
  - [x] Comparison (==, !=, <, >) [26:01:16] (included in above)
  - [x] Assignment (=, +=) [26:01:16] (included in above)
  - [x] Subscript [] [26:01:16] (included in above)
  - [x] Call () [26:01:16] (included in above)
  - [x] Pointer (*, ->) [26:01:16] (included in above)
- [x] **A.5 References & Move Semantics**
  - [x] Lvalue references (T&)
  - [x] Const references (const T&) [26:01:16] ([docs/dev/plan_references.md](docs/dev/plan_references.md))
  - [x] Rvalue references (T&&) [26:01:16] (included in above)
  - [x] std::move [26:01:16, 00:58] ([docs/dev/plan_std_move.md](docs/dev/plan_std_move.md))
  - [x] std::forward [26:01:16, 00:58] (included in above)

### 1.2 Phase B: Templates ✅
- [x] **B.1 Function Templates**
  - [x] Basic templates [26:01:16] ([docs/dev/plan_function_templates.md](docs/dev/plan_function_templates.md))
  - [x] Argument deduction
    - [x] Dependent type representation (CppType extensions) [26:01:16, 01:18] ([docs/dev/plan_dependent_types.md](docs/dev/plan_dependent_types.md))
    - [x] Basic deduction for simple types (T → int, T → double) [26:01:16, 01:26] ([docs/dev/plan_basic_type_deduction.md](docs/dev/plan_basic_type_deduction.md))
    - [x] Deduction for pointers/references (T* → int*) [26:01:16, 01:32]
    - [x] Explicit template arguments override [26:01:16, 01:40] ([docs/dev/plan_explicit_template_args.md](docs/dev/plan_explicit_template_args.md))
  - [x] Specialization [26:01:16, 02:03] ([docs/dev/plan_template_specialization.md](docs/dev/plan_template_specialization.md))
  - [x] Variadic templates [26:01:16, 03:02] ([docs/dev/plan_variadic_templates.md](docs/dev/plan_variadic_templates.md))
- [x] **B.2 Class Templates**
  - [x] Basic class templates [26:01:16, 03:45] ([docs/dev/plan_class_templates.md](docs/dev/plan_class_templates.md))
  - [x] Partial specialization [26:01:16, 04:30] ([docs/dev/plan_partial_specialization.md](docs/dev/plan_partial_specialization.md))
  - [x] Nested templates (member templates) [26:01:16, 05:00] ([docs/dev/plan_nested_templates.md](docs/dev/plan_nested_templates.md))
- [x] **B.3 SFINAE & Type Traits**
  - [x] TypeProperties foundation [26:01:16, 05:30] ([docs/dev/plan_sfinae.md](docs/dev/plan_sfinae.md))
  - [x] TypeTraitExpr AST node [26:01:16, 06:15]
  - [x] TypeTraitEvaluator (is_integral, is_same, etc.) [26:01:16, 06:15]
  - Note: std::enable_if, std::is_base_of (class hierarchy), std::conditional deferred to Phase C
- [x] **B.4 C++20 Concepts** ([docs/dev/plan_cpp20_concepts.md](docs/dev/plan_cpp20_concepts.md))
  - [x] B.4.1 AST representation (ConceptDecl, RequiresExpr, RequiresClause nodes) [26:01:16, 02:40]
  - [x] B.4.2 Parser support (handle concept cursors, requires clauses) [26:01:16, 02:40]
  - [x] B.4.3 Concept definitions (`concept Integral = ...`) [26:01:16, 02:40]
  - [x] B.4.4 Requires clauses on functions/templates (`requires Integral<T>`) [26:01:16, 02:40]
  - [x] B.4.5 Requires expressions (`requires { expr; }`) [26:01:16, 02:50]
  - Note: B.4.6 Standard concepts (std::integral, std::same_as) deferred to Phase C (Standard Library)

### 1.3 Phase C: Standard Library

#### C.0 Infrastructure (prerequisite)
- [x] **C.0.1 Header search path support** - ClangParser include paths for STL headers [26:01:16, 03:05]
- [ ] **C.0.2 Type alias support** - Parse and track `using` type aliases (e.g., `std::vector<T>::iterator`)

#### C.1 Containers
- [ ] **C.1.1 std::vector (basic)**
  - [ ] Parse vector template from `<vector>` header
  - [ ] Support push_back, pop_back, size, operator[]
  - [ ] Support begin(), end() iterators
- [ ] **C.1.2 std::string**
  - [ ] Parse string from `<string>` header
  - [ ] Basic operations (c_str(), size(), operator[])
- [ ] **C.1.3 Other containers** (deferred)
  - [ ] std::map / std::unordered_map
  - [ ] std::optional, std::variant

#### C.2 Smart Pointers
- [ ] std::unique_ptr
- [ ] std::shared_ptr
- [ ] std::weak_ptr

#### C.3 Concurrency
- [ ] std::thread
- [ ] std::mutex / std::lock_guard
- [ ] std::condition_variable
- [ ] std::atomic

#### C.4 Utilities
- [ ] std::function
- [ ] std::chrono
- [ ] std::move / std::forward (✅ basic support done in Phase A)

### 1.4 Phase D: C++20 Coroutines
- [ ] **D.1 Infrastructure**
  - [ ] `<coroutine>` header
  - [ ] std::coroutine_handle
  - [ ] std::suspend_always/never
- [ ] **D.2 Promise Types**
  - [ ] get_return_object()
  - [ ] initial_suspend / final_suspend
  - [ ] return_void / return_value
- [ ] **D.3 Awaitables**
  - [ ] await_ready/suspend/resume
  - [ ] co_await expression
- [ ] **D.4 Generators**
  - [ ] co_yield
  - [ ] Generator pattern

### 1.5 Phase E: Advanced Features
- [ ] **E.1 Exceptions**
  - [ ] try/catch/throw
  - [ ] noexcept
  - [ ] Stack unwinding
- [ ] **E.2 RTTI**
  - [ ] typeid
  - [ ] dynamic_cast
- [ ] **E.3 Lambdas**
  - [ ] Basic lambdas
  - [ ] Captures (value/reference)
  - [ ] Generic lambdas
- [ ] **E.4 Attributes**
  - [ ] [[nodiscard]]
  - [ ] [[maybe_unused]]

### 1.6 Phase F: Mako Integration
- [ ] **F.1 Build Individual Files**
  - [ ] `vendor/mako/src/rrr/misc/rand.cpp`
  - [ ] `vendor/mako/src/rrr/misc/marshal.cpp`
  - [ ] `vendor/mako/src/rrr/rpc/server.cpp`
- [ ] **F.2 Coroutine Files**
  - [ ] `vendor/mako/src/mako/vec/coroutine.cpp`
  - [ ] `vendor/mako/src/mako/vec/occ.cpp`
- [ ] **F.3 Full Build**
  - [ ] All rrr module
  - [ ] All mako module
  - [ ] Link and run tests

---

## 2. Clang AST → MIR (Supporting Infrastructure)

### 2.1 Basic Expressions
- [x] IntegerLiteral → Constant
- [x] FloatingLiteral → Constant
- [x] BoolLiteral → Constant
- [x] DeclRefExpr → Operand
- [x] BinaryOperator → extract actual op [26:01:16, 07:30]
- [x] UnaryOperator → extract actual op [26:01:16, 07:30]
- [x] CallExpr → Call terminator

### 2.2 Control Flow
- [x] ReturnStmt
- [x] IfStmt
- [x] WhileStmt
- [x] ForStmt [26:01:16, 06:45]
- [x] Switch statement [26:01:16, 07:00]
- [x] BreakStmt (loop context) [26:01:16, 07:15]
- [x] ContinueStmt (loop context) [26:01:16, 07:15]

### 2.3 rustc Integration
- [ ] Nightly + rustc-dev setup
- [ ] Callbacks trait
- [ ] mir_built query override
- [ ] mir_borrowck bypass for C++

---

## 3. Go Support (Deferred)

### 3.1 Go Parsing
- [ ] `fragile-go` crate
- [ ] Go SSA → MIR

### 3.2 Conservative GC
- [ ] `fragile-gc` crate
- [ ] Block-based heap
- [ ] Mark/sweep

### 3.3 Go Runtime
- [ ] Goroutines
- [ ] Channels
- [ ] Defer/panic/recover

---

## 4. Legacy Architecture (To Deprecate)

- `fragile-frontend-rust` - Tree-sitter Rust → HIR
- `fragile-frontend-cpp` - Tree-sitter C++ → HIR
- `fragile-frontend-go` - Tree-sitter Go → HIR
- `fragile-hir` - Custom HIR
- `fragile-codegen` - HIR → LLVM IR

Migration: After C++20 support is complete, deprecate these.

---

## 5. Testing & Milestones

### 5.1 Unit Tests
- [x] fragile-clang: 152 tests passing (27 unit + 125 integration)
- [x] fragile-rustc-driver: 6 tests passing
- [x] fragile-runtime: Compiles

### 5.2 Mako Milestones
- [ ] **M1**: Compile `rand.cpp` (minimal deps)
- [ ] **M2**: Compile `rrr/misc/*.cpp` (templates, STL)
- [ ] **M3**: Compile `rrr/rpc/*.cpp` (OOP, threads)
- [ ] **M4**: Compile `mako/vec/*.cpp` (coroutines)
- [ ] **M5**: Full Mako build
- [ ] **M6**: Mako tests pass

---

## Current Focus

**Primary: C++20/23 for Mako**

Next steps:
1. **A.1 Namespaces** - Mako uses `namespace rrr`, `namespace mako`
2. **A.2 Classes** - Complete class support with constructors/destructors
3. **A.3 Inheritance** - Virtual functions for polymorphism

Start with: Try to compile `vendor/mako/src/rrr/misc/rand.cpp` as first target.
