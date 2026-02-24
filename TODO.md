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
- [ ] 1) Parser/AST fidelity mismatch in real RapidJSON headers. Status: IN PROGRESS. Evidence: strict parser ignore is now narrowed to conjunctive match (`rapidjson/document.h` path + exact const-member diagnostic) and strict `filterkeydom` compile advances to downstream rustc unresolved-type/name blockers (`E0425`), not parse failures.
- [ ] 2) Duplicate symbol/type emission in single TU output. Status: IN PROGRESS. Evidence: strict replays for `example/capitalize/capitalize.cpp` and `example/filterkeydom/filterkeydom.cpp`, plus strict no-tests CMake first-failure capture, no longer report `E0428` duplicate-definition failures; first failures are now unresolved-type/name families (`E0425`).
- [ ] 3) Placeholder fallback for required rapidjson template types. Status: OPEN. Evidence: placeholder types (for example reader/handler forms) still surface on active compile paths and miss required methods.
- [ ] 4) C/C++ type normalization gaps. Status: OPEN. Evidence: unresolved/inconsistent aliases still appear for libc/libstd symbols (for example `__FILE`, atomic aliases, `void`-shape externs).
- [ ] 5) Cast/decay/call-shape lowering bugs. Status: OPEN. Evidence: strict transpiled output still produces array/pointer decay and call-shape mismatches in stream/setup paths.
- [ ] 6) Numeric/sign/enum lowering issues. Status: OPEN. Evidence: signedness/literal normalization failures still surface in constant/helper expressions.
- [ ] 7) Entrypoint correctness residual (`main` rollback/drop). Status: OPEN (partially mitigated). Evidence: Phase 0 removed shim-only false-positive links, but real example `main` preservation remains a tracked Phase 2 fix item.

### Parser fidelity breakdown (item 1)
- [x] 1.1) Add a targeted strict-parser diagnostic ignore for RapidJSON v1.1.0 `GenericStringRef::operator=` const-member assignment in `document.h` (C++ strict compile path only). Done 2026-02-24.
- [x] 1.2) Re-run strict compile for `example/filterkeydom/filterkeydom.cpp` and record the first post-parse failure class/command in capture logs. Done 2026-02-24. Evidence: `FRAGILEC_MODE=strict fragilec ... -c example/filterkeydom/filterkeydom.cpp` now fails in rustc (first class: duplicate emission `E0428`), not parse.
- [x] 1.3) Replace or narrow the parser diagnostic ignore with a semantic-fidelity fix once downstream compile/codegen blockers are cleared. Done 2026-02-24 (narrowed branch): parser ignore now requires both `rapidjson/document.h` path and the exact `GenericStringRef::operator=` const-member diagnostic text.
- [ ] 1.4) Replace the temporary narrowed parser ignore with a real semantic-fidelity fix once downstream compile/codegen blockers are cleared.

### Duplicate-emission breakdown (item 2)
- [x] 2.1) Suppress duplicate emission for preamble-owned helper symbols and known preamble-owned types/aliases (`__libcpp_atomic_refcount_*`, `__countl_zero_u64`, `__atomic_notify_*`, `__atomic_wait_*`, `fill_n_char_u64_i8`, `copy_n_char_i32_char`, `_timespec`, `fpos_mbstate_t`, `__cxx_atomic_impl___cxx_contention_t`) and block top-level module/type name collisions (for example `_Algorithm`). Done 2026-02-24. Evidence: strict `fragilec -c` replays for `capitalize.cpp` and `filterkeydom.cpp` no longer include `E0428` families.
- [x] 2.2) Add deterministic AST-codegen regression tests that assert no duplicate definitions are emitted for 2.1 symbols. Done 2026-02-24. Evidence: added/passing `ast_codegen` tests `test_preamble_owned_helper_functions_are_not_reemitted`, `test_preamble_owned_types_and_aliases_are_not_redefined`, `test_struct_generation_skips_module_name_collision`, and `test_placeholder_generation_skips_module_name_collision`.
- [x] 2.3) Re-run strict `filterkeydom` and strict full no-tests CMake build capture; record first remaining failure after 2.1/2.2. Done 2026-02-24. Evidence: strict capture logs now persist `first_failing_compile_class.txt`; both `filterkeydom` replay and strict no-tests CMake capture classify first remaining failure as `unresolved_name_or_type_e0425` and no longer include `E0428`.
- [ ] 2.4) Deduplicate helper/template emission path for overload-like helper names that currently emit multiple concrete bodies in one TU.
- [ ] 2.5) Deduplicate struct/type alias emission path when preamble alias placeholders and parsed concrete records coexist.

## Phase 2: Must-fix compiler correctness blockers
- [ ] Fix `main` rollback/drop behavior so real example `main` survives codegen + rustc object emission.
- [ ] Fix duplicate emission pipeline (helpers/types/templates) to eliminate `E0428` families.
- [ ] Fix placeholder degradation for required rapidjson template types (`Reader`, handlers, writers, streams).
- [ ] Fix libc/libstd type canonicalization (`__FILE`, atomic flag types, void aliases).
- [ ] Fix array decay and pointer cast lowering (`[T; N]` to pointer forms).
- [ ] Fix numeric/enum/sign normalization for constant tables and arithmetic expressions.
- [ ] Fix parser fidelity issue causing `document.h` const-member assignment failure.

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
