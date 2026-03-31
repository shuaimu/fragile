# M9.2.c.iv.f.6.c focused reactor probe non-increase inventory

Date: 2026-03-31
Owner task: `M9.2.c.iv.f.6.c`

## Scope

Re-run focused strict compile probes for reactor-family units and record:
- before/after unresolved-type invariant deltas (`f.6.b` -> `f.6.c`),
- non-increase across residual typed buckets.

Bounded execution: replay-only probe/accounting slice (no production code-path widening).

## Wrong-Approach Check

Reviewed before running probes:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- `docs/dev/wrong.md`.

Rejected approaches:
- forcing lane-green by introducing target-specific bypasses,
- muting unresolved-type invariant checks,
- changing compile arguments away from replay-root commands.

## Baseline (f.6.b)

Baseline probe roots from `M9.2.c.iv.f.6.b`:
- `/tmp/fragile_f6b_probe_single_20260331T051554Z`
- `/tmp/fragile_f6b_probe_single_20260331T051948Z`
- `/tmp/fragile_f6b_probe_single_20260331T052530Z`
- `/tmp/fragile_f6b_probe_single_20260331T053053Z`

Baseline aggregate:
- `aggregate_unresolved_invariant_count=0`
- `aggregate_rustc_compile_failed_count=3`
- `aggregate_E0308_count=3`
- `aggregate_E0425_count=0`
- `aggregate_E0599_count=0`
- `aggregate_E0609_count=0`
- `aggregate_E0277_count=0`
- `aggregate_E0061_count=0`

## f.6.c Focused Probe Rerun

Replay-root command source:
- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116/build_fragilec/compile_commands.json`
- selected command family: `CMakeFiles/rrr.dir/src/rrr/reactor/*.cc.o`

New probe batch:
- `/tmp/fragile_f6c_probe_batch_20260331T110557Z`

New per-unit probe roots:
- `/tmp/fragile_f6c_probe_single_20260331T110557Z_fiber_context_runtime`
- `/tmp/fragile_f6c_probe_single_20260331T110557Z_fiber_impl`
- `/tmp/fragile_f6c_probe_single_20260331T110557Z_quorum_event`
- `/tmp/fragile_f6c_probe_single_20260331T110557Z_event`

Per-unit summary:

| Unit | status | unresolved invariant count | rustc compile failed count | E0308 | E0425 | E0599 | E0609 | E0277 | E0061 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `fiber_context_runtime.cc` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` |
| `fiber_impl.cc` | `1` | `0` | `1` | `1` | `0` | `0` | `0` | `0` | `0` |
| `quorum_event.cc` | `1` | `0` | `1` | `1` | `0` | `0` | `0` | `0` | `0` |
| `event.cc` | `1` | `0` | `1` | `1` | `0` | `0` | `0` | `0` | `0` |

Aggregate after:
- `aggregate_unresolved_invariant_count=0`
- `aggregate_rustc_compile_failed_count=3`
- `aggregate_E0308_count=3`
- `aggregate_E0425_count=0`
- `aggregate_E0599_count=0`
- `aggregate_E0609_count=0`
- `aggregate_E0277_count=0`
- `aggregate_E0061_count=0`

## Before/After Delta and Non-Increase

`f.6.b` -> `f.6.c` aggregate delta:
- unresolved invariants: `0 -> 0` (`delta=0`, `non_increase=true`)
- rustc compile-failed markers: `3 -> 3` (`delta=0`, `non_increase=true`)
- `E0308`: `3 -> 3` (`delta=0`, `non_increase=true`)
- `E0425`: `0 -> 0` (`delta=0`, `non_increase=true`)
- `E0599`: `0 -> 0` (`delta=0`, `non_increase=true`)
- `E0609`: `0 -> 0` (`delta=0`, `non_increase=true`)
- `E0277`: `0 -> 0` (`delta=0`, `non_increase=true`)
- `E0061`: `0 -> 0` (`delta=0`, `non_increase=true`)

Outcome:
- unresolved-type invariant closure for `rrr_Future_State` remains stable at zero,
- residual typed tail remains stable (`E0308` only) and non-increasing.

## Evidence

- `TODO.md` closure entry for `M9.2.c.iv.f.6.c`
- `docs/fragile-dev-book.md` f.6.c entry
- `M9` closure tests in `crates/fragile-clang/tests/m9_rpc_closure_tests.rs`
- probe artifacts:
  - `/tmp/fragile_f6c_probe_batch_20260331T110557Z/rows.tsv`
  - `/tmp/fragile_f6c_probe_batch_20260331T110557Z/aggregate.txt`

Next leaf: `M9.2.c.iv.f.6.d`.
