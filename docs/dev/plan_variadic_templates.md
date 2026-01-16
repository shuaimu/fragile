# Plan: Variadic Templates

## Task
Implement basic variadic template support for function templates.

## Background
C++ variadic templates allow functions/classes to accept an arbitrary number of type parameters:

```cpp
// Pack declaration
template<typename... Args>
void print(Args... args) {}

// Mixed: regular + pack
template<typename T, typename... Rest>
T first(T head, Rest... tail) { return head; }

// sizeof... operator
template<typename... Args>
int count() { return sizeof...(Args); }
```

The `...` before a name declares a parameter pack, and `...` after a pattern expands it.

## Implementation

### 1. Add TemplateParamPack to CppType
```rust
pub enum CppType {
    // ... existing variants ...

    /// A template parameter pack (typename... Args)
    ParameterPack {
        /// Parameter name (e.g., "Args")
        name: String,
        /// Whether this is expanded (Args... vs Args)
        is_expanded: bool,
    },
}
```

### 2. Update CppFunctionTemplate
```rust
pub struct CppFunctionTemplate {
    // ... existing fields ...
    /// Which template parameters are packs (by index)
    pub parameter_packs: Vec<usize>,
}
```

### 3. Update parsing in parse.rs
In `get_template_type_params`:
- Check if template type parameter is a pack (name ends with `...` in spelling or use `clang_Cursor_isVariadic`)
- Track which parameters are packs

In `convert_type_with_template_ctx`:
- Handle pack types (types ending with `...`)

In `convert_cursor_kind`:
- Handle `CXCursor_PackExpansionExpr` (142)
- Handle `CXCursor_SizeOfPackExpr` (143)

### 4. Update type substitution
In `types.rs`:
- Add substitute logic for ParameterPack
- For now, treat unexpanded packs as errors (full expansion is complex)

## Files to Modify
- `crates/fragile-clang/src/types.rs` - Add ParameterPack variant
- `crates/fragile-clang/src/lib.rs` - Add parameter_packs field
- `crates/fragile-clang/src/parse.rs` - Detect pack parameters
- `crates/fragile-clang/src/convert.rs` - Initialize parameter_packs
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_variadic_template_parsed` - Pack parameter is detected
2. `test_variadic_template_type` - Args... type is ParameterPack
3. `test_sizeof_pack_expr` - sizeof...(Args) is detected
4. `test_mixed_template_params` - Regular + pack params work together

## Scope
This implements detection and representation only, not full pack expansion.
Full variadic template instantiation (expanding the pack) is significantly more complex
and would be a follow-up task.

## Estimated Size
~150 lines of code
