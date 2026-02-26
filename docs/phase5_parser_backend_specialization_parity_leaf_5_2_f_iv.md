# Phase 5.2.f.iv: Parser-backend specialization parity markers

Date: 2026-02-26

## Scope

Extend parser-backend parity fixture coverage with concrete template-specialization
markers and require libtooling parity against libclang/hybrid for those markers.

## Design rationale

Two gaps were identified while tightening parity markers:

1. Class-template specialization names in libtooling conversion sometimes dropped
   template arguments in `RecordDecl` names, which made specialization markers
   backend-divergent.
2. Function-template specialization `FunctionDecl` nodes can be omitted from
   computed top-level roots in the exported graph, which prevented deterministic
   emission of helper functions like `identity_i32` in libtooling-primary output.

The implementation keeps behavior bounded by:
- Only promoting function-decl roots that are concrete template-instantiation
  surfaces (`extras[4]` or non-empty template-arg payload in `extras[5]`),
  have a body, and have non-empty/non-internal names.
- Preserving existing variadic-template comment wording to avoid integration
  regressions in established tests.

## Implementation

- `crates/fragile-ast-exporter/src/AstExporter.cpp`
  - `VisitFunctionDecl` now treats non-empty template-specialization arg payloads
    as concrete instantiation metadata even when Clang does not mark
    `isTemplateInstantiation()`.

- `crates/fragile-clang/src/libtooling.rs`
  - `FunctionDecl` conversion now maps to `FunctionTemplateInstantiation` when
    either instantiation flag is set or specialization args are present.
  - `ClassTemplateSpecializationDecl` conversion now reconstructs specialization
    names with template args when the qualified name omits `<...>`.

- `crates/fragile-clang/src/lib.rs`
  - Libtooling translation-unit construction now promotes concrete function-
    template instantiation decl surfaces into root traversal when they are
    otherwise non-root in the exported graph.

- `crates/fragile-clang/src/ast_codegen.rs`
  - `generate_top_level` now emits parser-surfaced
    `FunctionTemplateInstantiation` nodes directly.

- `crates/fragile-clang/tests/parser_backend_parity_tests.rs`
  - Parity fixture now includes explicit class/function instantiation surfaces:
    `template struct Box<int>;` and `template int identity<int>(int);`.
  - Added specialization markers to parity manifest and assertions:
    - `pub fn identity_i32`
    - `pub struct Box_int_`

## Validation

Executed and passing:

- `cargo test -p fragile-clang --test parser_backend_parity_tests -- --nocapture`
- `cargo test -p fragile-clang libtooling::tests -- --nocapture`
- `cargo test -p fragile-clang --test integration_test test_variadic_template_transpile -- --nocapture`
- `cargo test` (full workspace)
