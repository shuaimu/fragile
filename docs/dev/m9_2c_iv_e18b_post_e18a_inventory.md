# M9.2.c.iv.e.18.b post-e.18.a strict inventory and bounded E0599 slice

## Scope
- Leaf: `M9.2.c.iv.e.18.b`
- Goal: re-run strict inventory after `e.18.a`, execute one bounded dominant residual slice (<1000 LOC), and publish deterministic non-increase evidence.
- Selected bounded slice: residual `E0599` method-surface misses (`op_call`, `swap`, `p`) from the post-`e.18.a` replay inventory.

## Plan Before Execution
1. Re-run strict replay inventory on `debugging/misc/basetypes/logging` with the same gnu++23 + full include profile used in prior leaves.
2. Select one bounded dominant residual class from fresh inventory (`E0599` method-surface misses).
3. Implement a generic compat-surface fix in `ast_codegen` (no target-specific behavior).
4. Add focused unit tests for the fix and duplicate-guard behavior.
5. Re-run strict replay inventory and publish deterministic before/after deltas.

## Wrong-Approach Check
- Re-read dev-book section `1.3` and `docs/dev/wrong.md` before patching.
- No target-specific conditionals (`mako`, `rpcbench`, `test_rpc`) were added.
- No force-native bypass or parser-backend rollback path was used.
- Change is a generic compat-surface completion in codegen post-processing, not a target-only source hack.

## Implementation Summary
- Extended `normalize_final_rpc_straggler_artifacts` with bounded residual-method compat traits:
  - `FragileStdFunctionVoidCompat` for `std_function_void___::op_call`
  - `FragileThreadSwapCompat` for `thread::swap`
  - `FragileUnitParamCompat` for unit-type `.p()` degradation
- Re-ran `normalize_final_rpc_straggler_artifacts` at the pipeline tail so late normalizations cannot reintroduce these method-surface misses.
- Added focused unit tests:
  - `test_normalize_final_rpc_straggler_artifacts_adds_std_function_void_op_call_compat`
  - `test_normalize_final_rpc_straggler_artifacts_adds_thread_swap_compat`
  - `test_normalize_final_rpc_straggler_artifacts_adds_unit_param_p_compat`
  - (plus existing atomic/chrono compat tests in the same slice).

## Strict Replay Profile
- Command shape per file:
  - `FRAGILEC_MODE=strict <fragilec> -c vendor/mako/src/rrr/base/<file>.cpp -std=gnu++23 -w` with full mako/gtest/rusty-cpp include tree.
- Baseline run-root (post-`e.18.a`, before `e.18.b` fix):
  - `/tmp/fragile_e18b_before_clean_Ehe6DE`
- After run-root (post `e.18.b` bounded slice):
  - `/tmp/fragile_e18b_after3_clean_phPOXq`

## Non-Increase Evidence

| Metric | Baseline | After | Delta |
|---|---:|---:|---:|
| Total errors | 251 | 246 | -5 |
| `E0425` | 63 | 63 | 0 |
| `E0599` | 6 | 1 | -5 |
| no method named `op_call` | 3 | 0 | -3 |
| no method named `swap` | 1 | 0 | -1 |
| no method named `p` | 1 | 0 | -1 |
| no method named `op_inc` | 1 | 1 | 0 |

Per-file (`total / E0425 / E0599`):
- `debugging`: `60/17/1 -> 59/17/0`
- `misc`: `61/17/1 -> 60/17/0`
- `basetypes`: `51/6/2 -> 50/6/1`
- `logging`: `79/23/2 -> 77/23/0`

Residual after this bounded slice:
- single `E0599` entry: `chrono_nanoseconds::op_inc` (`basetypes` lane).

## Validation
- Focused unit slice:
  - `cargo test -p fragile-clang normalize_final_rpc_straggler_artifacts -- --nocapture`
- Strict replay verification:
  - baseline and after run-roots above with same compile profile.
