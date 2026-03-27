# M9.2.c.iv.e.34.f.5.e.1 post-f.5.d strict replay inventory

Date: 2026-03-27  
Leaf: `M9.2.c.iv.e.34.f.5.e.1`

## Scope sizing (<1000 LOC)

- Execute one strict runtime replay run with deterministic manifests.
- Capture blocker inventory and compare with prior baseline run-root.
- Publish bounded decomposition for follow-up closure leaves.
- No codegen surface expansion in this leaf.

## Wrong-approach check

Re-reviewed before execution:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Conformance:

- no target-specific `mako`/`rpcbench`/`test_rpc` conditionals,
- no force-native bypass,
- no fake-success stubs or rollback broadening.

## Replay command and run-root

Command:

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206
```

Run-root:

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T064414Z_p402022`

Primary manifests:

- `strict_runtime_replay_manifest.txt`
- `strict_runtime_replay_blocker_inventory_manifest.txt`
- `lane_fragilec/build.stderr`

## Lane contract outcome

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`
- `runtime_all_trials_passed=false`

Blocker inventory summary:

- `rustc_error_total_count=303`
- `rustc_error_unique_count=72`
- `first_error_key=... marshal.cpp (parser-output-handoff)`
- baseline: `/tmp/fragile_m9_2_strict_runtime_replay_20260326T205524Z_p4045206`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=true`
- `non_increase_verdict=true`

## Dominant residual taxonomy

Top typed error families from blocker inventory:

- `E0308=118`
- `E0599=85`
- `E0609=34`
- `E0282=13`
- `E0606=6`

Failing compile units:

- `/home/shuai/workspace/fragile/vendor/mako/src/rrr/misc/marshal.cpp`
- `/home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/event.cc`
- `/home/shuai/workspace/fragile/vendor/mako/src/rrr/reactor/fiber_impl.cc`

Notable first residuals:

- `marshal.cpp`: `E0499` mutable borrow overlap in `track_write_2` call lane.
- `event.cc`: unresolved surface/type clusters (`fseeko`, `print_stack_trace`, `__emplace_unique`, string/view lane mismatches).
- `fiber_impl.cc`: large residual typed-lane/surface cluster continuing post-f.5.d.

## Bounded decomposition for e.34.f.5.e

Follow-up leaves:

1. `M9.2.c.iv.e.34.f.5.e.2` marshal residual closure (`E0499` and adjacent marshal lane artifacts).
2. `M9.2.c.iv.e.34.f.5.e.3` event residual closure (dominant `E0308`/`E0599`/`E0609`/`E0282`).
3. `M9.2.c.iv.e.34.f.5.e.4` fiber_impl residual closure (dominant typed-lane/surface regressions).
4. `M9.2.c.iv.e.34.f.5.e.5` full strict replay rerun and final lane contract verification.

This closes `M9.2.c.iv.e.34.f.5.e.1` and keeps `M9.2.c.iv.e.34.f.5.e` open pending bounded implementation leaves.
