# `misc.cpp` Strict Compile-Cost Investigation (2026-03-16)

## Goal
Explain why `vendor/mako/src/rrr/base/misc.cpp` is still timeout-bound in strict `fragilec` build-only replays, using deterministic artifacts only.

## Scope and Constraints
- Scope: strict single-lane (`fragilec`) compile path for `misc.cpp` in the active RPC blocker loop.
- Constraints from project policy:
  - no target-specific hacks or RPC-only code paths;
  - no semantic stubs/fake fallback bodies;
  - changes must stay generic and regression-tested.

## Deterministic Evidence

### Telemetry capture commands (checkpoint timing + callshape profile)
```bash
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_stage_timing_120_v2.txt \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_callshape_profile_120_v2.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iii_build_only_20260315_v2 \
  --lanes fragilec --max-replays 1 --timeout-seconds 120

FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_stage_timing_300_v2.txt \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_callshape_profile_300_v2.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iii_build_only_20260315_v2 \
  --lanes fragilec --max-replays 1 --timeout-seconds 300
```

### Stability commands (recent strict build-only loop)
```bash
cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1 \
  --lanes fragilec --build-only --jobs 4 --build-timeout-seconds 600

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing

python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1 \
  --lanes fragilec --max-replays 1 --timeout-seconds 300
```

## Stage Time Breakdown
Data sources:
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_stage_timing_120_v2.txt`
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_stage_timing_300_v2.txt`

| Timeout window | Export (ms) | Parse (ms) | Enrichment (ms) | Subtotal before codegen (ms) | Parse share | Enrichment share |
|---|---:|---:|---:|---:|---:|---:|
| 120s replay | 3,492 | 49,356 | 31,280 | 84,128 | 58.7% | 37.2% |
| 300s replay | 3,478 | 57,774 | 31,617 | 92,869 | 62.2% | 34.0% |

Observations:
- The pre-codegen path is already expensive (84s-93s) before deeper codegen checkpoints.
- Parse dominates pre-codegen wall time in both windows.
- Enrichment is the second-largest fixed cost.

## Checkpoint Progress and Byte Metrics
Data sources:
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_callshape_profile_120_v2.txt`
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_a_callshape_profile_300_v2.txt`

Checkpoint/status progression:
- 120s: `codegen_started -> codegen_after_template_collection`
- 300s: `codegen_started -> codegen_after_template_collection -> codegen_after_template_instantiation_generation`

Byte metric delta between windows:
- `input_bytes`: `0` (120s) -> `572,928` (300s), delta `+572,928`.
- `output_bytes`: `0` in both windows.

Interpretation:
- The 300s run reaches template-instantiation generation but still does not reach top-level generation.
- `output_bytes=0` indicates the profiled normalization stage did not complete in these windows; the bottleneck remains earlier in codegen.

## Replay/Inventory Stability Across Recent Iterations
Run roots sampled:
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1`
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1`
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1`
- `/tmp/fragile_rpc_leaf_2_6d_b_ii_c_c_iv_d_d_d_d_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_c_b_build_only_20260316_v1`

Stable markers across all four roots:
- Harness:
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
  - `lane_fragilec_test_rpc_status=-1`
- Inventory:
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `nonincrease_gate_pass=true`
- Replay:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`

Conclusion from stability:
- Current micro-optimizations are preserving correctness/non-regression but have not changed the blocker class or compile unit.

## Dominant Hotspots
1. Parse-stage wall-time in LibTooling export/parse pipeline for `misc.cpp`.
2. Codegen window between `codegen_after_template_collection` and `codegen_after_template_instantiation_generation`, and then before top-level generation completion.
3. Timeout persists before runtime gates (`test_rpc` and rpcbench trials remain skipped in build-only manifests).

## Prioritized Generic Follow-up Optimizations

### P1. Add finer codegen checkpoints around template-instantiation internals
- Idea: split current broad checkpoint into deterministic sub-checkpoints (candidate discovery, inference-shape resolution, concrete-info build, instantiation emission) to isolate the dominant sub-phase.
- Expected impact: converts coarse hotspot into actionable segment-level targets and reduces trial-and-error optimizations.
- Validation criteria:
  - new checkpoint history appears deterministically in 120s/300s profile files;
  - no change to blocker class/file;
  - focused `fragile-clang` tests for checkpoint instrumentation pass.

### P2. Reduce repeated template candidate/inference work across equivalent call-shapes
- Idea: expand generic memoization reuse for resolver/instantiation paths keyed by stable call-shape signatures (without target conditionals), especially where repeated misses/hits occur in the same TU.
- Expected impact: lower elapsed time between template-collection and template-instantiation checkpoints.
- Validation criteria:
  - 300s profile reaches later checkpoints (ideally `codegen_after_top_level_generation`);
  - `input_bytes` growth shifts earlier, and blocker timeout moves later or clears;
  - unit regression set for template-resolution correctness remains green.

### P3. Pre-codegen cost containment in parse/enrichment (generic)
- Idea: identify and eliminate redundant parse/enrichment traversals that do not affect emitted code for the active TU path.
- Expected impact: reduce the current ~84s-93s pre-codegen subtotal.
- Validation criteria:
  - measurable reduction in `stage_end parse` and/or `stage_end enrichment` elapsed_ms in repeatable 120s/300s captures;
  - no regression in AST/export correctness tests.

### P4. Keep non-regression and blocker-stability gates mandatory during optimization loop
- Idea: continue strict harness + inventory non-increase + replay capture for every optimization iteration.
- Expected impact: ensures performance tuning does not reintroduce previous compile-error families.
- Validation criteria:
  - `lane_fragilec_class_rank_delta_vs_baseline <= 0`
  - `lane_fragilec_e0425_delta_vs_baseline <= 0`
  - blocker class/file remain stable or improve.

## Regression Gates Required for Each Follow-up
- Focused tests in touched subsystem (`cargo test -p fragile-clang ...`).
- Strict build-only replay + inventory non-increase + replay capture.
- Workspace and Python sweeps:
  - `timeout 300s cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## Wrong-Approach Check
This report and proposed follow-ups adhere to the project wrong-approach policy:
- no target-name conditionals,
- no force-native bypass,
- no fake semantic stubs to mask unresolved transpiler gaps.
(Reference: `docs/fragile-dev-book.md` -> “1.3 Wrong Approaches (Do Not Do)”.)
