# RPC Compile Blocker Leaf 2.5 Design (2026-03-13)

## Objective

Leaf `2.5` requires deterministic blocker-inventory rerun with an explicit non-increase gate
against leaf-`2.1` baseline (blocker-class severity and `E0425` count).

## Scope Sizing

Estimated implementation was small (<500 LOC), so no further TODO decomposition was needed.

- script update: `scripts/mako_rpc_compile_blocker_inventory.py` (baseline/gate support)
- test update: `tests/python/test_mako_rpc_compile_blocker_inventory.py`
- docs/TODO updates

## Decision

Implement baseline-aware non-increase gating directly in the inventory script, rather than
manual comparison only, so this leaf is enforceable and repeatable:

- optional `--baseline-manifest <path>` input
- optional `--enforce-nonincreasing` hard gate
- deterministic lane-level delta fields in `rpc_compile_blocker_inventory_manifest.txt`
- nonzero exit when the enforced gate fails

Severity comparison rule:

- use deterministic blocker-class ordering (worst to best):
  `unresolved_name_or_type_e0425` -> ... -> `none`
- lane passes class gate when current class is equal or better than baseline
- lane passes count gate when current `E0425` <= baseline `E0425`
- skipped build (`build.status=-1`) is marked as non-comparable and fails lane gate
  under enforcement

## Wrong-Approach Check

Aligned with project constraints and `docs/dev/wrong.md`:

- no RPC target-name conditionals
- no semantic method-body stubs or fake-success behavior
- no force-native bypass
- gate outcome is derived from real lane artifacts/manifests only

## Implementation

Updated `scripts/mako_rpc_compile_blocker_inventory.py`:

- added CLI flags:
  - `--baseline-manifest`
  - `--enforce-nonincreasing`
- added deterministic blocker severity ordering
- added baseline-manifest parser + required-key validation
- added lane-level comparison fields:
  - `lane_<lane>_baseline_*`
  - `lane_<lane>_class_rank_delta_vs_baseline`
  - `lane_<lane>_class_nonworsening_vs_baseline`
  - `lane_<lane>_e0425_delta_vs_baseline`
  - `lane_<lane>_e0425_nonincrease_vs_baseline`
  - `lane_<lane>_executable_comparison_vs_baseline`
  - `lane_<lane>_nonincrease_gate_pass`
- added root field:
  - `nonincrease_gate_pass`
- manifest `task_leaf` behavior:
  - `2.1` without baseline
  - `2.5` with baseline
- enforced mode exits nonzero when gate fails

Updated `tests/python/test_mako_rpc_compile_blocker_inventory.py`:

- `test_inventory_nonincrease_gate_passes_for_better_or_equal_baseline`
- `test_inventory_nonincrease_gate_fails_when_class_severity_worsens`
- `test_inventory_nonincrease_gate_fails_when_e0425_count_increases`
- `test_inventory_nonincrease_gate_fails_for_missing_baseline_keys`

## Deterministic Rerun Evidence

Built deterministic baseline/current lane artifact roots from archived logs:

- baseline: `/tmp/fragile_rpc_leaf_2_5_baseline_20260313`
  - `lane_fragilec/build.stderr` from
    `logs/mako_bench_cmp_20260308/fragilec_rpcbench_build.log`
  - inventory result:
    - `lane_fragilec_first_failing_compile_class=unresolved_name_or_type_e0425`
    - `lane_fragilec_first_failing_compile_e0425_count=168`

- current: `/tmp/fragile_rpc_leaf_2_5_current_20260313`
  - `lane_fragilec/build.stderr` from
    `logs/mako_bench_cmp_20260308/fragilec_fixed_rpcbench_build3.log`
  - gated inventory command:
    - `python3 scripts/mako_rpc_compile_blocker_inventory.py --run-root /tmp/fragile_rpc_leaf_2_5_current_20260313 --baseline-manifest /tmp/fragile_rpc_leaf_2_5_baseline_20260313/rpc_compile_blocker_inventory_manifest.txt --enforce-nonincreasing`
  - inventory result:
    - `lane_fragilec_first_failing_compile_class=unresolved_name_or_type_e0425`
    - `lane_fragilec_first_failing_compile_e0425_count=28`
    - `lane_fragilec_e0425_delta_vs_baseline=-140`
    - `lane_fragilec_nonincrease_gate_pass=true`
    - `nonincrease_gate_pass=true`

## Validation

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tests/python/test_mako_rpc_compile_blocker_inventory.py -v`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/python -p 'test_*.py' -v`
- `cargo test --workspace` (known pre-existing `fragile-clang` baseline red cluster remains: `717 passed / 46 failed`)
- `FRAGILE_ENABLE_DEGRADED_FALLBACK=1 cargo test --workspace` (known pre-existing baseline red cluster remains: `739 passed / 24 failed`)
