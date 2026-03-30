# M9.2.c.iv.f.1 - Post-e.34 Residual Rebaseline Inventory

## Scope
Bounded closure for `M9.2.c.iv.f.1`:
- rebaseline the current strict replay residual blocker state using canonical terminal e.34 replay artifacts,
- record deterministic lane and blocker deltas,
- publish bounded next-leaf decomposition for the next closure cycle.

This leaf is evidence capture only; no parser/codegen source edits were made in this step.

## Artifacts
- Baseline run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T040328Z_p2989433`
- Rebaseline run root: `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`
- Rebaseline manifest:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053/strict_runtime_replay_manifest.txt`
- Rebaseline blocker inventory:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053/strict_runtime_replay_blocker_inventory_manifest.txt`

## Lane Contract Snapshot
From `strict_runtime_replay_manifest.txt` at run root `...053434Z_p3129053`:
- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`
- `runtime_all_trials_passed=false`

Lane remains red.

## Deterministic Blocker Delta
From `strict_runtime_replay_blocker_inventory_manifest.txt` at run root `...053434Z_p3129053`:
- `rustc_error_total_count: 218 -> 218` (vs baseline `...040328Z_p2989433`)
- `rustc_error_unique_count: 89 -> 89`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`
- `first_error_key=error:fragilec:[fragilec] fragile rustc object compile failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/reactor.cc (parser-output-handoff)`

Top residual keys in the rebaseline inventory include:
- `E0308:mismatched types` (`66`)
- `E0061:this method takes 0 arguments but 1 argument was supplied` (`7`)
- `E0599:no method named lock found for struct SpinMutex_Marshal in the current scope` (`5`)
- `E0282:type annotations needed` (`5`)
- `E0605:non-primitive cast ... as i32` (`4`)

Representative compile-unit failures remain:
- `reactor.cc`
- `rpc/client.cpp`
- `rpc/server.cpp`
- `rpc/utils.cpp`

## Next Bounded Leaves
`M9.2.c.iv.f.2` through `M9.2.c.iv.f.4` are reserved for:
1. dominant residual typed-cluster decomposition,
2. first bounded fix execution with focused probes,
3. end-to-end strict replay rerun and deterministic delta verification.

## Wrong-Approach Check
Re-reviewed section `1.3 Wrong Approaches (Do Not Do)` in `docs/fragile-dev-book.md` and `docs/dev/wrong.md`:
- no force-native bypass,
- no target-specific conditional hacks,
- no semantic stubs/fake bodies,
- no suppression-only edits without deterministic evidence.
