# M2.A2.3 Strict Compile Parser-Core Output Handoff (2026-03-17)

## Objective

Close TODO leaf `M2.A2.3` by routing strict compile active parser stages through
parser-core output handoff, while keeping a temporary explicit escape hatch for
hardening.

## Scope and Sizing

This change is under 1000 LOC and limited to:

- `crates/fragile-cli/src/bin/fragilec.rs`
- `crates/fragile-driver/src/lib.rs`
- `TODO.md`
- docs updates

No additional TODO decomposition is required for this leaf.

## Wrong-Approach Check

Reviewed before implementation:

- `docs/fragile-dev-book.md` section `1.3 Wrong Approaches (Do Not Do)`
- `docs/dev/wrong.md`

Guardrails followed:

- no fake/synthesized semantic bodies
- no target-specific hacks
- no native-force bypasses
- no silent fallback behavior

## Design

Strict compile behavior in both CLI and driver now does:

1. parser-core parse (`fragile-parser-clang` backend via registry)
2. parser manifest write (existing contract remains)
3. parser-output handoff codegen via
   `fragile_clang::transpile_parser_output_to_rust_with_options`
4. rustc object compile from transpiled Rust output

Temporary hardening escape hatch:

- env var: `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH`
- supported value: `libtooling`
- when set, parser-core parse still runs but codegen routes through legacy
  libtooling path
- unsupported values fail deterministically with a clear validation error

## Validation

Focused:

- `cargo test -p fragile-cli strict_compile_parser_core_backend_routes_through_parser_output_handoff`
- `cargo test -p fragile-cli parser_core_codegen_escape_hatch_validation`
- `cargo test -p fragile-driver parser_core_backend_routes_through_parser_output_handoff`
- `cargo test -p fragile-driver parser_core_codegen_escape_hatch_validation`

Full regression gates:

- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
- `cargo test --workspace --all-targets`

## User Manual

Default strict parser-core path (no escape hatch):

```bash
FRAGILEC_PARSER_BACKEND=fragile-parser-clang fragilec --strict-mode strict -c file.cpp -o file.o
```

Temporary hardening fallback to libtooling codegen:

```bash
FRAGILEC_PARSER_BACKEND=fragile-parser-clang \
FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling \
fragilec --strict-mode strict -c file.cpp -o file.o
```
