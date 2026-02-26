# Phase 5.2.f.i: Class-template-specialization declaration conversion

Date: 2026-02-26

## Scope

Implement concrete LibTooling conversion for `TagClassTemplateSpecializationDecl`
so specialization declarations no longer map to `Unknown(...)` in the converted AST.

## Implementation

- `libtooling.rs` now maps `TagClassTemplateSpecializationDecl` through a dedicated
  converter (`convert_class_template_specialization_decl_node`).
- Conversion result is `ClangNodeKind::RecordDecl` (constrained fallback shape) with:
  - specialization identity name preferring exporter qualified specialization spelling
  - concrete field list extracted from `TagFieldDecl` children
  - `is_definition` inferred from member-child presence
  - preserved converted child linkage (fields/methods)
- Added a test-only parse lock (`OnceLock<Mutex<()>>`) around parse-roundtrip tests in
  `libtooling.rs` to keep LibTooling parse invocations serialized and avoid intermittent
  concurrent parse crashes under `cargo test`.

## Validation

Added focused regressions in `libtooling.rs`:
- synthetic conversion test for `TagClassTemplateSpecializationDecl` -> `RecordDecl`
- parse-roundtrip fixture test asserting specialization node export and concrete conversion

Execution evidence:
- `cargo test -p fragile-clang libtooling::tests -- --nocapture` passes with the new coverage.
