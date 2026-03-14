# 2026-03-14 CI Leaf 2.8.b Breakdown

## Scope analyzed

Target TODO item:

- `2.8.b` Re-run CI-aligned local commands for `rapidjson-smoke-baseline` and `build` phases, then require zero failures.

## Why this is larger than a single small leaf

Running the exact workflow command set reproduced:

- rapidjson smoke commands: green,
- `cargo build --verbose`: green,
- `cargo test --verbose`: integration target emits many failing tests and then enters prolonged no-progress in libcxx integration tails.

This means the original zero-failure closure combines three different work streams:

1. Deterministic command-status/evidence capture,
2. Build-phase execution stability (completion without prolonged tail hangs),
3. Semantic correctness fixes for multiple integration failure families.

Closing all three in one leaf is too broad for a <500-LOC single-cycle change and risks non-deterministic progress reporting.

## Decomposed leaves

- `2.8.b.i` Capture deterministic CI-aligned status/log inventory (done in this cycle).
- `2.8.b.ii` Make build-phase replay deterministic to completion and persist final `build_phase_test` exit status.
- `2.8.b.iii` Fix top-ranked integration failure family from `2.8.b.i` with focused regressions.
- `2.8.b.iv` Re-run full CI-aligned command set and require all status codes `0`.

## Evidence artifacts from this cycle

- Run root: `/tmp/fragile_ci_leaf_2_8b_20260314`
- Status file: `/tmp/fragile_ci_leaf_2_8b_20260314/statuses.txt`
- Logs:
  - `/tmp/fragile_ci_leaf_2_8b_20260314/rapidjson_smoke_1.log`
  - `/tmp/fragile_ci_leaf_2_8b_20260314/rapidjson_smoke_2.log`
  - `/tmp/fragile_ci_leaf_2_8b_20260314/rapidjson_smoke_3.log`
  - `/tmp/fragile_ci_leaf_2_8b_20260314/rapidjson_smoke_4.log`
  - `/tmp/fragile_ci_leaf_2_8b_20260314/build_phase_build.log`
  - `/tmp/fragile_ci_leaf_2_8b_20260314/build_phase_test.log`

