# RPC Compile Blocker Leaf 2.6.c.iv.d.i Design Note (2026-03-13)

## Scope

Leaf: `2.6.c.iv.d.i`  
Objective: extend strict timeout profiling with deterministic codegen checkpoint
history so `2.6.c.iv.d` optimization targeting is evidence-driven beyond the
single `codegen_started` marker.

## Size/Complexity Check

This leaf is a small focused instrumentation/test update.
Estimated LOC impact: small (`<500 LOC`) across one Rust file plus docs/TODO.
No broad subsystem refactor.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no target-name-specific code paths
- no force-native fallback or escape hatch
- no fake semantic method bodies/stubs
- generic codegen instrumentation and deterministic replay evidence only

## Execution Plan

1. Extend profiling payload to preserve status history across writes.
2. Add explicit codegen phase checkpoints before the problematic-callshape
   normalizer.
3. Add focused regression coverage for checkpoint history behavior.
4. Rebuild release `fragilec` and capture strict replay evidence at 120s/300s.

## Changes Implemented

Updated `crates/fragile-clang/src/ast_codegen.rs`:

- `write_problematic_callshape_profile` now emits `status_history=` by appending
  to prior status payload when the profile file already exists.
- `problematic_callshape_profile_output_path` now supports optional
  `FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_OWNER_THREAD` gating so only the
  owning thread writes profile artifacts when profiling is enabled in
  parallel-test contexts.
- Added helper `write_problematic_callshape_codegen_checkpoint(...)`.
- `AstCodeGen::generate` now emits deterministic checkpoints:
  - `codegen_started`
  - `codegen_after_template_collection`
  - `codegen_after_template_instantiation_generation`
  - `codegen_after_top_level_generation`
  - `codegen_after_stub_generation`
  - existing `not_invoked` / `invoking` statuses via the shared helper
- Added focused test:
  - `test_generate_problematic_callshape_profile_records_codegen_checkpoint_history`

## Commands Executed

```bash
cargo test -p fragile-clang problematic_callshape -- --nocapture

cargo build --release -p fragile-cli --bin fragilec

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_i_callshape_profile_120_v2.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_i_stage_timing_120_v2.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_c_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120

FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_i_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_i_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_iv_c_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300
```

## Deterministic Evidence

Replay manifest (`/tmp/fragile_rpc_leaf_2_6c_iv_c_build_only_20260313/rpc_compile_blocker_replay_manifest.txt`):

- `timeout_seconds=120` and `timeout_seconds=300` runs both show:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_blocker_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`

Checkpoint profiles:

- 120s (`/tmp/fragile_rpc_leaf_2_6c_iv_d_i_callshape_profile_120_v2.txt`):
  - `status=codegen_started`
  - `status_history=codegen_started`
- 300s (`/tmp/fragile_rpc_leaf_2_6c_iv_d_i_callshape_profile_300_v1.txt`):
  - `status=codegen_after_template_instantiation_generation`
  - `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`

Stage timing traces:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_i_stage_timing_120_v2.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_i_stage_timing_300_v1.txt`

Both traces show `export`/`parse`/`enrichment` completion and entry into
`codegen`, confirming timeout remains codegen-bound while checkpoint progression
is now measurable.

## Outcome

Leaf `2.6.c.iv.d.i` is complete. The new checkpoint history narrows the next
hot-path target to work between
`codegen_after_template_instantiation_generation` and
`codegen_after_top_level_generation` for leaf `2.6.c.iv.d.ii`.
