# Phase 5.2.e.iii: Non-type/template-template parameter handling

Date: 2026-02-26

## Decision

Chose the **constrained fallback** path for this leaf: map LibTooling
`TagNonTypeTemplateParmDecl` and `TagTemplateTemplateParmDecl` into the existing
AST shape `ClangNodeKind::TemplateTypeParmDecl`, instead of introducing new
`ClangNodeKind` variants in this step.

Rationale:
- Keeps this leaf bounded well under the <500 LOC target.
- Removes `Unknown("TemplateParam")` nodes from LibTooling conversion on active paths.
- Preserves metadata needed by current codegen/template routing (`name`, `depth`, `index`, `is_pack`).
- Avoids broad match-exhaustiveness churn across the full AST/codegen surface before it is needed.

## Implementation

- `libtooling.rs` now maps:
  - `TagNonTypeTemplateParmDecl` -> fallback `TemplateTypeParmDecl`
  - `TagTemplateTemplateParmDecl` -> fallback `TemplateTypeParmDecl`
- Added shared helper `convert_template_param_decl_fallback_node(...)` with stable unnamed-parameter prefixes.
- Existing `FunctionTemplateDecl` conversion keeps using child tag inspection to derive
  template-parameter ordering and pack indices, so behavior remains deterministic.

## Validation

Added focused regressions in `libtooling.rs`:
- synthetic conversion test for non-type template parameter fallback mapping
- synthetic conversion test for template-template parameter fallback mapping
- parse-roundtrip test with `template<int N, template<typename> class W, typename T>`
  proving fallback conversion is concrete and avoids `Unknown("TemplateParam")` children
