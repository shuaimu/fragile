# Plan: Float Literal Support (Task 2.1.2)

## Date: 2026-01-17
## Status: COMPLETED

## Problem Statement

Float literals in the C++ → MIR pipeline are hardcoded to 64-bit (`f64`):
- Parser extracts value but discards the Clang type
- Converter hardcodes `bits: 64` for all float literals
- This causes incorrect codegen for `float` literals (which should be `f32`)

## Current Flow (Broken)

```
C++ Source: float x = 3.14f;
    ↓
Parser: FloatingLiteral(3.14)  // Type discarded, suffix lost!
    ↓
Converter: MirConstant::Float { value: 3.14, bits: 64 }  // Hardcoded!
    ↓
rustc: f64 type used instead of f32
```

## Target Flow (Fixed)

```
C++ Source: float x = 3.14f;
    ↓
Parser: FloatingLiteral { value: 3.14, cpp_type: CppType::Float }
    ↓
Converter: MirConstant::Float { value: 3.14, bits: 32 }
    ↓
rustc: f32 type correctly used
```

## Implementation Steps

### Step 1: Add cpp_type to FloatingLiteral AST node

File: `crates/fragile-clang/src/ast.rs`

Current:
```rust
FloatingLiteral(f64),
```

Change to:
```rust
FloatingLiteral {
    value: f64,
    cpp_type: Option<CppType>,
},
```

### Step 2: Capture type in parser

File: `crates/fragile-clang/src/parse.rs`

Add type capture similar to IntegerLiteral.

### Step 3: Update converter to use actual type

File: `crates/fragile-clang/src/convert.rs`

Use `CppType::bit_width()` to determine f32 vs f64.

### Step 4: Add tests

Add tests for:
- `float` literals (32-bit)
- `double` literals (64-bit)

## Files Changed

1. `crates/fragile-clang/src/ast.rs` - FloatingLiteral with type
2. `crates/fragile-clang/src/parse.rs` - Capture type for float literals
3. `crates/fragile-clang/src/convert.rs` - Use actual type info

## Estimated LOC: ~50-80 lines (including tests)
