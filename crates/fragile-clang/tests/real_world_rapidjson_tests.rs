//! Real-world RapidJSON fixture bootstrap tests.
//!
//! This target focuses on no-STL runtime examples (`condense`, `pretty`) to
//! provide deterministic next-stage development coverage.

use fragile_clang::{AstCodeGen, ClangParser};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread::{self, sleep};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RAPIDJSON_REPO_URL: &str = "https://github.com/Tencent/rapidjson.git";
const RAPIDJSON_PINNED_COMMIT: &str = "f54b0e47a08782a6131cc3d60f94d038fa6e0a51"; // v1.1.0
const RAPIDJSON_CACHE_DIR: &str = "/tmp/fragile_real_world_rapidjson";
const RAPIDJSON_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_rapidjson_native_baseline";
const RAPIDJSON_COMMAND_PLAN_DIR: &str = "/tmp/fragile_real_world_rapidjson_no_stl_command_plan";
const RAPIDJSON_FRAGILE_CONDENSE_REPLAY_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_fragile_condense_replay";
const RAPIDJSON_FRAGILEC_DRIVER_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_fragilec_driver_baseline";
const RAPIDJSON_STRICT_CAPITALIZE_CAPTURE_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_capitalize_capture";
const RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_capitalize_backend_surface_delta";
const RAPIDJSON_STRICT_TUTORIAL_BACKEND_SURFACE_DELTA_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_tutorial_backend_surface_delta";
const RAPIDJSON_TRANSPILE_STAGE_TIMING_PARSE_FIXTURE_DIR: &str =
    "/tmp/fragile_rapidjson_transpile_stage_timing_parse_fixture";
const RAPIDJSON_STRICT_FILTERKEYDOM_CAPTURE_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_filterkeydom_capture";
const RAPIDJSON_STRICT_CMAKE_NO_TESTS_BUILD_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_build";
const RAPIDJSON_STRICT_CMAKE_NO_TESTS_BACKEND_MATRIX_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_backend_matrix";
const RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_BUILD_TIMEOUT_SECS: u64 = 1200;
const RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_TIMEOUT_SECS: u64 = 180;
const COMMAND_TIMEOUT_STATUS: i32 = 124;
const FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV: &str = "FRAGILEC_TRANSPILE_STAGE_TIMING_PATH";
const RAPIDJSON_REQUIRED_PATHS: &[&str] = &[
    "include/rapidjson/document.h",
    "example/condense/condense.cpp",
    "example/pretty/pretty.cpp",
    "CMakeLists.txt",
];
const RAPIDJSON_NO_STL_EXAMPLES: &[(&str, &str)] = &[
    ("condense", "example/condense/condense.cpp"),
    ("pretty", "example/pretty/pretty.cpp"),
];
const RAPIDJSON_SAMPLE_JSON: &str = "{\"a\":1,\"b\":[true,false],\"msg\":\"hi\"}\n";
const RAPIDJSON_EXPECTED_CONDENSE_OUTPUT: &str = "{\"a\":1,\"b\":[true,false],\"msg\":\"hi\"}";
const RAPIDJSON_CONST_ASSIGN_PARSER_DIAGNOSTIC_FRAGMENT: &str =
    "cannot assign to non-static data member 'length' with const-qualified type 'const SizeType'";
const RAPIDJSON_DUPLICATE_DEFINITION_E0428_FRAGMENT: &str = "error[E0428]";
const RAPIDJSON_FILE_ALIAS_MISSING_TYPE_FRAGMENT: &str = "cannot find type `__FILE` in this scope";
const RAPIDJSON_STD_IDENTITY_MISSING_TYPE_FRAGMENT: &str =
    "cannot find type `std___identity` in this scope";
const RAPIDJSON_FUNCTIONAL_HASH_UNNAMED_STRUCT_MISSING_TYPE_FRAGMENT: &str =
    "cannot find type `_unnamed_struct_at__home_shuai_workspace_fragile_vendor_llvm_project_libcxx_include___functional_hash_h_";
const RAPIDJSON_ATOMIC_BASE_ALIAS_MISSING_TYPE_FRAGMENT: &str =
    "cannot find type `__cxx_atomic_base_impl_bool` in this scope";
const RAPIDJSON_ARRAY_MUT_PTR_CAST_ERROR_FRAGMENT: &str =
    "non-primitive cast: `[i8; 65536]` as `*mut i8`";
const RAPIDJSON_NON_PRIMITIVE_CAST_ERROR_FRAGMENT: &str = "non-primitive cast:";
const RAPIDJSON_NON_PRIMITIVE_CAST_E0605_FRAGMENT: &str = "error[E0605]";
const RAPIDJSON_ITEM5_CAST_DECAY_CALL_SHAPE_MARKERS: &[&str] = &[
    RAPIDJSON_ARRAY_MUT_PTR_CAST_ERROR_FRAGMENT,
    RAPIDJSON_NON_PRIMITIVE_CAST_ERROR_FRAGMENT,
    RAPIDJSON_NON_PRIMITIVE_CAST_E0605_FRAGMENT,
];
const RAPIDJSON_NUMERIC_U128_UNARY_NEG_FRAGMENT: &str =
    "error[E0600]: cannot apply unary operator `-` to type `u128`";
const RAPIDJSON_NUMERIC_U8_TO_CHAR_CAST_FRAGMENT: &str =
    "error[E0604]: only `u8` can be cast as `char`";
const RAPIDJSON_NUMERIC_I64_AS_FUNCTION_FRAGMENT: &str =
    "error[E0618]: expected function, found `i64`";
const RAPIDJSON_NUMERIC_POW10_U128_MIXED_WIDTH_FRAGMENT: &str =
    "static mut __gv___pow10_128: [u128; 40] = [0, 10u64";
const RAPIDJSON_NUMERIC_POW10_U128_OVERFLOW_FRAGMENT: &str = "error[E0080]: attempt to compute";
const RAPIDJSON_ITEM6_63_CLEARED_MARKERS: &[&str] = &[
    RAPIDJSON_NUMERIC_U8_TO_CHAR_CAST_FRAGMENT,
    RAPIDJSON_NUMERIC_I64_AS_FUNCTION_FRAGMENT,
];
const RAPIDJSON_ITEM6_62_CLEARED_MARKERS: &[&str] = &[
    RAPIDJSON_NUMERIC_U128_UNARY_NEG_FRAGMENT,
    RAPIDJSON_NUMERIC_POW10_U128_MIXED_WIDTH_FRAGMENT,
    RAPIDJSON_NUMERIC_POW10_U128_OVERFLOW_FRAGMENT,
];
const RAPIDJSON_STRICT_CMAKE_CAPITALIZE_GLOBAL_REMAP_BLOCKER_MARKERS: &[&str] = &[
    "cannot find value `__gv___c` in this scope",
    "cannot find value `__gv_fill_n` in this scope",
    "cannot find value `__gv_copy_n` in this scope",
];
const RAPIDJSON_STRICT_CMAKE_CAPITALIZE_CONSTEXPR_BLOCKER_MARKERS: &[&str] = &[
    "cannot find function `__constexpr_strlen_i8` in this scope",
    "cannot find function `__constexpr_strlen_u8` in this scope",
    "cannot find function `__constexpr_wmemchr_i32_i32` in this scope",
];
const RAPIDJSON_STRICT_CMAKE_CAPITALIZE_NEW0_CLEARED_MARKERS: &[&str] = &[
    "no function or associated item named `new_0` found for struct `CapitalizeFilter_Writer_FileWriteStream`",
];
const RAPIDJSON_FILTERKEYDOM_PLACEHOLDER_API_HOLE_MARKERS: &[&str] = &[
    "no function or associated item named `new_0` found for struct `FilterKeyReader_FileReadStream`",
    "no method named `Populate` found for struct `GenericDocument_UTF8_`",
    "no method named `Accept` found for struct `GenericDocument_UTF8_`",
];
const RAPIDJSON_GENERATED_SURFACE_PLACEHOLDER_MARKER: &str = "/// Placeholder for C++";
const RAPIDJSON_GENERATED_SURFACE_RAPIDJSON_PLACEHOLDER_MARKER: &str =
    "/// Placeholder for C++ `rapidjson::";
const RAPIDJSON_GENERATED_SURFACE_C_VOID_ALIAS_MARKER: &str = "= std::ffi::c_void;";
const RAPIDJSON_GENERATED_SURFACE_PARSE_UNSPECIFIC_MARKER: &str =
    "kParseErrorUnspecificSyntaxError";
const RAPIDJSON_NATIVE_LOG_FILES: &[&str] = &[
    "compile_condense.status",
    "compile_condense.stdout",
    "compile_condense.stderr",
    "run_condense.status",
    "run_condense.stdout",
    "run_condense.stderr",
    "compile_pretty.status",
    "compile_pretty.stdout",
    "compile_pretty.stderr",
    "run_pretty.status",
    "run_pretty.stdout",
    "run_pretty.stderr",
    "native_baseline_manifest.txt",
];
const RAPIDJSON_COMMAND_PLAN_LOG_FILES: &[&str] = &["no_stl_examples_manifest.txt"];
const RAPIDJSON_FRAGILE_CONDENSE_REPLAY_LOG_FILES: &[&str] = &[
    "rustc_fragile_condense.status",
    "rustc_fragile_condense.stdout",
    "rustc_fragile_condense.stderr",
    "fragile_condense_replay_manifest.txt",
];
const RAPIDJSON_FRAGILEC_DRIVER_LOG_FILES: &[&str] = &[
    "compile_condense_driver.status",
    "compile_condense_driver.stdout",
    "compile_condense_driver.stderr",
    "run_condense_driver.status",
    "run_condense_driver.stdout",
    "run_condense_driver.stderr",
    "compile_pretty_driver.status",
    "compile_pretty_driver.stdout",
    "compile_pretty_driver.stderr",
    "run_pretty_driver.status",
    "run_pretty_driver.stdout",
    "run_pretty_driver.stderr",
    "fragilec_driver.log",
    "fragilec_driver_manifest.txt",
];
const RAPIDJSON_STRICT_CMAKE_NO_TESTS_LOG_FILES: &[&str] = &[
    "cmake_configure.status",
    "cmake_configure.stdout",
    "cmake_configure.stderr",
    "cmake_build.status",
    "cmake_build.stdout",
    "cmake_build.stderr",
    "fragilec_driver.log",
    "first_failing_compile_command.txt",
    "first_failing_compile_stderr.txt",
    "first_failing_compile_class.txt",
    "strict_cmake_no_tests_manifest.txt",
];
const RAPIDJSON_STRICT_CMAKE_LOCAL_FIXTURE_LOG_FILES: &[&str] = &[
    "cmake_configure.status",
    "cmake_configure.stdout",
    "cmake_configure.stderr",
    "cmake_build.status",
    "cmake_build.stdout",
    "cmake_build.stderr",
    "fragilec_driver.log",
    "first_failing_compile_command.txt",
    "first_failing_compile_stderr.txt",
    "first_failing_compile_class.txt",
    "strict_cmake_local_fixture_manifest.txt",
];
const RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_LOCAL_FIXTURE_LOG_FILES: &[&str] = &[
    "strict_cmake_backend_matrix_local_fixture_manifest.txt",
    "backend_libclang/cmake_configure.status",
    "backend_libclang/cmake_configure.stdout",
    "backend_libclang/cmake_configure.stderr",
    "backend_libclang/cmake_build.status",
    "backend_libclang/cmake_build.stdout",
    "backend_libclang/cmake_build.stderr",
    "backend_libclang/fragilec_driver.log",
    "backend_libclang/first_failing_compile_command.txt",
    "backend_libclang/first_failing_compile_stderr.txt",
    "backend_libclang/first_failing_compile_class.txt",
    "backend_hybrid/cmake_configure.status",
    "backend_hybrid/cmake_configure.stdout",
    "backend_hybrid/cmake_configure.stderr",
    "backend_hybrid/cmake_build.status",
    "backend_hybrid/cmake_build.stdout",
    "backend_hybrid/cmake_build.stderr",
    "backend_hybrid/fragilec_driver.log",
    "backend_hybrid/first_failing_compile_command.txt",
    "backend_hybrid/first_failing_compile_stderr.txt",
    "backend_hybrid/first_failing_compile_class.txt",
    "backend_libtooling/cmake_configure.status",
    "backend_libtooling/cmake_configure.stdout",
    "backend_libtooling/cmake_configure.stderr",
    "backend_libtooling/cmake_build.status",
    "backend_libtooling/cmake_build.stdout",
    "backend_libtooling/cmake_build.stderr",
    "backend_libtooling/fragilec_driver.log",
    "backend_libtooling/first_failing_compile_command.txt",
    "backend_libtooling/first_failing_compile_stderr.txt",
    "backend_libtooling/first_failing_compile_class.txt",
];
const RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_LOG_FILES: &[&str] = &[
    "strict_cmake_backend_matrix_manifest.txt",
    "backend_libclang/cmake_configure.status",
    "backend_libclang/cmake_configure.stdout",
    "backend_libclang/cmake_configure.stderr",
    "backend_libclang/cmake_build.status",
    "backend_libclang/cmake_build.stdout",
    "backend_libclang/cmake_build.stderr",
    "backend_libclang/fragilec_driver.log",
    "backend_libclang/first_failing_compile_command.txt",
    "backend_libclang/first_failing_compile_stderr.txt",
    "backend_libclang/first_failing_compile_class.txt",
    "backend_libtooling/cmake_configure.status",
    "backend_libtooling/cmake_configure.stdout",
    "backend_libtooling/cmake_configure.stderr",
    "backend_libtooling/cmake_build.status",
    "backend_libtooling/cmake_build.stdout",
    "backend_libtooling/cmake_build.stderr",
    "backend_libtooling/fragilec_driver.log",
    "backend_libtooling/first_failing_compile_command.txt",
    "backend_libtooling/first_failing_compile_stderr.txt",
    "backend_libtooling/first_failing_compile_class.txt",
];
const RAPIDJSON_STRICT_BACKEND_TOGGLE_LOCAL_FIXTURE_LOG_FILES: &[&str] = &[
    "strict_backend_toggle_manifest.txt",
    "backend_libclang/compile.status",
    "backend_libclang/compile.stdout",
    "backend_libclang/compile.stderr",
    "backend_libclang/fragilec_driver.log",
    "backend_libclang/first_failing_compile_command.txt",
    "backend_libclang/first_failing_compile_stderr.txt",
    "backend_libclang/first_failing_compile_class.txt",
    "backend_hybrid/compile.status",
    "backend_hybrid/compile.stdout",
    "backend_hybrid/compile.stderr",
    "backend_hybrid/fragilec_driver.log",
    "backend_hybrid/first_failing_compile_command.txt",
    "backend_hybrid/first_failing_compile_stderr.txt",
    "backend_hybrid/first_failing_compile_class.txt",
    "backend_libtooling/compile.status",
    "backend_libtooling/compile.stdout",
    "backend_libtooling/compile.stderr",
    "backend_libtooling/fragilec_driver.log",
    "backend_libtooling/first_failing_compile_command.txt",
    "backend_libtooling/first_failing_compile_stderr.txt",
    "backend_libtooling/first_failing_compile_class.txt",
];
const RAPIDJSON_STRICT_CAPITALIZE_CAPTURE_LOG_FILES: &[&str] = &[
    "compile_capitalize.status",
    "compile_capitalize.stdout",
    "compile_capitalize.stderr",
    "fragilec_driver.log",
    "first_failing_compile_command.txt",
    "first_failing_compile_stderr.txt",
    "first_failing_compile_class.txt",
    "strict_capitalize_manifest.txt",
];
const RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_LOG_FILES: &[&str] = &[
    "strict_capitalize_backend_surface_delta_manifest.txt",
    "backend_libclang/compile_capitalize.status",
    "backend_libclang/compile_capitalize.stdout",
    "backend_libclang/compile_capitalize.stderr",
    "backend_libclang/transpile_stage_timing.log",
    "backend_libclang/fragilec_driver.log",
    "backend_libclang/first_failing_compile_command.txt",
    "backend_libclang/first_failing_compile_stderr.txt",
    "backend_libclang/first_failing_compile_class.txt",
    "backend_libtooling/compile_capitalize.status",
    "backend_libtooling/compile_capitalize.stdout",
    "backend_libtooling/compile_capitalize.stderr",
    "backend_libtooling/transpile_stage_timing.log",
    "backend_libtooling/fragilec_driver.log",
    "backend_libtooling/first_failing_compile_command.txt",
    "backend_libtooling/first_failing_compile_stderr.txt",
    "backend_libtooling/first_failing_compile_class.txt",
];
const RAPIDJSON_STRICT_TUTORIAL_BACKEND_SURFACE_DELTA_LOG_FILES: &[&str] = &[
    "strict_tutorial_backend_surface_delta_manifest.txt",
    "backend_libclang/compile_tutorial.status",
    "backend_libclang/compile_tutorial.stdout",
    "backend_libclang/compile_tutorial.stderr",
    "backend_libclang/transpile_stage_timing.log",
    "backend_libclang/fragilec_driver.log",
    "backend_libclang/first_failing_compile_command.txt",
    "backend_libclang/first_failing_compile_stderr.txt",
    "backend_libclang/first_failing_compile_class.txt",
    "backend_libtooling/compile_tutorial.status",
    "backend_libtooling/compile_tutorial.stdout",
    "backend_libtooling/compile_tutorial.stderr",
    "backend_libtooling/transpile_stage_timing.log",
    "backend_libtooling/fragilec_driver.log",
    "backend_libtooling/first_failing_compile_command.txt",
    "backend_libtooling/first_failing_compile_stderr.txt",
    "backend_libtooling/first_failing_compile_class.txt",
];
const RAPIDJSON_STRICT_FILTERKEYDOM_CAPTURE_LOG_FILES: &[&str] = &[
    "compile_filterkeydom.status",
    "compile_filterkeydom.stdout",
    "compile_filterkeydom.stderr",
    "fragilec_driver.log",
    "first_failing_compile_command.txt",
    "first_failing_compile_stderr.txt",
    "first_failing_compile_class.txt",
    "strict_filterkeydom_manifest.txt",
];
const RAPIDJSON_CI_SMOKE_REQUIRED_TEST_INVOCATIONS: &[&str] = &[
    "test_rapidjson_native_no_stl_examples_local_fixture_success",
    "test_rapidjson_no_stl_command_plan_local_fixture_success",
    "test_rapidjson_fragile_condense_single_tu_replay_local_fixture_success",
    "test_rapidjson_fragilec_driver_no_stl_examples_local_fixture_success",
];
const RAPIDJSON_NIGHTLY_REQUIRED_TEST_NAMES: &[&str] = &[
    "test_real_world_rapidjson_fixture_checkout_is_pinned",
    "test_real_world_rapidjson_no_stl_command_plan_generation",
    "test_real_world_rapidjson_native_no_stl_examples_baseline",
    "test_real_world_rapidjson_fragilec_native_no_stl_examples_baseline",
];
const RAPIDJSON_ORDERED_FAILURE_CLASS_LEDGER_MARKERS: &[&str] = &[
    "1) Parser/AST fidelity mismatch in real RapidJSON headers.",
    "2) Duplicate symbol/type emission in single TU output.",
    "3) Placeholder fallback for required rapidjson template types.",
    "4) C/C++ type normalization gaps.",
    "5) Cast/decay/call-shape lowering bugs.",
    "6) Numeric/sign/enum lowering issues.",
    "7) Entrypoint correctness residual (`main` rollback/drop).",
];

fn workspace_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve workspace root")
}

fn ensure_fragilec_binary() -> Result<PathBuf, String> {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    if let Some(path) = BIN.get() {
        return Ok(path.clone());
    }

    let workspace_root = workspace_root_dir();
    let fragilec = workspace_root.join("target/debug/fragilec");
    let output = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("fragile-cli")
        .arg("--bin")
        .arg("fragilec")
        .current_dir(&workspace_root)
        .output()
        .map_err(|e| format!("failed to build fragilec binary: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "failed to build fragilec binary\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let _ = BIN.set(fragilec.clone());
    Ok(fragilec)
}

fn read_workflow_file(file_name: &str) -> Result<String, String> {
    let workflow_path = workspace_root_dir()
        .join(".github")
        .join("workflows")
        .join(file_name);
    fs::read_to_string(&workflow_path)
        .map_err(|e| format!("failed to read workflow {}: {}", workflow_path.display(), e))
}

fn read_todo_file() -> Result<String, String> {
    let todo_path = workspace_root_dir().join("TODO.md");
    fs::read_to_string(&todo_path)
        .map_err(|e| format!("failed to read TODO {}: {}", todo_path.display(), e))
}

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<Output, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git {:?}: {}", args, e))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn git_stdout(args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let output = run_git(args, cwd)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn checkout_has_required_files(repo_dir: &Path, required_paths: &[&str]) -> bool {
    repo_dir.join(".git").exists() && required_paths.iter().all(|rel| repo_dir.join(rel).exists())
}

fn read_head(repo_dir: &Path) -> Option<String> {
    git_stdout(&["rev-parse", "HEAD"], Some(repo_dir)).ok()
}

fn synchronize_pinned_checkout(
    repo_url: &str,
    repo_dir: &Path,
    pinned_commit: &str,
) -> Result<(), String> {
    if !repo_dir.join(".git").exists() {
        if repo_dir.exists() {
            fs::remove_dir_all(repo_dir).map_err(|e| {
                format!(
                    "failed to remove partial checkout {}: {}",
                    repo_dir.display(),
                    e
                )
            })?;
        }

        let repo_dir_str = repo_dir.to_string_lossy().to_string();
        run_git(
            &["clone", "--no-tags", repo_url, repo_dir_str.as_str()],
            None,
        )?;
    }

    run_git(
        &["fetch", "--depth", "1", "origin", pinned_commit],
        Some(repo_dir),
    )?;
    run_git(&["checkout", "--detach", pinned_commit], Some(repo_dir))?;
    Ok(())
}

fn ensure_pinned_checkout(
    repo_url: &str,
    repo_dir: &Path,
    pinned_commit: &str,
    required_paths: &[&str],
) -> Result<PathBuf, String> {
    let repo_dir = repo_dir.to_path_buf();
    if checkout_has_required_files(&repo_dir, required_paths)
        && read_head(&repo_dir).as_deref() == Some(pinned_commit)
    {
        return Ok(repo_dir);
    }

    if let Some(parent) = repo_dir.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create parent dir {}: {}", parent.display(), e))?;
    }

    let lock_path = repo_dir.with_extension("clone.lock");
    let mut have_lock = false;
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lock_path)
    {
        Ok(_) => have_lock = true,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        Err(e) => {
            return Err(format!(
                "failed to create clone lock {}: {}",
                lock_path.display(),
                e
            ));
        }
    }

    if have_lock {
        let result = (|| -> Result<(), String> {
            if repo_dir.exists() && !repo_dir.join(".git").exists() {
                fs::remove_dir_all(&repo_dir).map_err(|e| {
                    format!(
                        "failed to remove stale checkout {}: {}",
                        repo_dir.display(),
                        e
                    )
                })?;
            }

            synchronize_pinned_checkout(repo_url, &repo_dir, pinned_commit)?;

            if !checkout_has_required_files(&repo_dir, required_paths) {
                return Err(format!(
                    "checkout missing required files at {}",
                    repo_dir.display()
                ));
            }

            let head = read_head(&repo_dir).ok_or_else(|| {
                format!(
                    "failed to query HEAD after checkout at {}",
                    repo_dir.display()
                )
            })?;
            if head != pinned_commit {
                return Err(format!(
                    "expected HEAD {} but found {} at {}",
                    pinned_commit,
                    head,
                    repo_dir.display()
                ));
            }
            Ok(())
        })();
        let _ = fs::remove_file(&lock_path);
        result?;
        return Ok(repo_dir);
    }

    for _ in 0..200 {
        if checkout_has_required_files(&repo_dir, required_paths)
            && read_head(&repo_dir).as_deref() == Some(pinned_commit)
        {
            return Ok(repo_dir);
        }
        sleep(Duration::from_millis(100));
    }

    Err(format!(
        "checkout for {} is not ready at {} (lock: {})",
        repo_url,
        repo_dir.display(),
        lock_path.display()
    ))
}

fn ensure_rapidjson_checkout() -> Result<PathBuf, String> {
    ensure_pinned_checkout(
        RAPIDJSON_REPO_URL,
        Path::new(RAPIDJSON_CACHE_DIR),
        RAPIDJSON_PINNED_COMMIT,
        RAPIDJSON_REQUIRED_PATHS,
    )
}

fn status_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn strict_cmake_backend_matrix_build_timeout() -> Duration {
    Duration::from_secs(RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_BUILD_TIMEOUT_SECS)
}

fn strict_backend_surface_delta_compile_timeout() -> Duration {
    Duration::from_secs(RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_TIMEOUT_SECS)
}

#[derive(Debug, Clone, Copy)]
struct StrictSingleTuBackendSurfaceCaptureConfig {
    run_root_prefix: &'static str,
    fixture_name: &'static str,
    log_dir_name: &'static str,
    source_rel_path: &'static str,
    output_obj_name: &'static str,
    compile_step_name: &'static str,
    manifest_file_name: &'static str,
    context_label: &'static str,
}

const STRICT_CAPITALIZE_BACKEND_SURFACE_CAPTURE_CONFIG: StrictSingleTuBackendSurfaceCaptureConfig =
    StrictSingleTuBackendSurfaceCaptureConfig {
        run_root_prefix: RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_DIR,
        fixture_name: "real_world_strict_capitalize_backend_surface_delta",
        log_dir_name: "strict_capitalize_backend_surface_delta_logs",
        source_rel_path: "example/capitalize/capitalize.cpp",
        output_obj_name: "capitalize.o",
        compile_step_name: "compile_capitalize",
        manifest_file_name: "strict_capitalize_backend_surface_delta_manifest.txt",
        context_label: "strict capitalize backend-surface",
    };

const STRICT_TUTORIAL_BACKEND_SURFACE_CAPTURE_CONFIG: StrictSingleTuBackendSurfaceCaptureConfig =
    StrictSingleTuBackendSurfaceCaptureConfig {
        run_root_prefix: RAPIDJSON_STRICT_TUTORIAL_BACKEND_SURFACE_DELTA_DIR,
        fixture_name: "real_world_strict_tutorial_backend_surface_delta",
        log_dir_name: "strict_tutorial_backend_surface_delta_logs",
        source_rel_path: "example/tutorial/tutorial.cpp",
        output_obj_name: "tutorial.o",
        compile_step_name: "compile_tutorial",
        manifest_file_name: "strict_tutorial_backend_surface_delta_manifest.txt",
        context_label: "strict tutorial backend-surface",
    };

fn run_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
    context: &str,
) -> Result<(Output, bool), String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {}", context, e))?;
    let mut child_stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("failed to capture stdout pipe for {}", context))?;
    let mut child_stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("failed to capture stderr pipe for {}", context))?;
    let stdout_thread = thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut stdout = Vec::new();
        child_stdout.read_to_end(&mut stdout)?;
        Ok(stdout)
    });
    let stderr_thread = thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut stderr = Vec::new();
        child_stderr.read_to_end(&mut stderr)?;
        Ok(stderr)
    });

    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("failed waiting for {}: {}", context, e))?
        {
            break status;
        }

        if start.elapsed() >= timeout {
            timed_out = true;
            let _ = child.kill();
            break child
                .wait()
                .map_err(|e| format!("failed to wait on timed-out {}: {}", context, e))?;
        }

        sleep(Duration::from_millis(100));
    };

    let stdout = stdout_thread
        .join()
        .map_err(|_| format!("failed joining stdout capture thread for {}", context))?
        .map_err(|e| format!("failed reading stdout for {}: {}", context, e))?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| format!("failed joining stderr capture thread for {}", context))?
        .map_err(|e| format!("failed reading stderr for {}: {}", context, e))?;
    let mut output = Output {
        status,
        stdout,
        stderr,
    };

    if timed_out {
        if !output.stderr.is_empty() && !output.stderr.ends_with(b"\n") {
            output.stderr.push(b'\n');
        }
        let timeout_msg = format!(
            "command timed out after {}s: {}",
            timeout.as_secs(),
            context
        );
        output.stderr.extend_from_slice(timeout_msg.as_bytes());
        output.stderr.push(b'\n');
    }
    Ok((output, timed_out))
}

fn read_status_file(path: &Path) -> Result<i32, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    raw.trim()
        .parse::<i32>()
        .map_err(|e| format!("failed to parse status file {}: {}", path.display(), e))
}

fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn manifest_line_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|part| {
        let (k, v) = part.split_once('=')?;
        if k == key { Some(v) } else { None }
    })
}

fn manifest_line_i64(line: &str, key: &str) -> Option<i64> {
    manifest_line_value(line, key)?.parse::<i64>().ok()
}

fn manifest_line_bool(line: &str, key: &str) -> Option<bool> {
    match manifest_line_value(line, key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_backend_matrix_delta_snapshot_from_manifest_line(
    backend_line: &str,
    fallback_timeout_incidence_delta_vs_baseline: Option<i64>,
) -> Option<BackendMatrixDeltaSnapshot> {
    Some(BackendMatrixDeltaSnapshot {
        configure_status_delta_vs_baseline: manifest_line_i64(
            backend_line,
            "configure_status_delta_vs_baseline",
        )?,
        build_status_delta_vs_baseline: manifest_line_i64(
            backend_line,
            "build_status_delta_vs_baseline",
        )?,
        class_delta_vs_baseline: manifest_line_bool(backend_line, "class_delta_vs_baseline")?,
        e0425_delta_vs_baseline: manifest_line_i64(backend_line, "e0425_delta_vs_baseline")?,
        timeout_incidence_delta_vs_baseline: manifest_line_i64(
            backend_line,
            "timeout_incidence_delta_vs_baseline",
        )
        .or(fallback_timeout_incidence_delta_vs_baseline)?,
    })
}

fn parse_backend_matrix_delta_snapshot_from_manifest(
    manifest: &str,
    backend_name: &str,
) -> Option<BackendMatrixDeltaSnapshot> {
    let backend_prefix = format!("backend={} ", backend_name);
    let backend_line = manifest
        .lines()
        .find(|line| line.starts_with(backend_prefix.as_str()))?;
    let baseline_line = manifest
        .lines()
        .find(|line| line.starts_with("baseline_backend="));
    let fallback_timeout_incidence_delta_vs_baseline = baseline_line.and_then(|baseline| {
        let backend_timed_out = manifest_line_bool(backend_line, "build_timed_out")?;
        let baseline_timed_out = manifest_line_bool(baseline, "baseline_build_timed_out")?;
        Some(bool_to_i64(backend_timed_out) - bool_to_i64(baseline_timed_out))
    });

    parse_backend_matrix_delta_snapshot_from_manifest_line(
        backend_line,
        fallback_timeout_incidence_delta_vs_baseline,
    )
}

fn compute_backend_matrix_delta_snapshot(
    result: &StrictCmakeBackendReplayResult,
    baseline: &StrictCmakeBackendReplayResult,
) -> BackendMatrixDeltaSnapshot {
    BackendMatrixDeltaSnapshot {
        configure_status_delta_vs_baseline: i64::from(result.configure_status)
            - i64::from(baseline.configure_status),
        build_status_delta_vs_baseline: i64::from(result.build_status)
            - i64::from(baseline.build_status),
        class_delta_vs_baseline: result.first_failure_class != baseline.first_failure_class,
        e0425_delta_vs_baseline: result.first_failure_e0425_count as i64
            - baseline.first_failure_e0425_count as i64,
        timeout_incidence_delta_vs_baseline: bool_to_i64(result.build_timed_out)
            - bool_to_i64(baseline.build_timed_out),
    }
}

fn ensure_backend_matrix_delta_non_increase(
    current: BackendMatrixDeltaSnapshot,
    baseline: BackendMatrixDeltaSnapshot,
) -> Result<(), String> {
    if current.configure_status_delta_vs_baseline > baseline.configure_status_delta_vs_baseline {
        return Err(format!(
            "configure-status delta regressed: current={} baseline={}",
            current.configure_status_delta_vs_baseline, baseline.configure_status_delta_vs_baseline
        ));
    }
    if current.build_status_delta_vs_baseline > baseline.build_status_delta_vs_baseline {
        return Err(format!(
            "build-status delta regressed: current={} baseline={}",
            current.build_status_delta_vs_baseline, baseline.build_status_delta_vs_baseline
        ));
    }
    if bool_to_i64(current.class_delta_vs_baseline) > bool_to_i64(baseline.class_delta_vs_baseline)
    {
        return Err(format!(
            "first-failure-class delta regressed: current={} baseline={}",
            current.class_delta_vs_baseline, baseline.class_delta_vs_baseline
        ));
    }
    if current.e0425_delta_vs_baseline > baseline.e0425_delta_vs_baseline {
        return Err(format!(
            "E0425 delta regressed: current={} baseline={}",
            current.e0425_delta_vs_baseline, baseline.e0425_delta_vs_baseline
        ));
    }
    if current.timeout_incidence_delta_vs_baseline > baseline.timeout_incidence_delta_vs_baseline {
        return Err(format!(
            "timeout-incidence delta regressed: current={} baseline={}",
            current.timeout_incidence_delta_vs_baseline,
            baseline.timeout_incidence_delta_vs_baseline
        ));
    }

    Ok(())
}

fn latest_completed_backend_matrix_delta_baseline(
    backend_name: &str,
) -> Option<(PathBuf, BackendMatrixDeltaSnapshot)> {
    let parent = Path::new(RAPIDJSON_STRICT_CMAKE_NO_TESTS_BACKEND_MATRIX_DIR)
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"));
    let run_root_prefix = format!("{}_", RAPIDJSON_STRICT_CMAKE_NO_TESTS_BACKEND_MATRIX_DIR);
    let mut candidates: Vec<(SystemTime, PathBuf, BackendMatrixDeltaSnapshot)> = Vec::new();

    let entries = fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let path_text = path.to_string_lossy();
        if !path_text.starts_with(run_root_prefix.as_str()) {
            continue;
        }

        let manifest_path = path
            .join("strict_cmake_backend_matrix_logs")
            .join("strict_cmake_backend_matrix_manifest.txt");
        if !manifest_path.exists() {
            continue;
        }

        let manifest = match fs::read_to_string(&manifest_path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        if !manifest.contains("fixture=real_world_strict_cmake_backend_matrix_first_failure") {
            continue;
        }

        let snapshot = match parse_backend_matrix_delta_snapshot_from_manifest(
            manifest.as_str(),
            backend_name,
        ) {
            Some(parsed) => parsed,
            None => continue,
        };
        let modified = match fs::metadata(&manifest_path).and_then(|meta| meta.modified()) {
            Ok(ts) => ts,
            Err(_) => UNIX_EPOCH,
        };

        candidates.push((modified, manifest_path, snapshot));
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates
        .into_iter()
        .next()
        .map(|(_, path, snapshot)| (path, snapshot))
}

fn rapidjson_pretty_output_matches_expected(pretty_stdout: &str) -> bool {
    pretty_stdout.contains("\n")
        && pretty_stdout.contains("\"msg\": \"hi\"")
        && pretty_stdout.contains("    \"a\": 1")
}

fn write_command_capture_raw(
    log_dir: &Path,
    step: &str,
    status: i32,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    fs::write(
        log_dir.join(format!("{}.status", step)),
        format!("{}\n", status),
    )
    .map_err(|e| format!("failed to write {}.status: {}", step, e))?;
    fs::write(log_dir.join(format!("{}.stdout", step)), stdout)
        .map_err(|e| format!("failed to write {}.stdout: {}", step, e))?;
    fs::write(log_dir.join(format!("{}.stderr", step)), stderr)
        .map_err(|e| format!("failed to write {}.stderr: {}", step, e))?;
    Ok(())
}

fn write_command_capture(log_dir: &Path, step: &str, output: &Output) -> Result<(), String> {
    write_command_capture_raw(
        log_dir,
        step,
        status_code(output),
        &output.stdout,
        &output.stderr,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FragilecDriverInvocation {
    cwd: String,
    args: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StrictBackendReplayResult {
    backend_name: &'static str,
    compile_status: i32,
    first_failure_class: String,
    first_failure_e0425_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StrictCmakeBackendReplayResult {
    backend_name: &'static str,
    configure_status: i32,
    build_status: i32,
    build_timed_out: bool,
    first_failure_class: String,
    first_failure_e0425_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackendMatrixDeltaSnapshot {
    configure_status_delta_vs_baseline: i64,
    build_status_delta_vs_baseline: i64,
    class_delta_vs_baseline: bool,
    e0425_delta_vs_baseline: i64,
    timeout_incidence_delta_vs_baseline: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedSurfaceInventory {
    line_count: usize,
    placeholder_count: usize,
    rapidjson_placeholder_count: usize,
    c_void_alias_count: usize,
    parse_unspecific_syntax_error_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TranspileStageTimingSnapshot {
    parse_ms: Option<u128>,
    export_ms: Option<u128>,
    enrichment_ms: Option<u128>,
    codegen_ms: Option<u128>,
    total_ms: Option<u128>,
    last_stage_started: Option<String>,
    last_stage_completed: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StrictCapitalizeBackendSurfaceReplayResult {
    backend_name: &'static str,
    compile_status: i32,
    compile_timed_out: bool,
    first_failure_class: String,
    first_failure_e0425_count: usize,
    sidecar_path: PathBuf,
    sidecar_exists: bool,
    generated_surface_inventory: Option<GeneratedSurfaceInventory>,
    transpile_stage_timing_path: PathBuf,
    transpile_stage_timing_exists: bool,
    transpile_stage_timing: TranspileStageTimingSnapshot,
}

fn parse_fragilec_driver_invocations(driver_log: &str) -> Vec<FragilecDriverInvocation> {
    let mut invocations = Vec::new();
    let mut current_cwd: Option<String> = None;

    for line in driver_log.lines() {
        if let Some(rest) = line.strip_prefix("cwd=") {
            current_cwd = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("args=") {
            invocations.push(FragilecDriverInvocation {
                cwd: current_cwd
                    .clone()
                    .unwrap_or_else(|| "<unknown cwd>".to_string()),
                args: rest.trim().to_string(),
            });
        }
    }

    invocations
}

fn canonicalize_failed_source_path(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_end_matches('.')
        .to_string()
}

fn first_failed_source_path_from_stream(stream: &str) -> Option<String> {
    for line in stream.lines() {
        if let Some(rest) = line.strip_prefix("[fragilec] fragile rustc object compile failed for ")
        {
            let path = canonicalize_failed_source_path(rest);
            if !path.is_empty() {
                return Some(path);
            }
        }
        if let Some(rest) = line.strip_prefix("Error while processing ") {
            let path = canonicalize_failed_source_path(rest);
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

fn first_failed_source_path(build_stdout: &str, build_stderr: &str) -> Option<String> {
    first_failed_source_path_from_stream(build_stderr)
        .or_else(|| first_failed_source_path_from_stream(build_stdout))
}

fn first_failing_compile_command_from_driver_log(
    driver_log: &str,
    source_path_hint: Option<&str>,
) -> Option<String> {
    let invocations = parse_fragilec_driver_invocations(driver_log);
    if let Some(source_path) = source_path_hint {
        let source_basename = Path::new(source_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(str::to_string);
        if let Some(inv) = invocations.iter().find(|inv| {
            if inv.args.contains(source_path) {
                return true;
            }
            let Some(source_basename) = source_basename.as_deref() else {
                return false;
            };
            inv.args.split_whitespace().any(|token| {
                let cleaned = token.trim_matches('"');
                Path::new(cleaned)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| name == source_basename)
                    .unwrap_or(false)
            })
        }) {
            return Some(format!("cwd={}\nargs={}", inv.cwd, inv.args));
        }
    }
    invocations
        .last()
        .map(|inv| format!("cwd={}\nargs={}", inv.cwd, inv.args))
}

fn source_scoped_failure_payload(stream: &str, source_path: &str) -> Option<String> {
    fn find_next_failure_marker(stream: &str, from: usize) -> Option<usize> {
        let mut earliest: Option<usize> = None;
        for marker in [
            "\n[fragilec] fragile rustc object compile failed for ",
            "\nError while processing ",
        ] {
            if let Some(rel) = stream[from..].find(marker) {
                let marker_start = from + rel + 1;
                earliest = Some(match earliest {
                    Some(current) => current.min(marker_start),
                    None => marker_start,
                });
            }
        }
        earliest
    }

    for marker in [
        format!(
            "[fragilec] fragile rustc object compile failed for {}",
            source_path
        ),
        format!("Error while processing {}.", source_path),
        format!("Error while processing {}", source_path),
    ] {
        if let Some(start) = stream.find(marker.as_str()) {
            let end = find_next_failure_marker(stream, start + marker.len()).unwrap_or(stream.len());
            let scoped = stream[start..end].trim();
            if !scoped.is_empty() {
                return Some(scoped.to_string());
            }
        }
    }
    None
}

fn select_first_failing_compile_capture(
    driver_log: &str,
    build_failed: bool,
    build_stdout: &str,
    build_stderr: &str,
) -> (String, String) {
    if !build_failed {
        return ("<none>".to_string(), "<none>".to_string());
    }

    let failed_source_path = first_failed_source_path(build_stdout, build_stderr);
    let command = first_failing_compile_command_from_driver_log(
        driver_log,
        failed_source_path.as_deref(),
    )
        .unwrap_or_else(|| "<unavailable>".to_string());
    let stderr = if let Some(source_path) = failed_source_path.as_deref() {
        source_scoped_failure_payload(build_stderr, source_path)
            .or_else(|| source_scoped_failure_payload(build_stdout, source_path))
            .or_else(|| {
                if !build_stderr.trim().is_empty() {
                    Some(build_stderr.trim().to_string())
                } else if !build_stdout.trim().is_empty() {
                    Some(build_stdout.trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "<none>".to_string())
    } else if !build_stderr.trim().is_empty() {
        build_stderr.trim().to_string()
    } else if !build_stdout.trim().is_empty() {
        build_stdout.trim().to_string()
    } else {
        "<none>".to_string()
    };
    (command, stderr)
}

fn classify_first_failing_compile_stderr(first_stderr: &str) -> &'static str {
    let stderr = first_stderr.trim();
    if stderr.is_empty() || stderr == "<none>" {
        return "none";
    }
    if stderr.contains("error[E0428]") {
        return "duplicate_definition_e0428";
    }
    if stderr.contains("error[E0425]") {
        return "unresolved_name_or_type_e0425";
    }
    if stderr.contains("error[E") {
        return "other_rustc_error";
    }
    "non_rustc_error"
}

fn count_error_e0425_occurrences(text: &str) -> usize {
    text.match_indices("error[E0425]").count()
}

fn collect_generated_surface_inventory(generated_rs: &str) -> GeneratedSurfaceInventory {
    GeneratedSurfaceInventory {
        line_count: generated_rs.lines().count(),
        placeholder_count: generated_rs
            .match_indices(RAPIDJSON_GENERATED_SURFACE_PLACEHOLDER_MARKER)
            .count(),
        rapidjson_placeholder_count: generated_rs
            .match_indices(RAPIDJSON_GENERATED_SURFACE_RAPIDJSON_PLACEHOLDER_MARKER)
            .count(),
        c_void_alias_count: generated_rs
            .match_indices(RAPIDJSON_GENERATED_SURFACE_C_VOID_ALIAS_MARKER)
            .count(),
        parse_unspecific_syntax_error_count: generated_rs
            .match_indices(RAPIDJSON_GENERATED_SURFACE_PARSE_UNSPECIFIC_MARKER)
            .count(),
    }
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "na".to_string())
}

fn format_optional_u128(value: Option<u128>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "na".to_string())
}

fn format_optional_i64(value: Option<i64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "na".to_string())
}

fn format_optional_str(value: Option<&str>) -> String {
    value
        .map(str::to_string)
        .unwrap_or_else(|| "na".to_string())
}

fn parse_key_value_token<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(format!("{key}=").as_str()))
}

fn assign_transpile_stage_elapsed(
    snapshot: &mut TranspileStageTimingSnapshot,
    stage: &str,
    elapsed_ms: Option<u128>,
) {
    match stage {
        "parse" => snapshot.parse_ms = elapsed_ms,
        "export" => snapshot.export_ms = elapsed_ms,
        "enrichment" => snapshot.enrichment_ms = elapsed_ms,
        "codegen" => snapshot.codegen_ms = elapsed_ms,
        _ => {}
    }
}

fn parse_transpile_stage_timing_trace(
    path: &Path,
) -> Result<(bool, TranspileStageTimingSnapshot), String> {
    if !path.exists() {
        return Ok((false, TranspileStageTimingSnapshot::default()));
    }
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read transpile stage timing trace {}: {}",
            path.display(),
            e
        )
    })?;
    let mut snapshot = TranspileStageTimingSnapshot::default();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(status) = line.strip_prefix("status=") {
            snapshot.status = Some(status.to_string());
            continue;
        }
        if line.starts_with("event=stage_start ") {
            if let Some(stage) = parse_key_value_token(line, "stage") {
                snapshot.last_stage_started = Some(stage.to_string());
            }
            continue;
        }
        if line.starts_with("event=stage_end ") || line.starts_with("event=stage_skip ") {
            if let Some(stage) = parse_key_value_token(line, "stage") {
                let elapsed_ms = parse_key_value_token(line, "elapsed_ms")
                    .and_then(|value| value.parse::<u128>().ok());
                assign_transpile_stage_elapsed(&mut snapshot, stage, elapsed_ms);
                snapshot.last_stage_completed = Some(stage.to_string());
            }
            continue;
        }
        if line.starts_with("summary ") {
            snapshot.parse_ms = parse_key_value_token(line, "parse_ms")
                .and_then(|value| value.parse::<u128>().ok());
            snapshot.export_ms = parse_key_value_token(line, "export_ms")
                .and_then(|value| value.parse::<u128>().ok());
            snapshot.enrichment_ms = parse_key_value_token(line, "enrichment_ms")
                .and_then(|value| value.parse::<u128>().ok());
            snapshot.codegen_ms = parse_key_value_token(line, "codegen_ms")
                .and_then(|value| value.parse::<u128>().ok());
            snapshot.total_ms = parse_key_value_token(line, "total_ms")
                .and_then(|value| value.parse::<u128>().ok());
            continue;
        }
    }

    Ok((true, snapshot))
}

fn write_first_failing_compile_capture_files(
    log_dir: &Path,
    first_command: &str,
    first_stderr: &str,
) -> Result<(), String> {
    fs::write(
        log_dir.join("first_failing_compile_command.txt"),
        format!("{}\n", first_command),
    )
    .map_err(|e| {
        format!(
            "failed to write first_failing_compile_command.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;
    fs::write(
        log_dir.join("first_failing_compile_stderr.txt"),
        format!("{}\n", first_stderr),
    )
    .map_err(|e| {
        format!(
            "failed to write first_failing_compile_stderr.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;
    Ok(())
}

fn write_first_failing_compile_class_file(log_dir: &Path, class: &str) -> Result<(), String> {
    fs::write(
        log_dir.join("first_failing_compile_class.txt"),
        format!("{}\n", class),
    )
    .map_err(|e| {
        format!(
            "failed to write first_failing_compile_class.txt in {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove {}: {}", path.display(), e))?;
    }
    fs::create_dir_all(path).map_err(|e| format!("failed to create {}: {}", path.display(), e))
}

fn unique_prefixed_dir(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX_EPOCH")
        .as_nanos();
    PathBuf::from(format!("{prefix}_{}_{}", std::process::id(), now))
}

fn compile_example(
    source_dir: &Path,
    source_rel: &str,
    output_path: &Path,
    log_dir: &Path,
    step_name: &str,
) -> Result<(), String> {
    let mut cmd = Command::new("c++");
    cmd.arg("-std=c++11")
        .arg("-O2")
        .arg("-DNDEBUG")
        .arg("-DRAPIDJSON_HAS_STDSTRING=0")
        .arg("-I")
        .arg(source_dir.join("include"))
        .arg(source_dir.join(source_rel))
        .arg("-o")
        .arg(output_path);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run C++ compiler for {}: {}", source_rel, e))?;
    write_command_capture(log_dir, step_name, &output)?;
    if !output.status.success() {
        return Err(format!(
            "C++ compile failed for {} with status {} (logs: {})",
            source_rel,
            status_code(&output),
            log_dir.display()
        ));
    }

    Ok(())
}

fn compile_transpiled_rust_lib(
    transpiled_rs: &Path,
    output_rlib: &Path,
    log_dir: &Path,
    step_name: &str,
) -> Result<(), String> {
    let output = Command::new("rustc")
        .env("RUSTC_BOOTSTRAP", "1")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("--crate-type")
        .arg("lib")
        .arg(transpiled_rs)
        .arg("-o")
        .arg(output_rlib)
        .output()
        .map_err(|e| format!("failed to run rustc for {}: {}", transpiled_rs.display(), e))?;
    write_command_capture(log_dir, step_name, &output)?;
    if !output.status.success() {
        return Err(format!(
            "fragile rustc single-tu compile failed with status {} (logs: {})\nstderr:\n{}",
            status_code(&output),
            log_dir.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn transpile_rapidjson_condense_source(
    source_dir: &Path,
    source_path: &Path,
) -> Result<String, String> {
    let include_paths = vec![source_dir.join("include").to_string_lossy().to_string()];
    let parser = ClangParser::with_paths_and_defines(include_paths, Vec::new()).map_err(|e| {
        format!(
            "failed to create parser for {}: {}",
            source_path.display(),
            e
        )
    })?;
    let ast = parser
        .parse_file(source_path)
        .map_err(|e| format!("failed to parse {}: {}", source_path.display(), e))?;
    Ok(AstCodeGen::new().generate(&ast.translation_unit))
}

fn run_example_with_stdin(
    binary_path: &Path,
    stdin_payload: &str,
    log_dir: &Path,
    step_name: &str,
) -> Result<Output, String> {
    let mut child = Command::new(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to execute {}: {}", binary_path.display(), e))?;

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(stdin_payload.as_bytes()) {
            // Some fixture binaries emit fixed output and may exit before reading stdin.
            if e.kind() != ErrorKind::BrokenPipe {
                return Err(format!(
                    "failed to write stdin for {}: {}",
                    binary_path.display(),
                    e
                ));
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for {}: {}", binary_path.display(), e))?;
    write_command_capture(log_dir, step_name, &output)?;
    Ok(output)
}

fn run_native_no_stl_examples_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let condense_bin = source_dir.join("condense_fragile_baseline");
    compile_example(
        source_dir,
        "example/condense/condense.cpp",
        &condense_bin,
        log_dir,
        "compile_condense",
    )?;

    let condense_output = run_example_with_stdin(
        &condense_bin,
        RAPIDJSON_SAMPLE_JSON,
        log_dir,
        "run_condense",
    )?;
    if !condense_output.status.success() {
        return Err(format!(
            "condense example failed with status {} (logs: {})",
            status_code(&condense_output),
            log_dir.display()
        ));
    }

    let condense_stdout = String::from_utf8_lossy(&condense_output.stdout);
    if condense_stdout.trim() != RAPIDJSON_EXPECTED_CONDENSE_OUTPUT {
        return Err(format!(
            "condense output mismatch: expected `{}` got `{}`",
            RAPIDJSON_EXPECTED_CONDENSE_OUTPUT,
            condense_stdout.trim()
        ));
    }

    let pretty_bin = source_dir.join("pretty_fragile_baseline");
    compile_example(
        source_dir,
        "example/pretty/pretty.cpp",
        &pretty_bin,
        log_dir,
        "compile_pretty",
    )?;

    let pretty_output =
        run_example_with_stdin(&pretty_bin, RAPIDJSON_SAMPLE_JSON, log_dir, "run_pretty")?;
    if !pretty_output.status.success() {
        return Err(format!(
            "pretty example failed with status {} (logs: {})",
            status_code(&pretty_output),
            log_dir.display()
        ));
    }

    let pretty_stdout = String::from_utf8_lossy(&pretty_output.stdout);
    if !(pretty_stdout.contains("\n")
        && pretty_stdout.contains("\"msg\": \"hi\"")
        && pretty_stdout.contains("    \"a\": 1"))
    {
        return Err(format!(
            "pretty output did not look pretty-formatted, got:\n{}",
            pretty_stdout
        ));
    }

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nexamples_count={}\nexample=condense compile_status=0 run_status=0\nexample=pretty compile_status=0 run_status=0\n",
        source_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        RAPIDJSON_NO_STL_EXAMPLES.len()
    );
    fs::write(log_dir.join("native_baseline_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write native_baseline_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(())
}

fn compile_example_with_cxx_env(
    source_dir: &Path,
    source_rel: &str,
    output_path: &Path,
    log_dir: &Path,
    step_name: &str,
    cxx: &Path,
    driver_log: &Path,
) -> Result<Output, String> {
    let source_arg = source_dir.join(source_rel);
    let output = Command::new("sh")
        .arg("-c")
        .arg("\"$CXX\" -std=c++11 -O2 -DNDEBUG -DRAPIDJSON_HAS_STDSTRING=0 -Iinclude \"$SRC\" -o \"$OUT\"")
        .current_dir(source_dir)
        .env("CXX", cxx.to_string_lossy().to_string())
        .env("SRC", source_arg.to_string_lossy().to_string())
        .env("OUT", output_path.to_string_lossy().to_string())
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| {
            format!(
                "failed to run fragilec-driver compile for {}: {}",
                source_arg.display(),
                e
            )
        })?;
    write_command_capture(log_dir, step_name, &output)?;
    Ok(output)
}

fn run_fragilec_driver_no_stl_examples_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let fragilec = ensure_fragilec_binary()?;
    let driver_log = log_dir.join("fragilec_driver.log");
    fs::write(&driver_log, "").map_err(|e| {
        format!(
            "failed to initialize fragilec driver log {}: {}",
            driver_log.display(),
            e
        )
    })?;

    let condense_bin = source_dir.join("condense_fragilec_driver");
    let condense_compile = compile_example_with_cxx_env(
        source_dir,
        "example/condense/condense.cpp",
        &condense_bin,
        log_dir,
        "compile_condense_driver",
        &fragilec,
        &driver_log,
    )?;
    if !condense_compile.status.success() {
        let stderr = String::from_utf8_lossy(&condense_compile.stderr);
        return Err(format!(
            "fragilec-driver condense compile failed with status {} (logs: {})\nstderr:\n{}",
            status_code(&condense_compile),
            log_dir.display(),
            stderr
        ));
    }
    if !condense_bin.exists() {
        return Err(format!(
            "fragilec-driver condense compile did not emit output binary {}",
            condense_bin.display()
        ));
    }
    let condense_output = run_example_with_stdin(
        &condense_bin,
        RAPIDJSON_SAMPLE_JSON,
        log_dir,
        "run_condense_driver",
    )?;

    let pretty_bin = source_dir.join("pretty_fragilec_driver");
    let pretty_compile = compile_example_with_cxx_env(
        source_dir,
        "example/pretty/pretty.cpp",
        &pretty_bin,
        log_dir,
        "compile_pretty_driver",
        &fragilec,
        &driver_log,
    )?;
    if !pretty_compile.status.success() {
        let stderr = String::from_utf8_lossy(&pretty_compile.stderr);
        return Err(format!(
            "fragilec-driver pretty compile failed with status {} (logs: {})\nstderr:\n{}",
            status_code(&pretty_compile),
            log_dir.display(),
            stderr
        ));
    }
    if !pretty_bin.exists() {
        return Err(format!(
            "fragilec-driver pretty compile did not emit output binary {}",
            pretty_bin.display()
        ));
    }
    let pretty_output = run_example_with_stdin(
        &pretty_bin,
        RAPIDJSON_SAMPLE_JSON,
        log_dir,
        "run_pretty_driver",
    )?;

    let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
        format!(
            "failed to read fragilec driver log {}: {}",
            driver_log.display(),
            e
        )
    })?;
    if driver_log_content.trim().is_empty() {
        return Err(format!(
            "fragilec driver log {} is empty; expected compiler invocations",
            driver_log.display()
        ));
    }

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nfragilec={}\nmode=strict\nexamples_count={}\nexample=condense compile_status={} run_status={}\nexample=pretty compile_status={} run_status={}\n",
        source_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        fragilec.display(),
        RAPIDJSON_NO_STL_EXAMPLES.len(),
        status_code(&condense_compile),
        status_code(&condense_output),
        status_code(&pretty_compile),
        status_code(&pretty_output)
    );
    fs::write(log_dir.join("fragilec_driver_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write fragilec_driver_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(())
}

fn run_no_stl_command_plan_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let mut manifest = String::new();
    manifest.push_str(&format!(
        "source_dir={}\npinned_commit={}\nexample_count={}\n",
        source_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        RAPIDJSON_NO_STL_EXAMPLES.len()
    ));

    for (idx, (name, source_rel)) in RAPIDJSON_NO_STL_EXAMPLES.iter().enumerate() {
        manifest.push_str(&format!(
            "compile[{idx}]=c++ -std=c++11 -O2 -DNDEBUG -DRAPIDJSON_HAS_STDSTRING=0 -Iinclude {source_rel} -o {name}\n"
        ));
        manifest.push_str(&format!("run[{idx}]=echo '<sample json>' | ./{name}\n"));
    }

    fs::write(log_dir.join("no_stl_examples_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write no_stl_examples_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_fragile_condense_single_tu_replay_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let source_path = source_dir.join("example/condense/condense.cpp");
    if !source_path.exists() {
        return Err(format!(
            "rapidjson condense source is missing at {}",
            source_path.display()
        ));
    }

    let transpiled = transpile_rapidjson_condense_source(source_dir, &source_path)?;
    let transpiled_rs = log_dir.join("fragile_condense_transpiled.rs");
    fs::write(&transpiled_rs, transpiled)
        .map_err(|e| format!("failed to write {}: {}", transpiled_rs.display(), e))?;

    let rlib_path = log_dir.join("fragile_condense.rlib");
    compile_transpiled_rust_lib(
        &transpiled_rs,
        &rlib_path,
        log_dir,
        "rustc_fragile_condense",
    )?;

    let object_size = fs::metadata(&rlib_path)
        .map_err(|e| format!("failed to stat {}: {}", rlib_path.display(), e))?
        .len();
    if object_size == 0 {
        return Err(format!(
            "fragile replay output {} is empty",
            rlib_path.display()
        ));
    }

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nsource=example/condense/condense.cpp\ntranspiled={}\noutput={}\noutput_size={}\n",
        source_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        transpiled_rs.display(),
        rlib_path.display(),
        object_size
    );
    fs::write(
        log_dir.join("fragile_condense_replay_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write fragile_condense_replay_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(())
}

fn run_rapidjson_native_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_NATIVE_BASELINE_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "native baseline worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("native_logs");
    run_native_no_stl_examples_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_rapidjson_fragilec_driver_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_FRAGILEC_DRIVER_BASELINE_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "fragilec-driver worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_fragilec_driver_no_stl_examples_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_rapidjson_no_stl_command_plan() -> Result<PathBuf, String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_COMMAND_PLAN_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "command-plan worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("command_plan_logs");
    run_no_stl_command_plan_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_rapidjson_fragile_condense_single_tu_replay() -> Result<PathBuf, String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_FRAGILE_CONDENSE_REPLAY_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "fragile replay worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("replay_logs");
    run_fragile_condense_single_tu_replay_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

/// Returns (log_dir, build_dir) where build_dir contains the CMake build output (bin/).
fn run_rapidjson_strict_cmake_no_tests_full_build_capture() -> Result<(PathBuf, PathBuf), String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_STRICT_CMAKE_NO_TESTS_BUILD_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "strict cmake worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("strict_cmake_logs");
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    let fragilec = ensure_fragilec_binary()?;
    let driver_log = log_dir.join("fragilec_driver.log");
    fs::write(&driver_log, "")
        .map_err(|e| format!("failed to initialize fragilec driver log: {}", e))?;

    let build_dir = worktree_dir.join("build_fragilec_strict");
    fs::create_dir_all(&build_dir)
        .map_err(|e| format!("failed to create build dir {}: {}", build_dir.display(), e))?;

    let configure_output = Command::new("cmake")
        .arg("-DRAPIDJSON_BUILD_TESTS=OFF")
        .arg("..")
        .current_dir(&build_dir)
        .env("CXX", fragilec.to_string_lossy().to_string())
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("failed to run rapidjson strict cmake configure: {}", e))?;
    write_command_capture(&log_dir, "cmake_configure", &configure_output)?;
    if !configure_output.status.success() {
        return Err(format!(
            "rapidjson strict cmake configure failed with status {} (logs: {})",
            status_code(&configure_output),
            log_dir.display()
        ));
    }

    let build_output = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .arg("--verbose")
        .arg("--")
        .arg("-j1")
        .arg("-k")
        .current_dir(&build_dir)
        .env("CXX", fragilec.to_string_lossy().to_string())
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("failed to run rapidjson strict cmake build: {}", e))?;
    write_command_capture(&log_dir, "cmake_build", &build_output)?;

    let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
        format!(
            "failed to read fragilec driver log {}: {}",
            driver_log.display(),
            e
        )
    })?;
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    let build_stderr = String::from_utf8_lossy(&build_output.stderr);
    let (first_command, first_stderr) = select_first_failing_compile_capture(
        &driver_log_content,
        !build_output.status.success(),
        &build_stdout,
        &build_stderr,
    );
    write_first_failing_compile_capture_files(&log_dir, &first_command, &first_stderr)?;
    let first_failure_class = classify_first_failing_compile_stderr(&first_stderr);
    write_first_failing_compile_class_file(&log_dir, first_failure_class)?;

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nfragilec={}\nmode=strict\nconfigure_status={}\nbuild_status={}\nfirst_failing_compile_command_file=first_failing_compile_command.txt\nfirst_failing_compile_stderr_file=first_failing_compile_stderr.txt\nfirst_failing_compile_class_file=first_failing_compile_class.txt\nfirst_failing_compile_class={}\n",
        worktree_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        fragilec.display(),
        status_code(&configure_output),
        status_code(&build_output),
        first_failure_class
    );
    fs::write(log_dir.join("strict_cmake_no_tests_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write strict_cmake_no_tests_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok((log_dir, build_dir))
}

fn run_rapidjson_strict_cmake_no_tests_backend_matrix_capture(
) -> Result<(PathBuf, Vec<StrictCmakeBackendReplayResult>), String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = unique_prefixed_dir(RAPIDJSON_STRICT_CMAKE_NO_TESTS_BACKEND_MATRIX_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "strict cmake backend-matrix worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("strict_cmake_backend_matrix_logs");
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    let fragilec = ensure_fragilec_binary()?;

    let backends: [(&str, &str); 2] = [("libclang", "libclang"), ("libtooling", "libtooling")];
    let build_timeout = strict_cmake_backend_matrix_build_timeout();
    let mut results = Vec::new();
    for (backend_name, backend_env_value) in backends {
        let backend_log_dir = log_dir.join(format!("backend_{backend_name}"));
        fs::create_dir_all(&backend_log_dir).map_err(|e| {
            format!(
                "failed to create strict cmake backend-matrix backend log dir {}: {}",
                backend_log_dir.display(),
                e
            )
        })?;
        let driver_log = backend_log_dir.join("fragilec_driver.log");
        fs::write(&driver_log, "").map_err(|e| {
            format!(
                "failed to initialize strict cmake backend-matrix fragilec driver log {}: {}",
                driver_log.display(),
                e
            )
        })?;

        let build_dir = worktree_dir.join(format!("build_fragilec_strict_{backend_name}"));
        fs::create_dir_all(&build_dir)
            .map_err(|e| format!("failed to create build dir {}: {}", build_dir.display(), e))?;

        let configure_output = Command::new("cmake")
            .arg("-DRAPIDJSON_BUILD_TESTS=OFF")
            .arg("..")
            .current_dir(&build_dir)
            .env("CXX", fragilec.to_string_lossy().to_string())
            .env("FRAGILEC_MODE", "strict")
            .env("FRAGILEC_PARSER_BACKEND", backend_env_value)
            .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
            .output()
            .map_err(|e| {
                format!(
                    "failed to run strict cmake backend-matrix configure for {}: {}",
                    backend_name, e
                )
            })?;
        write_command_capture(&backend_log_dir, "cmake_configure", &configure_output)?;
        let configure_status = status_code(&configure_output);

        let (build_status, build_timed_out, build_stdout, build_stderr) =
            if configure_output.status.success() {
                let mut build_cmd = Command::new("cmake");
                build_cmd
                    .arg("--build")
                    .arg(".")
                    .arg("--verbose")
                    .arg("--")
                    .arg("-j1")
                    .arg("-k")
                    .current_dir(&build_dir)
                    .env("CXX", fragilec.to_string_lossy().to_string())
                    .env("FRAGILEC_MODE", "strict")
                    .env("FRAGILEC_PARSER_BACKEND", backend_env_value)
                    .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string());
                let context = format!(
                    "strict cmake backend-matrix build for {} in {}",
                    backend_name,
                    build_dir.display()
                );
                let (build_output, timed_out) =
                    run_command_with_timeout(&mut build_cmd, build_timeout, context.as_str())?;
                let build_status = if timed_out {
                    COMMAND_TIMEOUT_STATUS
                } else {
                    status_code(&build_output)
                };
                write_command_capture_raw(
                    &backend_log_dir,
                    "cmake_build",
                    build_status,
                    &build_output.stdout,
                    &build_output.stderr,
                )?;
                (
                    build_status,
                    timed_out,
                    String::from_utf8_lossy(&build_output.stdout).to_string(),
                    String::from_utf8_lossy(&build_output.stderr).to_string(),
                )
            } else {
                let configure_stderr = String::from_utf8_lossy(&configure_output.stderr);
                let configure_stdout = String::from_utf8_lossy(&configure_output.stdout);
                let synthetic_build_stderr = if !configure_stderr.trim().is_empty() {
                    format!(
                        "cmake configure failed for backend {} with status {}\n{}",
                        backend_name,
                        configure_status,
                        configure_stderr.trim()
                    )
                } else if !configure_stdout.trim().is_empty() {
                    format!(
                        "cmake configure failed for backend {} with status {}\n{}",
                        backend_name,
                        configure_status,
                        configure_stdout.trim()
                    )
                } else {
                    format!(
                        "cmake configure failed for backend {} with status {} and no output",
                        backend_name, configure_status
                    )
                };
                fs::write(backend_log_dir.join("cmake_build.status"), "-1\n").map_err(|e| {
                    format!(
                        "failed to write synthetic cmake_build.status in {}: {}",
                        backend_log_dir.display(),
                        e
                    )
                })?;
                fs::write(backend_log_dir.join("cmake_build.stdout"), "").map_err(|e| {
                    format!(
                        "failed to write synthetic cmake_build.stdout in {}: {}",
                        backend_log_dir.display(),
                        e
                    )
                })?;
                fs::write(
                    backend_log_dir.join("cmake_build.stderr"),
                    format!("{}\n", synthetic_build_stderr),
                )
                .map_err(|e| {
                    format!(
                        "failed to write synthetic cmake_build.stderr in {}: {}",
                        backend_log_dir.display(),
                        e
                    )
                })?;
                (-1, false, String::new(), synthetic_build_stderr)
            };

        let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
            format!(
                "failed to read strict cmake backend-matrix fragilec driver log {}: {}",
                driver_log.display(),
                e
            )
        })?;
        let (first_command, first_stderr) = select_first_failing_compile_capture(
            &driver_log_content,
            build_status != 0,
            &build_stdout,
            &build_stderr,
        );
        write_first_failing_compile_capture_files(&backend_log_dir, &first_command, &first_stderr)?;
        let first_failure_class = if build_timed_out {
            "compile_timeout".to_string()
        } else {
            classify_first_failing_compile_stderr(&first_stderr).to_string()
        };
        write_first_failing_compile_class_file(&backend_log_dir, first_failure_class.as_str())?;
        let first_failure_e0425_count = count_error_e0425_occurrences(&first_stderr);

        results.push(StrictCmakeBackendReplayResult {
            backend_name,
            configure_status,
            build_status,
            build_timed_out,
            first_failure_class,
            first_failure_e0425_count,
        });
    }

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .ok_or_else(|| {
            "missing strict cmake backend-matrix baseline result for libclang".to_string()
        })?;
    let baseline_configure_status = baseline.configure_status;
    let baseline_build_status = baseline.build_status;
    let baseline_build_timed_out = baseline.build_timed_out;
    let baseline_first_failure_class = baseline.first_failure_class.clone();
    let baseline_first_failure_e0425_count = baseline.first_failure_e0425_count;

    let mut manifest = String::new();
    manifest.push_str("fixture=real_world_strict_cmake_backend_matrix_first_failure\n");
    manifest.push_str(&format!("source_dir={}\n", worktree_dir.display()));
    manifest.push_str(&format!("pinned_commit={}\n", RAPIDJSON_PINNED_COMMIT));
    manifest.push_str(&format!("fragilec={}\n", fragilec.display()));
    manifest.push_str("mode=strict\n");
    manifest.push_str(&format!("run_root={}\n", baseline_root.display()));
    manifest.push_str(&format!("build_timeout_secs={}\n", build_timeout.as_secs()));
    manifest.push_str("backends=libclang,libtooling\n");
    manifest.push_str(&format!(
        "baseline_backend=libclang baseline_configure_status={} baseline_build_status={} baseline_build_timed_out={} baseline_first_failure_class={} baseline_first_failure_e0425_count={}\n",
        baseline_configure_status,
        baseline_build_status,
        baseline_build_timed_out,
        baseline_first_failure_class,
        baseline_first_failure_e0425_count
    ));
    for result in &results {
        let configure_status_delta_vs_baseline =
            result.configure_status - baseline_configure_status;
        let build_status_delta_vs_baseline = result.build_status - baseline_build_status;
        let class_delta_vs_baseline = result.first_failure_class != baseline_first_failure_class;
        let e0425_delta_vs_baseline =
            result.first_failure_e0425_count as i64 - baseline_first_failure_e0425_count as i64;
        let timeout_incidence_delta_vs_baseline =
            bool_to_i64(result.build_timed_out) - bool_to_i64(baseline_build_timed_out);
        manifest.push_str(&format!(
            "backend={} configure_status={} build_status={} build_timed_out={} first_failure_class={} first_failure_e0425_count={} configure_status_delta_vs_baseline={} build_status_delta_vs_baseline={} class_delta_vs_baseline={} e0425_delta_vs_baseline={} timeout_incidence_delta_vs_baseline={}\n",
            result.backend_name,
            result.configure_status,
            result.build_status,
            result.build_timed_out,
            result.first_failure_class,
            result.first_failure_e0425_count,
            configure_status_delta_vs_baseline,
            build_status_delta_vs_baseline,
            class_delta_vs_baseline,
            e0425_delta_vs_baseline,
            timeout_incidence_delta_vs_baseline
        ));
    }
    fs::write(
        log_dir.join("strict_cmake_backend_matrix_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write strict_cmake_backend_matrix_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok((log_dir, results))
}

fn run_rapidjson_strict_single_tu_backend_surface_delta_capture(
    config: StrictSingleTuBackendSurfaceCaptureConfig,
) -> Result<(PathBuf, Vec<StrictCapitalizeBackendSurfaceReplayResult>), String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = unique_prefixed_dir(config.run_root_prefix);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "{} worktree expected commit {} but got {}",
            config.context_label,
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join(config.log_dir_name);
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    let fragilec = ensure_fragilec_binary()?;
    let source = worktree_dir.join(config.source_rel_path);
    let include_dir = worktree_dir.join("include");
    let compile_timeout = strict_backend_surface_delta_compile_timeout();

    let backends: [(&str, &str); 2] = [("libclang", "libclang"), ("libtooling", "libtooling")];
    let mut results = Vec::new();
    for (backend_name, backend_env_value) in backends {
        let backend_log_dir = log_dir.join(format!("backend_{backend_name}"));
        fs::create_dir_all(&backend_log_dir).map_err(|e| {
            format!(
                "failed to create {} backend log dir {}: {}",
                config.context_label,
                backend_log_dir.display(),
                e
            )
        })?;
        let driver_log = backend_log_dir.join("fragilec_driver.log");
        fs::write(&driver_log, "").map_err(|e| {
            format!(
                "failed to initialize {} fragilec driver log {}: {}",
                config.context_label,
                driver_log.display(),
                e
            )
        })?;

        let output_obj = backend_log_dir.join(config.output_obj_name);
        let sidecar_path = output_obj.with_extension("fragile.rs");
        let transpile_stage_timing_path = backend_log_dir.join("transpile_stage_timing.log");

        let mut compile_cmd = Command::new(&fragilec);
        compile_cmd
            .arg("-std=c++11")
            .arg("-I")
            .arg(include_dir.to_string_lossy().to_string())
            .arg("-c")
            .arg(source.to_string_lossy().to_string())
            .arg("-o")
            .arg(output_obj.to_string_lossy().to_string())
            .current_dir(&worktree_dir)
            .env("FRAGILEC_MODE", "strict")
            .env("FRAGILEC_PARSER_BACKEND", backend_env_value)
            .env("FRAGILEC_KEEP_RS", "1")
            .env(
                FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV,
                transpile_stage_timing_path.to_string_lossy().to_string(),
            )
            .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string());
        let context = format!(
            "{} compile for {} in {}",
            config.context_label,
            backend_name,
            worktree_dir.display()
        );
        let (compile_output, compile_timed_out) =
            run_command_with_timeout(&mut compile_cmd, compile_timeout, context.as_str())?;
        let compile_status = if compile_timed_out {
            COMMAND_TIMEOUT_STATUS
        } else {
            status_code(&compile_output)
        };
        write_command_capture_raw(
            &backend_log_dir,
            config.compile_step_name,
            compile_status,
            &compile_output.stdout,
            &compile_output.stderr,
        )?;

        let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
            format!(
                "failed to read {} fragilec driver log {}: {}",
                config.context_label,
                driver_log.display(),
                e
            )
        })?;
        let compile_stdout = String::from_utf8_lossy(&compile_output.stdout);
        let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
        let (first_command, first_stderr) = select_first_failing_compile_capture(
            &driver_log_content,
            compile_status != 0,
            &compile_stdout,
            &compile_stderr,
        );
        write_first_failing_compile_capture_files(&backend_log_dir, &first_command, &first_stderr)?;
        let first_failure_class = if compile_timed_out {
            "compile_timeout".to_string()
        } else {
            classify_first_failing_compile_stderr(&first_stderr).to_string()
        };
        write_first_failing_compile_class_file(&backend_log_dir, first_failure_class.as_str())?;
        let first_failure_e0425_count = count_error_e0425_occurrences(&first_stderr);
        let (transpile_stage_timing_exists, transpile_stage_timing) =
            parse_transpile_stage_timing_trace(&transpile_stage_timing_path)?;

        let sidecar_exists = sidecar_path.exists();
        let generated_surface_inventory = if sidecar_exists {
            let generated_rs = fs::read_to_string(&sidecar_path).map_err(|e| {
                format!(
                    "failed to read {} sidecar {}: {}",
                    config.context_label,
                    sidecar_path.display(),
                    e
                )
            })?;
            Some(collect_generated_surface_inventory(&generated_rs))
        } else {
            None
        };

        results.push(StrictCapitalizeBackendSurfaceReplayResult {
            backend_name,
            compile_status,
            compile_timed_out,
            first_failure_class,
            first_failure_e0425_count,
            sidecar_path,
            sidecar_exists,
            generated_surface_inventory,
            transpile_stage_timing_path,
            transpile_stage_timing_exists,
            transpile_stage_timing,
        });
    }

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .ok_or_else(|| {
            format!(
                "missing {} baseline result for libclang",
                config.context_label
            )
        })?;
    let baseline_line_count = baseline
        .generated_surface_inventory
        .as_ref()
        .map(|inv| inv.line_count);
    let baseline_placeholder_count = baseline
        .generated_surface_inventory
        .as_ref()
        .map(|inv| inv.placeholder_count);
    let baseline_rapidjson_placeholder_count = baseline
        .generated_surface_inventory
        .as_ref()
        .map(|inv| inv.rapidjson_placeholder_count);
    let baseline_c_void_alias_count = baseline
        .generated_surface_inventory
        .as_ref()
        .map(|inv| inv.c_void_alias_count);
    let baseline_parse_unspecific_count = baseline
        .generated_surface_inventory
        .as_ref()
        .map(|inv| inv.parse_unspecific_syntax_error_count);
    let baseline_timing_parse_ms = baseline.transpile_stage_timing.parse_ms;
    let baseline_timing_export_ms = baseline.transpile_stage_timing.export_ms;
    let baseline_timing_enrichment_ms = baseline.transpile_stage_timing.enrichment_ms;
    let baseline_timing_codegen_ms = baseline.transpile_stage_timing.codegen_ms;
    let baseline_timing_total_ms = baseline.transpile_stage_timing.total_ms;

    let mut manifest = String::new();
    manifest.push_str(&format!("fixture={}\n", config.fixture_name));
    manifest.push_str(&format!("source_dir={}\n", worktree_dir.display()));
    manifest.push_str(&format!("pinned_commit={}\n", RAPIDJSON_PINNED_COMMIT));
    manifest.push_str(&format!("fragilec={}\n", fragilec.display()));
    manifest.push_str("mode=strict\n");
    manifest.push_str(&format!("run_root={}\n", baseline_root.display()));
    manifest.push_str(&format!(
        "compile_timeout_secs={}\n",
        compile_timeout.as_secs()
    ));
    manifest.push_str("backends=libclang,libtooling\n");
    manifest.push_str(&format!(
        "baseline_backend=libclang baseline_compile_status={} baseline_compile_timed_out={} baseline_first_failure_class={} baseline_first_failure_e0425_count={} baseline_sidecar_exists={} baseline_surface_line_count={} baseline_surface_placeholder_count={} baseline_surface_rapidjson_placeholder_count={} baseline_surface_c_void_alias_count={} baseline_surface_parse_unspecific_count={} baseline_transpile_timing_exists={} baseline_transpile_parse_ms={} baseline_transpile_export_ms={} baseline_transpile_enrichment_ms={} baseline_transpile_codegen_ms={} baseline_transpile_total_ms={} baseline_transpile_last_stage_started={} baseline_transpile_last_stage_completed={} baseline_transpile_status={}\n",
        baseline.compile_status,
        baseline.compile_timed_out,
        baseline.first_failure_class,
        baseline.first_failure_e0425_count,
        baseline.sidecar_exists,
        format_optional_usize(baseline_line_count),
        format_optional_usize(baseline_placeholder_count),
        format_optional_usize(baseline_rapidjson_placeholder_count),
        format_optional_usize(baseline_c_void_alias_count),
        format_optional_usize(baseline_parse_unspecific_count),
        baseline.transpile_stage_timing_exists,
        format_optional_u128(baseline_timing_parse_ms),
        format_optional_u128(baseline_timing_export_ms),
        format_optional_u128(baseline_timing_enrichment_ms),
        format_optional_u128(baseline_timing_codegen_ms),
        format_optional_u128(baseline_timing_total_ms),
        format_optional_str(baseline.transpile_stage_timing.last_stage_started.as_deref()),
        format_optional_str(baseline.transpile_stage_timing.last_stage_completed.as_deref()),
        format_optional_str(baseline.transpile_stage_timing.status.as_deref()),
    ));
    for result in &results {
        let line_count = result
            .generated_surface_inventory
            .as_ref()
            .map(|inv| inv.line_count);
        let placeholder_count = result
            .generated_surface_inventory
            .as_ref()
            .map(|inv| inv.placeholder_count);
        let rapidjson_placeholder_count = result
            .generated_surface_inventory
            .as_ref()
            .map(|inv| inv.rapidjson_placeholder_count);
        let c_void_alias_count = result
            .generated_surface_inventory
            .as_ref()
            .map(|inv| inv.c_void_alias_count);
        let parse_unspecific_count = result
            .generated_surface_inventory
            .as_ref()
            .map(|inv| inv.parse_unspecific_syntax_error_count);
        let transpile_parse_ms = result.transpile_stage_timing.parse_ms;
        let transpile_export_ms = result.transpile_stage_timing.export_ms;
        let transpile_enrichment_ms = result.transpile_stage_timing.enrichment_ms;
        let transpile_codegen_ms = result.transpile_stage_timing.codegen_ms;
        let transpile_total_ms = result.transpile_stage_timing.total_ms;

        let line_count_delta_vs_baseline = match (line_count, baseline_line_count) {
            (Some(value), Some(base)) => Some(value as i64 - base as i64),
            _ => None,
        };
        let placeholder_delta_vs_baseline = match (placeholder_count, baseline_placeholder_count) {
            (Some(value), Some(base)) => Some(value as i64 - base as i64),
            _ => None,
        };
        let rapidjson_placeholder_delta_vs_baseline = match (
            rapidjson_placeholder_count,
            baseline_rapidjson_placeholder_count,
        ) {
            (Some(value), Some(base)) => Some(value as i64 - base as i64),
            _ => None,
        };
        let c_void_alias_delta_vs_baseline = match (c_void_alias_count, baseline_c_void_alias_count)
        {
            (Some(value), Some(base)) => Some(value as i64 - base as i64),
            _ => None,
        };
        let parse_unspecific_delta_vs_baseline =
            match (parse_unspecific_count, baseline_parse_unspecific_count) {
                (Some(value), Some(base)) => Some(value as i64 - base as i64),
                _ => None,
            };
        let transpile_total_delta_vs_baseline = match (transpile_total_ms, baseline_timing_total_ms)
        {
            (Some(value), Some(base)) => Some(value as i64 - base as i64),
            _ => None,
        };

        manifest.push_str(&format!(
            "backend={} compile_status={} compile_timed_out={} first_failure_class={} first_failure_e0425_count={} sidecar_exists={} sidecar_path={} transpile_timing_exists={} transpile_timing_path={} transpile_parse_ms={} transpile_export_ms={} transpile_enrichment_ms={} transpile_codegen_ms={} transpile_total_ms={} transpile_total_delta_vs_baseline={} transpile_last_stage_started={} transpile_last_stage_completed={} transpile_status={} surface_line_count={} surface_placeholder_count={} surface_rapidjson_placeholder_count={} surface_c_void_alias_count={} surface_parse_unspecific_count={} surface_line_count_delta_vs_baseline={} surface_placeholder_delta_vs_baseline={} surface_rapidjson_placeholder_delta_vs_baseline={} surface_c_void_alias_delta_vs_baseline={} surface_parse_unspecific_delta_vs_baseline={}\n",
            result.backend_name,
            result.compile_status,
            result.compile_timed_out,
            result.first_failure_class,
            result.first_failure_e0425_count,
            result.sidecar_exists,
            result.sidecar_path.display(),
            result.transpile_stage_timing_exists,
            result.transpile_stage_timing_path.display(),
            format_optional_u128(transpile_parse_ms),
            format_optional_u128(transpile_export_ms),
            format_optional_u128(transpile_enrichment_ms),
            format_optional_u128(transpile_codegen_ms),
            format_optional_u128(transpile_total_ms),
            format_optional_i64(transpile_total_delta_vs_baseline),
            format_optional_str(result.transpile_stage_timing.last_stage_started.as_deref()),
            format_optional_str(result.transpile_stage_timing.last_stage_completed.as_deref()),
            format_optional_str(result.transpile_stage_timing.status.as_deref()),
            format_optional_usize(line_count),
            format_optional_usize(placeholder_count),
            format_optional_usize(rapidjson_placeholder_count),
            format_optional_usize(c_void_alias_count),
            format_optional_usize(parse_unspecific_count),
            format_optional_i64(line_count_delta_vs_baseline),
            format_optional_i64(placeholder_delta_vs_baseline),
            format_optional_i64(rapidjson_placeholder_delta_vs_baseline),
            format_optional_i64(c_void_alias_delta_vs_baseline),
            format_optional_i64(parse_unspecific_delta_vs_baseline),
        ));
    }

    fs::write(
        log_dir.join(config.manifest_file_name),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write {} in {}: {}",
            config.manifest_file_name,
            log_dir.display(),
            e
        )
    })?;

    Ok((log_dir, results))
}

fn run_rapidjson_strict_capitalize_backend_surface_delta_capture(
) -> Result<(PathBuf, Vec<StrictCapitalizeBackendSurfaceReplayResult>), String> {
    run_rapidjson_strict_single_tu_backend_surface_delta_capture(
        STRICT_CAPITALIZE_BACKEND_SURFACE_CAPTURE_CONFIG,
    )
}

fn run_rapidjson_strict_tutorial_backend_surface_delta_capture(
) -> Result<(PathBuf, Vec<StrictCapitalizeBackendSurfaceReplayResult>), String> {
    run_rapidjson_strict_single_tu_backend_surface_delta_capture(
        STRICT_TUTORIAL_BACKEND_SURFACE_CAPTURE_CONFIG,
    )
}

fn run_rapidjson_strict_capitalize_compile_capture() -> Result<PathBuf, String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_STRICT_CAPITALIZE_CAPTURE_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "strict capitalize worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("strict_capitalize_logs");
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    let fragilec = ensure_fragilec_binary()?;
    let driver_log = log_dir.join("fragilec_driver.log");
    fs::write(&driver_log, "")
        .map_err(|e| format!("failed to initialize fragilec driver log: {}", e))?;

    let source = worktree_dir.join("example/capitalize/capitalize.cpp");
    let include_dir = worktree_dir.join("include");
    let output_obj = log_dir.join("capitalize.o");
    let compile_output = Command::new(&fragilec)
        .arg("-std=c++11")
        .arg("-I")
        .arg(include_dir.to_string_lossy().to_string())
        .arg("-c")
        .arg(source.to_string_lossy().to_string())
        .arg("-o")
        .arg(output_obj.to_string_lossy().to_string())
        .current_dir(&worktree_dir)
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("failed to run strict capitalize compile replay: {}", e))?;
    write_command_capture(&log_dir, "compile_capitalize", &compile_output)?;

    let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
        format!(
            "failed to read fragilec driver log {}: {}",
            driver_log.display(),
            e
        )
    })?;
    let compile_stdout = String::from_utf8_lossy(&compile_output.stdout);
    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    let (first_command, first_stderr) = select_first_failing_compile_capture(
        &driver_log_content,
        !compile_output.status.success(),
        &compile_stdout,
        &compile_stderr,
    );
    write_first_failing_compile_capture_files(&log_dir, &first_command, &first_stderr)?;
    let first_failure_class = classify_first_failing_compile_stderr(&first_stderr);
    write_first_failing_compile_class_file(&log_dir, first_failure_class)?;

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nfragilec={}\nmode=strict\ncompile_status={}\nfirst_failing_compile_command_file=first_failing_compile_command.txt\nfirst_failing_compile_stderr_file=first_failing_compile_stderr.txt\nfirst_failing_compile_class_file=first_failing_compile_class.txt\nfirst_failing_compile_class={}\n",
        worktree_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        fragilec.display(),
        status_code(&compile_output),
        first_failure_class
    );
    fs::write(log_dir.join("strict_capitalize_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write strict_capitalize_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(log_dir)
}

fn run_rapidjson_strict_filterkeydom_compile_capture() -> Result<PathBuf, String> {
    let checkout_dir = ensure_rapidjson_checkout()?;
    let baseline_root = PathBuf::from(RAPIDJSON_STRICT_FILTERKEYDOM_CAPTURE_DIR);
    reset_dir(&baseline_root)?;

    let worktree_dir = baseline_root.join("worktree");
    let checkout_dir_str = checkout_dir.to_string_lossy().to_string();
    let worktree_dir_str = worktree_dir.to_string_lossy().to_string();
    run_git(
        &[
            "clone",
            "--no-tags",
            "--local",
            checkout_dir_str.as_str(),
            worktree_dir_str.as_str(),
        ],
        None,
    )?;
    run_git(
        &["checkout", "--detach", RAPIDJSON_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != RAPIDJSON_PINNED_COMMIT {
        return Err(format!(
            "strict filterkeydom worktree expected commit {} but got {}",
            RAPIDJSON_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("strict_filterkeydom_logs");
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    let fragilec = ensure_fragilec_binary()?;
    let driver_log = log_dir.join("fragilec_driver.log");
    fs::write(&driver_log, "")
        .map_err(|e| format!("failed to initialize fragilec driver log: {}", e))?;

    let source = worktree_dir.join("example/filterkeydom/filterkeydom.cpp");
    let include_dir = worktree_dir.join("include");
    let output_obj = log_dir.join("filterkeydom.o");
    let compile_output = Command::new(&fragilec)
        .arg("-std=c++11")
        .arg("-I")
        .arg(include_dir.to_string_lossy().to_string())
        .arg("-c")
        .arg(source.to_string_lossy().to_string())
        .arg("-o")
        .arg(output_obj.to_string_lossy().to_string())
        .current_dir(&worktree_dir)
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("failed to run strict filterkeydom compile replay: {}", e))?;
    write_command_capture(&log_dir, "compile_filterkeydom", &compile_output)?;

    let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
        format!(
            "failed to read fragilec driver log {}: {}",
            driver_log.display(),
            e
        )
    })?;
    let compile_stdout = String::from_utf8_lossy(&compile_output.stdout);
    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    let (first_command, first_stderr) = select_first_failing_compile_capture(
        &driver_log_content,
        !compile_output.status.success(),
        &compile_stdout,
        &compile_stderr,
    );
    write_first_failing_compile_capture_files(&log_dir, &first_command, &first_stderr)?;
    let first_failure_class = classify_first_failing_compile_stderr(&first_stderr);
    write_first_failing_compile_class_file(&log_dir, first_failure_class)?;

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nfragilec={}\nmode=strict\ncompile_status={}\nfirst_failing_compile_command_file=first_failing_compile_command.txt\nfirst_failing_compile_stderr_file=first_failing_compile_stderr.txt\nfirst_failing_compile_class_file=first_failing_compile_class.txt\nfirst_failing_compile_class={}\n",
        worktree_dir.display(),
        RAPIDJSON_PINNED_COMMIT,
        fragilec.display(),
        status_code(&compile_output),
        first_failure_class
    );
    fs::write(log_dir.join("strict_filterkeydom_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write strict_filterkeydom_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(log_dir)
}

fn create_local_cmake_first_failure_fixture(base_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let project_dir = base_dir.join("local_first_failure_project");
    fs::create_dir_all(project_dir.join("src"))
        .map_err(|e| format!("failed to create local fixture source dir: {}", e))?;

    fs::write(
        project_dir.join("src/ok.cpp"),
        "int ok_function() { return 0; }\n",
    )
    .map_err(|e| format!("failed to write ok.cpp for local fixture: {}", e))?;
    fs::write(
        project_dir.join("src/fail.cpp"),
        "int fail_function() { return 1; }\n",
    )
    .map_err(|e| format!("failed to write fail.cpp for local fixture: {}", e))?;
    fs::write(
        project_dir.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.16)\nproject(LocalFirstFailureFixture CXX)\nadd_library(local_first_failure STATIC src/ok.cpp src/fail.cpp)\n",
    )
    .map_err(|e| format!("failed to write CMakeLists.txt for local fixture: {}", e))?;

    let fake_fragilec = base_dir.join("fake_fragilec.sh");
    fs::write(
        &fake_fragilec,
        r#"#!/usr/bin/env bash
set -euo pipefail
log="${FRAGILEC_LOG:-}"
if [[ -n "$log" ]]; then
  printf 'parser_backend=%s\n' "${FRAGILEC_PARSER_BACKEND:-<unset>}" >> "$log"
  printf 'cwd=%s\n' "$(pwd)" >> "$log"
  printf 'args=%s\n' "$*" >> "$log"
fi
sleep_before_fail="${FRAGILEC_LOCAL_FIXTURE_SLEEP_BEFORE_FAIL_SECS:-0}"
for arg in "$@"; do
  if [[ "$arg" == *"fail.cpp"* ]]; then
    if [[ "$sleep_before_fail" =~ ^[0-9]+$ ]] && (( sleep_before_fail > 0 )); then
      sleep "$sleep_before_fail"
    fi
    echo "forced local fixture compile failure for fail.cpp" >&2
    exit 42
  fi
done
exec c++ "$@"
"#,
    )
    .map_err(|e| format!("failed to write fake fragilec wrapper script: {}", e))?;
    let chmod_output = Command::new("chmod")
        .arg("+x")
        .arg(&fake_fragilec)
        .output()
        .map_err(|e| format!("failed to run chmod on fake fragilec wrapper script: {}", e))?;
    if !chmod_output.status.success() {
        return Err(format!(
            "chmod failed for fake fragilec wrapper script {}\nstdout:\n{}\nstderr:\n{}",
            fake_fragilec.display(),
            String::from_utf8_lossy(&chmod_output.stdout),
            String::from_utf8_lossy(&chmod_output.stderr)
        ));
    }

    Ok((project_dir, fake_fragilec))
}

fn run_local_strict_cmake_no_tests_first_failure_capture_fixture(
    root: &Path,
) -> Result<PathBuf, String> {
    let (project_dir, fake_fragilec) = create_local_cmake_first_failure_fixture(root)?;
    let log_dir = root.join("strict_cmake_local_fixture_logs");
    fs::create_dir_all(&log_dir).map_err(|e| {
        format!(
            "failed to create local fixture log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;
    let driver_log = log_dir.join("fragilec_driver.log");
    fs::write(&driver_log, "").map_err(|e| {
        format!(
            "failed to initialize local fixture fragilec driver log: {}",
            e
        )
    })?;

    let build_dir = project_dir.join("build");
    fs::create_dir_all(&build_dir).map_err(|e| {
        format!(
            "failed to create local fixture build dir {}: {}",
            build_dir.display(),
            e
        )
    })?;

    let configure_output = Command::new("cmake")
        .arg("-DRAPIDJSON_BUILD_TESTS=OFF")
        .arg("..")
        .current_dir(&build_dir)
        .env("CXX", fake_fragilec.to_string_lossy().to_string())
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("failed to run local fixture strict cmake configure: {}", e))?;
    write_command_capture(&log_dir, "cmake_configure", &configure_output)?;
    if !configure_output.status.success() {
        return Err(format!(
            "local fixture strict cmake configure failed with status {} (logs: {})",
            status_code(&configure_output),
            log_dir.display()
        ));
    }

    let build_output = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .arg("--verbose")
        .arg("--")
        .arg("-j1")
        .current_dir(&build_dir)
        .env("CXX", fake_fragilec.to_string_lossy().to_string())
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
        .output()
        .map_err(|e| format!("failed to run local fixture strict cmake build: {}", e))?;
    write_command_capture(&log_dir, "cmake_build", &build_output)?;

    let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
        format!(
            "failed to read local fixture fragilec driver log {}: {}",
            driver_log.display(),
            e
        )
    })?;
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    let build_stderr = String::from_utf8_lossy(&build_output.stderr);
    let (first_command, first_stderr) = select_first_failing_compile_capture(
        &driver_log_content,
        !build_output.status.success(),
        &build_stdout,
        &build_stderr,
    );
    write_first_failing_compile_capture_files(&log_dir, &first_command, &first_stderr)?;
    let first_failure_class = classify_first_failing_compile_stderr(&first_stderr);
    write_first_failing_compile_class_file(&log_dir, first_failure_class)?;

    let manifest = format!(
        "fixture=local_strict_cmake_first_failure\nsource_dir={}\nfake_fragilec={}\nconfigure_status={}\nbuild_status={}\nfirst_failing_compile_command_file=first_failing_compile_command.txt\nfirst_failing_compile_stderr_file=first_failing_compile_stderr.txt\nfirst_failing_compile_class_file=first_failing_compile_class.txt\nfirst_failing_compile_class={}\n",
        project_dir.display(),
        fake_fragilec.display(),
        status_code(&configure_output),
        status_code(&build_output),
        first_failure_class
    );
    fs::write(
        log_dir.join("strict_cmake_local_fixture_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write strict_cmake_local_fixture_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(log_dir)
}

fn run_local_strict_cmake_no_tests_backend_matrix_capture_fixture_with_options(
    root: &Path,
    build_timeout: Option<Duration>,
    sleep_before_fail_backend: Option<&str>,
) -> Result<(PathBuf, Vec<StrictBackendReplayResult>), String> {
    let (project_dir, fake_fragilec) = create_local_cmake_first_failure_fixture(root)?;
    let log_dir = root.join("strict_cmake_backend_matrix_local_fixture_logs");
    fs::create_dir_all(&log_dir).map_err(|e| {
        format!(
            "failed to create strict backend-matrix local fixture log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;

    let backends: [(&str, &str); 3] = [
        ("libclang", "libclang"),
        ("hybrid", "hybrid"),
        ("libtooling", "libtooling"),
    ];
    let mut results = Vec::new();
    for (backend_name, backend_env_value) in backends {
        let backend_log_dir = log_dir.join(format!("backend_{backend_name}"));
        fs::create_dir_all(&backend_log_dir).map_err(|e| {
            format!(
                "failed to create strict backend-matrix local fixture backend log dir {}: {}",
                backend_log_dir.display(),
                e
            )
        })?;
        let driver_log = backend_log_dir.join("fragilec_driver.log");
        fs::write(&driver_log, "").map_err(|e| {
            format!(
                "failed to initialize strict backend-matrix local fixture fragilec driver log {}: {}",
                driver_log.display(),
                e
            )
        })?;

        let build_dir = project_dir.join(format!("build_{backend_name}"));
        fs::create_dir_all(&build_dir).map_err(|e| {
            format!(
                "failed to create strict backend-matrix local fixture build dir {}: {}",
                build_dir.display(),
                e
            )
        })?;

        let configure_output = Command::new("cmake")
            .arg("-DRAPIDJSON_BUILD_TESTS=OFF")
            .arg("..")
            .current_dir(&build_dir)
            .env("CXX", fake_fragilec.to_string_lossy().to_string())
            .env("FRAGILEC_MODE", "strict")
            .env("FRAGILEC_PARSER_BACKEND", backend_env_value)
            .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
            .output()
            .map_err(|e| {
                format!(
                    "failed to run strict backend-matrix local fixture cmake configure for {}: {}",
                    backend_name, e
                )
            })?;
        write_command_capture(&backend_log_dir, "cmake_configure", &configure_output)?;
        if !configure_output.status.success() {
            return Err(format!(
                "strict backend-matrix local fixture cmake configure failed for {} with status {} (logs: {})",
                backend_name,
                status_code(&configure_output),
                backend_log_dir.display()
            ));
        }

        let mut build_cmd = Command::new("cmake");
        build_cmd
            .arg("--build")
            .arg(".")
            .arg("--verbose")
            .arg("--")
            .arg("-j1")
            .current_dir(&build_dir)
            .env("CXX", fake_fragilec.to_string_lossy().to_string())
            .env("FRAGILEC_MODE", "strict")
            .env("FRAGILEC_PARSER_BACKEND", backend_env_value)
            .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string());
        if sleep_before_fail_backend == Some(backend_name) {
            build_cmd.env("FRAGILEC_LOCAL_FIXTURE_SLEEP_BEFORE_FAIL_SECS", "3");
        }
        let (build_output, build_timed_out) = if let Some(timeout) = build_timeout {
            let context = format!(
                "strict backend-matrix local fixture cmake build for {} in {}",
                backend_name,
                build_dir.display()
            );
            run_command_with_timeout(&mut build_cmd, timeout, context.as_str())?
        } else {
            let output = build_cmd.output().map_err(|e| {
                format!(
                    "failed to run strict backend-matrix local fixture cmake build for {}: {}",
                    backend_name, e
                )
            })?;
            (output, false)
        };
        let build_status = if build_timed_out {
            COMMAND_TIMEOUT_STATUS
        } else {
            status_code(&build_output)
        };
        write_command_capture_raw(
            &backend_log_dir,
            "cmake_build",
            build_status,
            &build_output.stdout,
            &build_output.stderr,
        )?;

        let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
            format!(
                "failed to read strict backend-matrix local fixture fragilec driver log {}: {}",
                driver_log.display(),
                e
            )
        })?;
        let build_stdout = String::from_utf8_lossy(&build_output.stdout);
        let build_stderr = String::from_utf8_lossy(&build_output.stderr);
        let (first_command, first_stderr) = select_first_failing_compile_capture(
            &driver_log_content,
            build_status != 0,
            &build_stdout,
            &build_stderr,
        );
        write_first_failing_compile_capture_files(&backend_log_dir, &first_command, &first_stderr)?;
        let first_failure_class = if build_timed_out {
            "compile_timeout".to_string()
        } else {
            classify_first_failing_compile_stderr(&first_stderr).to_string()
        };
        write_first_failing_compile_class_file(&backend_log_dir, first_failure_class.as_str())?;
        let first_failure_e0425_count = count_error_e0425_occurrences(&first_stderr);

        results.push(StrictBackendReplayResult {
            backend_name,
            compile_status: build_status,
            first_failure_class,
            first_failure_e0425_count,
        });
    }

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .ok_or_else(|| {
            "missing strict backend-matrix local fixture baseline replay result for libclang"
                .to_string()
        })?;
    let baseline_e0425_count = baseline.first_failure_e0425_count;
    let baseline_status = baseline.compile_status;
    let baseline_class = baseline.first_failure_class.clone();

    let mut manifest = String::new();
    manifest.push_str("fixture=local_strict_cmake_backend_matrix_first_failure\n");
    manifest.push_str(&format!("source_dir={}\n", project_dir.display()));
    manifest.push_str(&format!("fake_fragilec={}\n", fake_fragilec.display()));
    manifest.push_str("mode=strict\n");
    manifest.push_str("backends=libclang,hybrid,libtooling\n");
    manifest.push_str(&format!(
        "baseline_backend=libclang baseline_build_status={} baseline_first_failure_class={} baseline_first_failure_e0425_count={}\n",
        baseline_status, baseline_class, baseline_e0425_count
    ));
    for result in &results {
        let build_status_delta_vs_baseline = result.compile_status - baseline_status;
        let e0425_delta_vs_baseline =
            result.first_failure_e0425_count as i64 - baseline_e0425_count as i64;
        let class_delta_vs_baseline = result.first_failure_class != baseline_class;
        manifest.push_str(&format!(
            "backend={} build_status={} first_failure_class={} first_failure_e0425_count={} build_status_delta_vs_baseline={} class_delta_vs_baseline={} e0425_delta_vs_baseline={}\n",
            result.backend_name,
            result.compile_status,
            result.first_failure_class,
            result.first_failure_e0425_count,
            build_status_delta_vs_baseline,
            class_delta_vs_baseline,
            e0425_delta_vs_baseline
        ));
    }
    fs::write(
        log_dir.join("strict_cmake_backend_matrix_local_fixture_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write strict_cmake_backend_matrix_local_fixture_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok((log_dir, results))
}

fn run_local_strict_cmake_no_tests_backend_matrix_capture_fixture(
    root: &Path,
) -> Result<(PathBuf, Vec<StrictBackendReplayResult>), String> {
    run_local_strict_cmake_no_tests_backend_matrix_capture_fixture_with_options(root, None, None)
}

fn run_local_strict_backend_toggle_e0425_delta_replay_fixture(
    root: &Path,
) -> Result<(PathBuf, Vec<StrictBackendReplayResult>), String> {
    let fixture_dir = root.join("strict_backend_toggle_fixture");
    fs::create_dir_all(&fixture_dir).map_err(|e| {
        format!(
            "failed to create strict backend-toggle fixture dir {}: {}",
            fixture_dir.display(),
            e
        )
    })?;
    let source_path = fixture_dir.join("backend_toggle_fixture.cpp");
    fs::write(
        &source_path,
        r#"
template<typename T>
struct Box {
    T value;
};

template<typename T>
T add_one(T value) {
    return value + 1;
}

int use_box() {
    Box<int> box{41};
    return add_one<int>(box.value);
}
"#,
    )
    .map_err(|e| {
        format!(
            "failed to write strict backend-toggle fixture source {}: {}",
            source_path.display(),
            e
        )
    })?;

    let log_dir = root.join("strict_backend_toggle_logs");
    fs::create_dir_all(&log_dir).map_err(|e| {
        format!(
            "failed to create strict backend-toggle log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;
    let fragilec = ensure_fragilec_binary()?;

    let backends: [(&str, &str); 3] = [
        ("libclang", "libclang"),
        ("hybrid", "hybrid"),
        ("libtooling", "libtooling"),
    ];
    let mut results = Vec::new();
    for (backend_name, backend_env_value) in backends {
        let backend_log_dir = log_dir.join(format!("backend_{backend_name}"));
        fs::create_dir_all(&backend_log_dir).map_err(|e| {
            format!(
                "failed to create strict backend-toggle backend log dir {}: {}",
                backend_log_dir.display(),
                e
            )
        })?;
        let driver_log = backend_log_dir.join("fragilec_driver.log");
        fs::write(&driver_log, "").map_err(|e| {
            format!(
                "failed to initialize strict backend-toggle fragilec driver log {}: {}",
                driver_log.display(),
                e
            )
        })?;
        let output_obj = backend_log_dir.join("backend_toggle_fixture.o");

        let compile_output = Command::new(&fragilec)
            .arg("-std=c++11")
            .arg("-c")
            .arg(source_path.to_string_lossy().to_string())
            .arg("-o")
            .arg(output_obj.to_string_lossy().to_string())
            .current_dir(&fixture_dir)
            .env("FRAGILEC_MODE", "strict")
            .env("FRAGILEC_PARSER_BACKEND", backend_env_value)
            .env("FRAGILEC_LOG", driver_log.to_string_lossy().to_string())
            .output()
            .map_err(|e| {
                format!(
                    "failed to run strict backend-toggle compile replay for {}: {}",
                    backend_name, e
                )
            })?;
        write_command_capture(&backend_log_dir, "compile", &compile_output)?;

        let driver_log_content = fs::read_to_string(&driver_log).map_err(|e| {
            format!(
                "failed to read strict backend-toggle fragilec driver log {}: {}",
                driver_log.display(),
                e
            )
        })?;
        let compile_stdout = String::from_utf8_lossy(&compile_output.stdout);
        let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
        let (first_command, first_stderr) = select_first_failing_compile_capture(
            &driver_log_content,
            !compile_output.status.success(),
            &compile_stdout,
            &compile_stderr,
        );
        write_first_failing_compile_capture_files(&backend_log_dir, &first_command, &first_stderr)?;
        let first_failure_class = classify_first_failing_compile_stderr(&first_stderr).to_string();
        write_first_failing_compile_class_file(&backend_log_dir, first_failure_class.as_str())?;
        let first_failure_e0425_count = count_error_e0425_occurrences(&first_stderr);

        results.push(StrictBackendReplayResult {
            backend_name,
            compile_status: status_code(&compile_output),
            first_failure_class,
            first_failure_e0425_count,
        });
    }

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .ok_or_else(|| {
            "missing strict backend-toggle baseline replay result for libclang".to_string()
        })?;
    let baseline_e0425_count = baseline.first_failure_e0425_count;
    let baseline_status = baseline.compile_status;

    let mut manifest = String::new();
    manifest.push_str("fixture=local_strict_backend_toggle_e0425_delta\n");
    manifest.push_str(&format!("source={}\n", source_path.display()));
    manifest.push_str(&format!("fragilec={}\n", fragilec.display()));
    manifest.push_str("mode=strict\n");
    manifest.push_str("backends=libclang,hybrid,libtooling\n");
    manifest.push_str(&format!(
        "baseline_backend=libclang baseline_compile_status={} baseline_first_failure_e0425_count={}\n",
        baseline_status, baseline_e0425_count
    ));
    for result in &results {
        let e0425_delta_vs_baseline =
            result.first_failure_e0425_count as i64 - baseline_e0425_count as i64;
        manifest.push_str(&format!(
            "backend={} compile_status={} first_failure_class={} first_failure_e0425_count={} e0425_delta_vs_baseline={}\n",
            result.backend_name,
            result.compile_status,
            result.first_failure_class,
            result.first_failure_e0425_count,
            e0425_delta_vs_baseline
        ));
    }
    fs::write(log_dir.join("strict_backend_toggle_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write strict_backend_toggle_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok((log_dir, results))
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("fragile_{prefix}_{}_{}", std::process::id(), now))
}

fn create_local_rapidjson_like_repo(base_dir: &Path) -> Result<(String, String, String), String> {
    let remote_dir = base_dir.join("remote");
    fs::create_dir_all(remote_dir.join("include/rapidjson"))
        .map_err(|e| format!("failed to create include dir: {}", e))?;
    fs::create_dir_all(remote_dir.join("example/condense"))
        .map_err(|e| format!("failed to create condense dir: {}", e))?;
    fs::create_dir_all(remote_dir.join("example/pretty"))
        .map_err(|e| format!("failed to create pretty dir: {}", e))?;

    fs::write(
        remote_dir.join("include/rapidjson/document.h"),
        "#pragma once\n",
    )
    .map_err(|e| format!("failed to write document.h: {}", e))?;
    fs::write(
        remote_dir.join("example/condense/condense.cpp"),
        "#include <cstdio>\nint main(){std::fputs(\"{\\\"a\\\":1,\\\"b\\\":[true,false],\\\"msg\\\":\\\"hi\\\"}\", stdout); return 0;}\n",
    )
    .map_err(|e| format!("failed to write condense.cpp: {}", e))?;
    fs::write(
        remote_dir.join("example/pretty/pretty.cpp"),
        "#include <cstdio>\nint main(){std::fputs(\"{\\n    \\\"a\\\": 1,\\n    \\\"b\\\": [\\n        true,\\n        false\\n    ],\\n    \\\"msg\\\": \\\"hi\\\"\\n}\\n\", stdout); return 0;}\n",
    )
    .map_err(|e| format!("failed to write pretty.cpp: {}", e))?;
    fs::write(
        remote_dir.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.5)\n",
    )
    .map_err(|e| format!("failed to write CMakeLists.txt: {}", e))?;

    run_git(&["init"], Some(&remote_dir))?;
    run_git(&["config", "user.name", "Fragile Test"], Some(&remote_dir))?;
    run_git(
        &["config", "user.email", "fragile-test@example.invalid"],
        Some(&remote_dir),
    )?;
    run_git(
        &[
            "add",
            "include/rapidjson/document.h",
            "example/condense/condense.cpp",
            "example/pretty/pretty.cpp",
            "CMakeLists.txt",
        ],
        Some(&remote_dir),
    )?;
    run_git(&["commit", "-m", "initial fixture"], Some(&remote_dir))?;

    let pinned_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;

    fs::write(
        remote_dir.join("example/condense/condense.cpp"),
        "#include <cstdio>\nint main(){std::fputs(\"{\\\"fixture\\\":2}\", stdout); return 0;}\n",
    )
    .map_err(|e| format!("failed to update condense.cpp: {}", e))?;
    run_git(&["add", "example/condense/condense.cpp"], Some(&remote_dir))?;
    run_git(&["commit", "-m", "update fixture"], Some(&remote_dir))?;

    let newer_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;
    let repo_url = remote_dir.to_string_lossy().to_string();
    Ok((repo_url, pinned_commit, newer_commit))
}

#[test]
fn test_ensure_pinned_checkout_clones_and_rewinds_local_rapidjson_fixture() {
    let root = unique_temp_dir("rapidjson_checkout_pin");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, newer_commit) = create_local_rapidjson_like_repo(&root)
        .expect("failed to create local rapidjson-like repo");
    let checkout_dir = root.join("checkout");

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        RAPIDJSON_REQUIRED_PATHS,
    )
    .expect("initial checkout should succeed");

    run_git(
        &["checkout", "--detach", newer_commit.as_str()],
        Some(&checkout_dir),
    )
    .expect("failed to move checkout to newer commit");
    let moved_head = read_head(&checkout_dir).expect("failed to read moved HEAD");
    assert_eq!(
        moved_head, newer_commit,
        "checkout should move before rewind"
    );

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        RAPIDJSON_REQUIRED_PATHS,
    )
    .expect("rewind checkout should succeed");

    let head = read_head(&checkout_dir).expect("failed to read pinned HEAD");
    assert_eq!(
        head, pinned_commit,
        "checkout should rewind to pinned commit"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_rapidjson_native_no_stl_examples_local_fixture_success() {
    let root = unique_temp_dir("rapidjson_native_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) = create_local_rapidjson_like_repo(&root)
        .expect("failed to create local rapidjson-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        RAPIDJSON_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("native_logs");
    run_native_no_stl_examples_in_tree(&checkout_dir, &log_dir)
        .expect("local rapidjson fixture baseline should succeed");

    for rel in RAPIDJSON_NATIVE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected baseline log file {}",
            log_dir.join(rel).display()
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_rapidjson_fragilec_driver_no_stl_examples_local_fixture_success() {
    let root = unique_temp_dir("rapidjson_fragilec_driver_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) = create_local_rapidjson_like_repo(&root)
        .expect("failed to create local rapidjson-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        RAPIDJSON_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("fragilec_driver_logs");
    run_fragilec_driver_no_stl_examples_in_tree(&checkout_dir, &log_dir)
        .expect("local rapidjson fragilec-driver baseline should succeed");

    for rel in RAPIDJSON_FRAGILEC_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragilec-driver log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("compile_condense_driver.status"))
            .expect("failed to read compile_condense_driver.status"),
        0,
        "strict fragilec-driver condense compile should succeed"
    );
    assert_eq!(
        read_status_file(&log_dir.join("compile_pretty_driver.status"))
            .expect("failed to read compile_pretty_driver.status"),
        0,
        "strict fragilec-driver pretty compile should succeed"
    );
    assert_eq!(
        read_status_file(&log_dir.join("run_condense_driver.status"))
            .expect("failed to read run_condense_driver.status"),
        0,
        "strict fragilec-driver condense run should succeed"
    );
    assert_eq!(
        read_status_file(&log_dir.join("run_pretty_driver.status"))
            .expect("failed to read run_pretty_driver.status"),
        0,
        "strict fragilec-driver pretty run should succeed"
    );
    let condense_stdout = fs::read_to_string(log_dir.join("run_condense_driver.stdout"))
        .expect("failed to read run_condense_driver.stdout");
    assert_eq!(
        condense_stdout.trim(),
        RAPIDJSON_EXPECTED_CONDENSE_OUTPUT,
        "strict fragilec-driver condense output should match expected compact JSON"
    );
    let pretty_stdout = fs::read_to_string(log_dir.join("run_pretty_driver.stdout"))
        .expect("failed to read run_pretty_driver.stdout");
    assert!(
        rapidjson_pretty_output_matches_expected(&pretty_stdout),
        "strict fragilec-driver pretty output should preserve JSON fields, got:\n{}",
        pretty_stdout
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_rapidjson_no_stl_command_plan_local_fixture_success() {
    let root = unique_temp_dir("rapidjson_plan_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) = create_local_rapidjson_like_repo(&root)
        .expect("failed to create local rapidjson-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        RAPIDJSON_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("command_plan_logs");
    run_no_stl_command_plan_in_tree(&checkout_dir, &log_dir)
        .expect("local command-plan generation should succeed");

    for rel in RAPIDJSON_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected command-plan log file {}",
            log_dir.join(rel).display()
        );
    }

    let manifest = fs::read_to_string(log_dir.join("no_stl_examples_manifest.txt"))
        .expect("failed to read no_stl_examples_manifest.txt");
    assert!(
        manifest.contains("example/condense/condense.cpp")
            && manifest.contains("example/pretty/pretty.cpp"),
        "manifest should include no-stl example sources, got:\n{}",
        manifest
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_rapidjson_fragile_condense_single_tu_replay_local_fixture_success() {
    let root = unique_temp_dir("rapidjson_fragile_single_tu_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) = create_local_rapidjson_like_repo(&root)
        .expect("failed to create local rapidjson-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        RAPIDJSON_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("replay_logs");
    run_fragile_condense_single_tu_replay_in_tree(&checkout_dir, &log_dir)
        .expect("local fragile condense single-tu replay should succeed");

    for rel in RAPIDJSON_FRAGILE_CONDENSE_REPLAY_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected replay log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("rustc_fragile_condense.status"))
            .expect("failed to read rustc_fragile_condense.status"),
        0,
        "local single-tu replay rustc status should be zero"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_fragilec_driver_invocations_extracts_cwd_and_args_pairs() {
    let driver_log = "cwd=/tmp/work\nargs=-std=c++11 -c a.cpp -o a.o \ncwd=/tmp/work\nargs=-std=c++11 -c b.cpp -o b.o \n";
    let invocations = parse_fragilec_driver_invocations(driver_log);
    assert_eq!(
        invocations,
        vec![
            FragilecDriverInvocation {
                cwd: "/tmp/work".to_string(),
                args: "-std=c++11 -c a.cpp -o a.o".to_string(),
            },
            FragilecDriverInvocation {
                cwd: "/tmp/work".to_string(),
                args: "-std=c++11 -c b.cpp -o b.o".to_string(),
            },
        ],
        "driver-log parser should capture each cwd/args invocation pair"
    );
}

#[test]
fn test_select_first_failing_compile_capture_prefers_source_matched_invocation() {
    let driver_log = "cwd=/tmp/work\nargs=-std=c++11 -c first.cpp -o first.o \ncwd=/tmp/work\nargs=-std=c++11 -c failing.cpp -o failing.o \ncwd=/tmp/work\nargs=-std=c++11 -c trailing.cpp -o trailing.o \n";
    let build_stderr = "[fragilec] fragile rustc object compile failed for /tmp/work/failing.cpp\nerror[E0507]: cannot move out of\ncommand timed out after 1200s: strict cmake backend-matrix build for libtooling\n";
    let (command, stderr) = select_first_failing_compile_capture(
        driver_log,
        true,
        "stdout text",
        build_stderr,
    );
    assert!(
        command.contains("failing.cpp"),
        "capture should report source-matched failing compile invocation (not trailing invocation), got:\n{}",
        command
    );
    assert!(
        stderr.starts_with("[fragilec] fragile rustc object compile failed for /tmp/work/failing.cpp"),
        "capture should scope stderr payload to the source-matched failing unit, got:\n{}",
        stderr
    );
}

#[test]
fn test_select_first_failing_compile_capture_matches_error_while_processing_marker() {
    let driver_log = "cwd=/tmp/work\nargs=-std=c++11 -c tutorial.cpp -o tutorial.o \ncwd=/tmp/work\nargs=-std=c++11 -c trailing.cpp -o trailing.o \n";
    let build_stderr = "4 warnings and 1 error generated.\nError while processing /tmp/work/tutorial.cpp.\n[fragilec] failed to transpile /tmp/work/tutorial.cpp with parser backend Libtooling: AST export failed with code 1\n";
    let (command, stderr) = select_first_failing_compile_capture(driver_log, true, "", build_stderr);
    assert!(
        command.contains("tutorial.cpp"),
        "capture should map `Error while processing` marker to tutorial compile invocation, got:\n{}",
        command
    );
    assert!(
        stderr.starts_with("Error while processing /tmp/work/tutorial.cpp."),
        "capture should scope stderr from the tutorial marker, got:\n{}",
        stderr
    );
}

#[test]
fn test_select_first_failing_compile_capture_falls_back_to_last_invocation_without_source_marker() {
    let driver_log = "cwd=/tmp/work\nargs=-std=c++11 -c first.cpp -o first.o \ncwd=/tmp/work\nargs=-std=c++11 -c last.cpp -o last.o \n";
    let (command, stderr) =
        select_first_failing_compile_capture(driver_log, true, "stdout text", "plain stderr text");
    assert!(
        command.contains("last.cpp"),
        "capture should fall back to last invocation when no source marker is present, got:\n{}",
        command
    );
    assert_eq!(stderr, "plain stderr text");
}

#[test]
fn test_select_first_failing_compile_capture_scopes_to_first_source_block_only() {
    let driver_log = "cwd=/tmp/work\nargs=-std=c++11 -c first.cpp -o first.o \ncwd=/tmp/work\nargs=-std=c++11 -c second.cpp -o second.o \n";
    let build_stderr = "[fragilec] fragile rustc object compile failed for /tmp/work/first.cpp\nerror[E0121]: placeholder `_` not allowed in item signatures\n[fragilec] fragile rustc object compile failed for /tmp/work/second.cpp\nerror[E0425]: cannot find type `T` in this scope\n";
    let (command, stderr) =
        select_first_failing_compile_capture(driver_log, true, "stdout text", build_stderr);
    assert!(
        command.contains("first.cpp"),
        "capture should select first source-matched invocation, got:\n{}",
        command
    );
    assert!(
        stderr.contains("first.cpp") && stderr.contains("error[E0121]"),
        "capture should include first source failure block, got:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("second.cpp"),
        "capture should exclude later source failure blocks, got:\n{}",
        stderr
    );
    assert!(
        !stderr.contains("error[E0425]"),
        "capture should not include later E0425 diagnostics from other TUs, got:\n{}",
        stderr
    );
    assert_eq!(
        classify_first_failing_compile_stderr(&stderr),
        "other_rustc_error",
        "first source block should classify using first TU diagnostics only"
    );
    assert_eq!(
        count_error_e0425_occurrences(&stderr),
        0,
        "first source block should not include E0425 count from later TUs"
    );
}

#[test]
fn test_select_first_failing_compile_capture_returns_none_when_build_succeeds() {
    let (command, stderr) =
        select_first_failing_compile_capture("cwd=/tmp\nargs=-c ok.cpp -o ok.o\n", false, "", "");
    assert_eq!(command, "<none>");
    assert_eq!(stderr, "<none>");
}

#[test]
fn test_run_command_with_timeout_drains_large_stderr_without_false_timeout() {
    let mut command = Command::new("bash");
    command.arg("-lc").arg(
        "i=0; while [ \"$i\" -lt 8000 ]; do echo \"drain-stderr-line-$i\" 1>&2; i=$((i+1)); done; exit 1",
    );
    let (output, timed_out) = run_command_with_timeout(
        &mut command,
        Duration::from_secs(10),
        "large-stderr timeout-drain fixture",
    )
    .expect("large-stderr timeout-drain fixture should execute");
    assert!(
        !timed_out,
        "large-stderr command should not be misclassified as timeout when pipes are drained"
    );
    assert_eq!(
        status_code(&output),
        1,
        "large-stderr command should preserve exit status when not timed out"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("drain-stderr-line-7999"),
        "large-stderr fixture should capture full stderr payload without pipe deadlock"
    );
    assert!(
        !stderr.contains("command timed out after"),
        "large-stderr fixture stderr should not include timeout sentinel when command exits normally"
    );
}

#[test]
fn test_classify_first_failing_compile_stderr_covers_known_error_families() {
    assert_eq!(
        classify_first_failing_compile_stderr(
            "error[E0428]: the name foo is defined multiple times"
        ),
        "duplicate_definition_e0428"
    );
    assert_eq!(
        classify_first_failing_compile_stderr("error[E0425]: cannot find type `T` in this scope"),
        "unresolved_name_or_type_e0425"
    );
    assert_eq!(
        classify_first_failing_compile_stderr("error[E0507]: cannot move out of"),
        "other_rustc_error"
    );
    assert_eq!(
        classify_first_failing_compile_stderr("forced local fixture compile failure"),
        "non_rustc_error"
    );
    assert_eq!(classify_first_failing_compile_stderr("<none>"), "none");
}

#[test]
fn test_collect_generated_surface_inventory_counts_expected_markers() {
    let generated = r#"
/// Placeholder for C++ `rapidjson::Reader`
pub type FragileFileAlias = std::ffi::c_void;
kParseErrorUnspecificSyntaxError
/// Placeholder for C++ `OtherType`
"#;
    let inventory = collect_generated_surface_inventory(generated);
    assert_eq!(inventory.line_count, generated.lines().count());
    assert_eq!(inventory.placeholder_count, 2);
    assert_eq!(inventory.rapidjson_placeholder_count, 1);
    assert_eq!(inventory.c_void_alias_count, 1);
    assert_eq!(inventory.parse_unspecific_syntax_error_count, 1);
}

#[test]
fn test_rapidjson_strict_cmake_local_fixture_replays_first_failure_capture() {
    let root = unique_temp_dir("rapidjson_strict_cmake_local_fixture_first_failure");
    fs::create_dir_all(&root).expect("failed to create local fixture root");

    let log_dir = run_local_strict_cmake_no_tests_first_failure_capture_fixture(&root)
        .expect("failed to run local strict cmake first-failure fixture");
    for rel in RAPIDJSON_STRICT_CMAKE_LOCAL_FIXTURE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict local fixture capture log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("cmake_configure.status"))
            .expect("failed to read local fixture cmake_configure.status"),
        0,
        "local fixture strict cmake configure should succeed"
    );
    let build_status = read_status_file(&log_dir.join("cmake_build.status"))
        .expect("failed to read local fixture cmake_build.status");
    assert_ne!(
        build_status, 0,
        "local fixture strict cmake build should fail to replay first-failure capture"
    );
    let first_command = fs::read_to_string(log_dir.join("first_failing_compile_command.txt"))
        .expect("failed to read local fixture first_failing_compile_command.txt");
    assert!(
        first_command.contains("fail.cpp"),
        "local fixture should capture fail.cpp compile command, got:\n{}",
        first_command
    );
    let first_stderr = fs::read_to_string(log_dir.join("first_failing_compile_stderr.txt"))
        .expect("failed to read local fixture first_failing_compile_stderr.txt");
    assert!(
        first_stderr.contains("forced local fixture compile failure for fail.cpp"),
        "local fixture should capture forced failing stderr, got:\n{}",
        first_stderr
    );
    let first_class = fs::read_to_string(log_dir.join("first_failing_compile_class.txt"))
        .expect("failed to read local fixture first_failing_compile_class.txt");
    assert_eq!(
        first_class.trim(),
        "non_rustc_error",
        "local fixture forced failure should classify as non-rustc error, got:\n{}",
        first_class
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_rapidjson_strict_cmake_backend_matrix_local_fixture_keeps_baseline_deltas() {
    let root = unique_temp_dir("rapidjson_strict_cmake_backend_matrix_local_fixture");
    fs::create_dir_all(&root).expect("failed to create strict backend-matrix local fixture root");

    let (log_dir, results) = run_local_strict_cmake_no_tests_backend_matrix_capture_fixture(&root)
        .expect("failed to run strict backend-matrix local fixture first-failure capture");

    for rel in RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_LOCAL_FIXTURE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict backend-matrix local fixture log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        results.len(),
        3,
        "strict backend-matrix local fixture should produce three backend replay results"
    );

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing strict backend-matrix baseline result for libclang");
    assert_ne!(
        baseline.compile_status, 0,
        "strict backend-matrix local fixture baseline should fail build to exercise first-failure capture"
    );
    assert_eq!(
        baseline.first_failure_class, "non_rustc_error",
        "strict backend-matrix local fixture baseline should classify forced wrapper failure as non-rustc"
    );

    let backend_env_pairs: [(&str, &str); 3] = [
        ("libclang", "libclang"),
        ("hybrid", "hybrid"),
        ("libtooling", "libtooling"),
    ];
    for (backend_name, backend_env_value) in backend_env_pairs {
        let backend = results
            .iter()
            .find(|entry| entry.backend_name == backend_name)
            .unwrap_or_else(|| {
                panic!(
                    "missing strict backend-matrix local fixture result for {}",
                    backend_name
                )
            });
        assert_eq!(
            backend.compile_status, baseline.compile_status,
            "strict backend-matrix local fixture build status should match baseline for backend {} (logs: {})",
            backend_name,
            log_dir.display()
        );
        assert_eq!(
            backend.first_failure_class, baseline.first_failure_class,
            "strict backend-matrix local fixture first-failure class should match baseline for backend {} (logs: {})",
            backend_name,
            log_dir.display()
        );
        assert_eq!(
            backend.first_failure_e0425_count, baseline.first_failure_e0425_count,
            "strict backend-matrix local fixture E0425 count should match baseline for backend {} (logs: {})",
            backend_name,
            log_dir.display()
        );

        let driver_log =
            fs::read_to_string(log_dir.join(format!("backend_{backend_name}/fragilec_driver.log")))
                .unwrap_or_default();
        assert!(
            driver_log.contains(format!("parser_backend={backend_env_value}").as_str()),
            "strict backend-matrix local fixture driver log should capture parser backend {} for backend {}. got:\n{}",
            backend_env_value,
            backend_name,
            driver_log
        );
    }

    let manifest =
        fs::read_to_string(log_dir.join("strict_cmake_backend_matrix_local_fixture_manifest.txt"))
            .expect("failed to read strict_cmake_backend_matrix_local_fixture_manifest.txt");
    assert!(
        manifest.contains("baseline_backend=libclang"),
        "strict backend-matrix local fixture manifest should record baseline backend, got:\n{}",
        manifest
    );
    for backend_name in ["libclang", "hybrid", "libtooling"] {
        assert!(
            manifest.contains(format!("backend={backend_name}").as_str()),
            "strict backend-matrix local fixture manifest should include backend entry {}, got:\n{}",
            backend_name,
            manifest
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_rapidjson_strict_cmake_backend_matrix_local_fixture_classifies_backend_timeout() {
    let root = unique_temp_dir("rapidjson_strict_cmake_backend_matrix_local_fixture_timeout");
    fs::create_dir_all(&root).expect("failed to create strict backend-matrix timeout fixture root");

    let (log_dir, results) =
        run_local_strict_cmake_no_tests_backend_matrix_capture_fixture_with_options(
            &root,
            Some(Duration::from_secs(1)),
            Some("libtooling"),
        )
        .expect("failed to run strict backend-matrix local fixture timeout replay");
    assert_eq!(
        results.len(),
        3,
        "strict backend-matrix timeout fixture should produce three backend replay results"
    );

    for backend_name in ["libclang", "hybrid"] {
        let backend = results
            .iter()
            .find(|entry| entry.backend_name == backend_name)
            .unwrap_or_else(|| {
                panic!(
                    "missing strict backend-matrix timeout result for {}",
                    backend_name
                )
            });
        assert_ne!(
            backend.compile_status,
            COMMAND_TIMEOUT_STATUS,
            "backend {} should not time out in timeout fixture (logs: {})",
            backend_name,
            log_dir.display()
        );
        assert_eq!(
            backend.first_failure_class,
            "non_rustc_error",
            "backend {} should keep forced-wrapper failure class when not timed out (logs: {})",
            backend_name,
            log_dir.display()
        );
    }

    let timed_out_backend = results
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing strict backend-matrix timeout result for libtooling");
    assert_eq!(
        timed_out_backend.compile_status,
        COMMAND_TIMEOUT_STATUS,
        "libtooling backend should use timeout sentinel status when timeout is forced (logs: {})",
        log_dir.display()
    );
    assert_eq!(
        timed_out_backend.first_failure_class,
        "compile_timeout",
        "libtooling backend should classify forced timeout as compile_timeout (logs: {})",
        log_dir.display()
    );

    assert_eq!(
        read_status_file(&log_dir.join("backend_libtooling/cmake_build.status"))
            .expect("failed to read backend_libtooling/cmake_build.status"),
        COMMAND_TIMEOUT_STATUS,
        "backend_libtooling cmake_build.status should persist timeout sentinel status"
    );
    let timeout_stderr = fs::read_to_string(log_dir.join("backend_libtooling/cmake_build.stderr"))
        .expect("failed to read backend_libtooling/cmake_build.stderr");
    assert!(
        timeout_stderr.contains("command timed out after 1s"),
        "backend_libtooling cmake_build.stderr should record timeout diagnostic, got:\n{}",
        timeout_stderr
    );
    let timeout_first_class =
        fs::read_to_string(log_dir.join("backend_libtooling/first_failing_compile_class.txt"))
            .expect("failed to read backend_libtooling/first_failing_compile_class.txt");
    assert_eq!(
        timeout_first_class.trim(),
        "compile_timeout",
        "backend_libtooling first failure class file should persist compile_timeout, got:\n{}",
        timeout_first_class
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_backend_matrix_delta_snapshot_from_manifest_line() {
    let line = "backend=libtooling configure_status=0 build_status=1 build_timed_out=false first_failure_class=other_rustc_error first_failure_e0425_count=0 configure_status_delta_vs_baseline=0 build_status_delta_vs_baseline=1 class_delta_vs_baseline=true e0425_delta_vs_baseline=0 timeout_incidence_delta_vs_baseline=0";
    let snapshot = parse_backend_matrix_delta_snapshot_from_manifest_line(line, None)
        .expect("expected backend matrix delta snapshot parse to succeed");
    assert_eq!(
        snapshot,
        BackendMatrixDeltaSnapshot {
            configure_status_delta_vs_baseline: 0,
            build_status_delta_vs_baseline: 1,
            class_delta_vs_baseline: true,
            e0425_delta_vs_baseline: 0,
            timeout_incidence_delta_vs_baseline: 0,
        }
    );
}

#[test]
fn test_parse_backend_matrix_delta_snapshot_from_legacy_manifest_without_timeout_delta_field() {
    let manifest = "fixture=real_world_strict_cmake_backend_matrix_first_failure\nbaseline_backend=libclang baseline_configure_status=0 baseline_build_status=2 baseline_build_timed_out=false baseline_first_failure_class=other_rustc_error baseline_first_failure_e0425_count=0\nbackend=libclang configure_status=0 build_status=2 build_timed_out=false first_failure_class=other_rustc_error first_failure_e0425_count=0 configure_status_delta_vs_baseline=0 build_status_delta_vs_baseline=0 class_delta_vs_baseline=false e0425_delta_vs_baseline=0\nbackend=libtooling configure_status=0 build_status=124 build_timed_out=true first_failure_class=compile_timeout first_failure_e0425_count=1 configure_status_delta_vs_baseline=0 build_status_delta_vs_baseline=122 class_delta_vs_baseline=true e0425_delta_vs_baseline=1\n";
    let snapshot = parse_backend_matrix_delta_snapshot_from_manifest(manifest, "libtooling")
        .expect("expected legacy manifest parse to derive timeout delta from build_timed_out");
    assert_eq!(
        snapshot,
        BackendMatrixDeltaSnapshot {
            configure_status_delta_vs_baseline: 0,
            build_status_delta_vs_baseline: 122,
            class_delta_vs_baseline: true,
            e0425_delta_vs_baseline: 1,
            timeout_incidence_delta_vs_baseline: 1,
        }
    );
}

#[test]
fn test_ensure_backend_matrix_delta_non_increase_enforces_all_dimensions() {
    let baseline = BackendMatrixDeltaSnapshot {
        configure_status_delta_vs_baseline: 1,
        build_status_delta_vs_baseline: 124,
        class_delta_vs_baseline: true,
        e0425_delta_vs_baseline: 40,
        timeout_incidence_delta_vs_baseline: 1,
    };
    let improved = BackendMatrixDeltaSnapshot {
        configure_status_delta_vs_baseline: 0,
        build_status_delta_vs_baseline: 1,
        class_delta_vs_baseline: true,
        e0425_delta_vs_baseline: 0,
        timeout_incidence_delta_vs_baseline: 0,
    };
    ensure_backend_matrix_delta_non_increase(improved, baseline)
        .expect("expected improved deltas to satisfy non-increase gate");

    let regressed_timeout = BackendMatrixDeltaSnapshot {
        timeout_incidence_delta_vs_baseline: 2,
        ..improved
    };
    let err = ensure_backend_matrix_delta_non_increase(regressed_timeout, baseline)
        .expect_err("expected timeout-incidence regression to fail non-increase gate");
    assert!(
        err.contains("timeout-incidence"),
        "expected timeout regression error context, got: {}",
        err
    );

    let regressed_e0425 = BackendMatrixDeltaSnapshot {
        e0425_delta_vs_baseline: 41,
        ..improved
    };
    let err = ensure_backend_matrix_delta_non_increase(regressed_e0425, baseline)
        .expect_err("expected E0425 regression to fail non-increase gate");
    assert!(
        err.contains("E0425"),
        "expected E0425 regression error context, got: {}",
        err
    );
}

#[test]
fn test_rapidjson_strict_backend_toggle_local_fixture_keeps_e0425_delta_at_baseline() {
    let root = unique_temp_dir("rapidjson_strict_backend_toggle_e0425_delta");
    fs::create_dir_all(&root).expect("failed to create strict backend-toggle fixture root");

    let (log_dir, results) = run_local_strict_backend_toggle_e0425_delta_replay_fixture(&root)
        .expect("failed to run strict backend-toggle E0425-delta fixture");
    for rel in RAPIDJSON_STRICT_BACKEND_TOGGLE_LOCAL_FIXTURE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict backend-toggle fixture log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        results.len(),
        3,
        "strict backend-toggle fixture should produce three backend replay results"
    );

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing strict backend-toggle baseline result for libclang");

    for result in &results {
        assert_eq!(
            result.first_failure_e0425_count, baseline.first_failure_e0425_count,
            "strict backend-toggle fixture should keep E0425 count delta at baseline for backend {} (logs: {})",
            result.backend_name,
            log_dir.display()
        );
        assert_eq!(
            result.compile_status, baseline.compile_status,
            "strict backend-toggle fixture compile status should match baseline backend for {} (logs: {})",
            result.backend_name,
            log_dir.display()
        );
        assert_eq!(
            result.first_failure_class == "unresolved_name_or_type_e0425",
            result.first_failure_e0425_count > 0,
            "strict backend-toggle fixture class/count mismatch for backend {} (logs: {})",
            result.backend_name,
            log_dir.display()
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_ci_workflow_keeps_rapidjson_smoke_coverage() {
    let ci_workflow = read_workflow_file("ci.yml")
        .expect("failed to read CI workflow for rapidjson smoke coverage");
    assert!(
        ci_workflow.contains("rapidjson-smoke-baseline"),
        "CI workflow should keep rapidjson smoke lane"
    );
    for invocation in RAPIDJSON_CI_SMOKE_REQUIRED_TEST_INVOCATIONS {
        assert!(
            ci_workflow.contains(invocation),
            "CI workflow should keep rapidjson smoke invocation `{}`",
            invocation
        );
    }
}

#[test]
fn test_rapidjson_nightly_workflow_keeps_matrix_coverage() {
    let nightly_workflow = read_workflow_file("rapidjson-nightly.yml")
        .expect("failed to read rapidjson nightly workflow for coverage");
    assert!(
        nightly_workflow.contains("rapidjson-nightly-matrix"),
        "rapidjson nightly workflow should keep matrix job"
    );
    for test_name in RAPIDJSON_NIGHTLY_REQUIRED_TEST_NAMES {
        assert!(
            nightly_workflow.contains(test_name),
            "rapidjson nightly workflow should keep matrix entry `{}`",
            test_name
        );
    }
}

#[test]
fn test_todo_keeps_ordered_failure_class_clearance_ledger() {
    let todo = read_todo_file().expect("failed to read TODO.md for ordered failure ledger");
    assert!(
        todo.contains("Ordered failure-class clearance ledger (active sequence)"),
        "TODO should keep ordered failure-class clearance ledger section"
    );

    let mut last_position = 0usize;
    for marker in RAPIDJSON_ORDERED_FAILURE_CLASS_LEDGER_MARKERS {
        let position = todo.find(marker).unwrap_or_else(|| {
            panic!(
                "TODO ordered failure-class ledger missing marker `{}`",
                marker
            )
        });
        assert!(
            position >= last_position,
            "TODO ordered failure-class marker `{}` should appear after prior markers",
            marker
        );
        last_position = position;
    }
}

#[test]
#[ignore = "real-world external project test (downloads rapidjson fixture)"]
fn test_real_world_rapidjson_fixture_checkout_is_pinned() {
    let repo_dir = ensure_rapidjson_checkout().expect("failed to prepare rapidjson checkout");
    for rel in RAPIDJSON_REQUIRED_PATHS {
        assert!(
            repo_dir.join(rel).exists(),
            "expected checkout path {}",
            repo_dir.join(rel).display()
        );
    }

    let head = read_head(&repo_dir).expect("failed to query rapidjson checkout HEAD");
    assert_eq!(
        head, RAPIDJSON_PINNED_COMMIT,
        "rapidjson checkout must stay pinned for deterministic runs"
    );
}

#[test]
#[ignore = "real-world external project test (builds/runs rapidjson no-stl examples baseline)"]
fn test_real_world_rapidjson_native_no_stl_examples_baseline() {
    let log_dir =
        run_rapidjson_native_baseline().expect("failed to run rapidjson native no-stl baseline");

    for rel in RAPIDJSON_NATIVE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected baseline log file {}",
            log_dir.join(rel).display()
        );
    }

    let manifest = fs::read_to_string(log_dir.join("native_baseline_manifest.txt"))
        .expect("failed to read native_baseline_manifest.txt");
    assert!(
        manifest.contains(RAPIDJSON_PINNED_COMMIT),
        "manifest should record pinned commit {}, got:\n{}",
        RAPIDJSON_PINNED_COMMIT,
        manifest
    );

    let condense_stdout = fs::read_to_string(log_dir.join("run_condense.stdout"))
        .expect("failed to read run_condense.stdout");
    assert_eq!(
        condense_stdout.trim(),
        RAPIDJSON_EXPECTED_CONDENSE_OUTPUT,
        "condense output should match expected compact JSON"
    );

    let pretty_stdout = fs::read_to_string(log_dir.join("run_pretty.stdout"))
        .expect("failed to read run_pretty.stdout");
    assert!(
        pretty_stdout.contains("\"msg\": \"hi\""),
        "pretty output should preserve JSON fields, got:\n{}",
        pretty_stdout
    );
}

#[test]
#[ignore = "real-world external project test (builds/runs rapidjson examples with CXX=fragilec strict-mode driver)"]
fn test_real_world_rapidjson_fragilec_native_no_stl_examples_baseline() {
    let log_dir = run_rapidjson_fragilec_driver_baseline()
        .expect("failed to run rapidjson fragilec-driver no-stl baseline");

    for rel in RAPIDJSON_FRAGILEC_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragilec-driver log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        read_status_file(&log_dir.join("compile_condense_driver.status"))
            .expect("failed to read compile_condense_driver.status"),
        0,
        "real-world strict condense compile should succeed"
    );
    assert_eq!(
        read_status_file(&log_dir.join("compile_pretty_driver.status"))
            .expect("failed to read compile_pretty_driver.status"),
        0,
        "real-world strict pretty compile should succeed"
    );
    let condense_run_status = read_status_file(&log_dir.join("run_condense_driver.status"))
        .expect("failed to read run_condense_driver.status");
    let pretty_run_status = read_status_file(&log_dir.join("run_pretty_driver.status"))
        .expect("failed to read run_pretty_driver.status");

    let condense_stdout = fs::read_to_string(log_dir.join("run_condense_driver.stdout"))
        .expect("failed to read run_condense_driver.stdout");
    let condense_stderr = fs::read_to_string(log_dir.join("run_condense_driver.stderr"))
        .expect("failed to read run_condense_driver.stderr");
    assert_eq!(
        condense_run_status, 0,
        "real-world strict condense run should succeed; stderr:\n{}",
        condense_stderr
    );
    assert!(
        condense_stderr.trim().is_empty(),
        "real-world strict condense stderr should be empty, got:\n{}",
        condense_stderr
    );
    assert_eq!(
        condense_stdout.trim(),
        RAPIDJSON_EXPECTED_CONDENSE_OUTPUT,
        "real-world strict condense output should match expected compact JSON"
    );

    let pretty_stdout = fs::read_to_string(log_dir.join("run_pretty_driver.stdout"))
        .expect("failed to read run_pretty_driver.stdout");
    let pretty_stderr = fs::read_to_string(log_dir.join("run_pretty_driver.stderr"))
        .expect("failed to read run_pretty_driver.stderr");
    assert_eq!(
        pretty_run_status, 0,
        "real-world strict pretty run should succeed; stderr:\n{}",
        pretty_stderr
    );
    assert!(
        pretty_stderr.trim().is_empty(),
        "real-world strict pretty stderr should be empty, got:\n{}",
        pretty_stderr
    );
    assert!(
        rapidjson_pretty_output_matches_expected(&pretty_stdout),
        "real-world strict pretty output should preserve JSON fields, got:\n{}",
        pretty_stdout
    );
}

#[test]
#[ignore = "real-world external project test (strict capitalize backend surface delta capture with FRAGILEC_KEEP_RS=1)"]
fn test_real_world_rapidjson_strict_capitalize_backend_surface_delta_capture() {
    let (log_dir, results) = run_rapidjson_strict_capitalize_backend_surface_delta_capture()
        .expect("failed to run strict capitalize backend surface-delta capture");
    let run_root = log_dir
        .parent()
        .expect("strict capitalize backend-surface log dir should have a run root");
    let expected_run_root_prefix =
        format!("{}_", RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_DIR);
    assert!(
        run_root
            .to_string_lossy()
            .starts_with(expected_run_root_prefix.as_str()),
        "expected strict capitalize backend-surface run root to start with {} but got {}",
        expected_run_root_prefix,
        run_root.display()
    );

    for rel in RAPIDJSON_STRICT_CAPITALIZE_BACKEND_SURFACE_DELTA_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict capitalize backend-surface log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        results.len(),
        2,
        "strict capitalize backend-surface capture should produce two backend replay results"
    );

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing strict capitalize backend-surface baseline result for libclang");
    assert!(
        baseline.sidecar_exists,
        "strict capitalize backend-surface baseline should emit a generated sidecar"
    );
    assert!(
        baseline.generated_surface_inventory.is_some(),
        "strict capitalize backend-surface baseline should have generated-surface inventory"
    );
    let baseline_inventory = baseline
        .generated_surface_inventory
        .as_ref()
        .expect("baseline inventory should exist");
    assert!(
        baseline_inventory.line_count > 0,
        "strict capitalize backend-surface baseline generated sidecar should be non-empty"
    );
    assert!(
        baseline_inventory.placeholder_count > 0 && baseline_inventory.c_void_alias_count > 0,
        "strict capitalize backend-surface baseline inventory should capture fallback markers"
    );
    assert!(
        baseline.transpile_stage_timing_exists,
        "strict capitalize backend-surface baseline should persist transpile timing trace"
    );
    assert_eq!(
        baseline.transpile_stage_timing.status.as_deref(),
        Some("completed"),
        "strict capitalize backend-surface baseline transpile timing should complete"
    );
    assert!(
        baseline.transpile_stage_timing.parse_ms.is_some()
            && baseline.transpile_stage_timing.codegen_ms.is_some()
            && baseline.transpile_stage_timing.total_ms.is_some(),
        "strict capitalize backend-surface baseline transpile timing should include parse/codegen/total durations"
    );

    let libtooling = results
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing strict capitalize backend-surface result for libtooling");
    let libtooling_class =
        fs::read_to_string(log_dir.join("backend_libtooling/first_failing_compile_class.txt"))
            .expect("failed to read backend_libtooling/first_failing_compile_class.txt");
    assert_eq!(
        libtooling_class.trim(),
        libtooling.first_failure_class,
        "strict capitalize backend-surface libtooling class file should match captured class"
    );
    let libtooling_stderr =
        fs::read_to_string(log_dir.join("backend_libtooling/compile_capitalize.stderr"))
            .expect("failed to read backend_libtooling/compile_capitalize.stderr");
    assert!(
        libtooling.transpile_stage_timing_exists,
        "strict capitalize backend-surface libtooling run should persist stage timing trace"
    );
    assert!(
        libtooling.transpile_stage_timing.last_stage_started.is_some(),
        "strict capitalize backend-surface libtooling timing trace should capture at least one started stage"
    );
    assert!(
        !libtooling.compile_timed_out,
        "strict capitalize backend-surface libtooling run must not timeout after hotspot fix"
    );
    assert_ne!(
        libtooling.compile_status, COMMAND_TIMEOUT_STATUS,
        "strict capitalize backend-surface libtooling run must not report timeout status"
    );
    assert_ne!(
        libtooling.first_failure_class, "compile_timeout",
        "strict capitalize backend-surface libtooling run must not classify compile_timeout"
    );
    assert!(
        !libtooling_stderr.contains("command timed out after"),
        "strict capitalize backend-surface libtooling stderr should not include timeout diagnostic after hotspot fix, got:\n{}",
        libtooling_stderr
    );
    assert!(
        libtooling.sidecar_exists,
        "strict capitalize backend-surface libtooling run should emit sidecar after hotspot fix"
    );
    assert_eq!(
        libtooling.transpile_stage_timing.status.as_deref(),
        Some("completed"),
        "strict capitalize backend-surface libtooling run should complete timing trace"
    );

    let manifest =
        fs::read_to_string(log_dir.join("strict_capitalize_backend_surface_delta_manifest.txt"))
            .expect("failed to read strict_capitalize_backend_surface_delta_manifest.txt");
    assert!(
        manifest.contains("baseline_backend=libclang"),
        "strict capitalize backend-surface manifest should include baseline metadata, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("compile_timeout_secs="),
        "strict capitalize backend-surface manifest should include timeout metadata, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("baseline_transpile_timing_exists=true"),
        "strict capitalize backend-surface manifest should include baseline transpile timing metadata, got:\n{}",
        manifest
    );
    let run_root_marker = format!("run_root={}", run_root.display());
    assert!(
        manifest.contains(run_root_marker.as_str()),
        "strict capitalize backend-surface manifest should include run_root marker `{}`. got:\n{}",
        run_root_marker,
        manifest
    );

    for result in &results {
        let line = manifest
            .lines()
            .find(|entry| entry.starts_with(format!("backend={} ", result.backend_name).as_str()))
            .unwrap_or_else(|| {
                panic!(
                    "strict capitalize backend-surface manifest missing backend line for {}:\n{}",
                    result.backend_name, manifest
                )
            });
        for marker in [
            format!("compile_status={}", result.compile_status),
            format!("compile_timed_out={}", result.compile_timed_out),
            format!("first_failure_class={}", result.first_failure_class),
            format!(
                "first_failure_e0425_count={}",
                result.first_failure_e0425_count
            ),
            format!("sidecar_exists={}", result.sidecar_exists),
            format!(
                "transpile_timing_exists={}",
                result.transpile_stage_timing_exists
            ),
            format!(
                "transpile_timing_path={}",
                result.transpile_stage_timing_path.display()
            ),
            format!(
                "transpile_parse_ms={}",
                format_optional_u128(result.transpile_stage_timing.parse_ms)
            ),
            format!(
                "transpile_export_ms={}",
                format_optional_u128(result.transpile_stage_timing.export_ms)
            ),
            format!(
                "transpile_enrichment_ms={}",
                format_optional_u128(result.transpile_stage_timing.enrichment_ms)
            ),
            format!(
                "transpile_codegen_ms={}",
                format_optional_u128(result.transpile_stage_timing.codegen_ms)
            ),
            format!(
                "transpile_total_ms={}",
                format_optional_u128(result.transpile_stage_timing.total_ms)
            ),
            format!(
                "transpile_last_stage_started={}",
                format_optional_str(result.transpile_stage_timing.last_stage_started.as_deref())
            ),
            format!(
                "transpile_last_stage_completed={}",
                format_optional_str(
                    result
                        .transpile_stage_timing
                        .last_stage_completed
                        .as_deref()
                )
            ),
            format!(
                "transpile_status={}",
                format_optional_str(result.transpile_stage_timing.status.as_deref())
            ),
        ] {
            assert!(
                line.contains(marker.as_str()),
                "strict capitalize backend-surface manifest line for {} should contain `{}`. line:\n{}",
                result.backend_name,
                marker,
                line
            );
        }
        if result.sidecar_exists {
            let inventory = result
                .generated_surface_inventory
                .as_ref()
                .expect("backend with sidecar should have inventory");
            for marker in [
                format!("surface_line_count={}", inventory.line_count),
                format!("surface_placeholder_count={}", inventory.placeholder_count),
                format!(
                    "surface_rapidjson_placeholder_count={}",
                    inventory.rapidjson_placeholder_count
                ),
                format!(
                    "surface_c_void_alias_count={}",
                    inventory.c_void_alias_count
                ),
                format!(
                    "surface_parse_unspecific_count={}",
                    inventory.parse_unspecific_syntax_error_count
                ),
            ] {
                assert!(
                    line.contains(marker.as_str()),
                    "strict capitalize backend-surface manifest line for {} should contain `{}`. line:\n{}",
                    result.backend_name,
                    marker,
                    line
                );
            }
        } else {
            assert!(
                line.contains("surface_line_count=na"),
                "strict capitalize backend-surface manifest line for {} should mark missing sidecar inventory as `na`. line:\n{}",
                result.backend_name,
                line
            );
        }
    }
}

#[test]
#[ignore = "real-world external project test (strict tutorial backend surface delta capture with FRAGILEC_KEEP_RS=1)"]
fn test_real_world_rapidjson_strict_tutorial_backend_surface_delta_capture() {
    let (log_dir, results) = run_rapidjson_strict_tutorial_backend_surface_delta_capture()
        .expect("failed to run strict tutorial backend surface-delta capture");
    let run_root = log_dir
        .parent()
        .expect("strict tutorial backend-surface log dir should have a run root");
    let expected_run_root_prefix = format!("{}_", RAPIDJSON_STRICT_TUTORIAL_BACKEND_SURFACE_DELTA_DIR);
    assert!(
        run_root
            .to_string_lossy()
            .starts_with(expected_run_root_prefix.as_str()),
        "expected strict tutorial backend-surface run root to start with {} but got {}",
        expected_run_root_prefix,
        run_root.display()
    );

    for rel in RAPIDJSON_STRICT_TUTORIAL_BACKEND_SURFACE_DELTA_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict tutorial backend-surface log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        results.len(),
        2,
        "strict tutorial backend-surface capture should produce two backend replay results"
    );

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing strict tutorial backend-surface baseline result for libclang");
    assert!(
        !baseline.compile_timed_out,
        "strict tutorial backend-surface baseline should not timeout"
    );
    assert_ne!(
        baseline.compile_status, COMMAND_TIMEOUT_STATUS,
        "strict tutorial backend-surface baseline should not report timeout sentinel status"
    );
    assert!(
        baseline.sidecar_exists,
        "strict tutorial backend-surface baseline should emit a generated sidecar"
    );
    assert!(
        baseline.transpile_stage_timing_exists,
        "strict tutorial backend-surface baseline should persist transpile timing trace"
    );

    let libtooling = results
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing strict tutorial backend-surface result for libtooling");
    assert!(
        !libtooling.compile_timed_out,
        "strict tutorial backend-surface libtooling run should complete without timeout so first blocker classification is deterministic"
    );
    assert_ne!(
        libtooling.compile_status, COMMAND_TIMEOUT_STATUS,
        "strict tutorial backend-surface libtooling run should not report timeout sentinel status"
    );
    assert_eq!(
        libtooling.first_failure_class, "unresolved_name_or_type_e0425",
        "strict tutorial backend-surface libtooling run should deterministically classify unresolved-name/type blocker after exporter unblocks"
    );
    assert_ne!(
        libtooling.first_failure_class, "none",
        "strict tutorial backend-surface libtooling run should capture an actual blocker classification"
    );
    let libtooling_first_stderr =
        fs::read_to_string(log_dir.join("backend_libtooling/first_failing_compile_stderr.txt"))
            .expect("failed to read backend_libtooling/first_failing_compile_stderr.txt");
    assert!(
        libtooling_first_stderr.contains("[fragilec] fragile rustc object compile failed"),
        "strict tutorial backend-surface libtooling first failure should capture rustc compile blocker after exporter unblocks, got:\n{}",
        libtooling_first_stderr
    );
    assert!(
        libtooling_first_stderr.contains("error[E0425]"),
        "strict tutorial backend-surface libtooling first failure should include unresolved-name/type diagnostics for deterministic classification, got:\n{}",
        libtooling_first_stderr
    );
    assert!(
        !libtooling_first_stderr.contains("AST export failed with code 1"),
        "strict tutorial backend-surface libtooling first failure should no longer report AST export blocker, got:\n{}",
        libtooling_first_stderr
    );
    assert!(
        libtooling.sidecar_exists,
        "strict tutorial backend-surface libtooling run should emit a generated sidecar after exporter unblock"
    );
    assert!(
        libtooling.transpile_stage_timing_exists,
        "strict tutorial backend-surface libtooling run should persist transpile timing trace"
    );
    assert!(
        libtooling.transpile_stage_timing.last_stage_started.is_some(),
        "strict tutorial backend-surface libtooling timing trace should capture at least one started stage"
    );

    let manifest =
        fs::read_to_string(log_dir.join("strict_tutorial_backend_surface_delta_manifest.txt"))
            .expect("failed to read strict_tutorial_backend_surface_delta_manifest.txt");
    assert!(
        manifest.contains("baseline_backend=libclang"),
        "strict tutorial backend-surface manifest should include baseline metadata, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("compile_timeout_secs="),
        "strict tutorial backend-surface manifest should include timeout metadata, got:\n{}",
        manifest
    );
    let run_root_marker = format!("run_root={}", run_root.display());
    assert!(
        manifest.contains(run_root_marker.as_str()),
        "strict tutorial backend-surface manifest should include run_root marker `{}`. got:\n{}",
        run_root_marker,
        manifest
    );

    for result in &results {
        let line = manifest
            .lines()
            .find(|entry| entry.starts_with(format!("backend={} ", result.backend_name).as_str()))
            .unwrap_or_else(|| {
                panic!(
                    "strict tutorial backend-surface manifest missing backend line for {}:\n{}",
                    result.backend_name, manifest
                )
            });
        for marker in [
            format!("compile_status={}", result.compile_status),
            format!("compile_timed_out={}", result.compile_timed_out),
            format!("first_failure_class={}", result.first_failure_class),
            format!(
                "first_failure_e0425_count={}",
                result.first_failure_e0425_count
            ),
            format!("sidecar_exists={}", result.sidecar_exists),
            format!(
                "transpile_timing_exists={}",
                result.transpile_stage_timing_exists
            ),
            format!(
                "transpile_timing_path={}",
                result.transpile_stage_timing_path.display()
            ),
            format!(
                "transpile_parse_ms={}",
                format_optional_u128(result.transpile_stage_timing.parse_ms)
            ),
            format!(
                "transpile_export_ms={}",
                format_optional_u128(result.transpile_stage_timing.export_ms)
            ),
            format!(
                "transpile_enrichment_ms={}",
                format_optional_u128(result.transpile_stage_timing.enrichment_ms)
            ),
            format!(
                "transpile_codegen_ms={}",
                format_optional_u128(result.transpile_stage_timing.codegen_ms)
            ),
            format!(
                "transpile_total_ms={}",
                format_optional_u128(result.transpile_stage_timing.total_ms)
            ),
            format!(
                "transpile_status={}",
                format_optional_str(result.transpile_stage_timing.status.as_deref())
            ),
        ] {
            assert!(
                line.contains(marker.as_str()),
                "strict tutorial backend-surface manifest line for {} should contain `{}`. line:\n{}",
                result.backend_name,
                marker,
                line
            );
        }
    }
}

#[test]
#[ignore = "real-world external project test (strict capitalize compile replay with fragilec first-failure capture)"]
fn test_real_world_rapidjson_strict_capitalize_compile_capture() {
    let log_dir = run_rapidjson_strict_capitalize_compile_capture()
        .expect("failed to run strict capitalize compile capture");

    for rel in RAPIDJSON_STRICT_CAPITALIZE_CAPTURE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict capitalize capture log file {}",
            log_dir.join(rel).display()
        );
    }

    let compile_status = read_status_file(&log_dir.join("compile_capitalize.status"))
        .expect("failed to read compile_capitalize.status");
    let compile_stdout = fs::read_to_string(log_dir.join("compile_capitalize.stdout"))
        .expect("failed to read compile_capitalize.stdout");
    let compile_stderr = fs::read_to_string(log_dir.join("compile_capitalize.stderr"))
        .expect("failed to read compile_capitalize.stderr");
    assert_ne!(
        compile_status, 0,
        "strict capitalize replay is expected to fail until downstream blockers are cleared"
    );

    let first_command = fs::read_to_string(log_dir.join("first_failing_compile_command.txt"))
        .expect("failed to read first_failing_compile_command.txt");
    assert!(
        first_command.contains("capitalize.cpp"),
        "strict capitalize replay should capture capitalize compile command, got:\n{}",
        first_command
    );

    let first_stderr = fs::read_to_string(log_dir.join("first_failing_compile_stderr.txt"))
        .expect("failed to read first_failing_compile_stderr.txt");
    assert!(
        first_stderr.contains("error[E0425]"),
        "strict capitalize replay should now surface unresolved-name/type E0425 errors, got:\n{}",
        first_stderr
    );
    for stream in [&compile_stdout, &compile_stderr, &first_stderr] {
        assert!(
            !stream.contains(RAPIDJSON_DUPLICATE_DEFINITION_E0428_FRAGMENT),
            "strict capitalize replay should not surface duplicate-definition E0428 in captured streams, got:\n{}",
            stream
        );
    }

    let first_class = fs::read_to_string(log_dir.join("first_failing_compile_class.txt"))
        .expect("failed to read first_failing_compile_class.txt");
    assert_eq!(
        first_class.trim(),
        "unresolved_name_or_type_e0425",
        "strict capitalize replay should classify first failure as unresolved E0425, got:\n{}",
        first_class
    );
}

#[test]
#[ignore = "real-world external project test (strict filterkeydom compile replay with fragilec first-failure capture)"]
fn test_real_world_rapidjson_strict_filterkeydom_compile_capture() {
    let log_dir = run_rapidjson_strict_filterkeydom_compile_capture()
        .expect("failed to run strict filterkeydom compile capture");

    for rel in RAPIDJSON_STRICT_FILTERKEYDOM_CAPTURE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict filterkeydom capture log file {}",
            log_dir.join(rel).display()
        );
    }

    let compile_status = read_status_file(&log_dir.join("compile_filterkeydom.status"))
        .expect("failed to read compile_filterkeydom.status");
    let compile_stdout = fs::read_to_string(log_dir.join("compile_filterkeydom.stdout"))
        .expect("failed to read compile_filterkeydom.stdout");
    let compile_stderr = fs::read_to_string(log_dir.join("compile_filterkeydom.stderr"))
        .expect("failed to read compile_filterkeydom.stderr");
    assert_eq!(
        compile_status, 0,
        "strict filterkeydom replay should now compile successfully"
    );

    for stream in [&compile_stdout, &compile_stderr] {
        assert!(
            !stream.contains(RAPIDJSON_DUPLICATE_DEFINITION_E0428_FRAGMENT),
            "strict filterkeydom replay should not surface duplicate-definition E0428 in captured streams, got:\n{}",
            stream
        );
    }

    // Extract the generated Rust file path from the fragilec driver log.
    let driver_log = fs::read_to_string(log_dir.join("fragilec_driver.log")).unwrap_or_default();
    let generated_rs_path = driver_log
        .lines()
        .find_map(|l| {
            l.split_whitespace()
                .find(|tok| tok.contains("fragilec_") && tok.ends_with("_filterkeydom.rs"))
        })
        .or_else(|| {
            compile_stderr.lines().find_map(|l| {
                l.split_whitespace()
                    .find(|tok| tok.contains("fragilec_") && tok.ends_with("_filterkeydom.rs"))
            })
        });

    if let Some(rs_path) = generated_rs_path {
        let generated_rs = fs::read_to_string(rs_path).unwrap_or_default();
        assert!(
            generated_rs.contains(
                "pub type GenericDocument_UTF8_ = GenericDocument_Encoding__Allocator__StackAllocator;"
            ),
            "strict filterkeydom replay should alias GenericDocument_UTF8_ to concrete specialization"
        );
        assert!(
            !generated_rs.contains("pub struct GenericDocument_UTF8_ {"),
            "strict filterkeydom replay should not emit opaque GenericDocument_UTF8_ placeholder"
        );
    }
}

#[test]
#[ignore = "real-world external project test (rapidjson cmake no-tests full build with fragilec strict and first-failure capture)"]
fn test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure() {
    let (log_dir, build_dir) = run_rapidjson_strict_cmake_no_tests_full_build_capture()
        .expect("failed to run rapidjson strict cmake no-tests build capture");

    for rel in RAPIDJSON_STRICT_CMAKE_NO_TESTS_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict cmake capture log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        read_status_file(&log_dir.join("cmake_configure.status"))
            .expect("failed to read cmake_configure.status"),
        0,
        "strict cmake configure should succeed with RAPIDJSON_BUILD_TESTS=OFF"
    );

    let _build_status = read_status_file(&log_dir.join("cmake_build.status"))
        .expect("failed to read cmake_build.status");
    let cmake_build_stdout = fs::read_to_string(log_dir.join("cmake_build.stdout"))
        .expect("failed to read cmake_build.stdout");
    let _cmake_build_stderr = fs::read_to_string(log_dir.join("cmake_build.stderr"))
        .expect("failed to read cmake_build.stderr");
    let _first_command = fs::read_to_string(log_dir.join("first_failing_compile_command.txt"))
        .expect("failed to read first_failing_compile_command.txt");
    let _first_stderr = fs::read_to_string(log_dir.join("first_failing_compile_stderr.txt"))
        .expect("failed to read first_failing_compile_stderr.txt");
    let _first_class = fs::read_to_string(log_dir.join("first_failing_compile_class.txt"))
        .expect("failed to read first_failing_compile_class.txt");
    // Regression marker checks on cmake_build_stdout only.
    // With -k (keep-going), cmake_build_stderr contains errors from known-failing targets
    // (serialize, tutorial) whose errors include markers like E0605. So we only check stdout
    // which reflects configure/link diagnostics, not per-target compile errors.
    {
        let stream = &cmake_build_stdout;
        assert!(
            !stream.contains("strict link requires a real `main` symbol for executable outputs"),
            "strict rapidjson no-tests replay should not regress to shim-only missing-main diagnostics, got:\n{}",
            stream
        );
        assert!(
            !stream.contains("main symbol diagnostic:\n  defining objects: <none>"),
            "strict rapidjson no-tests replay should not report shim-only missing-main symbol diagnostics, got:\n{}",
            stream
        );
        assert!(
            !stream.contains(RAPIDJSON_CONST_ASSIGN_PARSER_DIAGNOSTIC_FRAGMENT),
            "strict rapidjson no-tests replay should not regress to rapidjson document.h const-assignment parse diagnostics, got:\n{}",
            stream
        );
        assert!(
            !stream.contains(RAPIDJSON_DUPLICATE_DEFINITION_E0428_FRAGMENT),
            "strict rapidjson no-tests replay should not surface duplicate-definition E0428 in captured streams, got:\n{}",
            stream
        );
        assert!(
            !stream.contains(RAPIDJSON_FILE_ALIAS_MISSING_TYPE_FRAGMENT),
            "strict rapidjson no-tests replay should not regress to unresolved __FILE alias types, got:\n{}",
            stream
        );
        assert!(
            !stream.contains(RAPIDJSON_STD_IDENTITY_MISSING_TYPE_FRAGMENT),
            "strict rapidjson no-tests replay should not regress to unresolved std___identity alias types, got:\n{}",
            stream
        );
        assert!(
            !stream.contains(RAPIDJSON_FUNCTIONAL_HASH_UNNAMED_STRUCT_MISSING_TYPE_FRAGMENT),
            "strict rapidjson no-tests replay should not regress to unresolved libc++ functional-hash unnamed-struct aliases, got:\n{}",
            stream
        );
        assert!(
            !stream.contains(RAPIDJSON_ATOMIC_BASE_ALIAS_MISSING_TYPE_FRAGMENT),
            "strict rapidjson no-tests replay should not regress to unresolved __cxx_atomic_base_impl_bool alias types, got:\n{}",
            stream
        );
        for marker in RAPIDJSON_ITEM5_CAST_DECAY_CALL_SHAPE_MARKERS {
            assert!(
                !stream.contains(marker),
                "strict rapidjson no-tests replay should not regress to item-5 cast/decay/call-shape marker `{}`, got:\n{}",
                marker,
                stream
            );
        }
    }
    // --- Count successfully built targets ---
    let built_targets: Vec<&str> = cmake_build_stdout
        .lines()
        .filter_map(|line| {
            if line.contains("Built target ") {
                line.split("Built target ").last().map(|s| s.trim())
            } else {
                None
            }
        })
        .collect();

    // Known failing targets (require std::string type resolution in user constructors)
    let known_failing: &[&str] = &["serialize", "tutorial"];
    let required_targets: &[&str] = &[
        "capitalize",
        "condense",
        "filterkey",
        "filterkeydom",
        "jsonx",
        "messagereader",
        "parsebyparts",
        "pretty",
        "prettyauto",
        "schemavalidator",
        "simpledom",
        "simplereader",
        "simplewriter",
    ];

    // Assert all required targets built
    for target in required_targets {
        assert!(
            built_targets.contains(target),
            "required target `{}` should have built successfully; built targets: {:?}",
            target,
            built_targets
        );
    }

    // Write target build summary
    let target_summary = format!(
        "built_targets={}\nbuilt_count={}\nknown_failing={:?}\n",
        built_targets.join(","),
        built_targets.len(),
        known_failing,
    );
    fs::write(log_dir.join("target_build_summary.txt"), target_summary)
        .expect("failed to write target_build_summary.txt");

    // --- Runtime validation: run CMake-built condense and pretty ---
    // These should always be available since they're in required_targets.
    let bin_dir = build_dir.join("bin");

    // Run condense
    let condense_bin = bin_dir.join("condense");
    assert!(
        condense_bin.exists(),
        "CMake build should produce bin/condense at {}",
        condense_bin.display()
    );
    let condense_output = run_example_with_stdin(
        &condense_bin,
        RAPIDJSON_SAMPLE_JSON,
        &log_dir,
        "run_cmake_condense",
    )
    .expect("failed to run CMake-built condense");
    let condense_run_status = status_code(&condense_output);
    let condense_stdout = String::from_utf8_lossy(&condense_output.stdout).to_string();
    let condense_stderr = String::from_utf8_lossy(&condense_output.stderr).to_string();
    assert_eq!(
        condense_run_status, 0,
        "CMake-built condense should run successfully; stderr:\n{}",
        condense_stderr
    );
    assert!(
        condense_stderr.trim().is_empty(),
        "CMake-built condense stderr should be empty, got:\n{}",
        condense_stderr
    );
    assert_eq!(
        condense_stdout.trim(),
        RAPIDJSON_EXPECTED_CONDENSE_OUTPUT,
        "CMake-built condense output should match expected compact JSON"
    );

    // Run pretty
    let pretty_bin = bin_dir.join("pretty");
    assert!(
        pretty_bin.exists(),
        "CMake build should produce bin/pretty at {}",
        pretty_bin.display()
    );
    let pretty_output = run_example_with_stdin(
        &pretty_bin,
        RAPIDJSON_SAMPLE_JSON,
        &log_dir,
        "run_cmake_pretty",
    )
    .expect("failed to run CMake-built pretty");
    let pretty_run_status = status_code(&pretty_output);
    let pretty_stdout = String::from_utf8_lossy(&pretty_output.stdout).to_string();
    let pretty_stderr = String::from_utf8_lossy(&pretty_output.stderr).to_string();
    assert_eq!(
        pretty_run_status, 0,
        "CMake-built pretty should run successfully; stderr:\n{}",
        pretty_stderr
    );
    assert!(
        pretty_stderr.trim().is_empty(),
        "CMake-built pretty stderr should be empty, got:\n{}",
        pretty_stderr
    );
    assert!(
        rapidjson_pretty_output_matches_expected(&pretty_stdout),
        "CMake-built pretty output should match expected JSON structure, got:\n{}",
        pretty_stdout
    );

    // --- Native baseline comparison ---
    let source_dir = PathBuf::from(RAPIDJSON_STRICT_CMAKE_NO_TESTS_BUILD_DIR).join("worktree");
    let native_log_dir = log_dir.join("native_comparison");
    run_native_no_stl_examples_in_tree(&source_dir, &native_log_dir)
        .expect("failed to run native baseline for comparison");

    let native_condense_stdout = fs::read_to_string(native_log_dir.join("run_condense.stdout"))
        .expect("failed to read native run_condense.stdout");
    let native_pretty_stdout = fs::read_to_string(native_log_dir.join("run_pretty.stdout"))
        .expect("failed to read native run_pretty.stdout");

    assert_eq!(
        condense_stdout.trim(),
        native_condense_stdout.trim(),
        "CMake-built condense output should match native baseline"
    );
    assert_eq!(
        pretty_stdout.trim(),
        native_pretty_stdout.trim(),
        "CMake-built pretty output should match native baseline"
    );

    // Write runtime comparison manifest
    let comparison_manifest = format!(
        "cmake_condense_status={}\ncmake_pretty_status={}\nnative_condense_matches={}\nnative_pretty_matches={}\ncondense_output={}\npretty_output_lines={}\n",
        condense_run_status,
        pretty_run_status,
        condense_stdout.trim() == native_condense_stdout.trim(),
        pretty_stdout.trim() == native_pretty_stdout.trim(),
        condense_stdout.trim(),
        pretty_stdout.lines().count(),
    );
    fs::write(
        log_dir.join("runtime_comparison_manifest.txt"),
        comparison_manifest,
    )
    .expect("failed to write runtime_comparison_manifest.txt");
}

#[test]
#[ignore = "real-world external project test (rapidjson strict cmake no-tests backend matrix capture: libclang baseline vs libtooling)"]
fn test_real_world_rapidjson_strict_cmake_no_tests_backend_matrix_capture_first_failure() {
    let previous_libtooling_delta_baseline =
        latest_completed_backend_matrix_delta_baseline("libtooling");
    let (log_dir, results) = run_rapidjson_strict_cmake_no_tests_backend_matrix_capture()
        .expect("failed to run rapidjson strict cmake no-tests backend-matrix capture");
    let run_root = log_dir
        .parent()
        .expect("strict cmake backend-matrix log dir should have a run root");
    let expected_run_root_prefix =
        format!("{}_", RAPIDJSON_STRICT_CMAKE_NO_TESTS_BACKEND_MATRIX_DIR);
    assert!(
        run_root
            .to_string_lossy()
            .starts_with(expected_run_root_prefix.as_str()),
        "expected strict cmake backend-matrix run root to start with {} but got {}",
        expected_run_root_prefix,
        run_root.display()
    );

    for rel in RAPIDJSON_STRICT_CMAKE_BACKEND_MATRIX_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected strict cmake backend-matrix log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        results.len(),
        2,
        "strict cmake backend-matrix capture should produce two backend replay results"
    );

    let baseline = results
        .iter()
        .find(|entry| entry.backend_name == "libclang")
        .expect("missing strict cmake backend-matrix baseline result for libclang");
    assert_eq!(
        baseline.configure_status, 0,
        "strict cmake backend-matrix baseline configure should succeed"
    );

    let manifest = fs::read_to_string(log_dir.join("strict_cmake_backend_matrix_manifest.txt"))
        .expect("failed to read strict_cmake_backend_matrix_manifest.txt");
    assert!(
        manifest.contains("baseline_backend=libclang"),
        "strict cmake backend-matrix manifest should include baseline metadata, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("build_timeout_secs="),
        "strict cmake backend-matrix manifest should include build timeout metadata, got:\n{}",
        manifest
    );
    let run_root_marker = format!("run_root={}", run_root.display());
    assert!(
        manifest.contains(run_root_marker.as_str()),
        "strict cmake backend-matrix manifest should include run_root metadata `{}`. got:\n{}",
        run_root_marker,
        manifest
    );

    for result in &results {
        let line = manifest
            .lines()
            .find(|entry| entry.starts_with(format!("backend={} ", result.backend_name).as_str()))
            .unwrap_or_else(|| {
                panic!(
                    "strict cmake backend-matrix manifest missing backend line for {}:\n{}",
                    result.backend_name, manifest
                )
            });

        let configure_status_delta_vs_baseline =
            result.configure_status - baseline.configure_status;
        let build_status_delta_vs_baseline = result.build_status - baseline.build_status;
        let class_delta_vs_baseline = result.first_failure_class != baseline.first_failure_class;
        let e0425_delta_vs_baseline =
            result.first_failure_e0425_count as i64 - baseline.first_failure_e0425_count as i64;
        let timeout_incidence_delta_vs_baseline =
            bool_to_i64(result.build_timed_out) - bool_to_i64(baseline.build_timed_out);

        for marker in [
            format!("configure_status={}", result.configure_status),
            format!("build_status={}", result.build_status),
            format!("build_timed_out={}", result.build_timed_out),
            format!("first_failure_class={}", result.first_failure_class),
            format!(
                "first_failure_e0425_count={}",
                result.first_failure_e0425_count
            ),
            format!(
                "configure_status_delta_vs_baseline={}",
                configure_status_delta_vs_baseline
            ),
            format!(
                "build_status_delta_vs_baseline={}",
                build_status_delta_vs_baseline
            ),
            format!("class_delta_vs_baseline={}", class_delta_vs_baseline),
            format!("e0425_delta_vs_baseline={}", e0425_delta_vs_baseline),
            format!(
                "timeout_incidence_delta_vs_baseline={}",
                timeout_incidence_delta_vs_baseline
            ),
        ] {
            assert!(
                line.contains(marker.as_str()),
                "strict cmake backend-matrix manifest line for {} should contain `{}`. line:\n{}",
                result.backend_name,
                marker,
                line
            );
        }
    }

    let libtooling = results
        .iter()
        .find(|entry| entry.backend_name == "libtooling")
        .expect("missing strict cmake backend-matrix replay result for libtooling");
    assert!(
        !libtooling.build_timed_out,
        "strict cmake backend-matrix libtooling replay must complete without build timeout before closing 5.4.c.ii.3 (logs: {})",
        log_dir.display()
    );
    assert_ne!(
        libtooling.first_failure_class, "compile_timeout",
        "strict cmake backend-matrix libtooling first-failure class must not be compile_timeout before closing 5.4.c.ii.3 (logs: {})",
        log_dir.display()
    );
    assert_ne!(
        libtooling.build_status, COMMAND_TIMEOUT_STATUS,
        "strict cmake backend-matrix libtooling replay must not report timeout sentinel status before closing 5.4.c.ii.3 (logs: {})",
        log_dir.display()
    );
    let current_libtooling_delta = compute_backend_matrix_delta_snapshot(libtooling, baseline);
    if let Some((baseline_manifest_path, baseline_delta)) = previous_libtooling_delta_baseline {
        ensure_backend_matrix_delta_non_increase(current_libtooling_delta, baseline_delta)
            .unwrap_or_else(|why| {
                panic!(
                    "strict cmake backend-matrix libtooling delta must be non-increasing vs previous baseline manifest {}: {}\ncurrent={:?}\nbaseline={:?}",
                    baseline_manifest_path.display(),
                    why,
                    current_libtooling_delta,
                    baseline_delta
                )
            });
    }
}

#[test]
fn test_parse_transpile_stage_timing_trace_supports_complete_and_partial_logs() {
    let log_root = unique_prefixed_dir(RAPIDJSON_TRANSPILE_STAGE_TIMING_PARSE_FIXTURE_DIR);
    fs::create_dir_all(&log_root)
        .expect("failed to create transpile stage timing parse fixture log dir");

    let complete_trace = log_root.join("complete.log");
    fs::write(
        &complete_trace,
        "source=/tmp/fixture.cpp\nbackend=libclang\nstatus=started\nevent=stage_start stage=parse\nevent=stage_end stage=parse status=ok elapsed_ms=11\nevent=stage_skip stage=export elapsed_ms=0 reason=backend_without_export\nevent=stage_skip stage=enrichment elapsed_ms=0 reason=backend_without_enrichment\nevent=stage_start stage=codegen\nevent=stage_end stage=codegen status=ok elapsed_ms=9\nsummary parse_ms=11 export_ms=0 enrichment_ms=0 codegen_ms=9 total_ms=20\nstatus=completed\n",
    )
    .expect("failed to write complete transpile stage timing trace fixture");

    let (complete_exists, complete) = parse_transpile_stage_timing_trace(&complete_trace)
        .expect("failed to parse complete transpile stage timing trace");
    assert!(
        complete_exists,
        "complete trace should be reported as existing"
    );
    assert_eq!(complete.parse_ms, Some(11));
    assert_eq!(complete.export_ms, Some(0));
    assert_eq!(complete.enrichment_ms, Some(0));
    assert_eq!(complete.codegen_ms, Some(9));
    assert_eq!(complete.total_ms, Some(20));
    assert_eq!(complete.last_stage_started.as_deref(), Some("codegen"));
    assert_eq!(complete.last_stage_completed.as_deref(), Some("codegen"));
    assert_eq!(complete.status.as_deref(), Some("completed"));

    let partial_trace = log_root.join("partial.log");
    fs::write(
        &partial_trace,
        "source=/tmp/fixture.cpp\nbackend=libtooling\nstatus=started\nevent=stage_start stage=export\n",
    )
    .expect("failed to write partial transpile stage timing trace fixture");
    let (partial_exists, partial) = parse_transpile_stage_timing_trace(&partial_trace)
        .expect("failed to parse partial transpile stage timing trace");
    assert!(
        partial_exists,
        "partial trace should be reported as existing"
    );
    assert_eq!(partial.last_stage_started.as_deref(), Some("export"));
    assert_eq!(partial.last_stage_completed, None);
    assert_eq!(partial.status.as_deref(), Some("started"));
    assert_eq!(partial.total_ms, None);

    let missing_trace = log_root.join("missing.log");
    let (missing_exists, missing) = parse_transpile_stage_timing_trace(&missing_trace)
        .expect("failed to parse missing transpile stage timing trace");
    assert!(
        !missing_exists,
        "missing trace path should be reported as not existing"
    );
    assert_eq!(missing, TranspileStageTimingSnapshot::default());
}

#[test]
#[ignore = "real-world external project test (derives rapidjson no-stl command plan)"]
fn test_real_world_rapidjson_no_stl_command_plan_generation() {
    let log_dir = run_rapidjson_no_stl_command_plan()
        .expect("failed to run rapidjson no-stl command-plan baseline");

    for rel in RAPIDJSON_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected command-plan log file {}",
            log_dir.join(rel).display()
        );
    }

    let manifest = fs::read_to_string(log_dir.join("no_stl_examples_manifest.txt"))
        .expect("failed to read no_stl_examples_manifest.txt");
    assert!(
        manifest.contains("compile[0]=")
            && manifest.contains("example/condense/condense.cpp")
            && manifest.contains("example/pretty/pretty.cpp"),
        "manifest should include no-stl command-plan coverage, got:\n{}",
        manifest
    );
}

#[test]
#[ignore = "real-world external project test (replays rapidjson condense single TU through fragile)"]
fn test_real_world_rapidjson_fragile_condense_single_tu_replay() {
    match run_rapidjson_fragile_condense_single_tu_replay() {
        Ok(log_dir) => {
            for rel in RAPIDJSON_FRAGILE_CONDENSE_REPLAY_LOG_FILES {
                assert!(
                    log_dir.join(rel).exists(),
                    "expected replay log file {}",
                    log_dir.join(rel).display()
                );
            }
            assert_eq!(
                read_status_file(&log_dir.join("rustc_fragile_condense.status"))
                    .expect("failed to read rustc_fragile_condense.status"),
                0,
                "real-world fragile condense single-tu replay should compile when replay is unblocked"
            );
        }
        Err(err) => {
            let known_blockers = [
                "typename Encoding::Ch",
                "StaticAssertTypedef",
                "failed to parse",
            ];
            assert!(
                known_blockers.iter().any(|sig| err.contains(sig)),
                "unexpected rapidjson condense replay failure signature:\n{}",
                err
            );
        }
    }
}
