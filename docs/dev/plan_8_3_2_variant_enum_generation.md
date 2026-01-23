# Plan: std::variant Enum Generation (Task 8.3.2)

## Goal
Generate Rust enum definitions for each unique std::variant type found in the C++ code.

## Design

### Approach
Two-pass strategy:
1. **Collection pass**: Traverse AST to find all std::variant<...> types and extract their template arguments
2. **Generation pass**: Emit Rust enum definitions before other top-level declarations

### Generated Output Example

For `std::variant<int, double, std::string>`:
```rust
/// Generated Rust enum for std::variant<int, double, std::string>
#[derive(Clone, Debug)]
pub enum Variant_i32_f64_String {
    V0(i32),
    V1(f64),
    V2(String),
}
```

### Implementation Steps

1. **Make `parse_template_args` public** in types.rs so ast_codegen.rs can use it

2. **Add collection field** to AstCodeGen struct:
```rust
variant_types: HashMap<String, Vec<String>>,  // enum_name -> [type1, type2, ...]
```

3. **Add collection methods**:
```rust
fn collect_variant_types(&mut self, children: &[ClangNode])
fn collect_variant_from_type(&mut self, ty: &CppType)
```

4. **Add generation method**:
```rust
fn generate_variant_enum(&mut self, name: &str, types: &[String])
```

5. **Hook into generate() method**:
- After file header generation
- Before top-level declaration generation

### Files Modified
- `crates/fragile-clang/src/types.rs` - make `parse_template_args` public
- `crates/fragile-clang/src/ast_codegen.rs` - add collection and generation logic

## Estimated LOC
~100-150 lines
