# M5.A2.a Parser-Core Fixture Replay Legacy Fallback Gate (2026-03-18)

## Objective

Close TODO leaf `M5.A2.a` by extending parser-core fixture replay regression
coverage so active parser-output handoff output:

- emits deterministic observed-family manifest markers for covered mapped
  placeholder families
- rejects covered associative-family legacy deep STL fallback alias lanes
  (`std::collections::BTreeMap` / `std::collections::HashMap`) in alias
  closure output

## Scope and Sizing

This leaf is small (<1000 LOC):

- adjust one parser-core replay regression test in
  `crates/fragile-parser-clang/tests/stl_symbol_detection_fixture_tests.rs`
- add one helper for alias-line fallback detection used by that regression
- no production code path changes

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3` and
`docs/dev/wrong.md`:

- no semantic stubs/fake method bodies
- no target-specific hacks
- no force-native bypasses
- no silent acceptance of covered-family legacy deep STL fallback aliases

## Design

1. Keep deterministic observed-family manifest assertions in fixture replay
   output, including canonical type prefix markers for covered associative
   families (`map`, `unordered_map`).
2. Use alias-shape-aware fallback detection instead of global substring bans:
   - scan only `pub type ... = ...;` alias lines
   - treat `map_` / `std_map_` aliases targeting
     `std::collections::BTreeMap<...>` as violations
   - treat `unordered_map_` / `std_unordered_map_` aliases targeting
     `std::collections::HashMap<...>` as violations
3. Avoid false positives from legitimate runtime/support uses of
   `std::collections::HashMap`/`BTreeMap` that are not alias fallback lanes.

## Validation

Focused:

- `cargo test -p fragile-parser-clang parser_core_fixture_replay_gate_keeps_mapped_placeholder_families_resolved_in_active_handoff_output -- --nocapture`
- `cargo test -p fragile-parser-clang --test stl_symbol_detection_fixture_tests -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Notes

1. The replay gate now detects legacy deep STL fallback only in mapped
   associative alias closure lanes, which aligns with M5 mapping policy and
   avoids unrelated runtime-helper false positives.
2. Any future covered-family alias regression back to `std::collections::*`
   fallback lanes in active parser-output handoff replay now fails
   deterministically.
