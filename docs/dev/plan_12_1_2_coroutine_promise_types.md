# Plan: 12.1.2 Parse Coroutine Promise Types from Return Type

## Overview

In C++20 coroutines, the return type of a coroutine function determines its "promise type" via `std::coroutine_traits`. This task adds parsing support to extract and store promise type information.

## Background

### C++ Coroutine Promise Types

```cpp
// A coroutine returning Task<int> has promise type Task<int>::promise_type
Task<int> async_compute() {
    co_return 42;
}

// A generator returning Generator<int> has promise type Generator<int>::promise_type
Generator<int> count() {
    for (int i = 0; i < 10; ++i) {
        co_yield i;
    }
}
```

The promise type is determined by:
1. `std::coroutine_traits<ReturnType, ParamTypes...>::promise_type`
2. Usually `ReturnType::promise_type` for simple cases

### What We Need to Extract

From a coroutine return type, we need:
1. **Value type** - The `T` in `Task<T>` or `Generator<T>` (for Result/Iterator generation)
2. **Coroutine kind** - Is it async (Task), generator (Generator), or custom
3. **Promise type name** - For debugging/documentation

## Design

### 1. Add Coroutine Metadata to FunctionDecl (~15 LOC in ast.rs)

```rust
// New struct to hold coroutine-specific information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoroutineInfo {
    /// The value type for the coroutine (T in Task<T> or Generator<T>)
    pub value_type: Option<CppType>,
    /// The coroutine kind based on return type analysis
    pub kind: CoroutineKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoroutineKind {
    /// Uses co_await, maps to async fn
    Async,
    /// Uses co_yield, maps to Iterator/Generator
    Generator,
    /// Uses co_return only, basic coroutine
    Task,
    /// Unknown or custom coroutine type
    Custom,
}
```

Update `FunctionDecl`:
```rust
FunctionDecl {
    // ... existing fields ...
    is_coroutine: bool,
    coroutine_info: Option<CoroutineInfo>,  // NEW
}
```

### 2. Extract Promise Type Info in parse.rs (~45 LOC)

Add function to analyze coroutine return type:

```rust
fn extract_coroutine_info(&self, return_type: &CppType) -> Option<CoroutineInfo> {
    // Check if return type matches known coroutine patterns
    match return_type {
        CppType::Named(name) => {
            // Check for Task<T>, Generator<T>, etc.
            if let Some(value_type) = self.extract_template_arg(name, &["Task", "std::task", "cppcoro::task"]) {
                return Some(CoroutineInfo {
                    value_type: Some(value_type),
                    kind: CoroutineKind::Async,
                });
            }
            if let Some(value_type) = self.extract_template_arg(name, &["Generator", "std::generator", "cppcoro::generator"]) {
                return Some(CoroutineInfo {
                    value_type: Some(value_type),
                    kind: CoroutineKind::Generator,
                });
            }
            // Fallback: check if contains promise_type (future enhancement)
            None
        }
        _ => None,
    }
}

fn extract_template_arg(&self, type_name: &str, patterns: &[&str]) -> Option<CppType> {
    for pattern in patterns {
        if type_name.starts_with(pattern) && type_name.contains('<') {
            // Extract the template argument
            if let Some(start) = type_name.find('<') {
                if let Some(end) = type_name.rfind('>') {
                    let arg = &type_name[start + 1..end];
                    return Some(self.parse_type_name(arg.trim()));
                }
            }
        }
    }
    None
}
```

### 3. Update Code Generation in ast_codegen.rs (~20 LOC)

Use coroutine_info for better Rust output:

```rust
// For async coroutines with known value type:
// C++: Task<int> compute() -> Rust: async fn compute() -> i32

// For generators with known value type:
// C++: Generator<int> range() -> Rust: fn range() -> impl Iterator<Item=i32>
```

## Implementation Steps

1. Add `CoroutineInfo` and `CoroutineKind` to ast.rs
2. Add `coroutine_info` field to `FunctionDecl`
3. Implement `extract_coroutine_info` in parse.rs
4. Update code generation to use coroutine_info for return type
5. Add tests for various coroutine return types

## Testing

Test cases:
- `Task<int>` coroutine → `async fn -> i32`
- `Generator<std::string>` coroutine → `fn -> impl Iterator<Item=String>`
- Custom coroutine types → fallback to current behavior
- Non-coroutine functions → no coroutine_info

## Estimated LOC

- ast.rs: ~25 LOC (CoroutineInfo struct, CoroutineKind enum, field addition)
- parse.rs: ~45 LOC (extraction logic)
- ast_codegen.rs: ~10 LOC (use coroutine_info in generation)
- tests: ~30 LOC

**Total: ~80 LOC** (matches estimate)
