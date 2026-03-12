# RPC Benchmark Harness Leaf 1.5 Design (2026-03-12)

## Objective

Add explicit regression gates for completed harness leaves `1.1` through `1.4`:

- fast local fixture gate (default)
- ignored real-world replay gate (opt-in)

Both gates should assert required artifact/manifest contracts deterministically.

## Scope Sizing

Estimated implementation size was small (<500 LOC total across tests/docs):

- Python test additions: ~120-220 LOC
- docs/TODO updates: small

No additional TODO decomposition was required.

## Scope

Included:

- local-fixture gate test that validates:
  - expected-artifact contract file is complete and materialized on disk
  - manifest/comparison-manifest required fields are present and coherent
  - integrated `1.1`..`1.4` behavior (plan, build/runtime capture, QPS aggregation, verdict)
- ignored real-world replay gate test (env-gated) that validates:
  - required artifacts are present
  - comparison manifest is emitted
  - verdict consistency between main manifest and comparison manifest

Not included:

- forcing real-world replay in default local/CI runs

## Wrong-Approach Check

Aligned with `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`:

- no compiler/codegen target-name hacks
- no semantic fake method-body additions
- no force-native bypass path
- real-world gate remains opt-in to avoid hiding failures or inflating default runtime costs

## Test Execution

- default gate:
  - `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpcbench_harness.py -v`
- real-world gate:
  - `FRAGILE_RUN_REAL_WORLD_RPCBENCH_HARNESS=1 ... test_regression_gate_real_world_replay_emits_required_artifacts_and_manifests`
