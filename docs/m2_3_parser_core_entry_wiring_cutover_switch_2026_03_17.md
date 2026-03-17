# M2.3 Parser Entry Wiring and Cutover Switch (2026-03-17)

## Objective

Close TODO leaf `M2.3` by wiring strict transpiler entry points to the
`fragile-parser-core` backend trait path, selected by
`FRAGILEC_PARSER_BACKEND`.

## Scope and Sizing

This change stays below 1000 LOC:

- update strict backend selection and parse preflight in:
  - `crates/fragile-driver/src/lib.rs`
  - `crates/fragile-cli/src/bin/fragilec.rs`
- add parser-core/clang backend crate wiring deps:
  - `crates/fragile-driver/Cargo.toml`
  - `crates/fragile-cli/Cargo.toml`
- add/adjust strict backend and cutover-boundary unit tests
- update TODO and dev-book notes

No TODO decomposition was required.

## Wrong-Approach Check

Reviewed before implementing:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This implementation avoids forbidden shortcuts:

- no target-specific hacks
- no fake semantic fallback method bodies
- no force-native bypass behavior

## Design

Strict parser backend selection now accepts two explicit values:

- `libtooling`
- `fragile-parser-clang`

Driver/CLI behavior:

1. Parse backend value into a strict enum (`Libtooling` or `ParserCore` id).
2. For parser-core backend ids, register `FragileParserClangBackend` in
   `BackendRegistry` and run parse preflight via `ParseRequest`.
3. Validate parser output schema version (`v1`).
4. Return a deterministic cutover-boundary error indicating parser-core is wired
   but codegen cutover is not yet implemented.
5. Keep existing transpile path on explicit `libtooling`.

This gives a real parser-core integration seam now without introducing fake
codegen behavior.

## Validation

Focused:

- `cargo test -p fragile-driver`
- `cargo test -p fragile-cli`

Full regression gates for this leaf:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Use `FRAGILEC_PARSER_BACKEND` with:

- `libtooling`: current strict transpile/codegen path
- `fragile-parser-clang`: parser-core parse preflight path with deterministic
  cutover-boundary error until codegen cutover milestone lands
