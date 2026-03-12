# RPC Benchmark Harness User Manual (Leaf 1.1)

## Purpose

`mako_rpcbench_harness.py` provides deterministic command-plan and artifact-contract scaffolding for the active RPC bring-up benchmark lane.

Current scope (leaf `1.1`): planning artifacts only.

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

## Generated artifacts

Under `run_root`:

- `benchmark_harness_manifest.txt`
- `benchmark_harness_command_plan.txt`
- `benchmark_expected_artifacts.txt`
- lane/trial directories:
  - `lane_clang/trial_01`...
  - `lane_fragilec/trial_01`...

## Determinism contract

- Stable lane names: `clang`, `fragilec`
- Stable trial naming: `trial_01`, `trial_02`, ...
- Stable port map:
  - `clang`: `base_port + (trial-1)`
  - `fragilec`: `base_port + 100 + (trial-1)`
- Re-running with identical inputs and `run_root` rewrites identical plan/manifest files.

## Notes

- `leaf 1.1` does not execute configure/build/run commands.
- Later leaves (`1.2` onward) consume these contracts to add execution and aggregation behavior.
