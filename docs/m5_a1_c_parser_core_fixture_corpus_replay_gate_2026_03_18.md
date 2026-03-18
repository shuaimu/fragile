# M5.A1.c Parser-Core Fixture-Corpus Replay Gate (2026-03-18)

## Objective

Close TODO leaf `M5.A1.c` by adding a deterministic parser-core fixture replay
gate that:

- audits observed STL placeholder kinds from parser output, and
- asserts active parser-output handoff output does not leave unresolved mapped-
  family placeholder structs.

## Scope and Sizing

This leaf is small (<1000 LOC):

- add one replay gate test in `fragile-parser-clang` fixture tests
- add one focused mapping-completeness regression in `fragile-clang`
- tighten mapped-family completeness candidate filtering for known non-
  placeholder helper surfaces
- update docs/TODO status

## Wrong-Approach Check

Reviewed:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`
- `docs/dev/wrong.md`

No forbidden approach was used:

- no target-specific hacks
- no force-native bypasses
- no fake semantic fallback bodies

## Design and Implementation

### 1. Parser-core replay gate

Added test in:

- `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`

New test:

- `parser_core_fixture_replay_gate_keeps_mapped_placeholder_families_resolved_in_active_handoff_output`

Behavior:

1. Parse `m3_1_d/src/stl_symbol_detection.cpp` through parser-core backend.
2. Audit boundary placeholder manifest under `consume_symbols` and assert
   observed placeholder kinds exactly cover mapped families:
   - `map`, `unordered_map`, `vector`, `string`, `optional`, `variant`,
     `tuple`, `shared_ptr`, `unique_ptr`.
3. Replay active handoff via `fragile_clang::transpile_parser_output_to_rust`.
4. Assert deterministic observed-family mapping manifest lines are present in
   transpiled output.
5. Assert no unresolved mapped-family placeholder struct shapes remain in final
   output.

### 2. Completeness false-positive hardening

The replay gate surfaced deterministic false positives in mapping-completeness
validation for non-placeholder helper surfaces:

- `basic_string_view_*`
- tuple helper artifacts like `tuple_element_*` and `tuple_size_*`

Updated `crates/fragile-clang/src/lib.rs`:

- Added `parser_output_lowered_name_is_covered_family_candidate(...)`
- Wired it into
  `parser_output_covered_family_spec_for_lowered_name(...)`

This keeps mapped-family completeness checks focused on placeholder-lowered
target surfaces while ignoring known helper artifacts that are not mapped
placeholder resolution lanes.

Added regression:

- `parser_output_mapping_completeness_validation_ignores_string_view_and_tuple_helper_surfaces`

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_mapping_completeness_validation_ignores_string_view_and_tuple_helper_surfaces -- --nocapture`
- `cargo test -p fragile-parser-clang parser_core_fixture_replay_gate_keeps_mapped_placeholder_families_resolved_in_active_handoff_output -- --nocapture`
- `cargo test -p fragile-parser-clang stl_symbol_detection_fixture_ -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
