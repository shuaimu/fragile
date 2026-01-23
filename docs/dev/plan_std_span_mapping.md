# Plan: std::span<T> → &[T] Type Mapping

## Task
Implement type mapping from C++ `std::span<T>` to Rust `&[T]` slice type.

## Analysis

### C++ std::span
- C++20 feature representing a contiguous sequence of objects
- Non-owning reference to a sequence
- Can have static extent (known size at compile time) or dynamic extent

### Input Examples
- `std::span<int>` → `&[i32]` (dynamic extent, mutable by default)
- `std::span<const int>` → `&[i32]` (immutable)
- `std::span<int, 5>` → `&[i32; 5]` (static extent - optional complexity)
- `const std::span<int>` → `&[i32]` (const span itself)

### Simplification
For initial implementation:
- Map all `std::span<T>` to `&mut [T]` (mutable slice) for non-const element types
- Map `std::span<const T>` to `&[T]` (immutable slice)
- Ignore static extent for now (treat all as dynamic)

### Implementation Location
File: `crates/fragile-clang/src/types.rs`
Position: After `std::array<T, N>` handling

## Implementation Steps

1. Add pattern matching for `std::span<`
2. Strip prefix `std::span<` and suffix `>`
3. Check if element type starts with "const " to determine mutability
4. Handle possible extent parameter (ignore for now, just extract element type)
5. Convert element type recursively
6. Format as `&[T]` or `&mut [T]`

## Code

```rust
// Handle std::span<T> -> &[T] / &mut [T]
if let Some(rest) = name.strip_prefix("std::span<") {
    if let Some(inner) = rest.strip_suffix(">") {
        // Handle dynamic vs static extent: "int" or "int, 5"
        let element_str = if let Some(comma_idx) = inner.rfind(", ") {
            // Check if the part after comma is a number (extent)
            let after_comma = inner[comma_idx + 2..].trim();
            if after_comma.chars().all(|c| c.is_ascii_digit() || c == '_') {
                // It's an extent, ignore it
                &inner[..comma_idx]
            } else {
                inner // No numeric extent, use full string
            }
        } else {
            inner
        };

        let element_str = element_str.trim();

        // Check for const element type
        let (is_const, element_type_str) = if let Some(rest) = element_str.strip_prefix("const ") {
            (true, rest.trim())
        } else if element_str.ends_with(" const") {
            (true, element_str.strip_suffix(" const").unwrap().trim())
        } else {
            (false, element_str)
        };

        let element_type = CppType::Named(element_type_str.to_string());
        if is_const {
            return format!("&[{}]", element_type.to_rust_type_str());
        } else {
            return format!("&mut [{}]", element_type.to_rust_type_str());
        }
    }
}
```

## Testing

Add unit test:
```rust
#[test]
fn test_std_span_type_mapping() {
    // Dynamic extent, mutable
    assert_eq!(
        CppType::Named("std::span<int>".to_string()).to_rust_type_str(),
        "&mut [i32]"
    );
    // Const element type
    assert_eq!(
        CppType::Named("std::span<const int>".to_string()).to_rust_type_str(),
        "&[i32]"
    );
    // With static extent (ignored)
    assert_eq!(
        CppType::Named("std::span<double, 10>".to_string()).to_rust_type_str(),
        "&mut [f64]"
    );
}
```

## Implementation Complete

**Date**: 2026-01-22

**Changes Made**:
1. Added `std::span<T>` → `&mut [T]` / `&[T]` type mapping in `types.rs`
2. Handles both dynamic extent (no size param) and static extent (ignored)
3. Properly detects const element types for immutable slice generation
4. Added comprehensive unit tests
