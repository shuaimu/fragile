# RPC Compile Blocker Leaf 2.6.c.iv.d.ii Design Note (2026-03-13)

## Scope

Leaf: `2.6.c.iv.d.ii`  
Objective: optimize the next generic codegen hotspot in the checkpoint window
between `codegen_after_template_instantiation_generation` and
`codegen_after_top_level_generation`.

## Size/Complexity Check

This leaf is a focused codegen + test change.
Estimated impact is small (`<500 LOC`) in one Rust file plus docs/TODO updates.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no `rpcbench`/`test_rpc` target-name conditionals
- no force-native escape-hatch behavior
- no fake semantic method bodies or placeholder success stubs
- generic codegen data-path optimization only

## Plan

1. Identify clone-heavy paths inside top-level generation for reopened namespaces.
2. Reduce duplicate cloning without changing emitted semantics.
3. Add focused regression coverage for reopened namespace merge behavior.
4. Re-run strict replay captures and full regression suites.

## Changes Implemented

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- changed merged namespace node storage:
  - `collected_nodes: Vec<ClangNode>` -> `Vec<Option<ClangNode>>`
- collection pass stores `Some(grandchild.clone())` for indexed nodes.
- generation pass consumes merged entries with `slot.take()`:
  - avoids cloning the same merged child a second time during top-level emission.
- added focused regression:
  - `test_reopened_namespace_merges_all_children_without_dropping_entries`

## Commands Executed

```bash
cargo test -p fragile-clang reopened_namespace_merges_all_children_without_dropping_entries -- --nocapture
cargo test -p fragile-clang problematic_callshape -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300

FRAGILEC_MODE=strict \
python3 scripts/mako_rpcbench_harness.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_ii_build_only_20260313 \
  --lanes fragilec \
  --build-only \
  --jobs 4 \
  --build-timeout-seconds 180

python3 scripts/mako_rpc_compile_blocker_inventory.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_d_ii_build_only_20260313 \
  --lanes fragilec \
  --baseline-manifest /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt \
  --enforce-nonincreasing

cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Deterministic Evidence

Replay and profiling artifacts:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_callshape_profile_120_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_callshape_profile_300_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_stage_timing_120_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_stage_timing_300_v1.txt`

Profile highlights:

- 120s: `status_history=codegen_started`
- 300s: `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`
- 300s checkpoint output bytes decreased vs `d.i` capture:
  - `d.i`: `input_bytes=572172`
  - `d.ii`: `input_bytes=564725`

Strict build-only non-increase precheck (same post-change run):

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_build_only_20260313/benchmark_harness_manifest.txt`:
  - `lane_fragilec_configure_status=0`
  - `lane_fragilec_clean_status=0`
  - `lane_fragilec_build_status=124`
  - `lane_fragilec_failure_class=build_timeout`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_ii_build_only_20260313/rpc_compile_blocker_inventory_manifest.txt`:
  - `lane_fragilec_first_failing_compile_class=build_timeout`
  - `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
  - `lane_fragilec_class_rank_delta_vs_baseline=0`
  - `lane_fragilec_e0425_delta_vs_baseline=0`
  - `lane_fragilec_nonincrease_gate_pass=true`
  - `nonincrease_gate_pass=true`

Full suites:

- `cargo test --workspace --all-targets`: known baseline remains (`46` existing
  `fragile-clang` lib failures, unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`: `29` passed,
  `1` skipped

## Outcome

Leaf `2.6.c.iv.d.ii` is complete: reopened-namespace top-level generation now
avoids an avoidable second clone per merged child while keeping behavior locked
with focused regression coverage. Timeout blocker class remains
`build_timeout` on `src/rrr/base/misc.cpp` for the next iteration.
