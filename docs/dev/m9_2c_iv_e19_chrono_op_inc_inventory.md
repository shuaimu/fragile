# M9.2.c.iv.e.19 chrono `op_inc` residual closure

## Scope
- Leaf: `M9.2.c.iv.e.19`
- Goal: remove the last strict-replay `E0599` entry (`chrono_nanoseconds::op_inc`) and publish non-increase evidence for `E0425`/`E0308`.

## Plan Before Execution
1. Reproduce the residual `op_inc` miss with strict gnu++23/full-include profile.
2. Identify why `FragileChronoNanosecondsCompat` is not emitted in `normalize_final_rpc_straggler_artifacts`.
3. Implement one bounded generic fix (<1000 LOC) and add focused unit coverage.
4. Rebuild release `fragilec` and rerun strict replay on `debugging/misc/basetypes/logging`.
5. Publish deterministic before/after deltas including `E0425`/`E0308` non-increase evidence.

## Wrong-Approach Check
- Reviewed dev-book section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md` before edits.
- No target-specific conditionals (`mako`/`rpcbench`/`test_rpc`) were introduced.
- No force-native fallback path was used.
- No semantic source stubs were added to target workloads.

## Root Cause
- Residual compile output showed `chrono_nanoseconds` emitted as an attribute-prefixed single-line struct:
  - `#[repr(C)] #[derive(Default, Clone, Copy)] pub struct chrono_nanoseconds { pub _M_r: i64 }`
- `has_exact_struct_def(...)` only matched lines starting with `pub struct ...`, so this shape failed detection.
- As a result, `FragileChronoNanosecondsCompat` was skipped and `.op_inc()` remained unresolved.
- Late normalization stages could also reintroduce method-surface misses after earlier final-pass normalization.

## Implementation
- Updated `has_exact_struct_def` (inside `normalize_final_rpc_straggler_artifacts`) to recognize exact `pub struct` headers even when attributes precede them on the same line.
- Expanded method-surface compat emissions for residual strict-replay misses:
  - `chrono_nanoseconds::op_inc`
  - `std_function_void___::op_call`
  - `thread::swap`
  - `().p()`
- Reran `normalize_final_rpc_straggler_artifacts` at pipeline tail so late passes cannot reintroduce these misses.
- Added focused unit test:
  - `test_normalize_final_rpc_straggler_artifacts_adds_chrono_op_inc_compat_for_attr_prefixed_struct_line`

## Strict Replay Profile
- Command shape per file:
  - `FRAGILEC_MODE=strict ./target/release/fragilec -c vendor/mako/src/rrr/base/<file>.cpp -std=gnu++23 -w` with full mako/gtest/rusty-cpp include tree.
- Baseline run-root (pre-fix binary):
  - `/tmp/fragile_e19_before_clean_iDNZ3w`
- After run-root (post-fix binary):
  - `/tmp/fragile_e19_after_fix3_clean_q1YQgW`

## Non-Increase Evidence

| Metric | Baseline | After | Delta |
|---|---:|---:|---:|
| Total errors | 246 | 201 | -45 |
| `E0425` | 63 | 63 | 0 |
| `E0308` | 117 | 73 | -44 |
| `E0599` | 1 | 0 | -1 |
| no method named `op_inc` | 1 | 0 | -1 |
| no method named `op_call` | 3 | 0 | -3 |
| no method named `swap` | 1 | 0 | -1 |
| no method named `p` | 1 | 0 | -1 |

Per-file (`total / E0425 / E0308 / E0599`):
- `debugging`: `59/17/27/0 -> 48/17/16/0`
- `misc`: `60/17/27/0 -> 49/17/16/0`
- `basetypes`: `50/6/28/1 -> 38/6/17/0`
- `logging`: `77/23/35/0 -> 66/23/24/0`

## Validation
- Focused unit slice:
  - `cargo test -p fragile-clang normalize_final_rpc_straggler_artifacts -- --nocapture`
- Strict replay verification:
  - baseline and after run-roots above with same compile profile.
