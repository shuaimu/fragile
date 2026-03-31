# M9.2.c.iv.f.6.b unresolved-type rehydration slice inventory

Date: 2026-03-31
Owner task: `M9.2.c.iv.f.6.b`

## Scope

Execute one bounded (`<=300 LOC`) unresolved-type rehydration slice for
`rrr_Future_State` in reactor-family units, with focused compile probes and no
unit-specific branching.

## Wrong-Approach Check

Reviewed before edits:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- `docs/dev/wrong.md` guardrails.

Rejected approaches:
- whitelisting `rrr_Future_State` in invariant filters without rehydration,
- target/file-name-specific conditionals for `event.cc`/`fiber_*`/`quorum_event.cc`,
- semantic stubs that skip type-lane closure.

## Baseline

From `f.6.a` manifest (`docs/dev/m9_2c_iv_f6a_unresolved_type_invariant_manifest.md`):
- unresolved invariant signatures: `4`
- compile units: `event.cc`, `fiber_context_runtime.cc`, `fiber_impl.cc`, `quorum_event.cc`
- each unit had `rrr_Future_State` unresolved invariant count `1`.

Replay root anchor:
- `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116`

## Implementation

Bounded pass added in `crates/fragile-clang/src/ast_codegen.rs`:
- `normalize_f6b_rrr_future_state_unresolved_rehydration_slice`

Contract:
1. If `rrr_Future_State` is referenced and undefined, and a concrete
   non-generic state head exists (`State` or `rrr_State`), append
   `pub type rrr_Future_State = <state-head>;`.
2. If the unit already has `pub type State = rrr_Future_State;`, avoid alias
   cycles and materialize `rrr_Future_State` as concrete struct with
   `ready` / `timed_out` fields.

Added focused unit tests:
- `test_normalize_f6b_rrr_future_state_unresolved_rehydration_slice_adds_missing_alias`
- `test_normalize_f6b_rrr_future_state_unresolved_rehydration_slice_is_idempotent`
- `test_normalize_f6b_rrr_future_state_unresolved_rehydration_slice_rejects_generic_heads`
- `test_normalize_f6b_rrr_future_state_unresolved_rehydration_slice_breaks_state_alias_cycles`

## Focused Reactor Compile Probes

Commands were replay-root compile commands (rrr target) from
`build_fragilec/compile_commands.json` lines `1565/1571/1583/1589`, executed
with patched `target/release/fragilec`.

Probe roots and summary:

| Unit | Probe root | status | unresolved invariant count | rustc compile failed count | tail result |
| --- | --- | ---: | ---: | ---: | --- |
| `fiber_context_runtime.cc` | `/tmp/fragile_f6b_probe_single_20260331T051554Z` | `0` | `0` | `0` | clean object compile |
| `fiber_impl.cc` | `/tmp/fragile_f6b_probe_single_20260331T051948Z` | `1` | `0` | `1` | residual typed tail `E0308 _opaque [0; 64]` |
| `quorum_event.cc` | `/tmp/fragile_f6b_probe_single_20260331T052530Z` | `1` | `0` | `1` | residual typed tail `E0308 _opaque [0; 64]` |
| `event.cc` | `/tmp/fragile_f6b_probe_single_20260331T053053Z` | `1` | `0` | `1` | residual typed tail `E0308 _opaque [0; 64]` |

Aggregate unresolved invariant result across the four reactor-family probes:
- `aggregate_unresolved_invariant_count=0`

## Outcome

- `rrr_Future_State` unresolved-type invariant blockers are closed in the
  focused reactor compile probes (`4 -> 0`).
- Lane is not green yet; residual typed rustc tails remain and roll forward to
  `M9.2.c.iv.f.6.c` non-increase/delta accounting.

## Evidence

- `TODO.md` (`M9.2.c.iv.f.6.b` marked done with probe roots)
- `docs/fragile-dev-book.md` entry for 2026-03-31 f.6.b
- Probe logs:
  - `/tmp/fragile_f6b_probe_single_20260331T051554Z`
  - `/tmp/fragile_f6b_probe_single_20260331T051948Z`
  - `/tmp/fragile_f6b_probe_single_20260331T052530Z`
  - `/tmp/fragile_f6b_probe_single_20260331T053053Z`

Next leaf: `M9.2.c.iv.f.6.c`.
