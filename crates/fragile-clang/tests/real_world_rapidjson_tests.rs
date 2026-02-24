//! Real-world RapidJSON fixture bootstrap tests.
//!
//! This target focuses on no-STL runtime examples (`condense`, `pretty`) to
//! provide deterministic next-stage development coverage.

use fragile_clang::{AstCodeGen, ClangParser};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
const RAPIDJSON_STRICT_FILTERKEYDOM_CAPTURE_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_filterkeydom_capture";
const RAPIDJSON_STRICT_CMAKE_NO_TESTS_BUILD_DIR: &str =
    "/tmp/fragile_real_world_rapidjson_strict_cmake_no_tests_build";
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
    if !fragilec.exists() {
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

fn read_status_file(path: &Path) -> Result<i32, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    raw.trim()
        .parse::<i32>()
        .map_err(|e| format!("failed to parse status file {}: {}", path.display(), e))
}

fn rapidjson_pretty_output_matches_expected(pretty_stdout: &str) -> bool {
    pretty_stdout.contains("\n")
        && pretty_stdout.contains("\"msg\": \"hi\"")
        && pretty_stdout.contains("    \"a\": 1")
}

fn write_command_capture(log_dir: &Path, step: &str, output: &Output) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;
    fs::write(
        log_dir.join(format!("{}.status", step)),
        format!("{}\n", status_code(output)),
    )
    .map_err(|e| format!("failed to write {}.status: {}", step, e))?;
    fs::write(log_dir.join(format!("{}.stdout", step)), &output.stdout)
        .map_err(|e| format!("failed to write {}.stdout: {}", step, e))?;
    fs::write(log_dir.join(format!("{}.stderr", step)), &output.stderr)
        .map_err(|e| format!("failed to write {}.stderr: {}", step, e))?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FragilecDriverInvocation {
    cwd: String,
    args: String,
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

fn first_failing_compile_command_from_driver_log(driver_log: &str) -> Option<String> {
    let invocations = parse_fragilec_driver_invocations(driver_log);
    invocations
        .last()
        .map(|inv| format!("cwd={}\nargs={}", inv.cwd, inv.args))
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

    let command = first_failing_compile_command_from_driver_log(driver_log)
        .unwrap_or_else(|| "<unavailable>".to_string());
    let stderr = if !build_stderr.trim().is_empty() {
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

fn extract_filterkeydom_generated_rs_path(first_stderr: &str) -> Option<PathBuf> {
    for token in first_stderr.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| c == '"' || c == '\'' || c == '(' || c == ')');
        if let Some(start) = trimmed.find("/tmp/fragilec_") {
            let candidate = &trimmed[start..];
            if let Some(end) = candidate.find("_filterkeydom.rs") {
                let path_end = end + "_filterkeydom.rs".len();
                let path = &candidate[..path_end];
                return Some(PathBuf::from(path));
            }
        }
    }
    None
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

fn run_rapidjson_strict_cmake_no_tests_full_build_capture() -> Result<PathBuf, String> {
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

    Ok(log_dir)
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
        "#!/usr/bin/env bash\nset -euo pipefail\nlog=\"${FRAGILEC_LOG:-}\"\nif [[ -n \"$log\" ]]; then\n  printf 'cwd=%s\\n' \"$(pwd)\" >> \"$log\"\n  printf 'args=%s\\n' \"$*\" >> \"$log\"\nfi\nfor arg in \"$@\"; do\n  if [[ \"$arg\" == *\"fail.cpp\"* ]]; then\n    echo \"forced local fixture compile failure for fail.cpp\" >&2\n    exit 42\n  fi\ndone\nexec c++ \"$@\"\n",
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
fn test_select_first_failing_compile_capture_uses_last_driver_invocation_and_stderr() {
    let driver_log = "cwd=/tmp/work\nargs=-std=c++11 -c first.cpp -o first.o \ncwd=/tmp/work\nargs=-std=c++11 -c failing.cpp -o failing.o \n";
    let (command, stderr) = select_first_failing_compile_capture(
        driver_log,
        true,
        "stdout text",
        "failing stderr text",
    );
    assert!(
        command.contains("failing.cpp"),
        "capture should report failing compile invocation, got:\n{}",
        command
    );
    assert_eq!(
        stderr, "failing stderr text",
        "capture should prefer build stderr payload on failure"
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
    assert_ne!(
        compile_status, 0,
        "strict filterkeydom replay is expected to fail until downstream blockers are cleared"
    );

    let first_command = fs::read_to_string(log_dir.join("first_failing_compile_command.txt"))
        .expect("failed to read first_failing_compile_command.txt");
    assert!(
        first_command.contains("filterkeydom.cpp"),
        "strict filterkeydom replay should capture filterkeydom compile command, got:\n{}",
        first_command
    );

    let first_stderr = fs::read_to_string(log_dir.join("first_failing_compile_stderr.txt"))
        .expect("failed to read first_failing_compile_stderr.txt");
    assert!(
        first_stderr.contains("error[E0425]"),
        "strict filterkeydom replay should now surface unresolved-name/type E0425 errors, got:\n{}",
        first_stderr
    );
    for stream in [&compile_stdout, &compile_stderr, &first_stderr] {
        assert!(
            !stream.contains(RAPIDJSON_DUPLICATE_DEFINITION_E0428_FRAGMENT),
            "strict filterkeydom replay should not surface duplicate-definition E0428 in captured streams, got:\n{}",
            stream
        );
    }
    for marker in RAPIDJSON_FILTERKEYDOM_PLACEHOLDER_API_HOLE_MARKERS {
        assert!(
            !first_stderr.contains(marker),
            "strict filterkeydom replay should no longer surface RapidJSON placeholder API-hole marker `{}` after surface fallback fixes, got:\n{}",
            marker,
            first_stderr
        );
    }
    let generated_rs_path = extract_filterkeydom_generated_rs_path(&first_stderr)
        .expect("strict filterkeydom replay stderr should include generated fragilec Rust path");
    let generated_rs = fs::read_to_string(&generated_rs_path).unwrap_or_else(|err| {
        panic!(
            "failed to read generated filterkeydom Rust file {}: {}",
            generated_rs_path.display(),
            err
        )
    });
    assert!(
        generated_rs.contains(
            "pub type GenericDocument_UTF8_ = GenericDocument_Encoding__Allocator__StackAllocator;"
        ),
        "strict filterkeydom replay should alias GenericDocument_UTF8_ to concrete specialization, got:\n{}",
        generated_rs
    );
    assert!(
        !generated_rs.contains("pub struct GenericDocument_UTF8_ {"),
        "strict filterkeydom replay should not emit opaque GenericDocument_UTF8_ placeholder struct when concrete specialization is available, got:\n{}",
        generated_rs
    );
    assert!(
        !first_stderr.contains(RAPIDJSON_CONST_ASSIGN_PARSER_DIAGNOSTIC_FRAGMENT),
        "strict filterkeydom replay should not regress to rapidjson document.h const-assignment parse diagnostics, got:\n{}",
        first_stderr
    );
    assert!(
        !first_stderr.contains(RAPIDJSON_FILE_ALIAS_MISSING_TYPE_FRAGMENT),
        "strict filterkeydom replay should not regress to unresolved __FILE alias types, got:\n{}",
        first_stderr
    );
    assert!(
        !first_stderr.contains(RAPIDJSON_STD_IDENTITY_MISSING_TYPE_FRAGMENT),
        "strict filterkeydom replay should not regress to unresolved std___identity alias types, got:\n{}",
        first_stderr
    );
    assert!(
        !first_stderr.contains(RAPIDJSON_FUNCTIONAL_HASH_UNNAMED_STRUCT_MISSING_TYPE_FRAGMENT),
        "strict filterkeydom replay should not regress to unresolved libc++ functional-hash unnamed-struct aliases, got:\n{}",
        first_stderr
    );
    assert!(
        !first_stderr.contains(RAPIDJSON_ATOMIC_BASE_ALIAS_MISSING_TYPE_FRAGMENT),
        "strict filterkeydom replay should not regress to unresolved __cxx_atomic_base_impl_bool alias types, got:\n{}",
        first_stderr
    );
    for marker in RAPIDJSON_ITEM5_CAST_DECAY_CALL_SHAPE_MARKERS {
        assert!(
            !first_stderr.contains(marker),
            "strict filterkeydom replay should not regress to item-5 cast/decay/call-shape marker `{}`, got:\n{}",
            marker,
            first_stderr
        );
    }
    for marker in RAPIDJSON_ITEM6_63_CLEARED_MARKERS {
        assert!(
            !first_stderr.contains(marker),
            "strict filterkeydom replay should no longer surface cleared item-6.3 marker `{}`, got:\n{}",
            marker,
            first_stderr
        );
    }
    for marker in RAPIDJSON_ITEM6_62_CLEARED_MARKERS {
        assert!(
            !first_stderr.contains(marker),
            "strict filterkeydom replay should no longer surface cleared item-6.2 marker `{}`, got:\n{}",
            marker,
            first_stderr
        );
    }

    let first_class = fs::read_to_string(log_dir.join("first_failing_compile_class.txt"))
        .expect("failed to read first_failing_compile_class.txt");
    assert_eq!(
        first_class.trim(),
        "unresolved_name_or_type_e0425",
        "strict filterkeydom replay should classify first failure as unresolved E0425, got:\n{}",
        first_class
    );
}

#[test]
#[ignore = "real-world external project test (rapidjson cmake no-tests full build with fragilec strict and first-failure capture)"]
fn test_real_world_rapidjson_cmake_no_tests_full_build_with_fragilec_capture_first_failure() {
    let log_dir = run_rapidjson_strict_cmake_no_tests_full_build_capture()
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

    let build_status = read_status_file(&log_dir.join("cmake_build.status"))
        .expect("failed to read cmake_build.status");
    let cmake_build_stdout = fs::read_to_string(log_dir.join("cmake_build.stdout"))
        .expect("failed to read cmake_build.stdout");
    let cmake_build_stderr = fs::read_to_string(log_dir.join("cmake_build.stderr"))
        .expect("failed to read cmake_build.stderr");
    let first_command = fs::read_to_string(log_dir.join("first_failing_compile_command.txt"))
        .expect("failed to read first_failing_compile_command.txt");
    let first_stderr = fs::read_to_string(log_dir.join("first_failing_compile_stderr.txt"))
        .expect("failed to read first_failing_compile_stderr.txt");
    let first_class = fs::read_to_string(log_dir.join("first_failing_compile_class.txt"))
        .expect("failed to read first_failing_compile_class.txt");
    for stream in [&cmake_build_stdout, &cmake_build_stderr, &first_stderr] {
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
    if build_status != 0 {
        assert!(
            first_command.contains("args=") && !first_command.trim().is_empty(),
            "failing build should capture first failing compile command, got:\n{}",
            first_command
        );
        assert!(
            !first_stderr.trim().is_empty() && first_stderr.trim() != "<none>",
            "failing build should capture failing stderr diagnostics"
        );
        assert_eq!(
            first_class.trim(),
            "unresolved_name_or_type_e0425",
            "post-dedupe strict cmake first failure class should be unresolved name/type (E0425), got:\n{}",
            first_class
        );
        assert!(
            first_stderr.contains("error[E0425]"),
            "post-dedupe strict cmake first failure should include unresolved E0425 diagnostics"
        );
        for marker in RAPIDJSON_STRICT_CMAKE_CAPITALIZE_GLOBAL_REMAP_BLOCKER_MARKERS {
            assert!(
                !first_stderr.contains(marker),
                "strict rapidjson no-tests replay should no longer surface cleared 3.2 remap marker `{}`, got:\n{}",
                marker,
                first_stderr
            );
        }
        for marker in RAPIDJSON_STRICT_CMAKE_CAPITALIZE_CONSTEXPR_BLOCKER_MARKERS {
            assert!(
                !first_stderr.contains(marker),
                "strict rapidjson no-tests replay should no longer surface cleared 3.3 constexpr marker `{}`, got:\n{}",
                marker,
                first_stderr
            );
        }
        for marker in RAPIDJSON_STRICT_CMAKE_CAPITALIZE_NEW0_CLEARED_MARKERS {
            assert!(
                !first_stderr.contains(marker),
                "strict rapidjson no-tests replay should no longer surface cleared 3.4 marker `{}`, got:\n{}",
                marker,
                first_stderr
            );
        }
        for marker in RAPIDJSON_ITEM6_63_CLEARED_MARKERS {
            assert!(
                !first_stderr.contains(marker),
                "strict rapidjson no-tests replay should no longer surface cleared item-6.3 marker `{}`, got:\n{}",
                marker,
                first_stderr
            );
        }
        for marker in RAPIDJSON_ITEM6_62_CLEARED_MARKERS {
            assert!(
                !first_stderr.contains(marker),
                "strict rapidjson no-tests replay should no longer surface cleared item-6.2 marker `{}`, got:\n{}",
                marker,
                first_stderr
            );
        }
    } else {
        assert_eq!(
            first_command.trim(),
            "<none>",
            "successful build should not report failing compile command"
        );
        assert_eq!(
            first_stderr.trim(),
            "<none>",
            "successful build should not report failing compile stderr"
        );
        assert_eq!(
            first_class.trim(),
            "none",
            "successful build should classify first-failure class as none"
        );
    }
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
