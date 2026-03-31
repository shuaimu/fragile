# P0.b.3 Parser Invocation Fallback Removal Inventory

Date: 2026-03-23
Owner task: `P0.b.3`
Scope: remove legacy libtooling/libclang/hybrid parser-invocation APIs from `crates/fragile-clang/src/lib.rs` and keep parser-output handoff path only.

This document is the deliverable for `P0.b.3.a`.

## 1. Why This Was Decomposed

`P0.b.3` is not a safe single-shot edit under the `<1000 LOC` bounded-leaf rule.

Current impact surface:

- `crates/fragile-clang/src/lib.rs`
  - deprecated backend enum and backend-label dispatch
  - libtooling parse/export/enrichment invocation chain
  - legacy public transpile APIs (`transpile_cpp_to_rust*`, `generate_stubs` flow)
- dependent tests/examples
  - `crates/fragile-clang/tests/m7_shadow_mode_tests.rs`
  - `crates/fragile-clang/tests/m8_cutover_tests.rs`
  - `crates/fragile-clang/tests/parser_backend_parity_tests.rs`
  - `crates/fragile-clang/tests/p0_libtooling_removal_audit_tests.rs`
  - `crates/fragile-clang/examples/{transpile,list_test_runner,map_test_runner,unordered_map_test_runner}.rs`

Combined change set (API deletion + dependent updates + regression rewrites) is expected to exceed 1000 LOC.

## 2. Symbol Inventory (Line-Anchored)

Primary symbols in `crates/fragile-clang/src/lib.rs`:

- `ParserBackend::{Libclang, Libtooling, Hybrid}`
- `parser_backend_label(...)`
- `parse_libtooling_context(...)`
- `translation_unit_from_libtooling_context(...)`
- `apply_libtooling_enrichment(...)`
- `transpile_cpp_to_rust(...)`
- `transpile_cpp_to_rust_with_backend(...)`
- `transpile_cpp_to_rust_with_options(...)`
- `generate_stubs(...)` (libtooling parse path)
- `transpile_cpp_to_rust_with_libtooling(...)`

## 3. Bounded Execution Slices

### `P0.b.3.b` (target: <350 LOC)

- Remove deprecated backend variants and backend-dispatch labels from `lib.rs`.
- Keep parser-output handoff backend label contract unchanged (`parser-output-handoff`).

Files:

- `crates/fragile-clang/src/lib.rs`

### `P0.b.3.c` (target: <450 LOC)

- Remove legacy libtooling parser-invocation entry points and helper chain from `lib.rs`.
- Keep parser-output handoff entry points (`transpile_parser_output_to_rust*`) as the only production transpile path.

Files:

- `crates/fragile-clang/src/lib.rs`

### `P0.b.3.d` (target: <350 LOC)

- Update dependent tests/examples to stop using removed parser-backend APIs.
- Add anti-regression assertions that removed APIs/variants are not reintroduced.

Files:

- `crates/fragile-clang/tests/m7_shadow_mode_tests.rs`
- `crates/fragile-clang/tests/m8_cutover_tests.rs`
- `crates/fragile-clang/tests/parser_backend_parity_tests.rs`
- `crates/fragile-clang/tests/p0_libtooling_removal_audit_tests.rs`
- `crates/fragile-clang/examples/*.rs`

## 4. Validation Gates

Per-slice focused checks:

```bash
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test -p fragile-clang --test m8_cutover_tests
cargo test -p fragile-clang --test m7_shadow_mode_tests
```

Mandatory full gates after slice execution:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

## 5. Wrong-Approach Guard

Checked against:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Constraints applied:

- no target-specific hacks
- no force-native bypass
- no semantic stubs/fake behavior to hide parser gaps
