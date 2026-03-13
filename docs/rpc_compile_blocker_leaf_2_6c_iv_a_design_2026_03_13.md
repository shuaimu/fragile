# RPC Compile Blocker Leaf 2.6.c.iv.a Design (2026-03-13)

## Goal

Add deterministic hotspot profiling artifacts for strict timeout replay of
`src/rrr/base/misc.cpp` so subsequent optimization leaves can target verified
codegen hotspots instead of guessing.

## Scope

- Instrument `normalize_problematic_callshape_artifacts` with optional profile
  output keyed by `FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH`.
- Emit deterministic key/value metrics:
  - normalizer status (`codegen_started`, `not_invoked`, `invoking`, `started`,
    `in_progress`, `completed`)
  - input/output byte counts
  - line-level bulk-rewrite counters
  - elapsed milliseconds for completed/in-progress snapshots
- Keep runtime behavior unchanged when profiling env var is unset.

## Design

1. Add optional profile path lookup helper:
   - `problematic_callshape_profile_output_path()`
2. Add deterministic profile writer helper:
   - `write_problematic_callshape_profile(...)`
3. Seed profile status at codegen entry (`generate`) and before invoking
   `normalize_problematic_callshape_artifacts` to distinguish whether timeout
   occurs before the callshape normalizer is reached.
4. In `normalize_problematic_callshape_artifacts`, emit:
   - `started` snapshot
   - periodic `in_progress` snapshots every 2048 processed lines
   - `completed` snapshot at function return
5. Add focused unit regression proving profile emission and stable metrics.

## Wrong-Approach Check

Aligned with `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- No RPC-target-specific code paths.
- No semantic stub/fake method body synthesis.
- No force-native bypass.
- Failures/timeouts remain explicit; instrumentation only adds observability.

## Validation Commands

- `cargo test -p fragile-clang problematic_callshape -- --nocapture`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_a_callshape_profile_120_v4.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_a_stage_timing_120_v4.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_iii_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`

## Observed Outcome

- Timeout replays now emit deterministic profile artifacts even when codegen
  times out before reaching the callshape normalizer body.
- Both 120s and 300s replay profiles show `status=codegen_started` with zero
  callshape counters, indicating the timeout occurs earlier in codegen than
  `normalize_problematic_callshape_artifacts`.
