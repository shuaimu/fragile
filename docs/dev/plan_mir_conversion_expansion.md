# Plan: MIR Conversion Expansion

## Overview

Expand the MIR conversion in `fragile-rustc-driver/src/mir_convert.rs` to convert actual C++ MIR from fragile-clang into rustc's internal MIR format.

## Status: COMPLETE

All MIR conversion functions are implemented. TLS infrastructure for rustc integration is wired up.

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

## Integration Status

### Thread-Local Storage (TLS) Wiring ✅

The infrastructure for passing state to rustc query callbacks is complete:

1. **TLS Variables** (`rustc_integration.rs`):
   - `CPP_REGISTRY`: Stores `Arc<CppMirRegistry>` for function lookup
   - `CPP_FUNCTION_NAMES`: Stores `HashSet<String>` for quick name lookup

2. **Lifecycle Management**:
   - `set_cpp_registry()`: Called in `FragileCallbacks::config()` before compilation
   - `clear_cpp_registry()`: Called in `run_rustc()` after compilation

3. **Query Override**:
   - `override_queries_callback`: Installs custom `mir_built` query provider
   - Full MIR injection implemented with arena allocation

### mir_built Query Override ✅

Full MIR injection is now implemented:

1. **Function Detection**: Check `#[link_name]` attribute against registered C++ function names
2. **MIR Lookup**: Retrieve fragile MIR body from `CppMirRegistry` via TLS
3. **MIR Conversion**: Use `MirConvertCtx::convert_mir_body_full()` to convert to rustc MIR
4. **Arena Allocation**: Allocate body via `tcx.arena.alloc(Steal::new(body))`
5. **Fallback**: Non-C++ functions use original `mir_built` provider

### Remaining Work

1. **Generic Parameters**: Handle function templates with type substitution
2. **Borrow Check Bypass**: Override `mir_borrowck` for C++ functions (optional, since extern fns don't require it)

### End-to-End Testing

For full integration testing (requires nightly + rustc-dev):

1. Compile C++ file → MIR
2. Generate Rust stubs
3. Compile mixed binary with rustc
4. Execute and verify

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
