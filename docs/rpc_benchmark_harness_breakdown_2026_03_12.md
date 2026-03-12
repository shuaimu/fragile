# RPC Benchmark Harness Breakdown (2026-03-12)

## Context
Top active task in `TODO.md`:

- `1) Establish deterministic clean benchmark harness for clang and fragilec (configure, clean, build, run, trial aggregation, artifact logs)`

## Scope sizing
Implementing the full task in one pass is estimated to exceed ~500 LOC once all required behavior and tests are included:

- command-plan + manifest contract and deterministic naming
- configure/build execution orchestration for two lanes
- bounded runtime orchestration for `test_rpc` and rpcbench server/client
- trial aggregation and QPS extraction/parity verdict
- local-fixture + ignored real-world regression coverage

Estimated combined implementation + tests: ~900-1400 LOC.

## Decomposition decision
Split task `1)` into leaf tasks `1.1`..`1.5` in `TODO.md` so each leaf remains independently testable and below ~1000 LOC.

## First leaf selected
Execute `1.1` first:

- deterministic command-plan + manifest scaffolding for dual lanes
- shared args and stable lane/trial naming
- artifact-file contract checks via local-fixture regression tests

This leaf is intentionally non-invasive and avoids target-specific shortcuts. It prepares deterministic structure for the later execution/aggregation leaves.
