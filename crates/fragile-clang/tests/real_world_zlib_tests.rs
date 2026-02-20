//! Real-world zlib fixture bootstrap tests.
//!
//! The first phase for zlib parity needs a deterministic upstream checkout.
//! This test file adds a pinned fixture checkout helper and validates that
//! the helper is deterministic and idempotent.

use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const ZLIB_REPO_URL: &str = "https://github.com/madler/zlib.git";
const ZLIB_PINNED_COMMIT: &str = "51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf"; // v1.3.1
const ZLIB_CACHE_DIR: &str = "/tmp/fragile_real_world_zlib";
const ZLIB_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_zlib_native_baseline";
const ZLIB_CC_DRIVER_BASELINE_DIR: &str = "/tmp/fragile_real_world_zlib_cc_driver";
const ZLIB_REQUIRED_ARTIFACTS_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_required_artifacts";
const ZLIB_REQUIRED_PATHS: &[&str] = &["zlib.h", "configure", "Makefile.in"];
const ZLIB_REQUIRED_TEST_ARTIFACTS: &[&str] = &[
    "libz.a",
    "example",
    "minigzip",
    "examplesh",
    "minigzipsh",
    "example64",
    "minigzip64",
];
const ZLIB_BASELINE_LOG_FILES: &[&str] = &[
    "configure.status",
    "configure.stdout",
    "configure.stderr",
    "make_test.status",
    "make_test.stdout",
    "make_test.stderr",
    "baseline_manifest.txt",
];
const ZLIB_CC_DRIVER_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
];
const ZLIB_REQUIRED_ARTIFACT_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "artifact_manifest.txt",
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

fn run_native_baseline_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    if !source_dir.join("configure").exists() {
        return Err(format!(
            "native baseline source {} is missing configure script",
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

    let mut configure = Command::new("sh");
    configure.arg("./configure").current_dir(source_dir);
    configure.env("LC_ALL", "C").env("LANG", "C");
    let configure_output = configure.output().map_err(|e| {
        format!(
            "failed to run native baseline configure at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "configure", &configure_output)?;
    if !configure_output.status.success() {
        return Err(format!(
            "native baseline configure failed with status {} (logs: {})",
            status_code(&configure_output),
            log_dir.display()
        ));
    }

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

fn create_logging_cc_driver(driver_dir: &Path, log_path: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(driver_dir).map_err(|e| {
        format!(
            "failed to create driver dir {}: {}",
            driver_dir.display(),
            e
        )
    })?;

    let driver_path = driver_dir.join("fragile_cc_driver.sh");
    let script = r#"#!/bin/sh
set -eu
log_file="${FRAGILE_CC_DRIVER_LOG:-}"
if [ -z "$log_file" ]; then
  echo "FRAGILE_CC_DRIVER_LOG is required" 1>&2
  exit 97
fi
{
  printf 'cwd=%s\n' "$(pwd)"
  printf 'args='
  printf '%s ' "$@"
  printf '\n'
} >> "$log_file"
exec cc "$@"
"#;

    fs::write(&driver_path, script)
        .map_err(|e| format!("failed to write cc driver {}: {}", driver_path.display(), e))?;
    make_executable(&driver_path)?;

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create log dir {}: {}", parent.display(), e))?;
    }
    Ok(driver_path)
}

fn write_cc_driver_manifest(
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
    fs::write(log_dir.join("cc_driver_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write cc driver manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_cc_driver_baseline_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    make_target: &str,
) -> Result<(), String> {
    if !source_dir.join("configure").exists() {
        return Err(format!(
            "cc-driver baseline source {} is missing configure script",
            source_dir.display()
        ));
    }

    fs::create_dir_all(log_dir).map_err(|e| {
        format!(
            "failed to create cc-driver baseline log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;
    let cc_driver_log = log_dir.join("cc_driver.log");
    let cc_driver = create_logging_cc_driver(log_dir, &cc_driver_log)?;
    let cc_driver_str = cc_driver.to_string_lossy().to_string();
    let cc_driver_log_str = cc_driver_log.to_string_lossy().to_string();

    let mut configure = Command::new("sh");
    configure.arg("./configure").current_dir(source_dir);
    configure
        .env("CC", cc_driver_str.as_str())
        .env("FRAGILE_CC_DRIVER_LOG", cc_driver_log_str.as_str())
        .env("LC_ALL", "C")
        .env("LANG", "C");
    let configure_output = configure.output().map_err(|e| {
        format!(
            "failed to run cc-driver configure at {}: {}",
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "configure_driver", &configure_output)?;
    if !configure_output.status.success() {
        return Err(format!(
            "cc-driver configure failed with status {} (logs: {})",
            status_code(&configure_output),
            log_dir.display()
        ));
    }

    let mut make_cmd = Command::new("make");
    make_cmd.arg(make_target).current_dir(source_dir);
    make_cmd
        .env("CC", cc_driver_str.as_str())
        .env("FRAGILE_CC_DRIVER_LOG", cc_driver_log_str.as_str())
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("MAKEFLAGS", "-j1");
    let make_output = make_cmd.output().map_err(|e| {
        format!(
            "failed to run cc-driver make {} at {}: {}",
            make_target,
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, "make_driver", &make_output)?;
    if !make_output.status.success() {
        return Err(format!(
            "cc-driver make {} failed with status {} (logs: {})",
            make_target,
            status_code(&make_output),
            log_dir.display()
        ));
    }

    let driver_log = fs::read_to_string(&cc_driver_log).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            cc_driver_log.display(),
            e
        )
    })?;
    if driver_log.trim().is_empty() {
        return Err(format!(
            "cc-driver log {} is empty; expected compiler invocations",
            cc_driver_log.display()
        ));
    }

    write_cc_driver_manifest(log_dir, source_dir, make_target)?;
    Ok(())
}

fn ensure_required_artifacts_exist(
    source_dir: &Path,
    required_artifacts: &[&str],
) -> Result<(), String> {
    let mut missing: Vec<String> = Vec::new();
    for rel in required_artifacts {
        if !source_dir.join(rel).exists() {
            missing.push((*rel).to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "missing required artifacts after make in {}: {}",
            source_dir.display(),
            missing.join(", ")
        ))
    }
}

fn write_artifact_manifest(
    log_dir: &Path,
    source_dir: &Path,
    make_target: &str,
    required_artifacts: &[&str],
) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let manifest = format!(
        "source_dir={}\ncommit={}\nmake_target={}\nrequired_artifacts={}\n",
        source_dir.display(),
        head.trim(),
        make_target,
        required_artifacts.join(",")
    );
    fs::write(log_dir.join("artifact_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write artifact manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_cc_driver_required_artifacts_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    make_target: &str,
    required_artifacts: &[&str],
) -> Result<(), String> {
    run_cc_driver_baseline_in_tree(source_dir, log_dir, make_target)?;
    ensure_required_artifacts_exist(source_dir, required_artifacts)?;
    write_artifact_manifest(log_dir, source_dir, make_target, required_artifacts)?;
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

fn reset_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("failed to remove {}: {}", path.display(), e))?;
    }
    fs::create_dir_all(path).map_err(|e| format!("failed to create {}: {}", path.display(), e))
}

fn run_zlib_native_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_NATIVE_BASELINE_DIR);
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
        &["checkout", "--detach", ZLIB_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != ZLIB_PINNED_COMMIT {
        return Err(format!(
            "native baseline worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("native_logs");
    run_native_baseline_in_tree(&worktree_dir, &log_dir)?;
    write_baseline_manifest(&log_dir, &worktree_dir)?;

    Ok(log_dir)
}

fn run_zlib_cc_driver_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_CC_DRIVER_BASELINE_DIR);
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
        &["checkout", "--detach", ZLIB_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != ZLIB_PINNED_COMMIT {
        return Err(format!(
            "cc-driver worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_cc_driver_baseline_in_tree(&worktree_dir, &log_dir, "adler32.o")?;
    Ok(log_dir)
}

fn run_zlib_required_artifacts_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_REQUIRED_ARTIFACTS_BASELINE_DIR);
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
        &["checkout", "--detach", ZLIB_PINNED_COMMIT],
        Some(&worktree_dir),
    )?;

    let actual_head = read_head(&worktree_dir)
        .ok_or_else(|| format!("failed to read HEAD in {}", worktree_dir.display()))?;
    if actual_head != ZLIB_PINNED_COMMIT {
        return Err(format!(
            "required-artifacts worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_cc_driver_required_artifacts_in_tree(
        &worktree_dir,
        &log_dir,
        "all",
        ZLIB_REQUIRED_TEST_ARTIFACTS,
    )?;
    Ok(log_dir)
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

fn create_local_native_baseline_project(
    base_dir: &Path,
    fail_make_test: bool,
) -> Result<PathBuf, String> {
    let project_dir = base_dir.join("native_baseline_project");
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;

    let configure = r#"#!/bin/sh
set -eu
echo "configure stdout"
echo "configure stderr" 1>&2
printf 'configured\n' > config.status
"#;
    fs::write(project_dir.join("configure"), configure)
        .map_err(|e| format!("failed to write configure script: {}", e))?;

    let makefile = if fail_make_test {
        r#"test:
	@echo "make test stdout"
	@echo "make test stderr" 1>&2
	@exit 2
"#
    } else {
        r#"test:
	@echo "make test stdout"
	@echo "make test stderr" 1>&2
	@printf 'baseline output\n' > baseline.output
"#
    };
    fs::write(project_dir.join("Makefile"), makefile)
        .map_err(|e| format!("failed to write Makefile: {}", e))?;

    Ok(project_dir)
}

fn create_local_cc_driver_project(base_dir: &Path, fail_make: bool) -> Result<PathBuf, String> {
    let project_dir = base_dir.join("cc_driver_project");
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;

    let configure = r#"#!/bin/sh
set -eu
cat > conftest.c <<'EOF'
int main(void) { return 0; }
EOF
"${CC:-cc}" -c conftest.c -o conftest.o
rm -f conftest.c conftest.o
cat > Makefile <<'EOF'
CC ?= cc
all: hello.o

hello.o: hello.c
	$(CC) -c hello.c -o hello.o
EOF
"#;
    fs::write(project_dir.join("configure"), configure)
        .map_err(|e| format!("failed to write configure script: {}", e))?;
    make_executable(&project_dir.join("configure"))?;

    let source = if fail_make {
        "int broken( { return 0; }\n"
    } else {
        "int answer(void) { return 42; }\n"
    };
    fs::write(project_dir.join("hello.c"), source)
        .map_err(|e| format!("failed to write hello.c: {}", e))?;
    Ok(project_dir)
}

fn create_local_required_artifacts_project(
    base_dir: &Path,
    omit_minigzip64: bool,
) -> Result<PathBuf, String> {
    let project_dir = base_dir.join("required_artifacts_project");
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("failed to create {}: {}", project_dir.display(), e))?;

    let configure = r#"#!/bin/sh
set -eu
cat > conftest.c <<'EOF'
int main(void) { return 0; }
EOF
"${CC:-cc}" -c conftest.c -o conftest.o
rm -f conftest.c conftest.o
"#;
    fs::write(project_dir.join("configure"), configure)
        .map_err(|e| format!("failed to write configure script: {}", e))?;
    make_executable(&project_dir.join("configure"))?;

    fs::write(project_dir.join("tiny.c"), "int main(void) { return 0; }\n")
        .map_err(|e| format!("failed to write tiny.c: {}", e))?;

    let makefile = if omit_minigzip64 {
        r#"CC ?= cc
all: static shared all64
static: libz.a example minigzip
shared: examplesh minigzipsh
all64: example64

tiny.o: tiny.c
	$(CC) -c tiny.c -o tiny.o

libz.a: tiny.o
	ar rcs libz.a tiny.o

example: tiny.o
	$(CC) tiny.o -o example
minigzip: tiny.o
	$(CC) tiny.o -o minigzip
examplesh: tiny.o
	$(CC) tiny.o -o examplesh
minigzipsh: tiny.o
	$(CC) tiny.o -o minigzipsh
example64: tiny.o
	$(CC) tiny.o -o example64
"#
    } else {
        r#"CC ?= cc
all: static shared all64
static: libz.a example minigzip
shared: examplesh minigzipsh
all64: example64 minigzip64

tiny.o: tiny.c
	$(CC) -c tiny.c -o tiny.o

libz.a: tiny.o
	ar rcs libz.a tiny.o

example: tiny.o
	$(CC) tiny.o -o example
minigzip: tiny.o
	$(CC) tiny.o -o minigzip
examplesh: tiny.o
	$(CC) tiny.o -o examplesh
minigzipsh: tiny.o
	$(CC) tiny.o -o minigzipsh
example64: tiny.o
	$(CC) tiny.o -o example64
minigzip64: tiny.o
	$(CC) tiny.o -o minigzip64
"#
    };
    fs::write(project_dir.join("Makefile"), makefile)
        .map_err(|e| format!("failed to write Makefile: {}", e))?;

    Ok(project_dir)
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
    assert!(checkout_has_required_files(
        &checkout_dir,
        ZLIB_REQUIRED_PATHS
    ));
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(pinned_commit.as_str())
    );

    ensure_pinned_checkout(
        &repo_url,
        &checkout_dir,
        &pinned_commit,
        ZLIB_REQUIRED_PATHS,
    )
    .expect("second ensure should be idempotent");
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(pinned_commit.as_str())
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_ensure_pinned_checkout_rewinds_existing_checkout_to_pin() {
    let root = unique_temp_dir("zlib_fixture_rewind");
    fs::create_dir_all(&root).expect("failed to create test root");
    let (repo_url, pinned_commit, newer_commit) =
        create_local_zlib_like_repo(&root).expect("failed to create local remote repo");
    let checkout_dir = root.join("checkout");

    ensure_pinned_checkout(
        &repo_url,
        &checkout_dir,
        &pinned_commit,
        ZLIB_REQUIRED_PATHS,
    )
    .expect("failed to ensure first pinned checkout");
    run_git(
        &["checkout", "--detach", newer_commit.as_str()],
        Some(&checkout_dir),
    )
    .expect("failed to move checkout to newer commit");
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(newer_commit.as_str())
    );

    ensure_pinned_checkout(
        &repo_url,
        &checkout_dir,
        &pinned_commit,
        ZLIB_REQUIRED_PATHS,
    )
    .expect("failed to rewind checkout to pin");
    assert_eq!(
        read_head(&checkout_dir).as_deref(),
        Some(pinned_commit.as_str())
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_native_baseline_capture_logs_successful_flow() {
    let root = unique_temp_dir("zlib_native_baseline_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir =
        create_local_native_baseline_project(&root, false).expect("failed to create local project");
    let log_dir = root.join("logs");
    run_native_baseline_in_tree(&project_dir, &log_dir).expect("native baseline should succeed");
    write_baseline_manifest(&log_dir, &project_dir).expect("failed to write baseline manifest");

    assert!(
        project_dir.join("baseline.output").exists(),
        "baseline output artifact should exist"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("configure.status"))
            .expect("failed to read configure.status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("make_test.status"))
            .expect("failed to read make_test.status")
            .trim(),
        "0"
    );
    assert!(fs::read_to_string(log_dir.join("configure.stdout"))
        .expect("failed to read configure.stdout")
        .contains("configure stdout"));
    assert!(fs::read_to_string(log_dir.join("make_test.stderr"))
        .expect("failed to read make_test.stderr")
        .contains("make test stderr"));
    assert!(fs::read_to_string(log_dir.join("baseline_manifest.txt"))
        .expect("failed to read baseline manifest")
        .contains("source_dir="));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_native_baseline_capture_logs_failed_make_test() {
    let root = unique_temp_dir("zlib_native_baseline_failure");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir =
        create_local_native_baseline_project(&root, true).expect("failed to create local project");
    let log_dir = root.join("logs");
    let result = run_native_baseline_in_tree(&project_dir, &log_dir);
    assert!(result.is_err(), "expected make test failure to propagate");

    assert_eq!(
        fs::read_to_string(log_dir.join("configure.status"))
            .expect("failed to read configure.status")
            .trim(),
        "0"
    );
    assert_ne!(
        fs::read_to_string(log_dir.join("make_test.status"))
            .expect("failed to read make_test.status")
            .trim(),
        "0"
    );
    assert!(fs::read_to_string(log_dir.join("make_test.stdout"))
        .expect("failed to read make_test.stdout")
        .contains("make test stdout"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_cc_driver_path_invoked_by_configure_and_make_local_fixture() {
    let root = unique_temp_dir("zlib_cc_driver_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir =
        create_local_cc_driver_project(&root, false).expect("failed to create cc-driver project");
    let log_dir = root.join("logs");
    run_cc_driver_baseline_in_tree(&project_dir, &log_dir, "all")
        .expect("cc-driver baseline should succeed");

    assert!(
        project_dir.join("hello.o").exists(),
        "expected local make target to create hello.o"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("configure_driver.status"))
            .expect("failed to read configure_driver.status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("make_driver.status"))
            .expect("failed to read make_driver.status")
            .trim(),
        "0"
    );

    let driver_log =
        fs::read_to_string(log_dir.join("cc_driver.log")).expect("failed to read cc_driver.log");
    assert!(
        driver_log.contains("conftest.c"),
        "configure compile probe should route through CC driver: {}",
        driver_log
    );
    assert!(
        driver_log.contains("hello.c"),
        "make compile should route through CC driver: {}",
        driver_log
    );

    assert!(fs::read_to_string(log_dir.join("cc_driver_manifest.txt"))
        .expect("failed to read cc_driver manifest")
        .contains("make_target=all"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_cc_driver_path_surfaces_make_failure_with_logs() {
    let root = unique_temp_dir("zlib_cc_driver_failure");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir =
        create_local_cc_driver_project(&root, true).expect("failed to create cc-driver project");
    let log_dir = root.join("logs");
    let result = run_cc_driver_baseline_in_tree(&project_dir, &log_dir, "all");
    assert!(result.is_err(), "expected make failure to propagate");

    assert_eq!(
        fs::read_to_string(log_dir.join("configure_driver.status"))
            .expect("failed to read configure_driver.status")
            .trim(),
        "0"
    );
    assert_ne!(
        fs::read_to_string(log_dir.join("make_driver.status"))
            .expect("failed to read make_driver.status")
            .trim(),
        "0"
    );

    let driver_log =
        fs::read_to_string(log_dir.join("cc_driver.log")).expect("failed to read cc_driver.log");
    assert!(
        driver_log.contains("hello.c"),
        "failed make should still show CC driver invocation: {}",
        driver_log
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_required_artifacts_build_local_fixture_success() {
    let root = unique_temp_dir("zlib_required_artifacts_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, false)
        .expect("failed to create required-artifacts project");
    let log_dir = root.join("logs");
    run_cc_driver_required_artifacts_in_tree(
        &project_dir,
        &log_dir,
        "all",
        ZLIB_REQUIRED_TEST_ARTIFACTS,
    )
    .expect("required-artifacts build should succeed");

    for rel in ZLIB_REQUIRED_TEST_ARTIFACTS {
        assert!(
            project_dir.join(rel).exists(),
            "expected local artifact {}",
            project_dir.join(rel).display()
        );
    }

    assert!(fs::read_to_string(log_dir.join("artifact_manifest.txt"))
        .expect("failed to read artifact_manifest.txt")
        .contains("required_artifacts="));
    assert!(fs::read_to_string(log_dir.join("cc_driver.log"))
        .expect("failed to read cc_driver.log")
        .contains("tiny.c"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_required_artifacts_build_detects_missing_output() {
    let root = unique_temp_dir("zlib_required_artifacts_missing");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, true)
        .expect("failed to create required-artifacts project");
    let log_dir = root.join("logs");
    let result = run_cc_driver_required_artifacts_in_tree(
        &project_dir,
        &log_dir,
        "all",
        ZLIB_REQUIRED_TEST_ARTIFACTS,
    );
    let error = result.expect_err("missing artifact should fail validation");
    assert!(
        error.contains("missing required artifacts"),
        "unexpected error message: {}",
        error
    );
    assert!(
        error.contains("minigzip64"),
        "missing artifact should mention minigzip64: {}",
        error
    );
    assert!(
        log_dir.join("cc_driver.log").exists(),
        "cc-driver invocation log should still be emitted on missing artifact failure"
    );
    assert!(
        !log_dir.join("artifact_manifest.txt").exists(),
        "artifact manifest should not be written when validation fails"
    );

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

#[test]
#[ignore = "real-world external project test (downloads and builds zlib)"]
fn test_real_world_zlib_native_baseline_configure_make_test() {
    let log_dir = run_zlib_native_baseline().expect("failed to run zlib native baseline");
    for rel in ZLIB_BASELINE_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected native baseline log {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        fs::read_to_string(log_dir.join("configure.status"))
            .expect("failed to read configure.status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("make_test.status"))
            .expect("failed to read make_test.status")
            .trim(),
        "0"
    );
}

#[test]
#[ignore = "real-world external project test (downloads and configures zlib with CC driver)"]
fn test_real_world_zlib_cc_driver_path_configure_make_object() {
    let log_dir = run_zlib_cc_driver_baseline().expect("failed to run zlib cc-driver baseline");
    for rel in ZLIB_CC_DRIVER_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected cc-driver log {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        fs::read_to_string(log_dir.join("configure_driver.status"))
            .expect("failed to read configure_driver.status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("make_driver.status"))
            .expect("failed to read make_driver.status")
            .trim(),
        "0"
    );
    assert!(fs::read_to_string(log_dir.join("cc_driver.log"))
        .expect("failed to read cc_driver.log")
        .contains("adler32.c"));
}

#[test]
#[ignore = "real-world external project test (downloads and builds zlib make all artifact scope)"]
fn test_real_world_zlib_required_artifacts_for_make_all_scope() {
    let log_dir = run_zlib_required_artifacts_baseline()
        .expect("failed to run zlib required-artifacts build");
    for rel in ZLIB_REQUIRED_ARTIFACT_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected required-artifacts log {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        fs::read_to_string(log_dir.join("configure_driver.status"))
            .expect("failed to read configure_driver.status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("make_driver.status"))
            .expect("failed to read make_driver.status")
            .trim(),
        "0"
    );
    let manifest =
        fs::read_to_string(log_dir.join("artifact_manifest.txt")).expect("failed to read manifest");
    for artifact in ZLIB_REQUIRED_TEST_ARTIFACTS {
        assert!(
            manifest.contains(artifact),
            "artifact manifest should include {}: {}",
            artifact,
            manifest
        );
    }
}
