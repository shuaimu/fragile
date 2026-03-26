# M9.2.c.iv.e.34.f.5.a post-f.4 strict replay inventory and decomposition

Date: 2026-03-26  
Leaf: `M9.2.c.iv.e.34.f.5.a`

## Scope sizing (<1000 LOC)

- This leaf is inventory/decomposition only (no broad codegen rewrite).
- Changes are bounded to:
  - deterministic strict replay execution,
  - manifest/blocker inventory capture,
  - TODO decomposition update for follow-up bounded leaves.
- Total change size is well below 1000 LOC.

## Wrong-approach check

- Re-reviewed:
  - `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
  - `docs/dev/wrong.md`
- No target-specific hacks were introduced.
- No force-native bypass was used.
- No rollback-pattern or fake-stub expansion was introduced.

## Replay command and run root

Command:

```bash
FRAGILEC_MODE=strict \
python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260326T093427Z_p3345304
```

Run root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206`

## Lane contract result

From `strict_runtime_replay_manifest.txt`:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0`
- `runtime_all_trials_passed=false`

Blocker inventory summary:

- `blocker_error_total_count=478`
- `blocker_error_unique_count=94`
- `blocker_first_error_key=... marshal.cpp (parser-output-handoff)`
- baseline: `/tmp/fragile_m9_2_strict_runtime_replay_20260326T093427Z_p3345304`
- `blocker_non_increase_total_vs_baseline=true`
- `blocker_non_increase_unique_vs_baseline=true`
- `blocker_non_increase_verdict=true`

## Dominant blocker taxonomy

Top error classes (from `strict_runtime_replay_blocker_inventory_manifest.txt`):

- `E0609` = 154
- `E0599` = 127
- `E0308` = 121
- `E0282` = 13

Dominant lane/file local failures (`lane_fragilec/build.stderr`):

- `event.cc`: aborting due to 249 previous errors
- `fiber_impl.cc`: aborting due to 203 previous errors
- `marshal.cpp`: aborting due to 8 previous errors
- `fiber_context_runtime.cc`: aborting due to 2 previous errors

High-density symptom clusters in this run:

- `std::string` field-lane regressions (`data_`, `len_`, `capacity_`) and missing string helper surfaces (`grow`, `ensure_null_terminated`)
- degraded tree/internal-node field lanes (`__begin_node_`, `__end_node_`, `__size_`)
- unresolved/missing container method surfaces (`{begin,end,find,insert}` families)
- residual marshal/fiber-context lane mismatches (`rrr_Marshallable` fields, lifetime/type-lane noise)

## Bounded follow-up decomposition

To keep each closure slice below ~1000 LOC, `M9.2.c.iv.e.34.f.5` is decomposed into:

1. `M9.2.c.iv.e.34.f.5.b`:
   - shared `event.cc`/`fiber_impl.cc` `std::string` lane/surface regressions.
2. `M9.2.c.iv.e.34.f.5.c`:
   - shared `event.cc`/`fiber_impl.cc` container/internal-node lane regressions.
3. `M9.2.c.iv.e.34.f.5.d`:
   - residual `marshal.cpp`/`fiber_context_runtime.cc` blockers.
4. `M9.2.c.iv.e.34.f.5.e`:
   - strict replay rerun and lane contract closure verification.

## Evidence artifacts

- `/tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206/strict_runtime_replay_manifest.txt`
- `/tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206/strict_runtime_replay_blocker_inventory_manifest.txt`
- `/tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206/lane_fragilec/build.stderr`
