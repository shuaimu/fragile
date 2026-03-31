# P0.b.2.b.1.b.1.1 Immediate StrictParserBackend Declaration Removal

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.1` (immediate)

## Scope

Removed `StrictParserBackend::Libtooling` from `crates/fragile-driver/src/lib.rs` and updated strict-backend parsing/tests in the same file so the driver compiles with parser-core-only `StrictParserBackend` declarations.

## Why This Leaf Is Bounded

This change is localized to declaration and direct compile dependencies in one production file plus its related audit contract tests:

- `crates/fragile-driver/src/lib.rs`
- `crates/fragile-clang/tests/p0_libtooling_removal_audit_tests.rs`

Total edit size remains well under the <1000 LOC leaf target.

## Wrong-Approach Guard Check

Re-checked before/after implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific hacks, no force-native bypasses, and no semantic stubs were introduced.

## Implementation Notes

1. Removed `Libtooling` variant from `StrictParserBackend` enum.
2. Switched strict backend value parsing to parser-core-only supported values.
3. Removed explicit `FRAGILEC_PARSER_BACKEND=libtooling` backend-variant policy branch that depended on the removed enum variant.
4. Kept parser-core codegen escape-hatch handling (`ParserCoreCodegenEscapeHatch::Libtooling`) unchanged for its dedicated follow-up leaf.
5. Updated driver unit tests and P0 audit assertions to reflect:
   - immediate TODO decomposition labels
   - `StrictParserBackend::Libtooling` removal from fragile-driver only.

## Verification

Targeted verification run after edits:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

Full regression run for this cycle also completed:

```bash
cargo test --workspace --all-targets
python3 -m unittest discover -s tests/python -p 'test_*.py'
```

(Workspace run includes long-running `integration_test` and `m9_rpc_closure_tests` legs; both completed with zero failures.)
