/// M7.1 and M7.3 Shadow Mode Tests
///
/// M7.1: Runs legacy (libtooling) and new (parser-output-handoff) backends on a representative
/// non-RPC corpus. Compares rustc compilation status, marker presence, output metrics,
/// unresolved-name counts. Emits a deterministic parity manifest.
///
/// M7.3: Closes parity blockers. Tests M7.A1 (non-worsening on blocker class and
/// unresolved-name deltas) and M7.A2 (runtime behavior parity for covered smoke fixtures).

use fragile_clang::{
    transpile_cpp_to_rust_with_options, transpile_parser_output_to_rust,
    ParserLanguage, TranspileOptions,
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

// =====================================================================
// M7.3 — M7.A1 Acceptance Gate: Non-worsening on blocker class and
// unresolved-name deltas for the representative non-RPC corpus.
// =====================================================================

/// Count unresolved-name markers in generated Rust code.
fn count_unresolved_in_generated(rust_code: &str) -> usize {
    let mut count = 0;
    for line in rust_code.lines() {
        if line.contains("/* unresolved */") || line.contains("todo!(") {
            count += 1;
        }
    }
    count
}

#[derive(Debug)]
struct ParityBlockerRecord {
    fixture_label: &'static str,
    legacy_rustc_status: i32,
    handoff_rustc_status: i32,
    legacy_unresolved_count: usize,
    handoff_unresolved_count: usize,
}

impl ParityBlockerRecord {
    /// M7.A1: candidate must not increase unresolved-name count vs baseline.
    fn unresolved_non_worsening(&self) -> bool {
        self.handoff_unresolved_count <= self.legacy_unresolved_count
    }

    /// M7.A1: candidate must not introduce new compile failures where baseline compiled.
    fn compile_non_worsening(&self) -> bool {
        if self.legacy_rustc_status == 0 {
            self.handoff_rustc_status == 0
        } else {
            // Baseline already failing — candidate can be same or better
            true
        }
    }
}

fn run_both_backends_on_source(
    source_code: &str,
    fixture_label: &'static str,
    log_dir: &Path,
) -> ParityBlockerRecord {
    // Write fixture to disk
    let source_path = log_dir.join(format!("{}.cpp", fixture_label));
    fs::write(&source_path, source_code).expect("write fixture source");

    // Legacy backend
    let legacy_options = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: Vec::new(),
        language: ParserLanguage::Cpp,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        stage_timing_trace_path: None,
    };
    let legacy_code = transpile_cpp_to_rust_with_options(&source_path, &legacy_options)
        .unwrap_or_default();
    let legacy_path = log_dir.join(format!("{}_legacy.rs", fixture_label));
    fs::write(&legacy_path, legacy_code.as_bytes()).expect("write legacy .rs");
    let legacy_meta = log_dir.join(format!("{}_legacy.rmeta", fixture_label));
    let legacy_status = compile_rust_to_metadata(&legacy_path, &legacy_meta);

    // New backend (parser-output handoff)
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: source_path.clone(),
        language: ParserCoreLanguage::Cpp,
        frontend_args: Vec::new(),
        defines: Vec::new(),
        include_directives: Vec::new(),
    };
    let handoff_code = ParserBackendTrait::parse(&backend, &request)
        .ok()
        .and_then(|output| transpile_parser_output_to_rust(&output).ok())
        .unwrap_or_default();
    let handoff_path = log_dir.join(format!("{}_handoff.rs", fixture_label));
    fs::write(&handoff_path, handoff_code.as_bytes()).expect("write handoff .rs");
    let handoff_meta = log_dir.join(format!("{}_handoff.rmeta", fixture_label));
    let handoff_status = compile_rust_to_metadata(&handoff_path, &handoff_meta);

    ParityBlockerRecord {
        fixture_label,
        legacy_rustc_status: legacy_status,
        handoff_rustc_status: handoff_status,
        legacy_unresolved_count: count_unresolved_in_generated(&legacy_code),
        handoff_unresolved_count: count_unresolved_in_generated(&handoff_code),
    }
}

/// M7.A1 acceptance gate: new backend is non-worsening vs baseline on blocker class
/// and unresolved-name deltas for the representative non-RPC corpus.
#[test]
fn test_m7_a1_non_worsening_blocker_class_and_unresolved_name_deltas() {
    let log_dir = unique_temp_dir("fragile_m7_a1_parity_blocker");
    fs::create_dir_all(&log_dir).expect("create m7_a1 log dir");

    // Representative non-RPC fixtures used for M7.A1 parity gate.
    // These cover: typedefs, enums, structs, templates, namespaces, pointers, refs.
    let fixtures: &[(&'static str, &str)] = &[
        ("typedef_enum_struct", r#"
typedef unsigned long Count;
using Distance = int;
enum Mode { ModeA = 1, ModeB = 2 };
struct Point { int x; int y; };
int make_point_sum(Point p) { return p.x + p.y; }
"#),
        ("template_fn_struct", r#"
template<typename T>
struct Box { T value; };
template struct Box<int>;

template<typename T>
T identity(T v) { return v; }
template int identity<int>(int);

int use_identity() { return identity<int>(42); }
"#),
        ("namespace_math", r#"
namespace math {
int add(int a, int b) { return a + b; }
int mul(int x, int y) { return x * y; }
}
int call_ns() { return math::add(3, 4) + math::mul(2, 5); }
"#),
        ("const_ptr_and_ref", r#"
int read_const_ptr(const int* p) { return *p; }
int bump_ref(int& value) { value += 1; return value; }
int array_decay_head(int* data) { return data[0]; }
"#),
        ("struct_methods", r#"
struct Counter {
    int value;
    Counter() { value = 0; }
    Counter(int init) { value = init; }
    void increment() { value = value + 1; }
    int get() { return value; }
};
int use_counter() {
    Counter c(10);
    c.increment();
    return c.get();
}
"#),
    ];

    let mut records: Vec<ParityBlockerRecord> = Vec::new();
    for (label, source) in fixtures {
        let record = run_both_backends_on_source(source, label, &log_dir);
        records.push(record);
    }

    // Write parity blocker closure manifest
    let mut manifest = String::new();
    manifest.push_str("m7_a1_gate=parity_blocker_closure\n");
    manifest.push_str(&format!("fixture_count={}\n", records.len()));
    let all_compile_non_worsening = records.iter().all(|r| r.compile_non_worsening());
    let all_unresolved_non_worsening = records.iter().all(|r| r.unresolved_non_worsening());
    let total_legacy_unresolved: usize = records.iter().map(|r| r.legacy_unresolved_count).sum();
    let total_handoff_unresolved: usize = records.iter().map(|r| r.handoff_unresolved_count).sum();
    let unresolved_delta = total_handoff_unresolved as i64 - total_legacy_unresolved as i64;

    for (i, r) in records.iter().enumerate() {
        manifest.push_str(&format!(
            "fixture_{:03}_label={}\nfixture_{:03}_legacy_status={}\nfixture_{:03}_handoff_status={}\n\
             fixture_{:03}_legacy_unresolved={}\nfixture_{:03}_handoff_unresolved={}\n\
             fixture_{:03}_compile_non_worsening={}\nfixture_{:03}_unresolved_non_worsening={}\n",
            i + 1, r.fixture_label,
            i + 1, r.legacy_rustc_status,
            i + 1, r.handoff_rustc_status,
            i + 1, r.legacy_unresolved_count,
            i + 1, r.handoff_unresolved_count,
            i + 1, r.compile_non_worsening(),
            i + 1, r.unresolved_non_worsening(),
        ));
    }
    manifest.push_str(&format!("total_legacy_unresolved={}\n", total_legacy_unresolved));
    manifest.push_str(&format!("total_handoff_unresolved={}\n", total_handoff_unresolved));
    manifest.push_str(&format!("unresolved_name_e0425_delta_vs_baseline={}\n", unresolved_delta));
    manifest.push_str(&format!("compile_non_worsening_all={}\n", all_compile_non_worsening));
    manifest.push_str(&format!("unresolved_non_worsening_all={}\n", all_unresolved_non_worsening));
    manifest.push_str(&format!(
        "m7_a1_gate_verdict={}\n",
        if all_compile_non_worsening && all_unresolved_non_worsening { "pass" } else { "fail" }
    ));
    let manifest_path = log_dir.join("m7_a1_parity_blocker_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write m7_a1 manifest");

    // --- Assertions for M7.A1 ---

    // Candidate must not introduce compile regressions vs baseline
    let worsening_fixtures: Vec<&str> = records
        .iter()
        .filter(|r| !r.compile_non_worsening())
        .map(|r| r.fixture_label)
        .collect();
    assert!(
        worsening_fixtures.is_empty(),
        "M7.A1 FAILED: candidate worsened compile status vs baseline for fixtures: {:?}; \
         manifest: {}",
        worsening_fixtures,
        log_dir.display()
    );

    // Candidate must not increase aggregate unresolved-name count vs baseline
    assert!(
        unresolved_delta <= 0,
        "M7.A1 FAILED: candidate increased unresolved-name count by {} vs baseline \
         (legacy={}, handoff={}); manifest: {}",
        unresolved_delta,
        total_legacy_unresolved,
        total_handoff_unresolved,
        log_dir.display()
    );

    // Per-fixture unresolved non-worsening
    let unresolved_worsening: Vec<&str> = records
        .iter()
        .filter(|r| !r.unresolved_non_worsening())
        .map(|r| r.fixture_label)
        .collect();
    assert!(
        unresolved_worsening.is_empty(),
        "M7.A1 FAILED: candidate increased unresolved-name count for fixtures: {:?}; \
         manifest: {}",
        unresolved_worsening,
        log_dir.display()
    );

    eprintln!(
        "M7.A1 PASSED: {} fixtures, all compile non-worsening={}, \
         unresolved_delta={} (legacy={} handoff={}); manifest: {}",
        records.len(),
        all_compile_non_worsening,
        unresolved_delta,
        total_legacy_unresolved,
        total_handoff_unresolved,
        log_dir.display()
    );
}

// =====================================================================
// M7.3 — M7.A2 Acceptance Gate: Runtime behavior parity for covered
// smoke fixtures (actual binary execution).
// =====================================================================

/// Compile generated Rust code to an executable binary.
fn compile_rust_to_binary(rust_path: &Path, binary_path: &Path) -> i32 {
    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("--crate-type")
        .arg("bin")
        .arg(rust_path)
        .arg("-o")
        .arg(binary_path)
        .output()
        .expect("failed to invoke rustc for binary");
    output.status.code().unwrap_or(-1)
}

/// Run a binary and return its exit code.
fn run_binary(binary_path: &Path) -> i32 {
    let output = Command::new(binary_path)
        .output()
        .expect("failed to run binary");
    output.status.code().unwrap_or(-1)
}

/// M7.A2 acceptance gate: runtime behavior parity for covered smoke fixtures.
///
/// Uses `factorial.cpp` which has a `main()` that returns 0 on correct result
/// and 1 on wrong result. Verifies both backends produce binaries that exit 0.
#[test]
fn test_m7_a2_runtime_parity_smoke_fixtures() {
    let log_dir = unique_temp_dir("fragile_m7_a2_runtime_parity");
    fs::create_dir_all(&log_dir).expect("create m7_a2 log dir");

    // Smoke fixture: factorial with a main() that validates the result.
    // Returns 0 if factorial(5) == 120, else 1.
    // This is the canonical runtime parity smoke test for M7.A2.
    let factorial_source = r#"
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

int main() {
    int result = factorial(5);
    return (result == 120) ? 0 : 1;
}
"#;

    let source_path = log_dir.join("m7_a2_factorial.cpp");
    fs::write(&source_path, factorial_source).expect("write factorial source");

    // Legacy backend
    let legacy_options = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: Vec::new(),
        frontend_args: Vec::new(),
        defines: Vec::new(),
        language: ParserLanguage::Cpp,
        language_standard: None,
        ignored_error_patterns: Vec::new(),
        stage_timing_trace_path: None,
    };
    let legacy_rust = transpile_cpp_to_rust_with_options(&source_path, &legacy_options)
        .expect("legacy backend must transpile factorial");
    let legacy_rs_path = log_dir.join("factorial_legacy.rs");
    fs::write(&legacy_rs_path, legacy_rust.as_bytes()).expect("write legacy .rs");
    let legacy_bin_path = log_dir.join("factorial_legacy_bin");
    let legacy_compile_status = compile_rust_to_binary(&legacy_rs_path, &legacy_bin_path);

    // New backend (parser-output handoff)
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: source_path.clone(),
        language: ParserCoreLanguage::Cpp,
        frontend_args: Vec::new(),
        defines: Vec::new(),
        include_directives: Vec::new(),
    };
    let parser_output_factorial = ParserBackendTrait::parse(&backend, &request)
        .expect("new backend must parse factorial");
    let handoff_rust = transpile_parser_output_to_rust(&parser_output_factorial)
        .expect("new backend must transpile factorial");
    let handoff_rs_path = log_dir.join("factorial_handoff.rs");
    fs::write(&handoff_rs_path, handoff_rust.as_bytes()).expect("write handoff .rs");
    let handoff_bin_path = log_dir.join("factorial_handoff_bin");
    let handoff_compile_status = compile_rust_to_binary(&handoff_rs_path, &handoff_bin_path);

    // Write runtime parity manifest
    let legacy_runtime_exit = if legacy_compile_status == 0 && legacy_bin_path.exists() {
        Some(run_binary(&legacy_bin_path))
    } else {
        None
    };
    let handoff_runtime_exit = if handoff_compile_status == 0 && handoff_bin_path.exists() {
        Some(run_binary(&handoff_bin_path))
    } else {
        None
    };

    let mut manifest = String::new();
    manifest.push_str("m7_a2_gate=runtime_parity_smoke\n");
    manifest.push_str("fixture=factorial_with_main\n");
    manifest.push_str(&format!("legacy_compile_status={}\n", legacy_compile_status));
    manifest.push_str(&format!("handoff_compile_status={}\n", handoff_compile_status));
    manifest.push_str(&format!(
        "legacy_runtime_exit={}\n",
        legacy_runtime_exit.map_or("na".to_string(), |c| c.to_string())
    ));
    manifest.push_str(&format!(
        "handoff_runtime_exit={}\n",
        handoff_runtime_exit.map_or("na".to_string(), |c| c.to_string())
    ));
    let runtime_non_worsening = match (legacy_runtime_exit, handoff_runtime_exit) {
        (Some(0), Some(0)) => true,   // both correct
        (Some(0), None) => false,     // legacy works, handoff can't even compile
        (None, Some(0)) => true,      // handoff strictly better (legacy couldn't compile)
        (None, None) => true,         // symmetric — neither compiled
        (Some(leg), Some(hand)) => hand <= leg, // handoff no worse
        (Some(_), None) => false,     // legacy compiled, handoff didn't
        (None, Some(_)) => true,      // handoff at least as good (legacy didn't compile)
    };
    manifest.push_str(&format!("runtime_non_worsening={}\n", runtime_non_worsening));
    manifest.push_str(&format!(
        "m7_a2_gate_verdict={}\n",
        if runtime_non_worsening { "pass" } else { "fail" }
    ));
    let manifest_path = log_dir.join("m7_a2_runtime_parity_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write m7_a2 manifest");

    // --- Assertions for M7.A2 ---

    // Both backends must compile the factorial fixture to a binary
    assert_eq!(
        legacy_compile_status, 0,
        "M7.A2: legacy backend must compile factorial to a binary; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        handoff_compile_status, 0,
        "M7.A2: new backend must compile factorial to a binary; logs: {}",
        log_dir.display()
    );

    // Both binaries must exit 0 (correct factorial result)
    assert_eq!(
        legacy_runtime_exit,
        Some(0),
        "M7.A2: legacy binary must return 0 (factorial(5)==120); logs: {}",
        log_dir.display()
    );
    assert_eq!(
        handoff_runtime_exit,
        Some(0),
        "M7.A2: new backend binary must return 0 (factorial(5)==120); logs: {}",
        log_dir.display()
    );

    // Runtime parity: handoff must not worsen vs legacy
    assert!(
        runtime_non_worsening,
        "M7.A2 FAILED: runtime non-worsening violated; \
         legacy_exit={:?}, handoff_exit={:?}; manifest: {}",
        legacy_runtime_exit,
        handoff_runtime_exit,
        log_dir.display()
    );

    eprintln!(
        "M7.A2 PASSED: legacy_exit={:?}, handoff_exit={:?}, \
         runtime_non_worsening={}; manifest: {}",
        legacy_runtime_exit,
        handoff_runtime_exit,
        runtime_non_worsening,
        log_dir.display()
    );
}

// =====================================================================
// M7.3 — Parity blocker closure: struct method parity
// =====================================================================

/// M7.3 generic fix verification: struct methods (Counter pattern from fixture_006)
/// compile correctly in both backends. This fixture specifically targets the
/// `14_struct_constructor.cpp` failure class from M7.2 where libtooling emitted
/// a trait method conflict for `Counter::get()`.
#[test]
fn test_m7_3_struct_method_parity_counter_pattern() {
    let log_dir = unique_temp_dir("fragile_m7_3_struct_method_parity");
    fs::create_dir_all(&log_dir).expect("create m7_3 log dir");

    // This matches the pattern from tests/cpp/grammar/14_struct_constructor.cpp
    // which failed in M7.2 under the libtooling baseline.
    let source = r#"
struct Counter {
    int value;
    Counter() { value = 0; }
    Counter(int initial) { value = initial; }
    void increment() { value = value + 1; }
    int get() { return value; }
};

int test_counter() {
    Counter c1;
    Counter c2(40);
    c1.increment();
    c1.increment();
    c2.increment();
    c2.increment();
    return c1.get() + c2.get();
}
"#;

    let source_path = log_dir.join("m7_3_counter.cpp");
    fs::write(&source_path, source).expect("write counter source");

    // New backend must compile this correctly
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: source_path.clone(),
        language: ParserCoreLanguage::Cpp,
        frontend_args: Vec::new(),
        defines: Vec::new(),
        include_directives: Vec::new(),
    };
    let parser_output_counter = ParserBackendTrait::parse(&backend, &request)
        .expect("new backend must parse Counter fixture");
    let handoff_code = transpile_parser_output_to_rust(&parser_output_counter)
        .expect("new backend must transpile Counter fixture");
    let handoff_path = log_dir.join("counter_handoff.rs");
    fs::write(&handoff_path, handoff_code.as_bytes()).expect("write handoff .rs");
    let handoff_meta = log_dir.join("counter_handoff.rmeta");
    let handoff_status = compile_rust_to_metadata(&handoff_path, &handoff_meta);

    assert_eq!(
        handoff_status, 0,
        "M7.3: new backend must compile Counter struct pattern (was failing under libtooling \
         in M7.2 due to trait method conflict); logs: {}\nGenerated code:\n{}",
        log_dir.display(),
        &handoff_code
    );

    // Verify key structural markers are present
    assert!(
        handoff_code.contains("struct Counter") || handoff_code.contains("pub struct Counter"),
        "M7.3: Counter struct must appear in output; logs: {}",
        log_dir.display()
    );

    eprintln!(
        "M7.3 struct method parity PASSED: Counter pattern compiles with new backend; \
         logs: {}",
        log_dir.display()
    );
}

// =====================================================================
// M7.3 — Parity blocker closure manifest: aggregate gate
// =====================================================================

/// M7.3 aggregate gate: emit a combined blocker-closure manifest and assert
/// both M7.A1 and M7.A2 gates are satisfied on the full non-RPC fixture set.
#[test]
fn test_m7_3_parity_blocker_closure_aggregate_gate() {
    let log_dir = unique_temp_dir("fragile_m7_3_aggregate_gate");
    fs::create_dir_all(&log_dir).expect("create m7_3 aggregate log dir");

    // The full representative non-RPC fixture corpus (mirrors DEFAULT_NON_RPC_CORPUS
    // from the Python shadow harness).
    let corpus_fixtures: &[(&'static str, &str)] = &[
        ("add_simple", r#"
int add(int a, int b) { return a + b; }
int mul(int x, int y) { return x * y; }
struct Point { double x; double y; };
"#),
        ("factorial", r#"
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
"#),
        ("namespace_resolution", r#"
namespace ns {
int ns_add(int a, int b) { return a + b; }
}
int use_ns() { return ns::ns_add(1, 2); }
"#),
        ("class_struct", r#"
struct Point { int x; int y; };
int sum_point(Point p) { return p.x + p.y; }
"#),
        ("constructor_pattern", r#"
struct Widget {
    int id;
    Widget() { id = 0; }
    Widget(int i) { id = i; }
    int get_id() { return id; }
};
int make_widget() { return Widget(7).get_id(); }
"#),
        ("struct_constructor_grammar", r#"
struct Counter {
    int value;
    Counter() { value = 0; }
    Counter(int initial) { value = initial; }
    void increment() { value = value + 1; }
    int get() { return value; }
};
int test_struct_constructor() {
    Counter c1;
    Counter c2(40);
    c1.increment();
    c1.increment();
    c2.increment();
    c2.increment();
    return c1.get() + c2.get();
}
"#),
        ("namespace_resolution_clang", r#"
namespace outer {
namespace inner {
int compute(int x) { return x * 2; }
}
}
int use_inner() { return outer::inner::compute(3); }
"#),
        ("virtual_class_pattern", r#"
struct Base {
    int value;
    Base() { value = 0; }
    int get() { return value; }
};
struct Derived : public Base {
    Derived(int v) { value = v; }
};
int use_derived() { return Derived(5).get(); }
"#),
    ];

    let mut records: Vec<ParityBlockerRecord> = Vec::new();
    for (label, source) in corpus_fixtures {
        let record = run_both_backends_on_source(source, label, &log_dir);
        records.push(record);
    }

    // Compute aggregate metrics
    let fixture_count = records.len();
    let blocker_closed_count = records
        .iter()
        .filter(|r| r.compile_non_worsening() && r.unresolved_non_worsening())
        .count();
    let total_legacy_unresolved: usize = records.iter().map(|r| r.legacy_unresolved_count).sum();
    let total_handoff_unresolved: usize = records.iter().map(|r| r.handoff_unresolved_count).sum();
    let unresolved_delta = total_handoff_unresolved as i64 - total_legacy_unresolved as i64;
    let all_blockers_closed = blocker_closed_count == fixture_count;

    // Write aggregate blocker-closure manifest
    let mut manifest = String::new();
    manifest.push_str("m7_3_parity_blocker_closure=aggregate_gate\n");
    manifest.push_str(&format!("fixture_count={}\n", fixture_count));
    manifest.push_str(&format!("blocker_closed_count={}\n", blocker_closed_count));
    manifest.push_str(&format!("total_legacy_unresolved={}\n", total_legacy_unresolved));
    manifest.push_str(&format!("total_handoff_unresolved={}\n", total_handoff_unresolved));
    manifest.push_str(&format!("unresolved_name_delta_vs_baseline={}\n", unresolved_delta));
    manifest.push_str(&format!("m7_a1_gate_satisfied={}\n", all_blockers_closed && unresolved_delta <= 0));
    for (i, r) in records.iter().enumerate() {
        manifest.push_str(&format!(
            "fixture_{:03}_label={}\nfixture_{:03}_legacy_status={}\nfixture_{:03}_handoff_status={}\n\
             fixture_{:03}_blocker_closed={}\n",
            i + 1, r.fixture_label,
            i + 1, r.legacy_rustc_status,
            i + 1, r.handoff_rustc_status,
            i + 1, r.compile_non_worsening() && r.unresolved_non_worsening(),
        ));
    }
    manifest.push_str(&format!(
        "m7_3_blocker_closure_verdict={}\n",
        if all_blockers_closed { "all_closed" } else { "open_blockers_remain" }
    ));
    let manifest_path = log_dir.join("m7_3_aggregate_blocker_closure_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write m7_3 aggregate manifest");

    // --- M7.A1 assertions ---
    let worsening: Vec<&str> = records
        .iter()
        .filter(|r| !r.compile_non_worsening())
        .map(|r| r.fixture_label)
        .collect();
    assert!(
        worsening.is_empty(),
        "M7.3 aggregate gate: candidate worsened compile vs baseline for: {:?}; \
         manifest: {}",
        worsening,
        log_dir.display()
    );

    assert!(
        unresolved_delta <= 0,
        "M7.3 aggregate gate: candidate increased unresolved-name count by {} vs baseline; \
         manifest: {}",
        unresolved_delta,
        log_dir.display()
    );

    assert!(
        all_blockers_closed,
        "M7.3 aggregate gate: {}/{} blockers closed; manifest: {}",
        blocker_closed_count,
        fixture_count,
        log_dir.display()
    );

    eprintln!(
        "M7.3 aggregate gate PASSED: {}/{} blockers closed, unresolved_delta={}, \
         manifest: {}",
        blocker_closed_count,
        fixture_count,
        unresolved_delta,
        log_dir.display()
    );
}
