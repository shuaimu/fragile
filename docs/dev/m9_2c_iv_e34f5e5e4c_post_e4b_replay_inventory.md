# M9.2.c.iv.e.34.f.5.e.5.e.4.c.1 post-e.5.e.5.e.4.b replay inventory

## Scope

- Task leaf: `M9.2.c.iv.e.34.f.5.e.5.e.4.c.1`.
- Goal: capture deterministic strict replay evidence after `e.4.b` and publish a bounded decomposition for `e.4.c`.

## Wrong-approach check

- Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`.
- Reviewed `docs/dev/wrong.md`.
- No target-specific hacks, no force-native bypass, and no fake semantic stubs were introduced in this leaf.

## Replay command

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380
```

## Deterministic artifacts

- Run-root: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T125712Z_p2100737`
- Manifest: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T125712Z_p2100737/strict_runtime_replay_manifest.txt`
- Blocker manifest: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T125712Z_p2100737/strict_runtime_replay_blocker_inventory_manifest.txt`
- Build stderr: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T125712Z_p2100737/lane_fragilec/build.stderr`

## Lane-contract outcome

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`
- `runtime_all_trials_passed=false`

## Blocker inventory snapshot

- `rustc_error_total_count=82`
- `rustc_error_unique_count=55`
- `first_error_key=error:fragilec:[fragilec] fragile rustc object compile failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/quorum_event.cc (parser-output-handoff)`
- Baseline run-root: `/tmp/fragile_m9_2_strict_runtime_replay_20260328T092947Z_p1922380`
- Non-increase comparison:
  - `non_increase_total_vs_baseline=false`
  - `non_increase_unique_vs_baseline=false`
  - `non_increase_verdict=false`

Dominant families from build stderr:

- `E0599=28`
- `E0277=17`
- `E0308=10`
- `E0425=5`

Dominant failing files:

- `quorum_event.cc` (parser-output-handoff compile failure)
- `reactor.cc` (parser-output-handoff compile failure)
- `rpc/client.cpp` unresolved-type invariant: `rrr_Client_const`

## Bounded decomposition

1. `M9.2.c.iv.e.34.f.5.e.5.e.4.c.2`: reduce dominant `quorum_event.cc`/`reactor.cc` command-map + container/Rc surface regressions.
2. `M9.2.c.iv.e.34.f.5.e.5.e.4.c.3`: resolve residual unresolved-type/symbol gaps in `rpc/client.cpp` + reactor helpers.
3. `M9.2.c.iv.e.34.f.5.e.5.e.4.c.4`: rerun strict replay and verify full lane-contract closure.
