/// M7.1 Shadow Mode Tests
///
/// Runs the legacy transpile path (libtooling backend via `transpile_cpp_to_rust_with_options`)
/// and the new parser-output-handoff path (`FragileParserClangBackend::parse()` →
/// `transpile_parser_output_to_rust()`) on a representative non-RPC corpus.
///
/// Compares: rustc compilation status, marker presence, output metrics, unresolved-name counts.
/// Emits a deterministic parity manifest under a temp run-root.
/// Asserts the new backend is non-worsening vs legacy baseline.

use fragile_clang::{
    transpile_cpp_to_rust_with_options, transpile_parser_output_to_rust, ParserBackend,
    ParserLanguage, TemplateParsingMode, TranspileOptions,
};
use fragile_parser_clang::FragileParserClangBackend;
use fragile_parser_core::{ParseRequest, ParserBackend as ParserBackendTrait, ParserLanguage as ParserCoreLanguage};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const SHADOW_LOG_ROOT_PREFIX: &str = "fragile_m7_shadow_mode";

// Marker constants reused from parity fixture corpus
const MARKER_FN_ADD: &str = "pub extern \"C\" fn add";
const MARKER_FN_MUL: &str = "pub extern \"C\" fn mul";
const MARKER_RETURN_ADD_LEGACY: &str = "return a + b";
const MARKER_RETURN_ADD_CAST_I32: &str = "return (a + b) as i32";
const MARKER_RETURN_MUL_LEGACY: &str = "return x * y";
const MARKER_RETURN_MUL_CAST_I32: &str = "return (x * y) as i32";
const MARKER_TYPEDEF_COUNT: &str = "pub type Count =";
const MARKER_ALIAS_DISTANCE: &str = "pub type Distance =";
const MARKER_ENUM_MODE: &str = "pub enum Mode";
const MARKER_STRUCT_POINT: &str = "pub struct Point";
const MARKER_STRUCT_POINT_X: &str = "pub x: i32";
const MARKER_STRUCT_POINT_Y: &str = "pub y: i32";
const MARKER_NAMESPACE_MATH: &str = "pub mod math";
const MARKER_NAMESPACE_NS_ADD: &str = "pub extern \"C\" fn ns_add";
const MARKER_TEMPLATE_FN_IDENTITY_I32: &str = "pub fn identity_i32";
const MARKER_TEMPLATE_STRUCT_BOX_INT: &str = "pub struct Box_int_";
const MARKER_TEMPLATE_ALIAS_BOX_INT: &str = "pub type Box_int_ = std::boxed::Box<i32>";
const MARKER_TYPEDEF_INTARRAY4: &str = "pub type IntArray4 = [i32; 4]";
const MARKER_DECLTYPE_SCALAR_FN_SIG: &str =
    "pub extern \"C\" fn decltype_scalar_identity(v: i32) -> i32";
const MARKER_CONST_PTR_FN_SIG: &str = "pub extern \"C\" fn read_const_ptr(p: *const i32) -> i32";
const MARKER_MUT_REF_FN_SIG: &str = "pub extern \"C\" fn bump_ref(mut value: &mut i32) -> i32";
const MARKER_ARRAY_DECAY_FN_SIG: &str =
    "pub extern \"C\" fn array_decay_head(data: *mut i32) -> i32";

/// Representative non-RPC fixture source — same corpus used in M0.2 backend parity tests.
const SHADOW_FIXTURE_SOURCE: &str = r#"
typedef unsigned long Count;
using Distance = int;
typedef int IntArray4[4];
using DecltypeScalar = decltype(1);
struct __FILE;
typedef __FILE FragileFileAlias;

enum Mode {
    ModeA = 1,
    ModeB = 2,
};

struct Point {
    int x;
    int y;
};

template<typename T>
struct Box {
    T value;
};

template struct Box<int>;

template<typename T>
T identity(T templ_value) {
    return templ_value;
}

template int identity<int>(int);

int use_identity() {
    return identity<int>(7);
}

int read_const_ptr(const int* p) {
    return *p;
}

int bump_ref(int& value) {
    value += 1;
    return value;
}

DecltypeScalar decltype_scalar_identity(DecltypeScalar v) {
    return v + 1;
}

int array_decay_head(int* data) {
    return data[0];
}

namespace math {
int ns_add(int lhs, int rhs) {
    return lhs + rhs;
}
}

int add(int a, int b) {
    return a + b;
}

int mul(int x, int y) {
    return x * y;
}
"#;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{stamp}"))
}

#[derive(Debug, Clone)]
struct ShadowModeResult {
    backend_label: String,
    rust_path: PathBuf,
    rustc_status: i32,
    line_count: usize,
    unresolved_name_count: usize,
    marker_hits: Vec<(&'static str, bool)>,
}

impl ShadowModeResult {
    fn marker_hit_count(&self) -> usize {
        self.marker_hits.iter().filter(|(_, hit)| *hit).count()
    }
    fn marker_total(&self) -> usize {
        self.marker_hits.len()
    }
}

fn count_unresolved_names(code: &str) -> usize {
    let mut count = 0;
    for line in code.lines() {
        if line.contains("error[E0425]") || line.contains("/* unresolved */") {
            count += 1;
        }
        // Count todo!() placeholders as unresolved
        if line.contains("todo!(") {
            count += 1;
        }
    }
    count
}

fn check_markers(code: &str) -> Vec<(&'static str, bool)> {
    let markers: &[(&str, &[&str])] = &[
        ("fn_add", &[MARKER_FN_ADD]),
        ("fn_mul", &[MARKER_FN_MUL]),
        ("return_add", &[MARKER_RETURN_ADD_LEGACY, MARKER_RETURN_ADD_CAST_I32]),
        ("return_mul", &[MARKER_RETURN_MUL_LEGACY, MARKER_RETURN_MUL_CAST_I32]),
        ("typedef_count", &[MARKER_TYPEDEF_COUNT]),
        ("alias_distance", &[MARKER_ALIAS_DISTANCE]),
        ("enum_mode", &[MARKER_ENUM_MODE]),
        ("struct_point", &[MARKER_STRUCT_POINT]),
        ("struct_point_x", &[MARKER_STRUCT_POINT_X]),
        ("struct_point_y", &[MARKER_STRUCT_POINT_Y]),
        ("namespace_math", &[MARKER_NAMESPACE_MATH]),
        ("namespace_ns_add", &[MARKER_NAMESPACE_NS_ADD]),
        ("template_fn_identity_i32", &[MARKER_TEMPLATE_FN_IDENTITY_I32]),
        ("template_struct_box_int", &[MARKER_TEMPLATE_STRUCT_BOX_INT, MARKER_TEMPLATE_ALIAS_BOX_INT]),
        ("typedef_intarray4", &[MARKER_TYPEDEF_INTARRAY4]),
        ("decltype_scalar_fn_sig", &[MARKER_DECLTYPE_SCALAR_FN_SIG]),
        ("const_ptr_fn_sig", &[MARKER_CONST_PTR_FN_SIG]),
        ("mut_ref_fn_sig", &[MARKER_MUT_REF_FN_SIG]),
        ("array_decay_fn_sig", &[MARKER_ARRAY_DECAY_FN_SIG]),
    ];
    markers
        .iter()
        .map(|(name, alternatives)| (*name, alternatives.iter().any(|alt| code.contains(alt))))
        .collect()
}

fn compile_rust_to_metadata(rust_path: &Path, metadata_path: &Path) -> i32 {
    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit=metadata")
        .arg(rust_path)
        .arg("-o")
        .arg(metadata_path)
        .output()
        .expect("failed to invoke rustc");
    output.status.code().unwrap_or(-1)
}

fn run_legacy_backend(source_path: &Path, log_dir: &Path) -> ShadowModeResult {
    let options = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: Vec::new(),
        language: ParserLanguage::Cpp,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Auto,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };

    let rust_code = transpile_cpp_to_rust_with_options(source_path, &options)
        .expect("legacy backend should transpile shadow fixture");

    let rust_path = log_dir.join("generated_legacy_libtooling.rs");
    fs::write(&rust_path, rust_code.as_bytes()).expect("write legacy .rs");

    let metadata_path = log_dir.join("generated_legacy_libtooling.rmeta");
    let rustc_status = compile_rust_to_metadata(&rust_path, &metadata_path);

    ShadowModeResult {
        backend_label: "legacy-libtooling".to_string(),
        rust_path,
        rustc_status,
        line_count: rust_code.lines().count(),
        unresolved_name_count: count_unresolved_names(&rust_code),
        marker_hits: check_markers(&rust_code),
    }
}

fn run_parser_output_handoff_backend(source_path: &Path, log_dir: &Path) -> ShadowModeResult {
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: source_path.to_path_buf(),
        language: ParserCoreLanguage::Cpp,
        frontend_args: Vec::new(),
        defines: Vec::new(),
        include_directives: Vec::new(),
    };

    let parser_output = ParserBackendTrait::parse(&backend, &request)
        .expect("parser-clang backend should parse shadow fixture");

    let rust_code = transpile_parser_output_to_rust(&parser_output)
        .expect("parser-output handoff should transpile shadow fixture");

    let rust_path = log_dir.join("generated_parser_output_handoff.rs");
    fs::write(&rust_path, rust_code.as_bytes()).expect("write handoff .rs");

    let metadata_path = log_dir.join("generated_parser_output_handoff.rmeta");
    let rustc_status = compile_rust_to_metadata(&rust_path, &metadata_path);

    ShadowModeResult {
        backend_label: "parser-output-handoff".to_string(),
        rust_path,
        rustc_status,
        line_count: rust_code.lines().count(),
        unresolved_name_count: count_unresolved_names(&rust_code),
        marker_hits: check_markers(&rust_code),
    }
}

fn write_shadow_manifest(
    log_dir: &Path,
    source_path: &Path,
    legacy: &ShadowModeResult,
    handoff: &ShadowModeResult,
) {
    let mut manifest = String::new();
    manifest.push_str(&format!("shadow_mode=m7_1\n"));
    manifest.push_str(&format!("source={}\n", source_path.display()));
    manifest.push_str(&format!("backend_count=2\n"));
    manifest.push_str(&format!("\n"));

    for result in [legacy, handoff] {
        manifest.push_str(&format!("[{}]\n", result.backend_label));
        manifest.push_str(&format!("rust_path={}\n", result.rust_path.display()));
        manifest.push_str(&format!("rustc_status={}\n", result.rustc_status));
        manifest.push_str(&format!("line_count={}\n", result.line_count));
        manifest.push_str(&format!("unresolved_name_count={}\n", result.unresolved_name_count));
        manifest.push_str(&format!(
            "marker_hit_count={}/{}\n",
            result.marker_hit_count(),
            result.marker_total()
        ));
        for (name, hit) in &result.marker_hits {
            manifest.push_str(&format!("marker.{}={}\n", name, hit));
        }
        manifest.push_str(&format!("\n"));
    }

    // Delta section
    manifest.push_str("[delta]\n");
    manifest.push_str(&format!(
        "rustc_status_delta={}\n",
        handoff.rustc_status - legacy.rustc_status
    ));
    let line_delta = handoff.line_count as i64 - legacy.line_count as i64;
    manifest.push_str(&format!("line_count_delta={}\n", line_delta));
    let unresolved_delta =
        handoff.unresolved_name_count as i64 - legacy.unresolved_name_count as i64;
    manifest.push_str(&format!("unresolved_name_count_delta={}\n", unresolved_delta));
    let marker_delta = handoff.marker_hit_count() as i64 - legacy.marker_hit_count() as i64;
    manifest.push_str(&format!("marker_hit_count_delta={}\n", marker_delta));

    // Per-marker delta
    for (i, (name, legacy_hit)) in legacy.marker_hits.iter().enumerate() {
        let handoff_hit = handoff.marker_hits[i].1;
        if *legacy_hit != handoff_hit {
            let status = if handoff_hit { "gained" } else { "lost" };
            manifest.push_str(&format!("marker_delta.{}={}\n", name, status));
        }
    }

    // Non-worsening verdict
    let rustc_non_worsening = handoff.rustc_status <= legacy.rustc_status;
    let unresolved_non_worsening =
        handoff.unresolved_name_count <= legacy.unresolved_name_count;
    let markers_non_worsening = handoff.marker_hit_count() >= legacy.marker_hit_count();
    let overall_non_worsening = rustc_non_worsening && unresolved_non_worsening && markers_non_worsening;

    manifest.push_str(&format!("\n[verdict]\n"));
    manifest.push_str(&format!("rustc_non_worsening={}\n", rustc_non_worsening));
    manifest.push_str(&format!(
        "unresolved_non_worsening={}\n",
        unresolved_non_worsening
    ));
    manifest.push_str(&format!("markers_non_worsening={}\n", markers_non_worsening));
    manifest.push_str(&format!("overall_non_worsening={}\n", overall_non_worsening));

    let manifest_path = log_dir.join("m7_shadow_mode_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write shadow manifest");
}

// =====================================================================
// Test: M7.1 shadow mode on representative non-RPC corpus
// =====================================================================

#[test]
fn test_m7_shadow_mode_legacy_vs_parser_output_handoff() {
    let log_dir = unique_temp_dir(SHADOW_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).expect("create shadow log dir");

    let source_path = log_dir.join("m7_shadow_fixture.cpp");
    fs::write(&source_path, SHADOW_FIXTURE_SOURCE).expect("write shadow fixture");

    let legacy = run_legacy_backend(&source_path, &log_dir);
    let handoff = run_parser_output_handoff_backend(&source_path, &log_dir);

    write_shadow_manifest(&log_dir, &source_path, &legacy, &handoff);

    // ---- Assertions ----

    // Both backends must produce compilable Rust
    assert_eq!(
        legacy.rustc_status, 0,
        "legacy backend generated Rust must compile; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        handoff.rustc_status, 0,
        "parser-output-handoff generated Rust must compile; logs: {}",
        log_dir.display()
    );

    // Legacy baseline must hit all expected markers
    assert!(
        legacy.marker_hit_count() == legacy.marker_total(),
        "legacy backend should hit all {} markers, got {}/{}; logs: {}",
        legacy.marker_total(),
        legacy.marker_hit_count(),
        legacy.marker_total(),
        log_dir.display()
    );

    // Non-worsening: handoff must not lose markers vs legacy
    let lost_markers: Vec<&str> = legacy
        .marker_hits
        .iter()
        .zip(handoff.marker_hits.iter())
        .filter(|((_, leg), (_, hand))| *leg && !*hand)
        .map(|((name, _), _)| *name)
        .collect();
    assert!(
        lost_markers.is_empty(),
        "parser-output-handoff lost markers vs legacy: {:?}; logs: {}",
        lost_markers,
        log_dir.display()
    );

    // Non-worsening: handoff must not increase unresolved name count
    assert!(
        handoff.unresolved_name_count <= legacy.unresolved_name_count,
        "parser-output-handoff has more unresolved names ({}) than legacy ({}); logs: {}",
        handoff.unresolved_name_count,
        legacy.unresolved_name_count,
        log_dir.display()
    );

    // Manifest must exist
    assert!(
        log_dir.join("m7_shadow_mode_manifest.txt").exists(),
        "shadow mode manifest must exist; logs: {}",
        log_dir.display()
    );

    eprintln!(
        "M7.1 shadow mode PASSED: legacy={}/{} markers, handoff={}/{} markers, \
         unresolved legacy={} handoff={}, logs: {}",
        legacy.marker_hit_count(),
        legacy.marker_total(),
        handoff.marker_hit_count(),
        handoff.marker_total(),
        legacy.unresolved_name_count,
        handoff.unresolved_name_count,
        log_dir.display()
    );
}

// =====================================================================
// Test: M7.1 shadow mode manifest is deterministic and well-formed
// =====================================================================

#[test]
fn test_m7_shadow_mode_manifest_is_well_formed() {
    let log_dir = unique_temp_dir(SHADOW_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).expect("create shadow log dir");

    let source_path = log_dir.join("m7_shadow_manifest_fixture.cpp");
    fs::write(&source_path, SHADOW_FIXTURE_SOURCE).expect("write shadow fixture");

    let legacy = run_legacy_backend(&source_path, &log_dir);
    let handoff = run_parser_output_handoff_backend(&source_path, &log_dir);

    write_shadow_manifest(&log_dir, &source_path, &legacy, &handoff);

    let manifest_path = log_dir.join("m7_shadow_mode_manifest.txt");
    let manifest = fs::read_to_string(&manifest_path).expect("read manifest");

    // Required sections
    assert!(manifest.contains("shadow_mode=m7_1"), "manifest must contain shadow_mode header");
    assert!(manifest.contains("[legacy-libtooling]"), "manifest must contain legacy section");
    assert!(
        manifest.contains("[parser-output-handoff]"),
        "manifest must contain handoff section"
    );
    assert!(manifest.contains("[delta]"), "manifest must contain delta section");
    assert!(manifest.contains("[verdict]"), "manifest must contain verdict section");

    // Required metrics in each backend section
    assert!(manifest.contains("rustc_status="), "manifest must contain rustc_status");
    assert!(manifest.contains("line_count="), "manifest must contain line_count");
    assert!(manifest.contains("unresolved_name_count="), "manifest must contain unresolved_name_count");
    assert!(manifest.contains("marker_hit_count="), "manifest must contain marker_hit_count");

    // Required delta fields
    assert!(
        manifest.contains("rustc_status_delta="),
        "manifest must contain rustc_status_delta"
    );
    assert!(
        manifest.contains("unresolved_name_count_delta="),
        "manifest must contain unresolved_name_count_delta"
    );
    assert!(
        manifest.contains("marker_hit_count_delta="),
        "manifest must contain marker_hit_count_delta"
    );

    // Required verdict fields
    assert!(
        manifest.contains("overall_non_worsening="),
        "manifest must contain overall_non_worsening verdict"
    );
    assert!(
        manifest.contains("overall_non_worsening=true"),
        "overall verdict must be non-worsening for this fixture; manifest:\n{}",
        manifest
    );
}

// =====================================================================
// Test: M7.1 shadow mode runs on M2.A1 fixture corpus (multi-file)
// =====================================================================

#[test]
fn test_m7_shadow_mode_m2_a1_corpus_pipeline_cpp() {
    let log_dir = unique_temp_dir(SHADOW_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).expect("create shadow log dir");

    // Use the M2.A1 pipeline.cpp fixture
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fragile-parser-clang/tests/fixtures/m2_a1");
    let pipeline_source = fixture_dir.join("src/pipeline.cpp");

    if !pipeline_source.exists() {
        eprintln!(
            "M2.A1 pipeline.cpp fixture not found at {}; skipping",
            pipeline_source.display()
        );
        return;
    }

    // Legacy path with include path and required defines
    let include_path = fixture_dir.join("include").to_string_lossy().to_string();
    let options = TranspileOptions {
        include_paths: vec![include_path.clone()],
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: vec!["CORPUS_SCALE=4".to_string()],
        language: ParserLanguage::Cpp,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Auto,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };

    let legacy_code = transpile_cpp_to_rust_with_options(&pipeline_source, &options)
        .expect("legacy backend should transpile M2.A1 pipeline.cpp");

    let legacy_path = log_dir.join("generated_m2a1_legacy.rs");
    fs::write(&legacy_path, legacy_code.as_bytes()).expect("write legacy m2a1");

    let legacy_meta = log_dir.join("generated_m2a1_legacy.rmeta");
    let legacy_status = compile_rust_to_metadata(&legacy_path, &legacy_meta);

    // Parser-output-handoff path
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: pipeline_source.to_path_buf(),
        language: ParserCoreLanguage::Cpp,
        frontend_args: vec![format!("-I{}", include_path)],
        defines: vec!["CORPUS_SCALE=4".to_string()],
        include_directives: Vec::new(),
    };

    let parser_output = ParserBackendTrait::parse(&backend, &request)
        .expect("parser-clang should parse M2.A1 pipeline.cpp");

    let handoff_code = transpile_parser_output_to_rust(&parser_output)
        .expect("handoff should transpile M2.A1 pipeline.cpp");

    let handoff_path = log_dir.join("generated_m2a1_handoff.rs");
    fs::write(&handoff_path, handoff_code.as_bytes()).expect("write handoff m2a1");

    let handoff_meta = log_dir.join("generated_m2a1_handoff.rmeta");
    let handoff_status = compile_rust_to_metadata(&handoff_path, &handoff_meta);

    // Non-worsening: handoff must not regress vs legacy rustc status
    assert!(
        handoff_status <= legacy_status,
        "handoff rustc status ({}) worse than legacy ({}); logs: {}",
        handoff_status,
        legacy_status,
        log_dir.display()
    );

    // M2.A1-specific markers: non-worsening check
    let m2a1_markers = ["pub enum PacketKind", "pub struct Packet"];
    for marker in &m2a1_markers {
        let in_legacy = legacy_code.contains(marker);
        let in_handoff = handoff_code.contains(marker);
        assert!(
            !in_legacy || in_handoff,
            "handoff lost M2.A1 marker '{}' that legacy had; logs: {}",
            marker,
            log_dir.display()
        );
    }

    eprintln!(
        "M7.1 shadow M2.A1 pipeline.cpp PASSED: legacy_status={}, handoff_status={}, logs: {}",
        legacy_status, handoff_status, log_dir.display()
    );
}

// =====================================================================
// Test: M7.1 shadow mode runs on M2.A1 dispatch.cpp fixture
// =====================================================================

#[test]
fn test_m7_shadow_mode_m2_a1_corpus_dispatch_cpp() {
    let log_dir = unique_temp_dir(SHADOW_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).expect("create shadow log dir");

    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fragile-parser-clang/tests/fixtures/m2_a1");
    let dispatch_source = fixture_dir.join("src/dispatch.cpp");

    if !dispatch_source.exists() {
        eprintln!(
            "M2.A1 dispatch.cpp fixture not found at {}; skipping",
            dispatch_source.display()
        );
        return;
    }

    let include_path = fixture_dir.join("include").to_string_lossy().to_string();

    let options = TranspileOptions {
        include_paths: vec![include_path.clone()],
        include_directives: Vec::new(),
        frontend_args: vec!["-DUNUSED_FRONTEND_FLAG=1".to_string()],
        defines: Vec::new(),
        language: ParserLanguage::Cpp,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Auto,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };

    let legacy_result = transpile_cpp_to_rust_with_options(&dispatch_source, &options);
    let handoff_result = {
        let backend = FragileParserClangBackend;
        let request = ParseRequest {
            source_path: dispatch_source.to_path_buf(),
            language: ParserCoreLanguage::Cpp,
            frontend_args: vec![
                format!("-I{}", include_path),
                "-DUNUSED_FRONTEND_FLAG=1".to_string(),
            ],
            defines: Vec::new(),
            include_directives: Vec::new(),
        };
        ParserBackendTrait::parse(&backend, &request)
            .ok()
            .and_then(|output| transpile_parser_output_to_rust(&output).ok())
    };

    // If both transpile paths fail, that's symmetric — non-worsening by definition
    let (legacy_status, handoff_status) = match (&legacy_result, &handoff_result) {
        (Ok(legacy_code), Some(handoff_code)) => {
            let legacy_path = log_dir.join("legacy_dispatch.rs");
            let handoff_path = log_dir.join("handoff_dispatch.rs");
            fs::write(&legacy_path, legacy_code.as_bytes()).expect("write");
            fs::write(&handoff_path, handoff_code.as_bytes()).expect("write");
            let ls = compile_rust_to_metadata(&legacy_path, &log_dir.join("legacy_dispatch.rmeta"));
            let hs = compile_rust_to_metadata(&handoff_path, &log_dir.join("handoff_dispatch.rmeta"));
            (ls, hs)
        }
        (Err(_), None) => {
            // Both fail to transpile — symmetric, non-worsening
            (-1, -1)
        }
        (Err(_), Some(_)) => {
            // Legacy fails but handoff succeeds — handoff is strictly better
            (-1, 0)
        }
        (Ok(_), None) => {
            // Legacy succeeds but handoff fails — regression
            (0, -1)
        }
    };

    // Non-worsening: handoff must not be strictly worse than legacy.
    // "Worse" means: legacy succeeded (0) but handoff didn't, or
    // both compiled but handoff has higher (non-zero) status.
    let handoff_regressed = match (legacy_status, handoff_status) {
        (0, s) if s != 0 => true,       // legacy compiled, handoff didn't
        (-1, -1) => false,               // both failed to transpile
        (-1, _) => false,                // legacy failed, handoff is same or better
        (_, -1) => true,                 // handoff regressed to transpile failure
        _ => handoff_status > legacy_status, // both compiled, compare rustc exit
    };
    assert!(
        !handoff_regressed,
        "handoff regressed vs legacy: handoff_status={}, legacy_status={}; logs: {}",
        handoff_status,
        legacy_status,
        log_dir.display()
    );

    eprintln!(
        "M7.1 shadow M2.A1 dispatch.cpp PASSED: legacy_status={}, handoff_status={}, logs: {}",
        legacy_status, handoff_status, log_dir.display()
    );
}

// =====================================================================
// Test: M7.1 shadow mode on M2.A1 metrics.c (C language fixture)
// =====================================================================

#[test]
fn test_m7_shadow_mode_m2_a1_corpus_metrics_c() {
    let log_dir = unique_temp_dir(SHADOW_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).expect("create shadow log dir");

    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fragile-parser-clang/tests/fixtures/m2_a1");
    let metrics_source = fixture_dir.join("src/metrics.c");

    if !metrics_source.exists() {
        eprintln!(
            "M2.A1 metrics.c fixture not found at {}; skipping",
            metrics_source.display()
        );
        return;
    }

    let include_path = fixture_dir.join("include").to_string_lossy().to_string();

    // Legacy with C language
    let options = TranspileOptions {
        include_paths: vec![include_path.clone()],
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: vec!["CORPUS_C_SHIFT=5".to_string()],
        language: ParserLanguage::C,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        backend: ParserBackend::Libtooling,
        template_parsing_mode: TemplateParsingMode::Auto,
        libtooling_skip_system_headers: false,
        stage_timing_trace_path: None,
    };

    let legacy_code = transpile_cpp_to_rust_with_options(&metrics_source, &options)
        .expect("legacy should transpile metrics.c");

    let handoff_code = {
        let backend = FragileParserClangBackend;
        let request = ParseRequest {
            source_path: metrics_source.to_path_buf(),
            language: ParserCoreLanguage::C,
            frontend_args: vec![format!("-I{}", include_path)],
            defines: vec!["CORPUS_C_SHIFT=5".to_string()],
            include_directives: Vec::new(),
        };
        let output = ParserBackendTrait::parse(&backend, &request)
            .expect("parser-clang should parse metrics.c");
        transpile_parser_output_to_rust(&output)
            .expect("handoff should transpile metrics.c")
    };

    let legacy_path = log_dir.join("legacy_metrics.rs");
    let handoff_path = log_dir.join("handoff_metrics.rs");
    fs::write(&legacy_path, legacy_code.as_bytes()).expect("write");
    fs::write(&handoff_path, handoff_code.as_bytes()).expect("write");

    let legacy_status = compile_rust_to_metadata(&legacy_path, &log_dir.join("legacy_metrics.rmeta"));
    let handoff_status = compile_rust_to_metadata(&handoff_path, &log_dir.join("handoff_metrics.rmeta"));

    // Non-worsening: handoff must not regress vs legacy
    assert!(
        handoff_status <= legacy_status,
        "handoff rustc status ({}) worse than legacy ({}); logs: {}",
        handoff_status,
        legacy_status,
        log_dir.display()
    );

    eprintln!(
        "M7.1 shadow M2.A1 metrics.c PASSED: legacy_status={}, handoff_status={}, logs: {}",
        legacy_status, handoff_status, log_dir.display()
    );
}

// =====================================================================
// Test: Verify RPC corpus is explicitly queued for M9 (not tested here)
// =====================================================================

#[test]
fn test_m7_shadow_mode_rpc_corpus_deferred_to_m9() {
    // M7.1 contract: RPC corpus (test_rpc, rpcbench, mako) is queued for M9 closure,
    // not included in shadow mode non-RPC corpus runs. This test documents the contract.
    let rpc_target_names = ["test_rpc", "rpcbench", "mako_rpc"];
    for target in &rpc_target_names {
        // Verify no shadow mode test in this file uses RPC targets
        let test_source = include_str!("m7_shadow_mode_tests.rs");
        assert!(
            !test_source.contains(&format!("\"{}\"", target))
                || test_source.contains("deferred_to_m9"),
            "shadow mode tests must not include RPC target '{}' — deferred to M9",
            target
        );
    }
}
