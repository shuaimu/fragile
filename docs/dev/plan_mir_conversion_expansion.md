# Plan: MIR Conversion Expansion

## Overview

Expand the MIR conversion in `fragile-rustc-driver/src/mir_convert.rs` to convert actual C++ MIR from fragile-clang into rustc's internal MIR format.

## Status: IN PROGRESS

## Current State

The current `mir_convert.rs` creates trivial bodies with just a return. This needs to be expanded to:
1. Convert actual statements and terminators
2. Handle all MIR constructs from fragile-clang
3. Support proper type conversion

## Tasks Breakdown

### Task 1: Local Variable Conversion (~50 LOC)
Convert `Vec<MirLocal>` to `IndexVec<Local, LocalDecl>`:
- Map `MirLocal.ty` via `convert_type()`
- Handle `is_arg` flag (arguments are first N locals)
- Preserve debug names when available

### Task 2: Statement Conversion (~60 LOC)
Convert `MirStatement` to `rustc_middle::mir::Statement`:
- `Assign { target, value }` → `StatementKind::Assign(place, rvalue)`
- `Nop` → `StatementKind::Nop`

Requires:
- Task 3 (Place conversion)
- Task 4 (Rvalue conversion)

### Task 3: Place Conversion (~40 LOC)
Convert `MirPlace` to `rustc_middle::mir::Place`:
- `local` index → `Local`
- `projection` → `ProjectionElem` list
  - `Deref` → `ProjectionElem::Deref`
  - `Field(n)` → `ProjectionElem::Field(Field, Ty)` (needs field type lookup)
  - `Index(n)` → `ProjectionElem::Index(Local)`

### Task 4: Rvalue Conversion (~50 LOC)
Convert `MirRvalue` to `rustc_middle::mir::Rvalue`:
- `Use(operand)` → `Rvalue::Use(operand)`
- `BinaryOp { op, left, right }` → `Rvalue::BinaryOp(op, operand, operand)`
- `UnaryOp { op, operand }` → `Rvalue::UnaryOp(op, operand)`
- `Ref { place, mutability }` → `Rvalue::Ref(region, kind, place)`

Requires:
- Task 5 (Operand conversion)
- Task 6 (BinOp/UnaryOp conversion)

### Task 5: Operand Conversion (~30 LOC)
Convert `MirOperand` to `rustc_middle::mir::Operand`:
- `Copy(place)` → `Operand::Copy(place)`
- `Move(place)` → `Operand::Move(place)`
- `Constant(c)` → `Operand::Constant(const_val)`

Requires:
- Task 3 (Place conversion)
- Task 7 (Constant conversion)

### Task 6: Binary/Unary Op Conversion (~30 LOC)
Convert `MirBinOp` and `MirUnaryOp` to rustc equivalents:
- `Add` → `BinOp::Add`
- `Sub` → `BinOp::Sub`
- etc.

### Task 7: Constant Conversion (~40 LOC)
Convert `MirConstant` to `rustc_middle::ty::Const`:
- `Int { value, bits }` → integer constant
- `Float { value, bits }` → float constant
- `Bool(b)` → bool constant
- `Unit` → unit constant

### Task 8: Terminator Conversion (~100 LOC)
Convert `MirTerminator` to `rustc_middle::mir::Terminator`:
- `Return` → `TerminatorKind::Return`
- `Goto { target }` → `TerminatorKind::Goto { target }`
- `SwitchInt { operand, targets, otherwise }` → `TerminatorKind::SwitchInt { .. }`
- `Call { func, args, destination, target, unwind }` → `TerminatorKind::Call { .. }`
- `Unreachable` → `TerminatorKind::Unreachable`
- `Resume` → `TerminatorKind::UnwindResume`

Coroutine terminators (lower priority):
- `Yield` → `TerminatorKind::Yield`
- `Await` → Custom handling
- `CoroutineReturn` → Custom handling

### Task 9: Basic Block Assembly (~30 LOC)
Assemble `MirBasicBlock` into `BasicBlockData`:
- Convert statements via Task 2
- Convert terminator via Task 8
- Handle `is_cleanup` flag

### Task 10: Integration and Testing (~50 LOC)
- Wire up full conversion path
- Add integration tests with simple C++ functions
- Test basic operations: add, arithmetic, control flow

## Estimated Total: ~480 LOC

## Priority Order

1. **Task 6** - BinOp/UnaryOp (simplest, no deps)
2. **Task 7** - Constants (simple, no deps)
3. **Task 3** - Places (needed by most others)
4. **Task 5** - Operands (needs Task 3, 7)
5. **Task 4** - Rvalues (needs Task 5, 6)
6. **Task 2** - Statements (needs Task 3, 4)
7. **Task 8** - Terminators (needs Task 5)
8. **Task 1** - Locals (independent but affects block assembly)
9. **Task 9** - Basic blocks (needs all above)
10. **Task 10** - Integration (final step)

## Dependencies

This work depends on:
- rustc nightly with `rustc-dev` component
- `#![feature(rustc_private)]` enabled
- Understanding of rustc MIR internals

## Testing Strategy

1. Unit tests for each converter function
2. Integration test: simple `add(a, b)` function
3. Integration test: control flow (if/else)
4. Integration test: loops
5. End-to-end: compile and run mixed Rust/C++ binary
