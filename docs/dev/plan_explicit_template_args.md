# Plan: Explicit Template Arguments Override

## Task
Implement support for explicit template arguments that override automatic deduction.

## Background
In C++, template functions can be called with explicit template arguments:
```cpp
template<typename T, typename U> T convert(U x);
convert<int>(3.14);      // T = int (explicit), U = double (deduced)
convert<int, double>(x); // Both explicit
```

Explicit arguments take priority over deduction and are useful when:
1. A type parameter cannot be deduced (e.g., return type only)
2. You want to force a specific type
3. You need to prevent type deduction

## Implementation

### 1. Update TypeDeducer API
Add support for explicit arguments to deduction:

```rust
impl TypeDeducer {
    /// Deduce template arguments with some explicitly provided.
    ///
    /// Explicit arguments are applied first, then remaining params are deduced.
    pub fn deduce_with_explicit(
        template: &CppFunctionTemplate,
        explicit_args: &[CppType],  // Ordered by template param position
        call_arg_types: &[CppType],
    ) -> Result<HashMap<String, CppType>, DeductionError>;
}
```

### 2. Add deduce_and_instantiate_with_explicit
```rust
impl CppFunctionTemplate {
    pub fn deduce_and_instantiate_with_explicit(
        &self,
        explicit_args: &[CppType],
        call_arg_types: &[CppType],
    ) -> Result<CppFunction, DeductionError>;
}
```

### 3. Implementation Logic
1. Pre-populate substitutions map with explicit args
2. For each explicit arg (by index), add to substitutions: template_params[i] → explicit_args[i]
3. Run normal deduction for remaining parameters
4. Conflicts only occur if deduction disagrees with explicit args

## Files to Modify
- `crates/fragile-clang/src/deduce.rs` - Add deduce_with_explicit
- `crates/fragile-clang/src/lib.rs` - Add deduce_and_instantiate_with_explicit
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_explicit_single_arg` - Explicit T, no deduction needed
2. `test_explicit_with_deduction` - Explicit T, deduce U
3. `test_explicit_overrides_deduction` - Explicit arg overrides what would be deduced
4. `test_explicit_all_args` - All template args explicit

## Estimated Size
~50-100 lines of code (well under 500 LOC limit)
