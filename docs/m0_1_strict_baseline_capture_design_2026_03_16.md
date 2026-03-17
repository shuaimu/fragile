# M0.1 Strict Baseline Capture Design (2026-03-16)

## Objective

Close TODO leaf `M0.1` by recording deterministic strict-run baseline artifacts for:

- `test_rpc` / `rpcbench` strict harness path
- compile-blocker inventory
- focused replay stage-timing trace

## Scope and Sizing

This leaf is bounded and implementation-sized under 1000 LOC:

- new orchestration script: `scripts/mako_rpc_strict_baseline.py` (~500 LOC)
- new unit tests: `tests/python/test_mako_rpc_strict_baseline.py` (~300 LOC)

No task decomposition was required.

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md` before implementation:

- no target-specific parser/codegen conditionals
- no force-native bypass strategy
- no fake semantic method stubs
- no masked-success behavior

This change is orchestration and artifact capture only.

## Implementation

Added script:

- `scripts/mako_rpc_strict_baseline.py`

Behavior:

1. Runs strict harness capture (`mako_rpcbench_harness.py`).
2. Runs compile-blocker inventory (`mako_rpc_compile_blocker_inventory.py`).
3. Runs top blocker replay (`mako_rpc_compile_blocker_replay.py`) with
   `FRAGILEC_TRANSPILE_STAGE_TIMING_PATH`.
4. Emits:
   - `strict_baseline_commands.txt`
   - `strict_baseline_harness.*`
   - `strict_baseline_inventory.*`
   - `strict_baseline_replay.*`
   - `strict_baseline_manifest.txt`

Added tests:

- `tests/python/test_mako_rpc_strict_baseline.py`
  - success path where harness can fail nonzero but baseline capture still succeeds
  - missing stage-timing file path handling
  - inventory failure hard-fail behavior

## User Manual

Run strict baseline capture:

```bash
python3 scripts/mako_rpc_strict_baseline.py \
  --workspace-root /home/shuai/workspace/fragile \
  --mako-root /home/shuai/workspace/fragile/vendor/mako \
  --run-root /tmp/fragile_m0_1_strict_baseline_20260316_v1 \
  --lanes fragilec \
  --jobs 4 \
  --trials 1 \
  --build-timeout-seconds 180 \
  --replay-timeout-seconds 120 \
  --replay-max-replays 1
```

The script prints the `run_root` on success.

## Recorded Baseline (M0.1 Execution)

Run root:

- `/tmp/fragile_m0_1_strict_baseline_20260316_v1`

Key fields from `strict_baseline_manifest.txt`:

- `harness_status=1`
- `inventory_status=0`
- `replay_status=0`
- `lane_fragilec_configure_status=0`
- `lane_fragilec_clean_status=0`
- `lane_fragilec_build_status=124`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_timeout`
- `lane_fragilec_first_failing_compile_class=build_timeout`
- `lane_fragilec_first_failing_compile_file=src/rrr/base/misc.cpp`
- `replay_01_status=124`
- `replay_01_timed_out=true`
- `stage_timing_exists=true`
- `stage_timing_status=started`

## Validation

- `python3 -m unittest tests/python/test_mako_rpc_strict_baseline.py -v`
- `python3 -m unittest tests/python/test_mako_rpcbench_harness.py -v`
