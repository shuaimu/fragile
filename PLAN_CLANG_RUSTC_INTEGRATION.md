# Plan: Clang + rustc Integration

## Vision

Reuse existing compiler frontends (Clang for C++, rustc for Rust) and unify at the MIR level, leveraging rustc's borrow checker and codegen.

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   C++ Source              Rust Source                           │
│       │                       │                                 │
│       ▼                       ▼                                 │
│   ┌───────────┐          ┌───────────┐                          │
│   │   Clang   │          │   rustc   │                          │
│   │  Frontend │          │  Frontend │                          │
│   └─────┬─────┘          └─────┬─────┘                          │
│         │                      │                                │
│         ▼                      ▼                                │
│   ┌───────────┐          ┌───────────┐                          │
│   │ Clang AST │          │ rustc HIR │                          │
│   └─────┬─────┘          └─────┬─────┘                          │
│         │                      │                                │
│         │    ┌─────────────┐   │                                │
│         └───►│ Fragile MIR │◄──┘                                │
│              │  Converter  │                                    │
│              └──────┬──────┘                                    │
│                     │                                           │
│                     ▼                                           │
│              ┌─────────────┐                                    │
│              │  rustc MIR  │  ← Unified representation          │
│              └──────┬──────┘                                    │
│                     │                                           │
│                     ▼                                           │
│              ┌─────────────┐                                    │
│              │   rustc     │  ← Query override:                 │
│              │  Pipeline   │    - Skip borrow check for C++     │
│              │             │    - Normal codegen                │
│              └──────┬──────┘                                    │
│                     │                                           │
│                     ▼                                           │
│              ┌─────────────┐                                    │
│              │   Binary    │                                    │
│              └─────────────┘                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Current State

- `fragile-frontend-rust`: tree-sitter based Rust parser → Fragile HIR
- `fragile-frontend-cpp`: tree-sitter based C++ parser → Fragile HIR
- `fragile-hir`: Custom HIR representation
- `fragile-codegen`: HIR → LLVM IR codegen

## New Architecture

### Crates to Add

| Crate | Purpose |
|-------|---------|
| `fragile-clang` | Clang AST → rustc MIR conversion |
| `fragile-rustc-driver` | Custom rustc driver with query overrides |
| `fragile-runtime` | Runtime library for C++ features |

### Crates to Deprecate

| Crate | Reason |
|-------|--------|
| `fragile-frontend-rust` | Use rustc directly |
| `fragile-frontend-cpp` | Use Clang instead of tree-sitter |
| `fragile-hir` | Use rustc MIR directly |
| `fragile-codegen` | Use rustc codegen |

### Crates to Keep

| Crate | Purpose |
|-------|---------|
| `fragile-common` | Shared utilities |
| `fragile-cli` | Command-line interface (modified) |

## Dependencies

### Rust (Nightly Required)

```toml
[dependencies]
# rustc internals
rustc_driver = { version = "0.0.0" }
rustc_interface = { version = "0.0.0" }
rustc_middle = { version = "0.0.0" }
rustc_mir_build = { version = "0.0.0" }
rustc_hir = { version = "0.0.0" }
rustc_span = { version = "0.0.0" }
rustc_borrowck = { version = "0.0.0" }
```

### Clang

```toml
[dependencies]
# Clang bindings
clang = "2.0"        # libclang bindings
# OR
clang-sys = "1.0"    # Lower-level libclang
```

### Build Requirements

```bash
# Nightly Rust with rustc-dev component
rustup toolchain install nightly
rustup component add rustc-dev llvm-tools-preview --toolchain nightly

# Clang/LLVM development libraries
# Ubuntu/Debian:
sudo apt install libclang-dev llvm-dev

# macOS:
brew install llvm
```

## Implementation Phases

### Phase 1: Setup and Proof of Concept (2 iterations)

#### 1.1: Project Setup

```
fragile/
├── crates/
│   ├── fragile-clang/           # NEW
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parse.rs         # Clang AST parsing
│   │       └── convert.rs       # AST → MIR stubs
│   │
│   ├── fragile-rustc-driver/    # NEW
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── driver.rs        # Custom rustc driver
│   │       └── queries.rs       # Query overrides
│   │
│   └── fragile-runtime/         # NEW
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── exceptions.rs    # C++ exception support
│           └── memory.rs        # new/delete support
```

#### 1.2: Minimal End-to-End

Goal: Compile simplest C++ function through rustc.

```cpp
// test.cpp
int add(int a, int b) {
    return a + b;
}
```

1. Parse with Clang → Clang AST
2. Convert to rustc MIR (manually constructed)
3. Inject via query override
4. rustc codegen → working binary

### Phase 2: Clang AST → MIR Converter (5 iterations)

#### 2.1: Basic Expressions

| Clang AST | rustc MIR |
|-----------|-----------|
| `IntegerLiteral` | `Constant` |
| `BinaryOperator` | `Rvalue::BinaryOp` |
| `UnaryOperator` | `Rvalue::UnaryOp` |
| `DeclRefExpr` | `Operand::Copy/Move(Place)` |
| `CallExpr` | `TerminatorKind::Call` |

#### 2.2: Control Flow

| Clang AST | rustc MIR |
|-----------|-----------|
| `IfStmt` | `SwitchInt` + basic blocks |
| `WhileStmt` | Loop with `SwitchInt` |
| `ForStmt` | Loop with `SwitchInt` |
| `ReturnStmt` | `TerminatorKind::Return` |
| `BreakStmt` | `Goto` to loop exit |
| `ContinueStmt` | `Goto` to loop header |

#### 2.3: Types

| C++ Type | rustc Type |
|----------|-----------|
| `int` | `i32` |
| `long` | `i64` |
| `float` | `f32` |
| `double` | `f64` |
| `bool` | `bool` |
| `T*` | `*mut T` / `*const T` |
| `T&` | `*mut T` (raw pointer) |
| `struct` | `rustc Adt` |
| `class` | `rustc Adt` |

#### 2.4: Functions and Calling

- Function declarations → `extern "C"` items
- Function definitions → MIR bodies
- Method calls → explicit `self` parameter
- Overloaded functions → mangled names

#### 2.5: Classes and Structs

- Fields → struct fields
- Methods → associated functions with `self: *mut Self`
- Constructors → `fn new() -> Self`
- Destructors → `Drop` impl (via runtime)
- Virtual functions → vtable + runtime dispatch

### Phase 3: rustc Integration (3 iterations)

#### 3.1: Custom Driver

```rust
// fragile-rustc-driver/src/driver.rs

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;

use rustc_driver::Callbacks;
use rustc_interface::Queries;

pub struct FragileCallbacks {
    cpp_mir_bodies: HashMap<DefId, Body<'static>>,
}

impl Callbacks for FragileCallbacks {
    fn config(&mut self, config: &mut Config) {
        // Override queries
        config.override_queries = Some(|_session, providers| {
            providers.mir_built = custom_mir_built;
            providers.mir_borrowck = custom_borrowck;
        });
    }
}

fn custom_mir_built(tcx: TyCtxt<'_>, def_id: LocalDefId) -> &Body<'_> {
    // If this is a C++ function, return our custom MIR
    if is_cpp_function(tcx, def_id) {
        return get_cpp_mir(def_id);
    }
    // Otherwise, normal MIR building
    (DEFAULT_MIR_BUILT)(tcx, def_id)
}

fn custom_borrowck(tcx: TyCtxt<'_>, def_id: LocalDefId) -> &BorrowCheckResult<'_> {
    // Skip borrow checking for C++ functions
    if is_cpp_function(tcx, def_id) {
        return empty_borrowck_result();
    }
    // Normal borrow checking for Rust
    (DEFAULT_BORROWCK)(tcx, def_id)
}
```

#### 3.2: Item Injection

Generate Rust stub file for C++ declarations:

```rust
// Generated: cpp_stubs.rs

#[fragile::cpp_impl]
extern "C" {
    fn add(a: i32, b: i32) -> i32;

    fn Point_new(x: i32, y: i32) -> *mut Point;
    fn Point_getX(self_: *const Point) -> i32;
    fn Point_drop(self_: *mut Point);
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
```

#### 3.3: Cross-Language Calls

Rust calling C++:
```rust
fn rust_main() {
    unsafe {
        let result = add(1, 2);  // Calls C++ add()

        let p = Point_new(10, 20);
        let x = Point_getX(p);
        Point_drop(p);
    }
}
```

C++ calling Rust:
```cpp
// Rust function exposed
extern "C" int rust_add(int a, int b);

int cpp_caller() {
    return rust_add(1, 2);  // Calls Rust function
}
```

### Phase 4: C++ Feature Support (5 iterations)

#### 4.1: Exceptions

```cpp
void might_throw() {
    throw std::runtime_error("oops");
}

void caller() {
    try {
        might_throw();
    } catch (const std::exception& e) {
        handle(e);
    }
}
```

Lowered to MIR + runtime:

```
bb0: {
    call fragile_rt::try_begin() -> bb1;
}

bb1: {
    call might_throw() -> [return: bb2, unwind: bb3];
}

bb2: {
    call fragile_rt::try_end() -> bb4;
}

bb3 (cleanup): {
    _ex = call fragile_rt::catch_exception();
    call handle(_ex) -> bb4;
}

bb4: {
    return;
}
```

#### 4.2: Virtual Functions

```cpp
class Animal {
public:
    virtual void speak() = 0;
};

class Dog : public Animal {
public:
    void speak() override { bark(); }
};
```

Lowered to:

```
// Vtable structure
struct Animal_vtable {
    speak: fn(*mut Animal),
}

// Virtual call
bb0: {
    _vtable = load (*self).vtable;
    _speak_fn = load _vtable.speak;
    call _speak_fn(self) -> bb1;
}
```

#### 4.3: RAII / Destructors

```cpp
class Resource {
    int* data;
public:
    Resource() : data(new int[100]) {}
    ~Resource() { delete[] data; }
};

void use_resource() {
    Resource r;
    // ... use r ...
}  // r.~Resource() called here
```

Lowered to:

```
bb0: {
    _r = call Resource_new();
    goto bb1;
}

bb1: {
    // ... use _r ...
    goto bb2;
}

bb2: {
    call Resource_drop(&mut _r);
    return;
}
```

#### 4.4: new / delete

```cpp
int* p = new int(42);
delete p;

int* arr = new int[100];
delete[] arr;
```

Lowered to:

```
bb0: {
    _p = call fragile_rt::cpp_new::<i32>(42);
    call fragile_rt::cpp_delete(_p);

    _arr = call fragile_rt::cpp_new_array::<i32>(100);
    call fragile_rt::cpp_delete_array(_arr);
}
```

#### 4.5: Inheritance

```cpp
class Base {
    int x;
};

class Derived : public Base {
    int y;
};
```

Lowered to:

```rust
#[repr(C)]
struct Base {
    x: i32,
}

#[repr(C)]
struct Derived {
    _base: Base,  // Base class embedded
    y: i32,
}

// Upcasting: just pointer cast
fn upcast(d: *mut Derived) -> *mut Base {
    d as *mut Base
}
```

### Phase 5: Runtime Library (2 iterations)

#### 5.1: Core Runtime

```rust
// fragile-runtime/src/lib.rs

#![no_std]

// Exception handling (platform-specific)
#[cfg(unix)]
mod exceptions_unix;
#[cfg(windows)]
mod exceptions_windows;

pub use exceptions::*;

// Memory management
mod memory;
pub use memory::*;

// Virtual dispatch helpers
mod vtable;
pub use vtable::*;
```

#### 5.2: Exception Implementation

```rust
// fragile-runtime/src/exceptions.rs

use core::ffi::c_void;

#[repr(C)]
pub struct CppException {
    type_info: *const c_void,
    data: *mut c_void,
}

#[no_mangle]
pub extern "C" fn fragile_rt_throw(exception: *mut CppException) -> ! {
    #[cfg(unix)]
    unsafe {
        extern "C" {
            fn _Unwind_RaiseException(exc: *mut c_void) -> !;
        }
        _Unwind_RaiseException(exception as *mut c_void)
    }
}

#[no_mangle]
pub extern "C" fn fragile_rt_try_begin() {
    // Setup exception handling frame
    // Platform-specific implementation
}

#[no_mangle]
pub extern "C" fn fragile_rt_catch() -> *mut CppException {
    // Get current exception
    // Platform-specific implementation
}
```

### Phase 6: CLI and Integration (2 iterations)

#### 6.1: New CLI

```bash
# Compile mixed project
fragile build main.rs utils.cpp helper.cpp

# The flow:
# 1. Parse *.cpp with Clang → Clang AST
# 2. Convert Clang AST → MIR bodies (stored)
# 3. Generate cpp_stubs.rs with extern declarations
# 4. Run rustc on *.rs + cpp_stubs.rs with custom driver
# 5. Query override injects C++ MIR bodies
# 6. rustc codegen produces binary
```

#### 6.2: Build System Integration

```toml
# Cargo.toml
[package]
name = "my_mixed_project"

[fragile]
cpp_sources = ["src/utils.cpp", "src/helper.cpp"]
cpp_include_dirs = ["include/"]
cpp_defines = ["DEBUG=1"]
```

## Migration Path

### Step 1: Keep Old System Working

Don't delete existing crates yet. Run in parallel.

### Step 2: Implement New System

Build new crates alongside old ones.

### Step 3: Feature Parity

Match existing test cases with new system.

### Step 4: Deprecate Old

Mark old crates as deprecated, remove after validation.

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_clang_ast_to_mir_literal() {
    let clang_ast = parse_cpp("42");
    let mir = convert_to_mir(clang_ast);
    assert_eq!(mir, Constant(42));
}
```

### Integration Tests

```rust
#[test]
fn test_cpp_function_callable_from_rust() {
    // Compile test.cpp with add function
    // Compile test.rs that calls add
    // Run and check result
}
```

### End-to-End Tests

```bash
# tests/e2e/
tests/e2e/
├── simple_add/
│   ├── add.cpp
│   ├── main.rs
│   └── expected_output.txt
├── exceptions/
│   ├── throw_catch.cpp
│   ├── main.rs
│   └── expected_output.txt
└── virtual_dispatch/
    ├── animals.cpp
    ├── main.rs
    └── expected_output.txt
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| rustc internal APIs unstable | Pin to specific nightly, document version |
| Clang API changes | Use stable libclang C API, not C++ |
| Exception ABI differences | Test on all platforms, use LLVM's EH |
| Performance overhead | Benchmark, optimize runtime calls |
| Complexity | Start with subset, iterate |

## Success Criteria

1. **Minimal**: Compile simple C++ function, call from Rust
2. **Basic**: Structs, methods, control flow work
3. **Intermediate**: Exceptions, virtual functions work
4. **Full**: Run existing C++ test suite through Fragile

## Timeline Estimate

| Phase | Iterations | Description |
|-------|------------|-------------|
| 1 | 2 | Setup + proof of concept |
| 2 | 5 | Clang AST → MIR converter |
| 3 | 3 | rustc integration |
| 4 | 5 | C++ feature support |
| 5 | 2 | Runtime library |
| 6 | 2 | CLI and integration |
| **Total** | **19** | |

## Next Steps

1. Set up nightly Rust with rustc-dev components
2. Create `fragile-clang` crate with libclang dependency
3. Create `fragile-rustc-driver` crate with rustc internals
4. Implement Phase 1.2: Minimal end-to-end for `int add(int, int)`
