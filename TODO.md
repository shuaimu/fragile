# Fragile TODO

Last updated: 2026-03-17
Status: active plan is parser-first STL-opaque migration (LibTooling retirement path); RPC target closure is deferred to final hardening.

## Deprecation Notice
- [x] Previous active plan "RPC bring-up plan (active, 2026-03-12)" is deprecated and removed from this file.
- [x] Previous nested active items under that plan are deprecated and removed from this file.
- [x] Historical evidence remains available in git history for `TODO.md` and prior run artifacts.

## Parser Direction (Locked)
- [x] Do not parse deeply into STL internals.
- [x] For all STL usages, parser emits STL placeholder nodes only.
- [x] Codegen maps STL placeholders to pre-generated STL implementation only.
- [x] No STL source-to-source strict translation path in the active plan.

## Periodic Maintenance Cadence (Disabled)
- [x] Periodic 24-hour maintenance sweeps are disabled.
- [x] Active expanded periodic maintenance tasks are removed from this plan.

## Program Goal
Goal: replace LibTooling-centered parsing with a custom parser module that treats STL as opaque placeholders mapped to pre-generated STL codegen targets. `test_rpc` + `rpcbench` build/runtime/performance are lower-priority closure gates after parser cutover hardening.

## Priority Order (Highest to Lowest)
- [ ] P0 (highest): complete parser migration milestones M0-M8 and satisfy parser/regression gates.
- [ ] P1 (lower): complete RPC target closure milestone M9 and satisfy RPC build/runtime/performance gates.

## Non-Negotiable Constraints
- No `FRAGILEC_FORCE_NATIVE_SOURCES`.
- No target-specific hacks (`rpcbench*`, `test_rpc*`, `mako*` conditionals).
- No semantic stubs/fake function bodies to force compile success.
- Strict mode behavior and non-RPC coverage must remain green/non-regressing.
- Unknown STL shapes must fail deterministically with diagnostics (not silent fallback behavior).

## Program Acceptance Gates
- [ ] G1 STL Opaque Gate: parser emits placeholders (no deep STL subtree conversion for known STL symbols).
- [ ] G2 Mapping Gate: all emitted STL placeholders are resolved by pre-generated STL codegen mappings.
- [ ] G3 Regression Gate: touched subsystem tests + workspace/Python suites remain non-regressing.
- [ ] G4 Build Gate (deferred RPC): strict build succeeds for `test_rpc` and `rpcbench` with no force-native bypass.
- [ ] G5 Runtime Gate (deferred RPC): `test_rpc` exits successfully; `rpcbench` server/client run without crash/hang.
- [ ] G6 Performance Gate (deferred RPC): deterministic clang vs fragile comparison reports `fragile_avg_qps >= clang_avg_qps`.

## Milestone Roadmap

### M0) Baseline Freeze and Migration Harness
- [x] M0.1 Record baseline artifacts for parser migration runs (fixture corpus, stage timing, blocker inventory).
- [x] M0.2 Add parser-backend A/B harness for side-by-side run roots and deterministic manifest diffing.
- [x] M0.3 Define milestone run-root naming and required artifact contract.
Acceptance:
- [x] M0.A1 Baseline manifests are reproducible across two consecutive runs.
- [x] M0.A2 A/B harness can run old and new parser backends and emit comparable summaries.

### M1) Parser IR Contract with STL Placeholders
- [ ] M1.1 Define `ParserOutput v1` schema with explicit STL placeholder node kinds.
- [ ] M1.2 Add placeholder metadata contract: container family, element/key/value types, allocator/comparator/hash/equal policy shape, method/op selector.
- [ ] M1.3 Add deterministic serialization + fixture tests for placeholder IR.
Acceptance:
- [ ] M1.A1 Schema docs and fixture corpus are checked in.
- [ ] M1.A2 ParserOutput round-trip tests pass with deterministic output.

### M2) New Parser Module Bootstrap (Non-LibTooling Active Path)
- [ ] M2.1 Introduce `fragile-parser-core` trait and backend registry.
- [ ] M2.2 Implement `fragile-parser-clang` backend skeleton producing `ParserOutput v1`.
- [ ] M2.3 Wire transpiler entry points to backend trait behind feature/flag cutover switch.
Acceptance:
- [ ] M2.A1 New backend can parse and emit IR for a non-trivial fixture corpus.
- [ ] M2.A2 Pipeline compiles/runs without LibTooling-specific parse dependency in active path.

### M3) STL Boundary Detection (Opaque, Not Deep Parse)
- [ ] M3.1 Implement canonical STL symbol detection (`std::`, alias chains, using/typedef resolution).
- [ ] M3.2 Emit STL placeholders at boundary and stop deep subtree lowering for STL internals.
- [ ] M3.3 Add regression fixtures for common STL families (`vector`, `map`, `unordered_map`, `string`, `optional`, `variant`, `tuple`, `shared_ptr`, `unique_ptr`).
Acceptance:
- [ ] M3.A1 No deep STL AST subtrees appear in parser output for covered fixtures.
- [ ] M3.A2 Alias-heavy STL fixtures still resolve to correct placeholder families.

### M4) Pre-Generated STL Implementation Module
- [ ] M4.1 Create versioned pre-generated STL module layout and naming contract.
- [ ] M4.2 Implement/port required operations used by current benchmarks/tests (container ops, iterators, value semantics).
- [ ] M4.3 Add generation reproducibility checks and deterministic manifest for generated outputs.
Acceptance:
- [ ] M4.A1 Generated STL module is reproducible byte-for-byte from the same inputs.
- [ ] M4.A2 Required STL operation fixtures compile and execute successfully.

### M5) Codegen Placeholder Mapping Layer
- [ ] M5.1 Replace STL lowering paths with placeholder-to-pre-generated mapping in codegen.
- [ ] M5.2 Enforce mapping completeness checks (no silent fallback to legacy STL translation).
- [ ] M5.3 Add focused regressions for method/operator mapping correctness.
Acceptance:
- [ ] M5.A1 All placeholder nodes in corpus resolve to generated STL targets.
- [ ] M5.A2 No legacy deep STL translation path is invoked in active backend runs.

### M6) Diagnostic and Failure Policy for Unknown STL Shapes
- [ ] M6.1 Add deterministic error class for unsupported STL placeholder shapes.
- [ ] M6.2 Add actionable diagnostics payload (location, symbol, shape fingerprint, missing mapping key).
- [ ] M6.3 Add regression tests that assert failure is explicit and non-silent.
Acceptance:
- [ ] M6.A1 Unknown STL shapes fail with deterministic error class and metadata.
- [ ] M6.A2 No semantic stub/fake body is produced for unsupported shapes.

### M7) Shadow Mode and Parity Hardening
- [ ] M7.1 Run old/new parser backends in shadow mode on representative non-RPC corpus; queue RPC corpus for M9 closure.
- [ ] M7.2 Track parity metrics: first failure class, unresolved-name counts, runtime status, perf manifest fields.
- [ ] M7.3 Close parity blockers using generic fixes only.
Acceptance:
- [ ] M7.A1 New backend is non-worsening vs baseline on blocker class and unresolved-name deltas.
- [ ] M7.A2 Runtime behavior parity is established for covered smoke fixtures.

### M8) Cutover to New Parser Backend
- [ ] M8.1 Flip default parser backend to new module.
- [ ] M8.2 Keep temporary explicit escape hatch for one hardening window only.
- [ ] M8.3 Publish migration notes for developers and CI.
Acceptance:
- [ ] M8.A1 CI defaults use new backend with green required checks.
- [ ] M8.A2 Escape hatch usage is measured and trending to zero during hardening window.

### M9) Deferred RPC Target Closure (Lower Priority)
- [ ] M9.0 Start only after M0-M8 acceptance is complete.
- [ ] M9.1 Rebuild strict `test_rpc` + `rpcbench` with new parser backend and no force-native paths.
- [ ] M9.2 Run full strict runtime replay and capture deterministic runtime manifests.
- [ ] M9.3 Run deterministic clang vs fragile benchmark comparison and enforce no-regression gate.
Acceptance:
- [ ] M9.A1 `test_rpc` build/run pass gate is green.
- [ ] M9.A2 `rpcbench` server/client runtime gate is green.
- [ ] M9.A3 Performance gate (`fragile_avg_qps >= clang_avg_qps`) is green.

## Cross-Milestone Regression Gates (Required Each Iteration)
- [ ] R1 Focused touched-subsystem tests pass.
- [ ] R2 `cargo test --workspace --all-targets` non-regression check recorded.
- [ ] R3 `python3 -m unittest discover -s tests/python -p 'test_*.py'` pass/non-regression recorded.
- [ ] R4 Deterministic blocker inventory non-increase gate recorded when build is still red.

## RapidJSON Phase-2 Closure Ledger (Compatibility Snapshot)
Ordered failure-class clearance ledger (active sequence)
1) Parser/AST fidelity mismatch in real RapidJSON headers.
2) Duplicate symbol/type emission in single TU output.
3) Placeholder fallback for required rapidjson template types.
4) C/C++ type normalization gaps.
5) Cast/decay/call-shape lowering bugs.
6) Numeric/sign/enum lowering issues.
7) Entrypoint correctness residual (`main` rollback/drop).

- [x] Fix parser fidelity issue causing `document.h` const-member assignment failure.
- [x] Fix duplicate emission pipeline (helpers/types/templates) to eliminate `E0428` families.
- [x] Fix placeholder degradation for required rapidjson template types (`Reader`, handlers, writers, streams).
- [x] Fix libc/libstd type canonicalization (`__FILE`, atomic flag types, void aliases).
- [x] Fix array decay and pointer cast lowering (`[T; N]` to pointer forms).
- [x] Fix `main` rollback/drop behavior so real example `main` survives codegen + rustc object emission.

## Done Criteria
- [ ] D1 Milestones M0-M9 acceptance items all closed.
- [ ] D2 Program gates G1-G6 all green in one clean run window.
- [ ] D3 Old parser path removed from active production flow after hardening window.
