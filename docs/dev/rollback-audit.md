# Rollback Pattern Audit Report

**Date**: 2026-02-04 (Updated)
**Total Patterns**: 202
**Location**: `crates/fragile-clang/src/ast_codegen.rs`

## Overview

Rollback patterns are string-matching conditions that, when triggered, cause the transpiler to **delete the generated method body** instead of emitting it. This hides compilation errors but results in incomplete/broken APIs.

## Pattern Categories

### 1. Internal Field Access (37 patterns)

These patterns trigger when generated code accesses internal implementation fields that don't exist in the generated struct.

| Pattern | Count | Root Cause |
|---------|-------|------------|
| `._M_current` | 1 | Iterator internal field not generated |
| `._M_node` | 1 | Node pointer field not generated |
| `._M_t` | 1 | Tree implementation field missing |
| `._M_impl` | 1 | Allocator impl field missing |
| `._M_resource` | 1 | Resource field missing |
| `._M_alloc` | 1 | Allocator field missing |
| `._M_ptr` | 1 | Pointer field missing |
| `._M_max_size` | 1 | Max size field missing |
| `._M_f` | 1 | Function field missing |
| `._M_array` | 1 | Array field missing |
| `._M_state.*` | 6 | State machine fields missing |
| `.__ptr_` | 4+ | Internal pointer fields |
| `.__val_` | 4+ | Internal value fields |
| `.__i_` | 4+ | Internal index/iterator fields |
| `.__current_` | 2+ | Current position fields |
| `.__node_` | 1+ | Node pointer fields |
| Other `._M_*` | ~10 | Various internal fields |

**Root Cause**: When LibTooling extracts class definitions, it doesn't capture all internal fields from the template specialization. Fields declared in base classes or through inheritance may be missing.

**Fix Strategy**:
1. Audit LibTooling field extraction in `fragile-ast-exporter`
2. Ensure recursive extraction of fields from base classes
3. Handle template-dependent field declarations

### 2. c_void Type Issues (16 patterns)

These patterns trigger when template type parameters resolve to `c_void` instead of their actual types.

| Pattern | Count | Root Cause |
|---------|-------|------------|
| `+ c_void` | 2 | Pointer arithmetic with void |
| `c_void +` | 2 | Void in arithmetic expression |
| `c_void::new_` | 2 | Constructing void type |
| `c_void + &mut` | 1 | Void with mutable reference |
| Others | ~9 | Various c_void misuse |

**Root Cause**: Template parameter deduction fails, defaulting to `c_void` (void pointer) instead of the actual instantiated type. This happens when:
- Type aliases aren't resolved
- Dependent types aren't substituted
- decltype() expressions aren't evaluated

**Fix Strategy**:
1. Improve template type deduction in `types.rs`
2. Resolve type aliases before emission
3. Substitute dependent types with concrete types

### 3. VTable Issues (10 patterns)

These patterns trigger when generated code references virtual tables.

| Pattern | Count | Root Cause |
|---------|-------|------------|
| `.__vtable = &STD_COLLATE_*` | 3 | Collate vtable assignment |
| `.__vtable = &STD_CTYPE_*` | 4 | Ctype vtable assignment |
| `.__vtable = &STD_*` | 3 | Other locale vtables |

**Root Cause**: C++ virtual table initialization in constructors is being transpiled literally instead of being handled as a special case. Rust doesn't have vtables in the same way.

**Fix Strategy**:
1. Skip vtable initialization in constructor transpilation
2. Or: Generate Rust trait vtables that match the C++ layout
3. Or: Use a different approach for virtual dispatch

### 4. Builtin/Intrinsic Calls (8 patterns)

These patterns trigger when code calls compiler builtins or libc++ intrinsics.

| Pattern | Count | Root Cause |
|---------|-------|------------|
| `__builtin_operator_delete` | 1 | Memory deallocation |
| `__builtin_operator_new` | 1 | Memory allocation |
| `__libcpp_deallocate` | 1 | libc++ deallocation |
| `__cxx_atomic_store` | 1 | Atomic operations |
| `__to_address` | 1 | Pointer conversion |
| Others | ~3 | Various intrinsics |

**Root Cause**: These are compiler-specific intrinsics that need to be mapped to Rust equivalents.

**Fix Strategy**:
1. Map `__builtin_operator_new` → `std::alloc::alloc`
2. Map `__builtin_operator_delete` → `std::alloc::dealloc`
3. Map `__cxx_atomic_*` → `std::sync::atomic::*`
4. Map `__to_address` → raw pointer operations

### 5. Other Issues (133 patterns)

Miscellaneous patterns covering various edge cases:

| Category | Count | Examples |
|----------|-------|----------|
| Type casting issues | ~20 | `as duration__Rep`, `as _TreeIterator` |
| Placeholder types | ~15 | `DefaultType`, `_unnamed` |
| Return statement issues | ~10 | `return _Size + 0`, `return 0 /*...*/` |
| Arithmetic issues | ~10 | `*0`, `*1`, `+ 0` |
| Template-specific | ~30 | Various template instantiation issues |
| Iterator issues | ~20 | `counted_iterator`, `__wrap_iter` |
| Threading/sync | ~15 | `sem_init`, `pthread_*`, `gthread_*` |
| Other | ~13 | Various edge cases |

## Line Locations in ast_codegen.rs

The rollback patterns are concentrated in these areas:

1. **Lines ~2921-3100**: Main rollback check block (template method bodies)
2. **Lines ~3457-3600**: Secondary rollback checks (specific types)
3. **Lines ~9352-9600**: Additional pattern matching
4. **Lines ~14140-14200**: Final cleanup patterns

## Priority Order for Fixes

1. **High Priority**: Internal field access (37) - Blocks most container methods
2. **High Priority**: c_void type issues (16) - Causes type mismatches everywhere
3. **Medium Priority**: Builtin calls (8) - Needed for memory management
4. **Medium Priority**: VTable issues (10) - Needed for inheritance
5. **Lower Priority**: Other issues (133) - Mixed severity

## Metrics Tracking

Current state: **202 patterns**

To track progress:
```bash
grep -c "|| generated.contains" crates/fragile-clang/src/ast_codegen.rs
```

Target: **0 patterns**

Rule: Pattern count must decrease or stay same with every commit. NEVER increase.

## Next Steps

1. Start with 27.8.1.2: Fix internal field access patterns
2. Verify each fix removes patterns without breaking existing tests
3. Add new tests for each fixed category
4. Continue through remaining categories
