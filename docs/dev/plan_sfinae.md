# Plan: SFINAE & Type Traits Support

## Task
Implement SFINAE (Substitution Failure Is Not An Error) and basic type traits support.

## Background
SFINAE is a C++ template metaprogramming technique where invalid type substitutions
in templates cause the template to be silently ignored rather than producing an error.

Key patterns from Mako:
```cpp
// std::enable_if - only allow integral types
template<typename T>
typename std::enable_if<std::is_integral<T>::value, T>::type
identity(T x) { return x; }

// std::conditional - choose type based on condition
std::conditional<std::is_scalar<T>::value, T, const T&>::type

// requires clause (C++20) - constraint checking
template<typename T>
explicit MarshallDeputy(std::shared_ptr<T> sp_m)
  requires std::is_base_of_v<rrr::Marshallable, T>
```

## Complexity Assessment
This is a **MEDIUM-HIGH** complexity feature requiring 10-15 iterations.

## Scope
Focus on type traits used in Mako:
- std::is_integral<T>
- std::is_signed<T>
- std::is_scalar<T>
- std::is_trivially_copyable<T>
- std::is_base_of<Base, Derived>
- std::make_unsigned<T>
- std::enable_if<condition, type>
- std::conditional<condition, T, U>

## Implementation Phases

### Phase 1: Type Properties System (2-3 iterations)

Add TypeProperties to CppType:
```rust
pub struct TypeProperties {
    pub is_signed: bool,
    pub is_integral: bool,
    pub is_scalar: bool,
    pub is_trivially_copyable: bool,
    pub is_pointer: bool,
    pub is_reference: bool,
}

impl CppType {
    pub fn properties(&self) -> TypeProperties { ... }
}
```

Files:
- `crates/fragile-clang/src/types.rs` - Add properties() method

### Phase 2: Type Trait Expression Parsing (2 iterations)

Add AST nodes for type trait expressions:
```rust
/// Type trait expression (e.g., std::is_integral<T>::value)
TypeTraitExpr {
    trait_name: String,  // "is_integral", "is_signed", etc.
    type_arg: CppType,
},
```

Files:
- `crates/fragile-clang/src/ast.rs` - Add TypeTraitExpr variant
- `crates/fragile-clang/src/parse.rs` - Parse type trait expressions

### Phase 3: std::enable_if Support (2-3 iterations)

Handle enable_if in template parameter context:
```rust
fn try_enable_if(
    condition: bool,
    return_type: CppType,
) -> Option<CppType> {
    if condition {
        Some(return_type)
    } else {
        None  // SFINAE: substitution failure
    }
}
```

Files:
- `crates/fragile-clang/src/deduce.rs` - Add SFINAE evaluation
- `crates/fragile-clang/src/convert.rs` - Handle enable_if in templates

### Phase 4: std::conditional Support (1-2 iterations)

Implement conditional type selection:
```rust
fn resolve_conditional(
    condition: bool,
    true_type: CppType,
    false_type: CppType,
) -> CppType {
    if condition { true_type } else { false_type }
}
```

### Phase 5: Overload Resolution with SFINAE (3-4 iterations)

Update overload resolution to handle SFINAE failures:
```rust
fn resolve_overload(
    candidates: Vec<FunctionTemplate>,
    call_args: &[CppType],
) -> Result<FunctionTemplate, OverloadError> {
    let mut viable = Vec::new();
    for candidate in candidates {
        match try_instantiate(&candidate, call_args) {
            Ok(inst) => viable.push(inst),
            Err(SubstitutionFailure) => continue,  // SFINAE
        }
    }
    select_best(viable)
}
```

## Files to Modify
- `crates/fragile-clang/src/types.rs` - Type properties
- `crates/fragile-clang/src/ast.rs` - Type trait AST nodes
- `crates/fragile-clang/src/parse.rs` - Parse type traits
- `crates/fragile-clang/src/deduce.rs` - SFINAE evaluation
- `crates/fragile-clang/src/convert.rs` - MIR generation
- `crates/fragile-clang/tests/integration_test.rs` - Tests

## Test Cases
1. `test_type_properties_integral` - is_integral for int/double
2. `test_enable_if_basic` - Simple enable_if constraint
3. `test_conditional_type` - std::conditional type selection
4. `test_sfinae_overload_resolution` - SFINAE removes bad overloads

## Estimated Size
~1000-1500 lines of code total

## Dependencies
- [x] Function templates
- [x] Template specialization
- [x] Type deduction
- [x] Class templates

## Recommendation
Due to complexity, consider whether full SFINAE is needed for current goals.
Alternative: Implement only the type traits evaluation, defer overload resolution.

