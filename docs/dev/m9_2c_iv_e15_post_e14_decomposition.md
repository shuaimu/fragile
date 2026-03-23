# M9.2.c.iv.e.15 Post-e.14 Decomposition Plan

## Context

`M9.2.c.iv.e.15` is currently a broad WIP item:
- post-e.14 strict inventory totals: `debugging=209`, `misc=207`, `basetypes=194`, aggregate `610`;
- the task statement is too broad for a single bounded implementation change.

This document decomposes e.15 into bounded leaves with explicit ordering and acceptance checks.

## Inventory Anchors

Primary evidence anchors used for this decomposition:
- `TODO.md` e.14 summary (post-e.14 totals);
- `docs/dev/m9_2c_iv_rerun_inventory.md` (dominant-class distribution and strict replay error taxonomy);
- `docs/dev/m9_2c_iv_e5_closure_inventory.md` (prior dominant-class trend context).

Inference from these anchors:
- the next most valuable class to attack remains `E0308` type mismatch clusters;
- follow-on slices should target the next dominant class after rerun (`E0277`/`E0599`/`E0609`) based on refreshed counts.

## Bounded Leaf Breakdown (<1000 LOC each)

1. `M9.2.c.iv.e.15.a` (this leaf)
- Objective: publish bounded decomposition and ordering contract.
- Estimated change size: <200 LOC (docs + TODO + regression guards).
- Acceptance: TODO decomposed into e.15.a/b/c/d and this plan document checked in.

2. `M9.2.c.iv.e.15.b`
- Objective: fix one bounded `E0308` sub-cluster only (single normalized pattern family).
- Estimated change size: 200-700 LOC (normalizer + tests).
- Acceptance: measurable `E0308` reduction in refreshed inventory with no regression in unrelated classes.

3. `M9.2.c.iv.e.15.c`
- Objective: fix the next dominant class after e.15.b rerun (`E0277` or `E0599` or `E0609`) in one bounded slice.
- Estimated change size: 200-700 LOC.
- Acceptance: measurable reduction for selected class, focused regression tests, and no target-specific conditionals.

4. `M9.2.c.iv.e.15.d`
- Objective: rerun strict inventory and publish deltas for `debugging/misc/basetypes` with non-increase evidence.
- Estimated change size: <300 LOC (docs/tests/artifact validation).
- Acceptance: updated inventory artifact and TODO closure evidence for e.15.

## Ordering Constraints

- Run `e.15.b` before `e.15.c` so the second slice is selected from post-b refreshed dominance, not stale pre-b counts.
- Do not mix multiple error families in a single leaf.
- Keep each code edit bounded and generic (no target-specific hacks, no semantic stubs).

## Validation Contract Per Leaf

Each implementation leaf must keep the full regression gate green:
- `cargo test --workspace --all-targets --exclude fragile-clang`
- `cargo test -p fragile-clang --tests`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
