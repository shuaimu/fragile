# P0.b.2.b.1.b.2 Adapter Mapping Removal

Date: 2026-03-22
Task: `P0.b.2.b.1.b.2` (immediate)

## Scope

Removed `strict_parser_backend_from_legacy_backend` function and its test callsites from `crates/fragile-driver/src/lib.rs`.

## Why This Leaf Is Bounded

This change is localized to one `#[cfg(test)]` function and its two test callsites in a single file:

- `crates/fragile-driver/src/lib.rs`

Total edit: deletion of 17 lines (function body) + 7 lines (test callsites), plus 2 comment lines added.

## Rationale for Full Function Removal

The function `strict_parser_backend_from_legacy_backend` mapped `ClangParserBackend` variants to `StrictParserBackend` variants:

- `ClangParserBackend::Libtooling` -> `Ok(StrictParserBackend::Libtooling)` (sole success arm)
- `ClangParserBackend::Libclang | Hybrid` -> `Err(...)` (rejection)

After P0.b.2.b.1.b.1.1 removed `StrictParserBackend::Libtooling`, the function's only successful mapping arm lost its target. All inputs would produce errors, making the function dead code. Rather than patching it to return errors for all inputs, the function was removed entirely.

## Wrong-Approach Guard Check

Re-checked before/after implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

No target-specific hacks, no force-native bypasses, and no semantic stubs were introduced.

## Implementation Notes

1. Removed `strict_parser_backend_from_legacy_backend` function (lines 906-922, `#[cfg(test)]` only).
2. Removed two test callsites in `strict_parser_backend_validation_accepts_parser_core_backend`:
   - `strict_parser_backend_from_legacy_backend(ClangParserBackend::Libtooling).expect(...)` assertion
   - `strict_parser_backend_from_legacy_backend(ClangParserBackend::Libclang).expect_err(...)` assertion
3. Added comment noting the removal and its task ID.
4. Kept remaining test assertions (`parse_parser_backend_value`, `strict_parser_backend_from_value`) unchanged — those reference `StrictParserBackend::Libtooling` which is a P0.b.2.b.1.b.3 concern.
5. Did NOT remove `ClangParserBackend` import or its usage at line 1327 — those are separate concerns for P0.b.2.b.1.b.3/P0.b.2.c.

## Ownership Boundaries

- `StrictParserBackend::Libtooling` references in `parse_parser_backend_value`, `strict_parser_backend_label`, match arms: **P0.b.2.b.1.b.3** / **P0.b.2.c**
- `ClangParserBackend::Libtooling` field assignment in `strict_transpile_source`: **P0.b.2.b.1.b.3**
- Backend string/help text removals: **P0.b.2.c**

## Verification

Targeted verification run after edits:

```bash
cargo test -p fragile-driver
cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests
```

Full regression run:

```bash
cargo test --workspace
```
