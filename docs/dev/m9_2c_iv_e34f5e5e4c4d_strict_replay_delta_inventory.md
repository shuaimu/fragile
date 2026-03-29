# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.d - Strict Replay Delta Inventory

## Scope
Bounded closure for leaf `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.d`:
- rerun strict runtime replay end-to-end after `c.4.c.4`,
- capture deterministic lane-contract/manifests,
- verify green lane contract or deterministic blocker non-increase vs baseline.

No parser/codegen source edits were required for this leaf; this is replay evidence capture.

## Command
```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260329T040328Z_p2989433
```

## Run Roots
- Baseline: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T040328Z_p2989433`
- Post-`c.4.d`: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`

## Lane Contract Result
From `strict_runtime_replay_manifest.txt`:
- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`
- `runtime_all_trials_passed=false`

Lane contract remains red for this replay.

## Deterministic Delta Result
From `strict_runtime_replay_blocker_inventory_manifest.txt`:
- `rustc_error_total_count: 218 -> 218`
- `rustc_error_unique_count: 89 -> 89`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`
- `first_error_key=error:fragilec:[fragilec] fragile rustc object compile failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/reactor.cc (parser-output-handoff)`

Representative parser-output-handoff object failures in this run:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

## Conclusion
Leaf `c.4.d` is closed via deterministic replay evidence: strict lane contract is still red, but blocker inventory is stable and non-increasing versus the immediate baseline.

## Wrong-Approach Check
Reviewed against `docs/fragile-dev-book.md` and `docs/dev/wrong.md`:
- no target-specific conditionals introduced,
- no force-native bypass usage,
- no fake semantic stubs or rollback deletions,
- evidence-only replay capture for this leaf.
