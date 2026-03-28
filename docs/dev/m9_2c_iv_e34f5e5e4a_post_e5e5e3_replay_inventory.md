# M9.2.c.iv.e.34.f.5.e.5.e.4.a Post-e.5.e.5.e.3 Strict Replay Inventory

## Scope

- Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.a`
- Goal: capture deterministic replay evidence after `e.5.e.5.e.3` and bound follow-up closure leaves.
- Replay command:

```bash
FRAGILEC_MODE=strict \
python3 scripts/mako_rpc_strict_runtime_replay.py \
  --skip-fragilec-build \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452
```

Wrong-approach check completed before decomposition:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

## Replay Result

- Run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380`
- Baseline run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T000000Z_p1395452`
- Lane contract status:
  - `lane_fragilec_build_status=2`
  - `lane_fragilec_test_rpc_status=-1`
  - `lane_fragilec_failure_class=build_failed`
  - `lane_fragilec_completed_trials=0/1`
- Blocker inventory summary:
  - `rustc_error_total_count=12`
  - `rustc_error_unique_count=10`
  - `first_error_key=error:fragilec:[fragilec] fragile rustc object compile failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/event.cc (parser-output-handoff)`
  - `non_increase_total_vs_baseline=true`
  - `non_increase_unique_vs_baseline=true`
  - `non_increase_verdict=true`

Primary residual typed blocker family from `lane_fragilec/build.stderr`:

- shared `invalid_null_arguments` abort in `event.cc` and `fiber_impl.cc`:
  - `std::slice::from_raw_parts(std::ptr::null() as *const u8, (self.len_) as usize)`

## Bounded Decomposition

1. `M9.2.c.iv.e.34.f.5.e.5.e.4.b`
   - Resolve shared `invalid_null_arguments` lane in event/fiber compare paths with bounded generic normalization and focused unit tests.
2. `M9.2.c.iv.e.34.f.5.e.5.e.4.c`
   - Re-run strict runtime replay and verify strict lane contract/manifests/rpcbench statuses pass.

## Evidence Files

- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380/strict_runtime_replay_manifest.txt`
- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380/strict_runtime_replay_blocker_inventory_manifest.txt`
- `/tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380/lane_fragilec/build.stderr`
