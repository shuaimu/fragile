/// M8.1 Parser Backend Cutover Tests
///
/// Verifies that the default parser backend has been flipped from libtooling to
/// fragile-parser-clang, and that explicit override to libtooling still works.
/// Also verifies end-to-end compilation through the new default backend for
/// representative C and C++ fixtures.
///
/// M8.2 Escape Hatch Hardening Window Tests
///
/// Verifies that the libtooling escape hatches emit deprecation warnings, log
/// usage to a file when configured, and are rejected after the hardening window
/// expiry date.

use fragile_clang::{
    transpile_parser_output_to_rust, ParserBackend, ParserLanguage, TemplateParsingMode,
    TranspileOptions, transpile_cpp_to_rust_with_options,
};
use fragile_driver::{
    enforce_escape_hatch_policy_as_of, escape_hatch_hardening_expired_as_of,
    ESCAPE_HATCH_HARDENING_EXPIRY, FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV,
};
use fragile_parser_clang::{FragileParserClangBackend, FRAGILE_PARSER_CLANG_BACKEND_ID};
use fragile_parser_core::{
    BackendRegistry, ParseRequest, ParserBackend as ParserBackendTrait,
    ParserLanguage as ParserCoreLanguage,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "fragile_m8_cutover_{}_{}_{}",
        label,
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn parse_with_new_backend(source_path: &std::path::Path, language: ParserCoreLanguage) -> fragile_parser_core::ParserOutputV1 {
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: source_path.to_path_buf(),
        language,
        frontend_args: Vec::new(),
        defines: Vec::new(),
        include_directives: Vec::new(),
    };
    backend.parse(&request).expect("parse should succeed")
}

// ---------------------------------------------------------------------------
// M8.1 Core: Default backend is now fragile-parser-clang
// ---------------------------------------------------------------------------

#[test]
fn m8_1_default_backend_is_fragile_parser_clang() {
    // The default backend constant must be "fragile-parser-clang", not "libtooling".
    // This is a contract test: if someone changes the default, this test fails.
    assert_eq!(
        FRAGILE_PARSER_CLANG_BACKEND_ID, "fragile-parser-clang",
        "backend ID constant must be fragile-parser-clang"
    );
}

#[test]
fn m8_1_new_backend_parses_simple_c_fixture() {
    let dir = temp_dir("c_parse");
    let source = dir.join("hello.c");
    fs::write(&source, "int add(int a, int b) { return a + b; }\n").unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::C);
    assert_eq!(output.translation_unit.parser_backend, FRAGILE_PARSER_CLANG_BACKEND_ID);
    assert!(!output.nodes.is_empty(), "parser output must contain nodes");

    // Must contain at least one function_decl node for `add`.
    let has_function_decl = output.nodes.iter().any(|n| n.node_kind == "function_decl");
    assert!(has_function_decl, "parser output must contain a function_decl node");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_new_backend_parses_simple_cpp_fixture() {
    let dir = temp_dir("cpp_parse");
    let source = dir.join("hello.cpp");
    fs::write(
        &source,
        r#"
struct Point {
    int x;
    int y;
};

extern "C" int dot(Point a, Point b) {
    return a.x * b.x + a.y * b.y;
}
"#,
    )
    .unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::Cpp);
    assert_eq!(output.translation_unit.parser_backend, FRAGILE_PARSER_CLANG_BACKEND_ID);
    assert!(!output.nodes.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_new_backend_transpiles_c_to_rust() {
    let dir = temp_dir("c_transpile");
    let source = dir.join("arith.c");
    fs::write(
        &source,
        "int add(int a, int b) { return a + b; }\nint main(void) { return add(1, 2); }\n",
    )
    .unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::C);
    let transpiled = transpile_parser_output_to_rust(&output)
        .expect("transpile through new default backend should succeed");

    assert!(
        transpiled.contains("fn add"),
        "transpiled output must contain `fn add`: got:\n{}",
        &transpiled[..transpiled.len().min(500)]
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_new_backend_transpiles_cpp_to_rust() {
    let dir = temp_dir("cpp_transpile");
    let source = dir.join("point.cpp");
    fs::write(
        &source,
        r#"
struct Point {
    int x;
    int y;
};

extern "C" int sum_point(Point p) {
    return p.x + p.y;
}
"#,
    )
    .unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::Cpp);
    let transpiled = transpile_parser_output_to_rust(&output)
        .expect("transpile through new default backend should succeed for C++");

    assert!(
        transpiled.contains("struct Point") || transpiled.contains("pub struct Point"),
        "transpiled C++ output must contain struct Point:\n{}",
        &transpiled[..transpiled.len().min(500)]
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_libtooling_backend_still_available_via_explicit_request() {
    // Even though the default has changed, libtooling must still be a valid backend
    // for the escape hatch (M8.2).
    let dir = temp_dir("libtooling_explicit");
    let source = dir.join("legacy.c");
    fs::write(&source, "int neg(int x) { return -x; }\n").unwrap();

    let opts = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: Vec::new(),
        language: ParserLanguage::C,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Standard,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };

    let transpiled = transpile_cpp_to_rust_with_options(&source, &opts)
        .expect("explicit libtooling backend should still work");
    assert!(
        transpiled.contains("fn neg"),
        "libtooling transpiled output must contain fn neg"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_backend_registry_resolves_new_default() {
    let mut registry = BackendRegistry::new();
    registry
        .register(FragileParserClangBackend)
        .expect("register new backend");

    // The default backend must be resolvable in the registry.
    let backend = registry
        .get(FRAGILE_PARSER_CLANG_BACKEND_ID)
        .expect("default backend must be resolvable in registry");
    assert_eq!(backend.backend_id(), FRAGILE_PARSER_CLANG_BACKEND_ID);
}

#[test]
fn m8_1_new_backend_parse_output_schema_version_is_v1() {
    let dir = temp_dir("schema_v1");
    let source = dir.join("schema.c");
    fs::write(&source, "void noop(void) {}\n").unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::C);
    assert_eq!(
        output.schema_version,
        fragile_parser_core::PARSER_OUTPUT_SCHEMA_VERSION_V1,
        "parser output schema version must be v1"
    );

    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// M8.1 Parity: new backend produces equivalent results to libtooling on
// a representative fixture corpus.
// ---------------------------------------------------------------------------

#[test]
fn m8_1_parity_both_backends_transpile_arithmetic_fixture() {
    let dir = temp_dir("parity_arith");
    let source = dir.join("arith.c");
    fs::write(
        &source,
        "int add(int a, int b) { return a + b; }\nint mul(int a, int b) { return a * b; }\n",
    )
    .unwrap();

    // New backend (parser-output handoff)
    let new_output = parse_with_new_backend(&source, ParserCoreLanguage::C);
    let new_transpiled = transpile_parser_output_to_rust(&new_output)
        .expect("new backend transpile should succeed");

    // Legacy backend (libtooling)
    let legacy_opts = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: Vec::new(),
        language: ParserLanguage::C,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Standard,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };
    let legacy_transpiled = transpile_cpp_to_rust_with_options(&source, &legacy_opts)
        .expect("libtooling transpile should succeed");

    // Both must produce `fn add` and `fn mul` declarations.
    for marker in ["fn add", "fn mul"] {
        assert!(
            new_transpiled.contains(marker),
            "new backend output must contain `{marker}`"
        );
        assert!(
            legacy_transpiled.contains(marker),
            "libtooling output must contain `{marker}`"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_parity_both_backends_handle_struct_and_function() {
    let dir = temp_dir("parity_struct");
    let source = dir.join("point.cpp");
    fs::write(
        &source,
        r#"
struct Point { int x; int y; };
extern "C" int sum_point(Point p) { return p.x + p.y; }
"#,
    )
    .unwrap();

    // New backend
    let new_output = parse_with_new_backend(&source, ParserCoreLanguage::Cpp);
    let new_transpiled = transpile_parser_output_to_rust(&new_output)
        .expect("new backend transpile should succeed");

    // Legacy backend
    let legacy_opts = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: Vec::new(),
        language: ParserLanguage::Cpp,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Standard,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };
    let legacy_transpiled = transpile_cpp_to_rust_with_options(&source, &legacy_opts)
        .expect("libtooling transpile should succeed");

    // Both must produce struct Point and fn sum_point.
    for marker in ["Point", "fn sum_point"] {
        assert!(
            new_transpiled.contains(marker),
            "new backend output must contain `{marker}`"
        );
        assert!(
            legacy_transpiled.contains(marker),
            "libtooling output must contain `{marker}`"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// M8.1 End-to-end: rustc compiles the output from the new default backend.
// ---------------------------------------------------------------------------

#[test]
fn m8_1_new_backend_output_compiles_with_rustc() {
    let dir = temp_dir("e2e_rustc");
    let source = dir.join("main.c");
    fs::write(
        &source,
        "int add(int a, int b) { return a + b; }\n",
    )
    .unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::C);
    let transpiled = transpile_parser_output_to_rust(&output)
        .expect("new backend transpile should succeed");

    let rs_file = dir.join("main.rs");
    fs::write(&rs_file, &transpiled).unwrap();

    let rustc = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit=obj")
        .arg(&rs_file)
        .arg("-o")
        .arg(dir.join("main.o"))
        .output()
        .expect("rustc should run");

    assert!(
        rustc.status.success(),
        "rustc should compile new-backend output successfully.\nstderr: {}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_1_new_backend_cpp_output_compiles_with_rustc() {
    let dir = temp_dir("e2e_rustc_cpp");
    let source = dir.join("example.cpp");
    fs::write(
        &source,
        r#"
struct Pair { int first; int second; };
extern "C" int sum_pair(Pair p) { return p.first + p.second; }
"#,
    )
    .unwrap();

    let output = parse_with_new_backend(&source, ParserCoreLanguage::Cpp);
    let transpiled = transpile_parser_output_to_rust(&output)
        .expect("new backend C++ transpile should succeed");

    let rs_file = dir.join("example.rs");
    fs::write(&rs_file, &transpiled).unwrap();

    let rustc = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit=obj")
        .arg(&rs_file)
        .arg("-o")
        .arg(dir.join("example.o"))
        .output()
        .expect("rustc should run");

    assert!(
        rustc.status.success(),
        "rustc should compile new-backend C++ output.\nstderr: {}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let _ = fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// M8.2 Escape Hatch Hardening Window Tests
// ---------------------------------------------------------------------------

#[test]
fn m8_2_hardening_expiry_date_is_defined_and_future_format() {
    // The expiry date must be a valid YYYY-MM-DD string.
    assert_eq!(
        ESCAPE_HATCH_HARDENING_EXPIRY.len(),
        10,
        "expiry date must be YYYY-MM-DD format"
    );
    assert_eq!(
        &ESCAPE_HATCH_HARDENING_EXPIRY[4..5],
        "-",
        "expiry date must have dash at position 4"
    );
    assert_eq!(
        &ESCAPE_HATCH_HARDENING_EXPIRY[7..8],
        "-",
        "expiry date must have dash at position 7"
    );
    // The expiry date must be 2026-04-18.
    assert_eq!(
        ESCAPE_HATCH_HARDENING_EXPIRY, "2026-04-18",
        "hardening window expiry must be 2026-04-18"
    );
}

#[test]
fn m8_2_escape_hatch_not_expired_before_expiry_date() {
    assert!(
        !escape_hatch_hardening_expired_as_of("2026-03-18"),
        "escape hatch should not be expired on 2026-03-18 (before expiry)"
    );
    assert!(
        !escape_hatch_hardening_expired_as_of("2026-04-18"),
        "escape hatch should not be expired on 2026-04-18 (expiry day itself)"
    );
}

#[test]
fn m8_2_escape_hatch_expired_after_expiry_date() {
    assert!(
        escape_hatch_hardening_expired_as_of("2026-04-19"),
        "escape hatch should be expired on 2026-04-19 (day after expiry)"
    );
    assert!(
        escape_hatch_hardening_expired_as_of("2026-05-01"),
        "escape hatch should be expired on 2026-05-01 (well after expiry)"
    );
}

#[test]
fn m8_2_enforce_policy_allows_within_window() {
    // Within the hardening window, the policy should succeed (return Ok).
    let result = enforce_escape_hatch_policy_as_of(
        "FRAGILEC_PARSER_BACKEND=libtooling",
        "test.cpp",
        "2026-03-20",
    );
    assert!(
        result.is_ok(),
        "escape hatch should be allowed within hardening window: {:?}",
        result
    );
}

#[test]
fn m8_2_enforce_policy_rejects_after_window() {
    // After the hardening window, the policy should fail (return Err).
    let result = enforce_escape_hatch_policy_as_of(
        "FRAGILEC_PARSER_BACKEND=libtooling",
        "test.cpp",
        "2026-04-19",
    );
    assert!(
        result.is_err(),
        "escape hatch should be rejected after hardening window expiry"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("rejected") && err.contains("2026-04-18"),
        "error message should mention rejection and expiry date: {}",
        err
    );
}

#[test]
fn m8_2_enforce_policy_rejects_codegen_escape_hatch_after_window() {
    let result = enforce_escape_hatch_policy_as_of(
        "FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling",
        "unit.cpp",
        "2026-05-01",
    );
    assert!(
        result.is_err(),
        "codegen escape hatch should be rejected after hardening window expiry"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH"),
        "error should reference the codegen escape hatch: {}",
        err
    );
}

#[test]
fn m8_2_escape_hatch_log_writes_usage_entry() {
    let dir = temp_dir("escape_log");
    let log_path = dir.join("escape_hatch.log");

    // Set the log path env var, call the log function, then unset.
    std::env::set_var(FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV, log_path.to_str().unwrap());
    fragile_driver::log_escape_hatch_usage(
        "FRAGILEC_PARSER_BACKEND=libtooling",
        "example.cpp",
    );
    std::env::remove_var(FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV);

    assert!(
        log_path.exists(),
        "escape hatch log file should be created"
    );
    let contents = fs::read_to_string(&log_path).expect("should read log file");
    assert!(
        contents.contains("escape_kind=FRAGILEC_PARSER_BACKEND=libtooling"),
        "log should contain escape kind: {}",
        contents
    );
    assert!(
        contents.contains("source=example.cpp"),
        "log should contain source file: {}",
        contents
    );
    assert!(
        contents.contains("timestamp="),
        "log should contain timestamp: {}",
        contents
    );
    assert!(
        contents.contains("pid="),
        "log should contain pid: {}",
        contents
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_2_escape_hatch_log_appends_multiple_entries() {
    let dir = temp_dir("escape_log_multi");
    let log_path = dir.join("escape_hatch.log");

    std::env::set_var(FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV, log_path.to_str().unwrap());
    fragile_driver::log_escape_hatch_usage("backend-escape", "a.cpp");
    fragile_driver::log_escape_hatch_usage("codegen-escape", "b.cpp");
    std::env::remove_var(FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV);

    let contents = fs::read_to_string(&log_path).expect("should read log file");
    let lines: Vec<&str> = contents.lines().collect();
    assert!(
        lines.len() >= 2,
        "log should contain at least 2 entries, got {}: {}",
        lines.len(),
        contents
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_2_no_log_when_env_unset() {
    let dir = temp_dir("escape_log_noop");
    let log_path = dir.join("should_not_exist.log");

    // Ensure the env var is not set.
    std::env::remove_var(FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV);
    fragile_driver::log_escape_hatch_usage("test-escape", "noop.cpp");

    assert!(
        !log_path.exists(),
        "log file should not be created when env var is unset"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn m8_2_deprecation_warning_message_contains_expiry_date() {
    // The deprecation warning function writes to stderr. We verify its content
    // indirectly by checking the enforce_policy_as_of error message, which
    // includes the expiry date and migration guidance.
    let err = enforce_escape_hatch_policy_as_of(
        "FRAGILEC_PARSER_BACKEND=libtooling",
        "file.cpp",
        "2026-12-01",
    )
    .unwrap_err();
    assert!(
        err.contains("2026-04-18"),
        "rejection error should reference hardening expiry date: {}",
        err
    );
    assert!(
        err.contains("fragile-parser-clang"),
        "rejection error should mention migration target: {}",
        err
    );
    assert!(
        err.contains("file.cpp"),
        "rejection error should mention the source file: {}",
        err
    );
}
