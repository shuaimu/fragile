# M2.1 fragile-parser-core Trait and Registry (2026-03-17)

## Objective

Close TODO leaf `M2.1` by introducing a dedicated `fragile-parser-core` module
that defines parser backend trait contracts and deterministic backend registry
behavior.

## Scope and Sizing

This change is below 1000 LOC:

- add new workspace crate:
  `crates/fragile-parser-core`
- define parser backend trait + request/output models + registry:
  `crates/fragile-parser-core/src/lib.rs`
- add focused registry unit tests in that crate
- update workspace and TODO task status

No decomposition was required.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This leaf intentionally avoids forbidden patterns:

- no target-specific parser behavior
- no fake semantic fallback method bodies
- no force-native bypasses

## Design

Added new crate `fragile-parser-core` with:

- parser backend trait:
  - `ParserBackend` with `backend_id()` and `parse(...)`
- common request/output model for backend contracts:
  - `ParseRequest`
  - `ParserOutputV1`
  - `ParserTranslationUnit`
  - `ParserNode`
  - `ParserDiagnostic`
- deterministic backend registry:
  - `BackendRegistry` backed by `BTreeMap` for stable backend-id ordering
  - duplicate/unknown/invalid id validation
  - parse dispatch with backend-attributed error wrapping (`ParserCoreError`)

Unit tests cover:

- successful registration + dispatch
- duplicate backend rejection
- unknown backend failure
- wrapped backend parse failure
- deterministic sorted backend id listing
- invalid/empty backend id rejection

## Validation

Focused:

- `cargo test -p fragile-parser-core`

Full Python:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`

Full workspace:

- `cargo test --workspace --all-targets`

## User Manual

Use `fragile-parser-core` as the integration contract for next milestones:

1. `M2.2` adds a concrete backend that implements `ParserBackend`.
2. `M2.3` wires transpiler entry points to construct/select backends via
   `BackendRegistry`.
