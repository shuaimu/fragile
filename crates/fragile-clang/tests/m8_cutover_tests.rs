/// M8.1 Parser Backend Cutover Tests
///
/// Verifies that the default parser backend is fragile-parser-clang, and that
/// end-to-end compilation through the parser-core backend works for
/// representative C and C++ fixtures.
///
/// M8.2/M8.A2 escape hatch tests removed in P0.b.2.d (escape hatch
/// infrastructure hard-removed 2026-03-22).

use fragile_clang::{
    transpile_parser_output_to_rust, ParserLanguage,
    TranspileOptions, transpile_cpp_to_rust_with_options,
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

fn workspace_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should exist")
        .to_path_buf()
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
    // LibTooling backend is still available in fragile-clang for internal use
    // (will be removed in P0.b.3+).
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
// M8.A1 Acceptance: CI defaults use new backend with green required checks.
// ---------------------------------------------------------------------------

#[test]
fn m8_a1_ci_required_workflow_does_not_pin_parser_backend_or_escape_hatch() {
    let workflow_path = workspace_root_dir().join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("failed to read CI workflow");

    assert!(
        !workflow.contains("FRAGILEC_PARSER_BACKEND"),
        "required CI workflow must not pin parser backend override; found FRAGILEC_PARSER_BACKEND in {}",
        workflow_path.display()
    );
    assert!(
        !workflow.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH"),
        "required CI workflow must not set parser-core codegen escape hatch in {}",
        workflow_path.display()
    );
}

#[test]
fn m8_a1_ci_required_workflow_keeps_required_job_matrix_present() {
    let workflow_path = workspace_root_dir().join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("failed to read CI workflow");

    for job in [
        "build",
        "lint",
        "fmt",
        "zlib-smoke-parity",
        "tinyxml2-smoke-parity",
        "pugixml-smoke-baseline",
        "rapidjson-smoke-baseline",
    ] {
        let marker = format!("\n  {}:\n", job);
        assert!(
            workflow.contains(&marker),
            "required CI job `{}` missing from {}",
            job,
            workflow_path.display()
        );
    }
}
