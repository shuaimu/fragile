# Plan: Fix Namespace Function Call Path Resolution

## Problem

When transpiling C++ code with namespaces, the generated Rust code uses absolute namespace paths even when calling functions within the same namespace or a parent namespace.

### Example

C++ code:
```cpp
namespace foo {
    int helper() { return 42; }
    int test() { return helper(); }  // Same namespace call
}
```

Generated (incorrect):
```rust
pub mod foo {
    pub fn helper() -> i32 { return 42i32; }
    pub fn test() -> i32 { return foo::helper(); }  // ERROR: foo not visible here
}
```

Expected:
```rust
pub mod foo {
    pub fn helper() -> i32 { return 42i32; }
    pub fn test() -> i32 { return helper(); }  // or self::helper()
}
```

## Root Cause

In `ast_codegen.rs`, the `expr_to_string` function handles `DeclRefExpr` nodes (lines 2835-2842) by simply concatenating the `namespace_path` without considering the current namespace context:

```rust
let full_path = if namespace_path.is_empty() {
    ident.clone()
} else {
    let path: Vec<_> = namespace_path.iter()
        .map(|s| sanitize_identifier(s))
        .collect();
    format!("{}::{}", path.join("::"), ident)
};
```

## Solution

1. Add a `current_namespace: Vec<String>` field to `AstCodeGen` to track the current namespace stack
2. Update `generate_top_level` to push/pop namespace names when entering/leaving `NamespaceDecl` nodes
3. Modify `expr_to_string` for `DeclRefExpr` to compute relative paths:
   - If `namespace_path` equals `current_namespace`, use just `ident`
   - If `namespace_path` is a prefix of `current_namespace`, use `super::` as needed
   - If `namespace_path` starts with a common prefix, use relative path
   - Otherwise, use absolute path with `crate::` prefix

## Implementation Steps

1. Add `current_namespace: Vec<String>` field to `AstCodeGen` struct
2. Initialize it to empty in `AstCodeGen::new()`
3. In `generate_top_level` for `NamespaceDecl`:
   - Push namespace name before processing children
   - Pop after processing children
4. In `expr_to_string` for `DeclRefExpr`:
   - Compute relative path between `current_namespace` and `namespace_path`
   - Handle edge cases:
     - Same namespace: just use ident
     - Child namespace: use relative path (e.g., `inner::func`)
     - Parent namespace: use `super::` prefix
     - Sibling namespace: use `super::sibling::func`
     - Global scope from within namespace: use `super::` chain or `crate::`

## Test Cases

1. Same namespace call: `foo::helper()` from within `foo` → `helper()`
2. Nested namespace call: `outer::inner::func()` from within `outer` → `inner::func()`
3. Parent namespace call: `outer::func()` from within `outer::inner` → `super::func()`
4. Global function call from namespace: `global_func()` from within `ns` → `super::global_func()`
5. Sibling namespace call: `bar::func()` from within `foo` → `super::bar::func()`

## Estimated Changes

- `ast_codegen.rs`: ~50 lines added/modified
- Add helper function `compute_relative_path(&self, target_ns: &[String]) -> String`
