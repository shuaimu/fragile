# M5.A2.b Mapped-Context Legacy Alias Reject/Accept Regressions (2026-03-18)

## Objective

Close TODO leaf `M5.A2.b` by adding deterministic negative/positive
parser-output handoff regressions proving:

- covered mapped associative families reject legacy deep STL fallback alias
  forms (`std::collections::BTreeMap` / `std::collections::HashMap`)
- covered mapped associative families accept canonical pre-generated alias
  forms (`std_map_*` / `std_unordered_map_*`)

## Scope and Sizing

This leaf is small (<1000 LOC):

- add one focused `fragile-clang` handoff regression test
- update TODO/docs
- no production logic changes

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no fake semantic stubs
- no target-specific special casing
- no force-native bypass behavior
- no silent acceptance of covered-family legacy fallback alias lanes

## Design

Added test:

- `parser_output_codegen_active_handoff_mapped_associative_legacy_fallback_alias_forms_are_rejected_while_canonical_forms_are_accepted`

Behavior asserted:

1. Active mapped parser-output handoff for associative families emits canonical
   alias targets (`std_map_*`, `std_unordered_map_*`).
2. The covered-family legacy fallback validator accepts that canonical handoff
   output under mapped context.
3. Deterministic alias-lane mutation to legacy forms is rejected by the same
   validator with covered-family diagnostics.

This closes the explicit reject/accept proof gap for mapped-context associative
alias lanes.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_associative_legacy_fallback_alias_forms_are_rejected_while_canonical_forms_are_accepted -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_active_handoff_mapped_associative_supported_families_use_pre_generated_alias_targets -- --nocapture`
- `cargo test -p fragile-clang parser_output_legacy_deep_stl_translation_path_validation_rejects_covered_fallback_aliases -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
