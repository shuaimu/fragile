use fragile_clang::{
    transpile_cpp_to_rust_with_options, ParserBackend, ParserLanguage, TranspileOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const PARITY_LOG_ROOT_PREFIX: &str = "fragile_parser_backend_parity_local_fixture";
const MARKER_FN_ADD: &str = "pub extern \"C\" fn add";
const MARKER_FN_MUL: &str = "pub extern \"C\" fn mul";
const MARKER_RETURN_ADD: &str = "return a + b";
const MARKER_RETURN_MUL: &str = "return x * y";
const MARKER_TYPEDEF_COUNT: &str = "pub type Count =";
const MARKER_ALIAS_DISTANCE: &str = "pub type Distance =";
const MARKER_ENUM_MODE: &str = "pub enum Mode";
const MARKER_ENUM_MODE_A: &str = "ModeA = 1";
const MARKER_ENUM_MODE_B: &str = "ModeB = 2";
const MARKER_STRUCT_POINT: &str = "pub struct Point";
const MARKER_STRUCT_POINT_X: &str = "pub x: i32";
const MARKER_STRUCT_POINT_Y: &str = "pub y: i32";

#[derive(Debug, Clone)]
struct BackendReplayResult {
    backend_name: &'static str,
    rust_path: PathBuf,
    rustc_status: i32,
    has_fn_add: bool,
    has_fn_mul: bool,
    has_return_add: bool,
    has_return_mul: bool,
    has_typedef_count: bool,
    has_alias_distance: bool,
    has_enum_mode: bool,
    has_enum_mode_a: bool,
    has_enum_mode_b: bool,
    has_struct_point: bool,
    has_struct_point_x: bool,
    has_struct_point_y: bool,
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{stamp}"))
}

fn write_command_capture(log_dir: &Path, step: &str, output: &Output) -> Result<(), String> {
    fs::write(
        log_dir.join(format!("{step}.status")),
        output.status.code().unwrap_or(-1).to_string(),
    )
    .map_err(|e| {
        format!(
            "failed to write {step}.status in {}: {e}",
            log_dir.display()
        )
    })?;
    fs::write(log_dir.join(format!("{step}.stdout")), &output.stdout).map_err(|e| {
        format!(
            "failed to write {step}.stdout in {}: {e}",
            log_dir.display()
        )
    })?;
    fs::write(log_dir.join(format!("{step}.stderr")), &output.stderr).map_err(|e| {
        format!(
            "failed to write {step}.stderr in {}: {e}",
            log_dir.display()
        )
    })?;
    Ok(())
}

fn run_parser_backend_parity_local_fixture() -> Result<(PathBuf, Vec<BackendReplayResult>), String>
{
    let log_dir = unique_temp_dir(PARITY_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("failed to create parity log dir {}: {e}", log_dir.display()))?;

    let source_path = log_dir.join("parser_backend_parity_fixture.cpp");
    fs::write(
        &source_path,
        r#"
typedef unsigned long Count;
using Distance = int;

enum Mode {
    ModeA = 1,
    ModeB = 2,
};

struct Point {
    int x;
    int y;
};

int add(int a, int b) {
    return a + b;
}

int mul(int x, int y) {
    return x * y;
}
"#,
    )
    .map_err(|e| {
        format!(
            "failed to write fixture source {}: {e}",
            source_path.display()
        )
    })?;

    let backends: [(&str, ParserBackend); 3] = [
        ("libclang", ParserBackend::Libclang),
        ("hybrid", ParserBackend::Hybrid),
        ("libtooling", ParserBackend::Libtooling),
    ];
    let mut results = Vec::new();

    for (backend_name, backend) in backends {
        let options = TranspileOptions {
            include_paths: Vec::new(),
            defines: Vec::new(),
            language: ParserLanguage::Cpp,
            ignored_error_patterns: Vec::new(),
            backend,
        };

        let rust_code =
            transpile_cpp_to_rust_with_options(&source_path, &options).map_err(|e| {
                format!(
                    "backend {} failed to transpile fixture {}: {}",
                    backend_name,
                    source_path.display(),
                    e
                )
            })?;

        let rust_path = log_dir.join(format!("generated_{backend_name}.rs"));
        fs::write(&rust_path, rust_code.as_bytes())
            .map_err(|e| format!("failed to write {}: {e}", rust_path.display()))?;

        let metadata_path = log_dir.join(format!("generated_{backend_name}.rmeta"));
        let rustc_output = Command::new("rustc")
            .arg("--edition")
            .arg("2021")
            .arg("-A")
            .arg("warnings")
            .arg("--crate-type")
            .arg("lib")
            .arg("--emit=metadata")
            .arg(&rust_path)
            .arg("-o")
            .arg(&metadata_path)
            .output()
            .map_err(|e| format!("failed to run rustc for {}: {e}", rust_path.display()))?;
        write_command_capture(&log_dir, &format!("rustc_{backend_name}"), &rustc_output)?;

        results.push(BackendReplayResult {
            backend_name,
            rust_path,
            rustc_status: rustc_output.status.code().unwrap_or(-1),
            has_fn_add: rust_code.contains(MARKER_FN_ADD),
            has_fn_mul: rust_code.contains(MARKER_FN_MUL),
            has_return_add: rust_code.contains(MARKER_RETURN_ADD),
            has_return_mul: rust_code.contains(MARKER_RETURN_MUL),
            has_typedef_count: rust_code.contains(MARKER_TYPEDEF_COUNT),
            has_alias_distance: rust_code.contains(MARKER_ALIAS_DISTANCE),
            has_enum_mode: rust_code.contains(MARKER_ENUM_MODE),
            has_enum_mode_a: rust_code.contains(MARKER_ENUM_MODE_A),
            has_enum_mode_b: rust_code.contains(MARKER_ENUM_MODE_B),
            has_struct_point: rust_code.contains(MARKER_STRUCT_POINT),
            has_struct_point_x: rust_code.contains(MARKER_STRUCT_POINT_X),
            has_struct_point_y: rust_code.contains(MARKER_STRUCT_POINT_Y),
        });
    }

    let mut manifest = format!(
        "fixture=parser_backend_parity_local\nsource={}\nbackend_count={}\n",
        source_path.display(),
        results.len()
    );
    for result in &results {
        manifest.push_str(&format!(
            "backend={} rust_path={} rustc_status={} markers=fn_add:{},fn_mul:{},ret_add:{},ret_mul:{},typedef_count:{},alias_distance:{},enum_mode:{},enum_mode_a:{},enum_mode_b:{},struct_point:{},struct_point_x:{},struct_point_y:{}\n",
            result.backend_name,
            result.rust_path.display(),
            result.rustc_status,
            result.has_fn_add,
            result.has_fn_mul,
            result.has_return_add,
            result.has_return_mul,
            result.has_typedef_count,
            result.has_alias_distance,
            result.has_enum_mode,
            result.has_enum_mode_a,
            result.has_enum_mode_b,
            result.has_struct_point,
            result.has_struct_point_x,
            result.has_struct_point_y
        ));
    }
    fs::write(log_dir.join("parser_backend_parity_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write parser_backend_parity_manifest.txt in {}: {e}",
            log_dir.display()
        )
    })?;

    Ok((log_dir, results))
}

#[test]
fn test_parser_backend_parity_local_fixture_replay() {
    let (log_dir, results) = run_parser_backend_parity_local_fixture()
        .expect("failed to run parser-backend parity local fixture replay");

    let manifest_path = log_dir.join("parser_backend_parity_manifest.txt");
    assert!(
        manifest_path.exists(),
        "expected parity manifest at {}",
        manifest_path.display()
    );

    assert_eq!(
        results.len(),
        3,
        "expected parity replay results for libclang/hybrid/libtooling"
    );

    let reference = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing libclang reference result");
    for result in &results {
        assert_eq!(
            result.rustc_status,
            0,
            "expected backend {} generated Rust to compile; logs: {}",
            result.backend_name,
            log_dir.display()
        );
    }

    assert!(
        reference.has_fn_add
            && reference.has_fn_mul
            && reference.has_return_add
            && reference.has_return_mul
            && reference.has_typedef_count
            && reference.has_alias_distance
            && reference.has_enum_mode
            && reference.has_enum_mode_a
            && reference.has_enum_mode_b
            && reference.has_struct_point
            && reference.has_struct_point_x
            && reference.has_struct_point_y,
        "reference backend marker-set should contain expected function/return markers; logs: {}",
        log_dir.display()
    );

    let hybrid = results
        .iter()
        .find(|entry| entry.backend_name == "hybrid")
        .expect("missing hybrid parity result");
    assert_eq!(
        (
            hybrid.has_fn_add,
            hybrid.has_fn_mul,
            hybrid.has_return_add,
            hybrid.has_return_mul,
            hybrid.has_typedef_count,
            hybrid.has_alias_distance,
            hybrid.has_enum_mode,
            hybrid.has_enum_mode_a,
            hybrid.has_enum_mode_b,
            hybrid.has_struct_point,
            hybrid.has_struct_point_x,
            hybrid.has_struct_point_y
        ),
        (
            reference.has_fn_add,
            reference.has_fn_mul,
            reference.has_return_add,
            reference.has_return_mul,
            reference.has_typedef_count,
            reference.has_alias_distance,
            reference.has_enum_mode,
            reference.has_enum_mode_a,
            reference.has_enum_mode_b,
            reference.has_struct_point,
            reference.has_struct_point_x,
            reference.has_struct_point_y
        ),
        "hybrid backend should currently match libclang marker-set parity; logs: {}",
        log_dir.display()
    );

    let libtooling = results
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing libtooling parity result");
    assert_eq!(
        (
            libtooling.has_fn_add,
            libtooling.has_fn_mul,
            libtooling.has_return_add,
            libtooling.has_return_mul,
            libtooling.has_typedef_count,
            libtooling.has_alias_distance,
            libtooling.has_enum_mode,
            libtooling.has_enum_mode_a,
            libtooling.has_enum_mode_b,
            libtooling.has_struct_point,
            libtooling.has_struct_point_x,
            libtooling.has_struct_point_y
        ),
        (
            reference.has_fn_add,
            reference.has_fn_mul,
            reference.has_return_add,
            reference.has_return_mul,
            reference.has_typedef_count,
            reference.has_alias_distance,
            reference.has_enum_mode,
            reference.has_enum_mode_a,
            reference.has_enum_mode_b,
            reference.has_struct_point,
            reference.has_struct_point_x,
            reference.has_struct_point_y
        ),
        "libtooling backend should match libclang marker-set parity for this fixture; logs: {}",
        log_dir.display()
    );
}
