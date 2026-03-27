# M9.2.c.iv.e.34.f.5.e.5.a Post-e.5.e.4 Replay Inventory

## Scope

Task `M9.2.c.iv.e.34.f.5.e.5.a` captures a deterministic strict runtime replay inventory after e.5.e.4 and decomposes remaining blockers into bounded closure leaves.

## Wrong-approach check

Reviewed before decomposition:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Approach remains additive and bounded: no rollback deletion, no semantic type remapping, no native bypass.

## Replay command

```bash
FRAGILEC_MODE=strict python3 scripts/mako_rpc_strict_runtime_replay.py \
  --skip-fragilec-build \
  --baseline-run-root /tmp/fragile_m9_2_strict_runtime_replay_20260327T064414Z_p402022
```

## Run-root

- `/tmp/fragile_m9_2_strict_runtime_replay_20260327T172446Z_p981802`

## Lane contract result

From `strict_runtime_replay_manifest.txt`:

- `lane_fragilec_build_status=2`
- `lane_fragilec_test_rpc_status=-1`
- `lane_fragilec_failure_class=build_failed`
- `lane_fragilec_completed_trials=0/1`
- `runtime_all_trials_passed=false`

## Blocker inventory summary

From `strict_runtime_replay_blocker_inventory_manifest.txt`:

- `rustc_error_total_count=154`
- `rustc_error_unique_count=77`
- `first_error_key=event.cc parser-output-handoff compile failure`
- baseline root: `/tmp/fragile_m9_2_strict_runtime_replay_20260327T064414Z_p402022`
- `non_increase_total_vs_baseline=true`
- `non_increase_unique_vs_baseline=false`
- `non_increase_verdict=false`

Typed-family distribution:

- `E0599=56`
- `E0308=25`
- `E0277=18`
- `E0425=17`
- `E0133=7`
- `E0609=6`

Failing object files:

- `event.cc`
- `fiber_impl.cc`
- `quorum_event.cc`
- `reactor.cc`

Generated-file concentration from `lane_fragilec/build.stderr`:

- `reactor.rs` markers: `120`
- `event.rs` markers: `39`
- `quorum_event.rs` markers: `21`
- `fiber_impl.rs` markers: `4`

## Residual cluster decomposition

To keep each execution slice bounded (<1000 LOC), closure is decomposed under `M9.2.c.iv.e.34.f.5.e.5`:

1. `e.5.e.5.b`: shared cross-TU reactor-family stragglers (`print_stack_trace` path drift, `weak_ordering` return-lane mismatch, pointer-event `log` callshape drift).
2. `e.5.e.5.c`: `event.cc` `__string_view`/path/c_void lane cluster (`Default::default` on `c_void`, degraded `__compare` callshape, path unsafe-deref lane).
3. `e.5.e.5.d`: `quorum_event.cc`/`reactor.cc` command-map and event-base surface regressions (`rrr_Cmd*` symbol gaps, `Fiber::create_run__` drift, unordered-map `find/end/erase`, `IntEvent` base-field lanes).
4. `e.5.e.5.e`: end-to-end strict runtime replay rerun and lane-contract closure.

This closes `.a` (inventory/decomposition capture) while preserving deterministic evidence for follow-on bounded implementation leaves.
