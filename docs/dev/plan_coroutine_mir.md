# Plan: D.2 MIR Representation for Coroutines

## Overview

Add MIR (Mid-level Intermediate Representation) structures to represent C++20 coroutine operations.

## Background

In rustc MIR, coroutines are represented with special terminators for yield and return operations:
- `Yield` - Suspends the coroutine and yields a value
- Generator return - Returns from a generator with final value

## Design

### 1. New MirTerminator Variants (lib.rs)

Add to the `MirTerminator` enum:

```rust
/// Yield from a coroutine (co_yield in C++)
Yield {
    /// Value being yielded
    value: MirOperand,
    /// Block to resume at after yield
    resume: usize,
    /// Block for when the coroutine is dropped while suspended
    drop: Option<usize>,
},

/// Await an awaitable (co_await in C++)
Await {
    /// The awaitable being awaited
    awaitable: MirOperand,
    /// Destination place for the await result
    destination: MirPlace,
    /// Block to resume at after await completes
    resume: usize,
    /// Block for when the coroutine is dropped while suspended
    drop: Option<usize>,
},

/// Return from a coroutine (co_return in C++)
CoroutineReturn {
    /// Optional value being returned (None for co_return;)
    value: Option<MirOperand>,
},
```

### 2. MirBody Updates

Add a flag to track if a function body is a coroutine:

```rust
pub struct MirBody {
    /// Basic blocks in the MIR
    pub blocks: Vec<MirBasicBlock>,
    /// Local variable declarations
    pub locals: Vec<MirLocal>,
    /// Whether this is a coroutine body
    pub is_coroutine: bool,
}
```

### 3. No MirRvalue Changes Needed

The yield/await operations are control flow (terminators), not computations (rvalues).
No new MirRvalue variants are needed.

## Implementation Steps

1. Add `Yield`, `Await`, `CoroutineReturn` to `MirTerminator` enum
2. Add `is_coroutine` field to `MirBody`
3. Update `MirBody::new()` and `Default` impl
4. Update any code that pattern-matches on `MirTerminator`

## Test Plan

The MIR structures don't need separate tests - they will be tested through the conversion tests in D.3.

## Estimated LOC

- lib.rs: ~40 lines (new enum variants + field)
- convert.rs updates: ~10 lines (if any pattern matches need updating)
- Total: ~50 lines
