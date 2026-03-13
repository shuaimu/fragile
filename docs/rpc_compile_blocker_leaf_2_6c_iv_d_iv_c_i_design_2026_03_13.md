# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.i Design Notes (2026-03-13)

## Scope

Leaf `2.6.c.iv.d.iv.c.i` requires deterministic replay-only evidence capture:

- refresh strict timeout-derived replay profiling/timing artifacts
- lock checkpoint/status and byte-volume baselines after `2.6.c.iv.d.iv.b`
- identify the next generic optimization target window for `c.ii`

## LOC Budget Analysis

This leaf is measurement/triage only and does not require parser/codegen
changes. Expected production-code LOC delta: `0`.

## Execution Plan

1. Rebuild release `fragilec` for deterministic replay inputs.
2. Run timeout-derived focused replay at 120s and 300s with fresh
   `FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH` and
   `FRAGILEC_TRANSPILE_STAGE_TIMING_PATH`.
3. Extract checkpoint status history, input byte volume, and replay timeout
   markers to lock baseline values.
4. Use locked baseline to drive the next optimization leaf (`c.ii`).

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` Section `1.3` and `docs/dev/wrong.md`:

- no target-specific code paths
- no force-native bypass
- no synthesized semantic fallback bodies
- deterministic replay evidence only

## Commands Executed

```bash
cargo build --release -p fragile-cli --bin fragilec
```

```bash
FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_callshape_profile_120_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_stage_timing_120_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 120
```

```bash
FRAGILEC_MODE=strict \
FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_callshape_profile_300_v1.txt \
FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_stage_timing_300_v1.txt \
python3 scripts/mako_rpc_compile_blocker_replay.py \
  --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 \
  --lanes fragilec \
  --max-replays 1 \
  --timeout-seconds 300
```

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## Deterministic Evidence

Artifacts:

- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_callshape_profile_120_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_stage_timing_120_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_callshape_profile_300_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_i_stage_timing_300_v1.txt`
- `/tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313/rpc_compile_blocker_replay_manifest.txt`

Key values:

- 120s profile:
  - `status=codegen_started`
  - `status_history=codegen_started`
- 300s profile:
  - `status=codegen_after_template_instantiation_generation`
  - `status_history=codegen_started,codegen_after_template_collection,codegen_after_template_instantiation_generation`
  - `input_bytes=574973`
- Replay manifest:
  - `replay_01_status=124`
  - `replay_01_timed_out=true`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`

Comparison baseline for targeting:

- `2.6.c.iv.d.iv.a` 300s profile input bytes: `573974`
- `2.6.c.iv.d.iv.c.i` 300s profile input bytes: `574973`
- Delta: `+999` bytes, with unchanged checkpoint window
  (`codegen_after_template_instantiation_generation`).

## Regression Suite Results

- `cargo test --workspace --all-targets`
  - `fragile-clang` lib: `726 passed`, `46 failed` (known baseline, unchanged)
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
  - `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.c.i` is complete. The dominant timeout checkpoint remains in
the pre-top-level codegen window, and the locked baseline confirms byte volume
did not improve versus `iv.a`, guiding `2.6.c.iv.d.iv.c.ii` to target the same
window with a new generic optimization.
