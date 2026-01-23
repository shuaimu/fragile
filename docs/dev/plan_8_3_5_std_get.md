# Plan 8.3.5: std::get<T>/std::get<I> Support

## Goal
Convert `std::get<T>(variant)` and `std::get<I>(variant)` calls to Rust pattern matching.

## Analysis

### Input (C++)
```cpp
std::variant<int, double, bool> v = 42;
int x = std::get<int>(v);      // by type
int y = std::get<0>(v);        // by index
```

### Current Output (Wrong)
```rust
let x: i32 = get(v);
let y: i32 = get(v);
```

### Desired Output
```rust
let x: i32 = match &v {
    Variant_i32_f64_bool::V0(val) => val.clone(),
    _ => panic!("bad variant access"),
};
let y: i32 = match &v {
    Variant_i32_f64_bool::V0(val) => val.clone(),
    _ => panic!("bad variant access"),
};
```

## Key Observations

1. **Clang resolves template arguments** - The `std::get<int>` call type is `int&` (reference to int), telling us which variant to extract.

2. **Detection method** - Check if a CallExpr's callee is:
   - A DeclRefExpr with `name == "get"`
   - In namespace `std` (namespace_path contains "std")
   - Has exactly one argument that is a variant type

3. **Variant type detection** - We already have `get_variant_args()` to check if a type is a variant.

4. **Index mapping** - The return type `int&` tells us to extract `V0` (the first variant type). We match the return type (without reference) to find the index.

## Implementation Steps

### Step 1: Add helper to detect std::get calls (~20 LOC)
Location: `ast_codegen.rs`

```rust
/// Check if this is a std::get call on a variant and return (variant_arg, return_type)
fn is_std_get_call(node: &ClangNode) -> Option<(&ClangNode, &CppType)> {
    if let ClangNodeKind::CallExpr { ty } = &node.kind {
        // First child should be DeclRefExpr for "get" in namespace "std"
        if let Some(callee) = node.children.first() {
            if let ClangNodeKind::DeclRefExpr { name, namespace_path, .. } = &callee.kind {
                if name == "get" && namespace_path.contains(&"std".to_string()) {
                    // Second child is the variant argument
                    if let Some(variant_arg) = node.children.get(1) {
                        return Some((variant_arg, ty));
                    }
                }
            }
        }
    }
    None
}
```

### Step 2: Add helper to get variant index from return type (~30 LOC)
```rust
fn get_variant_index_from_type(variant_type: &CppType, extract_type: &CppType) -> Option<usize> {
    // Get the variant's type arguments
    let variant_args = Self::get_variant_args(variant_type)?;

    // Extract type may be a reference; strip it
    let target_type = match extract_type {
        CppType::Reference { pointee, .. } => pointee.as_ref(),
        _ => extract_type,
    };

    // Convert target to Rust type string for comparison
    let target_rust = target_type.to_rust_type_str();

    // Find matching index
    for (i, arg) in variant_args.iter().enumerate() {
        // Parse C++ type and convert to Rust
        let cpp_type = CppType::from_cpp_type_str(arg);
        let rust_type = cpp_type.to_rust_type_str();
        if rust_type == target_rust {
            return Some(i);
        }
    }
    None
}
```

### Step 3: Modify CallExpr handling in expr_to_string (~30 LOC)
Location: In the `CallExpr` arm of `expr_to_string`, add at the beginning:

```rust
ClangNodeKind::CallExpr { ty } => {
    // Check for std::get call on variant
    if let Some((variant_arg, return_type)) = Self::is_std_get_call(node) {
        let variant_type = Self::get_expr_type(variant_arg);
        if let Some(ref var_type) = variant_type {
            if let Some(variant_args) = Self::get_variant_args(var_type) {
                // Get the variant index from return type
                if let Some(idx) = Self::get_variant_index_from_type(var_type, return_type) {
                    let enum_name = Self::get_variant_enum_name(&variant_args);
                    let variant_expr = self.expr_to_string(variant_arg);
                    // Generate match expression
                    return format!(
                        "match &{} {{ {}::V{}(val) => val.clone(), _ => panic!(\"bad variant access\") }}",
                        variant_expr, enum_name, idx
                    );
                }
            }
        }
    }
    // ... existing code
}
```

## Estimated LOC
- Helper functions: ~50 LOC
- CallExpr modification: ~20 LOC
- Total: ~70 LOC (well under 500)

## Testing
Add E2E test `test_e2e_std_get` with:
- `std::get<Type>` access
- `std::get<Index>` access
- Multiple variant types

## Edge Cases
1. Return type is `int&` (reference) - need to strip reference
2. Both `std::get<int>` and `std::get<0>` should work (same output from Clang)
3. Wrong type/index should generate panic (not compile-time checked)
