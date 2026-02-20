//! Real-world zlib fixture bootstrap tests.
//!
//! The first phase for zlib parity needs a deterministic upstream checkout.
//! This test file adds a pinned fixture checkout helper and validates that
//! the helper is deterministic and idempotent.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ZLIB_REPO_URL: &str = "https://github.com/madler/zlib.git";
const ZLIB_PINNED_COMMIT: &str = "51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf"; // v1.3.1
const ZLIB_CACHE_DIR: &str = "/tmp/fragile_real_world_zlib";
const ZLIB_REQUIRED_PATHS: &[&str] = &["zlib.h", "configure", "Makefile.in"];

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

    run_git(&["fetch", "--depth", "1", "origin", pinned_commit], Some(repo_dir))?;
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

fn ensure_zlib_checkout() -> Result<PathBuf, String> {
    ensure_pinned_checkout(
        ZLIB_REPO_URL,
        Path::new(ZLIB_CACHE_DIR),
        ZLIB_PINNED_COMMIT,
        ZLIB_REQUIRED_PATHS,
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

fn create_local_zlib_like_repo(base_dir: &Path) -> Result<(String, String, String), String> {
    let remote_dir = base_dir.join("remote_repo");
    fs::create_dir_all(&remote_dir)
        .map_err(|e| format!("failed to create {}: {}", remote_dir.display(), e))?;

    run_git(&["init"], Some(&remote_dir))?;
    run_git(
        &["config", "user.email", "fragile-tests@example.invalid"],
        Some(&remote_dir),
    )?;
    run_git(&["config", "user.name", "Fragile Tests"], Some(&remote_dir))?;

    fs::write(remote_dir.join("zlib.h"), "/* zlib fixture header */\n")
        .map_err(|e| format!("failed to write zlib.h: {}", e))?;
    fs::write(remote_dir.join("configure"), "#!/bin/sh\nexit 0\n")
        .map_err(|e| format!("failed to write configure: {}", e))?;
    fs::write(remote_dir.join("Makefile.in"), "all:\n\t@echo ok\n")
        .map_err(|e| format!("failed to write Makefile.in: {}", e))?;
    run_git(
        &["add", "zlib.h", "configure", "Makefile.in"],
        Some(&remote_dir),
    )?;
    run_git(&["commit", "-m", "first"], Some(&remote_dir))?;
    let pinned_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;

    fs::write(remote_dir.join("zlib.h"), "/* zlib fixture header v2 */\n")
        .map_err(|e| format!("failed to update zlib.h: {}", e))?;
    run_git(&["add", "zlib.h"], Some(&remote_dir))?;
    run_git(&["commit", "-m", "second"], Some(&remote_dir))?;
    let newer_commit = git_stdout(&["rev-parse", "HEAD"], Some(&remote_dir))?;

    Ok((
        format!("file://{}", remote_dir.display()),
        pinned_commit,
        newer_commit,
    ))
}

#[test]
fn test_ensure_pinned_checkout_clones_and_pins_local_repo() {
    let root = unique_temp_dir("zlib_fixture_clone_pin");
    fs::create_dir_all(&root).expect("failed to create test root");
    let (repo_url, pinned_commit, _newer_commit) =
        create_local_zlib_like_repo(&root).expect("failed to create local remote repo");
    let checkout_dir = root.join("checkout");

    let checkout = ensure_pinned_checkout(
        &repo_url,
        &checkout_dir,
        &pinned_commit,
        ZLIB_REQUIRED_PATHS,
    )
    .expect("failed to ensure pinned checkout");
    assert_eq!(checkout, checkout_dir);
    assert!(checkout_has_required_files(&checkout_dir, ZLIB_REQUIRED_PATHS));
    assert_eq!(read_head(&checkout_dir).as_deref(), Some(pinned_commit.as_str()));

    ensure_pinned_checkout(&repo_url, &checkout_dir, &pinned_commit, ZLIB_REQUIRED_PATHS)
        .expect("second ensure should be idempotent");
    assert_eq!(read_head(&checkout_dir).as_deref(), Some(pinned_commit.as_str()));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_ensure_pinned_checkout_rewinds_existing_checkout_to_pin() {
    let root = unique_temp_dir("zlib_fixture_rewind");
    fs::create_dir_all(&root).expect("failed to create test root");
    let (repo_url, pinned_commit, newer_commit) =
        create_local_zlib_like_repo(&root).expect("failed to create local remote repo");
    let checkout_dir = root.join("checkout");

    ensure_pinned_checkout(&repo_url, &checkout_dir, &pinned_commit, ZLIB_REQUIRED_PATHS)
        .expect("failed to ensure first pinned checkout");
    run_git(
        &["checkout", "--detach", newer_commit.as_str()],
        Some(&checkout_dir),
    )
    .expect("failed to move checkout to newer commit");
    assert_eq!(read_head(&checkout_dir).as_deref(), Some(newer_commit.as_str()));

    ensure_pinned_checkout(&repo_url, &checkout_dir, &pinned_commit, ZLIB_REQUIRED_PATHS)
        .expect("failed to rewind checkout to pin");
    assert_eq!(read_head(&checkout_dir).as_deref(), Some(pinned_commit.as_str()));

    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore = "real-world external project test (downloads zlib fixture)"]
fn test_real_world_zlib_fixture_checkout_is_pinned() {
    let repo_dir = ensure_zlib_checkout().expect("failed to prepare zlib checkout");
    assert!(
        repo_dir.join("zlib.h").exists(),
        "expected {}",
        repo_dir.join("zlib.h").display()
    );
    assert!(
        repo_dir.join("configure").exists(),
        "expected {}",
        repo_dir.join("configure").display()
    );
    assert!(
        repo_dir.join("Makefile.in").exists(),
        "expected {}",
        repo_dir.join("Makefile.in").display()
    );

    let head = read_head(&repo_dir).expect("failed to query HEAD");
    assert_eq!(
        head, ZLIB_PINNED_COMMIT,
        "zlib checkout must stay pinned for deterministic parity runs"
    );
}
