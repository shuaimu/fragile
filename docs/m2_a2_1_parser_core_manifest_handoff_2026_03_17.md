# M2.A2.1 Parser-Core Manifest Handoff Contract (2026-03-17)

## Objective

Close TODO leaf `M2.A2.1` by adding a deterministic parser-core parse-manifest
handoff artifact in strict entry points (`fragile-driver` and `fragilec`).

## Scope and Sizing

This change stays below 1000 LOC:

- extend strict parser-core preflight in:
  - `crates/fragile-driver/src/lib.rs`
  - `crates/fragile-cli/src/bin/fragilec.rs`
- add deterministic manifest writer helpers in both files
- add focused unit tests for manifest determinism/shape
- update TODO/dev-book/docs

No additional TODO decomposition was required for this leaf.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

This implementation avoids forbidden shortcuts:

- no target-specific hacks
- no fake semantic fallback method bodies
- no force-native bypass strategy

## Design

Added optional parser-core manifest output controlled by:

- `FRAGILEC_PARSER_CORE_MANIFEST_DIR=<path>`

Behavior:

1. strict parser-core preflight parse runs as before for backend
   `fragile-parser-clang`
2. schema-version guard remains enforced (`1.0.0`)
3. when manifest dir env is set, write deterministic summary artifact per source:
   - stable source-hash filename
   - schema/backend/source/language metadata
   - frontend/define/include counts
   - node/diagnostic counts
   - first/last node id
   - sorted node-kind frequency lines

This creates an explicit parser-output handoff artifact without introducing fake
codegen behavior.

## Validation

Focused:

- `cargo test -p fragile-driver`
- `cargo test -p fragile-cli`

Full regression gates:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Set `FRAGILEC_PARSER_CORE_MANIFEST_DIR` to capture parser-core parse summaries:

```bash
FRAGILEC_PARSER_BACKEND=fragile-parser-clang \
FRAGILEC_PARSER_CORE_MANIFEST_DIR=/tmp/fragile_parser_manifests \
fragilec -c unit.cpp -o unit.o
```

Current cutover behavior still returns deterministic codegen-boundary error for
`fragile-parser-clang`, but the parse summary manifest is emitted first.
