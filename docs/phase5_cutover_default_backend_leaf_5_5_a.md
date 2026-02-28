# Phase 5.5.a: Strict Default Backend Cutover

Date: 2026-02-28

## Goal

Switch strict `fragilec` compile path default parser backend from `libclang` to
`libtooling` now that Phase 5.1-5.4 backend-parity gates are closed, while
keeping explicit backend override support (`FRAGILEC_PARSER_BACKEND=libclang|hybrid`)
as the hardening-window escape hatch.

## Changes

1. `crates/fragile-cli/src/bin/fragilec.rs`
   - `strict_parser_backend_from_value(None|empty)` now returns `ParserBackend::Libtooling`.
   - `--fragilec-help` environment docs now state:
     - `FRAGILEC_PARSER_BACKEND ... (default: libtooling)`.
   - Updated strict CLI parser-backend regression:
     - defaults (`None`, empty) now assert `Libtooling`
     - added explicit override assertion for trimmed `libclang`.

## Validation

- Focused regression:
  - `cargo test -p fragile-cli --bin fragilec strict_parser_backend_validation_accepts_supported_values -- --nocapture`
- Full suite:
  - `cargo test`

Both passed in this iteration.
