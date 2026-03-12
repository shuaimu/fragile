# RPC Benchmark Harness User Manual (Leaf 1.4)

## Purpose

`mako_rpcbench_harness.py` provides deterministic dual-lane (`clang`/`fragilec`) rpcbench planning and replay artifacts for the active RPC bring-up benchmark lane.

Current scope:

- leaf `1.1`: planning artifacts
- leaf `1.2`: configure/clean/build capture artifacts for both lanes
- leaf `1.3`: runtime replay for `test_rpc` and per-trial rpcbench server/client execution
- leaf `1.4`: QPS aggregation/comparison manifests with no-regression verdict

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

Execution mode (`1.4`, omit `--plan-only`):

```bash
python3 scripts/mako_rpcbench_harness.py \
  --workspace-root /home/shuai/workspace/fragile \
  --mako-root /home/shuai/workspace/fragile/vendor/mako \
  --run-root /tmp/fragile_mako_rpcbench_leaf_1_4 \
  --trials 3 \
  --jobs 16 \
  --base-port 18900 \
  --test-rpc-timeout-seconds 120 \
  --rpc-client-timeout-seconds 120 \
  --rpc-server-startup-wait-seconds 1.0 \
  --rpc-server-shutdown-timeout-seconds 10
```

## Generated artifacts

Under `run_root`:

- `benchmark_harness_manifest.txt`
- `benchmark_harness_command_plan.txt`
- `benchmark_expected_artifacts.txt`
- `benchmark_qps_comparison_manifest.txt`
- lane/trial directories:
  - `lane_clang/trial_01`...
  - `lane_fragilec/trial_01`...

When running in execution mode (`1.4`), each lane also gets:

- `lane_<lane>/configure.status|stdout|stderr`
- `lane_<lane>/clean.status|stdout|stderr`
- `lane_<lane>/build.status|stdout|stderr`
- `lane_<lane>/test_rpc.status|stdout|stderr`
- `lane_<lane>/failure_class.txt`
- `lane_<lane>/trial_<NN>/rpc_server.status|stdout|stderr`
- `lane_<lane>/trial_<NN>/rpc_client.status|stdout|stderr`
- `lane_<lane>/trial_<NN>_qps` fields in `benchmark_harness_manifest.txt`
- `lane_<lane>_avg_qps` fields in `benchmark_harness_manifest.txt`

Comparison fields (manifest + comparison manifest):

- `clang_avg_qps`
- `fragile_avg_qps`
- `fragile_minus_clang_qps`
- `fragile_over_clang_ratio`
- `no_regression_verdict` (`pass`, `fail`, `insufficient_data`, `not_executed`)

## Determinism contract

- Stable lane names: `clang`, `fragilec`
- Stable trial naming: `trial_01`, `trial_02`, ...
- Stable port map:
  - `clang`: `base_port + (trial-1)`
  - `fragilec`: `base_port + 100 + (trial-1)`
- Re-running with identical inputs and `run_root` rewrites identical plan/manifest files.

## Notes

- plan-only mode (`--plan-only`) keeps leaf `1.1` behavior and emits deterministic plan/manifest/artifact-contract files only.
- execution mode (default) runs configure/clean/build and runtime replay; lane-level failures are summarized in `failure_class.txt` and mirrored in `benchmark_harness_manifest.txt`.
- no-regression gate is enforced in execution mode: if verdict is `fail` or `insufficient_data`, the script exits nonzero.
- QPS parsing looks for `qps=<number>`/`<number> qps` markers in rpcbench client output and uses successful trials only.
