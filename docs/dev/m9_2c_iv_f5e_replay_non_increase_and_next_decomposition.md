# M9.2.c.iv.f.5.e strict replay non-increase and next decomposition

## Scope

- Leaf: `M9.2.c.iv.f.5.e`.
- Bounded execution-only slice (<1000 LOC): rerun strict runtime replay, verify deterministic non-increase against both required anchors (`f.1` baseline and `f.4` root), and publish the next bounded decomposition because lane-green is still unmet.

## Wrong-Approach Check

Checked before running the leaf:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no force-native bypass,
- no target-specific branch hacks,
- no semantic stub shortcuts to fake lane-green.

## Replay Command

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053 \
  --trials 1 \
  --skip-masstree-perf-target \
  --skip-clean-step
```

## Artifacts

- f.1 baseline root:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`
- f.4 replay root:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835`
- f.5.e replay root:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116`
- f.5.e replay manifests:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116/strict_runtime_replay_manifest.txt`
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T215446Z_p1184116/strict_runtime_replay_blocker_inventory_manifest.txt`

## Lane Contract Result

From `strict_runtime_replay_manifest.txt`:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`
- `runtime_all_trials_passed=false`

`M9.2.c.iv` cannot close on `f.5.e` because strict lane remains red.

## Deterministic Non-Increase Verification

### Versus f.1 baseline

From `strict_runtime_replay_blocker_inventory_manifest.txt` for f.5.e:

- `baseline_run_root=/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`
- `rustc_error_total_count=12`
- `rustc_error_unique_count=12`
- `baseline_error_total_count=218`
- `baseline_error_unique_count=89`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`

### Versus f.4 root

From f.4 blocker manifest (`/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/strict_runtime_replay_blocker_inventory_manifest.txt`):

- f.4 totals: `rustc_error_total_count=153`, `rustc_error_unique_count=85`

Compared with f.5.e totals (`12`, `12`):

- `total 12<=153` (non-increase)
- `unique 12<=85` (non-increase)

## Dominant Residual Blocker Signature

f.5.e replay first error key:

- `error:fragilec:[fragilec] fragile unresolved-type invariant failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/event.cc: rrr_Future_State`

Unresolved-type invariant failures captured in this replay:

- `event.cc`: `rrr_Future_State`
- `fiber_context_runtime.cc`: `rrr_Future_State`
- `fiber_impl.cc`: `rrr_Future_State`
- `quorum_event.cc`: `rrr_Future_State`

## Next Bounded Decomposition

Because lane-green is still unmet, publish the next bounded cycle under `M9.2.c.iv.f.6`:

1. `M9.2.c.iv.f.6.a` capture deterministic unresolved-type invariant manifest from f.5.e replay artifacts.
2. `M9.2.c.iv.f.6.b` execute one bounded unresolved-type rehydration slice (`<=300 LOC`) for `rrr_Future_State` reactor-family lanes.
3. `M9.2.c.iv.f.6.c` rerun focused strict probes and record unresolved-type invariant deltas with residual non-increase checks.
4. `M9.2.c.iv.f.6.d` rerun strict runtime replay and verify deterministic non-increase vs f.1/f.4/f.5.e anchors.
