# M9.2.c.iv.e.34.a — Post-e.33 Strict Replay Blocker Inventory

Date: 2026-03-25

## Task sizing analysis
- Parent task: `M9.2.c.iv.e.34` (full strict runtime replay contract closure).
- Initial assumption after e.33 was that end-to-end replay should be mostly orchestration-only.
- Live replay disproved that assumption: build lane failed with a broad newly surfaced blocker mix (`93` total / `38` unique error keys) across four source files and one parser-output mapping-completeness gate.
- Conclusion: direct completion of e.34 is not a bounded sub-1000-LOC change; decomposition into focused leaves is required before rerunning the end-to-end gate.

## Plan before execution
1. Run strict runtime replay with deterministic run-root capture.
2. Validate lane contract/manifests.
3. If lane contract fails, capture blocker inventory and classify dominant blocker families/files.
4. Expand e.34 into bounded follow-up leaves.

## Wrong-Approach Check
- Re-read `docs/dev/wrong.md` before triage.
- No rollback-pattern additions.
- No target-specific bypasses or force-native fallback.
- No fake/stub success markers; contract outcome recorded as failed where observed.

## Replay command and run root
- Command: `python3 scripts/mako_rpc_strict_runtime_replay.py`
- Run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260325T233520Z_p2595863`
- Run-root naming contract: valid (`run_root_name_is_contract_valid=true`).

## Lane contract outcome
From `strict_runtime_replay_manifest.txt` and `benchmark_harness_manifest.txt`:
- `harness_status=1`
- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `runtime_trial_passed_count=0`
- `runtime_trial_failed_count=1`

This does not satisfy e.34 acceptance (`build=0`, `test_rpc=0`, `failure_class=none`).

## Blocker inventory summary
From `strict_runtime_replay_blocker_inventory_manifest.txt`:
- `rustc_error_total_count=93`
- `rustc_error_unique_count=38`
- `first_error_key=...strop.cpp (parser-output-handoff)`

Dominant error keys:
- `31` × `E0425: cannot find type chunk in this scope` (`marshal.cpp`)
- `11` × `E0308: mismatched types` (primarily `strop.cpp`)
- `5` × `E0599: no method named op_add_assign for std_string` (`strop.cpp`)
- `4` × `E0599: no method named op_arrow for std_shared_ptr<T>`
- `3` × `E0425: cannot find type void in this scope` (`strop.cpp`)
- `3` × lifetime errors (`marshal.cpp`)

First failing compile/transpile units observed in `lane_fragilec/build.stderr`:
- `rrr/base/strop.cpp` (typed rustc compile failure)
- `rrr/reactor/epoll_wrapper.cc` (typed rustc compile failure)
- `rrr/misc/marshal.cpp` (typed rustc compile failure)
- `rrr/reactor/event.cc` (parser-output mapping-completeness gate failure)

## Mapping-completeness blocker details (event.cc)
- `event.cc` transpilation failed with covered `map` family alias canonical-target violations.
- Representative shape: aliases like `std_map_iterator_...` resolved to non-canonical `std___map_iterator_...` targets (expected prefix `std_map`).

## Decomposition decision
Because the failure surface spans distinct families and files, e.34 was decomposed into bounded leaves:
- `e.34.b`: event.cc mapping-completeness canonical target normalization.
- `e.34.c`: marshal.cpp `chunk` unresolved-type cluster.
- `e.34.d`: strop.cpp typed mismatch and missing-surface cluster.
- `e.34.e`: epoll_wrapper + marshal residual compat/lifetime blockers.
- `e.34.f`: rerun full strict runtime replay contract after blocker leaves close.
