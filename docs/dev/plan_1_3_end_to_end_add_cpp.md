# Plan: Task 1.3 - End-to-End Test: add.cpp via MIR Pipeline

## Objective

Make `add.cpp` compile to an object file via rustc's codegen (NOT clang++).

```cpp
// tests/cpp/add.cpp (or tests/cpp/hello.cpp which already has this)
int add(int a, int b) {
    return a + b;
}
```

## Current State

The current pipeline uses clang++ for C++ codegen:
1. Parse C++ with libclang → Clang AST
2. Convert Clang AST → Fragile MIR
3. Generate Rust stubs (extern "C" declarations)
4. Compile C++ with **clang++** → object file
5. Link object files with rustc

## Target State

New pipeline without clang++:
1. Parse C++ with libclang → Clang AST
2. Convert Clang AST → Fragile MIR
3. Generate **regular Rust functions** (NOT extern "C") with stub bodies
4. Register MIR bodies in `CppMirRegistry`
5. Run rustc with `mir_built` query override
6. Override returns injected C++ MIR instead of stub body
7. rustc generates object file for C++ functions via injected MIR

## Critical Issue: extern "C" functions don't have MIR

The current approach generates `extern "C"` blocks:
```rust
extern "C" {
    #[link_name = "_Z7add_cppii"]
    fn add_cpp(a: i32, b: i32) -> i32;
}
```

**Problem**: `extern "C"` functions are ForeignItems in rustc. ForeignItems
don't have MIR bodies - they're resolved by the linker. The `mir_built` query
is never called for them.

**Solution**: Generate regular Rust functions with stub bodies:
```rust
// #[fragile_cpp(mangled = "_Z7add_cppii")]
#[no_mangle]
pub fn add_cpp(a: i32, b: i32) -> i32 {
    unreachable!("Stub - MIR will be injected")
}
```

When `mir_built` is called for `add_cpp`, we intercept it and return the
converted C++ MIR instead of the stub body.

**Note**: We use `#[no_mangle]` to export with the original name, and could
use `#[export_name]` for the mangled C++ name if needed.

## Key Components

### Already Implemented
- `fragile_clang::ClangParser` - Parses C++ to AST
- `fragile_clang::MirConverter` - Converts AST to MIR
- `fragile_rustc_driver::CppMirRegistry` - Stores MIR bodies
- `fragile_rustc_driver::stubs::generate_rust_stubs()` - Generates extern stubs
- `fragile_rustc_driver::rustc_integration::fragile_mir_built()` - Query override

### Needs Implementation

1. **Function call resolution in MIR conversion** (Task 1.2.5)
   - Current: `mir_convert.rs` line 537 uses placeholder `Local::from_u32(0)`
   - Needed: Resolve function name to actual rustc function DefId

2. **Test the full pipeline** without clang++
   - Create a test that:
     a. Parses add.cpp
     b. Converts to MIR
     c. Registers in CppMirRegistry
     d. Generates stubs
     e. Runs rustc with callbacks
     f. Verifies object file was created
     g. Links and runs: `add(2, 3) == 5`

## Sub-tasks

### 1.3.1 Parse add.cpp → Clang AST
- Status: **DONE** - `ClangParser::parse_file()` works
- Test: `test_parse_add_function` in integration_test.rs

### 1.3.2 Convert Clang AST → Fragile MIR
- Status: **DONE** - `MirConverter::convert()` works
- Test: `test_convert_add_function` in integration_test.rs

### 1.3.3 Generate Rust wrapper that calls C++ via MIR injection
- Status: **DONE** [2026-01-17]
- Modified `stubs.rs` to generate regular Rust functions with stub bodies
- Functions use `#[export_name = "mangled"]` instead of extern "C"
- Stub body: `unreachable!("Fragile: C++ MIR should be injected")`
- Added `generate_rust_stubs_extern()` for backwards compatibility
- Updated `rustc_integration.rs` to detect regular functions (not just ForeignItems)

### 1.3.4 Compile via rustc (mir_built override active)
- Status: **NEEDS TESTING**
- `FragileCallbacks` implements `rustc_driver::Callbacks`
- Query override is installed in `config()`
- `fragile_mir_built()` converts and returns MIR
- Question: Does rustc accept the converted MIR?

### 1.3.5 Link and run: `add(2, 3) == 5`
- Status: **NOT STARTED**
- Requires 1.3.4 to work first

## Implementation Plan

### Phase 1: Verify MIR injection works (without function calls)

Create a simpler test case:
```cpp
int get_five() {
    return 5;
}
```

This function has no calls, just a return. If MIR injection works for this:
- The `mir_built` override is called
- MIR is converted without needing function call resolution
- rustc accepts and compiles the MIR

### Phase 2: Add function call support

For the `add` function:
```cpp
int add(int a, int b) {
    return a + b;
}
```

The MIR body will have:
- Local 0: return value
- Local 1: param a
- Local 2: param b
- Block 0:
  - _0 = Add(_1, _2)
  - return

This has no function calls, just a binary operation!

### Phase 3: Test with actual function calls

```cpp
int double_it(int x) {
    return add(x, x);  // This needs function call resolution
}
```

## Testing Strategy

1. Create `tests/clang_integration/test_mir_pipeline.rs`
2. Use `#[cfg(feature = "rustc-integration")]` for tests requiring nightly
3. Test MIR conversion independently of rustc (unit tests)
4. Test full pipeline with rustc (integration tests)

## Risk Analysis

### Risk 1: rustc rejects converted MIR
- MIR validation may fail if types/locals are wrong
- Mitigation: Add debug output, compare with native rustc MIR

### Risk 2: Borrow checker rejects C++ code
- C++ patterns may not satisfy Rust's borrow rules
- Mitigation: Override `mir_borrowck` to skip C++ functions

### Risk 3: Function resolution is complex
- Need to map function names to DefIds
- Mitigation: Start with leaf functions (no calls)

## Success Criteria

1. `cargo test --features rustc-integration` passes
2. A C++ function compiles via rustc without clang++
3. The resulting executable runs correctly

## Timeline Estimate

- Phase 1: Simple function (no calls) - Small task
- Phase 2: Binary operations - Already works
- Phase 3: Function calls - Requires 1.2.5 completion

## Next Steps

1. Write test for simple leaf function (`get_five`)
2. Verify `mir_built` override is triggered
3. Debug any MIR validation errors
4. Iterate until leaf functions work
5. Then tackle function call resolution
