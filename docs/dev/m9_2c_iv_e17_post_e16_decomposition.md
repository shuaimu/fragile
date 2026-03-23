# M9.2.c.iv.e.17 Post-e.16 Decomposition Plan

## Context

`M9.2.c.iv.e.17` is currently a broad WIP item and is too large for one bounded implementation step.

Current anchor state (post-e.16 inventory in TODO):
- strict compile totals remain high across `debugging/misc/basetypes/logging`;
- `E0308` is still dominant, followed by `E0609`/`E0599`/`E0425` clusters.

This document decomposes e.17 into bounded leaves so each execution slice stays under the <1000 LOC constraint.

## Inventory Anchors

Primary evidence anchors used for decomposition:
- `TODO.md` M9.2.c.iv.e inventory summary (post-e.16/post-e.15 state);
- `docs/dev/m9_2c_iv_e5_closure_inventory.md` for dominant-class trend context;
- `docs/dev/m9_2c_iv_rerun_inventory.md` for strict-lane blocker taxonomy and replay framing.

Inference from these anchors:
- first execution slice should still target one bounded `E0308` family only;
- second slice should be selected from refreshed post-b dominance, not stale pre-b counts;
- final closure leaf should be inventory publication and deterministic non-increase reporting.

## Bounded Leaf Breakdown (<1000 LOC each)

1. `M9.2.c.iv.e.17.a` (this leaf)
- Objective: publish decomposition and ordering contract.
- Estimated change size: <200 LOC (TODO + docs + regression guards).
- Acceptance: TODO includes `e.17.a/b/c/d`; this decomposition document is committed.

2. `M9.2.c.iv.e.17.b`
- Objective: implement one bounded dominant `E0308` fix family only.
- Estimated change size: 200-700 LOC (normalizer + focused tests).
- Acceptance: measurable `E0308` reduction in refreshed strict inventory with no target-specific hacks.

3. `M9.2.c.iv.e.17.c`
- Objective: implement one bounded next-dominant class slice (`E0609` or `E0599` or `E0425`) selected from post-b rerun.
- Estimated change size: 200-700 LOC.
- Acceptance: measurable reduction in selected class with focused tests and no regression in replay contract.

4. `M9.2.c.iv.e.17.d`
- Objective: rerun strict replay inventory and publish deterministic per-file deltas.
- Estimated change size: <300 LOC (docs/tests/artifact checks).
- Acceptance: updated inventory artifact + TODO closure evidence for e.17.

## Ordering Constraints

- Execute `e.17.b` before `e.17.c` so class selection for `e.17.c` is data-driven from refreshed dominance.
- Do not mix multiple dominant error families inside one leaf.
- Keep edits generic and transpiler-correct: no rollback-pattern additions, semantic stubs, or target-only bypasses.

## Validation Contract Per Leaf

Each implementation leaf must pass full regression gates:
- `cargo test --workspace --all-targets --exclude fragile-clang`
- `cargo test -p fragile-clang --tests`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
