# M9.2.c.iv.e.34.f.5.e.5.e.1 Inventory - Flat-Base Vtable Lane Closure

## Scope

- Task: `M9.2.c.iv.e.34.f.5.e.5.e.1`
- Goal: close the flat-base vtable lane regression where
  `(*__fragile_base).__base.__vtable` is emitted for single-base types
  (`rrr::Pollable`, `rrr_Marshallable`) that only expose `__vtable`.
- Bound: one bounded normalization update in `normalize_rpc_event_surface_artifacts`
  plus focused unit coverage (<1000 LOC).

## Wrong-approach check

Checked before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Compliance notes:

- No target-specific file-path branching.
- No native bypass / force-native escape hatch.
- No fake-success rollback/deletion pattern.
- Kept rewrite constrained to explicit vtable-shim lane evidence:
  `let __fragile_vtable = (*__fragile_base).__base.__vtable;` +
  `__fragile_self_arg` typed as `rrr::Pollable`/`rrr_Marshallable`.

## Baseline replay evidence

Replay command:

- `python3 scripts/mako_rpc_strict_runtime_replay.py --run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452 --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260327T211622Z_p1179723`

Run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452`

Lane result:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`
- blocker inventory: `total=22`, `unique=18`, non-increase vs baseline `true`

Targeted blocker signatures from `lane_fragilec/build.stderr`:

- `error[E0609]: no field `__base` on type `rrr::Pollable`` (2)
- `error[E0609]: no field `__base` on type `rrr_Marshallable`` (2)

## Implementation

File changed:

- `crates/fragile-clang/src/ast_codegen.rs`

Change summary:

1. In `normalize_rpc_event_surface_artifacts`, after the existing `__vtable -> __base.__vtable`
   rewrite, added a bounded corrective rewrite for flat-base shims:
   - when a line contains
     `let __fragile_vtable = (*__fragile_base).__base.__vtable;`
     and the same line declares `__fragile_self_arg` typed as
     `rrr::Pollable`, `rrr_Marshallable`, or `rrr::Marshallable`,
     rewrite back to `(*__fragile_base).__vtable`.
2. Kept derived event lane behavior unchanged (`Event` lanes continue using
   `__base.__vtable`).

## Focused validation

Commands run:

- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_preserves_flat_base_vtable_access_for_pollable_and_marshallable -- --nocapture`
- `cargo test -p fragile-clang test_normalize_rpc_event_surface_artifacts_rewrites_quorum_event_command_map_and_event_base_lanes -- --nocapture`

Results:

- Both targeted tests passed.

New/updated regression coverage:

- `test_normalize_rpc_event_surface_artifacts_preserves_flat_base_vtable_access_for_pollable_and_marshallable`
  - asserts flat-base `Pollable`/`Marshallable` lanes keep `__vtable`
  - asserts derived `Event` lane still normalizes to `__base.__vtable`

## Remaining closure path

Follow-up leaves remain under `M9.2.c.iv.e.34.f.5.e.5.e`:

- `M9.2.c.iv.e.34.f.5.e.5.e.2` swap `__assoc_sub_state` pointer/reference callshape mismatch
- `M9.2.c.iv.e.34.f.5.e.5.e.3` ordering/unsafe printf residual typed lanes
- `M9.2.c.iv.e.34.f.5.e.5.e.4` strict replay rerun + lane contract validation
