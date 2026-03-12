# RPC Benchmark Harness Leaf 1.3 Design (2026-03-12)

## Objective

Implement deterministic runtime replay for both benchmark lanes after configure/clean/build:

- `test_rpc` execution per lane
- `rpcbench` server/client execution per trial with deterministic lane/trial ports

## Scope

Included:

- bounded runtime process lifecycle controls:
  - `--test-rpc-timeout-seconds`
  - `--rpc-client-timeout-seconds`
  - `--rpc-server-startup-wait-seconds`
  - `--rpc-server-shutdown-timeout-seconds`
- deterministic runtime artifacts:
  - `lane_<lane>/test_rpc.status|stdout|stderr`
  - `lane_<lane>/trial_<NN>/rpc_server.status|stdout|stderr`
  - `lane_<lane>/trial_<NN>/rpc_client.status|stdout|stderr`
- deterministic runtime failure classification:
  - `test_rpc_timeout`, `test_rpc_failed`
  - `rpc_trial_<NN>_rpc_server_timeout|rpc_server_failed|rpc_client_timeout|rpc_client_failed`
- manifest enrichment:
  - `lane_<lane>_test_rpc_status`
  - `lane_<lane>_completed_trials`
  - `lane_<lane>_failure_class`

Not included:

- qps parsing/aggregation
- no-regression verdict output (`clang_avg_qps` vs `fragile_avg_qps`)

## Determinism and Safety Notes

- fixed lane order remains `clang`, then `fragilec`
- stable trial port mapping remains unchanged from leaves `1.1`/`1.2`
- skipped runtime steps always write explicit sentinel artifacts (`status=-1`)
- command timeout sentinel remains `124`
- command start failures are captured as explicit nonzero status (`127`) and persisted as regular artifacts

## Test Strategy

Use local fixture tests with a fake CMake shim that materializes fake `test_rpc`/`rpcbench` binaries:

- success path:
  - both lanes complete `configure/clean/build/test_rpc`
  - all trials produce successful `rpc_server`/`rpc_client` status artifacts
  - manifest reports `completed_trials=<trials>` and `failure_class=none`
- `test_rpc` failure path:
  - lane `test_rpc` nonzero
  - trial artifacts marked skipped
  - lane failure class `test_rpc_failed`
- runtime trial failure path:
  - client nonzero in first trial
  - lane failure class pinned to first runtime failure (`rpc_trial_01_rpc_client_failed`)
  - runtime artifacts remain deterministic for all trials
