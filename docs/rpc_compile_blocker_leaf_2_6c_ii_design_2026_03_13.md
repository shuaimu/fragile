# RPC Compile Blocker Leaf 2.6.c.ii Design (2026-03-13)

## Scope

Leaf `2.6.c.ii`: implement a generic fix set for the first blocker class from `2.6.c.i`
(`build_timeout` on `src/rrr/base/misc.cpp`) and lock behavior with focused regressions.

## Baseline

From `2.6.c.i` baseline replay root `/tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313`:

- blocker class: `build_timeout`
- blocker file: `src/rrr/base/misc.cpp`
- replay timeout at `120s` and `300s`
- stage timing reached `codegen` but did not complete before timeout

## Decision

Apply generic codegen hot-path reductions in `normalize_problematic_callshape_artifacts`
without changing semantic intent:

1. Add a per-line marker gate (`line_might_need_problematic_callshape_bulk_rewrites`) so
   the heavy callshape replacement bundle only runs for candidate lines.
2. Tighten several broad markers to reduce over-triggering (`Fiber::` -> `Fiber::create_run`,
   `pthread_` -> `super::pthread_`, `LoadBalancer::` -> `LoadBalancer::select_`,
   `v_len` -> specific `v_len` callshape markers, etc.).
3. Gate expensive per-line post-rewriters so they only execute when needed:
   - `rewrite_static_unsafe_binding_clone`
   - `replace_vtable_ptr_calls`
   - `rewrite_vtable_null_cast`

## Wrong-Approach Check

Aligned with `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no RPC-target conditionals
- no native-compiler bypass
- no fake semantic method-body stubs
- no masking of timeout failures

## Implementation

Updated:

- `crates/fragile-clang/src/ast_codegen.rs`

Added focused regressions:

- `test_line_might_need_problematic_callshape_bulk_rewrites_matches_known_needles`
- `test_normalize_problematic_callshape_artifacts_rewrites_target_line_and_preserves_unrelated_line`

## Validation

Focused unit coverage:

- `cargo test -p fragile-clang problematic_callshape`

Replay evidence using baseline run root:

- `python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`

Stage traces:

- `/tmp/fragile_rpc_2_6c_ii_after_opt_replay120_timing.txt`
- `/tmp/fragile_rpc_2_6c_ii_after_opt_replay300_timing.txt`

Observed outcome:

- parse/enrichment complete deterministically and replay still times out in `codegen`
- blocker class remains `build_timeout` on `src/rrr/base/misc.cpp`

This leaf contributes a generic, regression-locked optimization pass and preserves
an honest blocker status for the next non-increase/replay loop (`2.6.c.iii`/`2.6.c.iv`).
