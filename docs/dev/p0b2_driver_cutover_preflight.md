# P0.b.2 Driver Cutover Preflight

Date: 2026-03-21  
Status: pre-cutover preparation complete; destructive removal steps are blocked until 2026-04-18.

## Purpose

`P0.b.2` removes strict-path backend/escape-hatch selection from production drivers.
This preflight captures exact touch points and patch order so execution can be done in
small, auditable slices on cutover day.

## Wrong-Approach Guard

Checked against `docs/fragile-dev-book.md` section `1.3` and `docs/dev/wrong.md`.

- No target-specific behavior changes (`mako`, `rpcbench`, `test_rpc`).
- No native bypass (`FRAGILEC_FORCE_NATIVE_SOURCES`).
- No fake semantic stub insertion to force green tests.

## Primary Files

1. `crates/fragile-driver/src/lib.rs`
2. `crates/fragile-cli/src/bin/fragilec.rs`

## Symbol Inventory for Removal

### `crates/fragile-driver/src/lib.rs`

- `StrictParserBackend::Libtooling`
- `ParserCoreCodegenEscapeHatch::Libtooling`
- `parse_parser_backend_value` libtooling branch
- `strict_parser_backend_label` libtooling label arm
- supported-backend help/value message containing `libtooling`
- `use_libtooling_codegen_escape_hatch` variable and dependent routing branch
- `ClangParserBackend::Libtooling` fallthrough assignment
- `FRAGILEC_PARSER_BACKEND_ENV`
- `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV`
- `FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV`
- `ESCAPE_HATCH_HARDENING_EXPIRY`
- `escape_hatch_hardening_expired`
- `escape_hatch_hardening_expired_as_of`
- `emit_escape_hatch_deprecation_warning`
- `log_escape_hatch_usage`
- `enforce_escape_hatch_policy`
- `parse_escape_hatch_log`
- `generate_escape_hatch_usage_report`
- `assert_escape_hatch_trending_to_zero`

### `crates/fragile-cli/src/bin/fragilec.rs`

- `StrictParserBackend::Libtooling`
- `ParserCoreCodegenEscapeHatch::Libtooling`
- backend parsing branch accepting `libtooling`
- `use_libtooling_codegen_escape_hatch` variable and dependent routing branch
- `ClangParserBackend::Libtooling` fallthrough assignment
- help text mentioning `FRAGILEC_PARSER_BACKEND=...libtooling`
- help text mentioning `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling`

## Patch Slices (bounded)

### `P0.b.2.b`

- Remove enum variants (`StrictParserBackend::Libtooling`, `ParserCoreCodegenEscapeHatch::Libtooling`) from both files.
- Compile-fix match arms to parser-core-only model.

### `P0.b.2.c`

- Remove `libtooling` parse branches and backend label/help message entries.

### `P0.b.2.d`

- Remove escape-hatch env constants and policy/logging/reporting utilities from `fragile-driver`.

### `P0.b.2.e`

- Remove `use_libtooling_codegen_escape_hatch` and any `ClangParserBackend::Libtooling` fallthrough route.

### `P0.b.2.f`

- Update `fragilec` help output and tests to remove deprecated backend/escape-hatch language.

## Validation Checkpoints

After each slice:

- `cargo test -p fragile-driver --all-targets`
- `cargo test -p fragile-cli --bin fragilec`
- `cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests -- --nocapture`

After finishing `P0.b.2.f`:

- `cargo test --workspace --all-targets`
- `python3 -m unittest discover -s tests/python -p 'test_*.py'`
