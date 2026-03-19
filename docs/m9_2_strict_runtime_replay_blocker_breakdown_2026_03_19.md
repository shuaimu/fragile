# M9.2.c Strict Runtime Replay Blocker Breakdown (2026-03-19)

## Scope

TODO leaf `M9.2.c` requires one passing strict runtime replay run-root for fragilec lane (`test_rpc` + rpcbench runtime).

## Task Sizing Analysis

A direct fix for the current blocker is not a small patch.

- Observed failures are parser-output mapping-completeness errors across multiple covered STL families (`optional`, `string`, `tuple`, `variant`, `map`) and many compile units under `vendor/mako/src/rrr`.
- This implies coordinated parser/codegen mapping normalization work spanning many callsites and regression fixtures.
- Estimated implementation size is likely well above the ~1000 LOC guideline for a single leaf.

Because of that, `M9.2.c` was decomposed in `TODO.md` into smaller leaves (`M9.2.c.i` through `M9.2.c.iv`).

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section 1.3 and `docs/dev/wrong.md`.

- No target-specific parser/codegen hacks were introduced.
- No force-native bypass (`FRAGILEC_FORCE_NATIVE_SOURCES`) was used.
- No parser-core escape hatch bypass was used.
- No fake semantic stubs were introduced to force a green build.

## Replay Attempts and Deterministic Evidence

### Current pinned blocker run

- Run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260319T123532Z_p1154760`
- Manifest highlights:
  - `lane_fragilec_build_status=2`
  - `lane_fragilec_failure_class=build_failed`
  - `lane_fragilec_test_rpc_status=-1`
  - `runtime_all_trials_passed=false`
  - `missing_required_artifact_count=0`
- First failure class (from `lane_fragilec/build.stderr`): parser-output mapping-completeness checks reject non-canonical alias targets / unresolved placeholder structs for covered families.

### Earlier runs

- `/tmp/fragile_m9_2_strict_runtime_replay_20260319T093009Z_p683647`: `build_timeout`.
- `/tmp/fragile_m9_2_strict_runtime_replay_20260319T122235Z_p1110089`: interrupted long build while using debug fragilec.
- `/tmp/fragile_m9_2_strict_runtime_replay_20260319T123323Z_p1146128`: stale release fragilec mismatch (`unsupported FRAGILEC_PARSER_BACKEND value ... expected libtooling`).

## Work Completed in This Iteration

- Executed strict replay attempts and captured deterministic blocker artifacts.
- Updated `TODO.md` with smaller `M9.2.c` leaf decomposition (`M9.2.c.i`..`M9.2.c.iv`).
- Marked `M9.2.c.i` done with pinned evidence.
- Recorded rationale and blocker taxonomy in this document and in `docs/fragile-dev-book.md`.

## Next Leaf Sequence

- `M9.2.c.ii`: canonicalize `optional`/`string` mapping aliases in active parser-output handoff.
- `M9.2.c.iii`: canonicalize `tuple`/`variant`/`map` mapping aliases and close unresolved placeholders.
- `M9.2.c.iv`: rerun strict runtime replay to reach passing build + runtime manifests.
