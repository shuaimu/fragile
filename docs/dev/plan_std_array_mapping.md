# Plan: std::array<T, N> → [T; N] Type Mapping

## Task
Implement type mapping from C++ `std::array<T, N>` to Rust `[T; N]` fixed-size array.

## Analysis

### Input Examples
- `std::array<int, 5>` → `[i32; 5]`
- `std::array<double, 10>` → `[f64; 10]`
- `std::array<std::string, 3>` → `[String; 3]`
- `const std::array<int, 5>` → `[i32; 5]` (const handled elsewhere)

### Parsing Challenge
Unlike other STL containers where template arguments are all types, `std::array` has:
1. First argument: element type `T` (needs conversion via `to_rust_type_str()`)
2. Second argument: size `N` (integer literal, use as-is)

### Implementation Location
File: `crates/fragile-clang/src/types.rs`
Position: After `std::optional<T>` handling (line 229) before `std::map<K,V>` (line 230)

## Implementation Steps

1. Add pattern matching for `std::array<`
2. Strip prefix `std::array<` and suffix `>`
3. Find the last comma to split element type from size
   - Use rfind to find last comma (handles nested template types)
4. Extract element type string, convert recursively
5. Extract size string, trim whitespace
6. Format as `[{element_type}; {size}]`

## Code

```rust
// Handle std::array<T, N> -> [T; N]
if let Some(rest) = name.strip_prefix("std::array<") {
    if let Some(inner) = rest.strip_suffix(">") {
        // Find the last comma separating element type from size
        if let Some(comma_idx) = inner.rfind(", ") {
            let element_str = &inner[..comma_idx];
            let size_str = inner[comma_idx + 2..].trim();
            let element_type = CppType::Named(element_str.trim().to_string());
            return format!("[{}; {}]", element_type.to_rust_type_str(), size_str);
        }
    }
}
```

## Edge Cases

1. Nested template types: `std::array<std::vector<int>, 5>`
   - Using `rfind` for the comma handles this correctly
2. Const qualifier: `const std::array<int, 5>`
   - Already handled by earlier const stripping in the code flow

## Testing

Add unit test:
```rust
#[test]
fn test_std_array_type_mapping() {
    assert_eq!(
        CppType::Named("std::array<int, 5>".to_string()).to_rust_type_str(),
        "[i32; 5]"
    );
    assert_eq!(
        CppType::Named("std::array<double, 10>".to_string()).to_rust_type_str(),
        "[f64; 10]"
    );
    // Nested template
    assert_eq!(
        CppType::Named("std::array<std::vector<int>, 3>".to_string()).to_rust_type_str(),
        "[Vec<i32>; 3]"
    );
}
```

Add E2E test with actual C++ code using std::array.

## Implementation Complete

**Date**: 2026-01-22

**Changes Made**:
1. Added `std::array<T, N>` → `[T; N]` type mapping in `types.rs`
2. Added `float`, `double`, and `bool` to Named type handling for nested template conversion
3. Added comprehensive unit tests for the new type mapping
4. Updated documentation in `transpiler-status.md`

**Key Design Decision**: Used `rfind(", ")` to find the last comma, which correctly handles nested template types like `std::array<std::vector<int>, 5>`.
