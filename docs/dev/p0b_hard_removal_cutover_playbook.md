# P0.b Hard-Removal Cutover Playbook

Date: 2026-03-21  
Status: Pre-cutover planning complete; execution blocked until 2026-04-18 hardening-window expiry.

## Purpose

`P0.b` is the highest-priority open item, but it is too large for one safe patch and is
explicitly date-gated to on/after **2026-04-18**.

This document decomposes `P0.b` into small executable leaves and provides a cutover-day
execution manual that avoids shortcut approaches.

## Scope and Non-Goals

Scope:
- remove legacy LibTooling parser path from strict production flow;
- remove escape-hatch selection and associated telemetry/policy code;
- remove deprecated parser/backend API surfaces exposed by active crates.

Non-goals:
- no target-specific shortcuts (`mako`, `rpcbench`, `test_rpc`);
- no `FRAGILEC_FORCE_NATIVE_SOURCES` bypass;
- no fake semantic stubs to hide regressions.

## Why Decompose

Single-shot removal would span multiple crates and likely exceed 1000 LOC with high merge
and regression risk. Decomposed leaves keep each change bounded and independently testable.

## Leaf Breakdown (<1000 LOC each)

1. `P0.b.2` Driver/backend selector removal (`fragile-driver`, `fragilec`).
   - Estimated touch: 300-700 LOC.
   - Output: strict backend/escape-hatch selection removed from production entrypoints.

2. `P0.b.3` `fragile-clang/src/lib.rs` backend/fallback API removal.
   - Estimated touch: 250-600 LOC.
   - Output: parser-output handoff is the only strict production path.

3. `P0.b.4` `AstCodeGen` LibTooling enrichment state removal.
   - Estimated touch: 150-400 LOC.
   - Output: no libtooling-specific state/methods in codegen core.

4. `P0.b.5` `libtooling.rs` module deletion and re-export cleanup.
   - Estimated touch: 150-500 LOC net (large file deletion, small integration edits).
   - Output: no LibTooling module in fragile-clang public surface.

5. `P0.b.6` CLI `--use-libtooling` path and stale artifacts removal.
   - Estimated touch: 120-350 LOC.
   - Output: no CLI pre-parse LibTooling lane, no stale example/script artifacts.

6. `P0.b.7` Anti-regression test lane replacement for post-removal state.
   - Estimated touch: 150-500 LOC.
   - Output: tests fail if strict production flow reintroduces LibTooling parser path.

7. `P0.b.8` Full regression validation and artifact capture.
   - Estimated touch: <=150 LOC docs/test metadata updates.
   - Output: full Rust/Python suite evidence captured for cutover run.

8. `P0.b.9` Docs + README finalization.
   - Estimated touch: <=250 LOC.
   - Output: deprecation language removed; current operational contract documented.

## Execution Order (Cutover Day)

Run leaves in this strict order:
1. `P0.b.2`
2. `P0.b.3`
3. `P0.b.4`
4. `P0.b.5`
5. `P0.b.6`
6. `P0.b.7`
7. `P0.b.8`
8. `P0.b.9`

Reasoning:
- remove selection/fallback surfaces before deleting implementation modules;
- install anti-regression tests before final docs closure;
- run full suite before marking docs/README final.

## Cutover-Day Checklist

1. Confirm date gate:
   - `date -u` must be on/after `2026-04-18`.
2. Sync:
   - `git pull --rebase`
3. Execute leaf changes in order above.
4. Run targeted tests after each leaf touching code.
5. Run full regressions after `P0.b.7`:
   - `cargo test --workspace --all-targets`
   - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
6. Verify no binary/temp artifacts staged:
   - `git status --short`
7. Commit and push:
   - commit message references `P0.b.x` leaves completed.

## Regression Gates for Completion

Required before closing `P0.b`:
- strict production flow has no LibTooling parser selection path;
- no escape-hatch policy/telemetry code in production drivers;
- no `mod libtooling;` in `fragile-clang/src/lib.rs`;
- P0 anti-regression tests pass in post-removal mode;
- workspace Rust tests and Python suite are green.

## Wrong-Approach Guardrail

Cross-checked with `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`:
- do not bypass fragile translation with native compilation;
- do not add target-specific conditions to "make green";
- do not add placeholder/fake semantic bodies to hide missing removal work.
