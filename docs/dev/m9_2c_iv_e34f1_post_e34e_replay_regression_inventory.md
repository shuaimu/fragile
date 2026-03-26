# M9.2.c.iv.e.34.f.1 post-e.34.e strict replay regression inventory

Date: 2026-03-26  
Leaf: `M9.2.c.iv.e.34.f.1`

## Scope sizing (<1000 LOC)

- This leaf is inventory + decomposition only: no production code edits.
- Work performed:
  - re-run strict runtime replay end-to-end,
  - capture deterministic run-root/manifests,
  - classify blocker mix by file and error class,
  - publish bounded follow-up leaf ordering.

## Wrong-approach check

- Re-reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md` before replay/decomposition.
- No target-specific source hacks, no force-native bypasses, no fake behavior stubs were introduced in this leaf.

## Command and run root

Command:

```bash
python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260325T233520Z_p2595863
```

Run root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260326T093427Z_p3345304`

## Strict replay manifest summary

From `strict_runtime_replay_manifest.txt`:

- `harness_status=1` (`insufficient_data` comparison verdict accepted for M9.2 runtime-only mode)
- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_completed_trials=0`
- `lane_fragilec_failure_class=build_failed`
- `runtime_all_trials_passed=false`
- `runtime_trial_passed_count=0`
- `runtime_trial_failed_count=1`

From `strict_runtime_replay_blocker_inventory_manifest.txt`:

- `rustc_error_total_count=637`
- `rustc_error_unique_count=144`
- `first_error_key=error:fragilec:[fragilec] fragile rustc object compile failed for /home/shuai/workspace/fragile/vendor/mako/src/rrr/misc/marshal.cpp (parser-output-handoff)`
- baseline (`/tmp/fragile_m9_2_strict_runtime_replay_20260325T233520Z_p2595863`): `total=93`, `unique=38`
- `non_increase_total_vs_baseline=false`
- `non_increase_unique_vs_baseline=false`
- `non_increase_verdict=false`

## Failing file taxonomy

From `lane_fragilec/build.stderr` section markers:

- `rrr/reactor/event.cc`: `306` typed rustc errors
  - dominant classes: `E0609=112`, `E0599=77`, `E0308=67`
- `rrr/reactor/fiber_impl.cc`: `259` typed rustc errors
  - dominant classes: `E0609=112`, `E0308=55`, `E0599=49`
- `rrr/misc/marshal.cpp`: `43` typed rustc errors
  - dominant classes: `E0599=19`, `E0609=17`, `E0308=5`
- `rrr/reactor/fiber_context_runtime.cc`: `3` typed rustc errors
  - classes: `E0223=1`, `E0308=1`, `E0606=1`

Shared dominant signatures in `event.cc`/`fiber_impl.cc` include:

- parser-output syntax artifacts (`FiberContext { , ..Default::default() }`, `State { State::NEW }`),
- unresolved SIMD helper surfaces (`_mm_set1_epi8`, `_mm_cmpeq_epi8`, `_mm_movemask_epi8`, `_mm_and_si128`),
- container field-lane drift (`__tree_` vs `__table_`),
- compatibility surface misses (`std_shared_ptr` operator methods, filebuf field naming lanes).

Marshal lane residuals include:

- `std_shared_ptr` operator surface (`op_arrow`, `op_eq`) and downstream pointer flow,
- marshaling API surface regressions (`read`/`write`/`peek`/`op_shl`/`op_shr` lane mismatches),
- type-lane bridge artifacts (`bookmark`, enum/int and pointer/reference mismatches).

## Bounded follow-up decomposition

1. `M9.2.c.iv.e.34.f.2`:
  - fix shared `event.cc`/`fiber_impl.cc` syntax + intrinsic artifacts.
2. `M9.2.c.iv.e.34.f.3`:
  - fix shared `event.cc`/`fiber_impl.cc` container/smart-pointer/filebuf compatibility lanes.
3. `M9.2.c.iv.e.34.f.4`:
  - fix `marshal.cpp` residual compatibility lanes.
4. `M9.2.c.iv.e.34.f.5`:
  - re-run strict runtime replay and verify final lane/rpcbench contract pass.

This keeps each follow-up leaf bounded and independently verifiable with focused compile probes before the full end-to-end replay rerun.
