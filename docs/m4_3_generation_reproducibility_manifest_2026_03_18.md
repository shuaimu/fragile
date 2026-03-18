# M4.3 Generation Reproducibility Manifest (2026-03-18)

## Objective

Close TODO leaf `M4.3` by adding deterministic generation metadata and
reproducibility checks for pre-generated STL outputs.

## Scope and Sizing

This leaf is under 1000 LOC and intentionally narrow:

- add deterministic per-module manifest APIs in `fragile-stl` layout contract
- emit deterministic manifest comments from `AstCodeGen` preamble generation
- add focused reproducibility regressions in `fragile-stl` and `fragile-clang`

No placeholder-to-runtime mapping cutover is included here (`M5` remains
responsible).

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no fake semantic method bodies
- no target-specific special casing
- no force-native bypasses
- deterministic manifest generation uses stable module order and stable hashing

## Design

Updated:

- `crates/fragile-stl/src/layout_contract.rs`
- `crates/fragile-stl/tests/layout_contract_tests.rs`
- `crates/fragile-clang/src/ast_codegen.rs`

Implementation details:

- Added `PreGeneratedStlModuleManifestEntryV1` and APIs:
  - `pre_generated_stl_module_manifest_v1()`
  - `pre_generated_stl_module_manifest_text_v1()`
- Added deterministic per-module source fingerprinting (`fnv1a64`) and stable
  metadata fields (bytes, lines, module/source identity).
- `AstCodeGen::emit_stl_preamble` now emits a manifest comment block before
  module inlining.
- Added regressions for deterministic manifest generation and preamble manifest
  presence/reproducibility on identical AST input.

## Validation

Focused:

- `cargo test -p fragile-stl --test layout_contract_tests`
- `cargo test -p fragile-clang test_preamble_emits_versioned_fragile_stl_layout_contract_modules_in_order -- --nocapture`
- `cargo test -p fragile-clang test_preamble_generation_is_byte_reproducible_for_same_input_ast -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Manual

For deterministic generated-output manifest usage:

1. Call `pre_generated_stl_module_manifest_v1()` for structured per-module
   fingerprints.
2. Call `pre_generated_stl_module_manifest_text_v1()` for stable rendered
   manifest text suitable for embedding/logging.
3. Read generated preamble comments starting at
   `// fragile_stl module manifest:` for in-output reproducibility metadata.
