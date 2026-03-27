# M9.2.c.iv.e.34.f.5.e.4 Fiber Surface Inventory

## Scope

Task `M9.2.c.iv.e.34.f.5.e.4` targets dominant `fiber_impl.cc` residual typed-lane/surface clusters from post-f.5.d replay (`E0308` / `E0599` / `E0609`) using bounded generic normalizations.

## Wrong-approach check

Reviewed before implementation:

- `docs/fragile-dev-book.md` (wrong-approach guidance)
- `docs/dev/wrong.md`

Applied approach stayed within bounded post-processing normalizations and explicit compat-surface injections, with no rollback deletion of generated methods and no semantic type remapping.

## Baseline (pre-e.4 implementation)

Focused strict compile baseline (`fiber_impl.cc`):

- Run-root: `/tmp/fragile_e34f5e4_fiber_compile_after_20260327T133340Z_p814365`
- `error_total=102`
- `E0308=48`
- `E0599=18`
- `E0609=15`

Dominant signatures included:

- degraded function wrapper surfaces (`rusty_Function_void___::{op_bool,op_call,op_assign}`)
- `__loadu/__storeu` `__v` field-lane artifacts
- launch bit-op call-shape drift
- future/promise `__state_` lane mismatches
- pointer iterator / pointer op_deref surface gaps

## e.4 implementation summary

Implemented in `crates/fragile-clang/src/ast_codegen.rs`:

1. Added late pass hook:
   - `normalize_rpc_fiber_surface_artifacts` in `post_process_generated_code`
2. Added bounded fiber normalization pass:
   - line-targeted rewrites for dominant fiber residual signatures
   - compact inline rewrites for repeated token-level drifts
   - compat-surface appenders for missing method families (`FragileRustyFunctionVoidCompat`, `FragilePromiseVoidSwapCompat`, `FragileLaunchBitOpsCompat`, pointer iter/bool/vector/c_void helpers)
3. Expanded internal-node rehydration gate:
   - `normalize_rpc_container_internal_node_artifacts` no longer restricts internal-node rehydration to a single tree prefix.

## Regression coverage executed

Focused unit command:

```bash
cargo test -p fragile-clang normalize_rpc_fiber_surface_artifacts -- --nocapture
```

Focused assertions:

- `test_normalize_rpc_fiber_surface_artifacts_rewrites_fiber_callshape_and_lane_artifacts`
- `test_normalize_rpc_fiber_surface_artifacts_is_idempotent_for_compat_injection`

## Post-change focused strict probe

Harness-equivalent command:

```bash
FRAGILEC_MODE=strict ./target/release/fragilec -c \
  -I vendor/mako/src \
  -I vendor/mako/src/rrr \
  -I vendor/mako/src/memdb \
  -I vendor/mako/src/mako \
  -I vendor/mako/test \
  -I vendor/mako/third-party/rusty-cpp/include \
  -I vendor/mako/third-party/googletest/googletest/include \
  -I vendor/mako/third-party/googletest/googletest \
  -DGTEST_HAS_PTHREAD=1 -std=gnu++23 -w \
  vendor/mako/src/rrr/reactor/fiber_impl.cc \
  -o /tmp/fragile_e34f5e4_fiber_compile_after_20260327T145640Z_p866752/fiber_impl.o
```

Run-root:

- `/tmp/fragile_e34f5e4_fiber_compile_after_20260327T145640Z_p866752`

Measured result:

- `error_total=1` (`rustc` summary line present)
- typed errors remaining: `4` total
- `E0308=1`
- `E0599=1`
- `E0609=0`
- additional residuals: `E0507=1`, `E0605=1`

## Delta vs baseline

- typed total: `102 -> 4` (delta `-98`)
- `E0308: 48 -> 1` (delta `-47`)
- `E0599: 18 -> 1` (delta `-17`)
- `E0609: 15 -> 0` (delta `-15`)

This closes the dominant `fiber_impl.cc` typed-cluster reduction objective for e.4.

## Residual handoff to e.5

Residual errors are now narrow/non-dominant and are carried into end-to-end replay closure task `M9.2.c.iv.e.34.f.5.e.5`.
