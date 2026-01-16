# Plan: Basic Type Deduction for Simple Types

## Task
Implement template argument deduction for simple types (T → int, T → double).

## Background
When calling a function template like `identity<T>(T x)` without explicit template
arguments, the compiler must deduce T from the argument types:
- `identity(42)` → T = int
- `identity(3.14)` → T = double

## Implementation

### 1. Add TypeDeducer Module
Create `crates/fragile-clang/src/deduce.rs`:

```rust
pub struct TypeDeducer {
    // Maps template param name to deduced type
    deductions: HashMap<String, CppType>,
}

impl TypeDeducer {
    pub fn deduce(
        template: &CppFunctionTemplate,
        arg_types: &[CppType],
    ) -> Result<HashMap<String, CppType>, DeductionError>;
}
```

### 2. Basic Deduction Algorithm
For simple cases, match param type against arg type:

```rust
fn deduce_from_types(
    param_type: &CppType,
    arg_type: &CppType,
    deductions: &mut HashMap<String, CppType>,
) -> Result<(), DeductionError> {
    match param_type {
        CppType::TemplateParam { name, .. } => {
            // Direct match: T ← concrete type
            if let Some(existing) = deductions.get(name) {
                // Check consistency
                if existing != arg_type {
                    return Err(DeductionError::Conflict { ... });
                }
            } else {
                deductions.insert(name.clone(), arg_type.clone());
            }
            Ok(())
        }
        // Non-dependent types must match exactly
        _ if !param_type.is_dependent() => {
            if param_type == arg_type {
                Ok(())
            } else {
                Err(DeductionError::TypeMismatch { ... })
            }
        }
        _ => Ok(()) // Skip complex cases for now
    }
}
```

### 3. Add Substitution Method
In `types.rs`, add method to substitute template params:

```rust
impl CppType {
    pub fn substitute(&self, substitutions: &HashMap<String, CppType>) -> CppType {
        match self {
            CppType::TemplateParam { name, .. } => {
                substitutions.get(name).cloned().unwrap_or_else(|| self.clone())
            }
            CppType::Pointer { pointee, is_const } => {
                CppType::Pointer {
                    pointee: Box::new(pointee.substitute(substitutions)),
                    is_const: *is_const,
                }
            }
            // ... other cases
            _ => self.clone()
        }
    }
}
```

### 4. Instantiation Support
Add template instantiation to CppFunctionTemplate:

```rust
impl CppFunctionTemplate {
    pub fn instantiate(
        &self,
        substitutions: &HashMap<String, CppType>,
    ) -> CppFunction {
        CppFunction {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            return_type: self.return_type.substitute(substitutions),
            params: self.params.iter()
                .map(|(n, t)| (n.clone(), t.substitute(substitutions)))
                .collect(),
            // ...
        }
    }
}
```

## Files to Modify
- `crates/fragile-clang/src/deduce.rs` - New file for deduction logic
- `crates/fragile-clang/src/types.rs` - Add substitute() method
- `crates/fragile-clang/src/lib.rs` - Add module, instantiate method
- `crates/fragile-clang/tests/integration_test.rs` - Add tests

## Test Cases
1. `test_deduce_simple_int` - T deduced from int argument
2. `test_deduce_simple_double` - T deduced from double argument
3. `test_deduce_multiple_params` - Multiple T usages must be consistent
4. `test_substitute_simple` - Substitute T = int in template type
5. `test_instantiate_function` - Full instantiation test

## Estimated Size
~150-200 lines of code (well under 500 LOC limit)
