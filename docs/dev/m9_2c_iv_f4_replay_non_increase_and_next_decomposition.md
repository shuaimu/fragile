# M9.2.c.iv.f.4 strict replay non-increase and next decomposition

## Scope

- Leaf: `M9.2.c.iv.f.4`.
- Bounded execution-only slice (<1000 LOC): strict replay rerun, deterministic baseline comparison, and next bounded decomposition publication.

## Wrong-Approach Check

Checked before running the leaf:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No force-native bypasses, no target-specific conditionals, and no semantic stubs were introduced.

## Replay Command

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053 \
  --trials 1 \
  --skip-masstree-perf-target \
  --skip-clean-step
```

## Artifacts

- Baseline run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`
- Replay run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835`
- Replay manifest:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/strict_runtime_replay_manifest.txt`
- Replay blocker inventory manifest:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260330T130048Z_p617835/strict_runtime_replay_blocker_inventory_manifest.txt`

## Lane Contract Result

From `strict_runtime_replay_manifest.txt`:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`
- `runtime_all_trials_passed=false`

`M9.2.c.iv` cannot close on this replay because lane-green contract is still unmet.

## Deterministic Non-Increase vs f.1 Baseline

From `strict_runtime_replay_blocker_inventory_manifest.txt`:

- baseline totals: `rustc_error_total_count=218`, `rustc_error_unique_count=89`
- replay totals: `rustc_error_total_count=153`, `rustc_error_unique_count=85`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`

## Post-f.4 Dominant Residual Codes

Grouped from replay blocker inventory counts:

| error code | count |
| --- | ---: |
| `E0599` | 27 |
| `E0308` | 25 |
| `E0277` | 20 |
| `E0425` | 14 |
| `E0609` | 14 |

## Next Bounded Decomposition

Since lane-green is not reached, the next bounded cycle is published as `M9.2.c.iv.f.5`:

1. `M9.2.c.iv.f.5.a`: capture deterministic post-f.4 residual bucket manifest (`code -> compile-unit -> exemplar`).
2. `M9.2.c.iv.f.5.b`: execute first bounded dominant `E0599` compatibility-surface slice (`<=400 LOC`).
3. `M9.2.c.iv.f.5.c`: execute next bounded dominant `E0308` value-shape slice (`<=400 LOC`).
4. `M9.2.c.iv.f.5.d`: execute one bounded supporting `E0277/E0425/E0609` slice (`<=300 LOC`).
5. `M9.2.c.iv.f.5.e`: rerun strict replay and verify deterministic non-increase vs f.1 and f.4 roots.
