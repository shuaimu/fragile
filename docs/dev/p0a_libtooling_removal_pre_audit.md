# P0.a Pre-Removal Code Audit: LibTooling Parser Path

Date: 2026-03-21
Status: Complete
Gate: P0.b hard removal cutover on/after 2026-04-18

## Summary

This audit catalogs every production-path reference to the legacy LibTooling parser
backend across the codebase. All sites documented here must be removed or migrated
as part of P0.b (hard removal cutover).

## Audit Categories

### 1. Strict-Path Backend Selection (Production Drivers)

**Files:** `crates/fragile-driver/src/lib.rs`, `crates/fragile-cli/src/bin/fragilec.rs`

Both files contain duplicate structures for backend selection:

| Site | Description | Removal Action |
|------|-------------|----------------|
| `enum StrictParserBackend` | Has `Libtooling` variant | Remove variant, simplify to ParserCore only |
| `enum ParserCoreCodegenEscapeHatch` | Has `Libtooling` variant | Remove entire enum |
| `parse_parser_backend_value()` | Accepts "libtooling" input | Remove libtooling branch |
| `strict_parser_backend_label()` | Maps Libtooling to "libtooling" | Remove Libtooling arm |
| `supported_parser_backend_values_message()` | Includes "libtooling" | Remove from message |
| `use_libtooling_codegen_escape_hatch` variable | Controls legacy codegen routing | Remove variable and branch |

### 2. Parser Invocation Sites

**File:** `crates/fragile-clang/src/lib.rs`

| Site | Description | Removal Action |
|------|-------------|----------------|
| `parse_libtooling_context()` | Core LibTooling parse invocation | Remove function |
| `translation_unit_from_libtooling_context()` | Converts LibTooling AST to ClangNode | Remove function |
| `apply_libtooling_enrichment()` | Enriches codegen with LibTooling bodies | Remove function |
| `libtooling_parser_for_path()` | Constructs LibToolingParser with options | Remove function |
| `transpile_cpp_to_rust_with_options()` | Routes ALL backends through LibTooling | Migrate to parser-output handoff or remove |
| `transpile_cpp_to_rust_with_libtooling()` | Public API for direct LibTooling transpile | Remove function |
| `generate_stubs()` | Uses parse_libtooling_context | Migrate or remove |
| `ParserBackend::Libtooling` variant | Enum variant in public API | Remove variant |
| `ParserBackend::Libclang` variant | Legacy alias that routes to LibTooling | Remove variant |
| `ParserBackend::Hybrid` variant | Legacy alias that routes to LibTooling | Remove variant |

### 3. Escape-Hatch Support Infrastructure

**File:** `crates/fragile-driver/src/lib.rs`

| Site | Description | Removal Action |
|------|-------------|----------------|
| `FRAGILEC_PARSER_BACKEND_ENV` | Env var constant | Remove |
| `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV` | Env var constant | Remove |
| `FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV` | Env var constant | Remove |
| `ESCAPE_HATCH_HARDENING_EXPIRY` | Expiry date constant | Remove |
| `escape_hatch_hardening_expired()` | Checks if past expiry | Remove |
| `escape_hatch_hardening_expired_as_of()` | Testable expiry check | Remove |
| `emit_escape_hatch_deprecation_warning()` | Stderr warning emitter | Remove |
| `log_escape_hatch_usage()` | File-based usage logging | Remove |
| `enforce_escape_hatch_policy()` | Policy enforcement gate | Remove |
| `parse_escape_hatch_log()` | Log file parser | Remove |
| `generate_escape_hatch_usage_report()` | Report generator | Remove |
| `assert_escape_hatch_trending_to_zero()` | Trending gate | Remove |
| `EscapeHatchUsageReport` struct | Report data structure | Remove |
| `EscapeHatchLogEntry` struct | Log entry data structure | Remove |

### 4. CLI --use-libtooling Flag

**File:** `crates/fragile-cli/src/main.rs`

| Site | Description | Removal Action |
|------|-------------|----------------|
| `#[arg(long)] use_libtooling: bool` | CLI flag definition | Remove flag |
| `libtooling_results` HashMap | Pre-parse results storage | Remove variable and loop |
| `libtooling_field_types` HashMap | Field type data | Remove variable and loop |
| `set_libtooling_bodies()` call | Enrichment injection | Remove call |
| LibToolingParser import and usage | Direct parser usage | Remove imports |

### 5. LibTooling Module

**File:** `crates/fragile-clang/src/libtooling.rs`

The entire module (2500+ lines) implements the LibTooling AST exporter bridge. After
removal, none of the following public exports will exist:

- `LibToolingParser`
- `convert_to_clang_node`
- `extract_method_bodies`
- `extract_method_bodies_with_params`
- `extract_specialization_field_types`
- `extract_specialization_method_signatures`
- `MethodInfo`
- `MethodSignature`
- `SpecializationFieldInfo`
- `TemplateMethodInstantiation`

**Removal action:** Delete `libtooling.rs`, remove `mod libtooling;` declaration,
remove `pub use libtooling::{...}` re-exports from `lib.rs`.

### 6. AstCodeGen LibTooling State

**File:** `crates/fragile-clang/src/ast_codegen.rs`

| Site | Description | Removal Action |
|------|-------------|----------------|
| `libtooling_method_bodies` field | Stores LibTooling method bodies | Remove field |
| `specialization_field_types` field | LibTooling-resolved field types | Remove field |
| `specialization_methods` field | LibTooling-resolved method signatures | Remove field |
| `set_libtooling_bodies()` method | Sets method bodies from LibTooling | Remove method |
| `set_specialization_field_types()` method | Sets field types | Remove method |
| `set_specialization_method_signatures()` method | Sets method sigs | Remove method |
| `should_rollback_libtooling()` validator | Validates LibTooling-generated code | Remove function |

### 7. CI Workflows

**Directory:** `.github/workflows/`

All 6 workflow files are clean -- no references to `FRAGILEC_PARSER_BACKEND`,
`FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH`, or `libtooling`.

No action needed.

### 8. Examples

| File | Description | Removal Action |
|------|-------------|----------------|
| `examples/debug_libtooling.rs` | Debug example using LibToolingParser | Remove file |

### 9. Scripts

| File | Description | Removal Action |
|------|-------------|----------------|
| `scripts/escape_hatch_usage_report.py` | Escape hatch telemetry tool | Remove file |

### 10. Test Files with LibTooling References

These test files reference LibTooling but are test/validation code, not production paths.
They will need updates after P0.b removal:

| File | Reference Type |
|------|---------------|
| `crates/fragile-clang/tests/m8_cutover_tests.rs` | Tests for escape hatch, backend selection |
| `crates/fragile-clang/tests/m7_shadow_mode_tests.rs` | Shadow mode parity tests |
| `crates/fragile-clang/tests/parser_backend_parity_tests.rs` | Backend parity tests |
| `crates/fragile-clang/tests/real_world_rapidjson_tests.rs` | RapidJSON harness |
| `crates/fragile-clang/tests/real_world_yamlcpp_tests.rs` | YAML-CPP harness |
| `crates/fragile-clang/tests/runtime_correctness_tests.rs` | Runtime tests |
| `crates/fragile-ast-exporter/tests/integration_test.rs` | AST exporter tests |
| `tests/python/test_mako_rpc_strict_baseline.py` | RPC baseline tests |

### 11. fragile-ast-exporter Crate

**File:** `crates/fragile-ast-exporter/src/lib.rs`

The ast-exporter crate provides the CBOR AST export binary that LibTooling invokes.
After LibTooling removal, assess whether this crate is still needed (it may be used
by the `fragile-parser-clang` backend independently).

## Regression Test Gate

All audit assertions are captured in:
`crates/fragile-clang/tests/p0_libtooling_removal_audit_tests.rs`

These tests will intentionally break when P0.b removal is executed, providing a
deterministic checklist of what has been addressed.

## Total Cataloged Sites: 42

- Production driver sites: 16 (fragile-driver + fragilec)
- Parser invocation sites: 10 (fragile-clang/src/lib.rs)
- Escape-hatch infrastructure: 14 (fragile-driver)
- CLI flag sites: 5 (fragile-cli/src/main.rs)
- AstCodeGen state: 7 (ast_codegen.rs)
- Module: 1 (libtooling.rs -- entire file)
- Examples: 1 (debug_libtooling.rs)
- Scripts: 1 (escape_hatch_usage_report.py)
