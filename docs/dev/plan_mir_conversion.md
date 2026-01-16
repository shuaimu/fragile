# Plan: MIR Format Conversion (Task 2.3.3.2)

## Overview

Convert Fragile's simplified MIR representation (`fragile_clang::MirBody`) to rustc's internal MIR format (`rustc_middle::mir::Body`).

## Current Fragile MIR Structure

```rust
// From fragile-clang/src/lib.rs
pub struct MirBody {
    pub blocks: Vec<MirBasicBlock>,
    pub locals: Vec<MirLocal>,
    pub is_coroutine: bool,
}

pub struct MirBasicBlock {
    pub statements: Vec<MirStatement>,
    pub terminator: MirTerminator,
    pub is_cleanup: bool,
}

pub enum MirStatement {
    Assign { target: MirPlace, value: MirRvalue },
    Nop,
}

pub enum MirTerminator {
    Return,
    Goto { target: usize },
    SwitchInt { operand, targets, otherwise },
    Call { func, args, destination, target, unwind },
    Unreachable,
    Resume,
    Yield { value, resume, drop },
    Await { awaitable, destination, resume, drop },
    CoroutineReturn { value },
}

pub enum MirRvalue {
    Use(MirOperand),
    BinaryOp { op, left, right },
    UnaryOp { op, operand },
    Ref { place, mutability },
}

pub enum MirOperand {
    Copy(MirPlace),
    Move(MirPlace),
    Constant(MirConstant),
}

pub struct MirPlace {
    pub local: usize,
    pub projection: Vec<MirProjection>,
}

pub struct MirLocal {
    pub name: Option<String>,
    pub ty: CppType,
    pub is_arg: bool,
}
```

## rustc MIR Structure (simplified)

```rust
// From rustc_middle::mir
pub struct Body<'tcx> {
    pub basic_blocks: IndexVec<BasicBlock, BasicBlockData<'tcx>>,
    pub local_decls: IndexVec<Local, LocalDecl<'tcx>>,
    pub arg_count: usize,
    pub source_scopes: IndexVec<SourceScope, SourceScopeData<'tcx>>,
    pub span: Span,
    // ... many more fields
}

pub struct BasicBlockData<'tcx> {
    pub statements: Vec<Statement<'tcx>>,
    pub terminator: Option<Terminator<'tcx>>,
    pub is_cleanup: bool,
}

pub struct Statement<'tcx> {
    pub source_info: SourceInfo,
    pub kind: StatementKind<'tcx>,
}

pub struct Terminator<'tcx> {
    pub source_info: SourceInfo,
    pub kind: TerminatorKind<'tcx>,
}
```

## Conversion Challenges

### 1. Indexed Types
- rustc uses `IndexVec` with `newtype_index!` indices (`BasicBlock`, `Local`)
- Our code uses simple `usize`

### 2. Interned Types
- rustc uses `Ty<'tcx>` which is an interned pointer
- We use `CppType` which is a plain enum
- Requires `TyCtxt` to create `Ty` values

### 3. Source Information
- rustc requires `SourceInfo` (span + scope) for all statements/terminators
- We don't track source locations
- Need to create dummy spans

### 4. Missing Fields
- rustc `Body` has many fields we don't populate
- Need to understand defaults and required values

## Implementation Plan

### Phase 1: Minimal Conversion (~100 LOC)
Create a conversion that produces valid but minimal MIR:

1. **Create empty Body shell** with required fields:
   - `local_decls` from our `locals`
   - `basic_blocks` from our `blocks`
   - Dummy `source_scopes` and `span`

2. **Convert basic types**:
   - `usize` indices → `BasicBlock`, `Local` newtypes
   - `CppType` → `Ty<'tcx>` (Task 2.3.3.3)

3. **Convert statements**:
   - `MirStatement::Assign` → `StatementKind::Assign`
   - `MirStatement::Nop` → `StatementKind::Nop`

4. **Convert terminators**:
   - `MirTerminator::Return` → `TerminatorKind::Return`
   - `MirTerminator::Goto` → `TerminatorKind::Goto`
   - etc.

### Phase 2: Full Conversion (~100 LOC more)
Add support for:
- All terminator variants
- All rvalue variants
- Coroutine support (if needed)

## Dependencies

- Task 2.3.3.3 (Type conversion) is needed for `Ty<'tcx>` creation
- Can be developed in parallel with stubbed types

## Code Location

New file: `crates/fragile-rustc-driver/src/mir_convert.rs`

## Testing Strategy

1. Unit tests with hand-crafted MirBody
2. Integration test: parse C++ → generate MIR → convert → verify structure

## Estimated Effort

| Component | LOC | Notes |
|-----------|-----|-------|
| Body shell creation | ~30 | Index conversion, dummy fields |
| Statement conversion | ~40 | Assign, Nop |
| Terminator conversion | ~80 | Multiple variants |
| Place/Operand conversion | ~30 | Recursive structures |
| Type conversion | ~100 | Task 2.3.3.3 |
| **Total** | ~280 | |

## Alternative Approach

Instead of full MIR conversion, we could:
1. Generate C++ as actual native code (using LLVM)
2. Link the native code with Rust
3. Use `extern "C"` stubs without MIR injection

This would be simpler but loses the "zero FFI overhead" goal.
