# M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.4 - Strict Replay Delta Inventory

## Scope
Bounded closure for leaf `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4.c.4`:
- rerun strict runtime replay after `c.4.c.1`..`c.4.c.3`,
- capture deterministic lane manifest and blocker inventory,
- confirm green lane contract or deterministic non-increase vs baseline.

No parser/codegen source edits were required for this leaf; this is replay evidence capture.

## Command
```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T230041Z_p2676907
```

## Run Roots
- Baseline: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T230041Z_p2676907`
- Post-`c.4.c.*`: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T040328Z_p2989433`

## Lane Contract Result
From `strict_runtime_replay_manifest.txt`:
- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`
- `runtime_all_trials_passed=false`

Lane contract is still red for this replay.

## Deterministic Delta Result
From `strict_runtime_replay_blocker_inventory_manifest.txt`:
- `rustc_error_total_count: 264 -> 218`
- `rustc_error_unique_count: 91 -> 89`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`
- `first_error_key=error:fragilec:[fragilec] fragile rustc object compile failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/reactor.cc (parser-output-handoff)`

Representative surfaced failing objects in this run:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

## Conclusion
Leaf `c.4.c.4` is closed by deterministic replay evidence with blocker non-increase, while strict lane contract remains red. Follow-on closure proceeds under sibling leaf `c.4.d`.

## Wrong-Approach Check
Reviewed against `docs/fragile-dev-book.md` and `docs/dev/wrong.md`:
- no target-specific conditionals introduced,
- no force-native bypass usage,
- no fake semantic stubs or rollback deletions,
- evidence-only replay capture for this leaf.
