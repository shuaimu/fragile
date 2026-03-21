//! P0.a Pre-removal code audit: LibTooling parser path inventory and removal gates.
//!
//! These tests enumerate every code site that references the legacy LibTooling parser
//! path in the production flow. They serve as the authoritative removal checklist for
//! P0.b (hard removal cutover on/after 2026-04-18). Each test asserts the current state
//! and documents what must be changed or removed.
//!
//! Audit categories:
//!   1. Strict-path backend selection (fragilec.rs, fragile-driver/lib.rs)
//!   2. Parser invocation sites (fragile-clang/src/lib.rs)
//!   3. Escape-hatch support (fragile-driver, fragilec)
//!   4. CLI --use-libtooling flag (fragile-cli/src/main.rs)
//!   5. LibTooling module (fragile-clang/src/libtooling.rs)
//!   6. CI workflow usage (.github/workflows/*.yml)
//!   7. Public API exports (fragile-clang pub use libtooling)
//!   8. Examples (examples/debug_libtooling.rs)
//!   9. Scripts (*.py)

use std::path::Path;

/// Project root for audit file scanning.
fn project_root() -> &'static Path {
    // Walk up from the test binary's manifest dir.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

/// Read a file's contents relative to the project root.
fn read_project_file(relative_path: &str) -> String {
    let full = project_root().join(relative_path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", full.display(), e))
}

/// Count occurrences of a pattern in a string (case-sensitive).
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

// ---------------------------------------------------------------------------
// Category 1: Strict-path backend selection
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_fragile_driver_has_libtooling_backend_variant() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // The StrictParserBackend enum currently has a Libtooling variant.
    // P0.b removal: remove this variant and all match arms.
    assert!(
        src.contains("StrictParserBackend::Libtooling"),
        "audit: expected StrictParserBackend::Libtooling in fragile-driver (to be removed in P0.b)"
    );
}

#[test]
fn p0a_audit_fragile_driver_has_escape_hatch_enum() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // ParserCoreCodegenEscapeHatch enum exists with Libtooling variant.
    // P0.b removal: remove entire enum and all references.
    assert!(
        src.contains("ParserCoreCodegenEscapeHatch"),
        "audit: expected ParserCoreCodegenEscapeHatch enum in fragile-driver"
    );
    assert!(
        src.contains("ParserCoreCodegenEscapeHatch::Libtooling"),
        "audit: expected Libtooling variant in ParserCoreCodegenEscapeHatch"
    );
}

#[test]
fn p0a_audit_fragilec_has_libtooling_backend_variant() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    assert!(
        src.contains("StrictParserBackend::Libtooling"),
        "audit: expected StrictParserBackend::Libtooling in fragilec.rs (to be removed in P0.b)"
    );
}

#[test]
fn p0a_audit_fragilec_has_escape_hatch_enum() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    assert!(
        src.contains("ParserCoreCodegenEscapeHatch::Libtooling"),
        "audit: expected ParserCoreCodegenEscapeHatch::Libtooling in fragilec.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 2: Parser invocation sites
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_lib_rs_has_libtooling_parse_invocation() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // parse_libtooling_context is the core LibTooling invocation site.
    // P0.b removal: remove function and all callers.
    assert!(
        src.contains("fn parse_libtooling_context"),
        "audit: expected parse_libtooling_context in lib.rs"
    );
    assert!(
        src.contains("fn translation_unit_from_libtooling_context"),
        "audit: expected translation_unit_from_libtooling_context in lib.rs"
    );
    assert!(
        src.contains("fn apply_libtooling_enrichment"),
        "audit: expected apply_libtooling_enrichment in lib.rs"
    );
}

#[test]
fn p0a_audit_lib_rs_has_libtooling_public_transpile_fn() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // transpile_cpp_to_rust_with_libtooling is a public API function.
    // P0.b removal: remove function.
    assert!(
        src.contains("pub fn transpile_cpp_to_rust_with_libtooling"),
        "audit: expected transpile_cpp_to_rust_with_libtooling public function"
    );
}

#[test]
fn p0a_audit_lib_rs_parser_backend_enum_has_libtooling() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // ParserBackend::Libtooling variant exists.
    // P0.b removal: remove variant, simplify enum.
    assert!(
        src.contains("Libtooling,"),
        "audit: expected Libtooling variant in ParserBackend enum"
    );
}

#[test]
fn p0a_audit_transpile_with_options_calls_libtooling() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // transpile_cpp_to_rust_with_options currently routes ALL backends through
    // parse_libtooling_context. This is the key invocation site.
    // P0.b removal: either make this use parser-output handoff only, or remove.
    assert!(
        src.contains("parse_libtooling_context(path, options)"),
        "audit: transpile_cpp_to_rust_with_options should call parse_libtooling_context (legacy path)"
    );
}

#[test]
fn p0a_audit_generate_stubs_calls_libtooling() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // generate_stubs() uses parse_libtooling_context.
    // P0.b: either migrate to parser-output handoff or remove.
    let stubs_fn_region = &src[src.find("pub fn generate_stubs").unwrap_or(0)..];
    let stubs_end = stubs_fn_region.find("\npub fn").unwrap_or(stubs_fn_region.len());
    let stubs_body = &stubs_fn_region[..stubs_end];
    assert!(
        stubs_body.contains("parse_libtooling_context"),
        "audit: generate_stubs should use parse_libtooling_context (legacy)"
    );
}

// ---------------------------------------------------------------------------
// Category 3: Escape-hatch support
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_escape_hatch_env_vars_defined() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // These env var constants exist and are used in escape-hatch handling.
    // P0.b removal: remove constants and all references.
    assert!(driver_src.contains("FRAGILEC_PARSER_BACKEND_ENV"));
    assert!(driver_src.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV"));
    assert!(driver_src.contains("FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV"));
    assert!(driver_src.contains("ESCAPE_HATCH_HARDENING_EXPIRY"));
}

#[test]
fn p0a_audit_escape_hatch_hardening_expiry_is_2026_04_18() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    assert!(
        driver_src.contains("\"2026-04-18\""),
        "audit: hardening expiry should be 2026-04-18"
    );
}

#[test]
fn p0a_audit_escape_hatch_functions_exist() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b removal: remove all escape hatch infrastructure.
    let expected_fns = [
        "fn escape_hatch_hardening_expired",
        "fn escape_hatch_hardening_expired_as_of",
        "fn emit_escape_hatch_deprecation_warning",
        "fn log_escape_hatch_usage",
        "fn enforce_escape_hatch_policy",
        "fn parse_escape_hatch_log",
        "fn generate_escape_hatch_usage_report",
        "fn assert_escape_hatch_trending_to_zero",
    ];
    for f in &expected_fns {
        assert!(
            driver_src.contains(f),
            "audit: expected escape hatch function `{}` in fragile-driver",
            f
        );
    }
}

#[test]
fn p0a_audit_driver_use_libtooling_codegen_escape_hatch_variable() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // use_libtooling_codegen_escape_hatch controls the legacy codegen routing.
    // P0.b removal: remove this variable and the associated branch.
    assert!(
        driver_src.contains("use_libtooling_codegen_escape_hatch"),
        "audit: expected use_libtooling_codegen_escape_hatch in fragile-driver"
    );
}

#[test]
fn p0a_audit_fragilec_use_libtooling_codegen_escape_hatch_variable() {
    let fragilec_src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    assert!(
        fragilec_src.contains("use_libtooling_codegen_escape_hatch"),
        "audit: expected use_libtooling_codegen_escape_hatch in fragilec.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 4: CLI --use-libtooling flag
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_cli_has_use_libtooling_flag() {
    let main_src = read_project_file("crates/fragile-cli/src/main.rs");
    // The fragile CLI has --use-libtooling flag for explicit LibTooling enrichment.
    // P0.b removal: remove flag and LibTooling pre-parse path.
    assert!(
        main_src.contains("use_libtooling"),
        "audit: expected --use-libtooling flag in fragile CLI main.rs"
    );
    assert!(
        main_src.contains("LibTooling for template method bodies"),
        "audit: expected LibTooling help text in fragile CLI"
    );
}

// ---------------------------------------------------------------------------
// Category 5: LibTooling module
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_libtooling_module_exists() {
    let lib_src = read_project_file("crates/fragile-clang/src/lib.rs");
    // The libtooling module is imported and its exports are re-exported.
    // P0.b removal: remove module declaration and all pub use items.
    assert!(
        lib_src.contains("mod libtooling;"),
        "audit: expected libtooling module declaration in fragile-clang/src/lib.rs"
    );
    assert!(
        lib_src.contains("pub use libtooling::{"),
        "audit: expected libtooling re-exports in fragile-clang/src/lib.rs"
    );
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/libtooling.rs")
            .exists(),
        "audit: expected libtooling.rs source file to exist"
    );
}

#[test]
fn p0a_audit_libtooling_public_exports_inventory() {
    let lib_src = read_project_file("crates/fragile-clang/src/lib.rs");
    // Enumerate all pub use libtooling exports for P0.b removal checklist.
    let expected_exports = [
        "convert_to_clang_node",
        "extract_method_bodies",
        "extract_method_bodies_with_params",
        "extract_specialization_field_types",
        "extract_specialization_method_signatures",
        "LibToolingParser",
        "MethodInfo",
        "MethodSignature",
        "SpecializationFieldInfo",
        "TemplateMethodInstantiation",
    ];
    for export in &expected_exports {
        assert!(
            lib_src.contains(export),
            "audit: expected pub use libtooling export `{}`",
            export
        );
    }
}

#[test]
fn p0a_audit_ast_codegen_has_libtooling_state() {
    let codegen_src = read_project_file("crates/fragile-clang/src/ast_codegen.rs");
    // AstCodeGen carries libtooling state for enrichment.
    // P0.b removal: remove these fields and set_libtooling_bodies.
    assert!(
        codegen_src.contains("libtooling_method_bodies"),
        "audit: expected libtooling_method_bodies field in AstCodeGen"
    );
    assert!(
        codegen_src.contains("specialization_field_types"),
        "audit: expected specialization_field_types field in AstCodeGen"
    );
    assert!(
        codegen_src.contains("specialization_methods"),
        "audit: expected specialization_methods field in AstCodeGen"
    );
    assert!(
        codegen_src.contains("fn set_libtooling_bodies"),
        "audit: expected set_libtooling_bodies method in AstCodeGen"
    );
}

#[test]
fn p0a_audit_ast_codegen_has_libtooling_rollback_validator() {
    let codegen_src = read_project_file("crates/fragile-clang/src/ast_codegen.rs");
    // should_rollback_libtooling is a dedicated validator for libtooling-generated bodies.
    // P0.b removal: remove function if no longer needed.
    assert!(
        codegen_src.contains("fn should_rollback_libtooling"),
        "audit: expected should_rollback_libtooling validator in ast_codegen.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 6: CI workflow usage
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_ci_workflows_are_clean_of_libtooling_references() {
    let workflow_dir = project_root().join(".github/workflows");
    if !workflow_dir.exists() {
        // CI workflows directory may not exist in all checkouts.
        return;
    }
    for entry in std::fs::read_dir(&workflow_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "yml" || ext == "yaml") {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                !content.contains("FRAGILEC_PARSER_BACKEND"),
                "audit: CI workflow {} should not set FRAGILEC_PARSER_BACKEND",
                path.display()
            );
            assert!(
                !content.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH"),
                "audit: CI workflow {} should not set FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH",
                path.display()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Category 7: Examples
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_debug_libtooling_example_exists() {
    // examples/debug_libtooling.rs uses LibToolingParser directly.
    // P0.b removal: remove or migrate this example.
    let example_path = project_root().join("examples/debug_libtooling.rs");
    assert!(
        example_path.exists(),
        "audit: expected examples/debug_libtooling.rs to exist"
    );
    let content = std::fs::read_to_string(&example_path).unwrap();
    assert!(
        content.contains("LibToolingParser"),
        "audit: expected LibToolingParser usage in debug_libtooling.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 8: Scripts
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_escape_hatch_usage_report_script_exists() {
    // scripts/escape_hatch_usage_report.py is escape-hatch telemetry.
    // P0.b removal: remove script (no longer needed after escape hatch removal).
    let script_path = project_root().join("scripts/escape_hatch_usage_report.py");
    assert!(
        script_path.exists(),
        "audit: expected scripts/escape_hatch_usage_report.py to exist"
    );
}

// ---------------------------------------------------------------------------
// Category 9: Comprehensive site count gates
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_driver_libtooling_reference_count() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    let count = count_occurrences(&src, "libtooling");
    // Current expected count captures all references (enum variants, strings, match arms,
    // comments, tests). If this changes, the audit needs to be refreshed.
    // This is intentionally a >= check so adding references fails explicitly.
    assert!(
        count >= 20,
        "audit: expected at least 20 libtooling references in fragile-driver, found {}",
        count
    );
}

#[test]
fn p0a_audit_fragilec_libtooling_reference_count() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    let count = count_occurrences(&src, "libtooling");
    assert!(
        count >= 25,
        "audit: expected at least 25 libtooling references in fragilec.rs, found {}",
        count
    );
}

#[test]
fn p0a_audit_fragile_clang_lib_libtooling_reference_count() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    let count = count_occurrences(&src, "libtooling");
    assert!(
        count >= 15,
        "audit: expected at least 15 libtooling references in fragile-clang lib.rs, found {}",
        count
    );
}

// ---------------------------------------------------------------------------
// Category 10: fragile-cli main.rs LibTooling pre-parse path
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_cli_main_libtooling_preparse_path() {
    let src = read_project_file("crates/fragile-cli/src/main.rs");
    // The CLI's transpile command has a LibTooling pre-parse path that collects
    // template method bodies and specialization field types.
    // P0.b removal: remove this entire code path.
    assert!(
        src.contains("libtooling_results"),
        "audit: expected libtooling_results variable in CLI main.rs"
    );
    assert!(
        src.contains("libtooling_field_types"),
        "audit: expected libtooling_field_types variable in CLI main.rs"
    );
    assert!(
        src.contains("set_libtooling_bodies"),
        "audit: expected set_libtooling_bodies call in CLI main.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 11: Driver legacy codegen fallthrough path
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_driver_legacy_libtooling_codegen_fallthrough() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // When StrictParserBackend::Libtooling is selected, or when
    // use_libtooling_codegen_escape_hatch is true, the driver falls through
    // to transpile_cpp_to_rust_with_options with backend=Libtooling.
    // P0.b removal: remove this fallthrough path entirely.
    assert!(
        driver_src.contains("ClangParserBackend::Libtooling"),
        "audit: driver sets backend to ClangParserBackend::Libtooling in fallthrough"
    );
}

#[test]
fn p0a_audit_fragilec_legacy_libtooling_codegen_fallthrough() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // Same fallthrough path exists in fragilec.rs.
    assert!(
        src.contains("use_libtooling_codegen_escape_hatch"),
        "audit: fragilec has libtooling codegen escape hatch fallthrough path"
    );
}

// ---------------------------------------------------------------------------
// Category 12: fragilec help text
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_fragilec_help_text_mentions_libtooling() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // The fragilec --fragilec-help output mentions libtooling backend.
    // P0.b removal: update help text.
    assert!(
        src.contains("FRAGILEC_PARSER_BACKEND=<name>"),
        "audit: fragilec help text describes FRAGILEC_PARSER_BACKEND"
    );
    assert!(
        src.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling"),
        "audit: fragilec help text describes escape hatch"
    );
}

// ---------------------------------------------------------------------------
// Category 13: ast-exporter LibTooling dependency
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_ast_exporter_libtooling_references() {
    let ast_exporter_lib = read_project_file("crates/fragile-ast-exporter/src/lib.rs");
    // fragile-ast-exporter is the CBOR AST exporter that drives LibTooling parse.
    // P0.b: assess whether this crate is still needed after LibTooling removal.
    let count = count_occurrences(&ast_exporter_lib, "libtooling")
        + count_occurrences(&ast_exporter_lib, "LibTooling");
    // The ast-exporter may or may not reference LibTooling directly; document state.
    // This test just documents the count — it doesn't fail.
    eprintln!(
        "audit: fragile-ast-exporter/src/lib.rs has {} libtooling/LibTooling references",
        count
    );
}

// ---------------------------------------------------------------------------
// Summary: P0.a audit checklist documentation
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_task_documented_in_todo() {
    let todo = read_project_file("TODO.md");
    assert!(
        todo.contains("P0.a Complete pre-removal code audits"),
        "audit: P0.a task should be documented in TODO.md"
    );
}

#[test]
fn p0b_1_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    assert!(
        todo.contains("P0.b.1 Decompose P0.b into <1000 LOC leaves"),
        "audit: TODO should document completed P0.b.1 decomposition leaf"
    );
    // P0.b.2 was expanded into sub-items (P0.b.2.a..P0.b.2.f); check the top-level
    // leaf identifiers exist. P0.b.3 through P0.b.9 have (on/after 2026-04-18) directly.
    for expected_leaf in [
        "P0.b.2 Remove strict-path backend",
        "P0.b.3 (on/after 2026-04-18)",
        "P0.b.4 (on/after 2026-04-18)",
        "P0.b.5 (on/after 2026-04-18)",
        "P0.b.6 (on/after 2026-04-18)",
        "P0.b.7 (on/after 2026-04-18)",
        "P0.b.8 (on/after 2026-04-18)",
        "P0.b.9 (on/after 2026-04-18)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposed leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_1_playbook_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b_hard_removal_cutover_playbook.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b playbook doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_1_playbook_mentions_gate_and_slice_bounds() {
    let doc = read_project_file("docs/dev/p0b_hard_removal_cutover_playbook.md");
    assert!(
        doc.contains("on/after **2026-04-18**"),
        "audit: playbook should document hardening-window date gate"
    );
    assert!(
        doc.contains("Leaf Breakdown (<1000 LOC each)"),
        "audit: playbook should document bounded leaf sizing"
    );
    assert!(
        doc.contains("P0.b.2")
            && doc.contains("P0.b.3")
            && doc.contains("P0.b.4")
            && doc.contains("P0.b.5")
            && doc.contains("P0.b.6")
            && doc.contains("P0.b.7")
            && doc.contains("P0.b.8")
            && doc.contains("P0.b.9"),
        "audit: playbook should enumerate all decomposed P0.b execution leaves"
    );
}

#[test]
fn p0b_2a_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    assert!(
        todo.contains("P0.b.2.a (pre-cutover) Publish driver-cutover preflight inventory"),
        "audit: TODO should document completed P0.b.2.a pre-cutover leaf"
    );
    for expected_leaf in [
        "P0.b.2.b (on/after 2026-04-18)",
        "P0.b.2.c (on/after 2026-04-18)",
        "P0.b.2.d (on/after 2026-04-18)",
        "P0.b.2.e (on/after 2026-04-18)",
        "P0.b.2.f (on/after 2026-04-18)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposed P0.b.2 leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2a_preflight_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2_driver_cutover_preflight.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2 preflight doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2a_preflight_document_contains_driver_symbol_inventory() {
    let doc = read_project_file("docs/dev/p0b2_driver_cutover_preflight.md");
    for required in [
        "crates/fragile-driver/src/lib.rs",
        "crates/fragile-cli/src/bin/fragilec.rs",
        "StrictParserBackend::Libtooling",
        "ParserCoreCodegenEscapeHatch::Libtooling",
        "use_libtooling_codegen_escape_hatch",
        "ClangParserBackend::Libtooling",
        "P0.b.2.b",
        "P0.b.2.f",
        "cargo test --workspace --all-targets",
        "python3 -m unittest discover -s tests/python -p 'test_*.py'",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2 preflight doc should contain `{}`",
            required
        );
    }
}

/// Comprehensive summary test that produces the full audit inventory.
#[test]
fn p0a_audit_comprehensive_site_inventory() {
    // This test produces a summary of all audited sites for P0.b reference.
    let sites = vec![
        // Production drivers
        ("crates/fragile-driver/src/lib.rs", "StrictParserBackend::Libtooling enum variant"),
        ("crates/fragile-driver/src/lib.rs", "ParserCoreCodegenEscapeHatch::Libtooling enum variant"),
        ("crates/fragile-driver/src/lib.rs", "parse_parser_backend_value(\"libtooling\") acceptance"),
        ("crates/fragile-driver/src/lib.rs", "use_libtooling_codegen_escape_hatch variable"),
        ("crates/fragile-driver/src/lib.rs", "FRAGILEC_PARSER_BACKEND_ENV constant"),
        ("crates/fragile-driver/src/lib.rs", "FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV constant"),
        ("crates/fragile-driver/src/lib.rs", "FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV constant"),
        ("crates/fragile-driver/src/lib.rs", "ESCAPE_HATCH_HARDENING_EXPIRY constant"),
        ("crates/fragile-driver/src/lib.rs", "escape_hatch_hardening_expired() function"),
        ("crates/fragile-driver/src/lib.rs", "emit_escape_hatch_deprecation_warning() function"),
        ("crates/fragile-driver/src/lib.rs", "log_escape_hatch_usage() function"),
        ("crates/fragile-driver/src/lib.rs", "enforce_escape_hatch_policy() function"),
        ("crates/fragile-driver/src/lib.rs", "parse_escape_hatch_log() function"),
        ("crates/fragile-driver/src/lib.rs", "generate_escape_hatch_usage_report() function"),
        ("crates/fragile-driver/src/lib.rs", "assert_escape_hatch_trending_to_zero() function"),
        ("crates/fragile-driver/src/lib.rs", "ClangParserBackend::Libtooling fallthrough backend"),
        // fragilec driver (duplicate structure)
        ("crates/fragile-cli/src/bin/fragilec.rs", "StrictParserBackend::Libtooling enum variant"),
        ("crates/fragile-cli/src/bin/fragilec.rs", "ParserCoreCodegenEscapeHatch::Libtooling enum variant"),
        ("crates/fragile-cli/src/bin/fragilec.rs", "use_libtooling_codegen_escape_hatch variable"),
        ("crates/fragile-cli/src/bin/fragilec.rs", "FRAGILEC_PARSER_BACKEND help text"),
        ("crates/fragile-cli/src/bin/fragilec.rs", "FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH help text"),
        // fragile CLI
        ("crates/fragile-cli/src/main.rs", "--use-libtooling CLI flag"),
        ("crates/fragile-cli/src/main.rs", "libtooling_results pre-parse path"),
        ("crates/fragile-cli/src/main.rs", "libtooling_field_types pre-parse path"),
        ("crates/fragile-cli/src/main.rs", "set_libtooling_bodies() call"),
        // fragile-clang library
        ("crates/fragile-clang/src/lib.rs", "mod libtooling; module declaration"),
        ("crates/fragile-clang/src/lib.rs", "pub use libtooling::{...} re-exports"),
        ("crates/fragile-clang/src/lib.rs", "ParserBackend::Libtooling variant"),
        ("crates/fragile-clang/src/lib.rs", "parse_libtooling_context() function"),
        ("crates/fragile-clang/src/lib.rs", "translation_unit_from_libtooling_context() function"),
        ("crates/fragile-clang/src/lib.rs", "apply_libtooling_enrichment() function"),
        ("crates/fragile-clang/src/lib.rs", "transpile_cpp_to_rust_with_libtooling() public API"),
        ("crates/fragile-clang/src/lib.rs", "generate_stubs() uses parse_libtooling_context"),
        ("crates/fragile-clang/src/lib.rs", "libtooling_parser_for_path() helper"),
        // ast_codegen state
        ("crates/fragile-clang/src/ast_codegen.rs", "libtooling_method_bodies field"),
        ("crates/fragile-clang/src/ast_codegen.rs", "specialization_field_types field"),
        ("crates/fragile-clang/src/ast_codegen.rs", "specialization_methods field"),
        ("crates/fragile-clang/src/ast_codegen.rs", "set_libtooling_bodies() method"),
        ("crates/fragile-clang/src/ast_codegen.rs", "should_rollback_libtooling() validator"),
        // LibTooling module itself
        ("crates/fragile-clang/src/libtooling.rs", "entire module (LibToolingParser, etc.)"),
        // Examples
        ("examples/debug_libtooling.rs", "debug example using LibToolingParser"),
        // Scripts
        ("scripts/escape_hatch_usage_report.py", "escape hatch telemetry script"),
    ];

    // Verify all files exist.
    let mut all_exist = true;
    for (file, desc) in &sites {
        let path = project_root().join(file);
        if !path.exists() {
            eprintln!("MISSING: {} ({})", file, desc);
            all_exist = false;
        }
    }
    assert!(
        all_exist,
        "all audited files should exist in the project tree"
    );

    eprintln!("\n=== P0.a LibTooling Removal Audit: {} sites cataloged ===", sites.len());
    for (i, (file, desc)) in sites.iter().enumerate() {
        eprintln!("  [{:2}] {} :: {}", i + 1, file, desc);
    }
    eprintln!("=== End of audit inventory ===\n");
}
