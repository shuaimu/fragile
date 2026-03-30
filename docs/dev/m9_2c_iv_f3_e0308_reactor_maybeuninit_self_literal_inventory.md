# M9.2.c.iv.f.3 - E0308 Reactor MaybeUninit + Self-Literal Closure Inventory

## Scope
Leaf `M9.2.c.iv.f.3` executes the first bounded residual-fix slice selected by `f.2.e` dominant bucket evidence (`E0308`).

Bounded objective:
- close the highest-signal reactor-lane `E0308` value-shape mismatches without introducing target-specific hacks,
- capture focused four-unit txlog compile-probe before/after evidence.

Bounded size:
- `<1000 LOC` total (single normalization slice + focused tests + docs/tests updates).

## Wrong-Approach Check
Re-reviewed before edits:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrail confirmation:
- no force-native bypass,
- no target-specific hacks,
- no semantic stubs/fake runtime behavior,
- no suppression-only accounting.

## Implementation
Touched file:
- `crates/fragile-clang/src/ast_codegen.rs`

Bounded normalization updates in `normalize_e0308_bucket_b1_value_shape_mismatches`:

1. MaybeUninit RefCell borrow-lane rehydration
- Added static-binding discovery for `static mut NAME: MaybeUninit<RefCell<...>>`.
- Rewrote:
  - `std::cell::RefCell::borrow(NAME)` -> `std::cell::RefCell::borrow(unsafe { NAME.assume_init_ref() })`
  - `std::cell::RefCell::borrow_mut(NAME)` -> `std::cell::RefCell::borrow_mut(unsafe { NAME.assume_init_mut() })`
- This closes reactor-lane mismatches where `RefCell::borrow` expected `&RefCell<_>` but received `MaybeUninit<RefCell<_>>`.

2. Direct `Self { ... }` literal lane support for `__gv_None`
- Expanded self-literal detection beyond `let mut __self = Self { ... }` to include direct `Self { ... }` constructor returns.
- Existing typed `__gv_None` field-lane rewrite now also applies to direct literals, rehydrating:
  - `Option` fields to `Option::None`
  - `Mutex<Option<_>>` fields to `Mutex::new(Option::None)`
- This closes reactor `join_handle_` type mismatch (`Mutex<Option<JoinHandle<()>>>` vs unit from `__gv_None`).

## Focused Unit Tests
Command:
- `cargo test -p fragile-clang normalize_e0308_bucket_b1_value_shape_mismatches -- --nocapture`

Added/covered tests:
- `test_normalize_e0308_bucket_b1_value_shape_mismatches_rewrites_maybeuninit_refcell_borrow_lanes`
- `test_normalize_e0308_bucket_b1_value_shape_mismatches_rewrites_direct_self_literal_none_lane`
- existing B1 tests remained green.

Result: pass.

## Focused Compile-Probe Contract
Compile-command source:
- `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053/build_fragilec/compile_commands.json`

Scoped txlog compile units:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

Environment:
- `FRAGILEC_MODE=strict`

Artifacts:
- pre-f.3 baseline:
  - `/tmp/fragile_f2e_probe_after_20260330T104700Z_txlog/summary.txt`
- post-f.3 probe:
  - `/tmp/fragile_f3_probe_after_20260330T123349Z_txlog/summary.txt`

## Probe Results

### E0308 by Compile Unit
| compile unit | pre-f.3 E0308 | post-f.3 E0308 | delta |
|---|---:|---:|---:|
| `reactor.cc` | 4 | 1 | -3 |
| `rpc/client.cpp` | 20 | 19 | -1 |
| `rpc/server.cpp` | 4 | 4 | 0 |
| `rpc/utils.cpp` | 1 | 1 | 0 |
| **total** | **29** | **25** | **-4** |

### Other Typed Buckets (non-increase check)
| bucket | pre-f.3 total | post-f.3 total | delta |
|---|---:|---:|---:|
| `E0599` | 27 | 27 | 0 |
| `E0282` | 1 | 1 | 0 |
| `E0605` | 0 | 0 | 0 |

### Scoped `total_error_lines`
- pre-f.3: `145`
- post-f.3: `141`
- delta: `-4`

## Reactor-Lane Marker Closure
Reactor stderr comparison:
- before: `/tmp/fragile_f2e_probe_after_20260330T104700Z_txlog/reactor.cc.stderr.log`
- after: `/tmp/fragile_f3_probe_after_20260330T123349Z_txlog/reactor.cc.stderr.log`

Cleared `E0308` markers:
- `RefCell::borrow(REACTOR_SP_RUNNING_CORO_TH_)` expected `&RefCell<_>` found `MaybeUninit<RefCell<_>>` (2 lanes).
- `join_handle_: (unsafe { super::rusty::__gv_None }).clone()` expected `Mutex<Option<JoinHandle<()>>>` found unit (1 lane).

Residual reactor `E0308`:
- `__thread_id::new_1(poll_tid)` expected `u64`, found `std___thread_id` (1 lane).

## Residual Ownership
`f.3` closes the first bounded `E0308` slice and leaves remaining dominant residual ownership to `M9.2.c.iv.f.4` replay+next decomposition/closure flow.
