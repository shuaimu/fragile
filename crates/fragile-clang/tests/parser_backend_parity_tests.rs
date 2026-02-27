use fragile_clang::{
    convert_to_clang_node, transpile_cpp_to_rust_with_options, ClangNode, ClangNodeKind,
    ClangParser, CppType, LibToolingParser, ParserBackend, ParserLanguage, TranspileOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const PARITY_LOG_ROOT_PREFIX: &str = "fragile_parser_backend_parity_local_fixture";
const CPP_TYPE_SNAPSHOT_LOG_ROOT_PREFIX: &str = "fragile_parser_backend_cpp_type_snapshot_fixture";
const CPP_TYPE_QUALIFIER_DECAY_SNAPSHOT_LOG_ROOT_PREFIX: &str =
    "fragile_parser_backend_cpp_type_qualifier_decay_snapshot_fixture";
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
const MARKER_NAMESPACE_MATH: &str = "pub mod math";
const MARKER_NAMESPACE_NS_ADD: &str = "pub extern \"C\" fn ns_add";
const MARKER_TEMPLATE_FN_IDENTITY_I32: &str = "pub fn identity_i32";
const MARKER_TEMPLATE_STRUCT_BOX_INT: &str = "pub struct Box_int_";
const MARKER_TYPEDEF_INTARRAY4: &str = "pub type IntArray4 = [i32; 4]";
const MARKER_DECLTYPE_SCALAR_FN_SIG: &str =
    "pub extern \"C\" fn decltype_scalar_identity(v: i32) -> i32";
const MARKER_CONST_PTR_FN_SIG: &str = "pub extern \"C\" fn read_const_ptr(p: *const i32) -> i32";
const MARKER_MUT_REF_FN_SIG: &str = "pub extern \"C\" fn bump_ref(mut value: &mut i32) -> i32";
const MARKER_ARRAY_DECAY_FN_SIG: &str =
    "pub extern \"C\" fn array_decay_head(data: *mut i32) -> i32";
const MARKER_TYPEDEF_FRAGILE_FILE_ALIAS: &str = "pub type FragileFileAlias = std::ffi::c_void";
const MARKER_TEMPLATE_PLACEHOLDER_VALUE_TYPE_ALIAS: &str = "pub type value_type = std::ffi::c_void";
const MARKER_DEPENDENT_TYPE_PLACEHOLDER_STRUCT: &str = "pub struct _dependent_type;";

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
    has_namespace_math: bool,
    has_namespace_ns_add: bool,
    has_template_fn_identity_i32: bool,
    has_template_struct_box_int: bool,
    has_typedef_intarray4: bool,
    has_decltype_scalar_fn_sig: bool,
    has_const_ptr_fn_sig: bool,
    has_mut_ref_fn_sig: bool,
    has_array_decay_fn_sig: bool,
    has_typedef_fragile_file_alias: bool,
    has_template_placeholder_value_type_alias: bool,
    has_dependent_type_placeholder_struct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackendCppTypeSnapshot {
    backend_name: &'static str,
    decltype_alias_underlying: Option<CppType>,
    decltype_alias_fn_return: Option<CppType>,
    decltype_alias_fn_param0: Option<CppType>,
    decltype_direct_fn_return: Option<CppType>,
    decltype_direct_fn_param0: Option<CppType>,
    dependent_identity_return: Option<CppType>,
    dependent_identity_param0: Option<CppType>,
    dependent_holder_identity_return: Option<CppType>,
    dependent_holder_identity_param0: Option<CppType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackendQualifierDecaySnapshot {
    backend_name: &'static str,
    const_ptr_param0: Option<CppType>,
    mut_ptr_param0: Option<CppType>,
    const_ref_param0: Option<CppType>,
    mut_ref_param0: Option<CppType>,
    sized_array_alias_underlying: Option<CppType>,
    unsized_array_alias_underlying: Option<CppType>,
    decay_sized_array_param0: Option<CppType>,
    decay_unsized_array_param0: Option<CppType>,
    preserve_array_ref_boundary_param0: Option<CppType>,
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{stamp}"))
}

fn parser_backend_parity_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
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
            has_namespace_math: rust_code.contains(MARKER_NAMESPACE_MATH),
            has_namespace_ns_add: rust_code.contains(MARKER_NAMESPACE_NS_ADD),
            has_template_fn_identity_i32: rust_code.contains(MARKER_TEMPLATE_FN_IDENTITY_I32),
            has_template_struct_box_int: rust_code.contains(MARKER_TEMPLATE_STRUCT_BOX_INT),
            has_typedef_intarray4: rust_code.contains(MARKER_TYPEDEF_INTARRAY4),
            has_decltype_scalar_fn_sig: rust_code.contains(MARKER_DECLTYPE_SCALAR_FN_SIG),
            has_const_ptr_fn_sig: rust_code.contains(MARKER_CONST_PTR_FN_SIG),
            has_mut_ref_fn_sig: rust_code.contains(MARKER_MUT_REF_FN_SIG),
            has_array_decay_fn_sig: rust_code.contains(MARKER_ARRAY_DECAY_FN_SIG),
            has_typedef_fragile_file_alias: rust_code.contains(MARKER_TYPEDEF_FRAGILE_FILE_ALIAS),
            has_template_placeholder_value_type_alias: rust_code
                .contains(MARKER_TEMPLATE_PLACEHOLDER_VALUE_TYPE_ALIAS),
            has_dependent_type_placeholder_struct: rust_code
                .contains(MARKER_DEPENDENT_TYPE_PLACEHOLDER_STRUCT),
        });
    }

    let mut manifest = format!(
        "fixture=parser_backend_parity_local\nsource={}\nbackend_count={}\n",
        source_path.display(),
        results.len()
    );
    for result in &results {
        manifest.push_str(&format!(
            "backend={} rust_path={} rustc_status={} markers=fn_add:{},fn_mul:{},ret_add:{},ret_mul:{},typedef_count:{},alias_distance:{},enum_mode:{},enum_mode_a:{},enum_mode_b:{},struct_point:{},struct_point_x:{},struct_point_y:{},namespace_math:{},namespace_ns_add:{},template_fn_identity_i32:{},template_struct_box_int:{},typedef_intarray4:{},decltype_scalar_fn_sig:{},const_ptr_fn_sig:{},mut_ref_fn_sig:{},array_decay_fn_sig:{},typedef_fragile_file_alias:{},template_placeholder_value_type_alias:{},dependent_type_placeholder_struct:{}\n",
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
            result.has_struct_point_y,
            result.has_namespace_math,
            result.has_namespace_ns_add,
            result.has_template_fn_identity_i32,
            result.has_template_struct_box_int,
            result.has_typedef_intarray4,
            result.has_decltype_scalar_fn_sig,
            result.has_const_ptr_fn_sig,
            result.has_mut_ref_fn_sig,
            result.has_array_decay_fn_sig,
            result.has_typedef_fragile_file_alias,
            result.has_template_placeholder_value_type_alias,
            result.has_dependent_type_placeholder_struct
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
    let _guard = parser_backend_parity_test_lock()
        .lock()
        .expect("parity test lock should not be poisoned");
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
            && reference.has_struct_point_y
            && reference.has_namespace_math
            && reference.has_namespace_ns_add
            && reference.has_template_fn_identity_i32
            && reference.has_template_struct_box_int
            && reference.has_typedef_intarray4
            && reference.has_decltype_scalar_fn_sig
            && reference.has_const_ptr_fn_sig
            && reference.has_mut_ref_fn_sig
            && reference.has_array_decay_fn_sig
            && reference.has_typedef_fragile_file_alias
            && reference.has_template_placeholder_value_type_alias
            && reference.has_dependent_type_placeholder_struct,
        "reference backend marker-set should contain expected function/return markers; logs: {}",
        log_dir.display()
    );

    let hybrid = results
        .iter()
        .find(|entry| entry.backend_name == "hybrid")
        .expect("missing hybrid parity result");
    assert_eq!(
        [
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
            hybrid.has_struct_point_y,
            hybrid.has_namespace_math,
            hybrid.has_namespace_ns_add,
            hybrid.has_template_fn_identity_i32,
            hybrid.has_template_struct_box_int,
            hybrid.has_typedef_intarray4,
            hybrid.has_decltype_scalar_fn_sig,
            hybrid.has_const_ptr_fn_sig,
            hybrid.has_mut_ref_fn_sig,
            hybrid.has_array_decay_fn_sig,
            hybrid.has_typedef_fragile_file_alias,
            hybrid.has_template_placeholder_value_type_alias,
            hybrid.has_dependent_type_placeholder_struct
        ],
        [
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
            reference.has_struct_point_y,
            reference.has_namespace_math,
            reference.has_namespace_ns_add,
            reference.has_template_fn_identity_i32,
            reference.has_template_struct_box_int,
            reference.has_typedef_intarray4,
            reference.has_decltype_scalar_fn_sig,
            reference.has_const_ptr_fn_sig,
            reference.has_mut_ref_fn_sig,
            reference.has_array_decay_fn_sig,
            reference.has_typedef_fragile_file_alias,
            reference.has_template_placeholder_value_type_alias,
            reference.has_dependent_type_placeholder_struct
        ],
        "hybrid backend should currently match libclang marker-set parity; logs: {}",
        log_dir.display()
    );

    let libtooling = results
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing libtooling parity result");
    assert_eq!(
        [
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
            libtooling.has_struct_point_y,
            libtooling.has_namespace_math,
            libtooling.has_namespace_ns_add,
            libtooling.has_template_fn_identity_i32,
            libtooling.has_template_struct_box_int,
            libtooling.has_typedef_intarray4,
            libtooling.has_decltype_scalar_fn_sig,
            libtooling.has_const_ptr_fn_sig,
            libtooling.has_mut_ref_fn_sig,
            libtooling.has_array_decay_fn_sig,
            libtooling.has_typedef_fragile_file_alias,
            libtooling.has_template_placeholder_value_type_alias,
            libtooling.has_dependent_type_placeholder_struct
        ],
        [
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
            reference.has_struct_point_y,
            reference.has_namespace_math,
            reference.has_namespace_ns_add,
            reference.has_template_fn_identity_i32,
            reference.has_template_struct_box_int,
            reference.has_typedef_intarray4,
            reference.has_decltype_scalar_fn_sig,
            reference.has_const_ptr_fn_sig,
            reference.has_mut_ref_fn_sig,
            reference.has_array_decay_fn_sig,
            reference.has_typedef_fragile_file_alias,
            reference.has_template_placeholder_value_type_alias,
            reference.has_dependent_type_placeholder_struct
        ],
        "libtooling backend should match libclang marker-set parity for this fixture; logs: {}",
        log_dir.display()
    );
}

fn parse_translation_unit_for_backend(
    source_path: &Path,
    backend: ParserBackend,
) -> Result<ClangNode, String> {
    match backend {
        ParserBackend::Libclang | ParserBackend::Hybrid => {
            let parser = ClangParser::with_paths_defines_language_and_ignored_errors(
                Vec::new(),
                Vec::new(),
                ParserLanguage::Cpp,
                Vec::new(),
            )
            .map_err(|e| format!("failed to create libclang parser: {e}"))?;
            let ast = parser
                .parse_file(source_path)
                .map_err(|e| format!("libclang parse failed for {}: {e}", source_path.display()))?;
            Ok(ast.translation_unit)
        }
        ParserBackend::Libtooling => {
            let compile_dir = source_path
                .parent()
                .ok_or_else(|| format!("missing parent directory for {}", source_path.display()))?;
            let parser = LibToolingParser::new().with_compile_commands_dir(
                compile_dir.to_str().ok_or_else(|| {
                    format!("non UTF-8 compile dir path: {}", compile_dir.display())
                })?,
            );
            let ctx = parser.parse_file(source_path).map_err(|e| {
                format!("libtooling parse failed for {}: {e}", source_path.display())
            })?;
            let children = ctx
                .top_nodes
                .iter()
                .filter_map(|id| convert_to_clang_node(&ctx, *id))
                .collect();
            Ok(ClangNode::new(ClangNodeKind::TranslationUnit).with_children(children))
        }
    }
}

fn collect_cpp_type_snapshot(node: &ClangNode, snapshot: &mut BackendCppTypeSnapshot) {
    match &node.kind {
        ClangNodeKind::TypeAliasDecl {
            name,
            underlying_type,
        }
        | ClangNodeKind::TypedefDecl {
            name,
            underlying_type,
        } if name == "DecltypeAlias" => {
            snapshot.decltype_alias_underlying = Some(underlying_type.clone());
        }
        ClangNodeKind::FunctionDecl {
            name,
            return_type,
            params,
            ..
        } => {
            if name == "decltype_alias_identity" {
                snapshot.decltype_alias_fn_return = Some(return_type.clone());
                snapshot.decltype_alias_fn_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "decltype_direct_identity" {
                snapshot.decltype_direct_fn_return = Some(return_type.clone());
                snapshot.decltype_direct_fn_param0 = params.first().map(|(_, ty)| ty.clone());
            }
        }
        ClangNodeKind::FunctionTemplateDecl {
            name,
            return_type,
            params,
            ..
        } => {
            if name == "dependent_identity" {
                snapshot.dependent_identity_return = Some(return_type.clone());
                snapshot.dependent_identity_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "dependent_holder_identity" {
                snapshot.dependent_holder_identity_return = Some(return_type.clone());
                snapshot.dependent_holder_identity_param0 =
                    params.first().map(|(_, ty)| ty.clone());
            }
        }
        _ => {}
    }

    for child in &node.children {
        collect_cpp_type_snapshot(child, snapshot);
    }
}

fn collect_qualifier_decay_snapshot(
    node: &ClangNode,
    snapshot: &mut BackendQualifierDecaySnapshot,
) {
    match &node.kind {
        ClangNodeKind::TypeAliasDecl {
            name,
            underlying_type,
        }
        | ClangNodeKind::TypedefDecl {
            name,
            underlying_type,
        } if name == "SizedArrayAlias" => {
            snapshot.sized_array_alias_underlying = Some(underlying_type.clone());
        }
        ClangNodeKind::TypeAliasDecl {
            name,
            underlying_type,
        }
        | ClangNodeKind::TypedefDecl {
            name,
            underlying_type,
        } if name == "UnsizedArrayAlias" => {
            snapshot.unsized_array_alias_underlying = Some(underlying_type.clone());
        }
        ClangNodeKind::FunctionDecl { name, params, .. } => {
            if name == "read_const_ptr" {
                snapshot.const_ptr_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "read_mut_ptr" {
                snapshot.mut_ptr_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "read_const_ref" {
                snapshot.const_ref_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "bump_mut_ref" {
                snapshot.mut_ref_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "decay_sized_array_param" {
                snapshot.decay_sized_array_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "decay_unsized_array_param" {
                snapshot.decay_unsized_array_param0 = params.first().map(|(_, ty)| ty.clone());
            } else if name == "preserve_array_ref_boundary" {
                snapshot.preserve_array_ref_boundary_param0 =
                    params.first().map(|(_, ty)| ty.clone());
            }
        }
        _ => {}
    }

    for child in &node.children {
        collect_qualifier_decay_snapshot(child, snapshot);
    }
}

fn cpp_type_family_snapshot(ty: &CppType) -> String {
    match ty {
        CppType::TemplateParam { name, depth, index } => {
            format!("template_param:{name}:{depth}:{index}")
        }
        CppType::DependentType { spelling } => format!("dependent:{spelling}"),
        CppType::Named(name) if name.contains("type-parameter-") => {
            format!("named_type_parameter:{name}")
        }
        CppType::Named(name) if name.contains("value_type") || name.contains("Holder<") => {
            format!("named_dependent:{name}")
        }
        _ => format!("concrete:{:?}", ty),
    }
}

fn snapshot_entry(label: &str, ty: &Option<CppType>) -> String {
    match ty {
        Some(ty) => format!("{label}={}", cpp_type_family_snapshot(ty)),
        None => format!("{label}=<missing>"),
    }
}

fn run_cpp_type_snapshot_local_fixture() -> Result<(PathBuf, Vec<BackendCppTypeSnapshot>), String> {
    let log_dir = unique_temp_dir(CPP_TYPE_SNAPSHOT_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).map_err(|e| {
        format!(
            "failed to create cpp-type snapshot log dir {}: {e}",
            log_dir.display()
        )
    })?;

    let source_path = log_dir.join("parser_backend_cpp_type_snapshot_fixture.cpp");
    fs::write(
        &source_path,
        r#"
template<typename T>
struct Holder {
    using value_type = T;
};

template<typename T>
T dependent_identity(T value) {
    return value;
}

template<typename T>
typename Holder<T>::value_type dependent_holder_identity(typename Holder<T>::value_type value) {
    return value;
}

using DecltypeAlias = decltype(1 + 2);

DecltypeAlias decltype_alias_identity(DecltypeAlias value) {
    return value;
}

decltype(1 + 2) decltype_direct_identity(decltype(1 + 2) value) {
    return value;
}
"#,
    )
    .map_err(|e| {
        format!(
            "failed to write cpp-type snapshot fixture source {}: {e}",
            source_path.display()
        )
    })?;

    let backends: [(&str, ParserBackend); 3] = [
        ("libclang", ParserBackend::Libclang),
        ("hybrid", ParserBackend::Hybrid),
        ("libtooling", ParserBackend::Libtooling),
    ];

    let mut snapshots = Vec::new();
    for (backend_name, backend) in backends {
        let translation_unit =
            parse_translation_unit_for_backend(&source_path, backend).map_err(|e| {
                format!(
                    "backend {} failed to parse fixture {}: {}",
                    backend_name,
                    source_path.display(),
                    e
                )
            })?;

        let mut snapshot = BackendCppTypeSnapshot {
            backend_name,
            decltype_alias_underlying: None,
            decltype_alias_fn_return: None,
            decltype_alias_fn_param0: None,
            decltype_direct_fn_return: None,
            decltype_direct_fn_param0: None,
            dependent_identity_return: None,
            dependent_identity_param0: None,
            dependent_holder_identity_return: None,
            dependent_holder_identity_param0: None,
        };
        collect_cpp_type_snapshot(&translation_unit, &mut snapshot);
        snapshots.push(snapshot);
    }

    let mut manifest = format!(
        "fixture=parser_backend_cpp_type_snapshot_local\nsource={}\nbackend_count={}\n",
        source_path.display(),
        snapshots.len()
    );
    for snapshot in &snapshots {
        manifest.push_str(&format!(
            "backend={} {} {} {} {} {} {} {} {} {}\n",
            snapshot.backend_name,
            snapshot_entry(
                "decltype_alias_underlying",
                &snapshot.decltype_alias_underlying
            ),
            snapshot_entry(
                "decltype_alias_fn_return",
                &snapshot.decltype_alias_fn_return
            ),
            snapshot_entry(
                "decltype_alias_fn_param0",
                &snapshot.decltype_alias_fn_param0
            ),
            snapshot_entry(
                "decltype_direct_fn_return",
                &snapshot.decltype_direct_fn_return
            ),
            snapshot_entry(
                "decltype_direct_fn_param0",
                &snapshot.decltype_direct_fn_param0
            ),
            snapshot_entry(
                "dependent_identity_return",
                &snapshot.dependent_identity_return
            ),
            snapshot_entry(
                "dependent_identity_param0",
                &snapshot.dependent_identity_param0
            ),
            snapshot_entry(
                "dependent_holder_identity_return",
                &snapshot.dependent_holder_identity_return
            ),
            snapshot_entry(
                "dependent_holder_identity_param0",
                &snapshot.dependent_holder_identity_param0
            ),
        ));
    }
    fs::write(
        log_dir.join("parser_backend_cpp_type_snapshot_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write parser_backend_cpp_type_snapshot_manifest.txt in {}: {e}",
            log_dir.display()
        )
    })?;

    Ok((log_dir, snapshots))
}

#[test]
fn test_parser_backend_cpp_type_snapshot_decltype_and_template_families() {
    let _guard = parser_backend_parity_test_lock()
        .lock()
        .expect("parity test lock should not be poisoned");
    let (log_dir, snapshots) = run_cpp_type_snapshot_local_fixture()
        .expect("failed to run parser-backend cpp-type snapshot fixture");

    let manifest_path = log_dir.join("parser_backend_cpp_type_snapshot_manifest.txt");
    assert!(
        manifest_path.exists(),
        "expected cpp-type snapshot manifest at {}",
        manifest_path.display()
    );
    assert_eq!(
        snapshots.len(),
        3,
        "expected snapshot results for libclang/hybrid/libtooling"
    );

    let reference = snapshots
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing libclang snapshot");
    let hybrid = snapshots
        .iter()
        .find(|entry| entry.backend_name == "hybrid")
        .expect("missing hybrid snapshot");
    let libtooling = snapshots
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing libtooling snapshot");

    for snapshot in &snapshots {
        assert!(
            snapshot.decltype_alias_underlying.is_some()
                && snapshot.decltype_alias_fn_return.is_some()
                && snapshot.decltype_alias_fn_param0.is_some()
                && snapshot.decltype_direct_fn_return.is_some()
                && snapshot.decltype_direct_fn_param0.is_some()
                && snapshot.dependent_identity_return.is_some()
                && snapshot.dependent_identity_param0.is_some()
                && snapshot.dependent_holder_identity_return.is_some()
                && snapshot.dependent_holder_identity_param0.is_some(),
            "backend {} should expose all target CppType snapshot entries; logs: {}",
            snapshot.backend_name,
            log_dir.display()
        );
    }

    // Hybrid currently shares direct parser shape with libclang; keep this parity explicit.
    assert_eq!(
        [
            &hybrid.decltype_alias_underlying,
            &hybrid.decltype_alias_fn_return,
            &hybrid.decltype_alias_fn_param0,
            &hybrid.decltype_direct_fn_return,
            &hybrid.decltype_direct_fn_param0,
            &hybrid.dependent_identity_return,
            &hybrid.dependent_identity_param0,
            &hybrid.dependent_holder_identity_return,
            &hybrid.dependent_holder_identity_param0,
        ],
        [
            &reference.decltype_alias_underlying,
            &reference.decltype_alias_fn_return,
            &reference.decltype_alias_fn_param0,
            &reference.decltype_direct_fn_return,
            &reference.decltype_direct_fn_param0,
            &reference.dependent_identity_return,
            &reference.dependent_identity_param0,
            &reference.dependent_holder_identity_return,
            &reference.dependent_holder_identity_param0,
        ],
        "hybrid snapshot should match libclang direct-parser snapshot; logs: {}",
        log_dir.display()
    );

    // Lock current libclang/hybrid parse-roundtrip snapshot for decltype + dependent families.
    assert_eq!(
        reference.decltype_alias_underlying,
        Some(CppType::Named("decltype(1 + 2)".to_string())),
        "libclang decltype alias underlying snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.decltype_alias_fn_return,
        Some(CppType::Int { signed: true }),
        "libclang decltype alias return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.decltype_alias_fn_param0,
        Some(CppType::Int { signed: true }),
        "libclang decltype alias param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.decltype_direct_fn_return,
        Some(CppType::Named("decltype(1 + 2)".to_string())),
        "libclang direct decltype return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.decltype_direct_fn_param0,
        Some(CppType::Named("decltype(1 + 2)".to_string())),
        "libclang direct decltype param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.dependent_identity_return,
        Some(CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        }),
        "libclang dependent identity return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.dependent_identity_param0,
        Some(CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        }),
        "libclang dependent identity param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.dependent_holder_identity_return,
        Some(CppType::DependentType {
            spelling: "typename Holder<T>::value_type".to_string(),
        }),
        "libclang dependent holder return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.dependent_holder_identity_param0,
        Some(CppType::DependentType {
            spelling: "typename Holder<T>::value_type".to_string(),
        }),
        "libclang dependent holder param snapshot changed; logs: {}",
        log_dir.display()
    );

    // Lock current libtooling parse-roundtrip snapshot for the same families.
    assert_eq!(
        libtooling.decltype_alias_underlying,
        Some(CppType::Int { signed: true }),
        "libtooling decltype alias underlying snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.decltype_alias_fn_return,
        Some(CppType::Int { signed: true }),
        "libtooling decltype alias return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.decltype_alias_fn_param0,
        Some(CppType::Int { signed: true }),
        "libtooling decltype alias param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.decltype_direct_fn_return,
        Some(CppType::Int { signed: true }),
        "libtooling direct decltype return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.decltype_direct_fn_param0,
        Some(CppType::Int { signed: true }),
        "libtooling direct decltype param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.dependent_identity_return,
        Some(CppType::Int { signed: true }),
        "libtooling dependent identity return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.dependent_identity_param0,
        Some(CppType::Named("auto".to_string())),
        "libtooling dependent identity param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.dependent_holder_identity_return,
        Some(CppType::Int { signed: true }),
        "libtooling dependent holder return snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.dependent_holder_identity_param0,
        Some(CppType::Named("auto".to_string())),
        "libtooling dependent holder param snapshot changed; logs: {}",
        log_dir.display()
    );
}

fn run_qualifier_decay_cpp_type_snapshot_local_fixture(
) -> Result<(PathBuf, Vec<BackendQualifierDecaySnapshot>), String> {
    let log_dir = unique_temp_dir(CPP_TYPE_QUALIFIER_DECAY_SNAPSHOT_LOG_ROOT_PREFIX);
    fs::create_dir_all(&log_dir).map_err(|e| {
        format!(
            "failed to create qualifier/decay snapshot log dir {}: {e}",
            log_dir.display()
        )
    })?;

    let source_path = log_dir.join("parser_backend_cpp_type_qualifier_decay_snapshot_fixture.cpp");
    fs::write(
        &source_path,
        r#"
typedef int SizedArrayAlias[4];
typedef int UnsizedArrayAlias[];

int read_const_ptr(const int* value) {
    return *value;
}

int read_mut_ptr(int* value) {
    return *value;
}

int read_const_ref(const int& value) {
    return value;
}

int bump_mut_ref(int& value) {
    value += 1;
    return value;
}

int decay_sized_array_param(SizedArrayAlias value) {
    return value[0];
}

int decay_unsized_array_param(int value[]) {
    return value[0];
}

int preserve_array_ref_boundary(int (&value)[4]) {
    return value[0];
}
"#,
    )
    .map_err(|e| {
        format!(
            "failed to write qualifier/decay snapshot fixture source {}: {e}",
            source_path.display()
        )
    })?;

    let backends: [(&str, ParserBackend); 3] = [
        ("libclang", ParserBackend::Libclang),
        ("hybrid", ParserBackend::Hybrid),
        ("libtooling", ParserBackend::Libtooling),
    ];

    let mut snapshots = Vec::new();
    for (backend_name, backend) in backends {
        let translation_unit =
            parse_translation_unit_for_backend(&source_path, backend).map_err(|e| {
                format!(
                    "backend {} failed to parse qualifier/decay fixture {}: {}",
                    backend_name,
                    source_path.display(),
                    e
                )
            })?;

        let mut snapshot = BackendQualifierDecaySnapshot {
            backend_name,
            const_ptr_param0: None,
            mut_ptr_param0: None,
            const_ref_param0: None,
            mut_ref_param0: None,
            sized_array_alias_underlying: None,
            unsized_array_alias_underlying: None,
            decay_sized_array_param0: None,
            decay_unsized_array_param0: None,
            preserve_array_ref_boundary_param0: None,
        };
        collect_qualifier_decay_snapshot(&translation_unit, &mut snapshot);
        snapshots.push(snapshot);
    }

    let mut manifest = format!(
        "fixture=parser_backend_cpp_type_qualifier_decay_snapshot_local\nsource={}\nbackend_count={}\n",
        source_path.display(),
        snapshots.len()
    );
    for snapshot in &snapshots {
        manifest.push_str(&format!(
            "backend={} {} {} {} {} {} {} {} {} {}\n",
            snapshot.backend_name,
            snapshot_entry("const_ptr_param0", &snapshot.const_ptr_param0),
            snapshot_entry("mut_ptr_param0", &snapshot.mut_ptr_param0),
            snapshot_entry("const_ref_param0", &snapshot.const_ref_param0),
            snapshot_entry("mut_ref_param0", &snapshot.mut_ref_param0),
            snapshot_entry(
                "sized_array_alias_underlying",
                &snapshot.sized_array_alias_underlying
            ),
            snapshot_entry(
                "unsized_array_alias_underlying",
                &snapshot.unsized_array_alias_underlying
            ),
            snapshot_entry(
                "decay_sized_array_param0",
                &snapshot.decay_sized_array_param0
            ),
            snapshot_entry(
                "decay_unsized_array_param0",
                &snapshot.decay_unsized_array_param0
            ),
            snapshot_entry(
                "preserve_array_ref_boundary_param0",
                &snapshot.preserve_array_ref_boundary_param0
            ),
        ));
    }
    fs::write(
        log_dir.join("parser_backend_cpp_type_qualifier_decay_snapshot_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write parser_backend_cpp_type_qualifier_decay_snapshot_manifest.txt in {}: {e}",
            log_dir.display()
        )
    })?;

    Ok((log_dir, snapshots))
}

#[test]
fn test_parser_backend_cpp_type_snapshot_pointer_ref_qualifiers_and_array_decay_boundaries() {
    let _guard = parser_backend_parity_test_lock()
        .lock()
        .expect("parity test lock should not be poisoned");
    let (log_dir, snapshots) = run_qualifier_decay_cpp_type_snapshot_local_fixture()
        .expect("failed to run parser-backend qualifier/decay cpp-type snapshot fixture");

    let manifest_path =
        log_dir.join("parser_backend_cpp_type_qualifier_decay_snapshot_manifest.txt");
    assert!(
        manifest_path.exists(),
        "expected qualifier/decay cpp-type snapshot manifest at {}",
        manifest_path.display()
    );
    assert_eq!(
        snapshots.len(),
        3,
        "expected snapshot results for libclang/hybrid/libtooling"
    );

    let reference = snapshots
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing libclang snapshot");
    let hybrid = snapshots
        .iter()
        .find(|entry| entry.backend_name == "hybrid")
        .expect("missing hybrid snapshot");
    let libtooling = snapshots
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing libtooling snapshot");

    for snapshot in &snapshots {
        assert!(
            snapshot.const_ptr_param0.is_some()
                && snapshot.mut_ptr_param0.is_some()
                && snapshot.const_ref_param0.is_some()
                && snapshot.mut_ref_param0.is_some()
                && snapshot.sized_array_alias_underlying.is_some()
                && snapshot.unsized_array_alias_underlying.is_some()
                && snapshot.decay_sized_array_param0.is_some()
                && snapshot.decay_unsized_array_param0.is_some()
                && snapshot.preserve_array_ref_boundary_param0.is_some(),
            "backend {} should expose all qualifier/decay snapshot entries; logs: {}",
            snapshot.backend_name,
            log_dir.display()
        );
    }

    // Hybrid currently shares direct parser shape with libclang; keep this parity explicit.
    assert_eq!(
        [
            &hybrid.const_ptr_param0,
            &hybrid.mut_ptr_param0,
            &hybrid.const_ref_param0,
            &hybrid.mut_ref_param0,
            &hybrid.sized_array_alias_underlying,
            &hybrid.unsized_array_alias_underlying,
            &hybrid.decay_sized_array_param0,
            &hybrid.decay_unsized_array_param0,
            &hybrid.preserve_array_ref_boundary_param0,
        ],
        [
            &reference.const_ptr_param0,
            &reference.mut_ptr_param0,
            &reference.const_ref_param0,
            &reference.mut_ref_param0,
            &reference.sized_array_alias_underlying,
            &reference.unsized_array_alias_underlying,
            &reference.decay_sized_array_param0,
            &reference.decay_unsized_array_param0,
            &reference.preserve_array_ref_boundary_param0,
        ],
        "hybrid snapshot should match libclang direct-parser snapshot; logs: {}",
        log_dir.display()
    );

    let expected_const_ptr = Some(CppType::Pointer {
        pointee: Box::new(CppType::Int { signed: true }),
        is_const: true,
    });
    let expected_mut_ptr = Some(CppType::Pointer {
        pointee: Box::new(CppType::Int { signed: true }),
        is_const: false,
    });
    let expected_const_ref = Some(CppType::Reference {
        referent: Box::new(CppType::Int { signed: true }),
        is_const: true,
        is_rvalue: false,
    });
    let expected_mut_ref = Some(CppType::Reference {
        referent: Box::new(CppType::Int { signed: true }),
        is_const: false,
        is_rvalue: false,
    });
    let expected_sized_array = Some(CppType::Array {
        element: Box::new(CppType::Int { signed: true }),
        size: Some(4),
    });
    let expected_unsized_array = Some(CppType::Array {
        element: Box::new(CppType::Int { signed: true }),
        size: None,
    });
    let expected_decayed_mut_ptr = Some(CppType::Pointer {
        pointee: Box::new(CppType::Int { signed: true }),
        is_const: false,
    });
    let expected_array_ref_boundary = Some(CppType::Reference {
        referent: Box::new(CppType::Array {
            element: Box::new(CppType::Int { signed: true }),
            size: Some(4),
        }),
        is_const: false,
        is_rvalue: false,
    });

    // Lock current libclang/hybrid parse-roundtrip snapshot for qualifier + decay/boundary families.
    assert_eq!(
        reference.const_ptr_param0,
        expected_const_ptr,
        "libclang const pointer qualifier snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.mut_ptr_param0,
        expected_mut_ptr.clone(),
        "libclang mutable pointer qualifier snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.const_ref_param0,
        expected_const_ref,
        "libclang const reference qualifier snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.mut_ref_param0,
        expected_mut_ref.clone(),
        "libclang mutable reference qualifier snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.sized_array_alias_underlying,
        expected_sized_array.clone(),
        "libclang sized-array alias snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.unsized_array_alias_underlying,
        expected_unsized_array,
        "libclang unsized-array alias snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.decay_sized_array_param0,
        expected_sized_array.clone(),
        "libclang sized-array param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.decay_unsized_array_param0,
        expected_unsized_array.clone(),
        "libclang unsized-array param snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        reference.preserve_array_ref_boundary_param0,
        expected_array_ref_boundary,
        "libclang array-reference boundary snapshot changed; logs: {}",
        log_dir.display()
    );

    // Lock libtooling to current direct-parser shape for these families.
    assert_eq!(
        [
            &libtooling.const_ptr_param0,
            &libtooling.mut_ptr_param0,
            &libtooling.const_ref_param0,
            &libtooling.mut_ref_param0,
            &libtooling.sized_array_alias_underlying,
            &libtooling.unsized_array_alias_underlying,
            &libtooling.preserve_array_ref_boundary_param0,
        ],
        [
            &reference.const_ptr_param0,
            &reference.mut_ptr_param0,
            &reference.const_ref_param0,
            &reference.mut_ref_param0,
            &reference.sized_array_alias_underlying,
            &reference.unsized_array_alias_underlying,
            &reference.preserve_array_ref_boundary_param0,
        ],
        "libtooling qualifier + non-decay-boundary snapshot should match libclang direct-parser snapshot; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.decay_sized_array_param0,
        expected_decayed_mut_ptr.clone(),
        "libtooling sized-array decay snapshot changed; logs: {}",
        log_dir.display()
    );
    assert_eq!(
        libtooling.decay_unsized_array_param0,
        expected_decayed_mut_ptr,
        "libtooling unsized-array decay snapshot changed; logs: {}",
        log_dir.display()
    );
}
