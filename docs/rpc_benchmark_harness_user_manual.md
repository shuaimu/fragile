# RPC Benchmark Harness User Manual (Leaf 1.1)

## Purpose

`mako_rpcbench_harness.py` provides deterministic command-plan and artifact-contract scaffolding for the active RPC bring-up benchmark lane.

Current scope:

- leaf `1.1`: planning artifacts
- leaf `1.2`: configure/clean/build capture artifacts for both lanes

## Script

- Path: `scripts/mako_rpcbench_harness.py`

## Example

```bash
python3 scripts/mako_rpcbench_harness.py \
  --workspace-root /home/shuai/workspace/fragile \
  --mako-root /home/shuai/workspace/fragile/vendor/mako \
  --run-root /tmp/fragile_mako_rpcbench_leaf_1_1 \
  --plan-only \
  --trials 3 \
  --jobs 16 \
  --base-port 18900
```

The script prints `run_root` on success.

Execution mode (`1.2`, omit `--plan-only`):

```bash
python3 scripts/mako_rpcbench_harness.py \
  --workspace-root /home/shuai/workspace/fragile \
  --mako-root /home/shuai/workspace/fragile/vendor/mako \
  --run-root /tmp/fragile_mako_rpcbench_leaf_1_2 \
  --trials 3 \
  --jobs 16 \
  --base-port 18900
```

## Generated artifacts

Under `run_root`:

- `benchmark_harness_manifest.txt`
- `benchmark_harness_command_plan.txt`
- `benchmark_expected_artifacts.txt`
- lane/trial directories:
  - `lane_clang/trial_01`...
  - `lane_fragilec/trial_01`...

When running in execution mode (`1.2`), each lane also gets:

- `lane_<lane>/configure.status|stdout|stderr`
- `lane_<lane>/clean.status|stdout|stderr`
- `lane_<lane>/build.status|stdout|stderr`
- `lane_<lane>/failure_class.txt`

## Determinism contract

- Stable lane names: `clang`, `fragilec`
- Stable trial naming: `trial_01`, `trial_02`, ...
- Stable port map:
  - `clang`: `base_port + (trial-1)`
  - `fragilec`: `base_port + 100 + (trial-1)`
- Re-running with identical inputs and `run_root` rewrites identical plan/manifest files.

## Notes

- `leaf 1.2` executes only configure/clean/build.
- `test_rpc` runtime replay, rpcbench trial replay, and QPS aggregation are in later leaves (`1.3+`).
