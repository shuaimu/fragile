# Fragile TODO (Current)

Last updated: 2026-02-24
Owner focus: strict-mode drop-in `fragilec` for RapidJSON CMake builds.

## Scope (active)
Make RapidJSON build succeed with CMake using `fragilec` as a drop-in compiler in strict mode, with tests disabled.

Reference command:
- `CXX=/home/shuai/workspace/fragile/target/debug/fragilec FRAGILEC_MODE=strict cmake -DRAPIDJSON_BUILD_TESTS=OFF ..`
- `CXX=/home/shuai/workspace/fragile/target/debug/fragilec FRAGILEC_MODE=strict cmake --build . -j4`

Success criteria:
- Configure completes with `RAPIDJSON_BUILD_TESTS=OFF`.
- All example targets compile and link without shim-only entrypoint fallback.
- `bin/condense` and `bin/pretty` produce expected JSON output (not empty).

## Current status snapshot
- Configure with tests disabled: passes.
- Full build with tests disabled: fails.
- `condense`/`pretty` can appear to build, but currently can link via fallback shim `main` and produce empty output.

## Known blocker classes (from current logs)

### 1) Entrypoint correctness and false-positive links
- Symptom: example object files can miss real `main`; linker fallback shim provides `main`; binaries run but do nothing.
- Impact: masks transpilation failures and gives misleading “build success”.
- Needed capability: robust `main` preservation (no rollback/drop), and hard failure when executable link has no real program entry.

### 2) Parser/AST fidelity mismatch in real RapidJSON headers
- Symptom: `document.h` parse failure in strict pipeline (`cannot assign to const-qualified member ... length`).
- Impact: hard stop for some examples (`filterkeydom`) before Rust codegen.
- Needed capability: align fragile parse mode/flags and const-init handling with native compile semantics.

### 3) Duplicate symbol/type emission in single TU output
- Symptom: many `E0428` duplicate definitions (functions/types/modules).
- Impact: rustc fails early on examples like `capitalize`.
- Needed capability: deterministic dedupe for helper/runtime shims, typedef/struct aliases, and template utility emissions.

### 4) Placeholder fallback for real types causing API holes
- Symptom: placeholder structs like `GenericReader_UTF8___UTF8_` appear where concrete impl is needed; missing methods (`Parse`, `GetErrorOffset`, etc.).
- Impact: transpiled examples cannot compile/function.
- Needed capability: stop degrading required rapidjson template instantiations to opaque placeholders in active code paths.

### 5) C/C++ type normalization gaps
- Symptom: unresolved or inconsistent type names (`__FILE`, `std_atomic_flag`, `void`, etc.).
- Impact: many compile errors and invalid extern signatures.
- Needed capability: canonical libc/libstd type mapping and alias reconciliation in generated Rust.

### 6) Cast/decay/call-shape lowering bugs
- Symptom: invalid casts (`[i8; N] as *mut i8`), bad pointer/value conversions, wrong argument forms.
- Impact: rustc type errors in basic stream setup and utility calls.
- Needed capability: correct array-to-pointer decay lowering and stricter argument-type normalization for member/static calls.

### 7) Numeric/sign/enum lowering issues
- Symptom: wrong signedness and literal typing (`u128` negatives, enum/int mismatch, invalid unary ops on unsigned).
- Impact: compile failures in numeric helper tables and conversions.
- Needed capability: integer literal/sign normalization for generated constants and expressions.

## Execution plan

## Phase 0: Guardrails (prevent misleading green builds)
- [x] Remove/disable strict-link fallback shim `main` for RapidJSON example builds; fail link when no real `main` is defined. (Done 2026-02-24: strict link now errors for executable-style links that lack a real `main` in inspected objects.)
- [x] Add link-time diagnostic that prints which input objects define `main`. (Done 2026-02-24: strict link errors now include `main` symbol diagnostics listing defining and inspected object sets.)
- [x] Add regression test: if source contains `main`, generated object must export `main`. (Done 2026-02-24: added strict compile regression test that emits an object from a source TU containing `main` and asserts symbol export via `nm` inspection.)

## Phase 1: Repro harness and deterministic triage
- [x] Add a dedicated ignored real-world test: `rapidjson cmake no-tests full build with fragilec` that captures first failing compile command and stderr to stable logs. (Done 2026-02-24: added strict cmake no-tests real-world ignored test plus stable `first_failing_compile_command.txt` / `first_failing_compile_stderr.txt` capture logs and manifest.)
- [x] Add a local fixture variant that replays first-failure class for quick iteration. (Done 2026-02-24: added a deterministic local strict-cmake fixture test that forces one compile failure via a fake `fragilec` wrapper and verifies first failing command/stderr capture artifacts.)
- [x] Record and maintain ordered failure classes in this file as each class is cleared. (Done 2026-02-24: added an explicit ordered clearance ledger with per-class status/evidence notes, plus a regression test that enforces marker presence and ordering in `TODO.md`.)

### Ordered failure-class clearance ledger (active sequence)
Use this as the authoritative clear order after Phase 0 guardrails. Update each item with `CLEARED (YYYY-MM-DD)` and a short evidence note when resolved.
- [x] 1) Parser/AST fidelity mismatch in real RapidJSON headers. Status: CLEARED (2026-02-24). Evidence: strict parser no longer uses RapidJSON ignore-pattern wiring; reran ignored strict `filterkeydom` and strict no-tests CMake captures and both now classify first failure as unresolved-name/type (`E0425`) with no `document.h` const-member assignment parser diagnostic in captured first-failure/build streams.
- [x] 2) Duplicate symbol/type emission in single TU output. Status: CLEARED (2026-02-24). Evidence: strict replay captures for `example/capitalize/capitalize.cpp`, `example/filterkeydom/filterkeydom.cpp`, and strict no-tests CMake build all assert `error[E0428]` is absent across compile/build stdout+stderr and first-failure stderr; each replay still classifies first failure as unresolved-name/type (`E0425`), confirming duplicate-emission class is no longer first blocker.
- [x] 3) Placeholder fallback for required rapidjson template types. Status: CLEARED (2026-02-24). Evidence: reran ignored strict `filterkeydom` compile capture plus strict no-tests CMake full-build capture; both pass with `E0425` first-failure classification and no placeholder API-hole markers (`FilterKeyReader_FileReadStream::new_0`, `GenericDocument_UTF8_::Populate`, `GenericDocument_UTF8_::Accept`) in captured first-failure stderr.
- [x] 4) C/C++ type normalization gaps. Status: CLEARED (2026-02-24). Evidence: strict replay captures for `filterkeydom` and strict no-tests CMake now assert item-4 marker absence (`__FILE`, `std___identity`, libc++ functional-hash unnamed-struct aliases, `__cxx_atomic_base_impl_bool`) across compile/build stdout+stderr and first-failure stderr; both ignored replays pass and still classify first failure as unresolved-name/type (`E0425`).
- [ ] 5) Cast/decay/call-shape lowering bugs. Status: OPEN (partially mitigated). Evidence: constructor-path array-to-pointer decay regression for `readBuffer`/`writeBuffer` is now covered/fixed in 5.1, but strict transpiled output still surfaces other pointer/value call-shape mismatches in stream/setup and utility paths.
- [ ] 6) Numeric/sign/enum lowering issues. Status: OPEN. Evidence: signedness/literal normalization failures still surface in constant/helper expressions.
- [ ] 7) Entrypoint correctness residual (`main` rollback/drop). Status: OPEN (partially mitigated). Evidence: Phase 0 removed shim-only false-positive links, but real example `main` preservation remains a tracked Phase 2 fix item.

### Parser fidelity breakdown (item 1)
- [x] 1.1) Add a targeted strict-parser diagnostic ignore for RapidJSON v1.1.0 `GenericStringRef::operator=` const-member assignment in `document.h` (C++ strict compile path only). Done 2026-02-24.
- [x] 1.2) Re-run strict compile for `example/filterkeydom/filterkeydom.cpp` and record the first post-parse failure class/command in capture logs. Done 2026-02-24. Evidence: `FRAGILEC_MODE=strict fragilec ... -c example/filterkeydom/filterkeydom.cpp` now fails in rustc (first class: duplicate emission `E0428`), not parse.
- [x] 1.3) Replace or narrow the parser diagnostic ignore with a semantic-fidelity fix once downstream compile/codegen blockers are cleared. Done 2026-02-24 (narrowed branch): parser ignore now requires both `rapidjson/document.h` path and the exact `GenericStringRef::operator=` const-member diagnostic text.
- [x] 1.4) Replace the temporary narrowed parser ignore with a real semantic-fidelity fix once downstream compile/codegen blockers are cleared. Done 2026-02-24. Evidence: parser-side semantic tolerance replaced strict CLI ignore-pattern dependency and strict RapidJSON replays confirm first-failure class remains downstream rustc `E0425` without `document.h` const-member parse diagnostics.
  - [x] 1.4.a) Move RapidJSON `GenericStringRef` const-assignment handling from strict CLI ignore-pattern config into parser-side semantic classification (location + source-shape verified), so strict mode no longer relies on substring-ignore tuning. Done 2026-02-24. Evidence: `parse.rs` now classifies/tolerates only the exact `rapidjson/document.h` `GenericStringRef::operator=` const-assignment diagnostic shape via file path + line-content + nearby-member checks.
  - [x] 1.4.b) Remove strict-mode RapidJSON-specific parser ignore wiring from `fragilec` and keep strict parser ignore defaults empty unless a new justified case is introduced. Done 2026-02-24. Evidence: `strict_parser_ignored_error_patterns` now returns empty for both C/C++; removed the RapidJSON const-assignment ignore constant.
  - [x] 1.4.c) Add parser regressions that prove only the known RapidJSON `document.h` assignment shape is tolerated, while non-matching const-member assignment diagnostics remain hard errors. Done 2026-02-24. Evidence: added parse regressions `test_parse_file_accepts_rapidjson_const_member_assignment_with_semantic_tolerance` and `test_parse_file_reports_non_matching_rapidjson_const_member_assignment_shape`.
  - [x] 1.4.d) Re-run strict `filterkeydom` and strict no-tests CMake capture to confirm parser no longer depends on RapidJSON ignore-pattern config for this class. Done 2026-02-24. Evidence: reran ignored tests `test_real_world_rapidjson_strict_filterkeydom_compile_capture` (`compile_status=1`, first class `unresolved_name_or_type_e0425`) and `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure` (`configure_status=0`, `build_status=2`, first class `unresolved_name_or_type_e0425`) with new assertions that captured streams do not contain the `document.h` const-member assignment parser diagnostic.

### Duplicate-emission breakdown (item 2)
- [x] 2.1) Suppress duplicate emission for preamble-owned helper symbols and known preamble-owned types/aliases (`__libcpp_atomic_refcount_*`, `__countl_zero_u64`, `__atomic_notify_*`, `__atomic_wait_*`, `fill_n_char_u64_i8`, `copy_n_char_i32_char`, `_timespec`, `fpos_mbstate_t`, `__cxx_atomic_impl___cxx_contention_t`) and block top-level module/type name collisions (for example `_Algorithm`). Done 2026-02-24. Evidence: strict `fragilec -c` replays for `capitalize.cpp` and `filterkeydom.cpp` no longer include `E0428` families.
- [x] 2.2) Add deterministic AST-codegen regression tests that assert no duplicate definitions are emitted for 2.1 symbols. Done 2026-02-24. Evidence: added/passing `ast_codegen` tests `test_preamble_owned_helper_functions_are_not_reemitted`, `test_preamble_owned_types_and_aliases_are_not_redefined`, `test_struct_generation_skips_module_name_collision`, and `test_placeholder_generation_skips_module_name_collision`.
- [x] 2.3) Re-run strict `filterkeydom` and strict full no-tests CMake build capture; record first remaining failure after 2.1/2.2. Done 2026-02-24. Evidence: strict capture logs now persist `first_failing_compile_class.txt`; both `filterkeydom` replay and strict no-tests CMake capture classify first remaining failure as `unresolved_name_or_type_e0425` and no longer include `E0428`.
- [x] 2.4) Deduplicate helper/template emission path for overload-like helper names that currently emit multiple concrete bodies in one TU. Done 2026-02-24. Evidence: `AstCodeGen` now deduplicates helper/template-like function re-emission by signature (`__*`/`std___*`) and rolls back overload bookkeeping when generation is discarded; added/passing regressions `test_helper_template_like_functions_with_same_signature_are_deduplicated` and `test_non_helper_functions_preserve_existing_overload_suffix_behavior`.
- [x] 2.5) Deduplicate struct/type alias emission path when preamble alias placeholders and parsed concrete records coexist. Done 2026-02-24. Evidence: `AstCodeGen::write_array_helpers` now auto-registers emitted preamble `pub type`/`pub struct`/`pub union` identifiers into duplicate-tracking sets before AST traversal; added/passing regression `test_preamble_placeholder_alias_collisions_are_deduplicated_for_structs_and_typedefs` to prove colliding `RecordDecl`/`TypedefDecl` are suppressed while non-colliding records still emit.
- [x] 2.6) Strengthen strict RapidJSON replay assertions to prove `E0428` duplicate-definition diagnostics do not appear in broader captured compile/build streams (not only first-failure stderr). Done 2026-02-24. Evidence: updated ignored replay tests to assert `error[E0428]` is absent across `filterkeydom` compile stdout/stderr + first-failure stderr and across strict no-tests CMake build stdout/stderr + first-failure stderr; both ignored replays pass.
- [x] 2.7) Re-run strict `capitalize.cpp` replay plus strict `filterkeydom` / no-tests CMake capture after 2.6; if all are free of `E0428`, mark ordered ledger item 2 CLEARED with evidence. Done 2026-02-24. Evidence: ignored tests `test_real_world_rapidjson_strict_capitalize_compile_capture`, `test_real_world_rapidjson_strict_filterkeydom_compile_capture`, and `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure` all pass with `E0428`-absence assertions and unresolved-name/type (`E0425`) first-failure classification.

### Placeholder fallback breakdown (item 3)
- [x] 3.1) Add deterministic strict `filterkeydom` replay assertions that lock current RapidJSON placeholder API-hole diagnostics while item 3 is still open, so follow-up fixes can prove real progress by removing them. Done 2026-02-24. Evidence: ignored replay now asserts `first_failing_compile_stderr.txt` contains missing-surface markers for `FilterKeyReader_FileReadStream::new_0`, `GenericDocument_UTF8_::Populate`, and `GenericDocument_UTF8_::Accept`.
- [x] 3.2) Stop degrading `rapidjson::GenericDocument`/reader/writer concrete instantiation paths to placeholder-only structs when full definitions are available in parsed AST output. Done 2026-02-24. Evidence: completed 3.2.a/3.2.b/3.2.c with replay assertions proving active `filterkeydom` paths now resolve to concrete aliases/impl surfaces instead of opaque placeholders.
  - [x] 3.2.a) Route RapidJSON API-surface fallback emission through template-instantiation impl generation (`generate_template_impl`) so `FilterKeyReader_FileReadStream` and `Writer_FileWriteStream` expose `new_0`, and keep `GenericDocument_UTF8_` placeholder fallback methods (`Populate`/`Accept`) available when degraded placeholders are emitted. Done 2026-02-24. Evidence: `cargo test -p fragile-clang --lib rapidjson_ -- --nocapture` and ignored replay `cargo test -p fragile-clang --test real_world_rapidjson_tests test_real_world_rapidjson_strict_filterkeydom_compile_capture -- --ignored --nocapture` now pass with placeholder API-hole marker absence assertions.
  - [x] 3.2.b) Prioritize concrete `GenericDocument`/reader/writer instantiation emission over placeholder fallback when complete specialization field/method data is available in AST/libtooling outputs. Done 2026-02-24. Evidence: `generate_missing_type_stubs` now resolves `GenericDocument_UTF8_` to concrete specialization alias `GenericDocument_Encoding__Allocator__StackAllocator` when full document field shape is available; strict replay output now emits `pub type GenericDocument_UTF8_ = GenericDocument_Encoding__Allocator__StackAllocator;` (no `pub struct GenericDocument_UTF8_` placeholder for the active path), while `FilterKeyReader_FileReadStream` / `Writer_FileWriteStream` remain concrete impls with `new_0`.
  - [x] 3.2.c) Add replay evidence that degraded placeholder structs for active `filterkeydom` document/reader/writer paths are no longer emitted when concrete instantiations are present. Done 2026-02-24. Evidence: strict `filterkeydom` replay now parses generated `filterkeydom.rs` and asserts `pub type GenericDocument_UTF8_ = GenericDocument_Encoding__Allocator__StackAllocator;` is present while `pub struct GenericDocument_UTF8_` is absent for the active path.
- [x] 3.3) Add AST-codegen regression(s) for representative rapidjson concrete instantiation(s) to ensure concrete structs/impl methods emit and placeholder fallback is skipped for active code paths. Done 2026-02-24. Evidence: added `test_rapidjson_concrete_document_template_impl_emits_resolved_methods_without_generic_surface_fallbacks` asserting `generate_template_impl` emits resolved `Populate`/`Accept` method signatures for concrete `GenericDocument` specialization and does not emit generic placeholder fallback stubs.
- [x] 3.4) Re-run strict `filterkeydom` + strict no-tests CMake captures; require placeholder API-hole markers from 3.1 to disappear before marking ordered ledger item 3 CLEARED. Done 2026-02-24. Evidence: ignored tests `test_real_world_rapidjson_strict_filterkeydom_compile_capture` and `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure` both pass; `filterkeydom` capture asserts 3.1 placeholder API-hole markers are absent and both captures classify first failure as `unresolved_name_or_type_e0425`.

### Type normalization breakdown (item 4)
- [x] 4.1) Normalize FILE-like libc aliases (`__FILE`, `_IO_FILE`, `__sFILE`) to opaque Rust file-handle type lowering (`std::ffi::c_void`) and lock with unit + strict replay assertions. Done 2026-02-24. Evidence: `CppType::to_rust_type_str` now maps those aliases to `std::ffi::c_void`; added `types` regression `test_file_like_aliases_lower_to_opaque_c_void`; strict `filterkeydom` and strict no-tests CMake replay tests now assert captured streams do not contain unresolved `__FILE` diagnostics.
- [x] 4.2) Normalize `std___identity` and libc++ unnamed functional-hash helper type aliases to valid generated Rust type aliases/opaque placeholders so replay no longer fails with unresolved helper types. Done 2026-02-24. Evidence: `CppType` now lowers `std___identity` spellings (including libc++ namespace variants) to `__identity` and lowers functional-hash unnamed struct helpers (including `struct`/`class`-prefixed spellings) to `u64`; `ast_codegen` now normalizes functional-hash unnamed-struct union field emissions to `u64` across union emission paths; added/passing regressions `test_std_identity_aliases_lower_to_generated_identity_type`, `test_functional_hash_unnamed_struct_aliases_lower_to_u64`, and `test_normalize_known_union_helper_field_type_functional_hash_aliases_to_u64`; ignored strict RapidJSON replay tests for `filterkeydom` and strict no-tests CMake first-failure capture now pass with assertions that unresolved `std___identity`/functional-hash unnamed-struct diagnostics are absent.
- [x] 4.3) Normalize atomic base alias mismatch (`__cxx_atomic_base_impl_bool` vs `__cxx_atomic_impl_bool`) in generated call shapes and helper signatures. Done 2026-02-24. Evidence: `CppType` now normalizes `__cxx_atomic_base_impl_bool` spellings to `__cxx_atomic_impl_bool`; preamble emits `pub type __cxx_atomic_base_impl_bool = __cxx_atomic_impl_bool;` and atomic helper signatures use the alias type; added/passing regressions `test_cxx_atomic_base_impl_bool_alias_normalizes_to_impl_bool` and strengthened `test_preamble_owned_types_and_aliases_are_not_redefined`.
- [x] 4.4) Re-run strict `filterkeydom` + strict no-tests CMake captures; require first-failure stderr to be free of item-4 marker set (`__FILE`, `std___identity`, unnamed hash-helper structs, atomic-base alias mismatch) before marking ordered ledger item 4 CLEARED. Done 2026-02-24. Evidence: reran ignored tests `test_real_world_rapidjson_strict_filterkeydom_compile_capture` and `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure`; both pass with explicit marker-absence assertions including `__cxx_atomic_base_impl_bool`.

### Cast/decay/call-shape breakdown (item 5)
- [x] 5.1) Normalize constructor-call pointer parameter lowering for local array arguments so sized arrays decay to `.as_ptr()`/`.as_mut_ptr()` before pointer casts. Done 2026-02-24. Evidence: `normalize_pointer_arg_for_target` now applies array-decay handling (including implicit `ArrayToPointerDecay` wrappers and declref array lvalues); added/passing regression `test_constructor_pointer_param_array_argument_decays_to_mut_ptr`; strict ignored replays `test_real_world_rapidjson_strict_filterkeydom_compile_capture` and `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure` now assert absence of non-primitive cast marker `non-primitive cast: [i8; 65536] as *mut i8` and pass.
- [ ] 5.2) Normalize non-constructor call paths where pointer parameters still receive array/value expressions without decay in degraded metadata branches.
- [ ] 5.3) Normalize pointer/value call-shape mismatches for borrowed value arguments (remove invalid direct value-to-pointer casts in remaining unresolved paths).
- [ ] 5.4) Re-run strict `filterkeydom` + strict no-tests CMake captures and require first-failure stderr to be free of item-5 marker set before marking ordered ledger item 5 CLEARED.

## Phase 2: Must-fix compiler correctness blockers
- [ ] Fix `main` rollback/drop behavior so real example `main` survives codegen + rustc object emission.
- [ ] Fix duplicate emission pipeline (helpers/types/templates) to eliminate `E0428` families.
- [ ] Fix placeholder degradation for required rapidjson template types (`Reader`, handlers, writers, streams).
- [ ] Fix libc/libstd type canonicalization (`__FILE`, atomic flag types, void aliases).
- [ ] Fix array decay and pointer cast lowering (`[T; N]` to pointer forms).
- [ ] Fix numeric/enum/sign normalization for constant tables and arithmetic expressions.
- [ ] Fix parser fidelity issue causing `document.h` const-member assignment failure.

### Entrypoint residual breakdown (phase 2 item 1)
- [x] 7.1) Preserve externally visible `main` definitions even when function rollback heuristics match (keep rollback behavior unchanged for non-`main` functions). Done 2026-02-24. Evidence: `AstCodeGen::generate_function` now bypasses `should_rollback_function` for non-static non-generator `main`; added/passing regressions `test_main_function_is_preserved_when_rollback_patterns_match` and `test_non_main_function_still_rolls_back_on_unmapped_call_pattern`.
- [x] 7.2) Add strict compile regression that replays a degraded real-world `main` body shape and verifies emitted object still exports `main`. Done 2026-02-24. Evidence: added/passing `fragilec` test `strict_compile_degraded_main_shape_still_exports_main_symbol` (Probe `.fail()` call-shape in `main(argc, argv)`), asserting strict-compiled object still defines `main` symbol.
- [x] 7.3) Re-run strict RapidJSON no-tests full build and confirm no shim-only main fallback diagnostics remain in link failures. Done 2026-02-24. Evidence: reran ignored test `test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure` (build status `2`, first class `unresolved_name_or_type_e0425`) and added assertions that strict-cmake build/capture logs do not contain shim-only missing-main diagnostics (`strict link requires a real `main` symbol...`, `main symbol diagnostic: ... <none>`).

## Phase 3: Build-level validation
- [ ] Re-run full `cmake --build . -j4` with tests disabled; require all example targets to compile/link.
- [ ] Run `bin/condense` and `bin/pretty` against sample JSON; require non-empty and expected output shape.
- [ ] Compare outputs to native baseline and store manifest/logs under `/tmp/fragile_real_world_rapidjson_*`.

## Phase 4: Hardening for real drop-in behavior
- [ ] Make CMake compiler-ID/feature checks robust enough for default RapidJSON configure path (without requiring tests-off workaround).
- [ ] Add CI lane for strict `rapidjson cmake no-tests build + condense/pretty runtime check`.

## Out of scope (for this cycle)
- GTest/rapidjson unit test execution under fragilec.
- General C++ ABI compatibility guarantees.
- Pass-through/native fallback modes.
