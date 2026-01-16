# Plan: E.1.3 Stack Unwinding for C++ Exceptions

## Overview

Stack unwinding is the process of calling destructors for local objects when an exception is thrown. This is essential for proper C++ RAII semantics.

## Current State

- **AST Support**: Complete - TryStmt, CatchStmt, ThrowExpr nodes are parsed
- **MIR Representation**: Basic - Try/Catch/Throw terminators exist
- **Runtime**: Stub - Uses `panic!()` for throw, no actual unwinding

## Goal

Implement proper stack unwinding so destructors are called when exceptions propagate.

## Architecture Analysis

### Option 1: Rust's panic mechanism
- Pros: Built-in, works with Rust's unwinding
- Cons: Different ABI from C++, may not interop with C++ libraries

### Option 2: LLVM's exception handling intrinsics
- Pros: Standard C++ ABI compatible, proper interop
- Cons: Requires direct LLVM IR generation, complex

### Option 3: setjmp/longjmp based
- Pros: Portable, simple implementation
- Cons: Doesn't call destructors automatically, need manual tracking

### Recommended: Option 1 (Rust's panic) for MVP

For the initial implementation, we'll use Rust's panic mechanism because:
1. It already handles unwinding
2. We're compiling to rustc MIR anyway
3. Can upgrade to Option 2 later for full C++ ABI compatibility

## Implementation Design

### Phase 1: Cleanup Block Generation (~100 LOC)

When converting C++ to MIR, generate cleanup blocks for objects with destructors.

```rust
// In convert.rs, when processing a block with local objects:
struct CleanupInfo {
    local: MirLocal,
    destructor: String,  // Mangled destructor name
}

// Each block tracks objects needing cleanup
fn convert_block_with_cleanup(stmts: Vec<CppStmt>) -> (MirBasicBlock, Vec<CleanupInfo>) {
    // Convert statements
    // Track objects with destructors
    // Return block + cleanup info
}
```

### Phase 2: Unwind Terminators (~50 LOC)

Add unwind targets to terminators that can throw.

```rust
// In lib.rs MirTerminator:
pub enum MirTerminator {
    Call {
        func: String,
        args: Vec<MirOperand>,
        destination: Option<MirPlace>,
        target: Option<usize>,      // Normal continuation
        unwind: Option<usize>,      // Cleanup block on unwind
    },
    // ...
}
```

### Phase 3: Landing Pad Generation (~80 LOC)

Generate landing pads that run cleanup code.

```rust
// Example MIR for: { Foo f; might_throw(); }
//
// bb0:
//   _1 = Foo::Foo()         // construct f
//   call might_throw() -> [return: bb1, unwind: bb2]
// bb1:
//   call Foo::~Foo(&_1)     // destruct f (normal path)
//   return
// bb2 (cleanup):
//   call Foo::~Foo(&_1)     // destruct f (unwind path)
//   resume                  // continue unwinding
```

### Phase 4: Runtime Integration (~50 LOC)

Connect to fragile_runtime exception handling.

```rust
// In exceptions.rs:
/// Begin unwinding with destructor cleanup
#[no_mangle]
pub extern "C" fn fragile_rt_unwind_throw(exception: CppException) -> ! {
    // Store exception
    // Use std::panic::panic_any() for unwinding
    std::panic::panic_any(FragileException(exception))
}

/// Check if we're catching the right type
#[no_mangle]
pub extern "C" fn fragile_rt_unwind_catch(type_info: *const c_void) -> bool {
    // Check if current panic matches type
}
```

## Implementation Steps

### Step 1: Add Unwind Fields to MirTerminator
File: `crates/fragile-clang/src/lib.rs`
- Add `unwind: Option<usize>` to `Call` variant
- Add `cleanup: bool` field to `MirBasicBlock`

### Step 2: Track Destructors in Converter
File: `crates/fragile-clang/src/convert.rs`
- Track objects with destructors in scope
- Generate cleanup blocks when exiting scope

### Step 3: Add Resume Terminator
File: `crates/fragile-clang/src/lib.rs`
- Add `Resume` terminator for continuing unwinding

### Step 4: Update Runtime
File: `crates/fragile-runtime/src/exceptions.rs`
- Add unwinding-based throw
- Add catch with type matching

### Step 5: Add Tests
File: `crates/fragile-clang/tests/integration_test.rs`
- Test destructor calls on throw
- Test nested try/catch
- Test multiple objects in scope

## Estimated Effort

| Component | LOC | Complexity |
|-----------|-----|------------|
| MirTerminator changes | ~30 | Low |
| Cleanup tracking | ~100 | Medium |
| Landing pad generation | ~80 | Medium |
| Runtime changes | ~50 | Low |
| Tests | ~100 | Low |
| **Total** | ~360 | Medium |

## Testing Strategy

```cpp
// Test 1: Basic destructor call on throw
struct Counter {
    static int count;
    Counter() { count++; }
    ~Counter() { count--; }
};

void test_basic() {
    try {
        Counter c;
        throw 1;
    } catch (int) {}
    assert(Counter::count == 0);
}

// Test 2: Multiple objects
void test_multiple() {
    try {
        Counter c1;
        Counter c2;
        throw 1;
    } catch (int) {}
    assert(Counter::count == 0);
}

// Test 3: Nested try/catch
void test_nested() {
    Counter c1;
    try {
        Counter c2;
        try {
            Counter c3;
            throw 1;
        } catch (int) {
            throw;  // rethrow
        }
    } catch (int) {}
    assert(Counter::count == 1);  // c1 still alive
}
```

## Dependencies

- Phase A.2 Classes Complete (constructors/destructors) ✅
- Phase E.1 Exceptions (try/catch/throw) ✅

## Risks

1. **Rust panic interop**: May need careful handling at FFI boundaries
2. **Performance**: Cleanup tracking adds overhead
3. **C++ ABI compatibility**: Not compatible with native C++ exceptions initially

## Future Work

- Full C++ exception ABI compatibility (LLVM intrinsics)
- Stack trace generation
- Exception specifications enforcement
