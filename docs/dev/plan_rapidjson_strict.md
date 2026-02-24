# Plan: RapidJSON Strict Support

## Goal

Make RapidJSON's no-STL example path work under strict mode with `fragilec` as the compiler driver, without native fallback.

Strict means:
- `FRAGILEC_MODE=strict`
- no pass-through compile/link behavior
- end-to-end compile and run uses transpiled Rust/object outputs only

## Scope

In scope:
- `example/condense/condense.cpp`
- `example/pretty/pretty.cpp`
- existing strict driver harness in `crates/fragile-clang/tests/real_world_rapidjson_tests.rs`

Out of scope (for this phase):
- full RapidJSON upstream test matrix
- STL-enabled RapidJSON paths
- performance optimization

## Definition of Done

1. `test_rapidjson_fragilec_driver_no_stl_examples_local_fixture_success` passes.
2. `test_real_world_rapidjson_fragilec_native_no_stl_examples_baseline` passes when run with `--ignored`.
3. Both generated binaries (`condense`, `pretty`) compile and run with exit code `0`.
4. No pass-through/native fallback is used for compile or link.

## Current Gap Snapshot

Latest strict RapidJSON failures cluster into:
- unresolved dependent template spellings leaking into emitted Rust
- duplicate symbol/type generation in large TU output
- placeholder/stub collisions and invalid item generation
- method calls lowered on raw pointers without required deref/adaptation
- missing builtin shim coverage for math/compiler builtins
- enum/alias normalization mismatches across namespaced and flattened forms

## Milestones

### M1: Stable Repro and Error Bucketing
- Keep one deterministic failing repro for external RapidJSON strict baseline.
- Bucket diagnostics by root-cause family and track counts per run.
- Freeze a minimal failing subset for fast iteration.

### M2: Type and Template Hygiene
- Block unresolved dependent type spellings from code emission paths.
- Ensure type-name sanitation produces valid Rust identifiers consistently.
- Prevent invalid placeholder generation from non-item-like names.

### M3: Duplicate Emission Control
- Eliminate duplicate function/type emission in single TU generation.
- De-dup alias and placeholder generation using canonical keys.
- Add regression tests for duplicate symbol/type scenarios.

### M4: Pointer/Method Lowering Correctness
- Fix method-call lowering on raw pointers in strict output.
- Ensure mutable/const receiver decisions remain consistent for generated calls.
- Add focused tests around pointer receiver calls.

### M5: Builtin Shim Completion
- Add missing builtin mappings required by RapidJSON no-STL paths.
- Keep mappings explicit and covered by unit tests.
- Verify generated output no longer fails on missing builtin symbols.

### M6: Enum/Alias Normalization
- Normalize namespaced enum/typedef aliases used in RapidJSON diagnostics and matching.
- Ensure match arms and type references use one canonical representation.
- Add tests for cross-namespace alias consistency.

### M7: Lock and Gate
- Unignore and enforce real-world strict RapidJSON baseline in CI once green.
- Keep local fixture tests as fast guardrails.
- Record expected strict command path in manifest/log assertions.

## Validation Commands

Primary:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests -- --nocapture
```

Strict external baseline:

```bash
cargo test -p fragile-clang --test real_world_rapidjson_tests \
  test_real_world_rapidjson_fragilec_native_no_stl_examples_baseline \
  -- --ignored --nocapture --test-threads=1
```

## Constraints

- Do not reintroduce pass-through mode.
- Do not add native compiler fallback in strict mode.
- Keep strict behavior deterministic and loggable through existing harness manifests.
