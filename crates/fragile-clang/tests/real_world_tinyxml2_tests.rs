//! Real-world tinyxml2 fixture bootstrap tests.
//!
//! Phase 2 starts with a deterministic, pinned tinyxml2 checkout so all
//! subsequent baseline/parity work runs against a stable upstream snapshot.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TINYXML2_REPO_URL: &str = "https://github.com/leethomason/tinyxml2.git";
const TINYXML2_PINNED_COMMIT: &str = "9148bdf719e997d1f474be6bcc7943881046dba1"; // 11.0.0
const TINYXML2_CACHE_DIR: &str = "/tmp/fragile_real_world_tinyxml2";
const TINYXML2_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_tinyxml2_native_baseline";
const TINYXML2_MAKE_TEST_COMMAND_PLAN_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_make_test_command_plan";
const TINYXML2_MAKE_TEST_REPLAY_NATIVE_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_make_test_replay_native";
const TINYXML2_CXX_DRIVER_XMLTEST_DIR: &str = "/tmp/fragile_real_world_tinyxml2_cxx_driver_xmltest";
const TINYXML2_REQUIRED_PATHS: &[&str] = &[
    "tinyxml2.h",
    "tinyxml2.cpp",
    "xmltest.cpp",
    "CMakeLists.txt",
    "Makefile",
];
const TINYXML2_BASELINE_LOG_FILES: &[&str] = &[
    "make_test.status",
    "make_test.stdout",
    "make_test.stderr",
    "baseline_manifest.txt",
];
const TINYXML2_REQUIRED_TEST_BINARIES: &[&str] = &["xmltest"];
const TINYXML2_MAKE_TEST_COMMAND_PLAN_LOG_FILES: &[&str] = &[
    "make_test_dryrun.status",
    "make_test_dryrun.stdout",
    "make_test_dryrun.stderr",
    "make_test_commands_manifest.txt",
];
const TINYXML2_CXX_DRIVER_LOG_FILES: &[&str] = &[
    "make_clean_driver.status",
    "make_clean_driver.stdout",
    "make_clean_driver.stderr",
    "make_xmltest_driver.status",
    "make_xmltest_driver.stdout",
    "make_xmltest_driver.stderr",
    "cxx_driver.log",
    "cxx_driver_manifest.txt",
    "compile_units_manifest.txt",
];
const TINYXML2_MAKE_TEST_REPLAY_COMMAND_TIMEOUT_SECONDS: u64 = 15;

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
    run_git(&["reset", "--hard", pinned_commit], Some(repo_dir))?;
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

fn ensure_tinyxml2_checkout() -> Result<PathBuf, String> {
    ensure_pinned_checkout(
        TINYXML2_REPO_URL,
        Path::new(TINYXML2_CACHE_DIR),
        TINYXML2_PINNED_COMMIT,
        TINYXML2_REQUIRED_PATHS,
    )
}

fn status_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn write_command_capture(log_dir: &Path, step: &str, output: &Output) -> Result<(), String> {
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

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)
            .map_err(|e| format!("failed to stat {}: {}", path.display(), e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("failed to chmod {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn normalize_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            _ => out.push(comp.as_os_str()),
        }
    }
    out
}

fn normalize_path_for_manifest(raw: &str, cwd: &Path, source_root: &Path) -> String {
    let raw_path = Path::new(raw);
    let absolute = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        cwd.join(raw_path)
    };
    let absolute = lexical_normalize(&absolute);
    let source_root = lexical_normalize(source_root);
    if let Ok(rel) = absolute.strip_prefix(&source_root) {
        normalize_slashes(rel.to_string_lossy().as_ref())
    } else {
        normalize_slashes(absolute.to_string_lossy().as_ref())
    }
}

fn is_c_family_source_token(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    lower.ends_with(".c")
        || lower.ends_with(".cc")
        || lower.ends_with(".cpp")
        || lower.ends_with(".cxx")
        || lower.ends_with(".c++")
}

fn parse_compile_units_from_cxx_driver_log(
    driver_log: &str,
    source_dir: &Path,
) -> Result<Vec<(String, String)>, String> {
    let mut units: BTreeSet<(String, String)> = BTreeSet::new();
    let mut pending_cwd: Option<PathBuf> = None;

    for line in driver_log.lines() {
        if let Some(cwd_raw) = line.strip_prefix("cwd=") {
            pending_cwd = Some(PathBuf::from(cwd_raw.trim()));
            continue;
        }

        let Some(args_raw) = line.strip_prefix("args=") else {
            continue;
        };
        let command_cwd = pending_cwd
            .as_ref()
            .cloned()
            .unwrap_or_else(|| source_dir.to_path_buf());
        let tokens: Vec<&str> = args_raw.split_whitespace().collect();
        if tokens.is_empty() || !tokens.iter().any(|tok| *tok == "-c") {
            continue;
        }

        let mut source_token: Option<&str> = None;
        let mut object_token: Option<&str> = None;
        let mut idx = 0usize;
        while idx < tokens.len() {
            let tok = tokens[idx];
            if tok == "-o" {
                if let Some(next) = tokens.get(idx + 1) {
                    object_token = Some(*next);
                }
                idx += 2;
                continue;
            }
            if let Some(rest) = tok.strip_prefix("-o") {
                if !rest.is_empty() {
                    object_token = Some(rest);
                }
                idx += 1;
                continue;
            }
            if source_token.is_none() && !tok.starts_with('-') && is_c_family_source_token(tok) {
                source_token = Some(tok);
            }
            idx += 1;
        }

        let (source_raw, object_raw) = match (source_token, object_token) {
            (Some(source_raw), Some(object_raw)) => (source_raw, object_raw),
            _ => continue,
        };
        let source_rel = normalize_path_for_manifest(source_raw, &command_cwd, source_dir);
        let object_rel = normalize_path_for_manifest(object_raw, &command_cwd, source_dir);
        units.insert((source_rel, object_rel));
    }

    if units.is_empty() {
        return Err("no compile units found in cxx_driver.log".to_string());
    }
    Ok(units.into_iter().collect())
}

fn write_compile_units_manifest_from_cxx_driver_log(
    log_dir: &Path,
    source_dir: &Path,
) -> Result<usize, String> {
    let driver_log_path = log_dir.join("cxx_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path)
        .map_err(|e| format!("failed to read {}: {}", driver_log_path.display(), e))?;
    let units = parse_compile_units_from_cxx_driver_log(&driver_log, source_dir)?;

    let mut manifest = format!(
        "source_dir={}\ncompile_units_count={}\n",
        source_dir.display(),
        units.len()
    );
    for (source_rel, object_rel) in &units {
        manifest.push_str(&format!("source={} object={}\n", source_rel, object_rel));
    }
    fs::write(log_dir.join("compile_units_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write compile units manifest at {}: {}",
            log_dir.display(),
            e
        )
    })?;
    Ok(units.len())
}

fn create_logging_cxx_driver(driver_dir: &Path, log_path: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(driver_dir).map_err(|e| {
        format!(
            "failed to create CXX driver dir {}: {}",
            driver_dir.display(),
            e
        )
    })?;

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create CXX log dir {}: {}", parent.display(), e))?;
    }
    fs::write(log_path, "").map_err(|e| {
        format!(
            "failed to initialize CXX driver log {}: {}",
            log_path.display(),
            e
        )
    })?;

    let driver_path = driver_dir.join("fragile_tinyxml2_cxx_driver.sh");
    let script = r#"#!/bin/sh
set -eu
log_file="${FRAGILE_TINYXML2_CXX_DRIVER_LOG:-}"
if [ -z "$log_file" ]; then
  echo "FRAGILE_TINYXML2_CXX_DRIVER_LOG is required" 1>&2
  exit 97
fi
{
  printf 'cwd=%s\n' "$(pwd)"
  printf 'args='
  printf '%s ' "$@"
  printf '\n'
} >> "$log_file"
exec c++ "$@"
"#;

    fs::write(&driver_path, script).map_err(|e| {
        format!(
            "failed to write CXX driver script {}: {}",
            driver_path.display(),
            e
        )
    })?;
    make_executable(&driver_path)?;
    Ok(driver_path)
}

fn write_cxx_driver_manifest(
    log_dir: &Path,
    source_dir: &Path,
    make_target: &str,
) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let manifest = format!(
        "source_dir={}\ncommit={}\nmake_target={}\n",
        source_dir.display(),
        head.trim(),
        make_target
    );
    fs::write(log_dir.join("cxx_driver_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write CXX driver manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_cxx_driver_xmltest_baseline_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    if !source_dir.join("Makefile").exists() {
        return Err(format!(
            "CXX-driver baseline source {} is missing Makefile",
            source_dir.display()
        ));
    }

    fs::create_dir_all(log_dir).map_err(|e| {
        format!(
            "failed to create CXX-driver baseline log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;
    let cxx_driver_log = log_dir.join("cxx_driver.log");
    let cxx_driver = create_logging_cxx_driver(log_dir, &cxx_driver_log)?;
    let cxx_driver_str = cxx_driver.to_string_lossy().to_string();
    let cxx_driver_log_str = cxx_driver_log.to_string_lossy().to_string();

    let mut make_clean = Command::new("make");
    make_clean.arg("clean").current_dir(source_dir);
    make_clean
        .env("CXX", cxx_driver_str.as_str())
        .env(
            "FRAGILE_TINYXML2_CXX_DRIVER_LOG",
            cxx_driver_log_str.as_str(),
        )
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let make_clean_output = make_clean.output().map_err(|e| {
        format!(
            "failed to run CXX-driver make clean at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_clean_driver", &make_clean_output)?;
    if !make_clean_output.status.success() {
        return Err(format!(
            "CXX-driver make clean failed with status {} (logs: {})",
            status_code(&make_clean_output),
            log_dir.display()
        ));
    }

    let mut make_xmltest = Command::new("make");
    make_xmltest.arg("xmltest").current_dir(source_dir);
    make_xmltest
        .env("CXX", cxx_driver_str.as_str())
        .env(
            "FRAGILE_TINYXML2_CXX_DRIVER_LOG",
            cxx_driver_log_str.as_str(),
        )
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let make_xmltest_output = make_xmltest.output().map_err(|e| {
        format!(
            "failed to run CXX-driver make xmltest at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_xmltest_driver", &make_xmltest_output)?;
    if !make_xmltest_output.status.success() {
        return Err(format!(
            "CXX-driver make xmltest failed with status {} (logs: {})",
            status_code(&make_xmltest_output),
            log_dir.display()
        ));
    }

    write_cxx_driver_manifest(log_dir, source_dir, "xmltest")?;
    write_compile_units_manifest_from_cxx_driver_log(log_dir, source_dir)?;
    Ok(())
}

fn write_baseline_manifest(log_dir: &Path, source_dir: &Path) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let manifest = format!(
        "source_dir={}\ncommit={}\n",
        source_dir.display(),
        head.trim()
    );
    fs::write(log_dir.join("baseline_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write baseline manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_native_baseline_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    if !source_dir.join("Makefile").exists() {
        return Err(format!(
            "native baseline source {} is missing Makefile",
            source_dir.display()
        ));
    }

    fs::create_dir_all(log_dir).map_err(|e| {
        format!(
            "failed to create baseline log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;

    let mut make_test = Command::new("make");
    make_test.arg("test").current_dir(source_dir);
    make_test
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let make_test_output = make_test.output().map_err(|e| {
        format!(
            "failed to run native baseline make test at {}: {}",
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

    Ok(())
}

fn run_tinyxml2_native_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_tinyxml2_checkout()?;
    let baseline_root = PathBuf::from(TINYXML2_NATIVE_BASELINE_DIR);
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
        &["checkout", "--detach", TINYXML2_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != TINYXML2_PINNED_COMMIT {
        return Err(format!(
            "native baseline worktree expected commit {} but got {}",
            TINYXML2_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("native_logs");
    run_native_baseline_in_tree(&worktree_dir, &log_dir)?;
    write_baseline_manifest(&log_dir, &worktree_dir)?;
    Ok(log_dir)
}

fn normalize_make_command_line(line: &str) -> Option<String> {
    let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn command_invokes_binary(command_line: &str, binary: &str) -> bool {
    let expected = format!("./{}", binary);
    command_line.split_whitespace().any(|token| {
        token.trim_matches(|c: char| matches!(c, '|' | '&' | ';' | '\\' | '(' | ')')) == expected
    })
}

fn parse_make_test_logical_commands(dry_run_stdout: &str) -> Vec<String> {
    let mut logical_commands: Vec<String> = Vec::new();
    let mut current = String::new();

    for raw_line in dry_run_stdout.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let has_trailing_continuation = trimmed.ends_with('\\');
        let line_body = if has_trailing_continuation {
            trimmed[..trimmed.len() - 1].trim_end()
        } else {
            trimmed
        };

        if !line_body.is_empty() {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(line_body);
        }

        if !has_trailing_continuation {
            if let Some(normalized) = normalize_make_command_line(&current) {
                logical_commands.push(normalized);
            }
            current.clear();
        }
    }

    if !current.is_empty() {
        if let Some(normalized) = normalize_make_command_line(&current) {
            logical_commands.push(normalized);
        }
    }

    logical_commands
}

fn parse_make_test_commands_from_dry_run(
    dry_run_stdout: &str,
    required_binaries: &[&str],
) -> Result<Vec<String>, String> {
    let mut commands: Vec<String> = Vec::new();
    let mut seen_commands: BTreeSet<String> = BTreeSet::new();
    let mut covered_binaries: BTreeSet<String> = BTreeSet::new();

    for normalized in parse_make_test_logical_commands(dry_run_stdout) {
        let mut command_is_relevant = false;
        for binary in required_binaries {
            if command_invokes_binary(&normalized, binary) {
                covered_binaries.insert((*binary).to_string());
                command_is_relevant = true;
            }
        }
        if !command_is_relevant {
            continue;
        }
        if seen_commands.insert(normalized.clone()) {
            commands.push(normalized);
        }
    }

    if commands.is_empty() {
        return Err("no make test runtime commands found for required binaries".to_string());
    }

    let mut missing: Vec<String> = Vec::new();
    for binary in required_binaries {
        if !covered_binaries.contains(*binary) {
            missing.push((*binary).to_string());
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "make test command plan missing required binary invocations: {}",
            missing.join(", ")
        ));
    }

    Ok(commands)
}

fn run_make_test_dry_run_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(log_dir).map_err(|e| {
        format!(
            "failed to create make-test command-plan log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;

    let mut make_dryrun = Command::new("make");
    make_dryrun.arg("-n").arg("test").current_dir(source_dir);
    make_dryrun
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let dry_run_output = make_dryrun.output().map_err(|e| {
        format!(
            "failed to run make -n test at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_test_dryrun", &dry_run_output)?;
    if !dry_run_output.status.success() {
        return Err(format!(
            "make -n test failed with status {} (logs: {})",
            status_code(&dry_run_output),
            log_dir.display()
        ));
    }
    Ok(())
}

fn write_make_test_commands_manifest(
    log_dir: &Path,
    source_dir: &Path,
    required_binaries: &[&str],
) -> Result<usize, String> {
    let dry_run_path = log_dir.join("make_test_dryrun.stdout");
    let dry_run_stdout = fs::read_to_string(&dry_run_path)
        .map_err(|e| format!("failed to read {}: {}", dry_run_path.display(), e))?;
    let commands = parse_make_test_commands_from_dry_run(&dry_run_stdout, required_binaries)?;

    let mut manifest = format!(
        "source_dir={}\nmake_test_command_count={}\nrequired_binaries={}\n",
        source_dir.display(),
        commands.len(),
        required_binaries.join(","),
    );
    for command in &commands {
        manifest.push_str(&format!("command={}\n", command));
    }
    fs::write(log_dir.join("make_test_commands_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write make-test command manifest at {}: {}",
            log_dir.display(),
            e
        )
    })?;
    Ok(commands.len())
}

fn run_make_test_command_plan_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    run_make_test_dry_run_in_tree(source_dir, log_dir)?;
    write_make_test_commands_manifest(log_dir, source_dir, TINYXML2_REQUIRED_TEST_BINARIES)?;
    Ok(())
}

fn parse_make_test_commands_manifest_entries(manifest_text: &str) -> Result<Vec<String>, String> {
    let mut commands: Vec<String> = Vec::new();
    for (line_no, line) in manifest_text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("command=") else {
            continue;
        };
        let command = rest.trim();
        if command.is_empty() {
            return Err(format!(
                "invalid empty make-test command in manifest at line {}",
                line_no + 1
            ));
        }
        commands.push(command.to_string());
    }

    if commands.is_empty() {
        return Err("make-test command manifest has no command entries".to_string());
    }
    Ok(commands)
}

fn make_test_replay_step_name(idx: usize) -> String {
    format!("make_test_replay_{:02}", idx + 1)
}

fn run_make_test_replay_command_with_timeout_in_tree(
    source_dir: &Path,
    command_line: &str,
    timeout_seconds: u64,
) -> Result<Output, String> {
    let mut cmd = Command::new("timeout");
    cmd.arg(format!("{}s", timeout_seconds))
        .arg("sh")
        .arg("-c")
        .arg(command_line)
        .current_dir(source_dir);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            format!(
                "failed to run make-test replay command with timeout: `timeout` command is unavailable at {}",
                source_dir.display()
            )
        } else {
            format!(
                "failed to run make-test replay command with timeout at {}: {}",
                source_dir.display(),
                e
            )
        }
    })
}

fn replay_make_test_commands_from_manifest_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<usize, String> {
    let manifest_path = log_dir.join("make_test_commands_manifest.txt");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {}", manifest_path.display(), e))?;
    let commands = parse_make_test_commands_manifest_entries(&manifest)?;

    for (idx, command_line) in commands.iter().enumerate() {
        let output = run_make_test_replay_command_with_timeout_in_tree(
            source_dir,
            command_line,
            TINYXML2_MAKE_TEST_REPLAY_COMMAND_TIMEOUT_SECONDS,
        )?;
        let step = make_test_replay_step_name(idx);
        write_command_capture(log_dir, &step, &output)?;
        if !output.status.success() {
            let status = status_code(&output);
            if status == 124 {
                return Err(format!(
                    "make-test command replay timed out at command {} after {}s: {} (logs: {})",
                    idx + 1,
                    TINYXML2_MAKE_TEST_REPLAY_COMMAND_TIMEOUT_SECONDS,
                    command_line,
                    log_dir.display()
                ));
            }
            return Err(format!(
                "make-test command replay failed at command {} with status {}: {} (logs: {})",
                idx + 1,
                status,
                command_line,
                log_dir.display()
            ));
        }
    }

    let mut replay_manifest = format!(
        "source_dir={}\ncommand_replay_count={}\n",
        source_dir.display(),
        commands.len()
    );
    for (idx, command_line) in commands.iter().enumerate() {
        replay_manifest.push_str(&format!(
            "replay_step={} command={}\n",
            make_test_replay_step_name(idx),
            command_line
        ));
    }
    fs::write(
        log_dir.join("make_test_replay_manifest.txt"),
        replay_manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write make-test replay manifest at {}: {}",
            log_dir.display(),
            e
        )
    })?;

    Ok(commands.len())
}

fn run_make_test_command_replay_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    run_make_test_command_plan_in_tree(source_dir, log_dir)?;
    replay_make_test_commands_from_manifest_in_tree(source_dir, log_dir)?;
    Ok(())
}

fn run_make_xmltest_build_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    step_name: &str,
) -> Result<(), String> {
    fs::create_dir_all(log_dir).map_err(|e| {
        format!(
            "failed to create replay log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;

    let mut make_xmltest = Command::new("make");
    make_xmltest.arg("xmltest").current_dir(source_dir);
    make_xmltest
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let make_xmltest_output = make_xmltest.output().map_err(|e| {
        format!(
            "failed to run make xmltest at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, step_name, &make_xmltest_output)?;
    if !make_xmltest_output.status.success() {
        return Err(format!(
            "make xmltest failed with status {} (logs: {})",
            status_code(&make_xmltest_output),
            log_dir.display()
        ));
    }
    Ok(())
}

fn run_tinyxml2_native_make_test_command_replay_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    run_make_xmltest_build_in_tree(source_dir, log_dir, "make_xmltest_native")?;
    run_make_test_command_replay_in_tree(source_dir, log_dir)?;
    Ok(())
}

fn run_tinyxml2_fragile_make_test_command_replay_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    run_make_test_command_replay_in_tree(source_dir, log_dir)
}

fn run_tinyxml2_make_test_command_plan() -> Result<PathBuf, String> {
    let checkout_dir = ensure_tinyxml2_checkout()?;
    let baseline_root = PathBuf::from(TINYXML2_MAKE_TEST_COMMAND_PLAN_DIR);
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
        &["checkout", "--detach", TINYXML2_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != TINYXML2_PINNED_COMMIT {
        return Err(format!(
            "make-test command-plan worktree expected commit {} but got {}",
            TINYXML2_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("command_plan_logs");
    run_make_test_command_plan_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_tinyxml2_make_test_command_replay_native() -> Result<PathBuf, String> {
    let checkout_dir = ensure_tinyxml2_checkout()?;
    let baseline_root = PathBuf::from(TINYXML2_MAKE_TEST_REPLAY_NATIVE_DIR);
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
        &["checkout", "--detach", TINYXML2_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != TINYXML2_PINNED_COMMIT {
        return Err(format!(
            "make-test replay native worktree expected commit {} but got {}",
            TINYXML2_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("replay_logs");
    run_tinyxml2_native_make_test_command_replay_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_tinyxml2_cxx_driver_xmltest_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_tinyxml2_checkout()?;
    let baseline_root = PathBuf::from(TINYXML2_CXX_DRIVER_XMLTEST_DIR);
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
        &["checkout", "--detach", TINYXML2_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != TINYXML2_PINNED_COMMIT {
        return Err(format!(
            "CXX-driver xmltest worktree expected commit {} but got {}",
            TINYXML2_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_cxx_driver_xmltest_baseline_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn read_status_file(path: &Path) -> Result<i32, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read status file {}: {}", path.display(), e))?;
    raw.trim().parse::<i32>().map_err(|e| {
        format!(
            "failed to parse status code in {} (value: {:?}): {}",
            path.display(),
            raw.trim(),
            e
        )
    })
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fragile_{}_{}_{}",
        prefix,
        std::process::id(),
        nanos
    ))
}

fn create_local_tinyxml2_like_repo(base_dir: &Path) -> Result<(String, String, String), String> {
    let remote_dir = base_dir.join("remote_repo");
    fs::create_dir_all(&remote_dir)
        .map_err(|e| format!("failed to create {}: {}", remote_dir.display(), e))?;

    run_git(&["init"], Some(&remote_dir))?;
    run_git(
        &["config", "user.email", "fragile-tests@example.invalid"],
        Some(&remote_dir),
    )?;
    run_git(&["config", "user.name", "Fragile Tests"], Some(&remote_dir))?;

    fs::write(
        remote_dir.join("tinyxml2.h"),
        "/* tinyxml2 fixture header */\n",
    )
    .map_err(|e| format!("failed to write tinyxml2.h: {}", e))?;
    fs::write(
        remote_dir.join("tinyxml2.cpp"),
        "int tinyxml2_fixture_version(void) { return 1; }\n",
    )
    .map_err(|e| format!("failed to write tinyxml2.cpp: {}", e))?;
    fs::write(
        remote_dir.join("xmltest.cpp"),
        "int main(void) { return tinyxml2_fixture_version() == 1 ? 0 : 1; }\n",
    )
    .map_err(|e| format!("failed to write xmltest.cpp: {}", e))?;
    fs::write(
        remote_dir.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.10)\n",
    )
    .map_err(|e| format!("failed to write CMakeLists.txt: {}", e))?;
    fs::write(
        remote_dir.join("Makefile"),
        "test: xmltest\n\t@./xmltest\n\nxmltest:\n\t@printf '%s\\n' '#!/bin/sh' 'echo \"xmltest fixture: Pass 1, Fail 0\"' > xmltest\n\t@chmod +x xmltest\n",
    )
    .map_err(|e| format!("failed to write Makefile: {}", e))?;
    run_git(
        &[
            "add",
            "tinyxml2.h",
            "tinyxml2.cpp",
            "xmltest.cpp",
            "CMakeLists.txt",
            "Makefile",
        ],
        Some(&remote_dir),
    )?;
    run_git(&["commit", "-m", "first"], Some(&remote_dir))?;
    let pinned_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;

    fs::write(
        remote_dir.join("tinyxml2.cpp"),
        "int tinyxml2_fixture_version(void) { return 2; }\n",
    )
    .map_err(|e| format!("failed to update tinyxml2.cpp: {}", e))?;
    run_git(&["add", "tinyxml2.cpp"], Some(&remote_dir))?;
    run_git(&["commit", "-m", "second"], Some(&remote_dir))?;
    let newer_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;

    Ok((
        format!("file://{}", remote_dir.display()),
        pinned_commit,
        newer_commit,
    ))
}

fn create_local_tinyxml2_cxx_driver_project(base_dir: &Path) -> Result<PathBuf, String> {
    let project_dir = base_dir.join("tinyxml2_cxx_driver_project");
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;

    fs::write(
        project_dir.join("tinyxml2.h"),
        "#pragma once\nint tinyxml2_fixture_value(void);\n",
    )
    .map_err(|e| format!("failed to write tinyxml2.h: {}", e))?;
    fs::write(
        project_dir.join("tinyxml2.cpp"),
        "#include \"tinyxml2.h\"\nint tinyxml2_fixture_value(void) { return 7; }\n",
    )
    .map_err(|e| format!("failed to write tinyxml2.cpp: {}", e))?;
    fs::write(
        project_dir.join("xmltest.cpp"),
        "#include \"tinyxml2.h\"\nint main(void) { return tinyxml2_fixture_value() == 7 ? 0 : 1; }\n",
    )
    .map_err(|e| format!("failed to write xmltest.cpp: {}", e))?;
    fs::write(
        project_dir.join("Makefile"),
        "\
CXX ?= c++\n\
CXXFLAGS ?= -std=c++11 -O2\n\
\n\
xmltest: xmltest.o tinyxml2.o\n\
\t$(CXX) $(CXXFLAGS) xmltest.o tinyxml2.o -o $@\n\
\n\
xmltest.o: xmltest.cpp tinyxml2.h\n\
\t$(CXX) $(CXXFLAGS) -c xmltest.cpp -o $@\n\
\n\
tinyxml2.o: tinyxml2.cpp tinyxml2.h\n\
\t$(CXX) $(CXXFLAGS) -c tinyxml2.cpp -o $@\n\
\n\
test: xmltest\n\
\t./xmltest\n\
\n\
clean:\n\
\t$(RM) xmltest xmltest.o tinyxml2.o\n",
    )
    .map_err(|e| format!("failed to write Makefile: {}", e))?;

    Ok(project_dir)
}

fn assert_make_test_replay_artifacts_exist(log_dir: &Path) -> Result<usize, String> {
    let manifest =
        fs::read_to_string(log_dir.join("make_test_commands_manifest.txt")).map_err(|e| {
            format!(
                "failed to read make-test command manifest in {}: {}",
                log_dir.display(),
                e
            )
        })?;
    let commands = parse_make_test_commands_manifest_entries(&manifest)?;
    for (idx, _) in commands.iter().enumerate() {
        let step = make_test_replay_step_name(idx);
        for suffix in ["status", "stdout", "stderr"] {
            let path = log_dir.join(format!("{}.{}", step, suffix));
            if !path.exists() {
                return Err(format!("missing replay artifact {}", path.display()));
            }
        }
    }

    let replay_manifest = log_dir.join("make_test_replay_manifest.txt");
    if !replay_manifest.exists() {
        return Err(format!(
            "missing replay manifest {}",
            replay_manifest.display()
        ));
    }

    Ok(commands.len())
}

#[test]
fn test_ensure_pinned_checkout_clones_and_pins_local_tinyxml2_fixture() {
    let root = unique_temp_dir("tinyxml2_checkout_pin");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    let prepared = ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("ensure_pinned_checkout should clone and pin the repo");

    assert_eq!(prepared, checkout_dir);
    for rel in TINYXML2_REQUIRED_PATHS {
        assert!(
            prepared.join(rel).exists(),
            "expected required file {}",
            prepared.join(rel).display()
        );
    }
    let head = read_head(&prepared).expect("failed to read checkout HEAD");
    assert_eq!(head, pinned_commit, "checkout should be pinned to commit");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_ensure_pinned_checkout_rewinds_checkout_to_pinned_commit() {
    let root = unique_temp_dir("tinyxml2_checkout_rewind");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("initial checkout should succeed");
    run_git(
        &["checkout", "--detach", newer_commit.as_str()],
        Some(&checkout_dir),
    )
    .expect("failed to move checkout to newer commit");
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(newer_commit.as_str()),
        "test setup should move checkout away from pinned commit"
    );

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be rewound to pinned commit");
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(pinned_commit.as_str()),
        "checkout should be reset to pinned commit"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_ensure_pinned_checkout_restores_missing_required_file() {
    let root = unique_temp_dir("tinyxml2_checkout_restore_required_file");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("initial checkout should succeed");

    let removed = checkout_dir.join("xmltest.cpp");
    fs::remove_file(&removed).expect("failed to remove required file for test setup");
    assert!(
        !removed.exists(),
        "test setup should remove the required file"
    );

    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should repair missing required paths");
    assert!(removed.exists(), "missing required file should be restored");
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(pinned_commit.as_str()),
        "checkout should remain pinned after repairing missing files"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_run_native_baseline_in_tree_local_fixture_success() {
    let root = unique_temp_dir("tinyxml2_native_baseline_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&checkout_dir, &log_dir)
        .expect("native baseline should pass for local fixture");
    write_baseline_manifest(&log_dir, &checkout_dir)
        .expect("baseline manifest should be written for local fixture");

    for rel in TINYXML2_BASELINE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected baseline log file {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("make_test.status")).expect("failed to read make status"),
        0,
        "local fixture baseline should report success status"
    );
    let make_stdout = fs::read_to_string(log_dir.join("make_test.stdout"))
        .expect("failed to read make_test stdout");
    assert!(
        make_stdout.contains("Pass 1, Fail 0"),
        "fixture make_test stdout should include success marker, got:\n{}",
        make_stdout
    );
    let baseline_manifest = fs::read_to_string(log_dir.join("baseline_manifest.txt"))
        .expect("failed to read baseline manifest");
    assert!(
        baseline_manifest.contains(&format!("commit={}", pinned_commit)),
        "baseline manifest should record pinned commit {}:\n{}",
        pinned_commit,
        baseline_manifest
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_run_native_baseline_in_tree_reports_make_test_failure() {
    let root = unique_temp_dir("tinyxml2_native_baseline_failure");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    fs::write(
        checkout_dir.join("Makefile"),
        "test:\n\t@echo \"fixture make test failure\" >&2\n\t@exit 7\n",
    )
    .expect("failed to override Makefile with failing test target");

    let log_dir = root.join("native_logs");
    let err = run_native_baseline_in_tree(&checkout_dir, &log_dir)
        .expect_err("native baseline should fail when make test exits non-zero");
    assert!(
        err.contains("native baseline make test failed with status"),
        "failure should report make-test failure status, got: {}",
        err
    );
    let make_status =
        read_status_file(&log_dir.join("make_test.status")).expect("failed to read make status");
    assert_ne!(
        make_status, 0,
        "failure status should be non-zero in make_test.status"
    );
    assert_eq!(
        err,
        format!(
            "native baseline make test failed with status {} (logs: {})",
            make_status,
            log_dir.display()
        ),
        "reported failure status should match captured make_test.status"
    );
    let make_stderr = fs::read_to_string(log_dir.join("make_test.stderr"))
        .expect("failed to read make_test stderr");
    assert!(
        make_stderr.contains("fixture make test failure"),
        "stderr capture should contain fixture failure text, got:\n{}",
        make_stderr
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_make_test_commands_from_dry_run_normalizes_and_validates_coverage() {
    let dry_run_stdout = r#"
        cc -c tinyxml2.cpp -o tinyxml2.o
        ./xmltest \
          --gtest_filter=Smoke
        ./xmltest --gtest_filter=Smoke
    "#;

    let commands =
        parse_make_test_commands_from_dry_run(dry_run_stdout, TINYXML2_REQUIRED_TEST_BINARIES)
            .expect("dry-run parser should capture required runtime commands");
    assert_eq!(
        commands,
        vec!["./xmltest --gtest_filter=Smoke".to_string()],
        "logical command parser should normalize continuations and deduplicate"
    );
}

#[test]
fn test_parse_make_test_commands_from_dry_run_reports_missing_required_binary_invocations() {
    let dry_run_stdout = r#"
        echo "building tinyxml2"
        echo "test target has no runtime invocation"
    "#;

    let err =
        parse_make_test_commands_from_dry_run(dry_run_stdout, TINYXML2_REQUIRED_TEST_BINARIES)
            .expect_err("parser should fail when required binary invocation is absent");
    assert!(
        err.contains("no make test runtime commands found")
            || err.contains("missing required binary invocations"),
        "missing-coverage error should be explicit, got: {}",
        err
    );
}

#[test]
fn test_parse_compile_units_from_cxx_driver_log_normalizes_and_deduplicates() {
    let source_dir = Path::new("/tmp/tinyxml2_driver_parse");
    let driver_log = "\
cwd=/tmp/tinyxml2_driver_parse\n\
args=-std=c++11 -O2 -c tinyxml2.cpp -o tinyxml2.o \n\
cwd=/tmp/tinyxml2_driver_parse\n\
args=-std=c++11 -O2 -c ./xmltest.cpp -o ./xmltest.o \n\
cwd=/tmp/tinyxml2_driver_parse\n\
args=-std=c++11 -O2 -c tinyxml2.cpp -o tinyxml2.o \n";

    let units = parse_compile_units_from_cxx_driver_log(driver_log, source_dir)
        .expect("CXX-driver parse should capture compile units");
    assert_eq!(
        units,
        vec![
            ("tinyxml2.cpp".to_string(), "tinyxml2.o".to_string()),
            ("xmltest.cpp".to_string(), "xmltest.o".to_string())
        ],
        "compile-unit parser should normalize paths and deduplicate repeated entries"
    );
}

#[test]
fn test_parse_compile_units_from_cxx_driver_log_reports_missing_units() {
    let source_dir = Path::new("/tmp/tinyxml2_driver_parse_empty");
    let driver_log = "\
cwd=/tmp/tinyxml2_driver_parse_empty\n\
args=-std=c++11 xmltest.cpp tinyxml2.o -o xmltest \n";

    let err = parse_compile_units_from_cxx_driver_log(driver_log, source_dir)
        .expect_err("parser should fail when CXX driver log has no compile units");
    assert!(
        err.contains("no compile units found in cxx_driver.log"),
        "missing-unit error should be explicit, got: {}",
        err
    );
}

#[test]
fn test_cxx_driver_xmltest_baseline_local_fixture_success() {
    let root = unique_temp_dir("tinyxml2_cxx_driver_xmltest_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_tinyxml2_cxx_driver_project(&root)
        .expect("failed to create local tinyxml2 CXX-driver project");
    let log_dir = root.join("cxx_driver_logs");
    run_cxx_driver_xmltest_baseline_in_tree(&project_dir, &log_dir)
        .expect("CXX-driver baseline should succeed for local fixture");

    for rel in TINYXML2_CXX_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected CXX-driver log artifact {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_driver.status"))
            .expect("failed to read make_xmltest_driver.status"),
        0,
        "make xmltest should succeed for local CXX-driver fixture"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_clean_driver.status"))
            .expect("failed to read make_clean_driver.status"),
        0,
        "make clean should succeed for local CXX-driver fixture"
    );

    let compile_manifest = fs::read_to_string(log_dir.join("compile_units_manifest.txt"))
        .expect("failed to read compile_units_manifest.txt");
    assert!(
        compile_manifest.contains("source=tinyxml2.cpp object=tinyxml2.o"),
        "compile manifest should include tinyxml2 compile unit, got:\n{}",
        compile_manifest
    );
    assert!(
        compile_manifest.contains("source=xmltest.cpp object=xmltest.o"),
        "compile manifest should include xmltest compile unit, got:\n{}",
        compile_manifest
    );

    let xmltest_status = Command::new("./xmltest")
        .current_dir(&project_dir)
        .output()
        .expect("failed to execute local xmltest binary")
        .status;
    assert!(
        xmltest_status.success(),
        "local xmltest binary built via CXX-driver baseline should succeed"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_cxx_driver_xmltest_baseline_reports_missing_compile_coverage() {
    let root = unique_temp_dir("tinyxml2_cxx_driver_xmltest_missing_coverage");
    fs::create_dir_all(&root).expect("failed to create test root");
    let project_dir = root.join("tinyxml2_no_compile_project");
    fs::create_dir_all(&project_dir).expect("failed to create local project dir");
    fs::write(
        project_dir.join("Makefile"),
        "\
xmltest:\n\
\t@printf '%s\\n' '#!/bin/sh' 'exit 0' > xmltest\n\
\t@chmod +x xmltest\n\
\n\
clean:\n\
\t@rm -f xmltest\n",
    )
    .expect("failed to write no-compile Makefile");

    let log_dir = root.join("cxx_driver_logs");
    let err = run_cxx_driver_xmltest_baseline_in_tree(&project_dir, &log_dir)
        .expect_err("CXX-driver baseline should fail without compile coverage");
    assert!(
        err.contains("no compile units found in cxx_driver.log"),
        "missing compile-coverage error should be explicit, got: {}",
        err
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_driver.status"))
            .expect("failed to read make_xmltest_driver.status"),
        0,
        "fixture xmltest target should still execute before compile coverage validation fails"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_plan_local_fixture_success() {
    let root = unique_temp_dir("tinyxml2_make_test_command_plan_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("command_plan_logs");
    run_make_test_command_plan_in_tree(&checkout_dir, &log_dir)
        .expect("make-test command plan generation should succeed");

    for rel in TINYXML2_MAKE_TEST_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected command-plan log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        read_status_file(&log_dir.join("make_test_dryrun.status"))
            .expect("failed to read make_test_dryrun.status"),
        0,
        "make -n test should succeed for local fixture"
    );

    let manifest = fs::read_to_string(log_dir.join("make_test_commands_manifest.txt"))
        .expect("failed to read make_test_commands_manifest.txt");
    let commands = parse_make_test_commands_manifest_entries(&manifest)
        .expect("failed to parse command manifest entries");
    assert!(
        commands
            .iter()
            .any(|cmd| command_invokes_binary(cmd, TINYXML2_REQUIRED_TEST_BINARIES[0])),
        "manifest should contain xmltest runtime invocation, got: {:?}",
        commands
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_plan_local_fixture_detects_missing_coverage() {
    let root = unique_temp_dir("tinyxml2_make_test_command_plan_missing_coverage");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    fs::write(
        checkout_dir.join("Makefile"),
        "test:\n\t@echo \"tinyxml2 fixture has no runtime binary invocation\"\n",
    )
    .expect("failed to write no-runtime-invocation fixture Makefile");

    let log_dir = root.join("command_plan_logs");
    let err = run_make_test_command_plan_in_tree(&checkout_dir, &log_dir)
        .expect_err("command-plan generation should fail without xmltest invocation");
    assert!(
        err.contains("no make test runtime commands found")
            || err.contains("missing required binary invocations"),
        "missing coverage error should explain required invocation gap, got: {}",
        err
    );
    assert!(
        log_dir.join("make_test_dryrun.status").exists(),
        "dry-run status should still be captured before coverage validation failure"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_replay_local_fixture_native_runner_success() {
    let root = unique_temp_dir("tinyxml2_make_test_replay_native_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("replay_logs_native");
    run_tinyxml2_native_make_test_command_replay_in_tree(&checkout_dir, &log_dir)
        .expect("native replay runner should succeed for local fixture");

    for rel in TINYXML2_MAKE_TEST_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected replay prerequisite file {}",
            log_dir.join(rel).display()
        );
    }
    assert!(
        log_dir.join("make_xmltest_native.status").exists(),
        "native runner should capture build status"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_native.status"))
            .expect("failed to read make_xmltest_native.status"),
        0,
        "native runner should build xmltest successfully before replay"
    );

    let replay_count = assert_make_test_replay_artifacts_exist(&log_dir)
        .expect("expected replay artifacts to be captured");
    assert_eq!(
        replay_count, 1,
        "local tinyxml2 fixture should replay exactly one runtime command"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_test_replay_01.status"))
            .expect("failed to read replay status"),
        0,
        "native replay command should succeed"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_replay_local_fixture_reports_failing_command() {
    let root = unique_temp_dir("tinyxml2_make_test_replay_native_failure");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    fs::write(
        checkout_dir.join("Makefile"),
        "test: xmltest\n\t@./xmltest\n\nxmltest:\n\t@printf '%s\\n' '#!/bin/sh' 'echo \"fixture replay failure\" >&2' 'exit 9' > xmltest\n\t@chmod +x xmltest\n",
    )
    .expect("failed to write failing replay fixture Makefile");

    let log_dir = root.join("replay_logs_native");
    let err = run_tinyxml2_native_make_test_command_replay_in_tree(&checkout_dir, &log_dir)
        .expect_err("native replay runner should fail when runtime command exits non-zero");
    assert!(
        err.contains("make-test command replay failed at command 1 with status 9"),
        "failure should report replay command index and status, got: {}",
        err
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_native.status"))
            .expect("failed to read native build status"),
        0,
        "fixture should still build xmltest script successfully"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_test_replay_01.status"))
            .expect("failed to read replay failure status"),
        9,
        "replay status should capture failing command exit code"
    );
    let replay_stderr = fs::read_to_string(log_dir.join("make_test_replay_01.stderr"))
        .expect("failed to read replay stderr");
    assert!(
        replay_stderr.contains("fixture replay failure"),
        "captured stderr should contain fixture failure message, got:\n{}",
        replay_stderr
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_replay_local_fixture_fragile_runner_success_with_prebuilt_binary() {
    let root = unique_temp_dir("tinyxml2_make_test_replay_fragile_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    run_make_xmltest_build_in_tree(&checkout_dir, &root.join("prep_logs"), "make_xmltest_prep")
        .expect("fixture setup should prebuild xmltest for fragile-runner replay");

    let log_dir = root.join("replay_logs_fragile");
    run_tinyxml2_fragile_make_test_command_replay_in_tree(&checkout_dir, &log_dir)
        .expect("fragile replay runner should succeed when binary is prebuilt");

    let replay_count = assert_make_test_replay_artifacts_exist(&log_dir)
        .expect("expected replay artifacts to be captured");
    assert_eq!(
        replay_count, 1,
        "local tinyxml2 fixture should replay one runtime command"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_test_replay_01.status"))
            .expect("failed to read replay status"),
        0,
        "fragile replay command should succeed for prebuilt fixture"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_replay_local_fixture_fragile_runner_reports_missing_binary() {
    let root = unique_temp_dir("tinyxml2_make_test_replay_fragile_missing_binary");
    fs::create_dir_all(&root).expect("failed to create test root");

    let (repo_url, pinned_commit, _newer_commit) =
        create_local_tinyxml2_like_repo(&root).expect("failed to create local tinyxml2-like repo");
    let checkout_dir = root.join("checkout");
    ensure_pinned_checkout(
        repo_url.as_str(),
        &checkout_dir,
        pinned_commit.as_str(),
        TINYXML2_REQUIRED_PATHS,
    )
    .expect("checkout should be prepared");

    let log_dir = root.join("replay_logs_fragile");
    let err = run_tinyxml2_fragile_make_test_command_replay_in_tree(&checkout_dir, &log_dir)
        .expect_err("fragile replay runner should fail when xmltest is not prebuilt");
    assert!(
        err.contains("make-test command replay failed at command 1 with status 127"),
        "missing binary should fail at command replay with status 127, got: {}",
        err
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_test_replay_01.status"))
            .expect("failed to read replay status"),
        127,
        "replay status should record missing-binary failure"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore = "real-world external project test (downloads tinyxml2 fixture)"]
fn test_real_world_tinyxml2_fixture_checkout_is_pinned() {
    let repo_dir = ensure_tinyxml2_checkout().expect("failed to prepare tinyxml2 checkout");
    for rel in TINYXML2_REQUIRED_PATHS {
        assert!(
            repo_dir.join(rel).exists(),
            "expected required file {}",
            repo_dir.join(rel).display()
        );
    }

    let head = read_head(&repo_dir).expect("failed to query tinyxml2 checkout HEAD");
    assert_eq!(
        head, TINYXML2_PINNED_COMMIT,
        "tinyxml2 checkout must stay pinned for deterministic parity runs"
    );
}

#[test]
#[ignore = "real-world external project test (builds tinyxml2 baseline with make test)"]
fn test_real_world_tinyxml2_native_baseline_make_test() {
    let log_dir = run_tinyxml2_native_baseline().expect("failed to run tinyxml2 native baseline");
    for rel in TINYXML2_BASELINE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected baseline log file {}",
            log_dir.join(rel).display()
        );
    }

    let make_status =
        read_status_file(&log_dir.join("make_test.status")).expect("failed to read make status");
    assert_eq!(
        make_status, 0,
        "tinyxml2 native baseline make test should succeed"
    );
    let make_stdout = fs::read_to_string(log_dir.join("make_test.stdout"))
        .expect("failed to read make_test stdout");
    assert!(
        make_stdout.contains("Fail 0"),
        "tinyxml2 make_test stdout should report zero failing checks, got:\n{}",
        make_stdout
    );

    let manifest = fs::read_to_string(log_dir.join("baseline_manifest.txt"))
        .expect("failed to read baseline manifest");
    assert!(
        manifest.contains(&format!("commit={}", TINYXML2_PINNED_COMMIT)),
        "baseline manifest should pin tinyxml2 commit {}:\n{}",
        TINYXML2_PINNED_COMMIT,
        manifest
    );
}

#[test]
#[ignore = "real-world external project test (derives tinyxml2 make-test command plan)"]
fn test_real_world_tinyxml2_make_test_command_plan_generation() {
    let log_dir =
        run_tinyxml2_make_test_command_plan().expect("failed to run tinyxml2 make-test plan run");
    for rel in TINYXML2_MAKE_TEST_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected command-plan log file {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        read_status_file(&log_dir.join("make_test_dryrun.status"))
            .expect("failed to read make_test_dryrun.status"),
        0,
        "tinyxml2 make -n test should succeed for command-plan generation"
    );

    let manifest = fs::read_to_string(log_dir.join("make_test_commands_manifest.txt"))
        .expect("failed to read make-test command manifest");
    let commands = parse_make_test_commands_manifest_entries(&manifest)
        .expect("failed to parse make-test command manifest entries");
    assert!(
        commands
            .iter()
            .any(|cmd| command_invokes_binary(cmd, TINYXML2_REQUIRED_TEST_BINARIES[0])),
        "manifest should contain xmltest runtime command, got: {:?}",
        commands
    );
}

#[test]
#[ignore = "real-world external project test (captures tinyxml2 CXX-driver compile units for make xmltest)"]
fn test_real_world_tinyxml2_cxx_driver_xmltest_compile_manifest() {
    let log_dir = run_tinyxml2_cxx_driver_xmltest_baseline()
        .expect("failed to run tinyxml2 CXX-driver xmltest baseline");
    for rel in TINYXML2_CXX_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected CXX-driver artifact {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_driver.status"))
            .expect("failed to read make_xmltest_driver.status"),
        0,
        "tinyxml2 CXX-driver make xmltest should succeed"
    );

    let compile_manifest = fs::read_to_string(log_dir.join("compile_units_manifest.txt"))
        .expect("failed to read compile_units_manifest.txt");
    assert!(
        compile_manifest.contains("source=tinyxml2.cpp object=tinyxml2.o"),
        "real-world compile manifest should include tinyxml2 compile unit, got:\n{}",
        compile_manifest
    );
}

#[test]
#[ignore = "real-world external project test (replays tinyxml2 make-test command subset in native flow)"]
fn test_real_world_tinyxml2_make_test_command_subset_replay_native() {
    let log_dir = run_tinyxml2_make_test_command_replay_native()
        .expect("failed to run tinyxml2 native make-test replay run");
    for rel in TINYXML2_MAKE_TEST_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected replay prerequisite file {}",
            log_dir.join(rel).display()
        );
    }
    assert!(
        log_dir.join("make_xmltest_native.status").exists(),
        "native replay should capture make_xmltest_native status"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_native.status"))
            .expect("failed to read make_xmltest_native.status"),
        0,
        "native replay should build xmltest before command replay"
    );
    let replay_count =
        assert_make_test_replay_artifacts_exist(&log_dir).expect("missing replay artifacts");
    assert!(
        replay_count >= 1,
        "native replay should capture at least one runtime command replay"
    );
    for idx in 0..replay_count {
        let step = make_test_replay_step_name(idx);
        assert_eq!(
            read_status_file(&log_dir.join(format!("{}.status", step)))
                .expect("failed to read replay status"),
            0,
            "replay step {} should succeed",
            step
        );
    }
}
