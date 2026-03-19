# M7.1 Shadow Mode Non-RPC Corpus Replay (2026-03-18)

## Scope and Size Check
`M7.1` was scoped to a single deterministic harness plus focused tests, well under the requested ~1000 LOC envelope.

Implemented surface:
- `scripts/parser_shadow_non_rpc_corpus.py`
- `tests/python/test_parser_shadow_non_rpc_corpus.py`

## Design Rationale
Goal for this leaf is operational shadow evidence, not speculative parity policy expansion (`M7.2`) and not blocker closure work (`M7.3`).

Implemented behavior:
- run strict `fragilec` compile across a representative non-RPC fixture corpus under:
  - baseline backend: `libtooling`
  - candidate backend: `fragile-parser-clang`
- emit deterministic per-fixture logs and aggregate manifest:
  - `shadow_non_rpc_manifest.txt`
  - `shadow_non_rpc_required_artifacts_manifest.txt`
  - `shadow_non_rpc_commands.txt`
- emit explicit deferred RPC queue artifact for `M9` closure:
  - `rpc_corpus_queue_for_m9.txt`
  - queue items map directly to `M9.1`, `M9.2`, `M9.3`

Default representative non-RPC corpus:
- `tests/cpp/add_simple.cpp`
- `tests/cpp/factorial.cpp`
- `tests/cpp/namespace.cpp`
- `tests/cpp/class.cpp`
- `tests/cpp/constructor.cpp`
- `tests/cpp/grammar/14_struct_constructor.cpp`
- `tests/clang_integration/namespace_resolution.cpp`
- `tests/clang_integration/virtual_class.cpp`

## Wrong-Approach Compliance
Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:
- no semantic stubs/fake method bodies
- no target-specific hacks for RPC/non-RPC behavior
- no force-native bypasses
- no rollback pattern expansion

## Execution Evidence
Command:

```bash
python3 scripts/parser_shadow_non_rpc_corpus.py --compile-timeout-seconds 180
```

Run root:
- `/tmp/fragile_m7_1_shadow_non_rpc_20260318T225545Z_p3360421`

Summary from `shadow_non_rpc_manifest.txt`:
- `fixture_count=8`
- baseline (`libtooling`): `baseline_success_count=6`, `baseline_failure_count=2`
- candidate (`fragile-parser-clang`): `candidate_success_count=8`, `candidate_failure_count=0`
- `candidate_non_worsening_vs_baseline=true`
- `fixture_non_worsening_count=8`
- `missing_required_artifact_count=0`

Observed baseline failure examples (candidate passed both):
- `tests/cpp/constructor.cpp` (`E0599`, missing `Point::x_`/`Point::y_` methods)
- `tests/cpp/grammar/14_struct_constructor.cpp` (`E0599`, missing `Counter::get` method)

RPC queue evidence from `rpc_corpus_queue_for_m9.txt`:
- `rpc_targets=test_rpc,rpcbench`
- queued:
  - `M9.1`
  - `M9.2`
  - `M9.3`

## User Manual
Run with defaults:

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
