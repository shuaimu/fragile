# M9.2.c.iv.f.2 - Residual Typed-Error Cluster Decomposition

## Scope
Bounded closure for `M9.2.c.iv.f.2`:
- decompose the post-`f.1` strict-lane residual typed-error cluster into executable leaves,
- keep each fix leaf under ~1000 LOC total touched code,
- codify explicit no-shortcut constraints before implementation.

This leaf is decomposition/design only; it does not apply parser/codegen behavior changes.

## Inputs
From `f.1` inventory (`docs/dev/m9_2c_iv_f1_post_e34_residual_rebaseline_inventory.md`):
- replay roots:
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T040328Z_p2989433`
  - `/tmp/fragile_m9_2_strict_runtime_replay_20260329T053434Z_p3129053`
- stable residual lane contract:
  - `lane_fragilec_build_status=2`
  - `lane_fragilec_test_rpc_status=-1`
  - `lane_fragilec_failure_class=build_failed`
- dominant typed/error keys:
  - `E0308:mismatched types` (`66`)
  - `E0061:this method takes 0 arguments but 1 argument was supplied` (`7`)
  - `E0599:no method named lock found for struct SpinMutex_Marshal in the current scope` (`5`)
  - `E0282:type annotations needed` (`5`)
  - `E0605:non-primitive cast ... as i32` (`4`)

## Wrong-Approach Check
Re-reviewed:
- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails for all `f.2.*` follow-up leaves:
- no `FRAGILEC_FORCE_NATIVE_SOURCES`,
- no target-specific hacks (`rpcbench*`, `test_rpc*`, `mako*` conditionals),
- no semantic stubs/fake bodies to force compile success,
- no suppression-only edits without deterministic replay evidence.

## Decomposition (Bounded Leaves)

### M9.2.c.iv.f.2.a - Deterministic bucket manifest
Goal:
- produce a typed-error bucket manifest keyed by `error-key -> compile-unit -> exemplar signature` for:
  - `reactor.cc`
  - `rpc/client.cpp`
  - `rpc/server.cpp`
  - `rpc/utils.cpp`

Bound:
- data/manifest extraction + test updates only (`<=250 LOC` expected).

Exit evidence:
- deterministic bucket manifest doc + closure tests asserting stable key set and compile-unit coverage.

### M9.2.c.iv.f.2.b - Dominant E0308 bucket-B1 slice
Goal:
- implement the first dominant `E0308` slice for direct value-shape mismatches:
  - pointer/null mutability lane mismatches,
  - direct assignment/return type-shape mismatches that do not require semantic stubs.

Bound:
- focused normalizer/codegen edits (`<=400 LOC`).

Exit evidence:
- focused compile probes on the four residual compile units show non-increase and targeted `E0308` bucket reduction.

### M9.2.c.iv.f.2.c - E0061/E0599 compatibility slice
Goal:
- close method-surface arity/signature residuals (`E0061`) and lock-like missing method surfaces (`E0599`) using generic compatibility normalization.

Bound:
- focused surface/normalization edits (`<=300 LOC`).

Exit evidence:
- focused probes show reduction in `E0061`/`E0599` typed buckets without regressions in other major classes.

### M9.2.c.iv.f.2.d - E0282/E0605 inference-cast slice
Goal:
- close residual inference (`E0282`) and non-primitive cast (`E0605`) buckets via generic inference/cast normalization.

Bound:
- focused inference/cast edits (`<=300 LOC`).

Exit evidence:
- focused probes show reduction in `E0282`/`E0605` buckets and deterministic non-increase against prior probe baseline.

### M9.2.c.iv.f.2.e - Re-probe and replay comparison
Goal:
- run residual compile probes + strict replay inventory comparison versus `f.1` baseline,
- record deterministic non-increase and identify the next dominant residual bucket.

Bound:
- orchestration/doc/test updates (`<=250 LOC` expected).

Exit evidence:
- updated inventory comparison with explicit `total/unique` non-increase verdict and next-bucket ownership leaf.

## Sequencing
Execution order:
1. `f.2.a`
2. `f.2.b`
3. `f.2.c`
4. `f.2.d`
5. `f.2.e`

This sequencing preserves deterministic measurement first, then dominant-to-minor bucket closure, then replay validation.
