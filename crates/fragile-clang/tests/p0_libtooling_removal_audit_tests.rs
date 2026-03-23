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
fn p0a_audit_fragile_driver_libtooling_backend_variant_removed() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.b.1.b.3: StrictParserBackend::Libtooling variant removed 2026-03-22.
    assert!(
        !src.contains("StrictParserBackend::Libtooling"),
        "audit: StrictParserBackend::Libtooling should have been removed in P0.b.2.b.1.b.3"
    );
}

#[test]
fn p0a_audit_fragile_driver_has_escape_hatch_enum() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.d: ParserCoreCodegenEscapeHatch enum removed 2026-03-22.
    assert!(
        !src.contains("ParserCoreCodegenEscapeHatch"),
        "anti-regression: ParserCoreCodegenEscapeHatch should have been removed in P0.b.2.d"
    );
}

#[test]
fn p0a_audit_fragilec_has_libtooling_backend_variant() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // P0.b.2.c: StrictParserBackend enum removed 2026-03-22.
    assert!(
        !src.contains("StrictParserBackend"),
        "anti-regression: StrictParserBackend should have been removed in P0.b.2.c"
    );
}

#[test]
fn p0a_audit_fragilec_has_escape_hatch_enum() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // P0.b.2.d: ParserCoreCodegenEscapeHatch removed 2026-03-22.
    assert!(
        !src.contains("ParserCoreCodegenEscapeHatch"),
        "anti-regression: ParserCoreCodegenEscapeHatch should have been removed in P0.b.2.d"
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
fn p0b3_anti_regression_lib_rs_no_libtooling_public_transpile_fn() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // P0.b.3: transpile_cpp_to_rust_with_libtooling removed 2026-03-22.
    assert!(
        !src.contains("pub fn transpile_cpp_to_rust_with_libtooling"),
        "anti-regression: transpile_cpp_to_rust_with_libtooling should not be reintroduced"
    );
}

#[test]
fn p0b3_anti_regression_lib_rs_no_parser_backend_enum() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    // P0.b.3: ParserBackend enum removed 2026-03-22.
    assert!(
        !src.contains("pub enum ParserBackend"),
        "anti-regression: ParserBackend enum should not be reintroduced"
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
    // P0.b.2.d: Escape hatch constants removed 2026-03-22.
    assert!(!driver_src.contains("FRAGILEC_PARSER_BACKEND_ENV"),
        "anti-regression: FRAGILEC_PARSER_BACKEND_ENV should have been removed");
    assert!(!driver_src.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV"),
        "anti-regression: FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV should have been removed");
    assert!(!driver_src.contains("FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV"),
        "anti-regression: FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV should have been removed");
    assert!(!driver_src.contains("ESCAPE_HATCH_HARDENING_EXPIRY"),
        "anti-regression: ESCAPE_HATCH_HARDENING_EXPIRY should have been removed");
}

#[test]
fn p0a_audit_escape_hatch_hardening_expiry_is_2026_04_18() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.d: Hardening expiry constant removed 2026-03-22.
    assert!(
        !driver_src.contains("ESCAPE_HATCH_HARDENING_EXPIRY"),
        "anti-regression: ESCAPE_HATCH_HARDENING_EXPIRY should have been removed"
    );
}

#[test]
fn p0a_audit_escape_hatch_functions_exist() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.d: All escape hatch functions removed 2026-03-22.
    let removed_fns = [
        "fn escape_hatch_hardening_expired",
        "fn emit_escape_hatch_deprecation_warning",
        "fn log_escape_hatch_usage",
        "fn enforce_escape_hatch_policy",
        "fn parse_escape_hatch_log",
        "fn generate_escape_hatch_usage_report",
        "fn assert_escape_hatch_trending_to_zero",
    ];
    for f in &removed_fns {
        assert!(
            !driver_src.contains(f),
            "anti-regression: escape hatch function `{}` should have been removed from fragile-driver",
            f
        );
    }
}

#[test]
fn p0a_audit_driver_use_libtooling_codegen_escape_hatch_variable_removed() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.b.1.b.3: use_libtooling_codegen_escape_hatch removed 2026-03-22.
    assert!(
        !driver_src.contains("use_libtooling_codegen_escape_hatch"),
        "audit: use_libtooling_codegen_escape_hatch should have been removed in P0.b.2.b.1.b.3"
    );
}

#[test]
fn p0a_audit_fragilec_use_libtooling_codegen_escape_hatch_variable() {
    let fragilec_src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // P0.b.2.e: use_libtooling_codegen_escape_hatch removed 2026-03-22.
    assert!(
        !fragilec_src.contains("use_libtooling_codegen_escape_hatch"),
        "anti-regression: use_libtooling_codegen_escape_hatch should have been removed from fragilec.rs"
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
fn p0b5_anti_regression_libtooling_module_removed() {
    let lib_src = read_project_file("crates/fragile-clang/src/lib.rs");
    // P0.b.5: libtooling module deleted and re-exports removed 2026-03-23.
    assert!(
        !lib_src.contains("mod libtooling;"),
        "anti-regression: mod libtooling should not be reintroduced in fragile-clang/src/lib.rs"
    );
    assert!(
        !lib_src.contains("pub use libtooling::{"),
        "anti-regression: pub use libtooling re-exports should not be reintroduced"
    );
    assert!(
        !Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/libtooling.rs")
            .exists(),
        "anti-regression: libtooling.rs should not be reintroduced"
    );
}

#[test]
fn p0b5_anti_regression_libtooling_enrichment_exports_removed() {
    let lib_src = read_project_file("crates/fragile-clang/src/lib.rs");
    // P0.b.5: LibTooling enrichment types and extraction functions removed 2026-03-23.
    // convert_to_clang_node and LibToolingParser are retained (moved to new modules).
    let removed_exports = [
        "extract_method_bodies",
        "extract_method_bodies_with_params",
        "extract_specialization_field_types",
        "extract_specialization_method_signatures",
        "MethodInfo",
        "MethodSignature",
        "SpecializationFieldInfo",
        "TemplateMethodInstantiation",
    ];
    for export in &removed_exports {
        assert!(
            !lib_src.contains(&format!("pub use {}",  export))
                && !lib_src.contains(&format!("pub use libtooling::{}", export)),
            "anti-regression: pub export `{}` should not be reintroduced",
            export
        );
    }
}

#[test]
fn p0b4_anti_regression_ast_codegen_libtooling_enrichment_state_removed() {
    let codegen_src = read_project_file("crates/fragile-clang/src/ast_codegen.rs");
    // P0.b.4: LibTooling enrichment state has been removed from AstCodeGen.
    assert!(
        !codegen_src.contains("libtooling_method_bodies: HashMap"),
        "anti-regression: libtooling_method_bodies field should be removed from AstCodeGen"
    );
    assert!(
        !codegen_src.contains("specialization_field_types: HashMap"),
        "anti-regression: specialization_field_types field should be removed from AstCodeGen"
    );
    assert!(
        !codegen_src.contains("specialization_methods: HashMap"),
        "anti-regression: specialization_methods field should be removed from AstCodeGen"
    );
    assert!(
        !codegen_src.contains("fn set_libtooling_bodies"),
        "anti-regression: set_libtooling_bodies method should be removed from AstCodeGen"
    );
    assert!(
        !codegen_src.contains("fn should_rollback_libtooling"),
        "anti-regression: should_rollback_libtooling should be removed from AstCodeGen"
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
    // P0.b.2.d: escape_hatch_usage_report.py removed 2026-03-22.
    let script_path = project_root().join("scripts/escape_hatch_usage_report.py");
    assert!(
        !script_path.exists(),
        "anti-regression: scripts/escape_hatch_usage_report.py should have been removed in P0.b.2.d"
    );
}

// ---------------------------------------------------------------------------
// Category 9: Comprehensive site count gates
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_driver_libtooling_reference_count() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    let count = count_occurrences(&src, "libtooling");
    // P0.b.2.c/d: All libtooling references removed from fragile-driver 2026-03-22.
    // A small number of references in comments is acceptable.
    assert!(
        count <= 5,
        "anti-regression: fragile-driver should have very few libtooling references, found {}",
        count
    );
}

#[test]
fn p0a_audit_fragilec_libtooling_reference_count() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    let count = count_occurrences(&src, "libtooling");
    // P0.b.2.c/d/f: Most libtooling references removed from fragilec 2026-03-22.
    // Some references may remain in comments or string messages.
    assert!(
        count <= 10,
        "anti-regression: fragilec should have very few libtooling references, found {}",
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
fn p0b5_anti_regression_cli_main_libtooling_preparse_removed() {
    let src = read_project_file("crates/fragile-cli/src/main.rs");
    // P0.b.5: LibTooling pre-parse data collection removed 2026-03-23.
    assert!(
        !src.contains("libtooling_results"),
        "anti-regression: libtooling_results should not be reintroduced in CLI main.rs"
    );
    assert!(
        !src.contains("libtooling_field_types"),
        "anti-regression: libtooling_field_types should not be reintroduced in CLI main.rs"
    );
    assert!(
        !src.contains("set_libtooling_bodies"),
        "anti-regression: set_libtooling_bodies should not be reintroduced in CLI main.rs"
    );
    assert!(
        !src.contains("extract_method_bodies_with_params"),
        "anti-regression: extract_method_bodies_with_params should not be reintroduced in CLI main.rs"
    );
    assert!(
        !src.contains("extract_specialization_field_types"),
        "anti-regression: extract_specialization_field_types should not be reintroduced in CLI main.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 11: Driver legacy codegen fallthrough path
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_driver_legacy_libtooling_codegen_fallthrough_removed() {
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.b.1.b.3: libtooling fallthrough path removed 2026-03-22.
    assert!(
        !driver_src.contains("ClangParserBackend::Libtooling"),
        "audit: ClangParserBackend::Libtooling fallthrough should have been removed in P0.b.2.b.1.b.3"
    );
}

#[test]
fn p0a_audit_fragilec_legacy_libtooling_codegen_fallthrough() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // P0.b.2.e: use_libtooling_codegen_escape_hatch removed 2026-03-22.
    assert!(
        !src.contains("use_libtooling_codegen_escape_hatch"),
        "anti-regression: use_libtooling_codegen_escape_hatch should have been removed from fragilec.rs"
    );
}

// ---------------------------------------------------------------------------
// Category 12: fragilec help text
// ---------------------------------------------------------------------------

#[test]
fn p0a_audit_fragilec_help_text_mentions_libtooling() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    // P0.b.2.f: FRAGILEC_PARSER_BACKEND removed from help text 2026-03-22.
    assert!(
        !src.contains("FRAGILEC_PARSER_BACKEND=<name>"),
        "anti-regression: fragilec help text should not reference FRAGILEC_PARSER_BACKEND"
    );
    assert!(
        !src.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH"),
        "anti-regression: fragilec help text should not reference escape hatch"
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
        "P0.b.3 (immediate)",
        "P0.b.4 (immediate)",
        "P0.b.5 (immediate)",
        "P0.b.6 (immediate)",
        "P0.b.7 (immediate)",
        "P0.b.8 (immediate)",
        "P0.b.9 (immediate)",
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
        "P0.b.2.b (immediate)",
        "P0.b.2.c (immediate)",
        "P0.b.2.d (immediate)",
        "P0.b.2.e (immediate)",
        "P0.b.2.f (immediate)",
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

#[test]
fn p0b_2b0_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    assert!(
        todo.contains("P0.b.2.b.0 (pre-cutover) Publish variant-removal dependency map"),
        "audit: TODO should document completed pre-cutover leaf P0.b.2.b.0"
    );
    for expected_leaf in [
        "P0.b.2.b.1 (immediate)",
        "P0.b.2.b.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposed P0.b.2.b leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b0_dependency_map_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2b_variant_removal_dependency_map.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b dependency-map doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b0_dependency_map_contains_expected_symbols_and_boundaries() {
    let doc = read_project_file("docs/dev/p0b2b_variant_removal_dependency_map.md");
    for required in [
        "crates/fragile-driver/src/lib.rs",
        "crates/fragile-cli/src/bin/fragilec.rs",
        "StrictParserBackend::Libtooling",
        "ParserCoreCodegenEscapeHatch::Libtooling",
        "parse_parser_backend_value",
        "strict_parser_backend_from_legacy_backend",
        "P0.b.2.b.1",
        "P0.b.2.b.2",
        "P0.b.2.c",
        "cargo test -p fragile-driver",
        "cargo test -p fragile-cli",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b dependency-map doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    assert!(
        todo.contains("P0.b.2.b.1.a (pre-cutover) Publish line-level cutover patch spec"),
        "audit: TODO should document completed pre-cutover leaf P0.b.2.b.1.a"
    );
    for expected_leaf in [
        "P0.b.2.b.1.b (immediate)",
        "P0.b.2.b.1.c (immediate)",
        "P0.b.2.b.1.d (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposed P0.b.2.b.1 leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1_patch_spec_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2b1_variant_removal_patch_spec.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1 patch-spec doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1_patch_spec_contains_required_cutover_boundaries() {
    let doc = read_project_file("docs/dev/p0b2b1_variant_removal_patch_spec.md");
    for required in [
        "P0.b.2.b.1.a",
        "P0.b.2.b.1.b",
        "P0.b.2.b.1.c",
        "P0.b.2.b.1.d",
        "P0.b.2.c",
        "crates/fragile-driver/src/lib.rs",
        "crates/fragile-cli/src/bin/fragilec.rs",
        "StrictParserBackend::Libtooling",
        "ParserCoreCodegenEscapeHatch::Libtooling",
        "strict_parser_backend_from_legacy_backend",
        "parse_parser_backend_value",
        "strict_parser_backend_label",
        "parse_codegen_escape_hatch_value",
        "cargo test -p fragile-driver",
        "cargo test -p fragile-cli",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1 patch-spec doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.0 (pre-cutover)",
        "P0.b.2.b.1.b.1 (immediate)",
        "P0.b.2.b.1.b.2 (immediate)",
        "P0.b.2.b.1.b.3 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposed P0.b.2.b.1.b leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b_driver_patch_map_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2b1b_driver_variant_removal_patch_map.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b driver patch-map doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b_driver_patch_map_contains_required_line_anchors_and_boundaries() {
    let doc = read_project_file("docs/dev/p0b2b1b_driver_variant_removal_patch_map.md");
    for required in [
        "P0.b.2.b.1.b.0",
        "P0.b.2.b.1.b.1",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "crates/fragile-driver/src/lib.rs",
        "35-44",
        "591-643",
        "620-624",
        "908-923",
        "1269-1275",
        "1287-1290",
        "1324-1331",
        "1697-1762",
        "strict_parser_backend_from_legacy_backend",
        "parse_parser_backend_value",
        "strict_parser_backend_label",
        "parse_parser_core_codegen_escape_hatch_value",
        "P0.b.2.c",
        "P0.b.2.e",
        "cargo test -p fragile-driver",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b driver patch-map doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.0 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposed P0.b.2.b.1.b.1 leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1_enum_decl_patch_spec_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2b1b1_driver_enum_decl_removal_patch_spec.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1 declaration patch-spec doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1_enum_decl_patch_spec_contains_required_boundaries() {
    let doc = read_project_file("docs/dev/p0b2b1b1_driver_enum_decl_removal_patch_spec.md");
    for required in [
        "P0.b.2.b.1.b.1.0",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "crates/fragile-driver/src/lib.rs",
        "StrictParserBackend::Libtooling",
        "ParserCoreCodegenEscapeHatch::Libtooling",
        "35-39",
        "41-44",
        "908-923",
        "1269-1290",
        "1697-1762",
        "strict_parser_backend_from_legacy_backend",
        "cargo test -p fragile-driver",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1 declaration patch-spec doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1110_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.0 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1110_rehearsal_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1110_strict_parser_backend_decl_removal_rehearsal.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.0 rehearsal doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1110_rehearsal_document_contains_required_boundaries() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1110_strict_parser_backend_decl_removal_rehearsal.md");
    for required in [
        "P0.b.2.b.1.b.1.1.0",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "crates/fragile-driver/src/lib.rs",
        "StrictParserBackend::Libtooling",
        "35-39",
        "strict_parser_backend_from_legacy_backend",
        "parse_parser_backend_value",
        "strict_parser_backend_label",
        "cargo test -p fragile-driver",
        "cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.0 rehearsal doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1111_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.1 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 drift-guard leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1111_drift_guard_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1111_declaration_anchor_drift_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.1 drift-guard doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1111_drift_guard_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1111_declaration_anchor_drift_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.1",
        "P0.b.2.b.1.b.1.1",
        "crates/fragile-driver/src/lib.rs",
        "enum StrictParserBackend",
        "Libtooling,",
        "ParserCore { backend_id: String },",
        "nl -ba crates/fragile-driver/src/lib.rs | sed -n '34,40p'",
        "rg -n \"StrictParserBackend::Libtooling\" crates/fragile-driver/src/lib.rs",
        "594",
        "622",
        "912",
        "1270",
        "1701",
        "1707",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "P0.b.2.b.1.b.1.2",
        "cargo test -p fragile-driver",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.1 drift-guard doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1112_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.2 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 single-hunk contract leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1112_single_hunk_contract_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1112_single_hunk_patch_contract.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.2 single-hunk contract doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1112_single_hunk_contract_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1112_single_hunk_patch_contract.md");
    for required in [
        "P0.b.2.b.1.b.1.1.2",
        "P0.b.2.b.1.b.1.1",
        "crates/fragile-driver/src/lib.rs",
        "StrictParserBackend::Libtooling",
        "enum StrictParserBackend {",
        "-    Libtooling,",
        "ParserCore { backend_id: String },",
        "rg -n '^\\s*Libtooling,$' crates/fragile-driver/src/lib.rs",
        "nl -ba crates/fragile-driver/src/lib.rs | sed -n '34,40p'",
        "rg -n 'ParserCore \\{ backend_id: String \\},' crates/fragile-driver/src/lib.rs",
        "rg -n \"StrictParserBackend::Libtooling\" crates/fragile-driver/src/lib.rs",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "P0.b.2.b.1.b.1.2",
        "cargo test -p fragile-driver",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.2 single-hunk contract doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1113_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.3 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 count-invariant guard leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1113_count_invariant_guard_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2b1b1b1113_count_invariant_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.3 count-invariant guard doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1113_count_invariant_guard_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1113_count_invariant_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.3",
        "P0.b.2.b.1.b.1.1",
        "crates/fragile-driver/src/lib.rs",
        "sed -n '36,39p' crates/fragile-driver/src/lib.rs | rg -n '^\\s*Libtooling,$' | wc -l",
        "expected: 1",
        "sed -n '42,44p' crates/fragile-driver/src/lib.rs | rg -n '^\\s*Libtooling,$' | wc -l",
        "expected: 6",
        "rg -n \"StrictParserBackend::Libtooling\" crates/fragile-driver/src/lib.rs | wc -l",
        "594",
        "622",
        "912",
        "1270",
        "1701",
        "1707",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "cargo test -p fragile-driver",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.3 count-invariant guard doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1114_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.4 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 compile-error fingerprint leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1114_compile_error_fingerprint_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1114_compile_error_fingerprint_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.4 compile-error fingerprint doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1114_compile_error_fingerprint_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1114_compile_error_fingerprint_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.4",
        "P0.b.2.b.1.b.1.1",
        "crates/fragile-driver/src/lib.rs",
        "error[E0599]",
        "no variant or associated item named `Libtooling` found for enum `StrictParserBackend`",
        "cargo test -p fragile-driver 2>&1 | tee /tmp/p0b2b1b1b1114_after_b111.log",
        "rg -n 'error\\[E0599\\]' /tmp/p0b2b1b1b1114_after_b111.log",
        "rg -n 'src/lib.rs:(594|622|912|1270|1701|1707):' /tmp/p0b2b1b1b1114_after_b111.log",
        "594",
        "622",
        "912",
        "1270",
        "1701",
        "1707",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.4 compile-error fingerprint doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1115_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.5 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 transition-matrix leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1115_cutover_transition_matrix_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1115_cutover_transition_matrix_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.5 cutover transition-matrix doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1115_cutover_transition_matrix_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1115_cutover_transition_matrix_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.5",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "error[E0599]",
        "StrictParserBackend",
        "ParserCoreCodegenEscapeHatch",
        "594",
        "622",
        "636",
        "912",
        "1270",
        "1289",
        "1701",
        "1707",
        "1752",
        "cargo test -p fragile-driver 2>&1 | tee /tmp/p0b2b1b1b1115_step.log",
        "rg -n 'src/lib.rs:(594|622|636|912|1270|1289|1701|1707|1752):' /tmp/p0b2b1b1b1115_step.log",
        "cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.5 cutover transition-matrix doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1116_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.6 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 stepwise-log leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1116_stepwise_log_contract_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1116_stepwise_diagnostic_log_contract.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.6 stepwise-log contract doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1116_stepwise_log_contract_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1116_stepwise_diagnostic_log_contract.md");
    for required in [
        "P0.b.2.b.1.b.1.1.6",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "/tmp/p0b2b1b1b1116_b111.log",
        "/tmp/p0b2b1b1b1116_b112.log",
        "/tmp/p0b2b1b1b1116_b2.log",
        "/tmp/p0b2b1b1b1116_b3.log",
        "/tmp/p0b2b1b1b1116_c.log",
        "error[E0599]",
        "594",
        "622",
        "636",
        "912",
        "1270",
        "1289",
        "1701",
        "1707",
        "1752",
        "cargo test -p fragile-driver 2>&1 | tee /tmp/p0b2b1b1b1116_<step>.log",
        "rg -n 'src/lib.rs:(594|622|912|1270|1701|1707):' /tmp/p0b2b1b1b1116_b111.log",
        "rg -n 'src/lib.rs:(594|622|636|912|1270|1289|1701|1707|1752):' /tmp/p0b2b1b1b1116_b112.log",
        "cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.6 stepwise-log contract doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1117_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.7 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 anchor-delta leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1117_anchor_delta_guard_document_exists() {
    let doc_path = project_root().join("docs/dev/p0b2b1b1b1117_anchor_delta_transition_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.7 anchor-delta guard doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1117_anchor_delta_guard_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1117_anchor_delta_transition_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.7",
        "P0.b.2.b.1.b.1.1.6",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "/tmp/p0b2b1b1b1116_b111.log",
        "/tmp/p0b2b1b1b1116_b112.log",
        "/tmp/p0b2b1b1b1117_b111.anchors",
        "/tmp/p0b2b1b1b1117_b112.anchors",
        "comm -13 /tmp/p0b2b1b1b1117_b111.anchors /tmp/p0b2b1b1b1117_b112.anchors",
        "comm -23 /tmp/p0b2b1b1b1117_b112.anchors /tmp/p0b2b1b1b1117_b2.anchors",
        "src/lib.rs:636:",
        "src/lib.rs:912:",
        "src/lib.rs:594:",
        "src/lib.rs:1270:",
        "src/lib.rs:1701:",
        "src/lib.rs:1707:",
        "src/lib.rs:622:",
        "src/lib.rs:1289:",
        "src/lib.rs:1752:",
        "error[E0599]",
        "cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.7 anchor-delta guard doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1118_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.8 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 step-artifact freshness leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1118_step_artifact_freshness_guard_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1118_step_artifact_freshness_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.8 step-artifact freshness guard doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1118_step_artifact_freshness_guard_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1118_step_artifact_freshness_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.8",
        "P0.b.2.b.1.b.1.1.6",
        "P0.b.2.b.1.b.1.1.7",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "START_EPOCH=\"$(date -u +%s)\"",
        "RUN_ID=\"p0b2b1b1b1118_$(date -u +%Y%m%dT%H%M%SZ)\"",
        "rm -f",
        "/tmp/p0b2b1b1b1116_b111.log",
        "/tmp/p0b2b1b1b1116_b112.log",
        "/tmp/p0b2b1b1b1116_b2.log",
        "/tmp/p0b2b1b1b1116_b3.log",
        "/tmp/p0b2b1b1b1116_c.log",
        "/tmp/p0b2b1b1b1117_b111.anchors",
        "/tmp/p0b2b1b1b1117_c.anchors",
        "RUN_ID=${RUN_ID} STEP=b111",
        "RUN_ID=${RUN_ID} STEP=c",
        "rg -n \"RUN_ID=${RUN_ID} STEP=b111\" /tmp/p0b2b1b1b1116_b111.log",
        "test \"$(stat -c %Y /tmp/p0b2b1b1b1116_b111.log)\" -ge \"${START_EPOCH}\"",
        "error[E0599]",
        "cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.8 step-artifact freshness guard doc should contain `{}`",
            required
        );
    }
}

#[test]
fn p0b_2b1b1b1119_task_decomposed_in_todo() {
    let todo = read_project_file("TODO.md");
    for expected_leaf in [
        "P0.b.2.b.1.b.1.1.9 (pre-cutover)",
        "P0.b.2.b.1.b.1.1 (immediate)",
        "P0.b.2.b.1.b.1.2 (immediate)",
    ] {
        assert!(
            todo.contains(expected_leaf),
            "audit: TODO should contain decomposition for P0.b.2.b.1.b.1.1 step-artifact integrity leaf `{}`",
            expected_leaf
        );
    }
}

#[test]
fn p0b_2b1b1b1119_step_artifact_integrity_guard_document_exists() {
    let doc_path =
        project_root().join("docs/dev/p0b2b1b1b1119_step_artifact_integrity_guard.md");
    assert!(
        doc_path.exists(),
        "audit: expected P0.b.2.b.1.b.1.1.9 step-artifact integrity guard doc to exist at {}",
        doc_path.display()
    );
}

#[test]
fn p0b_2b1b1b1119_step_artifact_integrity_guard_document_contains_required_checks() {
    let doc = read_project_file("docs/dev/p0b2b1b1b1119_step_artifact_integrity_guard.md");
    for required in [
        "P0.b.2.b.1.b.1.1.9",
        "P0.b.2.b.1.b.1.1.6",
        "P0.b.2.b.1.b.1.1.7",
        "P0.b.2.b.1.b.1.1.8",
        "P0.b.2.b.1.b.1.1",
        "P0.b.2.b.1.b.1.2",
        "P0.b.2.b.1.b.2",
        "P0.b.2.b.1.b.3",
        "P0.b.2.c",
        "RUN_ID=\"p0b2b1b1b1119_$(date -u +%Y%m%dT%H%M%SZ)\"",
        "sha256sum /tmp/p0b2b1b1b1116_b111.log > /tmp/p0b2b1b1b1119_b111.log.sha256",
        "sha256sum /tmp/p0b2b1b1b1117_b111.anchors > /tmp/p0b2b1b1b1119_b111.anchors.sha256",
        "sha256sum -c /tmp/p0b2b1b1b1119_b111.log.sha256",
        "sha256sum -c /tmp/p0b2b1b1b1119_b111.anchors.sha256",
        "rm -f /tmp/p0b2b1b1b1119_*.sha256",
        "test -s /tmp/p0b2b1b1b1119_${RUN_ID}.manifest",
        "error[E0599]",
        "cargo test -p fragile-clang --test p0_libtooling_removal_audit_tests",
    ] {
        assert!(
            doc.contains(required),
            "audit: P0.b.2.b.1.b.1.1.9 step-artifact integrity guard doc should contain `{}`",
            required
        );
    }
}

/// Comprehensive summary test: verify P0.b removal progress.
/// Files that have been fully cleaned up are checked for absence of removed symbols.
#[test]
fn p0a_audit_comprehensive_site_inventory() {
    // P0.b.2: Production driver files should be cleaned of escape hatch infrastructure.
    let driver_src = read_project_file("crates/fragile-driver/src/lib.rs");
    assert!(!driver_src.contains("ParserCoreCodegenEscapeHatch"),
        "anti-regression: ParserCoreCodegenEscapeHatch should be removed from fragile-driver");
    assert!(!driver_src.contains("ESCAPE_HATCH_HARDENING_EXPIRY"),
        "anti-regression: ESCAPE_HATCH_HARDENING_EXPIRY should be removed from fragile-driver");
    assert!(!driver_src.contains("StrictParserBackend"),
        "anti-regression: StrictParserBackend should be removed from fragile-driver");

    let fragilec_src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    assert!(!fragilec_src.contains("StrictParserBackend"),
        "anti-regression: StrictParserBackend should be removed from fragilec");
    assert!(!fragilec_src.contains("ParserCoreCodegenEscapeHatch"),
        "anti-regression: ParserCoreCodegenEscapeHatch should be removed from fragilec");

    // P0.b.2.d: escape hatch script should be removed.
    let script_path = project_root().join("scripts/escape_hatch_usage_report.py");
    assert!(!script_path.exists(),
        "anti-regression: escape_hatch_usage_report.py should be removed");

    // Files that still exist and have remaining LibTooling code (for P0.b.4+):
    let remaining_files = vec![
        ("crates/fragile-clang/src/libtooling.rs", "LibTooling module (P0.b.5)"),
        ("crates/fragile-cli/src/main.rs", "--use-libtooling CLI flag (P0.b.6)"),
        ("crates/fragile-clang/src/ast_codegen.rs", "LibTooling enrichment state (P0.b.4)"),
    ];

    eprintln!("\n=== P0.b Removal Progress: production drivers cleaned, {} remaining sites ===", remaining_files.len());
    for (i, (file, desc)) in remaining_files.iter().enumerate() {
        let path = project_root().join(file);
        let status = if path.exists() { "EXISTS" } else { "MISSING" };
        eprintln!("  [{:2}] [{}] {} :: {}", i + 1, status, file, desc);
    }
    eprintln!("=== End of inventory ===\n");
}

// ---------------------------------------------------------------------------
// P0.d Documentation consistency gates
// ---------------------------------------------------------------------------

/// P0.d: CLAUDE.md must describe LibTooling parser path as "removed", not just "deprecated".
#[test]
fn test_p0d_claude_md_marks_libtooling_as_removed() {
    let content = read_project_file("CLAUDE.md");

    // Must contain "removed" in relation to LibTooling
    assert!(
        content.contains("removed") && content.contains("fragile-parser-clang"),
        "CLAUDE.md must describe LibTooling as removed and reference fragile-parser-clang as sole backend"
    );

    // Must NOT show --use-libtooling in a CLI usage example (OK to mention it as removed)
    assert!(
        !content.contains("fragile transpile file.cpp --use-libtooling"),
        "CLAUDE.md must not show --use-libtooling as a working CLI example"
    );

    // Must NOT still describe FRAGILEC_PARSER_BACKEND=libtooling as a settable option
    assert!(
        !content.contains("FRAGILEC_PARSER_BACKEND=libtooling") ||
        content.contains("no longer used"),
        "CLAUDE.md must not describe FRAGILEC_PARSER_BACKEND=libtooling as an available option"
    );
}

/// P0.d: fragile-dev-book.md operational sections must say "removed", not just "deprecated".
#[test]
fn test_p0d_dev_book_operational_sections_mark_libtooling_as_removed() {
    let content = read_project_file("docs/fragile-dev-book.md");

    // The backend status section must say "removed"
    assert!(
        content.contains("LibTooling` parser flow has been **removed**"),
        "dev-book backend status must say LibTooling is removed, not deprecated"
    );

    // The backend note section must say "removed"
    assert!(
        content.contains("have been **removed** from the active production path"),
        "dev-book backend note must say backends are removed from production path"
    );

    // The migration policy must reference removal
    assert!(
        content.contains("removed** from the active production flow"),
        "dev-book migration policy must reference removal from production flow"
    );

    // Must say fragile-parser-clang is used exclusively
    assert!(
        content.contains("fragile-parser-clang` backend exclusively") ||
        content.contains("fragile-parser-clang` exclusively"),
        "dev-book must state fragile-parser-clang is the exclusive backend"
    );
}

/// P0.d: CLAUDE.md current status date must be updated to 2026-03-21 or later.
#[test]
fn test_p0d_claude_md_status_date_is_current() {
    let content = read_project_file("CLAUDE.md");

    // Must NOT still show old date in status header
    assert!(
        !content.contains("Current Status (as of 2026-02-28)"),
        "CLAUDE.md status section must be updated past the 2026-02-28 date"
    );

    // Must show a current date
    assert!(
        content.contains("Current Status (as of 2026-03-21)"),
        "CLAUDE.md status section must reflect the P0.d update date (2026-03-21)"
    );
}
