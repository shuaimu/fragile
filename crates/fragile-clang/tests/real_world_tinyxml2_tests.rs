//! Real-world tinyxml2 fixture bootstrap tests.
//!
//! Phase 2 starts with a deterministic, pinned tinyxml2 checkout so all
//! subsequent baseline/parity work runs against a stable upstream snapshot.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TINYXML2_REPO_URL: &str = "https://github.com/leethomason/tinyxml2.git";
const TINYXML2_PINNED_COMMIT: &str = "9148bdf719e997d1f474be6bcc7943881046dba1"; // 11.0.0
const TINYXML2_CACHE_DIR: &str = "/tmp/fragile_real_world_tinyxml2";
const TINYXML2_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_tinyxml2_native_baseline";
const TINYXML2_MAKE_TEST_COMMAND_PLAN_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_make_test_command_plan";
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
