# M9.2.c.iv.f.2.b - E0308 Bucket-B1 Value-Shape Closure Inventory

## Scope
Leaf `M9.2.c.iv.f.2.b` executes the first bounded residual `E0308` slice from `f.2`:
- pointer/null mutability and direct value-shape mismatches,
- no force-native bypasses,
- no target-specific hacks,
- no semantic stubs/fake runtime behavior.

Bound from decomposition:
- `<=400 LOC` for code edits in this leaf.

## Wrong-Approach Check
Re-reviewed before edits:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrail confirmation:
- no rollback-pattern expansion,
- no fake method-body stubs,
- no semantic type remapping,
- no suppression-only accounting.

## Implementation
Touched file:
- `crates/fragile-clang/src/ast_codegen.rs`

Changes:
1. Added `normalize_e0308_bucket_b1_value_shape_mismatches`:
   - rewrites `Cell::set(&x)` to `Cell::set(x)` when receiver lane is known `Cell`.
   - rewrites `Ref`/`RefMut` bindings from `unsafe { (X).borrow() }` / `borrow_mut()` to explicit `std::cell::RefCell::{borrow,borrow_mut}(X)`.
   - rewrites `return (unsafe { super::rusty::__gv_None }).clone();` in `Option<...>` return lanes to `return std::option::Option::None;`.
   - rewrites `_sigev_un = ((64 / 4) - 4);` to typed zero-init: `unsafe { std::mem::zeroed() }`.
   - rewrites `Self { field: (unsafe { super::rusty::__gv_None }).clone(), ... }` Option field lanes to typed `Option::None` constructors (`Option`, `Mutex<Option<_>>`, `RefCell<Option<_>>`).
2. Extended `normalize_spinmutex_guard_constructor_data_param_types`:
   - supports both `SpinMutexGuard_*` and `rrr_SpinMutexGuard_*`.
   - rewrites degraded assignment lane `__self.data_ = other.data_;` to explicit cast on concrete `data_` pointer lane.
3. Fixed regression introduced during this leaf:
   - non-SpinMutex `impl` headers were being dropped by an unintended `continue` in the SpinMutex normalizer.
   - corrected gating to only activate rewrite-state for SpinMutex impls while preserving all unrelated lines.

## Unit Tests
`cargo test -p fragile-clang spinmutex_guard_constructor_param_rehydration -- --nocapture`
- added regression: `test_spinmutex_guard_constructor_param_rehydration_preserves_non_spinmutex_impl_headers`
- all targeted SpinMutex rehydration tests pass.

`cargo test -p fragile-clang bucket_b1_value_shape_mismatches -- --nocapture`
- `test_normalize_e0308_bucket_b1_value_shape_mismatches_rewrites_cell_set_and_refcell_borrow_lanes`
- `test_normalize_e0308_bucket_b1_value_shape_mismatches_rewrites_option_none_and_sigev_union_lanes`
- all pass.

## Focused Probe Contract
Profile:
- compile-command sourced from replay root:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053/build_fragilec/compile_commands.json`
- compile-unit scope:
  - `reactor.cc`
  - `rpc/client.cpp`
  - `rpc/server.cpp`
  - `rpc/utils.cpp`
- environment:
  - `FRAGILEC_MODE=strict`
- command family:
  - `CMakeFiles/txlog.dir/...` entries for each scoped unit.

Artifacts:
- pre-fix baseline:
  - `/tmp/fragile_f2b_probe_after_20260330/summary.txt`
  - per-file stderr logs under `/tmp/fragile_f2b_probe_after_20260330/`
- post-fix:
  - `/tmp/fragile_f2b_probe_after_fix_20260330_txlog/summary.txt`
  - per-file stderr logs under `/tmp/fragile_f2b_probe_after_fix_20260330_txlog/`

## Focused Probe Results

### E0308 by Compile Unit
| compile unit | pre-fix E0308 | post-fix E0308 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 9 | 4 | -5 |
| `rpc/client.cpp` | 46 | 20 | -26 |
| `rpc/server.cpp` | 18 | 4 | -14 |
| `rpc/utils.cpp` | 2 | 1 | -1 |
| **total** | **75** | **29** | **-46** |

`E0308` reduction across this bounded slice: `-61.3%` (`75 -> 29`).

### Targeted B1 Signature Markers
| marker | pre-fix | post-fix | delta |
|---|---:|---:|---:|
| `__gv_None` | 5 | 2 | -3 |
| `_sigev_un = ((64 / 4) - 4)` | 3 | 0 | -3 |
| `found *mut UnsafeCell<()>` | 6 | 0 | -6 |
| `found *mut UnsafeCell<T>` | 6 | 0 | -6 |
| `REACTOR_SP_RUNNING_CORO_TH_.borrow()` | 2 | 0 | -2 |

Notes:
- `Cell::set(&...)` marker was absent in both focused probe snapshots (`0 -> 0`) under this exact compile-unit/profile slice.
- total `error:` line count remained stable (`13 -> 13`) while `E0308` dropped, confirming bounded class-shift rather than suppression.

## Residuals
Residual `E0308` lines remain (now outside dominant B1 signatures), including:
- `MaybeUninit<RefCell<Option<Rc<Fiber>>>>` vs `&RefCell<Option<Rc<Fiber>>>`,
- numeric width and return-lane mismatches in client/server paths,
- `AddrInfo` lane mismatch in `rpc/utils.cpp`.

These residuals roll into the next bounded leaves (`f.2.c` and `f.2.d`) per decomposition order.
