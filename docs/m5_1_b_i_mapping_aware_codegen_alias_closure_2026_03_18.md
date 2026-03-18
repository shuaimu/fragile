# M5.1.b.i Mapping-Aware Codegen Alias Closure (2026-03-18)

## Objective

Close TODO leaf `M5.1.b.i` by plumbing parser-output placeholder mappings into
`AstCodeGen` and using mapping-aware unresolved associative alias closure in the
active generate path.

## Scope and Sizing

This leaf stays below 1000 LOC and targets one concrete wiring step:

- pass parser-output placeholder node-kind -> canonical prefix mappings into codegen state
- use mapping-aware closure logic for unresolved associative lowered names (`map_*`, `unordered_map_*`)
- in active mapping-aware runs, avoid routing mapped associative families through legacy `std::collections::*` fallback when shape is unsupported
- keep legacy closure behavior unchanged when parser-output mappings are absent

Out of scope:

- full family-dispatch replacement for all STL families (`M5.1.b.ii` / `M5.1.b.iii`)
- complete removal of legacy fallback lanes (`M5.1.c`)

## Wrong-Approach Check

Checked against `docs/fragile-dev-book.md` section `1.3 Wrong Approaches`:

- no target-specific conditionals
- no force-native escape hatch
- no synthetic semantic fallback method bodies
- unsupported mapped associative shapes remain explicit unresolved placeholder lanes, not silently remapped to unrelated std semantics

## Design

### 1. Codegen state plumbing

`AstCodeGen` now stores parser-output placeholder mappings:

- field: `parser_output_stl_placeholder_prefixes: BTreeMap<String, String>`
- setter: `set_parser_output_stl_placeholder_mappings(...)`

Parser-output handoff (`transpile_parser_output_to_rust_with_options`) now:

1. resolves placeholder mappings,
2. injects them into `AstCodeGen`,
3. runs generation with mapping context attached.

### 2. Mapping-aware unresolved associative closure

The unresolved type closure path now has mapping-aware entry:

- `close_unresolved_type_reference_gaps_with_parser_output_placeholder_mappings(...)`

When mapping context is present for associative families:

- `map` / `unordered_map` family lanes are treated as mapping-owned,
- supported pre-generated concrete lane currently resolves `int/int` ->
  `std_map_int__int` / `std_unordered_map_int__int` (via canonical prefix),
- unsupported mapped associative shapes do **not** fall back to legacy
  `std::collections::{BTreeMap, HashMap}` aliasing.

Without mapping context, legacy behavior remains unchanged.

### 3. Missing-stub alias path alignment

`resolve_missing_stub_concrete_alias_target(...)` now respects mapping-owned
associative families:

- mapping-aware alias resolution first,
- legacy associative fallback only when family is not mapping-controlled.

## Validation

Focused tests:

- `cargo test -p fragile-clang close_unresolved_type_reference_gaps_with_placeholder_mapping -- --nocapture`
- `cargo test -p fragile-clang parser_output_codegen_uses_handoff_metadata_without_libtooling_export -- --nocapture`
- `cargo test -p fragile-clang parser_output_stl_placeholder_mapping -- --nocapture`

Added unit tests:

- mapping-aware closure blocks legacy `std::collections::BTreeMap` fallback for mapped `map_*` unsupported shapes
- mapping-aware closure resolves `map_int__int` to pre-generated `std_map_int__int` when concrete pre-generated surface is present

## User Notes

For future mapped family rollout:

1. Keep parser-output mapping table and fragile-stl contract entries in sync.
2. Add mapping-aware concrete alias rules before removing legacy fallback paths.
3. Add explicit tests for both:
   - supported concrete mapped lane
   - unsupported mapped lane that must remain non-silent.
