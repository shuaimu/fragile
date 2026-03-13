# RPC Compile Blocker Leaf 2.6.c.iv.d.iv.c.iv.c.iii.a Design (2026-03-13)

## Scope and sizing

Leaf: `2.6.c.iv.d.iv.c.iv.c.iii.a`

Targeted change size was estimated well below the requested bound:

- code path update: `generate_fn_template_instantiations` (single function, small edit)
- focused regression: one unit test in `ast_codegen` test module
- total patch footprint: `~60` added lines / `~7` removed lines

## Problem

In the pre-top-level codegen window, function-template instantiation generation staged the entire pending map by cloning every key/value pair into an intermediate `Vec`. For large pending sets this adds avoidable cloning/allocation overhead.

## Wrong-approach check

Validated against `docs/fragile-dev-book.md` Section 1.3 and `docs/dev/wrong.md`:

- no RPC- or target-specific branching
- no native-source bypass / force-native fallback
- no fake semantic fallback bodies
- generic internal data-flow optimization only

## Implementation

Updated file:

- `crates/fragile-clang/src/ast_codegen.rs`

Key change:

- replaced clone-backed staging in `generate_fn_template_instantiations` with ownership transfer:
  - from cloning `pending_fn_instantiations.iter().map(...clone...)`
  - to `let instantiations = std::mem::take(&mut self.pending_fn_instantiations);`
- preserved behavior where newly discovered pending function instantiations are left in `pending_fn_instantiations` for subsequent iterations.

Focused regression added:

- `test_generate_fn_template_instantiations_consumes_pending_map_and_generates_functions`
- locks three behaviors:
  - concrete function emission still occurs (`pub fn twice_i32(...)`)
  - generated-function registry is updated
  - current pending map is consumed (no clone-backed staging residue)

## Validation

Executed commands:

- `cargo test -p fragile-clang test_generate_fn_template_instantiations_consumes_pending_map_and_generates_functions -- --nocapture`
- `cargo build --release -p fragile-cli --bin fragilec`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_a_callshape_profile_120_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_a_stage_timing_120_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 120`
- `FRAGILEC_MODE=strict FRAGILEC_PROBLEMATIC_CALLSHAPE_PROFILE_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_a_callshape_profile_300_v1.txt FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=/tmp/fragile_rpc_leaf_2_6c_iv_d_iv_c_iv_c_iii_a_stage_timing_300_v1.txt python3 scripts/mako_rpc_compile_blocker_replay.py --run-root /tmp/fragile_rpc_leaf_2_6c_i_build_only_20260313 --lanes fragilec --max-replays 1 --timeout-seconds 300`
- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Evidence highlights:

- 120s profile: `status=codegen_after_template_collection`
- 300s profile: `status=codegen_after_template_instantiation_generation`
- 300s profile `input_bytes=574875` (for reference, prior `c.iv.c.i` captured `573560`)
- replay remains timeout-bound on the same blocker TU:
  - `replay_01_status=124`
  - `replay_01_first_failure_class=build_timeout`
  - `replay_01_blocker_file=src/rrr/base/misc.cpp`
- full-suite baseline parity retained:
  - `cargo test --workspace --all-targets`: `fragile-clang` lib `730 passed / 46 failed` (failure count unchanged)
  - Python suite: `Ran 29 tests`, `OK (skipped=1)`

## Outcome

Leaf `2.6.c.iv.d.iv.c.iv.c.iii.a` is implemented with focused regression coverage and replay evidence captured. The primary strict replay blocker class remains `build_timeout` on `misc.cpp`; continuation is tracked by leaf `2.6.c.iv.d.iv.c.iv.c.iii.b`.
