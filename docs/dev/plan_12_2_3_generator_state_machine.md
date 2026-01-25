# Plan: Generator State Machine (Task 12.2.3)

## Overview

Generate state machine struct for generators to enable stable Rust support. Currently generators use unstable `yield` keyword; this task replaces it with explicit state machine implementation.

## Current Implementation

```rust
// Current (unstable Rust)
pub fn range() -> impl Iterator<Item=i32> {
    yield 1;
    yield 2;
    yield 3;
}
```

## Target Implementation

```rust
// Target (stable Rust)
pub struct RangeGenerator {
    __state: i32,
    // captured local variables go here
}

impl Iterator for RangeGenerator {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.__state {
            0 => { self.__state = 1; Some(1) }
            1 => { self.__state = 2; Some(2) }
            2 => { self.__state = 3; Some(3) }
            _ => None
        }
    }
}

pub fn range() -> impl Iterator<Item=i32> {
    RangeGenerator { __state: 0 }
}
```

## Implementation Steps

### 1. Detect Generator Functions
- Check `is_coroutine` and `coroutine_info.kind == Generator`
- Extract yield points from function body

### 2. Collect Generator State
- Find all local variables that live across yield points
- These become fields in the state machine struct

### 3. Generate State Machine Struct
- Create `{FunctionName}Generator` struct
- Add `__state: i32` field for state tracking
- Add fields for captured local variables

### 4. Generate Iterator Implementation
- Implement `Iterator` trait for the struct
- `type Item = T` where T is the yield type
- Transform function body into state machine in `next()`

### 5. Transform Yield Points
- Each `co_yield value` becomes a state transition + return Some(value)
- End of function returns None

### 6. Update Function Body
- Replace function body with just returning the generator instance

## Files to Modify

- `crates/fragile-clang/src/ast_codegen.rs`:
  - Add `generate_generator_struct` method
  - Modify `generate_function` to detect generators
  - Add state machine transformation logic

## Example Transformation

C++ input:
```cpp
Generator<int> count_to_3() {
    int i = 1;
    while (i <= 3) {
        co_yield i;
        i++;
    }
}
```

Rust output:
```rust
pub struct CountTo3Generator {
    __state: i32,
    i: i32,
}

impl Iterator for CountTo3Generator {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.__state {
                0 => {
                    self.i = 1;
                    self.__state = 1;
                }
                1 => {
                    if self.i <= 3 {
                        let result = self.i;
                        self.i += 1;
                        return Some(result);
                    } else {
                        self.__state = -1;
                    }
                }
                _ => return None,
            }
        }
    }
}

pub fn count_to_3() -> impl Iterator<Item=i32> {
    CountTo3Generator { __state: 0, i: 0 }
}
```

## Simplification for Initial Implementation

For the initial implementation, we'll handle the common case:
- Simple generators with sequential yields (no loops/conditions around yields)
- Each yield becomes a state

This covers patterns like:
```cpp
Generator<int> simple() {
    co_yield 1;
    co_yield 2;
    co_yield 3;
}
```

Complex control flow around yields can be handled later.

## Estimated LOC: ~200
