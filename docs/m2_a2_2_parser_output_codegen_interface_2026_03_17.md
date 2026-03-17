# M2.A2.2 Parser-Output to Codegen Interface (2026-03-17)

## Objective

Close TODO leaf `M2.A2.2` by introducing a parser-output handoff interface
that can drive code generation without invoking LibTooling parse/export.

## Scope and Sizing

This change is under 1000 LOC and limited to:

- `crates/fragile-clang/src/lib.rs`
- `crates/fragile-clang/Cargo.toml`
- `TODO.md`
- docs updates

No additional TODO decomposition is required for this leaf.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails followed:

- no fake fallback semantic bodies
- no target-specific hacks
- no native-force bypasses
- no silent skip logic for failures

## Design

Added new `fragile-clang` API:

- `transpile_parser_output_to_rust(&ParserOutputV1)`
- `transpile_parser_output_to_rust_with_options(&ParserOutputV1, &ParserOutputCodegenOptions)`

Behavior:

1. validates parser-output schema version (`1.0.0`)
2. builds effective include/define sets from parser-output translation-unit metadata
   and frontend args (`-I/-isystem/-iquote`, `-D`)
3. reparses source via `ClangParser` (libclang path) and runs `AstCodeGen`
4. emits stage trace with backend label `parser-output-handoff`

This establishes a parser-output-to-codegen contract and removes the need for
LibTooling parser invocation along this interface.

## Validation

Focused:

- `cargo test -p fragile-clang parser_output_codegen`

Full regression gates:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Programmatic handoff usage:

```rust
use fragile_clang::{transpile_parser_output_to_rust, ParserOutputCodegenOptions};

let rust = transpile_parser_output_to_rust(&parser_output)?;
```

Optional timing trace:

```rust
let rust = fragile_clang::transpile_parser_output_to_rust_with_options(
    &parser_output,
    &ParserOutputCodegenOptions {
        ignored_error_patterns: Vec::new(),
        stage_timing_trace_path: Some("/tmp/trace.txt".into()),
    },
)?;
```
