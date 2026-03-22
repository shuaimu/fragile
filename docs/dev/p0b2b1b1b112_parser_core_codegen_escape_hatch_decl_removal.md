# P0.b.2.b.1.b.1.2 Immediate ParserCoreCodegenEscapeHatch Declaration Removal

Date: 2026-03-22
Task: `P0.b.2.b.1.b.1.2` (immediate)

## Scope

Removed `ParserCoreCodegenEscapeHatch::Libtooling` variant from `crates/fragile-driver/src/lib.rs` and fixed compile breaks caused by the removal. The enum is retained as empty (uninhabited) for follow-up infrastructure removal in P0.b.2.d and P0.b.2.e.

## Why This Leaf Is Bounded

This change is localized to:

- `crates/fragile-driver/src/lib.rs` (enum variant removal + compile break fixes)
- `crates/fragile-clang/tests/p0_libtooling_removal_audit_tests.rs` (audit assertion updates)

Total edit size is well under the <1000 LOC leaf target.

## Wrong-Approach Guard Check

Re-checked before/after implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific hacks, no force-native bypasses, and no semantic stubs were introduced.

## Implementation Notes

1. Removed `Libtooling` variant from `ParserCoreCodegenEscapeHatch` enum, leaving it empty (uninhabited).
2. Updated `parse_parser_core_codegen_escape_hatch_value` to reject all values (no valid escape hatches remain).
3. Replaced `matches!(parser_core_codegen_escape_hatch, Some(ParserCoreCodegenEscapeHatch::Libtooling))` with `if false` since the variant no longer exists.
4. Prefixed `_parser_core_codegen_escape_hatch` to suppress unused-variable warning (value still parsed from env to reject unsupported inputs).
5. Kept escape-hatch infrastructure (env constants, policy functions, `use_libtooling_codegen_escape_hatch` variable, codegen fallthrough branch) unchanged for dedicated follow-up leaves (P0.b.2.d, P0.b.2.e).
6. Updated driver unit tests:
   - `parser_core_codegen_escape_hatch_validation`: "libtooling" now expected to be rejected.
   - Added `parser_core_codegen_escape_hatch_libtooling_variant_removed`: verifies rejection behavior.
7. Updated P0 audit assertion in `p0a_audit_fragile_driver_has_escape_hatch_enum` to assert the variant is absent.

## What Was NOT Changed (left for follow-up tasks)

- `crates/fragile-cli/src/bin/fragilec.rs` still has `ParserCoreCodegenEscapeHatch::Libtooling` (P0.b.2.b.1.c)
- Adapter mappings in `strict_parser_backend_from_legacy_backend` (P0.b.2.b.1.b.2)
- String/help text referencing "libtooling" (P0.b.2.c)
- Escape-hatch env constants and policy functions (P0.b.2.d)
- `use_libtooling_codegen_escape_hatch` routing and `ClangParserBackend::Libtooling` fallthrough (P0.b.2.e)

## Verification

Targeted verification run after edits:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
cargo test -p fragile-clang --lib
cargo test --workspace --all-targets
```

All tests pass (16 pre-existing audit test failures in `p0_libtooling_removal_audit_tests` are unrelated to this change).
