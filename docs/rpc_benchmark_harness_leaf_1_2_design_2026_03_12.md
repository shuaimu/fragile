# RPC Benchmark Harness Leaf 1.2 Design (2026-03-12)

## Objective

Implement clean configure/build execution capture for both lanes (`clang`, `fragilec`) with deterministic artifacts and failure-class metadata.

## Scope

Included:

- lane-ordered execution of:
  - `cmake -S/-B` configure
  - `cmake --build <dir> --target clean`
  - `cmake --build <dir> --target test_rpc rpcbench masstree_perf`
- per-step artifact files for each lane:
  - `<step>.status`, `<step>.stdout`, `<step>.stderr`
- deterministic failure classification:
  - `none`, `configure_failed`, `configure_timeout`, `clean_failed`, `clean_timeout`, `build_failed`, `build_timeout`
- skip semantics:
  - if `configure` fails, mark `clean`/`build` as skipped (`status=-1`)
  - if `clean` fails, mark `build` as skipped (`status=-1`)
- manifest enrichment with lane statuses/failure classes

Not included (later leaves):

- `test_rpc` runtime execution
- rpcbench server/client trial execution
- QPS parsing and no-regression verdict aggregation

## Determinism details

- fixed lane order: `clang`, then `fragilec`
- fixed file names for every captured step
- explicit timeout status sentinel: `124`
- explicit skip status sentinel: `-1`

## Testing strategy

Use local fake-CMake fixture tests to avoid real project builds:

- success path: both lanes all-zero status and `failure_class=none`
- failure path: lane-specific configure failure, deterministic skipped follow-up status files, and correct failure class persisted to file and manifest
