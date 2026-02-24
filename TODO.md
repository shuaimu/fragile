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
- [ ] Add a local fixture variant that replays first-failure class for quick iteration.
- [ ] Record and maintain ordered failure classes in this file as each class is cleared.

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
