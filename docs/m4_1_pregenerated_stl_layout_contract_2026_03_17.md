# M4.1 Versioned Pre-Generated STL Layout + Naming Contract (2026-03-17)

## Objective

Close TODO leaf `M4.1` by creating a versioned contract for pre-generated STL
module layout and naming so downstream mapping work (`M5+`) has one
deterministic source of truth.

## Scope and Sizing

This leaf is below 1000 LOC and scoped to contract + wiring + regression tests:

- add explicit v1 layout contract for pre-generated STL module order/file names
- add explicit v1 family naming contract for required STL placeholder families
- wire codegen preamble emission to contract manifest (remove duplicated module
  list)
- add focused contract regression tests in `fragile-stl` and `fragile-clang`

Out of scope for this leaf:

- implementing missing STL family operations (`M4.2`)
- reproducibility manifests/checks (`M4.3`)

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails preserved:

- no target-specific conditionals
- no force-native bypasses
- no semantic fallback method stubs to hide missing STL behavior
- no silent family remaps to unrelated Rust std container semantics

## Design

### 1. Single-source contract in `fragile-stl`

Added `crates/fragile-stl/src/layout_contract.rs` with:

- `PREGENERATED_STL_LAYOUT_VERSION_V1 = "v1"`
- `PREGENERATED_STL_LAYOUT_NAMESPACE_V1 = "fragile_stl::v1"`
- deterministic ordered `PREGENERATED_STL_MODULES_V1` manifest:
  - `module_id`
  - `source_file`
  - `sentinel`
- deterministic `PREGENERATED_STL_FAMILY_CONTRACT_V1` for required families:
  - `family`
  - `module_id`
  - `canonical_type_prefix`
  - `status` (`Available` or `Planned`)

Also added helper APIs:

- `pre_generated_stl_modules_v1()`
- `pre_generated_stl_family_contract_v1()`
- `pre_generated_stl_family_contract_entry_v1(family)`
- `pre_generated_stl_module_source_v1(module_id)`

### 2. Contract-driven preamble emission

Updated `crates/fragile-clang/src/ast_codegen.rs` `emit_stl_preamble()`:

- emits a layout marker comment with contract version + namespace
- iterates `pre_generated_stl_modules_v1()` and loads source text via
  `pre_generated_stl_module_source_v1()`
- fails deterministically if a module in the contract has no resolvable source

This removes duplicated hardcoded preamble file ordering from `fragile-clang`.

### 3. Contract validation coverage

Added `crates/fragile-stl/tests/layout_contract_tests.rs`:

- module manifest order + uniqueness lock
- module source resolution + sentinel presence checks
- required family coverage checks (`vector`, `map`, `unordered_map`, `string`,
  `optional`, `variant`, `tuple`, `shared_ptr`, `unique_ptr`)
- available-family canonical prefix presence in mapped module source

Added `fragile-clang` regression:

- `test_preamble_emits_versioned_fragile_stl_layout_contract_modules_in_order`
  verifies preamble contract marker + module marker order + sentinel presence

## Validation

Focused:

- `cargo test -p fragile-stl layout_contract_tests -- --nocapture`
- `cargo test -p fragile-clang --lib test_preamble_emits_versioned_fragile_stl_layout_contract_modules_in_order -- --nocapture`

Full regression:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

## User Manual

When adding/modifying pre-generated STL modules:

1. Update `layout_contract.rs` v1 manifest (or add v2 manifest for breaking
   layout changes).
2. Keep module order deterministic and provide a stable sentinel per module.
3. Update family naming/status contract entries.
4. Ensure `fragile-clang` preamble emission still consumes the contract and
   regression tests stay green.
