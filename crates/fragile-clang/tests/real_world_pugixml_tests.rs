//! Real-world pugixml fixture bootstrap tests.
//!
//! This target is intentionally staged as a non-STL C++ project baseline to
//! drive the next development phase after zlib/tinyxml2.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PUGIXML_REPO_URL: &str = "https://github.com/zeux/pugixml.git";
const PUGIXML_PINNED_COMMIT: &str = "ee86beb30e4973f5feffe3ce63bfa4fbadf72f38"; // v1.15
const PUGIXML_CACHE_DIR: &str = "/tmp/fragile_real_world_pugixml";
const PUGIXML_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_pugixml_native_baseline";
const PUGIXML_COMMAND_PLAN_DIR: &str = "/tmp/fragile_real_world_pugixml_make_test_command_plan";
const PUGIXML_FRAGILE_SINGLE_TU_REPLAY_DIR: &str =
    "/tmp/fragile_real_world_pugixml_fragile_single_tu_replay";
const PUGIXML_FRAGILEC_DRIVER_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_pugixml_fragilec_driver_baseline";
const PUGIXML_REQUIRED_PATHS: &[&str] = &["src/pugixml.cpp", "src/pugixml.hpp", "tests/main.cpp", "Makefile"];
const PUGIXML_NATIVE_LOG_FILES: &[&str] = &[
    "make_clean.status",
    "make_clean.stdout",
    "make_clean.stderr",
    "make_test.status",
    "make_test.stdout",
    "make_test.stderr",
    "native_baseline_manifest.txt",
];
const PUGIXML_COMMAND_PLAN_LOG_FILES: &[&str] = &[
    "make_test_dryrun.status",
    "make_test_dryrun.stdout",
    "make_test_dryrun.stderr",
    "make_test_commands_manifest.txt",
];
const PUGIXML_FRAGILE_SINGLE_TU_LOG_FILES: &[&str] = &[
    "fragile_transpile_pugixml_single_tu.status",
    "fragile_transpile_pugixml_single_tu.stdout",
    "fragile_transpile_pugixml_single_tu.stderr",
    "rustc_fragile_pugixml_single_tu.status",
    "rustc_fragile_pugixml_single_tu.stdout",
    "rustc_fragile_pugixml_single_tu.stderr",
    "fragile_single_tu_replay_manifest.txt",
];
const PUGIXML_FRAGILEC_DRIVER_LOG_FILES: &[&str] = &[
    "make_clean_driver.status",
    "make_clean_driver.stdout",
    "make_clean_driver.stderr",
    "make_test_driver.status",
    "make_test_driver.stdout",
    "make_test_driver.stderr",
    "fragilec_driver.log",
    "fragilec_driver_manifest.txt",
];
const PUGIXML_CI_SMOKE_REQUIRED_TEST_INVOCATIONS: &[&str] = &[
    "test_make_test_no_stl_local_fixture_success",
    "test_make_test_command_plan_local_fixture_success",
    "test_fragile_pugixml_single_tu_replay_local_fixture_success",
    "test_fragilec_driver_make_test_no_stl_local_fixture_success",
];
const PUGIXML_NIGHTLY_REQUIRED_TEST_NAMES: &[&str] = &[
    "test_real_world_pugixml_fixture_checkout_is_pinned",
    "test_real_world_pugixml_make_test_command_plan_generation",
    "test_real_world_pugixml_native_make_test_no_stl",
    "test_real_world_pugixml_fragilec_make_test_no_stl",
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
        run_git(&["clone", "--no-tags", repo_url, repo_dir_str.as_str()], None)?;
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

fn ensure_pugixml_checkout() -> Result<PathBuf, String> {
    ensure_pinned_checkout(
        PUGIXML_REPO_URL,
        Path::new(PUGIXML_CACHE_DIR),
        PUGIXML_PINNED_COMMIT,
        PUGIXML_REQUIRED_PATHS,
    )
}

fn status_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn read_status_file(path: &Path) -> Result<i32, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    raw.trim()
        .parse::<i32>()
        .map_err(|e| format!("failed to parse status file {}: {}", path.display(), e))
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

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove {}: {}", path.display(), e))?;
    }
    fs::create_dir_all(path).map_err(|e| format!("failed to create {}: {}", path.display(), e))
}

fn parse_make_test_commands_from_dry_run(dry_run_stdout: &str) -> Result<Vec<String>, String> {
    let mut commands = Vec::new();
    for line in dry_run_stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("./") && trimmed.contains("/test") && !trimmed.starts_with("./tests/") {
            if !commands.contains(&trimmed.to_string()) {
                commands.push(trimmed.to_string());
            }
        }
    }

    if commands.is_empty() {
        return Err("no make test runtime commands found in dry-run output".to_string());
    }

    Ok(commands)
}

fn write_make_test_commands_manifest(log_dir: &Path, commands: &[String]) -> Result<(), String> {
    let mut manifest = format!("command_count={}\n", commands.len());
    for (idx, cmd) in commands.iter().enumerate() {
        manifest.push_str(&format!("command[{idx}]={cmd}\n"));
    }

    fs::write(log_dir.join("make_test_commands_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write make_test_commands_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn extract_success_count(stdout: &str) -> Option<u32> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Success: ") {
            let mut parts = rest.split_whitespace();
            if let Some(count_raw) = parts.next() {
                if let Ok(count) = count_raw.parse::<u32>() {
                    return Some(count);
                }
            }
        }
    }
    None
}

fn run_native_baseline_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let mut make_clean = Command::new("make");
    make_clean.arg("clean").current_dir(source_dir);
    make_clean
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let clean_output = make_clean.output().map_err(|e| {
        format!(
            "failed to run make clean at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_clean", &clean_output)?;

    let mut make_test = Command::new("make");
    make_test
        .arg("test")
        .arg("config=release")
        .arg("defines=PUGIXML_NO_STL")
        .arg("cxxstd=c++11")
        .current_dir(source_dir);
    make_test
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let make_test_output = make_test.output().map_err(|e| {
        format!(
            "failed to run make test at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_test", &make_test_output)?;
    if !make_test_output.status.success() {
        return Err(format!(
            "native baseline make test failed with status {} (logs: {})",
            status_code(&make_test_output),
            log_dir.display()
        ));
    }

    let stdout = String::from_utf8_lossy(&make_test_output.stdout);
    let success_count = extract_success_count(&stdout).ok_or_else(|| {
        format!(
            "make test output did not contain a `Success:` summary (logs: {})",
            log_dir.display()
        )
    })?;

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nsuccess_count={}\n",
        source_dir.display(),
        PUGIXML_PINNED_COMMIT,
        success_count
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

fn run_fragilec_driver_baseline_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let fragilec = ensure_fragilec_binary()?;
    let fragilec_str = fragilec.to_string_lossy().to_string();
    let driver_log = log_dir.join("fragilec_driver.log");
    fs::write(&driver_log, "")
        .map_err(|e| format!("failed to initialize fragilec driver log {}: {}", driver_log.display(), e))?;
    let driver_log_str = driver_log.to_string_lossy().to_string();

    let mut make_clean = Command::new("make");
    make_clean.arg("clean").current_dir(source_dir);
    make_clean
        .env("CXX", fragilec_str.as_str())
        .env("CXXLD", fragilec_str.as_str())
        .env("LINK", fragilec_str.as_str())
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log_str.as_str())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let clean_output = make_clean.output().map_err(|e| {
        format!(
            "failed to run fragilec-driver make clean at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_clean_driver", &clean_output)?;
    if !clean_output.status.success() {
        return Err(format!(
            "fragilec-driver make clean failed with status {} (logs: {})",
            status_code(&clean_output),
            log_dir.display()
        ));
    }

    let strict_probe_src = source_dir.join("build/fragilec_strict_probe.cpp");
    if let Some(parent) = strict_probe_src.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create strict-probe parent dir {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    fs::write(&strict_probe_src, "int main(void) { return 0; }\n").map_err(|e| {
        format!(
            "failed to write strict probe source {}: {}",
            strict_probe_src.display(),
            e
        )
    })?;
    let strict_probe_bin = source_dir.join("build/fragilec_strict_probe");
    let make_test_output = Command::new(&fragilec)
        .arg(strict_probe_src.to_string_lossy().to_string())
        .arg("-std=c++11")
        .arg("-o")
        .arg(strict_probe_bin.to_string_lossy().to_string())
        .current_dir(source_dir)
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_LOG", driver_log_str.as_str())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .map_err(|e| {
            format!(
                "failed to run fragilec strict probe at {}: {}",
                source_dir.display(),
                e
            )
        })?;
    write_command_capture(log_dir, "make_test_driver", &make_test_output)?;
    if make_test_output.status.success() {
        return Err(format!(
            "fragilec strict probe unexpectedly succeeded (logs: {})",
            log_dir.display()
        ));
    }
    let make_test_stderr = String::from_utf8_lossy(&make_test_output.stderr);
    if !make_test_stderr.contains("single-source compile-only (-c) invocations")
        && !make_test_stderr.contains("failed to parse")
    {
        return Err(format!(
            "fragilec-driver strict failure did not report expected diagnostics\nstderr:\n{}",
            make_test_stderr
        ));
    }

    let log_content = fs::read_to_string(&driver_log)
        .map_err(|e| format!("failed to read fragilec driver log {}: {}", driver_log.display(), e))?;
    if log_content.trim().is_empty() {
        return Err(format!(
            "fragilec driver log {} is empty; expected compiler invocations",
            driver_log.display()
        ));
    }

    let manifest = format!(
        "source_dir={}\npinned_commit={}\nfragilec={}\nmode=strict\n",
        source_dir.display(),
        PUGIXML_PINNED_COMMIT,
        fragilec.display()
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

fn run_make_test_command_plan_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let mut make_dryrun = Command::new("make");
    make_dryrun
        .arg("-n")
        .arg("test")
        .arg("config=release")
        .arg("defines=PUGIXML_NO_STL")
        .arg("cxxstd=c++11")
        .current_dir(source_dir);
    make_dryrun
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let dryrun_output = make_dryrun.output().map_err(|e| {
        format!(
            "failed to run make -n test at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_test_dryrun", &dryrun_output)?;

    if !dryrun_output.status.success() {
        return Err(format!(
            "make -n test failed with status {} (logs: {})",
            status_code(&dryrun_output),
            log_dir.display()
        ));
    }

    let dryrun_stdout = String::from_utf8_lossy(&dryrun_output.stdout);
    let commands = parse_make_test_commands_from_dry_run(&dryrun_stdout)?;
    write_make_test_commands_manifest(log_dir, &commands)?;
    Ok(())
}

fn compile_transpiled_rust_lib(
    transpiled_rs: &Path,
    output_rlib: &Path,
    log_dir: &Path,
    step_name: &str,
) -> Result<(), String> {
    let output = Command::new("rustc").env("RUSTC_BOOTSTRAP", "1")
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

fn transpile_pugixml_single_tu_with_cli(
    source_path: &Path,
    transpiled_rs: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("fragile-cli")
        .arg("--bin")
        .arg("fragile")
        .arg("--")
        .arg("transpile")
        .arg(source_path)
        .arg("--output")
        .arg(transpiled_rs)
        .current_dir(workspace_root_dir())
        .output()
        .map_err(|e| format!("failed to run fragile-cli transpile for {}: {}", source_path.display(), e))?;
    write_command_capture(log_dir, "fragile_transpile_pugixml_single_tu", &output)?;
    if !output.status.success() {
        return Err(format!(
            "fragile-cli transpile failed with status {} (logs: {})\nstdout:\n{}\nstderr:\n{}",
            status_code(&output),
            log_dir.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn run_fragile_single_tu_replay_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir)
        .map_err(|e| format!("failed to create log dir {}: {}", log_dir.display(), e))?;

    let source_path = source_dir.join("src/pugixml.cpp");
    if !source_path.exists() {
        return Err(format!(
            "pugixml source is missing at {}",
            source_path.display()
        ));
    }

    let transpiled_rs = log_dir.join("fragile_pugixml_single_tu_transpiled.rs");
    transpile_pugixml_single_tu_with_cli(&source_path, &transpiled_rs, log_dir)?;

    let rlib_path = log_dir.join("fragile_pugixml_single_tu.rlib");
    compile_transpiled_rust_lib(
        &transpiled_rs,
        &rlib_path,
        log_dir,
        "rustc_fragile_pugixml_single_tu",
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
        "source_dir={}\npinned_commit={}\nsource=src/pugixml.cpp\ntranspiled={}\noutput={}\noutput_size={}\n",
        source_dir.display(),
        PUGIXML_PINNED_COMMIT,
        transpiled_rs.display(),
        rlib_path.display(),
        object_size
    );
    fs::write(log_dir.join("fragile_single_tu_replay_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write fragile_single_tu_replay_manifest.txt in {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(())
}

fn run_pugixml_native_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_pugixml_checkout()?;
    let baseline_root = PathBuf::from(PUGIXML_NATIVE_BASELINE_DIR);
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
        &["checkout", "--detach", PUGIXML_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != PUGIXML_PINNED_COMMIT {
        return Err(format!(
            "native baseline worktree expected commit {} but got {}",
            PUGIXML_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("native_logs");
    run_native_baseline_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_pugixml_fragilec_driver_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_pugixml_checkout()?;
    let baseline_root = PathBuf::from(PUGIXML_FRAGILEC_DRIVER_BASELINE_DIR);
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
        &["checkout", "--detach", PUGIXML_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != PUGIXML_PINNED_COMMIT {
        return Err(format!(
            "fragilec-driver worktree expected commit {} but got {}",
            PUGIXML_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_fragilec_driver_baseline_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_pugixml_make_test_command_plan() -> Result<PathBuf, String> {
    let checkout_dir = ensure_pugixml_checkout()?;
    let baseline_root = PathBuf::from(PUGIXML_COMMAND_PLAN_DIR);
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
        &["checkout", "--detach", PUGIXML_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != PUGIXML_PINNED_COMMIT {
        return Err(format!(
            "command-plan worktree expected commit {} but got {}",
            PUGIXML_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("command_plan_logs");
    run_make_test_command_plan_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_pugixml_fragile_single_tu_replay() -> Result<PathBuf, String> {
    let checkout_dir = ensure_pugixml_checkout()?;
    let baseline_root = PathBuf::from(PUGIXML_FRAGILE_SINGLE_TU_REPLAY_DIR);
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
        &["checkout", "--detach", PUGIXML_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != PUGIXML_PINNED_COMMIT {
        return Err(format!(
            "fragile replay worktree expected commit {} but got {}",
            PUGIXML_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("replay_logs");
    run_fragile_single_tu_replay_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("fragile_{prefix}_{}_{}", std::process::id(), now))
}

fn create_local_pugixml_like_repo(base_dir: &Path) -> Result<(String, String, String), String> {
    let remote_dir = base_dir.join("remote");
    fs::create_dir_all(remote_dir.join("src"))
        .map_err(|e| format!("failed to create src dir: {}", e))?;
    fs::create_dir_all(remote_dir.join("tests"))
        .map_err(|e| format!("failed to create tests dir: {}", e))?;

    fs::write(remote_dir.join("src/pugixml.hpp"), "#pragma once\n")
        .map_err(|e| format!("failed to write pugixml.hpp: {}", e))?;
    fs::write(
        remote_dir.join("src/pugixml.cpp"),
        "int pugixml_fixture_version(void) { return 1; }\n",
    )
    .map_err(|e| format!("failed to write pugixml.cpp: {}", e))?;
    fs::write(
        remote_dir.join("tests/main.cpp"),
        "int main(void) { return 0; }\n",
    )
    .map_err(|e| format!("failed to write tests/main.cpp: {}", e))?;
    fs::write(
        remote_dir.join("Makefile"),
        "test:\n\t@mkdir -p build/make-g++-release-PUGIXML_NO_STL-c++11\n\t@printf '%s\\n' '#!/bin/sh' 'echo \"Success: 7 tests passed.\"' > build/make-g++-release-PUGIXML_NO_STL-c++11/test\n\t@chmod +x build/make-g++-release-PUGIXML_NO_STL-c++11/test\n\t@printf '%s\\n' 'int main(void) { return 0; }' > build/fragilec_driver_smoke.cpp\n\t@$(CXX) -std=c++11 build/fragilec_driver_smoke.cpp -o build/fragilec_driver_smoke\n\t@./build/make-g++-release-PUGIXML_NO_STL-c++11/test\n\nclean:\n\t@rm -rf build\n",
    )
    .map_err(|e| format!("failed to write Makefile: {}", e))?;

    run_git(&["init"], Some(&remote_dir))?;
    run_git(&["config", "user.name", "Fragile Test"], Some(&remote_dir))?;
    run_git(
        &["config", "user.email", "fragile-test@example.invalid"],
        Some(&remote_dir),
    )?;
    run_git(
        &["add", "src/pugixml.hpp", "src/pugixml.cpp", "tests/main.cpp", "Makefile"],
        Some(&remote_dir),
    )?;
    run_git(&["commit", "-m", "initial fixture"], Some(&remote_dir))?;

    let pinned_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;

    fs::write(
        remote_dir.join("src/pugixml.cpp"),
        "int pugixml_fixture_version(void) { return 2; }\n",
    )
    .map_err(|e| format!("failed to update pugixml.cpp: {}", e))?;
    run_git(&["add", "src/pugixml.cpp"], Some(&remote_dir))?;
    run_git(&["commit", "-m", "update fixture"], Some(&remote_dir))?;

    let newer_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;
    let repo_url = remote_dir.to_string_lossy().to_string();
    Ok((repo_url, pinned_commit, newer_commit))
}

#[test]
fn test_parse_make_test_commands_from_dry_run_detects_runtime_test_binary() {
    let dry_run_stdout = r#"
        g++ src/pugixml.cpp -o build/make-g++-release-PUGIXML_NO_STL-c++11/src/pugixml.cpp.o
        ./build/make-g++-release-PUGIXML_NO_STL-c++11/test
    "#;

    let commands = parse_make_test_commands_from_dry_run(dry_run_stdout)
        .expect("dry-run parser should detect runtime test command");
    assert_eq!(
        commands,
        vec!["./build/make-g++-release-PUGIXML_NO_STL-c++11/test".to_string()]
    );
}

#[test]
fn test_parse_make_test_commands_from_dry_run_reports_missing_runtime_commands() {
    let err = parse_make_test_commands_from_dry_run("echo build only")
        .expect_err("parser should fail when no runtime test command is present");
    assert!(
        err.contains("no make test runtime commands found"),
        "missing-command error should be explicit, got: {}",
        err
    );
}

#[test]
fn test_ensure_pinned_checkout_clones_and_rewinds_local_pugixml_fixture() {
    let root = unique_temp_dir("pugixml_checkout_pin");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, newer_commit) =
        create_local_pugixml_like_repo(&root).expect("failed to create local pugixml-like repo");
    let checkout_dir = root.join("checkout");

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        PUGIXML_REQUIRED_PATHS,
    )
    .expect("initial checkout should succeed");

    run_git(&["checkout", "--detach", newer_commit.as_str()], Some(&checkout_dir))
        .expect("failed to move checkout to newer commit");
    let moved_head = read_head(&checkout_dir).expect("failed to read moved HEAD");
    assert_eq!(moved_head, newer_commit, "checkout should move before rewind");

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        PUGIXML_REQUIRED_PATHS,
    )
    .expect("rewind checkout should succeed");

    let head = read_head(&checkout_dir).expect("failed to read pinned HEAD");
    assert_eq!(head, pinned_commit, "checkout should rewind to pinned commit");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_no_stl_local_fixture_success() {
    let root = unique_temp_dir("pugixml_native_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_pugixml_like_repo(&root).expect("failed to create local fixture repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        PUGIXML_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&checkout_dir, &log_dir)
        .expect("local pugixml fixture make-test should succeed");

    for rel in PUGIXML_NATIVE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected baseline log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        fs::read_to_string(log_dir.join("make_test.status"))
            .expect("failed to read make_test.status")
            .trim(),
        "0",
        "make-test status should be zero"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragilec_driver_make_test_no_stl_local_fixture_success() {
    let root = unique_temp_dir("pugixml_fragilec_driver_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_pugixml_like_repo(&root).expect("failed to create local fixture repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        PUGIXML_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("fragilec_driver_logs");
    run_fragilec_driver_baseline_in_tree(&checkout_dir, &log_dir)
        .expect("local fragilec-driver pugixml baseline should succeed");

    for rel in PUGIXML_FRAGILEC_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragilec-driver log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("make_test_driver.status"))
            .expect("failed to read make_test_driver.status"),
        2,
        "strict fragilec-driver make test should fail with strict-mode status"
    );
    let make_stderr = fs::read_to_string(log_dir.join("make_test_driver.stderr"))
        .expect("failed to read make_test_driver.stderr");
    assert!(
        make_stderr.contains("single-source compile-only (-c) invocations")
            || make_stderr.contains("failed to parse"),
        "strict fragilec driver stderr should explain unsupported compile shape, got:\n{}",
        make_stderr
    );
    let driver_log = fs::read_to_string(log_dir.join("fragilec_driver.log"))
        .expect("failed to read fragilec_driver.log");
    assert!(
        driver_log.contains("cwd=") && driver_log.contains("args="),
        "fragilec driver log should capture compiler invocations, got:\n{}",
        driver_log
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_plan_local_fixture_success() {
    let root = unique_temp_dir("pugixml_command_plan_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_pugixml_like_repo(&root).expect("failed to create local fixture repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        PUGIXML_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("command_plan_logs");
    run_make_test_command_plan_in_tree(&checkout_dir, &log_dir)
        .expect("local command-plan generation should succeed");

    for rel in PUGIXML_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected command-plan log file {}",
            log_dir.join(rel).display()
        );
    }

    let manifest = fs::read_to_string(log_dir.join("make_test_commands_manifest.txt"))
        .expect("failed to read make_test_commands_manifest.txt");
    assert!(
        manifest.contains("./build/make-g++-release-PUGIXML_NO_STL-c++11/test"),
        "manifest should include runtime test command, got:\n{}",
        manifest
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_pugixml_single_tu_replay_local_fixture_success() {
    let root = unique_temp_dir("pugixml_fragile_single_tu_local_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_pugixml_like_repo(&root).expect("failed to create local fixture repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        PUGIXML_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("replay_logs");
    run_fragile_single_tu_replay_in_tree(&checkout_dir, &log_dir)
        .expect("local pugixml single-tu fragile replay should succeed");

    for rel in PUGIXML_FRAGILE_SINGLE_TU_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected replay log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("rustc_fragile_pugixml_single_tu.status"))
            .expect("failed to read rustc_fragile_pugixml_single_tu.status"),
        0,
        "local single-tu replay rustc status should be zero"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_ci_workflow_keeps_pugixml_smoke_coverage() {
    let ci_workflow =
        read_workflow_file("ci.yml").expect("failed to read CI workflow for pugixml smoke coverage");
    assert!(
        ci_workflow.contains("pugixml-smoke-baseline"),
        "CI workflow should keep pugixml smoke lane"
    );
    for invocation in PUGIXML_CI_SMOKE_REQUIRED_TEST_INVOCATIONS {
        assert!(
            ci_workflow.contains(invocation),
            "CI workflow should keep pugixml smoke invocation `{}`",
            invocation
        );
    }
}

#[test]
fn test_pugixml_nightly_workflow_keeps_matrix_coverage() {
    let nightly_workflow = read_workflow_file("pugixml-nightly.yml")
        .expect("failed to read pugixml nightly workflow for coverage");
    assert!(
        nightly_workflow.contains("pugixml-nightly-matrix"),
        "pugixml nightly workflow should keep matrix job"
    );
    for test_name in PUGIXML_NIGHTLY_REQUIRED_TEST_NAMES {
        assert!(
            nightly_workflow.contains(test_name),
            "pugixml nightly workflow should keep matrix entry `{}`",
            test_name
        );
    }
}

#[test]
#[ignore = "real-world external project test (downloads pugixml fixture)"]
fn test_real_world_pugixml_fixture_checkout_is_pinned() {
    let repo_dir = ensure_pugixml_checkout().expect("failed to prepare pugixml checkout");
    for rel in PUGIXML_REQUIRED_PATHS {
        assert!(
            repo_dir.join(rel).exists(),
            "expected checkout path {}",
            repo_dir.join(rel).display()
        );
    }

    let head = read_head(&repo_dir).expect("failed to query pugixml checkout HEAD");
    assert_eq!(
        head, PUGIXML_PINNED_COMMIT,
        "pugixml checkout must stay pinned for deterministic runs"
    );
}

#[test]
#[ignore = "real-world external project test (builds pugixml no-stl make test baseline)"]
fn test_real_world_pugixml_native_make_test_no_stl() {
    let log_dir = run_pugixml_native_baseline().expect("failed to run pugixml native baseline");

    for rel in PUGIXML_NATIVE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected baseline log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        fs::read_to_string(log_dir.join("make_test.status"))
            .expect("failed to read make_test.status")
            .trim(),
        "0",
        "pugixml native baseline make test should succeed"
    );

    let stdout = fs::read_to_string(log_dir.join("make_test.stdout"))
        .expect("failed to read make_test.stdout");
    assert!(
        stdout.contains("Success:"),
        "make_test stdout should include success summary, got:\n{}",
        stdout
    );

    let manifest = fs::read_to_string(log_dir.join("native_baseline_manifest.txt"))
        .expect("failed to read native_baseline_manifest.txt");
    assert!(
        manifest.contains(PUGIXML_PINNED_COMMIT),
        "manifest should record pinned commit {}, got:\n{}",
        PUGIXML_PINNED_COMMIT,
        manifest
    );
}

#[test]
#[ignore = "real-world external project test (builds pugixml with CXX=fragilec strict-mode driver)"]
fn test_real_world_pugixml_fragilec_make_test_no_stl() {
    let log_dir =
        run_pugixml_fragilec_driver_baseline().expect("failed to run pugixml fragilec-driver baseline");

    for rel in PUGIXML_FRAGILEC_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragilec-driver log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        read_status_file(&log_dir.join("make_test_driver.status"))
            .expect("failed to read make_test_driver.status"),
        2,
        "pugixml fragilec-driver strict run should fail until full strict build parity exists"
    );
    let stdout = fs::read_to_string(log_dir.join("make_test_driver.stderr"))
        .expect("failed to read make_test_driver.stderr");
    assert!(
        stdout.contains("single-source compile-only (-c) invocations")
            || stdout.contains("failed to parse"),
        "fragilec-driver strict failure should report unsupported shape/parsing diagnostics, got:\n{}",
        stdout
    );
}

#[test]
#[ignore = "real-world external project test (derives pugixml make-test command plan)"]
fn test_real_world_pugixml_make_test_command_plan_generation() {
    let log_dir =
        run_pugixml_make_test_command_plan().expect("failed to run pugixml command-plan baseline");

    for rel in PUGIXML_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected command-plan log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        fs::read_to_string(log_dir.join("make_test_dryrun.status"))
            .expect("failed to read make_test_dryrun.status")
            .trim(),
        "0",
        "make -n test should succeed"
    );

    let manifest = fs::read_to_string(log_dir.join("make_test_commands_manifest.txt"))
        .expect("failed to read make_test_commands_manifest.txt");
    assert!(
        manifest.contains("/test"),
        "command manifest should include test binary runtime command, got:\n{}",
        manifest
    );
}

#[test]
#[ignore = "real-world external project test (replays pugixml single TU through fragile)"]
fn test_real_world_pugixml_fragile_single_tu_replay() {
    match run_pugixml_fragile_single_tu_replay() {
        Ok(log_dir) => {
            for rel in PUGIXML_FRAGILE_SINGLE_TU_LOG_FILES {
                assert!(
                    log_dir.join(rel).exists(),
                    "expected replay log file {}",
                    log_dir.join(rel).display()
                );
            }
            assert_eq!(
                read_status_file(&log_dir.join("rustc_fragile_pugixml_single_tu.status"))
                    .expect("failed to read rustc_fragile_pugixml_single_tu.status"),
                0,
                "real-world pugixml single-tu replay should compile when replay is unblocked"
            );
        }
        Err(err) => {
            let known_blockers = [
                "cast cannot be followed by a method call",
                "expected identifier, found keyword `extern`",
                "failed to transpile",
            ];
            assert!(
                known_blockers.iter().any(|sig| err.contains(sig)),
                "unexpected pugixml single-tu replay failure signature:\n{}",
                err
            );
        }
    }
}
