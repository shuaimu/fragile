//! Real-world tinyxml2 fixture bootstrap tests.
//!
//! Phase 2 starts with a deterministic, pinned tinyxml2 checkout so all
//! subsequent baseline/parity work runs against a stable upstream snapshot.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TINYXML2_REPO_URL: &str = "https://github.com/leethomason/tinyxml2.git";
const TINYXML2_PINNED_COMMIT: &str = "9148bdf719e997d1f474be6bcc7943881046dba1"; // 11.0.0
const TINYXML2_CACHE_DIR: &str = "/tmp/fragile_real_world_tinyxml2";
const TINYXML2_REQUIRED_PATHS: &[&str] =
    &["tinyxml2.h", "tinyxml2.cpp", "xmltest.cpp", "CMakeLists.txt"];

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

    fs::write(remote_dir.join("tinyxml2.h"), "/* tinyxml2 fixture header */\n")
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
    fs::write(remote_dir.join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.10)\n")
        .map_err(|e| format!("failed to write CMakeLists.txt: {}", e))?;
    run_git(
        &["add", "tinyxml2.h", "tinyxml2.cpp", "xmltest.cpp", "CMakeLists.txt"],
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

    let (repo_url, pinned_commit, _newer_commit) = create_local_tinyxml2_like_repo(&root)
        .expect("failed to create local tinyxml2-like repo");
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

    let (repo_url, pinned_commit, newer_commit) = create_local_tinyxml2_like_repo(&root)
        .expect("failed to create local tinyxml2-like repo");
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

    let (repo_url, pinned_commit, _newer_commit) = create_local_tinyxml2_like_repo(&root)
        .expect("failed to create local tinyxml2-like repo");
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
