# P0.b.6 CLI `--use-libtooling` Removal (2026-03-23)

## Scope and Sizing

- Leaf task: remove deprecated CLI `--use-libtooling` pre-parse path and related stale artifacts.
- Estimated edit surface: <200 LOC across CLI wiring, one example file, and anti-regression tests.
- This is well below the <1000 LOC leaf guideline, so no further decomposition is required.

## Findings

1. `crates/fragile-cli/src/main.rs` still exposed `use_libtooling: bool` and emitted a deprecation warning even though P0.b.5 removed LibTooling enrichment data flow.
2. `examples/debug_libtooling.rs` was still present and referenced removed/legacy LibTooling APIs.
3. `crates/fragile-clang/tests/p0_libtooling_removal_audit_tests.rs` still had audit expectations that the flag/example existed.
4. `crates/fragile-clang/tests/p0c_anti_regression_tests.rs` still budgeted 7 `libtooling` references in `main.rs` and aggregate 64 across production files.

## Implementation Decisions

- Remove the CLI flag entirely instead of keeping a deprecated no-op path.
- Delete the stale debug example instead of migrating it to parser-core, because the task explicitly targets LibTooling artifact removal.
- Convert stale presence-audit checks into anti-regression absence checks.
- Lower P0.c ceilings to reflect the reduced production-path reference budget.

## Wrong-Approach Check

Reviewed `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)` and `docs/dev/wrong.md` before edits:

- No target-specific hacks added.
- No escape-hatch/native bypass introduced.
- No semantic fake stubs introduced.
- Changes are generic production-path cleanup with anti-regression coverage.
