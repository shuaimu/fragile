# Plan: Integer Literal Support (Task 2.1)

## Date: 2026-01-17
## Status: COMPLETED

## Problem Statement

Integer literals in the C++ → MIR pipeline lose type information:
- Parser extracts value but discards the Clang type
- Converter hardcodes `bits: 32` for all integer literals
- This causes incorrect codegen for unsigned and non-32-bit integers

## Current Flow (Broken)

```
C++ Source: unsigned int x = 4294967295;
    ↓
Parser: IntegerLiteral(4294967295)  // Type discarded!
    ↓
Converter: MirConstant::Int { value: 4294967295, bits: 32 }  // Hardcoded!
    ↓
rustc: ScalarInt::try_from_int(4294967295 as i128, Size::from_bits(32))
    ↓
Result: Interpreted as i32, wraps to -1 (WRONG!)
```

## Target Flow (Fixed)

```
C++ Source: unsigned int x = 4294967295;
    ↓
Parser: IntegerLiteral { value: 4294967295, cpp_type: CppType::Int { signed: false } }
    ↓
Converter: MirConstant::Int { value: 4294967295, bits: 32, signed: false }
    ↓
rustc: ScalarInt::try_from_uint(4294967295, Size::from_bits(32))
    ↓
Result: Correct u32 value (4294967295)
```

## Implementation Steps

### Step 1: Add signedness to MirConstant::Int

File: `crates/fragile-clang/src/lib.rs`

Current:
```rust
pub enum MirConstant {
    Int { value: i128, bits: u32 },
    ...
}
```

Change to:
```rust
pub enum MirConstant {
    Int { value: i128, bits: u32, signed: bool },
    ...
}
```

### Step 2: Add bit_width() and is_signed() to CppType

File: `crates/fragile-clang/src/types.rs`

Add methods:
```rust
impl CppType {
    pub fn bit_width(&self) -> Option<u32> {
        match self {
            CppType::Bool => Some(8),  // Rust bool is 1-byte
            CppType::Char { .. } => Some(8),
            CppType::Short { .. } => Some(16),
            CppType::Int { .. } => Some(32),
            CppType::Long { .. } => Some(64),  // Assume LP64
            CppType::LongLong { .. } => Some(64),
            _ => None,
        }
    }

    pub fn is_signed(&self) -> Option<bool> {
        match self {
            CppType::Bool => Some(false),  // Unsigned
            CppType::Char { signed } => Some(*signed),
            CppType::Short { signed } => Some(*signed),
            CppType::Int { signed } => Some(*signed),
            CppType::Long { signed } => Some(*signed),
            CppType::LongLong { signed } => Some(*signed),
            _ => None,
        }
    }
}
```

### Step 3: Add type to IntegerLiteral AST node

File: `crates/fragile-clang/src/ast.rs`

Current:
```rust
IntegerLiteral(i128),
```

Change to:
```rust
IntegerLiteral {
    value: i128,
    cpp_type: Option<CppType>,
},
```

### Step 4: Capture type in parser

File: `crates/fragile-clang/src/parse.rs`

Current:
```rust
clang_sys::CXCursor_IntegerLiteral => {
    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
    if eval.is_null() {
        ClangNodeKind::IntegerLiteral(0)
    } else {
        let result = clang_sys::clang_EvalResult_getAsInt(eval) as i128;
        clang_sys::clang_EvalResult_dispose(eval);
        ClangNodeKind::IntegerLiteral(result)
    }
}
```

Change to:
```rust
clang_sys::CXCursor_IntegerLiteral => {
    let eval = clang_sys::clang_Cursor_Evaluate(cursor);
    let value = if eval.is_null() {
        0i128
    } else {
        let result = clang_sys::clang_EvalResult_getAsInt(eval) as i128;
        clang_sys::clang_EvalResult_dispose(eval);
        result
    };

    // Capture the type
    let clang_type = clang_sys::clang_getCursorType(cursor);
    let cpp_type = parse_clang_type(clang_type);  // Need to implement or call existing

    ClangNodeKind::IntegerLiteral { value, cpp_type }
}
```

### Step 5: Update converter to use actual type

File: `crates/fragile-clang/src/convert.rs`

Current:
```rust
ClangNodeKind::IntegerLiteral(value) => {
    Ok(MirOperand::Constant(MirConstant::Int {
        value: *value,
        bits: 32,
    }))
}
```

Change to:
```rust
ClangNodeKind::IntegerLiteral { value, cpp_type } => {
    let (bits, signed) = match cpp_type {
        Some(ty) => {
            let bits = ty.bit_width().unwrap_or(32);
            let signed = ty.is_signed().unwrap_or(true);
            (bits, signed)
        }
        None => (32, true),  // Default to i32
    };
    Ok(MirOperand::Constant(MirConstant::Int {
        value: *value,
        bits,
        signed,
    }))
}
```

### Step 6: Update rustc MIR conversion

File: `crates/fragile-rustc-driver/src/mir_convert.rs`

Update `convert_constant()` to handle signedness:
- Use `tcx.types.u8/u16/u32/u64/u128` for unsigned types
- Use `ScalarInt::try_from_uint()` for unsigned values

### Step 7: Update all call sites that construct MirConstant::Int

Search for all places that create `MirConstant::Int` and add the `signed` field.

## Test Plan

Create test file: `tests/clang_integration/test_integer_literals.cpp`

```cpp
// Test various integer literal types
int test_i32() { return 42; }
unsigned int test_u32() { return 4294967295u; }
long test_i64() { return 9223372036854775807L; }
unsigned long test_u64() { return 18446744073709551615UL; }
short test_i16() { return 32767; }
unsigned short test_u16() { return 65535u; }
```

Also add Rust integration test to verify correct values at runtime.

## Files Changed

1. `crates/fragile-clang/src/lib.rs` - MirConstant::Int signedness
2. `crates/fragile-clang/src/types.rs` - bit_width(), is_signed() methods
3. `crates/fragile-clang/src/ast.rs` - IntegerLiteral with type
4. `crates/fragile-clang/src/parse.rs` - Capture type for literals
5. `crates/fragile-clang/src/convert.rs` - Use actual type info
6. `crates/fragile-rustc-driver/src/mir_convert.rs` - Handle unsigned types

## Estimated LOC: ~150-200 lines (including tests)

---

## Implementation Notes (Completed 2026-01-17)

### Key Changes Made

1. **MirConstant::Int** now has a `signed: bool` field to distinguish signed vs unsigned
2. **CppType::bit_width()** returns the bit width for integer types (8/16/32/64) using LP64 model
3. **IntegerLiteral AST node** now stores `Option<CppType>` to preserve the Clang type info
4. **Parser** uses `clang_EvalResult_isUnsignedInt()`, `clang_EvalResult_getAsUnsigned()`, and `clang_EvalResult_getAsLongLong()` to correctly extract literal values (fixes sign extension issues)
5. **Converter** extracts bit width and signedness from the C++ type
6. **mir_convert.rs** now handles unsigned types using `tcx.types.u8/u16/u32/u64/u128` and `ScalarInt::try_from_uint()`

### Tests Added

- `test_integer_literal_type_int` - verifies `int` literal type (32-bit signed)
- `test_integer_literal_type_unsigned` - verifies `unsigned int` literal type (32-bit unsigned)
- `test_integer_literal_type_long` - verifies `long` literal type (64-bit signed)
- `test_integer_literal_type_unsigned_long` - verifies `unsigned long` literal type (64-bit unsigned)
- `test_bit_width_primitive_types` - unit tests for `CppType::bit_width()`
- `test_bit_width_pointer_and_reference` - unit tests for pointer/ref bit widths
- `test_bit_width_no_fixed_width` - unit tests for types without fixed bit width
- `test_is_signed_integer_types` - unit tests for `CppType::is_signed()`

### Log File

Test log: `logs/20260117_integer_literals_test_all.log`
