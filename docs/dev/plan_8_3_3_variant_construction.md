# Plan: std::variant Construction/Assignment (Task 8.3.3)

## Goal
Handle std::variant initialization and assignment by wrapping values in the appropriate enum variant.

## Problem
When C++ code does:
```cpp
std::variant<int, double> v = 42;
```

The current transpilation generates:
```rust
let mut v: Variant_i32_f64 = Default::default();
```

But it should generate:
```rust
let mut v: Variant_i32_f64 = Variant_i32_f64::V0(42);
```

## Design

### Key Insight
C++ variant construction calls a templated constructor that determines the active variant index based on the argument type. We need to:
1. Detect when the target type is a variant
2. Find the type of the initializer expression
3. Match that type to the correct variant index
4. Wrap the initializer in the enum variant constructor

### Implementation Steps

1. **Add helper to check if type is variant and get args**:
```rust
fn is_variant_type(ty: &CppType) -> Option<Vec<String>> {
    if let CppType::Named(name) = ty {
        if let Some(rest) = name.strip_prefix("std::variant<") {
            if let Some(inner) = rest.strip_suffix(">") {
                return Some(parse_template_args(inner));
            }
        }
    }
    None
}
```

2. **Add helper to find variant index for type**:
```rust
fn find_variant_index_for_type(variant_args: &[String], init_type: &CppType) -> Option<usize> {
    let init_type_str = init_type.to_rust_type_str();
    for (idx, arg) in variant_args.iter().enumerate() {
        let arg_type = CppType::Named(arg.clone()).to_rust_type_str();
        if arg_type == init_type_str {
            return Some(idx);
        }
    }
    None
}
```

3. **Modify initialization code**:
In the VarDecl handling, after computing `expr`:
- If type is variant AND we can determine the init type AND it matches a variant index
- Wrap as: `EnumName::V{idx}({expr})`

### Challenges
- Getting the initializer's C++ type (need to look at CXXConstructExpr or ImplicitCastExpr)
- Some initializers go through implicit conversions

### Testing
- Simple int initialization
- Double initialization
- char (converts to int in some cases)
- Multiple different variant types

## Files Modified
- `crates/fragile-clang/src/ast_codegen.rs` - add helpers and modify VarDecl handling

## Estimated LOC
~50-80 lines
