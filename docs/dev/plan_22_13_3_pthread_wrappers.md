# Plan: pthread Wrappers (Task 22.13.3) ✅ COMPLETED

## Overview

Implement pthread wrappers in fragile-runtime to support transpiled multithreaded C++ code.
When C++ code using `std::thread` is transpiled, libc++ internally calls pthread functions.
We provide these functions using Rust's std::thread.

## Key Design Decision: Function Name Prefixing

**IMPORTANT**: All pthread functions are prefixed with `fragile_` to avoid symbol conflicts
with system pthread functions in glibc/libc. Without this prefix, the `#[no_mangle]` exported
functions would intercept calls from Rust's std::thread (which uses the real pthread internally),
causing infinite recursion or segfaults.

The transpiler must generate calls to `fragile_pthread_create`, `fragile_pthread_join`, etc.
instead of the standard pthread names.

## Implemented Types

```rust
// Thread handle - wraps Rust JoinHandle via heap-allocated Box
#[repr(C)]
pub struct fragile_pthread_t {
    pub id: u64,                    // Unique thread ID
    pub handle_ptr: *mut c_void,    // Box<JoinHandle<usize>> as raw pointer
}

// Thread attributes (for API compatibility)
#[repr(C)]
pub struct fragile_pthread_attr_t {
    detach_state: c_int,  // 0 = joinable, 1 = detached
}
```

## Implemented Functions

### Core Threading
- `fragile_pthread_create(thread, attr, start_routine, arg)` - Create a new thread
- `fragile_pthread_join(thread, retval)` - Wait for thread to finish
- `fragile_pthread_self()` - Get current thread ID
- `fragile_pthread_equal(t1, t2)` - Compare thread IDs
- `fragile_pthread_detach(thread)` - Detach a thread
- `fragile_pthread_exit(retval)` - Exit current thread (via panic)

### Attributes
- `fragile_pthread_attr_init(attr)` - Initialize attributes
- `fragile_pthread_attr_destroy(attr)` - Destroy attributes
- `fragile_pthread_attr_setdetachstate(attr, state)` - Set detach state
- `fragile_pthread_attr_getdetachstate(attr, state)` - Get detach state

## Implementation Details

### Thread Safety
- Raw pointers (`*mut c_void`) cannot be sent across threads safely
- Solved by wrapping in `ThreadStartInfo` struct with `arg` as `usize`
- `unsafe impl Send for ThreadStartInfo {}` allows crossing thread boundary

### JoinHandle Storage
- Rust's `JoinHandle<T>` is stored on the heap via `Box::into_raw()`
- Recovered in `pthread_join` via `Box::from_raw()`
- Thread result stored as `usize` to avoid raw pointer issues

### Thread ID Generation
- Uses `AtomicU64` counter for unique thread IDs
- `pthread_self` extracts ID from Rust's `ThreadId` debug representation

## Files

- `crates/fragile-runtime/src/pthread.rs` - All pthread implementations (~250 LOC)
- `crates/fragile-runtime/src/lib.rs` - Module declaration and re-export

## Tests

3 tests implemented:
1. `test_pthread_self` - Verifies current thread ID is valid
2. `test_pthread_attr` - Tests attribute init/set/get/destroy
3. `test_pthread_create_join` - Tests full thread creation and join

## Status

✅ COMPLETED 2026-01-24

All tests passing (14 total in fragile-runtime, including 3 new pthread tests).
