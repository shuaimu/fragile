# Plan: Section 2.3 rustc Integration

## Overview

Integrate Fragile's C++ MIR with rustc's compilation pipeline by implementing custom query overrides.

## Status: TASKS 2.3.1 AND 2.3.2 COMPLETE

### Completed Setup
1. **Nightly Rust toolchain**: `rustup toolchain install nightly` ✅
2. **rustc-dev component**: `rustup component add rustc-dev llvm-tools-preview --toolchain nightly` ✅
3. **Building**: `cargo +nightly build --features rustc-integration` ✅

### Files Modified/Created
- `crates/fragile-rustc-driver/Cargo.toml` - Added `tempfile` dep, updated comments
- `crates/fragile-rustc-driver/build.rs` - Created: finds rustc sysroot and sets link paths
- `crates/fragile-rustc-driver/src/lib.rs` - Added `#![feature(rustc_private)]` for feature gate
- `crates/fragile-rustc-driver/src/rustc_integration.rs` - Created: `FragileCallbacks` implementation

## Architecture

### Current Implementation (fragile-rustc-driver)

The crate has 4 modules:
- **queries.rs**: `CppMirRegistry` stores C++ functions and their MIR bodies
- **stubs.rs**: Generates Rust `extern "C"` stubs for C++ functions
- **driver.rs**: `FragileDriver` that delegates to rustc_integration when feature enabled
- **rustc_integration.rs** (new): `FragileCallbacks` implementing `rustc_driver::Callbacks`

### Target Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ C++ Source  │────▶│  fragile-clang   │────▶│  CppMirRegistry │
└─────────────┘     │  (AST → MIR)     │     │  (stores MIR)   │
                    └──────────────────┘     └────────┬────────┘
                                                      │
┌─────────────┐     ┌──────────────────┐              │
│ Rust Source │────▶│  FragileDriver   │◀─────────────┘
└─────────────┘     │  (rustc wrapper) │
        │           └────────┬─────────┘
        │                    │
        ▼                    ▼
┌───────────────────────────────────────────────────────────┐
│                    rustc (with overrides)                  │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  Query: mir_built                                    │  │
│  │  - If DefId is C++: return pre-computed MIR         │  │
│  │  - If DefId is Rust: use normal rustc pipeline      │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────┐
│   Binary    │
└─────────────┘
```

## Implementation Tasks

### Task 2.3.1: Nightly + rustc-dev Setup ✅ DONE
**Effort**: Environment setup + build infrastructure

1. Install nightly: `rustup toolchain install nightly` ✅
2. Add components: `rustup component add rustc-dev llvm-tools-preview --toolchain nightly` ✅
3. Created `build.rs` to find rustc sysroot and set library paths ✅
4. Added `#![feature(rustc_private)]` to lib.rs (conditionally) ✅
5. Added `tempfile` dependency for temporary stub file handling ✅

### Task 2.3.2: Callbacks Trait Implementation ✅ DONE
**File**: `crates/fragile-rustc-driver/src/rustc_integration.rs`

Created `FragileCallbacks` struct with full `rustc_driver::Callbacks` implementation:

```rust
// Actual implementation (from rustc_integration.rs)
extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

pub struct FragileCallbacks {
    pub mir_registry: Arc<CppMirRegistry>,
    pub stubs_path: Option<PathBuf>,
    pub cpp_function_names: Vec<String>,
}

impl rustc_driver::Callbacks for FragileCallbacks {
    fn config(&mut self, config: &mut Config) {
        // Set up override_queries for MIR injection (infrastructure ready)
        config.override_queries = Some(|_session, providers| {
            // Query override logic goes here
        });
    }

    fn after_crate_root_parsing(&mut self, ...) -> Compilation { ... }
    fn after_expansion<'tcx>(&mut self, ...) -> Compilation { ... }
    fn after_analysis<'tcx>(&mut self, ...) -> Compilation { ... }
}
```

Also created `run_rustc()` function to:
1. Create temporary file for C++ stubs
2. Build rustc argument list
3. Call `rustc_driver::run_compiler()` with our callbacks

### Task 2.3.3: mir_built Query Override (~80 LOC)
**File**: `crates/fragile-rustc-driver/src/driver.rs`

```rust
config.override_queries = Some(|_session, providers| {
    let orig_mir_built = providers.mir_built;
    providers.mir_built = |tcx, def_id| {
        // Check if this DefId corresponds to a C++ function
        if is_cpp_function(tcx, def_id) {
            // Return pre-computed MIR from registry
            get_cpp_mir(tcx, def_id)
        } else {
            // Fall back to normal rustc pipeline
            orig_mir_built(tcx, def_id)
        }
    };
});
```

### Task 2.3.4: mir_borrowck Bypass (~50 LOC)
**File**: `crates/fragile-rustc-driver/src/driver.rs`

```rust
providers.mir_borrowck = |tcx, def_id| {
    if is_cpp_function(tcx, def_id) {
        // Skip borrow checking for C++ code
        // Return "unsafe" borrowck result
        tcx.arena.alloc(BorrowCheckResult::default())
    } else {
        orig_mir_borrowck(tcx, def_id)
    }
};
```

## Key Challenges

### 1. DefId Mapping
Need to map between:
- rustc's `DefId` (unique identifier for items)
- Our function names/mangled names in `CppMirRegistry`

**Solution**: Use the stub function names. The stubs have `#[link_name = "mangled"]` attributes, and we can use rustc's `Symbol` interning to look up matching entries.

### 2. MIR Format Compatibility
Our `MirBody` format must match rustc's internal representation.

**Current MirBody fields:**
- `blocks: Vec<MirBasicBlock>` with statements and terminators
- `locals: Vec<MirLocal>` for variable declarations
- `is_coroutine: bool`

**rustc's MIR structure:**
- Similar but uses interned types (`Ty`, `Place`, `Operand`)
- Has `SourceInfo` for debugging
- Uses indexed types (`BasicBlock`, `Local`)

**Solution**: Create conversion functions in `queries.rs` that transform our simplified MIR to rustc's format.

### 3. Type Representation
C++ types must map to rustc's `Ty` type.

**Already implemented**: `CppType::to_rust_type_str()` generates Rust type strings.

**Needed**: Function to convert to rustc's `Ty` using `TyCtxt`.

## Cargo.toml Updates

```toml
[features]
default = []
rustc-integration = []

[dependencies]
# Conditional rustc crate dependencies
rustc_driver = { version = "*", optional = true }
rustc_interface = { version = "*", optional = true }
rustc_middle = { version = "*", optional = true }
rustc_span = { version = "*", optional = true }
```

Note: rustc crates don't use semantic versioning - must match the toolchain version.

## Testing Strategy

### Without rustc-integration (current)
- Unit tests for `CppMirRegistry` operations
- Unit tests for stub generation
- Integration tests verify pipeline without actual compilation

### With rustc-integration
- End-to-end test: C++ add function → Rust caller → binary execution
- Test MIR injection for various function signatures
- Test struct layout compatibility

## Estimated Effort

| Task | LOC | Complexity |
|------|-----|------------|
| 2.3.1 Nightly setup | 0 | Low (env setup) |
| 2.3.2 Callbacks trait | ~100 | Medium |
| 2.3.3 mir_built override | ~80 | High |
| 2.3.4 mir_borrowck bypass | ~50 | Medium |
| MIR format conversion | ~150 | High |
| **Total** | ~380 | High |

## Dependencies

This task depends on:
- Nightly Rust with rustc-dev (user must install)
- Understanding of rustc internals (TyCtxt, Providers, Queries)

This task blocks:
- Actual mixed Rust/C++ compilation
- End-to-end Mako integration tests

## References

- [rustc-dev-guide: MIR](https://rustc-dev-guide.rust-lang.org/mir/index.html)
- [rustc_interface Callbacks](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Callbacks.html)
- [rustc_middle::mir](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/index.html)
