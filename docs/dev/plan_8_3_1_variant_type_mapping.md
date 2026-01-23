# Plan: std::variant Type Mapping (Task 8.3.1)

## Goal
Parse `std::variant<T1, T2, ...>` C++ types and generate appropriate Rust enum types.

## Design

### Approach
Use string-based pattern matching (same approach as other STL types) in `to_rust_type_str()`.

### Key Challenges
1. **Nested templates**: `std::variant<std::vector<int>, std::string>` - need to correctly parse template arguments
2. **Unique enum names**: Need to generate unique enum names for each variant type
3. **Type extraction**: Parse comma-separated template args while respecting nesting

### Implementation

#### 1. Add helper function for parsing template arguments
```rust
/// Parse comma-separated template arguments, respecting nested templates.
/// Returns a vector of trimmed argument strings.
fn parse_template_args(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in args.chars() {
        match ch {
            '<' => {
                depth += 1;
                current.push(ch);
            }
            '>' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                result.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}
```

#### 2. Add std::variant handling in `to_rust_type_str()`
```rust
// Handle std::variant<T1, T2, ...> -> Variant_T1_T2_...
if let Some(rest) = name.strip_prefix("std::variant<") {
    if let Some(inner) = rest.strip_suffix(">") {
        let args = parse_template_args(inner);
        let rust_types: Vec<String> = args.iter()
            .map(|a| CppType::Named(a.clone()).to_rust_type_str())
            .collect();

        // Generate unique enum name from types
        let enum_name = format!("Variant_{}", rust_types.join("_"));
        return enum_name;
    }
}
```

#### 3. Note on enum generation
For task 8.3.1 (type mapping only), we just return a unique type name.
The actual enum definition generation is task 8.3.2.

### Testing

1. Unit tests in types.rs for:
   - Basic variant: `std::variant<int, double>`
   - With strings: `std::variant<int, std::string>`
   - Nested templates: `std::variant<std::vector<int>, std::optional<double>>`

2. Verify that generated names are valid Rust identifiers

## Files Modified
- `crates/fragile-clang/src/types.rs` - add parse_template_args helper and std::variant handling

## Estimated LOC
~50-60 lines (helper function + pattern matching + tests)
