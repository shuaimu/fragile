# Phase 5.2.f analysis: specialization declaration gaps

Date: 2026-02-26

## Scope check

`5.2.f` is too large for a single leaf under the <500 LOC target.
It spans both class-template specialization declaration conversion and
function-template specialization surfaces, plus parity harness coverage.

## Decomposition decision

Split into bounded leaves:
- `5.2.f.i`: class-template-specialization declaration conversion from
  `Unknown(...)` to concrete node shape with child linkage.
- `5.2.f.ii`: class-template-specialization metadata parity tests for exporter
  extras (qualified name, template arg list, implicit/explicit markers).
- `5.2.f.iii`: free-function specialization handling for instantiated
  `FunctionDecl` surfaces.
- `5.2.f.iv`: parser-backend parity marker expansion for specialization
  declarations.

## First leaf execution target

Execute `5.2.f.i` in this cycle.

Planned implementation size: ~200-350 LOC across `libtooling.rs` conversion +
focused tests. This stays within the leaf-size constraint and avoids introducing
new `ClangNodeKind` variants until metadata parity is locked.
