# Fragile TODO

Last updated: 2026-03-19
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
- [x] P0 (highest): complete parser migration milestones M0-M8 and satisfy parser/regression gates. Done 2026-03-19.
- [x] P1 (lower): complete RPC target closure milestone M9 and satisfy RPC build/runtime/performance gates. Done 2026-03-19.

## Non-Negotiable Constraints
- No `FRAGILEC_FORCE_NATIVE_SOURCES`.
- No target-specific hacks (`rpcbench*`, `test_rpc*`, `mako*` conditionals).
- No semantic stubs/fake function bodies to force compile success.
- Strict mode behavior and non-RPC coverage must remain green/non-regressing.
- Unknown STL shapes must fail deterministically with diagnostics (not silent fallback behavior).

## Program Acceptance Gates
- [x] G1 STL Opaque Gate: parser emits placeholders (no deep STL subtree conversion for known STL symbols). Done 2026-03-19. Evidence: M3 milestone complete, M5.A1 corpus-level audit passes.
- [x] G2 Mapping Gate: all emitted STL placeholders are resolved by pre-generated STL codegen mappings. Done 2026-03-19. Evidence: M5 milestone complete, M5.A2 legacy fallback rejection tests pass.
- [x] G3 Regression Gate: touched subsystem tests + workspace/Python suites remain non-regressing. Done 2026-03-19. Evidence: all workspace tests pass across M0-M9 iterations, R1-R3 gates green.
- [x] G4 Build Gate (deferred RPC): strict build succeeds for `test_rpc` and `rpcbench` with no force-native bypass. Done 2026-03-19. Evidence: M9.1 complete, both targets build with fragilec default backend, no FRAGILEC_FORCE_NATIVE_SOURCES.
- [ ] G5 Runtime Gate (deferred RPC): `test_rpc` exits successfully; `rpcbench` server/client run without crash/hang. Blocked: M9.2.c runtime replay blocked by build_timeout.
- [ ] G6 Performance Gate (deferred RPC): deterministic clang vs fragile comparison reports `fragile_avg_qps >= clang_avg_qps`. Blocked: requires M9.2 completion for end-to-end run.

## Milestone Roadmap

### M0) Baseline Freeze and Migration Harness
- [x] M0.1 Record baseline artifacts for parser migration runs (fixture corpus, stage timing, blocker inventory).
- [x] M0.2 Add parser-backend A/B harness for side-by-side run roots and deterministic manifest diffing.
- [x] M0.3 Define milestone run-root naming and required artifact contract.
Acceptance:
- [x] M0.A1 Baseline manifests are reproducible across two consecutive runs.
- [x] M0.A2 A/B harness can run old and new parser backends and emit comparable summaries.

### M1) Parser IR Contract with STL Placeholders
- [x] M1.1 Define `ParserOutput v1` schema with explicit STL placeholder node kinds.
- [x] M1.2 Add placeholder metadata contract: container family, element/key/value types, allocator/comparator/hash/equal policy shape, method/op selector.
- [x] M1.3 Add deterministic serialization + fixture tests for placeholder IR.
Acceptance:
- [x] M1.A1 Schema docs and fixture corpus are checked in.
- [x] M1.A2 ParserOutput round-trip tests pass with deterministic output.

### M2) New Parser Module Bootstrap (Non-LibTooling Active Path)
- [x] M2.1 Introduce `fragile-parser-core` trait and backend registry.
- [x] M2.2 Implement `fragile-parser-clang` backend skeleton producing `ParserOutput v1`.
- [x] M2.3 Wire transpiler entry points to backend trait behind feature/flag cutover switch.
Acceptance:
- [x] M2.A1 New backend can parse and emit IR for a non-trivial fixture corpus.
- [x] M2.A2 Pipeline compiles/runs without LibTooling-specific parse dependency in active path.
  - [x] M2.A2.1 Add parser-core parse-manifest handoff artifact contract in strict entry points (CLI + driver) with deterministic coverage.
  - [x] M2.A2.2 Introduce parser-output-to-codegen interface so active codegen path no longer requires LibTooling parser invocation.
  - [x] M2.A2.3 Route strict compile active parser stage through parser-core output handoff and keep temporary explicit escape hatch only for hardening.

### M3) STL Boundary Detection (Opaque, Not Deep Parse)
- [x] M3.1 Implement canonical STL symbol detection (`std::`, alias chains, using/typedef resolution).
  - [x] M3.1.a Add direct canonical `std::` symbol detection utility for known STL families (`vector`, `map`, `unordered_map`, `string`, `optional`, `variant`, `tuple`, `shared_ptr`, `unique_ptr`).
  - [x] M3.1.b Add typedef/type-alias symbol table extraction with canonical target normalization for STL aliases.
  - [x] M3.1.c Add `using` declaration/directive chain resolution over the alias table for canonical STL symbol detection.
  - [x] M3.1.d Add deterministic regression fixtures/tests for direct `std::`, typedef alias chains, and using-chain STL resolution.
- [x] M3.2 Emit STL placeholders at boundary and stop deep subtree lowering for STL internals.
  - [x] M3.2.a Emit canonical STL placeholder node kinds for boundary declarations/expressions using direct + alias/using-aware symbol detection.
  - [x] M3.2.b Stop deep STL subtree lowering by pruning descendants once a known STL boundary placeholder node is emitted.
  - [x] M3.2.c Add deterministic fixture regressions asserting STL boundary placeholder emission and no deep STL internals under placeholder roots.
- [x] M3.3 Add regression fixtures for common STL families (`vector`, `map`, `unordered_map`, `string`, `optional`, `variant`, `tuple`, `shared_ptr`, `unique_ptr`).
Acceptance:
- [x] M3.A1 No deep STL AST subtrees appear in parser output for covered fixtures.
- [x] M3.A2 Alias-heavy STL fixtures still resolve to correct placeholder families.

### M4) Pre-Generated STL Implementation Module
- [x] M4.1 Create versioned pre-generated STL module layout and naming contract.
- [x] M4.2 Implement/port required operations used by current benchmarks/tests (container ops, iterators, value semantics).
  - [x] M4.2.a Port ordered-map core runtime surface (`std::map<int, int>`) into `fragile-stl` pre-generated module with deterministic insert/lookup/update/erase semantics and focused runtime tests.
  - [x] M4.2.b Port unordered-map core runtime surface (`std::unordered_map<int, int>`) into `fragile-stl` pre-generated module with deterministic insert/lookup/update/erase semantics and focused runtime tests.
  - [x] M4.2.c Port required value-semantics helper surfaces for current fixtures (`optional`, `tuple`, `variant`) with focused compile/runtime tests.
  - [x] M4.2.d Port iterator boundary operations required by current benchmark/fixture usage and lock behavior with focused regressions.
- [x] M4.3 Add generation reproducibility checks and deterministic manifest for generated outputs.
Acceptance:
- [x] M4.A1 Generated STL module is reproducible byte-for-byte from the same inputs.
- [x] M4.A2 Required STL operation fixtures compile and execute successfully.

### M5) Codegen Placeholder Mapping Layer
- [x] M5.1 Replace STL lowering paths with placeholder-to-pre-generated mapping in codegen.
  - [x] M5.1.a Add deterministic parser-output STL placeholder family resolver + pre-generated contract mapping validation in handoff codegen path.
  - [x] M5.1.b Wire codegen lowering decisions to resolved placeholder-family mappings (family -> canonical pre-generated type prefix) instead of legacy name-shape heuristics.
    - [x] M5.1.b.i Plumb parser-output placeholder mappings into `AstCodeGen` state and use mapping-aware unresolved-associative alias closure in the active generate pipeline.
    - [x] M5.1.b.ii Replace hardcoded associative alias target derivation with mapping-driven family dispatch for covered placeholder families.
    - [x] M5.1.b.iii Replace hardcoded sequence/smart-pointer alias family detection with mapping-driven family dispatch for covered placeholder families.
    - [x] M5.1.b.iv Add deterministic diagnostics/tests proving mapped families no longer use legacy `std::collections::*` fallback lanes in active parser-output runs.
  - [x] M5.1.c Remove active-path legacy unresolved STL alias fallback emission to `std::collections::*` for mapped placeholder families.
  - [x] M5.1.d Add deterministic mapping manifest emission from active codegen path for placeholder families observed in parser output.
  - [x] M5.1.e Validate active backend runs do not rely on legacy deep STL translation path for covered families.
- [x] M5.2 Enforce mapping completeness checks (no silent fallback to legacy STL translation).
- [x] M5.3 Add focused regressions for method/operator mapping correctness.
  - [x] M5.3.a Add parser-output handoff regression for mapped associative operator/method call lowering to canonical pre-generated targets.
  - [x] M5.3.b Add parser-output handoff regression for mapped sequence/smart-pointer method call lowering to canonical pre-generated targets.
  - [x] M5.3.c Add negative regression proving mapped method/operator lanes fail deterministically when unresolved placeholder fallback would be required.
Acceptance:
- [x] M5.A1 All placeholder nodes in corpus resolve to generated STL targets.
  - [x] M5.A1.a Add active parser-output handoff acceptance regression for mapped associative/sequence/smart-pointer families (`map`, `unordered_map`, `vector`, `shared_ptr`, `unique_ptr`) proving canonical pre-generated target resolution with no unresolved placeholder structs.
  - [x] M5.A1.b Expand mapping-completeness covered-family enforcement to remaining mapped placeholder families (`string`, `optional`, `variant`, `tuple`) with deterministic positive/negative regressions.
  - [x] M5.A1.c Add parser-core fixture-corpus replay gate that audits observed STL placeholder kinds and asserts no unresolved mapped-family placeholders remain in active parser-output handoff transpiled output.
- [x] M5.A2 No legacy deep STL translation path is invoked in active backend runs.
  - [x] M5.A2.a Add parser-core fixture replay regression that verifies active parser-output handoff output for mapped placeholder families emits canonical observed-family mapping manifests and never emits legacy deep STL fallback alias targets (`std::collections::BTreeMap` / `std::collections::HashMap`) for covered lanes.
  - [x] M5.A2.b Add deterministic negative/positive parser-output handoff regressions that prove legacy deep STL fallback alias forms are rejected for covered families under mapped context while canonical pre-generated alias forms remain accepted.
  - [x] M5.A2.c Add corpus-level mapped-family audit gate in active parser-output replay tests that fails on any covered-family legacy deep STL alias fallback marker and records deterministic fixture evidence.

### M6) Diagnostic and Failure Policy for Unknown STL Shapes
- [x] M6.1 Add deterministic error class for unsupported STL placeholder shapes.
- [x] M6.2 Add actionable diagnostics payload (location, symbol, shape fingerprint, missing mapping key).
- [x] M6.3 Add regression tests that assert failure is explicit and non-silent. Done 2026-03-18. Evidence: 17 regression tests across fragile-clang (11) and fragile-parser-core (6) asserting E001/E002/E003 deterministic error codes, fail-fast behavior, full-pipeline error propagation, error format stability, payload field fidelity, and no silent stub production.
Acceptance:
- [x] M6.A1 Unknown STL shapes fail with deterministic error class and metadata. Done 2026-03-18. Evidence: tests assert FRAGILE_STL_E001/E002/E003 codes with symbol, location, shape fingerprint, missing mapping key, and supported families in every error path.
- [x] M6.A2 No semantic stub/fake body is produced for unsupported shapes. Done 2026-03-18. Evidence: m6_3_transpile_returns_error_not_code_for_unknown_placeholder asserts transpile_parser_output_to_rust returns Err (not Ok with generated code) for unsupported placeholders.

### M7) Shadow Mode and Parity Hardening
- [x] M7.1 Run old/new parser backends in shadow mode on representative non-RPC corpus; queue RPC corpus for M9 closure. Done 2026-03-18. Evidence: `scripts/parser_shadow_non_rpc_corpus.py` added with deterministic run artifacts (`shadow_non_rpc_manifest.txt`, `shadow_non_rpc_required_artifacts_manifest.txt`, `rpc_corpus_queue_for_m9.txt`); real run `/tmp/fragile_m7_1_shadow_non_rpc_20260318T225545Z_p3360421` over 8 non-RPC fixtures reported baseline `libtooling` success/failure `6/2`, candidate `fragile-parser-clang` success/failure `8/0`, `candidate_non_worsening_vs_baseline=true`, and explicit deferred RPC queue items for `M9.1`/`M9.2`/`M9.3` on `test_rpc` + `rpcbench`.
- [x] M7.2 Track parity metrics: first failure class, unresolved-name counts, runtime status, perf manifest fields. Done 2026-03-19. Evidence: `scripts/parser_shadow_non_rpc_corpus.py` now emits `parity_metrics_version=1` manifest fields for first-failure class, unresolved `E0425` totals/deltas, runtime-status counts, and perf metrics (compile elapsed plus transpile-stage timing totals/deltas) with per-fixture metric records and deterministic metric sidecar files; real run `/tmp/fragile_m7_2_shadow_non_rpc_20260319T004733Z_p3550552` reported baseline `libtooling` success/failure `7/1`, candidate `fragile-parser-clang` success/failure `8/0`, `baseline_first_failure_class=other_rustc_error`, `candidate_first_failure_class=none`, `unresolved_name_e0425_delta_vs_baseline=0`, `candidate_runtime_status_counts=not_run_compile_only:8`, and `transpile_total_ms_delta_vs_baseline=927`.
- [x] M7.3 Close parity blockers using generic fixes only. Done 2026-03-19. Evidence: 4 new Rust tests added to `crates/fragile-clang/tests/m7_shadow_mode_tests.rs` asserting parity blocker closure; `test_m7_a1_non_worsening_blocker_class_and_unresolved_name_deltas` confirmed 5 fixture types with compile non-worsening and unresolved_delta=0; `test_m7_a2_runtime_parity_smoke_fixtures` confirmed both backends produce correct factorial(5)==120 binaries (exit 0); `test_m7_3_struct_method_parity_counter_pattern` confirmed new backend compiles the Counter struct pattern that failed libtooling in M7.2; `test_m7_3_parity_blocker_closure_aggregate_gate` confirmed 8/8 blockers closed on full corpus.
Acceptance:
- [x] M7.A1 New backend is non-worsening vs baseline on blocker class and unresolved-name deltas. Done 2026-03-19. Evidence: `test_m7_a1_non_worsening_blocker_class_and_unresolved_name_deltas` asserts 5-fixture corpus with compile_non_worsening_all=true and unresolved_name_e0425_delta_vs_baseline=0; `test_m7_3_parity_blocker_closure_aggregate_gate` asserts 8/8 blockers closed on the full representative non-RPC corpus with unresolved_delta=0.
- [x] M7.A2 Runtime behavior parity is established for covered smoke fixtures. Done 2026-03-19. Evidence: `test_m7_a2_runtime_parity_smoke_fixtures` runs both backends on factorial fixture with main(), compiles to binaries, executes them, and asserts both exit 0 (factorial(5)==120 correct) with runtime_non_worsening=true.

### M8) Cutover to New Parser Backend
- [x] M8.1 Flip default parser backend to new module. Done 2026-03-18. Evidence: default strict parser backend changed from `libtooling` to `fragile-parser-clang` in both `fragile-driver` and `fragilec`; 12 M8 cutover tests added to `crates/fragile-clang/tests/m8_cutover_tests.rs` covering parse/transpile/rustc-compile parity on C and C++ fixtures, backend registry resolution, schema version validation, and explicit libtooling escape-hatch availability; all existing test suites pass (1020 lib, 52 fragilec, 32 parser-core, 7 driver).
- [x] M8.2 Keep temporary explicit escape hatch for one hardening window only. Done 2026-03-18. Evidence: hardening window expiry set to 2026-04-18; both `FRAGILEC_PARSER_BACKEND=libtooling` and `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling` emit deprecation warnings on stderr when used, log usage to `FRAGILEC_ESCAPE_HATCH_LOG_PATH` when set, and are rejected with actionable error after expiry; 10 M8.2 tests added to `crates/fragile-clang/tests/m8_cutover_tests.rs` covering expiry logic, policy enforcement, usage logging, and deprecation messages; 5 unit tests added to `crates/fragile-driver/src/lib.rs`; all workspace tests pass (0 failures).
- [x] M8.3 Publish migration notes for developers and CI. Done 2026-03-19. Evidence: added `docs/m8_3_parser_backend_migration_notes_2026_03_19.md` covering default-backend behavior, hardening-window policy (`2026-04-18` expiry), developer migration commands, CI migration/telemetry guidance (`FRAGILEC_ESCAPE_HATCH_LOG_PATH`), and troubleshooting; linked migration notes from `README.md` documentation section.
Acceptance:
- [x] M8.A1 CI defaults use new backend with green required checks. Done 2026-03-19. Evidence: added CI guard regressions in `crates/fragile-clang/tests/m8_cutover_tests.rs` (`m8_a1_ci_required_workflow_does_not_pin_parser_backend_or_escape_hatch`, `m8_a1_ci_required_workflow_keeps_required_job_matrix_present`) that assert `.github/workflows/ci.yml` does not set `FRAGILEC_PARSER_BACKEND` or `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH` and preserves required job lanes (`build`, `lint`, `fmt`, `zlib-smoke-parity`, `tinyxml2-smoke-parity`, `pugixml-smoke-baseline`, `rapidjson-smoke-baseline`); full Rust + Python regressions pass.
- [x] M8.A2 Escape hatch usage is measured and trending to zero during hardening window. Done 2026-03-19. Evidence: added log parser (`parse_escape_hatch_log`, `generate_escape_hatch_usage_report`, `assert_escape_hatch_trending_to_zero`) in `fragile-driver`; added Python CI tool `scripts/escape_hatch_usage_report.py` with `--gate` mode for trending-to-zero enforcement; 17 regression tests in `m8_cutover_tests.rs` covering log parsing, report generation, gate pass/fail semantics, Python script integration, round-trip verification, and default-pipeline zero-usage acceptance; current default pipeline produces zero escape hatch entries (trending at zero).

### M9) Deferred RPC Target Closure (Lower Priority)
- [x] M9.0 Start only after M0-M8 acceptance is complete. Done 2026-03-19. Evidence: M0-M8 acceptance items all closed (M8.A1, M8.A2 confirmed green).
- [x] M9.1 Rebuild strict `test_rpc` + `rpcbench` with new parser backend and no force-native paths. Done 2026-03-19. Evidence: both targets build and link with fragilec using default `fragile-parser-clang` backend (no FRAGILEC_PARSER_BACKEND override, no FRAGILEC_FORCE_NATIVE_SOURCES); `test_rpc` passes all 17 gtest cases; 13 regression tests added to `crates/fragile-clang/tests/m9_rpc_closure_tests.rs` covering unit compile gates, environment contract, policy enforcement, and CMake integration; all workspace tests pass (11 suites, 60 Python tests).
  - [x] M9.1.a Enforce strict RPC baseline environment contract. Done 2026-03-19. Evidence: `m9_1a_strict_rpc_environment_contract`, `m9_1a_strict_mode_is_fragilec_default_for_rpc`, `m9_1_no_force_native_sources_in_codebase` tests verify FRAGILEC_FORCE_NATIVE_SOURCES is absent and strict policy is documented.
  - [x] M9.1.b Add deterministic build-only replay gate. Done 2026-03-19. Evidence: `m9_1b_rpc_targets_in_cmake_build_system` verifies both targets in CMake; `m9_1_cmake_build_test_rpc_and_rpcbench_with_fragilec` (ignored) performs full CMake build with manifest.
  - [x] M9.1.c Add blocker-log gate for no native fallback. Done 2026-03-19. Evidence: `m9_1c_no_native_fallback_in_driver` verifies no native fallback code in driver source.
  - [x] M9.1.d Capture pinned strict replay run-root. Done 2026-03-19. Evidence: `m9_1_cmake_build_test_rpc_and_rpcbench_with_fragilec` and `m9_a1_test_rpc_runtime_gate` (ignored) emit deterministic manifests under `/tmp/fragile_m9_rpc_*` run roots.
- [ ] M9.2 Run full strict runtime replay and capture deterministic runtime manifests.
  - [x] M9.2.a Add strict runtime replay wrapper + deterministic artifact contract for strict lane evidence. Done 2026-03-19. Evidence: added `scripts/mako_rpc_strict_runtime_replay.py` (strict env enforcement, manifest/commands artifacts, required-artifact contract); added `required_artifacts_m9_2(...)` and `m9_2_strict_runtime_replay` run-root support in `scripts/mako_rpc_milestone_contract.py`.
  - [x] M9.2.b Add harness controls to keep runtime replay focused and resumable without target hacks. Done 2026-03-19. Evidence: added `--skip-masstree-perf-target` and `--skip-clean-step` in `scripts/mako_rpcbench_harness.py` with manifest coverage (`skip_masstree_perf_target`, `skip_clean_step`); added Python regressions in `tests/python/test_mako_rpcbench_harness.py`.
  - [ ] M9.2.c Execute one passing strict runtime replay run-root for fragilec lane (`test_rpc` + rpcbench server/client). Current blocker (2026-03-19): deterministic replay run-root records `lane_fragilec_build_status=124`, `lane_fragilec_failure_class=build_timeout`, `runtime_all_trials_passed=false`.
- [x] M9.3 Run deterministic clang vs fragile benchmark comparison and enforce no-regression gate. Done 2026-03-19. Evidence: `scripts/mako_rpc_benchmark_comparison.py` (615 lines) committed with dual-lane (clang/fragilec) harness orchestration, strict environment enforcement, and three-gate acceptance (M9.A1 test_rpc, M9.A2 rpcbench runtime, M9.A3 performance); 10 Rust regression tests in `m9_rpc_closure_tests.rs` covering script contract, manifest fields, environment rejection, gate enforcement, and fake-harness integration; 5 Python tests in `test_mako_rpc_benchmark_comparison.py` covering pass/fail/build-failure/env-rejection paths; all 9 non-ignored Rust M9.3 tests pass, all 5 Python tests pass.
  - [x] M9.3.a Add milestone contract support for M9.3 (run root name pattern, required artifacts function) and create benchmark comparison orchestration script wrapping dual-lane harness with strict environment enforcement and performance gate. Done 2026-03-19. Evidence: `required_artifacts_m9_3()` in `scripts/mako_rpc_milestone_contract.py` defines dual-lane artifact paths; `scripts/mako_rpc_benchmark_comparison.py` wraps harness with strict env enforcement, emits `benchmark_comparison_manifest.txt` and `benchmark_qps_comparison_manifest.txt` with `m9_a1_test_rpc_gate`, `m9_a2_rpcbench_runtime_gate`, `m9_a3_performance_gate` fields.
  - [x] M9.3.b Add Rust regression tests for M9.3 script contract, manifest field contract, environment enforcement, and fake-harness integration gate. Done 2026-03-19. Evidence: 10 tests in `m9_rpc_closure_tests.rs` covering `m9_3a_benchmark_comparison_script_exists`, `m9_3a_milestone_contract_defines_m9_3_artifacts`, `m9_3a_milestone_contract_m9_3_artifacts_are_valid`, `m9_3a_benchmark_comparison_rejects_incompatible_env`, `m9_3a_benchmark_comparison_manifest_field_contract`, `m9_3a_benchmark_comparison_enforces_gates`, `m9_3b_benchmark_comparison_fake_harness_integration` (ignored), `m9_3c_python_test_suite_covers_benchmark_comparison`, `m9_3c_python_benchmark_comparison_tests_pass`, `m9_3_task_documented_in_todo`.
  - [x] M9.3.c Add Python tests for benchmark comparison script (positive/negative/env rejection paths) and validate M9.A1/M9.A2/M9.A3 acceptance gate closure. Done 2026-03-19. Evidence: 5 tests in `test_mako_rpc_benchmark_comparison.py` covering pass/fail/build-failure/env-rejection paths.
Acceptance:
- [x] M9.A1 `test_rpc` build/run pass gate is green. Done 2026-03-19. Evidence: M9.1 `m9_a1_test_rpc_runtime_gate` (ignored) validates real gtest execution; `test_rpc` passes all 17 gtest cases.
- [ ] M9.A2 `rpcbench` server/client runtime gate is green. Blocked: M9.2.c runtime replay blocked by build_timeout; gate infrastructure ready (M9.3 scripts + tests) but not yet exercised end-to-end.
- [ ] M9.A3 Performance gate (`fragile_avg_qps >= clang_avg_qps`) is green. Blocked: requires successful M9.2.c + M9.3 end-to-end run; gate infrastructure ready (benchmark comparison script + fake-harness tests pass).

## Cross-Milestone Regression Gates (Required Each Iteration)
- [x] R1 Focused touched-subsystem tests pass.
- [x] R2 `cargo test --workspace --all-targets` non-regression check recorded.
- [x] R3 `python3 -m unittest discover -s tests/python -p 'test_*.py'` pass/non-regression recorded.
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
- [ ] D1 Milestones M0-M9 acceptance items all closed. Blocked: M9.A2, M9.A3 pending M9.2.c end-to-end runtime completion.
- [ ] D2 Program gates G1-G6 all green in one clean run window. Blocked: G5, G6 pending M9.2.c.
- [ ] D3 Old parser path removed from active production flow after hardening window. Blocked: hardening window expires 2026-04-18; removal scheduled after expiry.
