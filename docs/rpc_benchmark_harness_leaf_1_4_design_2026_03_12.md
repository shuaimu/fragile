# RPC Benchmark Harness Leaf 1.4 Design (2026-03-12)

## Objective

Implement deterministic rpcbench trial QPS aggregation and lane comparison metadata:

- parse per-trial client QPS from runtime artifacts
- compute `clang`/`fragilec` average QPS
- emit deterministic comparison manifests
- persist an explicit no-regression verdict (`fragile_avg_qps >= clang_avg_qps`)

## Scope Sizing

Estimated implementation size for this leaf was within a moderate patch budget:

- harness code updates: ~200-300 LOC
- fixture regression updates: ~120-220 LOC
- docs/TODO updates: small

This was judged small enough to execute directly without further nested TODO breakdown.

## Scope

Included:

- deterministic QPS extraction from rpcbench client output (`qps=<num>` / `<num> qps`)
- per-trial QPS capture:
  - `lane_<lane>_trial_<NN>_qps`
- per-lane averages:
  - `lane_<lane>_avg_qps`
- global comparison fields:
  - `clang_avg_qps`
  - `fragile_avg_qps`
  - `fragile_minus_clang_qps`
  - `fragile_over_clang_ratio`
  - `no_regression_verdict`
- deterministic comparison artifact file:
  - `benchmark_qps_comparison_manifest.txt`
- runtime gate behavior:
  - execution exits nonzero when verdict is `fail` or `insufficient_data`

Not included:

- ignored real-world replay regression harness for the comparison path (deferred to leaf `1.5`)

## Wrong-Approach Check

Aligned with `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`:

- no RPC target-name codegen conditionals
- no semantic fake method-body fallback additions
- no force-native bypass path
- no hidden pass behavior: missing comparison data is explicitly surfaced as `insufficient_data`

## Test Strategy

Local fixture tests validate:

- pass verdict (`fragile` >= `clang`)
- fail verdict (`fragile` < `clang`) with nonzero exit
- insufficient data verdict (QPS markers absent) with nonzero exit
- existing runtime failure paths continue to produce deterministic skip/failure artifacts
