# M9.2.c.iv.e.18.a std_atomic compat coverage (strict replay evidence)

## Scope
- Leaf: `M9.2.c.iv.e.18.a`
- Goal: eliminate residual `E0599` misses in the `std_atomic_int` / `std_atomic_bool` lane (`store` / `load`) without introducing target-specific behavior.
- Boundedness: localized to `normalize_final_rpc_straggler_artifacts` and focused tests in `ast_codegen` (<1000 LOC).

## Plan Before Execution
1. Confirm dominant residual lane from prior replay artifacts (`/tmp/fragile_e17c_after_release2_nOGJrO`).
2. Extend generic atomic compat injection so std-prefixed atomic stubs receive the same trait/impl coverage as legacy atomic stubs.
3. Add duplication guards so pre-existing impl blocks are not emitted twice.
4. Rebuild release `fragilec` and rerun strict replay across `debugging/misc/basetypes/logging` with gnu++23 + full include profile.
5. Publish deterministic before/after deltas and residual inventory.

## Wrong-Approach Check
- Re-read `docs/dev/wrong.md` and dev-book wrong-approach rules before implementation.
- No target-specific conditionals (`mako`, `rpcbench`, `test_rpc`) were introduced.
- No force-native bypass or backend rollback path was used.
- No semantic-stub shortcut: change is a generic compat-surface completion for generated atomic stubs only.

## Implementation Summary
- In `normalize_final_rpc_straggler_artifacts`, atomic compat trait emission now triggers when either legacy or std-prefixed atomic stubs are present:
  - `atomic_int` or `std_atomic_int`
  - `atomic_bool` or `std_atomic_bool`
- Added std-prefixed impl emission:
  - `impl FragileAtomicIntCompat for std_atomic_int`
  - `impl FragileAtomicBoolCompat for std_atomic_bool`
- Added duplicate guards for each concrete impl target to prevent double emission.
- Added unit tests validating add/skip behavior for std-prefixed impls.

## Strict Replay Profile
- Command shape: `FRAGILEC_MODE=strict ./target/release/fragilec -c vendor/mako/src/rrr/base/{debugging,misc,basetypes,logging}.cpp -std=gnu++23 -w` with full mako/gtest/rusty-cpp include tree.
- Baseline run-root: `/tmp/fragile_e17c_after_release2_nOGJrO`
- After run-root: `/tmp/fragile_e17e_after_D0aQTo`

## Non-Increase Evidence

| Metric | Baseline | After | Delta |
|---|---:|---:|---:|
| `error[E0599]` total | 13 | 6 | -7 |
| no method named `store` | 5 | 0 | -5 |
| no method named `load` | 2 | 0 | -2 |
| `std_atomic_int` mention lines | 18 | 0 | -18 |
| `std_atomic_bool` mention lines | 3 | 0 | -3 |

Residual `E0599` classes after this leaf:
- `op_call` on `std_function_void___` (3)
- `op_inc` on `chrono_nanoseconds` (1)
- `swap` on `&mut thread` (1)
- `p` on `()` (1)

## Validation
- Targeted unit slice:
  - `cargo test -p fragile-clang normalize_final_rpc_straggler_artifacts -- --nocapture`
- Full regression gates executed after implementation (see commit-level test log):
  - `cargo test --workspace --all-targets`
  - `python3 -m unittest discover -s tests/python -p 'test_*.py'`
