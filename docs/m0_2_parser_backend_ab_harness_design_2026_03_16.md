# M0.2 Parser Backend A/B Harness Design (2026-03-16)

## Objective

Close TODO leaf `M0.2` by adding a deterministic parser-backend A/B harness that:

- runs strict baseline capture side-by-side in separate run roots
- selects parser backend per run via `FRAGILEC_PARSER_BACKEND`
- emits deterministic comparable manifest snapshots and deterministic diff artifacts

## Scope and Sizing

This leaf is bounded and remains under 1000 LOC:

- new harness script: `scripts/mako_rpc_parser_backend_ab.py` (~430 LOC)
- new unit tests: `tests/python/test_mako_rpc_parser_backend_ab.py` (~250 LOC)

No additional TODO decomposition was required.

## Wrong-Approach Check

Reviewed `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md` before implementation.

Confirmed:

- no target-specific parser/codegen conditionals
- no force-native bypasses
- no fake semantic method bodies or fallback stubs
- no masked-success behavior

This change is orchestration and deterministic artifact diffing only.

## Implementation

Added:

- `scripts/mako_rpc_parser_backend_ab.py`

Behavior:

1. Builds deterministic side-by-side run roots under one parent run root:
   - `baseline_<backend>`
   - `candidate_<backend>`
2. Runs `mako_rpc_strict_baseline.py` twice (baseline + candidate), injecting:
   - `FRAGILEC_PARSER_BACKEND=<backend>`
3. Captures command status/stdout/stderr for each run under the parent run root.
4. Loads each `strict_baseline_manifest.txt`, removes non-comparable path keys, and writes:
   - `parser_backend_ab_baseline_comparable_manifest.txt`
   - `parser_backend_ab_candidate_comparable_manifest.txt`
5. Emits deterministic A/B diff manifest:
   - `parser_backend_ab_manifest.txt`
   - stable sorted diff/missing-key entries
   - comparable SHA-256 digests
   - equality summary fields

## User Manual

Run parser backend A/B strict baseline capture:

```bash
python3 scripts/mako_rpc_parser_backend_ab.py \
  --workspace-root /home/shuai/workspace/fragile \
  --mako-root /home/shuai/workspace/fragile/vendor/mako \
  --run-root /tmp/fragile_m0_2_parser_backend_ab_20260316_v1 \
  --baseline-backend libtooling \
  --candidate-backend libclang \
  --lanes fragilec \
  --jobs 4 \
  --trials 1 \
  --build-timeout-seconds 180 \
  --replay-timeout-seconds 120 \
  --replay-max-replays 1
```

The script prints the parent `run_root` on success.

## Validation

Focused regressions:

- `python3 -m unittest tests/python/test_mako_rpc_parser_backend_ab.py -v`
- `python3 -m unittest tests/python/test_mako_rpc_strict_baseline.py -v`

