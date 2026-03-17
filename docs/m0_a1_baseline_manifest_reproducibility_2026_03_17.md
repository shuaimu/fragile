# M0.A1 Baseline Manifest Reproducibility (2026-03-17)

## Objective

Close TODO acceptance leaf `M0.A1` by ensuring baseline manifests can be compared
deterministically across two consecutive strict-baseline runs.

## Scope and Sizing

This change is bounded well below 1000 LOC:

- update deterministic comparable-manifest emission in
  `scripts/mako_rpc_strict_baseline.py`
- extend focused Python regressions in
  `tests/python/test_mako_rpc_strict_baseline.py`
- close acceptance marker in `TODO.md`

No decomposition was required.

## Wrong-Approach Check

Reviewed `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md` before
implementation.

Confirmed this iteration is orchestration/manifest logic only:

- no target-specific parser/codegen behavior
- no fake semantic stubs or masked-success fallbacks
- no force-native bypass strategy

## Design

`mako_rpc_strict_baseline.py` now emits:

- `strict_baseline_manifest.txt` (full manifest with run-local metadata)
- `strict_baseline_comparable_manifest.txt` (stable comparable subset)
- `comparable_manifest_sha256` and `comparable_manifest_key_count`

Comparable-manifest filtering excludes run-local/path/timing-volatile keys
(`run_root`, manifest path fields, stage timing path/raw timing/error fields, and
comparable self-reference keys).

## Validation

Focused:

- `python3 -m unittest tests/python/test_mako_rpc_strict_baseline.py -v`

Full Python:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace:

- `cargo test --workspace --all-targets`

## User Manual

Run baseline capture twice and compare the emitted comparable manifest hashes:

```bash
python3 scripts/mako_rpc_strict_baseline.py --run-root /tmp/fragile_m0_1_run1
python3 scripts/mako_rpc_strict_baseline.py --run-root /tmp/fragile_m0_1_run2

grep '^comparable_manifest_sha256=' /tmp/fragile_m0_1_run1/strict_baseline_manifest.txt
grep '^comparable_manifest_sha256=' /tmp/fragile_m0_1_run2/strict_baseline_manifest.txt
```

Matching hashes indicate reproducible comparable baseline manifests across the two
runs.
