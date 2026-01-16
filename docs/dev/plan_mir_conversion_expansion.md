# Plan: MIR Conversion Expansion

## Overview

Expand the MIR conversion in `fragile-rustc-driver/src/mir_convert.rs` to convert actual C++ MIR from fragile-clang into rustc's internal MIR format.

## Status: COMPLETE (except integration testing)

All MIR conversion functions are implemented. Integration testing remains.

## Completed Implementation

### MirConvertCtx Methods (mir_convert.rs)

| Method | LOC | Status |
|--------|-----|--------|
| `convert_binop` | 18 | ✅ Complete |
| `convert_unop` | 5 | ✅ Complete |
| `convert_constant` | 35 | ✅ Complete |
| `convert_place` | 30 | ✅ Complete |
| `convert_operand` | 15 | ✅ Complete |
| `convert_rvalue` | 25 | ✅ Complete |
| `convert_statement` | 15 | ✅ Complete |
| `convert_terminator` | 80 | ✅ Complete |
| `convert_local` | 3 | ✅ Complete |
| `convert_basic_block` | 15 | ✅ Complete |
| `convert_mir_body_full` | 50 | ✅ Complete |

**Total: ~290 LOC**

### Supported Conversions

#### BinOp
- Add, Sub, Mul, Div, Rem
- BitAnd, BitOr, BitXor, Shl, Shr
- Eq, Ne, Lt, Le, Gt, Ge

#### UnaryOp
- Neg, Not

#### Constants
- Int (8/16/32/64/128 bits)
- Float (32/64 bits)
- Bool
- Unit (void)

#### Places
- Local variables
- Projections: Deref, Field, Index

#### Rvalues
- Use (copy operand)
- BinaryOp
- UnaryOp
- Ref (borrow)

#### Statements
- Assign
- Nop

#### Terminators
- Return
- Goto
- SwitchInt
- Call (placeholder func, needs resolution)
- Unreachable
- UnwindResume
- Yield (coroutine)
- Await (coroutine, lowered to Yield)
- CoroutineReturn (lowered to Return)

## Remaining Work

### Integration Testing (Task 10)

To complete the integration:

1. **Wire up full compilation path**
   - Connect `convert_mir_body_full` to the rustc query override
   - Implement function call resolution (string → DefId)

2. **Add integration tests**
   - Simple arithmetic function
   - Control flow (if/else)
   - Loops

3. **End-to-end test**
   - Compile C++ file
   - Generate Rust stubs
   - Compile mixed binary
   - Execute and verify

## Notes

### Function Call Resolution

The `Call` terminator currently uses a placeholder function operand. Full implementation needs:
- Resolve function name string to DefId
- Handle C++ name mangling
- Support cross-module calls

### Coroutine Support

Coroutine terminators (Yield, Await, CoroutineReturn) are mapped to basic equivalents.
Full coroutine support requires:
- State machine generation
- Promise type handling
- Suspension point tracking

### Type Tracking

Field projections currently use `unit` as placeholder type. Full implementation needs:
- Track local variable types through conversion
- Lookup field types from struct definitions
- Handle generic types
