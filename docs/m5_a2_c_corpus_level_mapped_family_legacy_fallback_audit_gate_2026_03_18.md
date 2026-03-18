# M5.A2.c Corpus-Level Mapped-Family Legacy Fallback Audit Gate (2026-03-18)

## Objective

Close TODO leaf `M5.A2.c` by adding a corpus-level active parser-output replay
audit gate that:

- fails on any covered-family legacy deep STL fallback alias marker in replayed
  handoff output
- records deterministic fixture evidence for covered mapped associative families
  observed by parser output

## Scope and Sizing

This leaf is small (<1000 LOC):

- extend parser-core replay fixture tests in
  `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`
- add one corpus-level replay audit test and helper functions
- update TODO/docs
- no production code path changes

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no target-specific hacks
- no semantic stubs/fake method bodies
- no force-native bypasses
- no silent acceptance for covered-family legacy fallback alias lanes

## Design

1. Added fixture-corpus source enumeration helper over `m3_1_d/src` with
   deterministic sorting.
2. Added parser-node-to-covered-family helper for mapped associative families:
   - `stl_map_placeholder` -> `map`
   - `stl_unordered_map_placeholder` -> `unordered_map`
3. Added covered-family-aware alias marker audit helper that flags only covered
   family alias lines resolving to legacy fallback targets:
   - `map_` / `std_map_` -> `std::collections::BTreeMap<...>`
   - `unordered_map_` / `std_unordered_map_` ->
     `std::collections::HashMap<...>`
4. Added corpus-level audit test:
   - `parser_core_fixture_corpus_replay_audit_gate_rejects_covered_family_legacy_fallback_alias_markers`
   - replays each fixture through active handoff transpilation
   - records deterministic fixture evidence of covered families
   - fails with fixture-scoped evidence if any covered-family legacy fallback
     alias marker is detected

## Validation

Focused:

- `cargo test -p fragile-parser-clang parser_core_fixture_corpus_replay_audit_gate_rejects_covered_family_legacy_fallback_alias_markers -- --nocapture`
- `cargo test -p fragile-parser-clang parser_core_fixture_replay_gate_keeps_mapped_placeholder_families_resolved_in_active_handoff_output -- --nocapture`
- `cargo test -p fragile-parser-clang --test stl_symbol_detection_fixture_tests -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
