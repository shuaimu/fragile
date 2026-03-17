# M3.1 Canonical STL Symbol Detection Breakdown (2026-03-17)

## Objective

Decompose TODO item `M3.1` into bounded leaves (<1000 LOC each), execute the
first leaf, and record design rationale for the parser-first STL-opaque path.

## Scope and Sizing

`M3.1` as originally written was too broad for one implementation step because
it bundles:

- direct `std::` symbol matching
- typedef alias-chain normalization
- `using` chain resolution
- regression coverage across all of the above

It is now decomposed into:

- `M3.1.a` direct canonical `std::` symbol detection utility
- `M3.1.b` typedef/type-alias table extraction + canonical target normalization
- `M3.1.c` using declaration/directive chain resolution
- `M3.1.d` deterministic direct/alias/using regression fixtures

Each leaf remains comfortably below 1000 LOC.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails followed:

- no semantic stubs/fallback method bodies
- no target-specific hacks
- no force-native bypasses
- no silent fallback behavior

## Leaf Executed: M3.1.a

Implemented direct canonical STL symbol detection utility in
`fragile-parser-clang`:

- detects direct `std::` family spellings from node `name` and `cpp_type`
- supports known families:
  `vector`, `map`, `unordered_map`, `string` (including `basic_string`),
  `optional`, `variant`, `tuple`, `shared_ptr`, `unique_ptr`
- handles common inline/passthrough namespace segments:
  `std::__1::...`, `std::__cxx11::...`, `std::pmr::...`,
  `std::experimental::...`
- rejects non-`std` and non-target spellings deterministically

## Validation

Focused:

- `cargo test -p fragile-parser-clang`

Full regression gates:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

`fragile-parser-clang` now exposes:

- `detect_direct_std_stl_family(name, cpp_type) -> Option<&'static str>`

This is a direct-symbol detector intended as the first-stage building block for
`M3.1.b`/`M3.1.c` alias-chain aware detection and the later `M3.2` placeholder
boundary emission.
