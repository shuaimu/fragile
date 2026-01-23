# Plan: std::visit Support (Task 8.3.4)

## Overview

Implement support for mapping C++ `std::visit` calls to Rust match expressions.

## C++ std::visit Semantics

```cpp
// Single variant
std::variant<int, double, bool> v = 42;
auto result = std::visit([](auto x) { return x * 2; }, v);

// Multiple variants (cartesian product)
std::variant<int, double> v1 = 10;
std::variant<bool, std::string> v2 = true;
std::visit([](auto x, auto y) { /* ... */ }, v1, v2);
```

## Rust Mapping

```rust
// Single variant
let result = match &v {
    Variant_i32_f64_bool::V0(x) => visitor(x),
    Variant_i32_f64_bool::V1(x) => visitor(x),
    Variant_i32_f64_bool::V2(x) => visitor(x),
};

// Multiple variants - cartesian product
let result = match (&v1, &v2) {
    (Variant_i32_f64::V0(x), Variant_bool_String::V0(y)) => visitor(x, y),
    (Variant_i32_f64::V0(x), Variant_bool_String::V1(y)) => visitor(x, y),
    (Variant_i32_f64::V1(x), Variant_bool_String::V0(y)) => visitor(x, y),
    (Variant_i32_f64::V1(x), Variant_bool_String::V1(y)) => visitor(x, y),
};
```

## Implementation Subtasks

### 8.3.4.1 Detection (~60 LOC)
- Add `is_std_visit_call()` function to detect `std::visit(visitor, variants...)`
- Extract visitor expression and variant arguments
- Pattern: Similar to existing `is_std_get_call()`

### 8.3.4.2 Single Variant Support (~80 LOC)
- Generate match expression for single variant
- Handle lambda visitors by inlining call in each arm
- Use existing variant type infrastructure

### 8.3.4.3 Multiple Variant Support (~80 LOC)
- Generate cartesian product of match arms
- Use tuple pattern matching for N variants
- Handle nested tuple access

### 8.3.4.4 Functor/Function Support (~60 LOC)
- Detect visitor type (lambda vs functor vs function)
- For functors: use `visitor.op_call(x)`
- For functions: call directly

### 8.3.4.5 Tests (~50 LOC)
- Single variant with lambda
- Multiple variants with lambda
- Edge cases (empty variant, void return)

## Key Helper Functions

Existing to reuse:
- `get_variant_args(ty)` - Extract template args from variant type
- `get_variant_enum_name(ty)` - Get generated enum name
- `find_variant_index()` - Map C++ type to variant index

New to add:
- `is_std_visit_call(node)` - Detect std::visit calls
- `generate_visit_match(visitor, variants)` - Generate match expression
