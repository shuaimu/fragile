# M7.2 Shadow Parity Metrics (2026-03-19)

## Scope and Size Check
`M7.2` was implemented as an incremental extension to the existing shadow harness and tests.

Implementation surface:
- `scripts/parser_shadow_non_rpc_corpus.py`
- `tests/python/test_parser_shadow_non_rpc_corpus.py`

This leaf stayed below the requested ~1000 LOC change envelope.

## Goal
Track parity metrics in deterministic shadow-run artifacts for representative non-RPC corpus replay:
- first failure class
- unresolved-name counts
- runtime status
- performance manifest fields

## Design Decisions
1. Reuse `M7.1` harness and keep one deterministic run contract
- No new ad-hoc script.
- Existing run-root artifact model preserved and expanded.

2. Add explicit compile-only runtime status instead of fake runtime replay
- Because this harness compiles with `-c`, runtime replay is intentionally not executed.
- Runtime metric field is still tracked with explicit non-runtime states:
  - `not_run_compile_only`
  - `not_run_compile_failed`
  - `not_run_compile_timeout`

3. Track performance via two deterministic sources
- wall-clock compile elapsed (`compile_elapsed_ms`)
- transpile stage timing (`FRAGILEC_TRANSPILE_STAGE_TIMING_PATH`) with:
  - `parse_ms`, `export_ms`, `enrichment_ms`, `codegen_ms`, `total_ms`, `status`

4. Keep first-failure classification deterministic and aligned with existing conventions
- `none`
- `compile_timeout`
- `duplicate_definition_e0428`
- `unresolved_name_or_type_e0425`
- `other_rustc_error`
- `non_rustc_error`

5. Unresolved-name metric uses explicit `E0425` count
- `count_error_e0425_occurrences(stderr)` per fixture/backend
- aggregate totals + candidate-vs-baseline deltas in manifest

## Wrong-Approach Compliance
Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:
- no fake semantic stubs/fallback method bodies
- no target-specific parser/codegen hacks
- no force-native bypasses
- no rollback-pattern expansion

## Real Run Evidence
Command:

```bash
python3 scripts/parser_shadow_non_rpc_corpus.py --compile-timeout-seconds 180
```

Run root:
- `/tmp/fragile_m7_2_shadow_non_rpc_20260319T004733Z_p3550552`

Key summary from `shadow_non_rpc_manifest.txt`:
- `task_leaf=M7.2`
- `parity_metrics_version=1`
- `fixture_count=8`
- baseline (`libtooling`): `baseline_success_count=7`, `baseline_failure_count=1`
- candidate (`fragile-parser-clang`): `candidate_success_count=8`, `candidate_failure_count=0`
- `candidate_non_worsening_vs_baseline=true`
- `baseline_first_failure_class=other_rustc_error`
- `candidate_first_failure_class=none`
- `baseline_unresolved_name_e0425_total=0`
- `candidate_unresolved_name_e0425_total=0`
- `candidate_runtime_status_counts=not_run_compile_only:8`
- `baseline_transpile_timing_present_count=8`
- `candidate_transpile_timing_present_count=8`
- `transpile_total_ms_delta_vs_baseline=927`
- `missing_required_artifact_count=0`

RPC queue artifact remains explicit:
- `rpc_corpus_queue_for_m9.txt`
- `queued_item_001_todo=M9.1`
- `queued_item_002_todo=M9.2`
- `queued_item_003_todo=M9.3`

## User Manual
Default run:

```bash
python3 scripts/parser_shadow_non_rpc_corpus.py
```

Override corpus entries:

```bash
python3 scripts/parser_shadow_non_rpc_corpus.py \
  --fixture tests/cpp/add_simple.cpp \
  --fixture tests/cpp/factorial.cpp
```

Key flags:
- `--baseline-backend` (default: `libtooling`)
- `--candidate-backend` (default: `fragile-parser-clang`)
- `--compile-timeout-seconds`
- `--run-root`
- `--fragilec-bin`
- `--skip-fragilec-build`
