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
- [-] **A.1 Namespaces**
  - [x] `namespace foo { }` declarations [26:01:15, 23:35] ([docs/dev/plan_namespace_declarations.md](docs/dev/plan_namespace_declarations.md))
  - [x] Nested namespaces `foo::bar` [26:01:15, 23:35] (included in above)
  - [x] `using namespace` [26:01:15, 23:41] ([docs/dev/plan_using_namespace.md](docs/dev/plan_using_namespace.md))
  - [ ] Name resolution
- [-] **A.2 Classes Complete**
  - [x] Field declarations
  - [x] Access specifiers (public/private/protected) [26:01:15, 23:46] ([docs/dev/plan_access_specifiers.md](docs/dev/plan_access_specifiers.md))
  - [x] Constructors (default, copy, move) [26:01:15] ([docs/dev/plan_constructors.md](docs/dev/plan_constructors.md))
  - [x] Destructors [26:01:15] (included in above)
  - [ ] Member initializer lists
  - [ ] Static members
  - [ ] Friend declarations
- [ ] **A.3 Inheritance**
  - [ ] Single inheritance
  - [ ] Multiple inheritance
  - [ ] Virtual functions + vtables
  - [ ] Pure virtual (= 0)
  - [ ] Override/final specifiers
- [ ] **A.4 Operator Overloading**
  - [ ] Arithmetic (+, -, *, /)
  - [ ] Comparison (==, !=, <, >)
  - [ ] Assignment (=, +=)
  - [ ] Subscript []
  - [ ] Call ()
  - [ ] Pointer (*, ->)
- [ ] **A.5 References & Move Semantics**
  - [x] Lvalue references (T&)
  - [ ] Const references (const T&)
  - [ ] Rvalue references (T&&)
  - [ ] std::move
  - [ ] std::forward

### 1.2 Phase B: Templates
- [ ] **B.1 Function Templates**
  - [ ] Basic templates
  - [ ] Argument deduction
  - [ ] Specialization
  - [ ] Variadic templates
- [ ] **B.2 Class Templates**
  - [ ] Basic class templates
  - [ ] Partial specialization
  - [ ] Nested templates
- [ ] **B.3 SFINAE & Type Traits**
  - [ ] std::enable_if
  - [ ] std::is_same, std::is_base_of
  - [ ] std::conditional
- [ ] **B.4 C++20 Concepts**
  - [ ] `requires` clauses
  - [ ] Concept definitions
  - [ ] Standard concepts

### 1.3 Phase C: Standard Library
- [ ] **C.1 Containers**
  - [ ] std::vector
  - [ ] std::string
  - [ ] std::map / std::unordered_map
  - [ ] std::optional
  - [ ] std::variant
- [ ] **C.2 Smart Pointers**
  - [ ] std::unique_ptr
  - [ ] std::shared_ptr
  - [ ] std::weak_ptr
- [ ] **C.3 Concurrency**
  - [ ] std::thread
  - [ ] std::mutex / std::lock_guard
  - [ ] std::condition_variable
  - [ ] std::atomic
- [ ] **C.4 Utilities**
  - [ ] std::function
  - [ ] std::chrono
  - [ ] std::move / std::forward

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
- [ ] BinaryOperator → extract actual op
- [ ] UnaryOperator → extract actual op
- [x] CallExpr → Call terminator

### 2.2 Control Flow
- [x] ReturnStmt
- [x] IfStmt
- [x] WhileStmt
- [ ] ForStmt
- [ ] BreakStmt (loop context)
- [ ] ContinueStmt (loop context)
- [ ] Switch statement

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
- [x] fragile-clang: 26 tests passing (6 unit + 20 integration)
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
