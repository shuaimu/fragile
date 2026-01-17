# Plan: G.1.2 Full Type Conversion

## Overview

Enhance the `convert_type` function in `mir_convert.rs` to properly convert all C++ types to rustc types, including recursive pointee/referent conversion and special handling for arrays, named types, and function types.

## Current State

The current implementation handles:
- Primitive types (void, bool, char, short, int, long, long long, float, double) - ✅
- Pointers - uses `*const ()` placeholder
- References - uses `&()` / `&mut ()` placeholder

## Tasks

### 1. Recursive Pointer Type Conversion (~30 LOC)
- Convert `CppType::Pointer { pointee, is_const }` to proper `*const T` / `*mut T`
- Recursively convert pointee type
- Handle nested pointers (e.g., `int**`)

### 2. Recursive Reference Type Conversion (~30 LOC)
- Convert `CppType::Reference { referent, is_const, is_rvalue }` properly
- For FFI, references become raw pointers (Rust FFI convention)
- Rvalue references (T&&) become `*mut T` (ownership transfer semantics)
- Const lvalue references (const T&) become `*const T`

### 3. Array Type Conversion (~40 LOC)
- Fixed-size arrays `[T; N]` - use `rustc_middle::ty::Ty::new_array`
- Unsized arrays become pointers (for FFI)

### 4. Named Type Handling (~40 LOC)
- For known types (e.g., `size_t`, `uintptr_t`), map to Rust equivalents
- For struct/class types, use opaque pointers or look up in registry
- For enum types, map to appropriate integer type

### 5. Function Pointer Types (~30 LOC)
- Convert `CppType::Function { return_type, params, is_variadic }` to `fn(T1, T2) -> R`
- Handle variadic functions appropriately

### 6. Template Parameter Handling (~20 LOC)
- Template parameters should not reach MIR conversion (they should be instantiated)
- Add assert/warning for unexpected template params
- Use unit type as fallback

## Implementation

The key insight is that the conversion must be **recursive** - a pointer to int needs to:
1. Convert `int` → `i32`
2. Wrap in `Ty::new_ptr(..., i32, mutability)`

## Testing

Add unit tests in a separate integration test file that:
1. Creates a MirBody with various types
2. Converts using MirConvertCtx
3. Verifies the resulting rustc types are correct

## Risk Assessment

- **Low risk**: Pointer/reference/array recursion is straightforward
- **Medium risk**: Named types require external lookup (may need registry integration)
- **Low risk**: Function types follow established patterns

## Estimated LOC: ~200

This is within the ~200 LOC estimate from TODO.md.
