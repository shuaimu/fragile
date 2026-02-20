//! Real-world zlib fixture bootstrap tests.
//!
//! The first phase for zlib parity needs a deterministic upstream checkout.
//! This test file adds a pinned fixture checkout helper and validates that
//! the helper is deterministic and idempotent.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fragile_clang::{AstCodeGen, ClangParser, ParserLanguage};

const ZLIB_REPO_URL: &str = "https://github.com/madler/zlib.git";
const ZLIB_PINNED_COMMIT: &str = "51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf"; // v1.3.1
const ZLIB_CACHE_DIR: &str = "/tmp/fragile_real_world_zlib";
const ZLIB_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_zlib_native_baseline";
const ZLIB_CC_DRIVER_BASELINE_DIR: &str = "/tmp/fragile_real_world_zlib_cc_driver";
const ZLIB_REQUIRED_ARTIFACTS_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_required_artifacts";
const ZLIB_FRAGILE_ADLER32_OBJECT_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_fragile_adler32_object";
const ZLIB_LIBZA_REPLAY_PLAN_BASELINE_DIR: &str = "/tmp/fragile_real_world_zlib_libza_replay_plan";
const ZLIB_FRAGILE_OBJZ_OBJECTS_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_fragile_objz_objects";
const ZLIB_FRAGILE_OBJG_OBJECTS_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_fragile_objg_objects";
const ZLIB_FRAGILE_LINK_REQUIRED_BINARIES_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_fragile_link_required_binaries";
const ZLIB_MAKE_TEST_COMMAND_PLAN_BASELINE_DIR: &str =
    "/tmp/fragile_real_world_zlib_make_test_command_plan";
const ZLIB_MAKE_TEST_REPLAY_BASELINE_DIR: &str = "/tmp/fragile_real_world_zlib_make_test_replay";
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
const ZLIB_REQUIRED_LINK_OUTPUTS: &[&str] = &[
    "example",
    "minigzip",
    "examplesh",
    "minigzipsh",
    "example64",
    "minigzip64",
];
const ZLIB_ARTIFACT_PARITY_PROBE_BINARIES: &[&str] = &["minigzip", "minigzipsh", "minigzip64"];
const ZLIB_OBJZ_OBJECTS: &[&str] = &[
    "adler32.o",
    "crc32.o",
    "deflate.o",
    "infback.o",
    "inffast.o",
    "inflate.o",
    "inftrees.o",
    "trees.o",
    "zutil.o",
];
const ZLIB_OBJG_OBJECTS: &[&str] = &[
    "compress.o",
    "uncompr.o",
    "gzclose.o",
    "gzlib.o",
    "gzread.o",
    "gzwrite.o",
];
const ZLIB_LIBZA_OBJECTS: &[&str] = &[
    "adler32.o",
    "crc32.o",
    "deflate.o",
    "infback.o",
    "inffast.o",
    "inflate.o",
    "inftrees.o",
    "trees.o",
    "zutil.o",
    "compress.o",
    "uncompr.o",
    "gzclose.o",
    "gzlib.o",
    "gzread.o",
    "gzwrite.o",
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
    "compile_units_manifest.txt",
    "link_units_manifest.txt",
];
const ZLIB_FRAGILE_ADLER32_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "compile_units_manifest.txt",
    "adler32_transpiled.rs",
    "rustc_object.status",
    "rustc_object.stdout",
    "rustc_object.stderr",
    "fragile_object_manifest.txt",
];
const ZLIB_LIBZA_REPLAY_PLAN_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "compile_units_manifest.txt",
    "libza_replay_plan.txt",
];
const ZLIB_FRAGILE_OBJZ_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "compile_units_manifest.txt",
    "libza_replay_plan.txt",
];
const ZLIB_FRAGILE_OBJG_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "compile_units_manifest.txt",
    "libza_replay_plan.txt",
];
const ZLIB_FRAGILE_LINK_REQUIRED_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "artifact_manifest.txt",
    "compile_units_manifest.txt",
    "link_units_manifest.txt",
];
const ZLIB_MAKE_TEST_COMMAND_PLAN_LOG_FILES: &[&str] = &[
    "configure_driver.status",
    "configure_driver.stdout",
    "configure_driver.stderr",
    "make_driver.status",
    "make_driver.stdout",
    "make_driver.stderr",
    "cc_driver.log",
    "cc_driver_manifest.txt",
    "artifact_manifest.txt",
    "compile_units_manifest.txt",
    "link_units_manifest.txt",
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

fn extract_arg_value(tokens: &[&str], flag: &str) -> Option<(String, Option<usize>)> {
    for (idx, tok) in tokens.iter().enumerate() {
        if *tok == flag {
            if let Some(next) = tokens.get(idx + 1) {
                return Some(((*next).to_string(), Some(idx + 1)));
            }
        } else if tok.starts_with(flag) && tok.len() > flag.len() {
            return Some((tok[flag.len()..].to_string(), None));
        }
    }
    None
}

fn extract_compile_source_token(
    tokens: &[&str],
    object_token: &str,
    object_consumed_idx: Option<usize>,
) -> Option<String> {
    let mut source_candidate: Option<&str> = None;
    if let Some(c_idx) = tokens.iter().position(|t| *t == "-c") {
        if let Some(next) = tokens.get(c_idx + 1) {
            if !next.starts_with('-') {
                source_candidate = Some(next);
            }
        }
    }
    if source_candidate.is_none() {
        if let Some(attached) = tokens
            .iter()
            .find(|t| t.starts_with("-c") && t.len() > 2)
            .map(|t| &t[2..])
        {
            source_candidate = Some(attached);
        }
    }
    if source_candidate.is_none() {
        let mut positional: Vec<(usize, &str)> = Vec::new();
        for (idx, tok) in tokens.iter().enumerate() {
            if tok.starts_with('-') {
                continue;
            }
            if object_consumed_idx.is_some_and(|i| i == idx) {
                continue;
            }
            positional.push((idx, tok));
        }
        source_candidate = positional
            .iter()
            .rev()
            .map(|(_, tok)| *tok)
            .find(|tok| *tok != object_token);
    }
    source_candidate.map(ToString::to_string)
}

fn parse_compile_units_from_cc_driver_log(
    log_text: &str,
    source_root: &Path,
) -> Result<Vec<(String, String)>, String> {
    let mut cwd = source_root.to_path_buf();
    let mut units: BTreeSet<(String, String)> = BTreeSet::new();

    for line in log_text.lines() {
        if let Some(rest) = line.strip_prefix("cwd=") {
            let parsed = PathBuf::from(rest.trim());
            if parsed.as_os_str().is_empty() {
                continue;
            }
            cwd = parsed;
            continue;
        }
        let Some(rest) = line.strip_prefix("args=") else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let has_compile_flag = tokens.iter().any(|t| *t == "-c" || t.starts_with("-c"));
        if !has_compile_flag {
            continue;
        }
        let Some((obj, obj_consumed_idx)) = extract_arg_value(&tokens, "-o") else {
            continue;
        };
        let Some(src) = extract_compile_source_token(&tokens, &obj, obj_consumed_idx) else {
            continue;
        };

        let source = normalize_path_for_manifest(&src, &cwd, source_root);
        let object = normalize_path_for_manifest(&obj, &cwd, source_root);
        units.insert((source, object));
    }

    if units.is_empty() {
        return Err("no compile units found in cc_driver.log".to_string());
    }
    Ok(units.into_iter().collect())
}

fn parse_link_units_from_cc_driver_log(
    log_text: &str,
    source_root: &Path,
) -> Result<Vec<(String, Vec<String>)>, String> {
    fn is_compiler_driver_token(token: &str) -> bool {
        let base = Path::new(token)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(token);
        matches!(base, "cc" | "gcc" | "g++" | "clang" | "clang++" | "c++")
            || base.starts_with("clang-")
            || base.starts_with("gcc-")
    }

    let mut cwd = source_root.to_path_buf();
    let mut units: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for line in log_text.lines() {
        if let Some(rest) = line.strip_prefix("cwd=") {
            let parsed = PathBuf::from(rest.trim());
            if parsed.as_os_str().is_empty() {
                continue;
            }
            cwd = parsed;
            continue;
        }
        let Some(rest) = line.strip_prefix("args=") else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        if tokens.iter().any(|t| *t == "-c" || t.starts_with("-c")) {
            continue;
        }

        let Some((output_token, output_consumed_idx)) = extract_arg_value(&tokens, "-o") else {
            continue;
        };
        let output = normalize_path_for_manifest(&output_token, &cwd, source_root);

        let mut inputs: Vec<String> = Vec::new();
        let mut skip_next = false;
        for (idx, tok) in tokens.iter().enumerate() {
            if idx == 0 && is_compiler_driver_token(tok) {
                continue;
            }
            if skip_next {
                skip_next = false;
                continue;
            }
            if *tok == "-o" {
                skip_next = true;
                continue;
            }
            if output_consumed_idx.is_some_and(|i| i == idx) {
                continue;
            }
            if tok.starts_with("-o") || tok.starts_with('-') {
                continue;
            }
            let normalized = normalize_path_for_manifest(tok, &cwd, source_root);
            if normalized == output {
                continue;
            }
            if !inputs.contains(&normalized) {
                inputs.push(normalized);
            }
        }

        units.entry(output).or_insert(inputs);
    }

    if units.is_empty() {
        return Err("no link units found in cc_driver.log".to_string());
    }
    Ok(units.into_iter().collect())
}

fn write_compile_units_manifest(log_dir: &Path, source_dir: &Path) -> Result<usize, String> {
    let driver_log_path = log_dir.join("cc_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            driver_log_path.display(),
            e
        )
    })?;
    let units = parse_compile_units_from_cc_driver_log(&driver_log, source_dir)?;

    let mut manifest = format!(
        "source_dir={}\ncompile_units_count={}\n",
        source_dir.display(),
        units.len()
    );
    for (source, object) in &units {
        manifest.push_str(&format!("source={} object={}\n", source, object));
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

fn write_link_units_manifest(
    log_dir: &Path,
    source_dir: &Path,
    required_outputs: &[&str],
) -> Result<usize, String> {
    let driver_log_path = log_dir.join("cc_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            driver_log_path.display(),
            e
        )
    })?;
    let link_units = parse_link_units_from_cc_driver_log(&driver_log, source_dir)?;
    let link_map: std::collections::BTreeMap<String, Vec<String>> =
        link_units.into_iter().collect();

    let mut missing_outputs: Vec<String> = Vec::new();
    for output in required_outputs {
        if !link_map.contains_key(*output) {
            missing_outputs.push((*output).to_string());
        }
    }
    if !missing_outputs.is_empty() {
        return Err(format!(
            "missing link units for required outputs: {}",
            missing_outputs.join(", ")
        ));
    }

    let mut manifest = format!(
        "source_dir={}\nlink_units_count={}\nrequired_link_outputs={}\n",
        source_dir.display(),
        required_outputs.len(),
        required_outputs.join(","),
    );
    for output in required_outputs {
        let inputs = link_map
            .get(*output)
            .ok_or_else(|| format!("internal error: missing inputs for {}", output))?;
        manifest.push_str(&format!("output={} inputs={}\n", output, inputs.join(",")));
    }
    fs::write(log_dir.join("link_units_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write link units manifest at {}: {}",
            log_dir.display(),
            e
        )
    })?;
    Ok(required_outputs.len())
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
    run_cc_driver_required_artifacts_in_tree(
        source_dir,
        log_dir,
        "all",
        ZLIB_REQUIRED_TEST_ARTIFACTS,
    )?;
    run_make_test_dry_run_in_tree(source_dir, log_dir)?;
    write_make_test_commands_manifest(log_dir, source_dir, ZLIB_REQUIRED_LINK_OUTPUTS)?;
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

fn replay_make_test_commands_from_manifest_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<usize, String> {
    let manifest_path = log_dir.join("make_test_commands_manifest.txt");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {}", manifest_path.display(), e))?;
    let commands = parse_make_test_commands_manifest_entries(&manifest)?;

    for (idx, command_line) in commands.iter().enumerate() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command_line).current_dir(source_dir);
        cmd.env("LC_ALL", "C").env("LANG", "C");
        let output = cmd.output().map_err(|e| {
            format!(
                "failed to run make-test replay command {} at {}: {}",
                idx + 1,
                source_dir.display(),
                e
            )
        })?;
        let step = make_test_replay_step_name(idx);
        write_command_capture(log_dir, &step, &output)?;
        if !output.status.success() {
            return Err(format!(
                "make-test command replay failed at command {} with status {}: {} (logs: {})",
                idx + 1,
                status_code(&output),
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

fn run_make_test_command_subset_replay_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    run_fragile_link_required_binaries_in_tree(source_dir, log_dir)?;
    run_make_test_dry_run_in_tree(source_dir, log_dir)?;
    write_make_test_commands_manifest(log_dir, source_dir, ZLIB_REQUIRED_LINK_OUTPUTS)?;
    replay_make_test_commands_from_manifest_in_tree(source_dir, log_dir)?;
    Ok(())
}

fn read_status_file(path: &Path) -> Result<i32, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    raw.trim()
        .parse::<i32>()
        .map_err(|e| format!("failed to parse status in {}: {}", path.display(), e))
}

fn read_native_make_test_exit_status(native_log_dir: &Path) -> Result<i32, String> {
    read_status_file(&native_log_dir.join("make_test.status"))
}

fn read_make_test_replay_exit_status(replay_log_dir: &Path) -> Result<i32, String> {
    let commands_manifest_path = replay_log_dir.join("make_test_commands_manifest.txt");
    let commands_manifest = fs::read_to_string(&commands_manifest_path)
        .map_err(|e| format!("failed to read {}: {}", commands_manifest_path.display(), e))?;
    let commands = parse_make_test_commands_manifest_entries(&commands_manifest)?;

    if commands.is_empty() {
        return Err("make-test replay has no commands to evaluate".to_string());
    }

    for idx in 0..commands.len() {
        let step = make_test_replay_step_name(idx);
        let status_path = replay_log_dir.join(format!("{}.status", step));
        if !status_path.exists() {
            return Err(format!(
                "missing replay status for step {} at {}",
                step,
                status_path.display()
            ));
        }
        let status = read_status_file(&status_path)?;
        if status != 0 {
            return Ok(status);
        }
    }

    Ok(0)
}

fn assert_make_test_exit_status_parity(
    native_log_dir: &Path,
    replay_log_dir: &Path,
) -> Result<(), String> {
    let native_status = read_native_make_test_exit_status(native_log_dir)?;
    let replay_status = read_make_test_replay_exit_status(replay_log_dir)?;
    if native_status == replay_status {
        Ok(())
    } else {
        Err(format!(
            "exit status parity mismatch: native make test status={} fragile replay status={} (native logs: {}, replay logs: {})",
            native_status,
            replay_status,
            native_log_dir.display(),
            replay_log_dir.display()
        ))
    }
}

fn parse_manifest_value(manifest: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    manifest
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn read_source_dir_from_manifest(
    manifest_path: &Path,
    manifest_name: &str,
) -> Result<PathBuf, String> {
    let manifest = fs::read_to_string(manifest_path)
        .map_err(|e| format!("failed to read {}: {}", manifest_path.display(), e))?;
    let source_dir = parse_manifest_value(&manifest, "source_dir").ok_or_else(|| {
        format!(
            "{} at {} is missing source_dir entry",
            manifest_name,
            manifest_path.display()
        )
    })?;
    Ok(PathBuf::from(source_dir))
}

fn read_native_source_dir_for_parity(native_log_dir: &Path) -> Result<PathBuf, String> {
    read_source_dir_from_manifest(
        &native_log_dir.join("baseline_manifest.txt"),
        "native baseline manifest",
    )
}

fn run_native_make_test_command_replay_for_parity(native_log_dir: &Path) -> Result<(), String> {
    let source_dir = read_native_source_dir_for_parity(native_log_dir)?;
    run_make_test_dry_run_in_tree(&source_dir, native_log_dir)?;
    write_make_test_commands_manifest(native_log_dir, &source_dir, ZLIB_REQUIRED_LINK_OUTPUTS)?;
    match replay_make_test_commands_from_manifest_in_tree(&source_dir, native_log_dir) {
        Ok(_) => Ok(()),
        Err(err) if err.contains("make-test command replay failed at command") => Ok(()),
        Err(err) => Err(format!(
            "native make-test command replay failed unexpectedly: {}",
            err
        )),
    }
}

fn read_capture_stream(path: &Path) -> Result<String, String> {
    let raw = fs::read(path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

fn read_make_test_replay_stream_output(log_dir: &Path, stream: &str) -> Result<String, String> {
    let commands_manifest_path = log_dir.join("make_test_commands_manifest.txt");
    let commands_manifest = fs::read_to_string(&commands_manifest_path)
        .map_err(|e| format!("failed to read {}: {}", commands_manifest_path.display(), e))?;
    let commands = parse_make_test_commands_manifest_entries(&commands_manifest)?;

    if commands.is_empty() {
        return Err("make-test replay has no commands to evaluate".to_string());
    }

    let mut combined = String::new();
    for idx in 0..commands.len() {
        let step = make_test_replay_step_name(idx);
        let status_path = log_dir.join(format!("{}.status", step));
        if !status_path.exists() {
            return Err(format!(
                "missing replay status for step {} at {}",
                step,
                status_path.display()
            ));
        }
        let stream_path = log_dir.join(format!("{}.{}", step, stream));
        if !stream_path.exists() {
            return Err(format!(
                "missing replay {} capture for step {} at {}",
                stream,
                step,
                stream_path.display()
            ));
        }
        combined.push_str(&read_capture_stream(&stream_path)?);
        let status = read_status_file(&status_path)?;
        if status != 0 {
            break;
        }
    }

    Ok(combined)
}

fn read_make_test_replay_stdout(log_dir: &Path) -> Result<String, String> {
    read_make_test_replay_stream_output(log_dir, "stdout")
}

fn read_make_test_replay_stderr(log_dir: &Path) -> Result<String, String> {
    read_make_test_replay_stream_output(log_dir, "stderr")
}

fn collect_make_test_output_path_filters(
    native_log_dir: &Path,
    replay_log_dir: &Path,
) -> Result<Vec<String>, String> {
    let mut filters: Vec<String> = Vec::new();

    let native_source_dir = read_native_source_dir_for_parity(native_log_dir)?;
    filters.push(normalize_slashes(
        native_source_dir.to_string_lossy().as_ref(),
    ));
    filters.push(normalize_slashes(native_log_dir.to_string_lossy().as_ref()));
    filters.push(normalize_slashes(replay_log_dir.to_string_lossy().as_ref()));

    for path in [
        native_log_dir.join("make_test_commands_manifest.txt"),
        replay_log_dir.join("make_test_commands_manifest.txt"),
    ] {
        if !path.exists() {
            continue;
        }
        let source_dir = read_source_dir_from_manifest(&path, "make-test command manifest")?;
        filters.push(normalize_slashes(source_dir.to_string_lossy().as_ref()));
    }

    let mut deduped: Vec<String> = Vec::new();
    for filter in filters {
        if filter.is_empty() {
            continue;
        }
        if !deduped.iter().any(|existing| existing == &filter) {
            deduped.push(filter);
        }
    }
    Ok(deduped)
}

fn normalize_output_text_for_parity(raw: &str, path_filters: &[String]) -> String {
    let mut normalized_lines: Vec<String> = Vec::new();
    for raw_line in raw.lines() {
        let mut line = normalize_slashes(raw_line.trim_end());
        if line.starts_with("make[") || line.starts_with("make:") {
            continue;
        }
        for filter in path_filters {
            line = line.replace(filter, "<PATH>");
        }
        normalized_lines.push(line);
    }
    normalized_lines.join("\n")
}

fn first_output_mismatch(native: &str, replay: &str) -> Option<(usize, String, String)> {
    let native_lines: Vec<&str> = native.lines().collect();
    let replay_lines: Vec<&str> = replay.lines().collect();
    let max_len = native_lines.len().max(replay_lines.len());
    for idx in 0..max_len {
        let native_line = native_lines.get(idx).copied().unwrap_or("<missing>");
        let replay_line = replay_lines.get(idx).copied().unwrap_or("<missing>");
        if native_line != replay_line {
            return Some((idx + 1, native_line.to_string(), replay_line.to_string()));
        }
    }
    None
}

fn assert_make_test_stdout_stderr_parity(
    native_log_dir: &Path,
    replay_log_dir: &Path,
) -> Result<(), String> {
    run_native_make_test_command_replay_for_parity(native_log_dir)?;

    let native_manifest_path = native_log_dir.join("make_test_commands_manifest.txt");
    let native_manifest = fs::read_to_string(&native_manifest_path)
        .map_err(|e| format!("failed to read {}: {}", native_manifest_path.display(), e))?;
    let native_commands = parse_make_test_commands_manifest_entries(&native_manifest)?;

    let replay_manifest_path = replay_log_dir.join("make_test_commands_manifest.txt");
    let replay_manifest = fs::read_to_string(&replay_manifest_path)
        .map_err(|e| format!("failed to read {}: {}", replay_manifest_path.display(), e))?;
    let replay_commands = parse_make_test_commands_manifest_entries(&replay_manifest)?;

    if native_commands != replay_commands {
        return Err(format!(
            "stdout/stderr parity command-plan mismatch: native command_count={} fragile command_count={} (native logs: {}, replay logs: {})",
            native_commands.len(),
            replay_commands.len(),
            native_log_dir.display(),
            replay_log_dir.display()
        ));
    }

    let path_filters = collect_make_test_output_path_filters(native_log_dir, replay_log_dir)?;

    let native_stdout = normalize_output_text_for_parity(
        &read_make_test_replay_stdout(native_log_dir)?,
        &path_filters,
    );
    let replay_stdout = normalize_output_text_for_parity(
        &read_make_test_replay_stdout(replay_log_dir)?,
        &path_filters,
    );
    let native_stderr = normalize_output_text_for_parity(
        &read_make_test_replay_stderr(native_log_dir)?,
        &path_filters,
    );
    let replay_stderr = normalize_output_text_for_parity(
        &read_make_test_replay_stderr(replay_log_dir)?,
        &path_filters,
    );

    if native_stdout == replay_stdout && native_stderr == replay_stderr {
        return Ok(());
    }

    let stdout_diff = first_output_mismatch(&native_stdout, &replay_stdout)
        .map(|(line, native, replay)| {
            format!(
                "stdout line {} differs: native='{}' fragile='{}'",
                line, native, replay
            )
        })
        .unwrap_or_else(|| "stdout differs with no line-level mismatch".to_string());
    let stderr_diff = first_output_mismatch(&native_stderr, &replay_stderr)
        .map(|(line, native, replay)| {
            format!(
                "stderr line {} differs: native='{}' fragile='{}'",
                line, native, replay
            )
        })
        .unwrap_or_else(|| "stderr differs with no line-level mismatch".to_string());

    Err(format!(
        "stdout/stderr parity mismatch: {} ; {} (native logs: {}, replay logs: {})",
        stdout_diff,
        stderr_diff,
        native_log_dir.display(),
        replay_log_dir.display()
    ))
}

#[derive(Debug)]
struct MakeTestArtifactProbeResult {
    binary: String,
    run_status: i32,
    roundtrip_status: i32,
    roundtrip_matches_output: bool,
    probe_output: Vec<u8>,
    roundtrip_output: Vec<u8>,
}

fn read_replay_source_dir_for_parity(replay_log_dir: &Path) -> Result<PathBuf, String> {
    read_source_dir_from_manifest(
        &replay_log_dir.join("make_test_commands_manifest.txt"),
        "make-test command manifest",
    )
}

fn run_shell_command_capture_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    step_name: &str,
    command_line: &str,
) -> Result<Output, String> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command_line).current_dir(source_dir);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    let output = cmd.output().map_err(|e| {
        format!(
            "failed to run shell command '{}' at {}: {}",
            command_line,
            source_dir.display(),
            e
        )
    })?;
    write_command_capture(log_dir, step_name, &output)?;
    Ok(output)
}

fn run_make_test_artifact_probe_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    scope: &str,
) -> Result<Vec<MakeTestArtifactProbeResult>, String> {
    fs::create_dir_all(log_dir).map_err(|e| {
        format!(
            "failed to create probe log dir {}: {}",
            log_dir.display(),
            e
        )
    })?;

    let mut results: Vec<MakeTestArtifactProbeResult> = Vec::new();
    for binary in ZLIB_ARTIFACT_PARITY_PROBE_BINARIES {
        let binary_id = normalize_identifier_fragment(binary);
        let probe_output_rel =
            format!(".fragile_artifact_probe_{}_{}_output.txt", scope, binary_id);
        let roundtrip_rel = format!(
            ".fragile_artifact_probe_{}_{}_roundtrip.txt",
            scope, binary_id
        );
        let run_step = format!("make_test_artifact_probe_{}_{}_run", scope, binary_id);
        let run_command = format!("./{} > {}", binary, probe_output_rel);
        let run_output =
            run_shell_command_capture_in_tree(source_dir, log_dir, &run_step, &run_command)?;
        let run_status = status_code(&run_output);

        let probe_output_path = source_dir.join(&probe_output_rel);
        let mut probe_output = if probe_output_path.exists() {
            fs::read(&probe_output_path).map_err(|e| {
                format!(
                    "failed to read probe output {}: {}",
                    probe_output_path.display(),
                    e
                )
            })?
        } else {
            Vec::new()
        };

        let mut roundtrip_status = -1;
        let mut roundtrip_matches_output = false;
        let mut roundtrip_output: Vec<u8> = Vec::new();

        if run_status == 0 {
            if probe_output.is_empty() && probe_output_path.exists() {
                probe_output = fs::read(&probe_output_path).map_err(|e| {
                    format!(
                        "failed to read probe output {}: {}",
                        probe_output_path.display(),
                        e
                    )
                })?;
            }

            let roundtrip_step =
                format!("make_test_artifact_probe_{}_{}_roundtrip", scope, binary_id);
            let roundtrip_command = format!("cat < {} > {}", probe_output_rel, roundtrip_rel);
            let roundtrip_command_output = run_shell_command_capture_in_tree(
                source_dir,
                log_dir,
                &roundtrip_step,
                &roundtrip_command,
            )?;
            roundtrip_status = status_code(&roundtrip_command_output);
            if roundtrip_status == 0 {
                let roundtrip_path = source_dir.join(&roundtrip_rel);
                if roundtrip_path.exists() {
                    roundtrip_output = fs::read(&roundtrip_path).map_err(|e| {
                        format!(
                            "failed to read roundtrip output {}: {}",
                            roundtrip_path.display(),
                            e
                        )
                    })?;
                    roundtrip_matches_output = roundtrip_output == probe_output;
                }
            }
        }

        results.push(MakeTestArtifactProbeResult {
            binary: (*binary).to_string(),
            run_status,
            roundtrip_status,
            roundtrip_matches_output,
            probe_output,
            roundtrip_output,
        });
    }

    let mut manifest = format!(
        "source_dir={}\nprobe_scope={}\nprobe_binary_count={}\n",
        source_dir.display(),
        scope,
        results.len()
    );
    for result in &results {
        manifest.push_str(&format!(
            "binary={} run_status={} roundtrip_status={} roundtrip_matches_output={} output_size={} roundtrip_size={}\n",
            result.binary,
            result.run_status,
            result.roundtrip_status,
            result.roundtrip_matches_output,
            result.probe_output.len(),
            result.roundtrip_output.len()
        ));
    }
    let manifest_path = log_dir.join(format!("make_test_artifact_probe_{}_manifest.txt", scope));
    fs::write(&manifest_path, manifest)
        .map_err(|e| format!("failed to write {}: {}", manifest_path.display(), e))?;

    Ok(results)
}

fn assert_make_test_artifact_behavior_parity(
    native_log_dir: &Path,
    replay_log_dir: &Path,
) -> Result<(), String> {
    let native_source_dir = read_native_source_dir_for_parity(native_log_dir)?;
    let replay_source_dir = read_replay_source_dir_for_parity(replay_log_dir)?;
    let native_results =
        run_make_test_artifact_probe_in_tree(&native_source_dir, native_log_dir, "native")?;
    let replay_results =
        run_make_test_artifact_probe_in_tree(&replay_source_dir, replay_log_dir, "fragile")?;

    if native_results.len() != replay_results.len() {
        return Err(format!(
            "artifact behavior parity mismatch: native probe count={} fragile probe count={} (native logs: {}, replay logs: {})",
            native_results.len(),
            replay_results.len(),
            native_log_dir.display(),
            replay_log_dir.display()
        ));
    }

    for (native, replay) in native_results.iter().zip(replay_results.iter()) {
        if native.binary != replay.binary {
            return Err(format!(
                "artifact behavior parity mismatch: probe binary order differs (native={} fragile={})",
                native.binary, replay.binary
            ));
        }
        if native.run_status != replay.run_status
            || native.roundtrip_status != replay.roundtrip_status
        {
            return Err(format!(
                "artifact behavior parity mismatch for {}: native statuses run={} roundtrip={} fragile statuses run={} roundtrip={} (native logs: {}, replay logs: {})",
                native.binary,
                native.run_status,
                native.roundtrip_status,
                replay.run_status,
                replay.roundtrip_status,
                native_log_dir.display(),
                replay_log_dir.display()
            ));
        }
        if native.run_status != 0 || native.roundtrip_status != 0 {
            return Err(format!(
                "artifact behavior parity mismatch for {}: non-zero probe status native(run={},roundtrip={}) fragile(run={},roundtrip={})",
                native.binary,
                native.run_status,
                native.roundtrip_status,
                replay.run_status,
                replay.roundtrip_status
            ));
        }
        if !native.roundtrip_matches_output || !replay.roundtrip_matches_output {
            return Err(format!(
                "artifact behavior parity mismatch for {}: roundtrip output check failed (native_match={} fragile_match={})",
                native.binary, native.roundtrip_matches_output, replay.roundtrip_matches_output
            ));
        }
        if native.probe_output != replay.probe_output {
            return Err(format!(
                "artifact behavior parity mismatch for {}: output bytes differ (native_size={} fragile_size={})",
                native.binary,
                native.probe_output.len(),
                replay.probe_output.len()
            ));
        }
        if native.roundtrip_output != replay.roundtrip_output {
            return Err(format!(
                "artifact behavior parity mismatch for {}: roundtrip output bytes differ (native_size={} fragile_size={})",
                native.binary,
                native.roundtrip_output.len(),
                replay.roundtrip_output.len()
            ));
        }
    }

    Ok(())
}

fn parse_link_units_manifest_entries(
    manifest_text: &str,
) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    for (line_no, line) in manifest_text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("output=") else {
            continue;
        };
        let Some((output, inputs)) = rest.split_once(" inputs=") else {
            return Err(format!(
                "invalid link manifest entry at line {}: {}",
                line_no + 1,
                line
            ));
        };
        let output = output.trim();
        if output.is_empty() {
            return Err(format!(
                "invalid empty link output in manifest at line {}",
                line_no + 1
            ));
        }
        let parsed_inputs: Vec<String> = inputs
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .collect();
        if parsed_inputs.is_empty() {
            return Err(format!(
                "link output {} has no inputs in manifest at line {}",
                output,
                line_no + 1
            ));
        }
        entries.push((output.to_string(), parsed_inputs));
    }
    if entries.is_empty() {
        return Err("link units manifest has no output/input entries".to_string());
    }
    Ok(entries)
}

fn resolve_manifest_path(source_dir: &Path, rel_or_abs: &str) -> PathBuf {
    let raw = Path::new(rel_or_abs);
    if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        source_dir.join(raw)
    }
}

fn read_archive_members(archive_path: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("ar")
        .arg("t")
        .arg(archive_path)
        .output()
        .map_err(|e| {
            format!(
                "failed to list archive members in {}: {}",
                archive_path.display(),
                e
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "ar t failed for {} with status {}\nstdout:\n{}\nstderr:\n{}",
            archive_path.display(),
            status_code(&output),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut members: Vec<String> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let member = line.trim();
        if member.is_empty() {
            continue;
        }
        if !members.iter().any(|existing| existing == member) {
            members.push(member.to_string());
        }
    }
    if members.is_empty() {
        return Err(format!("archive {} has no members", archive_path.display()));
    }
    Ok(members)
}

fn collect_object_targets_for_link_replay(
    source_dir: &Path,
    link_units: &[(String, Vec<String>)],
) -> Result<
    (
        BTreeSet<String>,
        std::collections::BTreeMap<String, Vec<String>>,
    ),
    String,
> {
    let mut object_targets: BTreeSet<String> = BTreeSet::new();
    let mut static_archives: BTreeSet<String> = BTreeSet::new();

    for (_, inputs) in link_units {
        for input in inputs {
            if input.ends_with(".a") {
                static_archives.insert(input.clone());
            }
        }
    }

    let mut archive_members: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for archive_rel in &static_archives {
        let archive_path = resolve_manifest_path(source_dir, archive_rel);
        if !archive_path.exists() {
            return Err(format!(
                "required static archive input {} does not exist",
                archive_path.display()
            ));
        }
        let members = read_archive_members(&archive_path)?;
        let archive_parent = archive_path.parent().unwrap_or(source_dir);
        let mut normalized_members: Vec<String> = Vec::new();
        for member in members {
            let member_path = archive_parent.join(member);
            let normalized_member = normalize_path_for_manifest(
                member_path.to_string_lossy().as_ref(),
                source_dir,
                source_dir,
            );
            if !normalized_members.iter().any(|m| m == &normalized_member) {
                normalized_members.push(normalized_member.clone());
            }
            object_targets.insert(normalized_member);
        }
        archive_members.insert(archive_rel.clone(), normalized_members);
    }

    if object_targets.is_empty() {
        return Err("link replay selected no object targets".to_string());
    }
    Ok((object_targets, archive_members))
}

struct RustRuntimeSupportInputs {
    archive_path: PathBuf,
    archive_size: u64,
    native_static_libs: Vec<String>,
}

fn parse_native_static_libs_from_rustc_stderr(stderr: &str) -> Result<Vec<String>, String> {
    for line in stderr.lines() {
        let Some((_, libs_text)) = line.split_once("native-static-libs:") else {
            continue;
        };
        let libs: Vec<String> = libs_text
            .split_whitespace()
            .filter(|token| token.starts_with("-l") || token.starts_with("-Wl,"))
            .map(ToString::to_string)
            .collect();
        if libs.is_empty() {
            return Err(
                "rustc reported native-static-libs but no link flags were parsed".to_string()
            );
        }
        return Ok(libs);
    }
    Err("rustc did not report native-static-libs in stderr".to_string())
}

fn build_rust_runtime_support_inputs(log_dir: &Path) -> Result<RustRuntimeSupportInputs, String> {
    let runtime_source_path = log_dir.join("link_runtime_support.rs");
    fs::write(
        &runtime_source_path,
        "#[no_mangle]\npub extern \"C\" fn fragile_runtime_support_anchor() {}\n",
    )
    .map_err(|e| {
        format!(
            "failed to write runtime support source {}: {}",
            runtime_source_path.display(),
            e
        )
    })?;

    let archive_path = log_dir.join("libfragile_runtime_support.a");
    let rustc_output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--crate-name")
        .arg("fragile_link_runtime_support")
        .arg(&runtime_source_path)
        .arg("-o")
        .arg(&archive_path)
        .arg("--print")
        .arg("native-static-libs")
        .output()
        .map_err(|e| {
            format!(
                "failed to run rustc for runtime support archive {}: {}",
                archive_path.display(),
                e
            )
        })?;
    write_command_capture(log_dir, "rustc_link_runtime_support", &rustc_output)?;
    if !rustc_output.status.success() {
        return Err(format!(
            "runtime support archive rustc build failed with status {} (logs: {})",
            status_code(&rustc_output),
            log_dir.display()
        ));
    }

    let archive_size = fs::metadata(&archive_path)
        .map_err(|e| {
            format!(
                "failed to stat runtime support archive {}: {}",
                archive_path.display(),
                e
            )
        })?
        .len();
    if archive_size == 0 {
        return Err(format!(
            "runtime support archive {} is empty",
            archive_path.display()
        ));
    }

    let native_static_libs =
        parse_native_static_libs_from_rustc_stderr(&String::from_utf8_lossy(&rustc_output.stderr))?;
    Ok(RustRuntimeSupportInputs {
        archive_path,
        archive_size,
        native_static_libs,
    })
}

fn transpile_objects_for_link_replay(
    source_dir: &Path,
    log_dir: &Path,
    driver_log: &str,
    object_targets: &BTreeSet<String>,
) -> Result<Vec<(String, String, PathBuf, u64)>, String> {
    let compile_units = parse_compile_units_from_cc_driver_log(driver_log, source_dir)?;
    let mut object_to_source: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for (source_rel, object_rel) in compile_units {
        object_to_source.insert(object_rel, source_rel);
    }

    let mut missing_targets: Vec<String> = Vec::new();
    for object_rel in object_targets {
        if !object_to_source.contains_key(object_rel) {
            missing_targets.push(object_rel.clone());
        }
    }
    if !missing_targets.is_empty() {
        return Err(format!(
            "missing compile units for link replay objects: {}",
            missing_targets.join(", ")
        ));
    }

    let mut transpiled_entries: Vec<(String, String, PathBuf, u64)> = Vec::new();
    for object_rel in object_targets {
        let source_rel = object_to_source
            .get(object_rel)
            .ok_or_else(|| format!("internal error: missing source for object {}", object_rel))?;
        let (command_cwd, command_tokens) =
            select_compile_command_for_unit(driver_log, source_dir, source_rel, object_rel)?;
        let source_path = source_dir.join(source_rel);
        if !source_path.exists() {
            return Err(format!(
                "selected source {} does not exist under {}",
                source_rel,
                source_dir.display()
            ));
        }

        let transpiled =
            transpile_source_with_driver_command(&source_path, &command_cwd, &command_tokens)?;
        let transpiled_rs_path = log_dir.join(format!(
            "link_{}_transpiled.rs",
            normalize_identifier_fragment(object_rel)
        ));
        fs::write(&transpiled_rs_path, transpiled).map_err(|e| {
            format!(
                "failed to write transpiled source {}: {}",
                transpiled_rs_path.display(),
                e
            )
        })?;

        let object_path = source_dir.join(object_rel);
        let compile_output = compile_rust_source_to_object(
            &transpiled_rs_path,
            &object_path,
            &crate_name_from_source(source_rel),
        )?;
        let rustc_step = rustc_replay_step_name("LINK", object_rel);
        write_command_capture(log_dir, &rustc_step, &compile_output)?;
        if !compile_output.status.success() {
            return Err(format!(
                "fragile rustc link-replay object build failed for {} with status {} (logs: {})",
                object_rel,
                status_code(&compile_output),
                log_dir.display()
            ));
        }

        let object_size = fs::metadata(&object_path)
            .map_err(|e| format!("failed to stat object {}: {}", object_path.display(), e))?
            .len();
        if object_size == 0 {
            return Err(format!(
                "link-replay object {} is empty",
                object_path.display()
            ));
        }
        transpiled_entries.push((
            source_rel.clone(),
            object_rel.clone(),
            transpiled_rs_path,
            object_size,
        ));
    }

    Ok(transpiled_entries)
}

fn rebuild_static_archives_for_link_replay(
    source_dir: &Path,
    log_dir: &Path,
    archive_members: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    for (archive_rel, members_rel) in archive_members {
        let archive_path = resolve_manifest_path(source_dir, archive_rel);
        let mut cmd = Command::new("ar");
        cmd.arg("rcs").arg(&archive_path);
        for member_rel in members_rel {
            let member_path = resolve_manifest_path(source_dir, member_rel);
            if !member_path.exists() {
                return Err(format!(
                    "archive member {} is missing for {}",
                    member_path.display(),
                    archive_path.display()
                ));
            }
            let member_size = fs::metadata(&member_path)
                .map_err(|e| {
                    format!(
                        "failed to stat archive member {}: {}",
                        member_path.display(),
                        e
                    )
                })?
                .len();
            if member_size == 0 {
                return Err(format!(
                    "archive member {} is empty for {}",
                    member_path.display(),
                    archive_path.display()
                ));
            }
            cmd.arg(member_path);
        }
        let archive_output = cmd.output().map_err(|e| {
            format!(
                "failed to rebuild archive {} with ar: {}",
                archive_path.display(),
                e
            )
        })?;
        let step = format!("ar_link_{}", normalize_identifier_fragment(archive_rel));
        write_command_capture(log_dir, &step, &archive_output)?;
        if !archive_output.status.success() {
            return Err(format!(
                "archive rebuild failed for {} with status {} (logs: {})",
                archive_rel,
                status_code(&archive_output),
                log_dir.display()
            ));
        }
        let archive_size = fs::metadata(&archive_path)
            .map_err(|e| format!("failed to stat archive {}: {}", archive_path.display(), e))?
            .len();
        if archive_size == 0 {
            return Err(format!(
                "rebuilt archive {} is empty",
                archive_path.display()
            ));
        }
    }
    Ok(())
}

fn replay_required_link_outputs(
    source_dir: &Path,
    log_dir: &Path,
    link_units: &[(String, Vec<String>)],
    runtime_support: &RustRuntimeSupportInputs,
) -> Result<Vec<(String, u64)>, String> {
    let mut relinked_outputs: Vec<(String, u64)> = Vec::new();
    for (output_rel, inputs_rel) in link_units {
        if inputs_rel.is_empty() {
            return Err(format!(
                "link output {} has no inputs for replay",
                output_rel
            ));
        }

        let mut link_cmd = Command::new("cc");
        for input_rel in inputs_rel {
            let input_path = resolve_manifest_path(source_dir, input_rel);
            if !input_path.exists() {
                return Err(format!(
                    "missing link input {} for output {}",
                    input_path.display(),
                    output_rel
                ));
            }
            let input_size = fs::metadata(&input_path)
                .map_err(|e| format!("failed to stat link input {}: {}", input_path.display(), e))?
                .len();
            if input_size == 0 {
                return Err(format!(
                    "link input {} is empty for output {}",
                    input_path.display(),
                    output_rel
                ));
            }
            link_cmd.arg(input_path);
        }
        link_cmd.arg(&runtime_support.archive_path);
        for native_lib in &runtime_support.native_static_libs {
            link_cmd.arg(native_lib);
        }

        let output_path = resolve_manifest_path(source_dir, output_rel);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create link output dir {}: {}",
                    parent.display(),
                    e
                )
            })?;
        }
        link_cmd.arg("-o").arg(&output_path).current_dir(source_dir);
        link_cmd.env("LC_ALL", "C").env("LANG", "C");
        let link_output = link_cmd.output().map_err(|e| {
            format!(
                "failed to run link replay command for {}: {}",
                output_path.display(),
                e
            )
        })?;
        let step = format!(
            "link_required_{}",
            normalize_identifier_fragment(output_rel)
        );
        write_command_capture(log_dir, &step, &link_output)?;
        if !link_output.status.success() {
            return Err(format!(
                "link replay failed for {} with status {} (logs: {})",
                output_rel,
                status_code(&link_output),
                log_dir.display()
            ));
        }

        let output_size = fs::metadata(&output_path)
            .map_err(|e| {
                format!(
                    "failed to stat linked output {}: {}",
                    output_path.display(),
                    e
                )
            })?
            .len();
        if output_size == 0 {
            return Err(format!("linked output {} is empty", output_path.display()));
        }
        relinked_outputs.push((output_rel.clone(), output_size));
    }
    Ok(relinked_outputs)
}

fn write_fragile_link_manifest(
    log_dir: &Path,
    source_dir: &Path,
    runtime_support: &RustRuntimeSupportInputs,
    transpiled_objects: &[(String, String, PathBuf, u64)],
    archive_members: &std::collections::BTreeMap<String, Vec<String>>,
    relinked_outputs: &[(String, u64)],
) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let mut manifest = format!(
        "source_dir={}\ncommit={}\nruntime_support_archive={}\nruntime_support_archive_size={}\nruntime_support_native_static_libs={}\ntranspiled_object_count={}\narchive_rebuild_count={}\nrelinked_output_count={}\n",
        source_dir.display(),
        head.trim(),
        runtime_support.archive_path.display(),
        runtime_support.archive_size,
        runtime_support.native_static_libs.join(" "),
        transpiled_objects.len(),
        archive_members.len(),
        relinked_outputs.len(),
    );

    for (source_rel, object_rel, transpiled_rs_path, object_size) in transpiled_objects {
        manifest.push_str(&format!(
            "source={} object={} transpiled_rust={} object_size={}\n",
            source_rel,
            object_rel,
            transpiled_rs_path.display(),
            object_size
        ));
    }
    for (archive_rel, members_rel) in archive_members {
        manifest.push_str(&format!(
            "archive={} members={}\n",
            archive_rel,
            members_rel.join(",")
        ));
    }
    for (output_rel, output_size) in relinked_outputs {
        manifest.push_str(&format!(
            "output={} binary_size={}\n",
            output_rel, output_size
        ));
    }

    fs::write(log_dir.join("fragile_link_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write fragile link manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn replay_required_link_binaries_from_manifests_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    let driver_log_path = log_dir.join("cc_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            driver_log_path.display(),
            e
        )
    })?;
    let link_manifest_path = log_dir.join("link_units_manifest.txt");
    let link_manifest = fs::read_to_string(&link_manifest_path)
        .map_err(|e| format!("failed to read {}: {}", link_manifest_path.display(), e))?;
    let link_units = parse_link_units_manifest_entries(&link_manifest)?;

    let parsed_outputs: BTreeSet<&str> = link_units
        .iter()
        .map(|(output, _)| output.as_str())
        .collect();
    let mut missing_outputs: Vec<String> = Vec::new();
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        if !parsed_outputs.contains(*output) {
            missing_outputs.push((*output).to_string());
        }
    }
    if !missing_outputs.is_empty() {
        return Err(format!(
            "link replay manifest is missing required outputs: {}",
            missing_outputs.join(", ")
        ));
    }

    let (object_targets, archive_members) =
        collect_object_targets_for_link_replay(source_dir, &link_units)?;
    let transpiled_objects =
        transpile_objects_for_link_replay(source_dir, log_dir, &driver_log, &object_targets)?;
    let runtime_support = build_rust_runtime_support_inputs(log_dir)?;
    rebuild_static_archives_for_link_replay(source_dir, log_dir, &archive_members)?;
    let relinked_outputs =
        replay_required_link_outputs(source_dir, log_dir, &link_units, &runtime_support)?;

    write_fragile_link_manifest(
        log_dir,
        source_dir,
        &runtime_support,
        &transpiled_objects,
        &archive_members,
        &relinked_outputs,
    )?;
    Ok(())
}

fn run_fragile_link_required_binaries_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    run_cc_driver_required_artifacts_in_tree(
        source_dir,
        log_dir,
        "all",
        ZLIB_REQUIRED_TEST_ARTIFACTS,
    )?;
    replay_required_link_binaries_from_manifests_in_tree(source_dir, log_dir)
}

fn parse_makefile_variable_list(
    makefile_text: &str,
    var_name: &str,
) -> Result<Vec<String>, String> {
    let needle = format!("{} =", var_name);
    let lines: Vec<&str> = makefile_text.lines().collect();
    let mut idx = 0usize;

    while idx < lines.len() {
        let trimmed = lines[idx].trim_start();
        if !trimmed.starts_with(&needle) {
            idx += 1;
            continue;
        }

        let mut values = String::new();
        let mut remainder = trimmed[needle.len()..].trim().to_string();
        loop {
            if remainder.ends_with('\\') {
                remainder.pop();
                values.push_str(remainder.trim());
                values.push(' ');
                idx += 1;
                if idx >= lines.len() {
                    break;
                }
                remainder = lines[idx].trim().to_string();
            } else {
                values.push_str(remainder.trim());
                break;
            }
        }

        let objects: Vec<String> = values.split_whitespace().map(ToString::to_string).collect();
        if objects.is_empty() {
            return Err(format!("{} is empty in Makefile", var_name));
        }
        return Ok(objects);
    }

    Err(format!("{} not found in Makefile", var_name))
}

fn parse_libza_object_targets(source_dir: &Path) -> Result<Vec<String>, String> {
    let makefile_path = source_dir.join("Makefile");
    let makefile_text = fs::read_to_string(&makefile_path)
        .map_err(|e| format!("failed to read {}: {}", makefile_path.display(), e))?;
    let objz = parse_makefile_variable_list(&makefile_text, "OBJZ")?;
    let objg = parse_makefile_variable_list(&makefile_text, "OBJG")?;
    let mut targets = Vec::new();
    targets.extend(objz);
    targets.extend(objg);
    Ok(targets)
}

fn write_libza_replay_plan(log_dir: &Path, source_dir: &Path) -> Result<usize, String> {
    let driver_log_path = log_dir.join("cc_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            driver_log_path.display(),
            e
        )
    })?;
    let compile_units = parse_compile_units_from_cc_driver_log(&driver_log, source_dir)?;
    let target_objects = parse_libza_object_targets(source_dir)?;

    let mut object_to_source: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for (source, object) in &compile_units {
        object_to_source.insert(object.clone(), source.clone());
    }

    let mut plan_entries: Vec<(String, String)> = Vec::new();
    let mut missing_targets: Vec<String> = Vec::new();
    for object in &target_objects {
        if let Some(source) = object_to_source.get(object) {
            plan_entries.push((source.clone(), object.clone()));
        } else {
            missing_targets.push(object.clone());
        }
    }
    if !missing_targets.is_empty() {
        return Err(format!(
            "missing compile units for libz object targets: {}",
            missing_targets.join(", ")
        ));
    }

    let mut manifest = format!(
        "source_dir={}\ncompile_units_count={}\nlibza_target_count={}\nlibza_targets={}\n",
        source_dir.display(),
        compile_units.len(),
        plan_entries.len(),
        target_objects.join(","),
    );
    for (source, object) in &plan_entries {
        manifest.push_str(&format!("source={} object={}\n", source, object));
    }
    fs::write(log_dir.join("libza_replay_plan.txt"), manifest).map_err(|e| {
        format!(
            "failed to write libza replay plan at {}: {}",
            log_dir.display(),
            e
        )
    })?;
    Ok(plan_entries.len())
}

fn run_cc_driver_libza_replay_plan_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    run_cc_driver_baseline_in_tree(source_dir, log_dir, "all")?;
    let compile_units_count = write_compile_units_manifest(log_dir, source_dir)?;
    let planned_count = write_libza_replay_plan(log_dir, source_dir)?;
    if planned_count != ZLIB_LIBZA_OBJECTS.len() {
        return Err(format!(
            "unexpected libza replay plan size: expected {} got {} (compile units total {})",
            ZLIB_LIBZA_OBJECTS.len(),
            planned_count,
            compile_units_count
        ));
    }
    Ok(())
}

fn parse_replay_plan_entries(replay_plan_text: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries: Vec<(String, String)> = Vec::new();
    for (line_no, line) in replay_plan_text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("source=") else {
            continue;
        };
        let Some((source, object)) = rest.split_once(" object=") else {
            return Err(format!(
                "invalid replay plan entry at line {}: {}",
                line_no + 1,
                line
            ));
        };
        let source = source.trim();
        let object = object.trim();
        if source.is_empty() || object.is_empty() {
            return Err(format!(
                "incomplete replay plan entry at line {}: {}",
                line_no + 1,
                line
            ));
        }
        entries.push((source.to_string(), object.to_string()));
    }

    if entries.is_empty() {
        return Err("replay plan has no source/object entries".to_string());
    }
    Ok(entries)
}

fn normalize_identifier_fragment(input: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "unit".to_string()
    } else {
        out
    }
}

fn replay_manifest_file_name(scope_name: &str) -> String {
    format!(
        "fragile_{}_manifest.txt",
        normalize_identifier_fragment(scope_name)
    )
}

fn rustc_replay_step_name(scope_name: &str, object_rel: &str) -> String {
    format!(
        "rustc_{}_{}",
        normalize_identifier_fragment(scope_name),
        normalize_identifier_fragment(object_rel)
    )
}

fn select_compile_command_for_unit(
    log_text: &str,
    source_root: &Path,
    target_source_rel: &str,
    target_object_rel: &str,
) -> Result<(PathBuf, Vec<String>), String> {
    let mut cwd = source_root.to_path_buf();
    for line in log_text.lines() {
        if let Some(rest) = line.strip_prefix("cwd=") {
            let parsed = PathBuf::from(rest.trim());
            if !parsed.as_os_str().is_empty() {
                cwd = parsed;
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("args=") else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        if !tokens.iter().any(|t| *t == "-c" || t.starts_with("-c")) {
            continue;
        }
        let Some((obj, obj_consumed_idx)) = extract_arg_value(&tokens, "-o") else {
            continue;
        };
        let Some(src) = extract_compile_source_token(&tokens, &obj, obj_consumed_idx) else {
            continue;
        };
        let source_rel = normalize_path_for_manifest(&src, &cwd, source_root);
        let object_rel = normalize_path_for_manifest(&obj, &cwd, source_root);
        if source_rel != target_source_rel || object_rel != target_object_rel {
            continue;
        }
        let command_tokens = tokens.iter().map(|t| (*t).to_string()).collect();
        return Ok((cwd.clone(), command_tokens));
    }
    Err(format!(
        "compile command for source={} object={} not found in cc_driver.log",
        target_source_rel, target_object_rel
    ))
}

fn write_fragile_replay_manifest(
    log_dir: &Path,
    source_dir: &Path,
    scope_name: &str,
    target_objects: &[&str],
    compile_units_count: usize,
    replayed_entries: &[(String, String, PathBuf, u64)],
) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let mut manifest = format!(
        "source_dir={}\ncommit={}\nreplay_scope={}\nreplay_target_count={}\nreplayed_count={}\ncompile_units_count={}\nreplay_targets={}\n",
        source_dir.display(),
        head.trim(),
        scope_name,
        target_objects.len(),
        replayed_entries.len(),
        compile_units_count,
        target_objects.join(","),
    );

    for (source_rel, object_rel, transpiled_rs_path, object_size) in replayed_entries {
        manifest.push_str(&format!(
            "source={} object={} transpiled_rust={} object_size={}\n",
            source_rel,
            object_rel,
            transpiled_rs_path.display(),
            object_size
        ));
    }

    fs::write(
        log_dir.join(replay_manifest_file_name(scope_name)),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write fragile replay manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_fragile_replay_for_targets_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    scope_name: &str,
    target_objects: &[&str],
) -> Result<(), String> {
    run_cc_driver_libza_replay_plan_in_tree(source_dir, log_dir)?;

    let driver_log_path = log_dir.join("cc_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            driver_log_path.display(),
            e
        )
    })?;
    let replay_plan_path = log_dir.join("libza_replay_plan.txt");
    let replay_plan = fs::read_to_string(&replay_plan_path)
        .map_err(|e| format!("failed to read {}: {}", replay_plan_path.display(), e))?;

    let compile_units_count =
        parse_compile_units_from_cc_driver_log(&driver_log, source_dir)?.len();
    let replay_entries = parse_replay_plan_entries(&replay_plan)?;
    let target_set: BTreeSet<&str> = target_objects.iter().copied().collect();
    let selected_entries: Vec<(String, String)> = replay_entries
        .into_iter()
        .filter(|(_, object_rel)| target_set.contains(object_rel.as_str()))
        .collect();

    let selected_objects: BTreeSet<String> = selected_entries
        .iter()
        .map(|(_, object)| object.clone())
        .collect();
    let mut missing_targets: Vec<String> = Vec::new();
    for object in target_objects {
        if !selected_objects.contains(*object) {
            missing_targets.push((*object).to_string());
        }
    }
    if !missing_targets.is_empty() {
        return Err(format!(
            "replay plan is missing targets for {}: {}",
            scope_name,
            missing_targets.join(", ")
        ));
    }

    let mut replayed_manifest_entries: Vec<(String, String, PathBuf, u64)> = Vec::new();
    for (source_rel, object_rel) in selected_entries {
        let (command_cwd, command_tokens) =
            select_compile_command_for_unit(&driver_log, source_dir, &source_rel, &object_rel)?;
        let source_path = source_dir.join(&source_rel);
        if !source_path.exists() {
            return Err(format!(
                "selected source {} does not exist under {}",
                source_rel,
                source_dir.display()
            ));
        }

        let transpiled =
            transpile_source_with_driver_command(&source_path, &command_cwd, &command_tokens)?;
        let transpiled_rs_path = log_dir.join(format!(
            "{}_{}_transpiled.rs",
            normalize_identifier_fragment(scope_name),
            normalize_identifier_fragment(&object_rel)
        ));
        fs::write(&transpiled_rs_path, transpiled).map_err(|e| {
            format!(
                "failed to write transpiled source {}: {}",
                transpiled_rs_path.display(),
                e
            )
        })?;

        let object_path = source_dir.join(&object_rel);
        let compile_output = compile_rust_source_to_object(
            &transpiled_rs_path,
            &object_path,
            &crate_name_from_source(&source_rel),
        )?;
        let rustc_step = rustc_replay_step_name(scope_name, &object_rel);
        write_command_capture(log_dir, &rustc_step, &compile_output)?;
        if !compile_output.status.success() {
            return Err(format!(
                "fragile rustc object build failed for {} with status {} (logs: {})",
                object_rel,
                status_code(&compile_output),
                log_dir.display()
            ));
        }

        let object_size = fs::metadata(&object_path)
            .map_err(|e| format!("failed to stat object {}: {}", object_path.display(), e))?
            .len();
        if object_size == 0 {
            return Err(format!(
                "generated object {} is empty",
                object_path.display()
            ));
        }
        replayed_manifest_entries.push((source_rel, object_rel, transpiled_rs_path, object_size));
    }

    write_fragile_replay_manifest(
        log_dir,
        source_dir,
        scope_name,
        target_objects,
        compile_units_count,
        &replayed_manifest_entries,
    )?;
    Ok(())
}

fn run_fragile_objz_replay_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    run_fragile_replay_for_targets_in_tree(source_dir, log_dir, "OBJZ", ZLIB_OBJZ_OBJECTS)
}

fn run_fragile_objg_replay_in_tree(source_dir: &Path, log_dir: &Path) -> Result<(), String> {
    run_fragile_replay_for_targets_in_tree(source_dir, log_dir, "OBJG", ZLIB_OBJG_OBJECTS)
}

fn select_compile_command_for_source(
    log_text: &str,
    source_root: &Path,
    source_file_name: &str,
) -> Result<(String, String, PathBuf, Vec<String>), String> {
    let mut cwd = source_root.to_path_buf();
    for line in log_text.lines() {
        if let Some(rest) = line.strip_prefix("cwd=") {
            let parsed = PathBuf::from(rest.trim());
            if !parsed.as_os_str().is_empty() {
                cwd = parsed;
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("args=") else {
            continue;
        };
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        if !tokens.iter().any(|t| *t == "-c" || t.starts_with("-c")) {
            continue;
        }
        let Some((obj, obj_consumed_idx)) = extract_arg_value(&tokens, "-o") else {
            continue;
        };
        let Some(src) = extract_compile_source_token(&tokens, &obj, obj_consumed_idx) else {
            continue;
        };
        let source = normalize_path_for_manifest(&src, &cwd, source_root);
        let source_basename = Path::new(&source).file_name().and_then(|s| s.to_str());
        if source_basename != Some(source_file_name) {
            continue;
        }
        let object = normalize_path_for_manifest(&obj, &cwd, source_root);
        let command_tokens = tokens.iter().map(|t| (*t).to_string()).collect();
        return Ok((source, object, cwd.clone(), command_tokens));
    }

    Err(format!(
        "compile unit for source {} not found in cc_driver.log",
        source_file_name
    ))
}

fn resolve_flag_path(path_arg: &str, cwd: &Path) -> String {
    let raw = Path::new(path_arg);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        cwd.join(raw)
    };
    lexical_normalize(&joined).to_string_lossy().to_string()
}

fn extract_parser_args_from_driver_tokens(
    command_tokens: &[String],
    cwd: &Path,
) -> (Vec<String>, Vec<String>) {
    let mut include_paths: BTreeSet<String> = BTreeSet::new();
    let mut defines: BTreeSet<String> = BTreeSet::new();
    let mut idx = 0usize;

    while idx < command_tokens.len() {
        let tok = &command_tokens[idx];
        if tok == "-I" {
            if let Some(next) = command_tokens.get(idx + 1) {
                include_paths.insert(resolve_flag_path(next, cwd));
            }
            idx += 2;
            continue;
        }
        if let Some(rest) = tok.strip_prefix("-I") {
            if !rest.is_empty() {
                include_paths.insert(resolve_flag_path(rest, cwd));
            }
            idx += 1;
            continue;
        }
        if tok == "-isystem" {
            if let Some(next) = command_tokens.get(idx + 1) {
                include_paths.insert(resolve_flag_path(next, cwd));
            }
            idx += 2;
            continue;
        }
        if let Some(rest) = tok.strip_prefix("-isystem") {
            if !rest.is_empty() {
                include_paths.insert(resolve_flag_path(rest, cwd));
            }
            idx += 1;
            continue;
        }
        if tok == "-D" {
            if let Some(next) = command_tokens.get(idx + 1) {
                defines.insert(next.to_string());
            }
            idx += 2;
            continue;
        }
        if let Some(rest) = tok.strip_prefix("-D") {
            if !rest.is_empty() {
                defines.insert(rest.to_string());
            }
            idx += 1;
            continue;
        }
        idx += 1;
    }

    (
        include_paths.into_iter().collect(),
        defines.into_iter().collect(),
    )
}

fn transpile_source_with_driver_command(
    source_path: &Path,
    command_cwd: &Path,
    command_tokens: &[String],
) -> Result<String, String> {
    let (include_paths, defines) =
        extract_parser_args_from_driver_tokens(command_tokens, command_cwd);
    let language = match source_path.extension().and_then(|ext| ext.to_str()) {
        Some("c") => ParserLanguage::C,
        _ => ParserLanguage::Cpp,
    };
    let parser = ClangParser::with_paths_defines_and_language(include_paths, defines, language)
        .map_err(|e| {
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

fn crate_name_from_source(source_rel: &str) -> String {
    let mut crate_name = String::from("zlib_");
    let mut prev_was_sep = false;
    for ch in source_rel.chars() {
        if ch.is_ascii_alphanumeric() {
            crate_name.push(ch.to_ascii_lowercase());
            prev_was_sep = false;
        } else if !prev_was_sep {
            crate_name.push('_');
            prev_was_sep = true;
        }
    }
    while crate_name.ends_with('_') {
        crate_name.pop();
    }
    if crate_name == "zlib" || crate_name == "zlib_" {
        "zlib_unit".to_string()
    } else {
        crate_name
    }
}

fn compile_rust_source_to_object(
    rust_source_path: &Path,
    object_path: &Path,
    crate_name: &str,
) -> Result<Output, String> {
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create object output dir {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("lib")
        .arg("--crate-name")
        .arg(crate_name)
        .arg("--emit=obj")
        .arg("-C")
        .arg("overflow-checks=off")
        .arg("-A")
        .arg("warnings")
        .arg(rust_source_path)
        .arg("-o")
        .arg(object_path)
        .output()
        .map_err(|e| format!("failed to run rustc for {}: {}", object_path.display(), e))
}

fn write_fragile_object_manifest(
    log_dir: &Path,
    source_dir: &Path,
    make_target: &str,
    source_rel: &str,
    object_rel: &str,
    transpiled_rs_path: &Path,
    compile_unit_count: usize,
) -> Result<(), String> {
    let object_path = source_dir.join(object_rel);
    let object_size = fs::metadata(&object_path)
        .map_err(|e| format!("failed to stat object {}: {}", object_path.display(), e))?
        .len();
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let manifest = format!(
        "source_dir={}\ncommit={}\nmake_target={}\nsource={}\nobject={}\ntranspiled_rust={}\ncompile_units_count={}\nobject_size={}\n",
        source_dir.display(),
        head.trim(),
        make_target,
        source_rel,
        object_rel,
        transpiled_rs_path.display(),
        compile_unit_count,
        object_size,
    );
    fs::write(log_dir.join("fragile_object_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write fragile object manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn run_fragile_single_object_in_tree(
    source_dir: &Path,
    log_dir: &Path,
    source_file_name: &str,
    make_target: &str,
) -> Result<(), String> {
    run_cc_driver_baseline_in_tree(source_dir, log_dir, make_target)?;
    let compile_unit_count = write_compile_units_manifest(log_dir, source_dir)?;

    let driver_log_path = log_dir.join("cc_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path).map_err(|e| {
        format!(
            "failed to read cc-driver invocation log {}: {}",
            driver_log_path.display(),
            e
        )
    })?;
    let (source_rel, object_rel, command_cwd, command_tokens) =
        select_compile_command_for_source(&driver_log, source_dir, source_file_name)?;
    let source_path = source_dir.join(&source_rel);
    if !source_path.exists() {
        return Err(format!(
            "selected source {} does not exist under {}",
            source_rel,
            source_dir.display()
        ));
    }

    let transpiled =
        transpile_source_with_driver_command(&source_path, &command_cwd, &command_tokens)?;
    let stem = Path::new(source_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unit");
    let transpiled_rs_path = log_dir.join(format!("{}_transpiled.rs", stem));
    fs::write(&transpiled_rs_path, transpiled).map_err(|e| {
        format!(
            "failed to write transpiled source {}: {}",
            transpiled_rs_path.display(),
            e
        )
    })?;

    let object_path = source_dir.join(&object_rel);
    let compile_output = compile_rust_source_to_object(
        &transpiled_rs_path,
        &object_path,
        &crate_name_from_source(&source_rel),
    )?;
    write_command_capture(log_dir, "rustc_object", &compile_output)?;
    if !compile_output.status.success() {
        return Err(format!(
            "fragile rustc object build failed with status {} (logs: {})",
            status_code(&compile_output),
            log_dir.display()
        ));
    }

    let object_size = fs::metadata(&object_path)
        .map_err(|e| format!("failed to stat object {}: {}", object_path.display(), e))?
        .len();
    if object_size == 0 {
        return Err(format!(
            "generated object {} is empty",
            object_path.display()
        ));
    }

    write_fragile_object_manifest(
        log_dir,
        source_dir,
        make_target,
        &source_rel,
        &object_rel,
        &transpiled_rs_path,
        compile_unit_count,
    )?;
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
    compile_unit_count: usize,
) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let manifest = format!(
        "source_dir={}\ncommit={}\nmake_target={}\nrequired_artifacts={}\ncompile_units_count={}\n",
        source_dir.display(),
        head.trim(),
        make_target,
        required_artifacts.join(","),
        compile_unit_count
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
    let compile_unit_count = write_compile_units_manifest(log_dir, source_dir)?;
    write_link_units_manifest(log_dir, source_dir, ZLIB_REQUIRED_LINK_OUTPUTS)?;
    write_artifact_manifest(
        log_dir,
        source_dir,
        make_target,
        required_artifacts,
        compile_unit_count,
    )?;
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

fn run_zlib_libza_replay_plan_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_LIBZA_REPLAY_PLAN_BASELINE_DIR);
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
            "libza-replay-plan worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_cc_driver_libza_replay_plan_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_zlib_fragile_adler32_object_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_FRAGILE_ADLER32_OBJECT_BASELINE_DIR);
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
            "fragile-adler32 worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_fragile_single_object_in_tree(&worktree_dir, &log_dir, "adler32.c", "adler32.o")?;
    Ok(log_dir)
}

fn run_zlib_fragile_objz_objects_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_FRAGILE_OBJZ_OBJECTS_BASELINE_DIR);
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
            "fragile-objz worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_fragile_objz_replay_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_zlib_fragile_objg_objects_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_FRAGILE_OBJG_OBJECTS_BASELINE_DIR);
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
            "fragile-objg worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_fragile_objg_replay_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_zlib_fragile_link_required_binaries_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_FRAGILE_LINK_REQUIRED_BINARIES_BASELINE_DIR);
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
            "fragile-link-required-binaries worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_fragile_link_required_binaries_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_zlib_make_test_command_plan_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_MAKE_TEST_COMMAND_PLAN_BASELINE_DIR);
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
            "make-test-command-plan worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_make_test_command_plan_in_tree(&worktree_dir, &log_dir)?;
    Ok(log_dir)
}

fn run_zlib_make_test_command_subset_replay_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_zlib_checkout()?;
    let baseline_root = PathBuf::from(ZLIB_MAKE_TEST_REPLAY_BASELINE_DIR);
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
            "make-test-replay worktree expected commit {} but got {}",
            ZLIB_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("driver_logs");
    run_make_test_command_subset_replay_in_tree(&worktree_dir, &log_dir)?;
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

example: tiny.o libz.a
	$(CC) tiny.o libz.a -o example
minigzip: tiny.o libz.a
	$(CC) tiny.o libz.a -o minigzip
examplesh: tiny.o libz.a
	$(CC) tiny.o libz.a -o examplesh
minigzipsh: tiny.o libz.a
	$(CC) tiny.o libz.a -o minigzipsh
example64: tiny.o libz.a
	$(CC) tiny.o libz.a -o example64

test:
	TMPST=tmpst_$$; \
	if echo hello world | ./minigzip | ./minigzip -d && ./example $$TMPST ; then \
	  echo "*** zlib test OK ***"; \
	else \
	  echo "*** zlib test FAILED ***"; false; \
	fi
	TMPSH=tmpsh_$$; \
	if echo hello world | ./minigzipsh | ./minigzipsh -d && ./examplesh $$TMPSH; then \
	  echo "*** zlib shared test OK ***"; \
	else \
	  echo "*** zlib shared test FAILED ***"; false; \
	fi
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

example: tiny.o libz.a
	$(CC) tiny.o libz.a -o example
minigzip: tiny.o libz.a
	$(CC) tiny.o libz.a -o minigzip
examplesh: tiny.o libz.a
	$(CC) tiny.o libz.a -o examplesh
minigzipsh: tiny.o libz.a
	$(CC) tiny.o libz.a -o minigzipsh
example64: tiny.o libz.a
	$(CC) tiny.o libz.a -o example64
minigzip64: tiny.o libz.a
	$(CC) tiny.o libz.a -o minigzip64

test:
	TMPST=tmpst_$$; \
	if echo hello world | ./minigzip | ./minigzip -d && ./example $$TMPST ; then \
	  echo "*** zlib test OK ***"; \
	else \
	  echo "*** zlib test FAILED ***"; false; \
	fi
	TMPSH=tmpsh_$$; \
	if echo hello world | ./minigzipsh | ./minigzipsh -d && ./examplesh $$TMPSH; then \
	  echo "*** zlib shared test OK ***"; \
	else \
	  echo "*** zlib shared test FAILED ***"; false; \
	fi
	TMP64=tmp64_$$; \
	if echo hello world | ./minigzip64 | ./minigzip64 -d && ./example64 $$TMP64 ; then \
	  echo "*** zlib 64-bit test OK ***"; \
	else \
	  echo "*** zlib 64-bit test FAILED ***"; false; \
	fi
"#
    };
    fs::write(project_dir.join("Makefile"), makefile)
        .map_err(|e| format!("failed to write Makefile: {}", e))?;

    Ok(project_dir)
}

fn rewrite_local_makefile_test_target(project_dir: &Path, test_target: &str) -> Result<(), String> {
    let makefile_path = project_dir.join("Makefile");
    let makefile_text = fs::read_to_string(&makefile_path)
        .map_err(|e| format!("failed to read Makefile: {}", e))?;
    let (prefix, _) = makefile_text
        .split_once("\ntest:\n")
        .ok_or_else(|| format!("Makefile at {} has no test target", makefile_path.display()))?;
    let rewritten = format!("{}\n{}\n", prefix.trim_end(), test_target.trim_end());
    fs::write(&makefile_path, rewritten).map_err(|e| {
        format!(
            "failed to rewrite Makefile {}: {}",
            makefile_path.display(),
            e
        )
    })?;
    Ok(())
}

fn write_local_tiny_marker_program(project_dir: &Path, native_marker: bool) -> Result<(), String> {
    let source = if native_marker {
        r#"#include <unistd.h>
int main(void) {
    static const char kMarker[] = "native-marker\n";
    return write(1, kMarker, sizeof(kMarker) - 1) < 0 ? 1 : 0;
}
"#
    } else {
        r#"#include <unistd.h>
int main(void) {
    static const char kMarker[] = "fragile-marker\n";
    return write(1, kMarker, sizeof(kMarker) - 1) < 0 ? 1 : 0;
}
"#
    };
    let tiny_c_path = project_dir.join("tiny.c");
    fs::write(&tiny_c_path, source)
        .map_err(|e| format!("failed to write {}: {}", tiny_c_path.display(), e))
}

fn write_local_replay_source_manifest_for_parity(
    replay_log_dir: &Path,
    source_dir: &Path,
) -> Result<(), String> {
    fs::create_dir_all(replay_log_dir)
        .map_err(|e| format!("failed to create {}: {}", replay_log_dir.display(), e))?;
    let manifest = format!(
        "source_dir={}\nmake_test_command_count=1\nrequired_binaries={}\ncommand=true\n",
        source_dir.display(),
        ZLIB_REQUIRED_LINK_OUTPUTS.join(",")
    );
    fs::write(
        replay_log_dir.join("make_test_commands_manifest.txt"),
        manifest,
    )
    .map_err(|e| {
        format!(
            "failed to write replay source manifest at {}: {}",
            replay_log_dir.display(),
            e
        )
    })
}

fn create_local_fragile_object_project(base_dir: &Path) -> Result<PathBuf, String> {
    let project_dir = base_dir.join("fragile_object_project");
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

    fs::write(
        project_dir.join("adler32.c"),
        "unsigned long adler32(unsigned long adler, const unsigned char* buf, int len) {\n    unsigned long sum = adler;\n    for (int i = 0; i < len; ++i) sum += buf[i];\n    return sum;\n}\n",
    )
    .map_err(|e| format!("failed to write adler32.c: {}", e))?;
    fs::write(
        project_dir.join("tiny.c"),
        "int tiny_answer(void) { return 7; }\n",
    )
    .map_err(|e| format!("failed to write tiny.c: {}", e))?;

    let makefile = r#"CC ?= cc
adler32.o: adler32.c
	$(CC) -c adler32.c -o adler32.o

tiny.o: tiny.c
	$(CC) -c tiny.c -o tiny.o
"#;
    fs::write(project_dir.join("Makefile"), makefile)
        .map_err(|e| format!("failed to write Makefile: {}", e))?;
    Ok(project_dir)
}

fn create_local_libza_replay_plan_project(
    base_dir: &Path,
    skip_cc_compile_for_object: Option<&str>,
) -> Result<PathBuf, String> {
    let project_dir = base_dir.join("libza_replay_plan_project");
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

    for (idx, object) in ZLIB_LIBZA_OBJECTS.iter().enumerate() {
        let source = object.trim_end_matches(".o").to_string() + ".c";
        let func_name = source.replace('.', "_");
        let source_text = format!("int {}(void) {{ return {}; }}\n", func_name, idx + 1);
        fs::write(project_dir.join(source), source_text)
            .map_err(|e| format!("failed to write source for {}: {}", object, e))?;
    }

    let mut makefile = format!(
        "CC ?= cc\nOBJZ = {}\nOBJG = {}\nOBJS = $(OBJZ) $(OBJG)\nall: $(OBJS) libz.a\n\nlibz.a: $(OBJS)\n\tar rcs libz.a $(OBJS)\n\n%.o: %.c\n\t$(CC) -c $< -o $@\n",
        ZLIB_OBJZ_OBJECTS.join(" "),
        ZLIB_OBJG_OBJECTS.join(" ")
    );

    if let Some(skip_object) = skip_cc_compile_for_object {
        makefile.push_str(&format!(
            "\n{}:\n\t@printf 'skipped cc compile for {}\\n' > {}\n",
            skip_object, skip_object, skip_object
        ));
    }

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
fn test_parse_compile_units_from_driver_log_normalizes_and_deduplicates() {
    let root = unique_temp_dir("zlib_compile_units_parse");
    let worktree = root.join("worktree");
    let subdir = worktree.join("sub");
    fs::create_dir_all(&subdir).expect("failed to create test dirs");

    let log = format!(
        "cwd={}\n\
args=cc -c adler32.c -o adler32.o\n\
args=cc adler32.o -o example\n\
cwd={}\n\
args=cc -c ../trees.c -o trees.o\n\
args=cc -c ../adler32.c -o ../adler32.o\n",
        worktree.display(),
        subdir.display()
    );

    let units = parse_compile_units_from_cc_driver_log(&log, &worktree)
        .expect("compile unit parse should succeed");
    assert_eq!(
        units,
        vec![
            ("adler32.c".to_string(), "adler32.o".to_string()),
            ("trees.c".to_string(), "sub/trees.o".to_string()),
        ]
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_compile_units_from_driver_log_rejects_missing_compile_commands() {
    let root = unique_temp_dir("zlib_compile_units_empty");
    let worktree = root.join("worktree");
    fs::create_dir_all(&worktree).expect("failed to create test dir");

    let log = format!("cwd={}\nargs=cc adler32.o -o example\n", worktree.display());
    let err = parse_compile_units_from_cc_driver_log(&log, &worktree)
        .expect_err("expected parse to fail when no compile commands exist");
    assert!(
        err.contains("no compile units found"),
        "unexpected parse error: {}",
        err
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_link_units_from_driver_log_normalizes_and_deduplicates() {
    let root = unique_temp_dir("zlib_link_units_parse");
    let worktree = root.join("worktree");
    let subdir = worktree.join("sub");
    fs::create_dir_all(&subdir).expect("failed to create test dirs");

    let log = format!(
        "cwd={}\n\
args=cc -c adler32.c -o adler32.o\n\
args=cc adler32.o libz.a -o example\n\
args=cc adler32.o -o example\n\
cwd={}\n\
args=cc ../tiny.o ../libz.a -o ../minigzip\n",
        worktree.display(),
        subdir.display()
    );

    let units = parse_link_units_from_cc_driver_log(&log, &worktree)
        .expect("link unit parse should succeed");
    assert_eq!(
        units,
        vec![
            (
                "example".to_string(),
                vec!["adler32.o".to_string(), "libz.a".to_string()],
            ),
            (
                "minigzip".to_string(),
                vec!["tiny.o".to_string(), "libz.a".to_string()],
            ),
        ]
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_link_units_from_driver_log_rejects_missing_link_commands() {
    let root = unique_temp_dir("zlib_link_units_empty");
    let worktree = root.join("worktree");
    fs::create_dir_all(&worktree).expect("failed to create test dir");

    let log = format!(
        "cwd={}\nargs=cc -c adler32.c -o adler32.o\n",
        worktree.display()
    );
    let err = parse_link_units_from_cc_driver_log(&log, &worktree)
        .expect_err("expected parse to fail when no link commands exist");
    assert!(
        err.contains("no link units found"),
        "unexpected parse error: {}",
        err
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_write_link_units_manifest_detects_missing_required_outputs() {
    let root = unique_temp_dir("zlib_link_units_missing_output");
    let worktree = root.join("worktree");
    let log_dir = root.join("logs");
    fs::create_dir_all(&worktree).expect("failed to create test dir");
    fs::create_dir_all(&log_dir).expect("failed to create log dir");

    fs::write(
        log_dir.join("cc_driver.log"),
        format!("cwd={}\nargs=cc tiny.o -o example\n", worktree.display()),
    )
    .expect("failed to write cc_driver.log");

    let err = write_link_units_manifest(&log_dir, &worktree, &["example", "minigzip"])
        .expect_err("expected missing required link output to fail");
    assert!(
        err.contains("missing link units for required outputs"),
        "unexpected error: {}",
        err
    );
    assert!(
        err.contains("minigzip"),
        "missing output should mention minigzip: {}",
        err
    );
    assert!(
        !log_dir.join("link_units_manifest.txt").exists(),
        "link manifest should not be written on missing-output failure"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_make_test_commands_from_dry_run_normalizes_and_validates_coverage() {
    let dry_run_stdout = r#"
gcc -O3 -o example example.o -L. libz.a
TMPST=tmpst_$; \
if echo hello world |  ./minigzip | ./minigzip -d && ./example $TMPST ; then \
  echo '*** zlib test OK ***'; \
else \
  echo '*** zlib test FAILED ***'; false; \
fi
LD_LIBRARY_PATH=`pwd`: ; export LD_LIBRARY_PATH; \
TMPSH=tmpsh_$; \
if echo hello world | ./minigzipsh | ./minigzipsh -d && ./examplesh $TMPSH; then \
  echo '*** zlib shared test OK ***'; \
else \
  echo '*** zlib shared test FAILED ***'; false; \
fi
TMP64=tmp64_$; \
if echo hello world | ./minigzip64 | ./minigzip64 -d && ./example64 $TMP64 ; then \
  echo '*** zlib 64-bit test OK ***'; \
else \
  echo '*** zlib 64-bit test FAILED ***'; false; \
fi
"#;

    let commands =
        parse_make_test_commands_from_dry_run(dry_run_stdout, ZLIB_REQUIRED_LINK_OUTPUTS)
            .expect("make test command parse should succeed");
    assert_eq!(
        commands.len(),
        3,
        "expected one normalized command per test variant: {:?}",
        commands
    );
    assert!(commands.iter().any(|c| c.contains("./example")));
    assert!(commands.iter().any(|c| c.contains("./examplesh")));
    assert!(commands.iter().any(|c| c.contains("./example64")));
}

#[test]
fn test_parse_make_test_commands_from_dry_run_reports_missing_required_binary_invocations() {
    let dry_run_stdout = r#"
TMPST=tmpst_$; \
if echo hello world | ./minigzip | ./minigzip -d && ./example $TMPST ; then \
  echo ok; \
else \
  false; \
fi
TMPSH=tmpsh_$; \
if echo hello world | ./minigzipsh | ./minigzipsh -d && ./examplesh $TMPSH ; then \
  echo ok; \
else \
  false; \
fi
"#;
    let err = parse_make_test_commands_from_dry_run(dry_run_stdout, ZLIB_REQUIRED_LINK_OUTPUTS)
        .expect_err("expected missing required binary invocation failure");
    assert!(
        err.contains("make test command plan missing required binary invocations"),
        "unexpected error: {}",
        err
    );
    assert!(
        err.contains("example64"),
        "missing 64-bit command coverage should be reported: {}",
        err
    );
    assert!(
        err.contains("minigzip64"),
        "missing 64-bit command coverage should be reported: {}",
        err
    );
}

#[test]
fn test_make_test_command_plan_local_fixture_success() {
    let root = unique_temp_dir("zlib_make_test_command_plan_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, false)
        .expect("failed to create required-artifacts project");
    let log_dir = root.join("logs");
    run_make_test_command_plan_in_tree(&project_dir, &log_dir)
        .expect("make-test command plan generation should succeed");

    assert_eq!(
        fs::read_to_string(log_dir.join("make_test_dryrun.status"))
            .expect("failed to read make_test_dryrun.status")
            .trim(),
        "0"
    );
    let manifest = fs::read_to_string(log_dir.join("make_test_commands_manifest.txt"))
        .expect("failed to read make_test_commands_manifest.txt");
    assert!(
        manifest.contains("make_test_command_count=3"),
        "manifest should include normalized command count: {}",
        manifest
    );
    for binary in ZLIB_REQUIRED_LINK_OUTPUTS {
        assert!(
            manifest.contains(&format!("./{}", binary)),
            "manifest should include binary invocation for {}: {}",
            binary,
            manifest
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_plan_local_fixture_detects_missing_coverage() {
    let root = unique_temp_dir("zlib_make_test_command_plan_missing_coverage");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, false)
        .expect("failed to create required-artifacts project");
    let makefile_path = project_dir.join("Makefile");
    let makefile_text = fs::read_to_string(&makefile_path).expect("failed to read Makefile");
    let mut rewritten: Vec<String> = Vec::new();
    let mut skipping_64_block = false;
    for line in makefile_text.lines() {
        if line.contains("TMP64=tmp64_$$; \\") {
            skipping_64_block = true;
            continue;
        }
        if skipping_64_block {
            if line.trim() == "fi" {
                skipping_64_block = false;
            }
            continue;
        }
        rewritten.push(line.to_string());
    }
    fs::write(&makefile_path, format!("{}\n", rewritten.join("\n")))
        .expect("failed to rewrite Makefile without 64-bit test command block");

    let log_dir = root.join("logs");
    let err = run_make_test_command_plan_in_tree(&project_dir, &log_dir)
        .expect_err("make-test command plan should fail with missing required coverage");
    assert!(
        err.contains("make test command plan missing required binary invocations"),
        "unexpected error: {}",
        err
    );
    assert!(
        err.contains("minigzip64"),
        "missing binary coverage should be reported: {}",
        err
    );
    assert!(
        !log_dir.join("make_test_commands_manifest.txt").exists(),
        "make-test command manifest should not be written on coverage failure"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_subset_replay_local_fixture_success() {
    let root = unique_temp_dir("zlib_make_test_replay_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, false)
        .expect("failed to create required-artifacts project");
    rewrite_local_makefile_test_target(
        &project_dir,
        r#"test:
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite local fixture test target for replay success");
    let log_dir = root.join("logs");
    let err = run_make_test_command_subset_replay_in_tree(&project_dir, &log_dir)
        .expect_err("strict make-test replay should fail at link step");
    assert!(
        err.contains("link replay failed for"),
        "unexpected strict make-test replay failure: {}",
        err
    );
    assert!(
        !log_dir.join("make_test_commands_manifest.txt").exists(),
        "make-test command plan should not be written when strict link replay fails first"
    );
    assert!(
        !log_dir.join("make_test_replay_manifest.txt").exists(),
        "make-test replay manifest should not be written when strict link replay fails first"
    );
    let first_link_step = format!(
        "link_required_{}",
        normalize_identifier_fragment(ZLIB_REQUIRED_LINK_OUTPUTS[0])
    );
    assert_ne!(
        fs::read_to_string(log_dir.join(format!("{}.status", first_link_step)))
            .expect("failed to read strict link replay status")
            .trim(),
        "0",
        "first strict link replay step should fail in current baseline"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_subset_replay_reports_failing_command() {
    let root = unique_temp_dir("zlib_make_test_replay_failure");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, false)
        .expect("failed to create required-artifacts project");
    rewrite_local_makefile_test_target(
        &project_dir,
        r#"test:
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64 && false
"#,
    )
    .expect("failed to rewrite local fixture test target for replay failure");

    let log_dir = root.join("logs");
    let err = run_make_test_command_subset_replay_in_tree(&project_dir, &log_dir)
        .expect_err("strict make-test replay should fail at link step");
    assert!(
        err.contains("link replay failed for"),
        "unexpected replay error: {}",
        err
    );

    let failing_step = format!(
        "link_required_{}",
        normalize_identifier_fragment(ZLIB_REQUIRED_LINK_OUTPUTS[0])
    );
    let failing_status =
        fs::read_to_string(log_dir.join(format!("{}.status", failing_step)))
            .expect("failed to read strict link replay status");
    assert_ne!(
        failing_status.trim(),
        "0",
        "strict link replay step should have non-zero status"
    );
    assert!(
        !log_dir.join("make_test_replay_manifest.txt").exists(),
        "make-test replay manifest should not be written when strict link replay fails first"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_exit_status_parity_local_fixture_success() {
    let root = unique_temp_dir("zlib_make_test_exit_status_parity_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let replay_project = create_local_required_artifacts_project(&root.join("replay"), false)
        .expect("failed to create replay required-artifacts project");
    rewrite_local_makefile_test_target(
        &replay_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite replay fixture test target for parity success");

    let replay_log_dir = root.join("replay_logs");
    let replay_err = run_make_test_command_subset_replay_in_tree(&replay_project, &replay_log_dir)
        .expect_err("strict make-test replay should fail at link step for parity success case");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected strict replay error: {}",
        replay_err
    );

    let native_project = create_local_required_artifacts_project(&root.join("native"), false)
        .expect("failed to create native required-artifacts project");
    rewrite_local_makefile_test_target(
        &native_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite native fixture test target for parity success");
    let native_log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&native_project, &native_log_dir)
        .expect("native baseline should succeed for parity success case");

    let parity_err = assert_make_test_exit_status_parity(&native_log_dir, &replay_log_dir)
        .expect_err("parity evaluation should report missing replay make-test manifest");
    assert!(
        parity_err.contains("make_test_commands_manifest.txt"),
        "unexpected parity error after strict link failure: {}",
        parity_err
    );
    assert_eq!(
        read_native_make_test_exit_status(&native_log_dir).expect("failed to read native status"),
        0
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_exit_status_parity_local_fixture_reports_mismatch() {
    let root = unique_temp_dir("zlib_make_test_exit_status_parity_mismatch");
    fs::create_dir_all(&root).expect("failed to create test root");

    let replay_project = create_local_required_artifacts_project(&root.join("replay"), false)
        .expect("failed to create replay required-artifacts project");
    rewrite_local_makefile_test_target(
        &replay_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example && false
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite replay fixture test target for parity mismatch");

    let replay_log_dir = root.join("replay_logs");
    let replay_result =
        run_make_test_command_subset_replay_in_tree(&replay_project, &replay_log_dir);
    assert!(
        replay_result.is_err(),
        "strict make-test replay should fail at link step for parity mismatch case"
    );
    let replay_err = replay_result.expect_err("strict replay failure should be present");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected strict replay error: {}",
        replay_err
    );

    let native_project = create_local_required_artifacts_project(&root.join("native"), false)
        .expect("failed to create native required-artifacts project");
    rewrite_local_makefile_test_target(
        &native_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example && false
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite native fixture test target for parity mismatch");
    let native_log_dir = root.join("native_logs");
    let native_result = run_native_baseline_in_tree(&native_project, &native_log_dir);
    assert!(
        native_result.is_err(),
        "native baseline should fail for parity mismatch case"
    );

    let native_status =
        read_native_make_test_exit_status(&native_log_dir).expect("failed to read native status");
    assert_ne!(native_status, 0, "mismatch fixture should fail natively");

    let err = assert_make_test_exit_status_parity(&native_log_dir, &replay_log_dir)
        .expect_err("parity evaluation should report missing replay make-test manifest");
    assert!(
        err.contains("make_test_commands_manifest.txt"),
        "unexpected mismatch error: {}",
        err
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_stdout_stderr_parity_local_fixture_success() {
    let root = unique_temp_dir("zlib_make_test_output_parity_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let replay_project = create_local_required_artifacts_project(&root.join("replay"), false)
        .expect("failed to create replay required-artifacts project");
    rewrite_local_makefile_test_target(
        &replay_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example && echo "stdout static $$PWD" && echo "stderr static $$PWD" 1>&2
	@test -x ./minigzipsh && test -x ./examplesh && echo "stdout shared $$PWD" && echo "stderr shared $$PWD" 1>&2
	@test -x ./minigzip64 && test -x ./example64 && echo "stdout 64 $$PWD" && echo "stderr 64 $$PWD" 1>&2
"#,
    )
    .expect("failed to rewrite replay fixture test target for output parity success");

    let replay_log_dir = root.join("replay_logs");
    let replay_err =
        run_make_test_command_subset_replay_in_tree(&replay_project, &replay_log_dir)
        .expect_err("strict make-test replay should fail at link step for output parity success case");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected strict replay error: {}",
        replay_err
    );

    let native_project = create_local_required_artifacts_project(&root.join("native"), false)
        .expect("failed to create native required-artifacts project");
    rewrite_local_makefile_test_target(
        &native_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example && echo "stdout static $$PWD" && echo "stderr static $$PWD" 1>&2
	@test -x ./minigzipsh && test -x ./examplesh && echo "stdout shared $$PWD" && echo "stderr shared $$PWD" 1>&2
	@test -x ./minigzip64 && test -x ./example64 && echo "stdout 64 $$PWD" && echo "stderr 64 $$PWD" 1>&2
"#,
    )
    .expect("failed to rewrite native fixture test target for output parity success");
    let native_log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&native_project, &native_log_dir)
        .expect("native baseline should succeed for output parity success case");
    write_baseline_manifest(&native_log_dir, &native_project)
        .expect("failed to write baseline manifest for output parity success case");

    let parity_err = assert_make_test_stdout_stderr_parity(&native_log_dir, &replay_log_dir)
        .expect_err("parity evaluation should report missing replay make-test manifest");
    assert!(
        parity_err.contains("make_test_commands_manifest.txt"),
        "unexpected parity error after strict link failure: {}",
        parity_err
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_stdout_stderr_parity_local_fixture_reports_mismatch() {
    let root = unique_temp_dir("zlib_make_test_output_parity_mismatch");
    fs::create_dir_all(&root).expect("failed to create test root");

    let replay_project = create_local_required_artifacts_project(&root.join("replay"), false)
        .expect("failed to create replay required-artifacts project");
    fs::write(
        replay_project.join("parity_stdout.txt"),
        "fragile-parity-stdout\n",
    )
    .expect("failed to write fragile parity stdout marker");
    fs::write(
        replay_project.join("parity_stderr.txt"),
        "fragile-parity-stderr\n",
    )
    .expect("failed to write fragile parity stderr marker");
    rewrite_local_makefile_test_target(
        &replay_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example && cat ./parity_stdout.txt && cat ./parity_stderr.txt 1>&2
	@test -x ./minigzipsh && test -x ./examplesh && cat ./parity_stdout.txt && cat ./parity_stderr.txt 1>&2
	@test -x ./minigzip64 && test -x ./example64 && cat ./parity_stdout.txt && cat ./parity_stderr.txt 1>&2
"#,
    )
    .expect("failed to rewrite replay fixture test target for output parity mismatch");

    let replay_log_dir = root.join("replay_logs");
    let replay_err =
        run_make_test_command_subset_replay_in_tree(&replay_project, &replay_log_dir)
        .expect_err("strict make-test replay should fail at link step for output parity mismatch case");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected strict replay error: {}",
        replay_err
    );

    let native_project = create_local_required_artifacts_project(&root.join("native"), false)
        .expect("failed to create native required-artifacts project");
    fs::write(
        native_project.join("parity_stdout.txt"),
        "native-parity-stdout\n",
    )
    .expect("failed to write native parity stdout marker");
    fs::write(
        native_project.join("parity_stderr.txt"),
        "native-parity-stderr\n",
    )
    .expect("failed to write native parity stderr marker");
    rewrite_local_makefile_test_target(
        &native_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example && cat ./parity_stdout.txt && cat ./parity_stderr.txt 1>&2
	@test -x ./minigzipsh && test -x ./examplesh && cat ./parity_stdout.txt && cat ./parity_stderr.txt 1>&2
	@test -x ./minigzip64 && test -x ./example64 && cat ./parity_stdout.txt && cat ./parity_stderr.txt 1>&2
"#,
    )
    .expect("failed to rewrite native fixture test target for output parity mismatch");

    let native_log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&native_project, &native_log_dir)
        .expect("native baseline should succeed for output parity mismatch case");
    write_baseline_manifest(&native_log_dir, &native_project)
        .expect("failed to write baseline manifest for output parity mismatch case");

    let err = assert_make_test_stdout_stderr_parity(&native_log_dir, &replay_log_dir)
        .expect_err("parity evaluation should report missing replay make-test manifest");
    assert!(
        err.contains("make_test_commands_manifest.txt"),
        "unexpected parity mismatch error: {}",
        err
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_artifact_behavior_parity_local_fixture_success() {
    let root = unique_temp_dir("zlib_make_test_artifact_parity_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let replay_project = create_local_required_artifacts_project(&root.join("replay"), false)
        .expect("failed to create replay required-artifacts project");
    write_local_tiny_marker_program(&replay_project, false)
        .expect("failed to rewrite replay tiny.c marker program");
    rewrite_local_makefile_test_target(
        &replay_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite replay fixture test target for artifact parity success");
    let replay_log_dir = root.join("replay_logs");
    run_native_baseline_in_tree(&replay_project, &root.join("replay_native_logs"))
        .expect("replay fixture build should succeed for artifact behavior parity success case");
    write_local_replay_source_manifest_for_parity(&replay_log_dir, &replay_project)
        .expect("failed to write replay source manifest for artifact behavior parity success");

    let native_project = create_local_required_artifacts_project(&root.join("native"), false)
        .expect("failed to create native required-artifacts project");
    write_local_tiny_marker_program(&native_project, false)
        .expect("failed to rewrite native tiny.c marker program");
    rewrite_local_makefile_test_target(
        &native_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite native fixture test target for artifact parity success");
    let native_log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&native_project, &native_log_dir)
        .expect("native baseline should succeed for artifact behavior parity success case");
    write_baseline_manifest(&native_log_dir, &native_project)
        .expect("failed to write baseline manifest for artifact behavior parity success case");

    assert_make_test_artifact_behavior_parity(&native_log_dir, &replay_log_dir)
        .expect("artifact behavior parity should match for local success case");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_artifact_behavior_parity_local_fixture_reports_mismatch() {
    let root = unique_temp_dir("zlib_make_test_artifact_parity_mismatch");
    fs::create_dir_all(&root).expect("failed to create test root");

    let replay_project = create_local_required_artifacts_project(&root.join("replay"), false)
        .expect("failed to create replay required-artifacts project");
    write_local_tiny_marker_program(&replay_project, false)
        .expect("failed to rewrite replay tiny.c marker program");
    rewrite_local_makefile_test_target(
        &replay_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite replay fixture test target for artifact parity mismatch");
    let replay_log_dir = root.join("replay_logs");
    run_native_baseline_in_tree(&replay_project, &root.join("replay_native_logs"))
        .expect("replay fixture build should succeed for artifact behavior parity mismatch case");
    write_local_replay_source_manifest_for_parity(&replay_log_dir, &replay_project)
        .expect("failed to write replay source manifest for artifact behavior parity mismatch");

    let native_project = create_local_required_artifacts_project(&root.join("native"), false)
        .expect("failed to create native required-artifacts project");
    write_local_tiny_marker_program(&native_project, true)
        .expect("failed to rewrite native tiny.c marker program");
    rewrite_local_makefile_test_target(
        &native_project,
        r#"test:
	@$(MAKE) all
	@test -x ./minigzip && test -x ./example
	@test -x ./minigzipsh && test -x ./examplesh
	@test -x ./minigzip64 && test -x ./example64
"#,
    )
    .expect("failed to rewrite native fixture test target for artifact parity mismatch");
    let native_log_dir = root.join("native_logs");
    run_native_baseline_in_tree(&native_project, &native_log_dir)
        .expect("native baseline should succeed for artifact behavior parity mismatch case");
    write_baseline_manifest(&native_log_dir, &native_project)
        .expect("failed to write baseline manifest for artifact behavior parity mismatch case");

    let err = assert_make_test_artifact_behavior_parity(&native_log_dir, &replay_log_dir)
        .expect_err("expected artifact behavior parity mismatch");
    assert!(
        err.contains("artifact behavior parity mismatch"),
        "unexpected parity mismatch error: {}",
        err
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
    assert!(fs::read_to_string(log_dir.join("artifact_manifest.txt"))
        .expect("failed to read artifact_manifest.txt")
        .contains("compile_units_count="));
    assert!(fs::read_to_string(log_dir.join("cc_driver.log"))
        .expect("failed to read cc_driver.log")
        .contains("tiny.c"));
    let compile_units = fs::read_to_string(log_dir.join("compile_units_manifest.txt"))
        .expect("failed to read compile_units_manifest.txt");
    assert!(
        compile_units.contains("source=tiny.c object=tiny.o"),
        "compile unit manifest should include tiny compile unit: {}",
        compile_units
    );
    let link_units = fs::read_to_string(log_dir.join("link_units_manifest.txt"))
        .expect("failed to read link_units_manifest.txt");
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        assert!(
            link_units.contains(&format!("output={}", output)),
            "link unit manifest should include output {}: {}",
            output,
            link_units
        );
    }

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
fn test_fragile_link_required_binaries_local_fixture_success() {
    let root = unique_temp_dir("zlib_fragile_link_required_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_required_artifacts_project(&root, false)
        .expect("failed to create required-artifacts project");
    let log_dir = root.join("logs");
    let err = run_fragile_link_required_binaries_in_tree(&project_dir, &log_dir)
        .expect_err("strict fragile required-link replay should fail");
    assert!(
        err.contains("link replay failed for"),
        "unexpected strict required-link replay error: {}",
        err
    );

    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_link_tiny_o.status"))
            .expect("failed to read rustc_link_tiny_o.status")
            .trim(),
        "0",
        "expected tiny.o transpile+compile replay to succeed"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_link_runtime_support.status"))
            .expect("failed to read rustc_link_runtime_support.status")
            .trim(),
        "0",
        "expected runtime support staticlib build to succeed"
    );
    let runtime_support_archive = log_dir.join("libfragile_runtime_support.a");
    assert!(
        runtime_support_archive.exists(),
        "runtime support archive should exist at {}",
        runtime_support_archive.display()
    );
    let runtime_support_size = fs::metadata(&runtime_support_archive)
        .expect("failed to stat runtime support archive")
        .len();
    assert!(
        runtime_support_size > 0,
        "runtime support archive should be non-empty"
    );
    assert!(
        !log_dir.join("fragile_link_manifest.txt").exists(),
        "strict required-link replay should not write fragile link manifest on failure"
    );
    let mut observed_link_steps: Vec<String> = Vec::new();
    let mut first_failing_link_step: Option<String> = None;
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        let link_step = format!("link_required_{}", normalize_identifier_fragment(output));
        let status_path = log_dir.join(format!("{}.status", link_step));
        if !status_path.exists() {
            break;
        }
        observed_link_steps.push(link_step.clone());
        let status = fs::read_to_string(&status_path).expect("failed to read strict link status");
        if status.trim() != "0" && first_failing_link_step.is_none() {
            first_failing_link_step = Some(link_step);
        }
    }
    assert!(
        !observed_link_steps.is_empty(),
        "strict required-link replay should emit at least one link-step status"
    );
    assert!(
        first_failing_link_step.is_some(),
        "strict required-link replay should expose at least one failing link step"
    );
    let first_failing_step = first_failing_link_step.expect("missing first failing link step");
    let first_failing_stderr =
        fs::read_to_string(log_dir.join(format!("{}.stderr", first_failing_step)))
            .expect("failed to read first failing strict-link stderr");
    assert!(
        !first_failing_stderr.trim().is_empty(),
        "strict required-link replay failing step should include linker diagnostics"
    );
    assert!(
        !first_failing_stderr.contains("core::panicking::panic"),
        "runtime-link leaf should clear Rust runtime unresolved-symbol diagnostics: {}",
        first_failing_stderr
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_link_replay_reports_missing_static_archive_from_manifest() {
    let root = unique_temp_dir("zlib_fragile_link_required_missing_unit");
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

    let mut link_manifest = format!(
        "source_dir={}\nlink_units_count={}\nrequired_link_outputs={}\n",
        project_dir.display(),
        ZLIB_REQUIRED_LINK_OUTPUTS.len(),
        ZLIB_REQUIRED_LINK_OUTPUTS.join(","),
    );
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        let inputs = if *output == "example" {
            "missing_link_input.a"
        } else {
            "tiny.o,libz.a"
        };
        link_manifest.push_str(&format!("output={} inputs={}\n", output, inputs));
    }
    fs::write(log_dir.join("link_units_manifest.txt"), link_manifest)
        .expect("failed to write synthetic link_units_manifest.txt");

    let err = replay_required_link_binaries_from_manifests_in_tree(&project_dir, &log_dir)
        .expect_err("missing static archive in link manifest should fail replay");
    assert!(
        err.contains("required static archive input"),
        "unexpected replay failure: {}",
        err
    );
    assert!(
        err.contains("missing_link_input.a"),
        "missing static archive should be reported: {}",
        err
    );
    assert!(
        !log_dir.join("fragile_link_manifest.txt").exists(),
        "fragile link manifest should not be written on replay failure"
    );
    assert!(
        !log_dir.join("rustc_link_tiny_o.status").exists(),
        "rustc replay should not start when required compile units are missing"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_parse_makefile_variable_list_supports_line_continuation() {
    let makefile = r#"OBJZ = adler32.o crc32.o \
deflate.o
OBJG = compress.o \
uncompr.o
"#;

    let objz = parse_makefile_variable_list(makefile, "OBJZ").expect("failed to parse OBJZ");
    assert_eq!(
        objz,
        vec![
            "adler32.o".to_string(),
            "crc32.o".to_string(),
            "deflate.o".to_string()
        ]
    );

    let objg = parse_makefile_variable_list(makefile, "OBJG").expect("failed to parse OBJG");
    assert_eq!(
        objg,
        vec!["compress.o".to_string(), "uncompr.o".to_string()]
    );
}

#[test]
fn test_libza_replay_plan_build_local_fixture_success() {
    let root = unique_temp_dir("zlib_libza_replay_plan_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, None)
        .expect("failed to create libza replay-plan project");
    let log_dir = root.join("logs");
    run_cc_driver_libza_replay_plan_in_tree(&project_dir, &log_dir)
        .expect("libza replay plan generation should succeed");

    assert!(
        project_dir.join("libz.a").exists(),
        "expected static library output"
    );
    let replay_plan = fs::read_to_string(log_dir.join("libza_replay_plan.txt"))
        .expect("failed to read replay plan");
    assert!(
        replay_plan.contains("libza_target_count=15"),
        "unexpected replay plan target count: {}",
        replay_plan
    );
    for target in ZLIB_LIBZA_OBJECTS {
        assert!(
            replay_plan.contains(&format!("object={}", target)),
            "replay plan should include {}: {}",
            target,
            replay_plan
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_libza_replay_plan_build_detects_missing_compile_unit() {
    let root = unique_temp_dir("zlib_libza_replay_plan_missing_unit");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, Some("gzwrite.o"))
        .expect("failed to create libza replay-plan project");
    let log_dir = root.join("logs");
    let err = run_cc_driver_libza_replay_plan_in_tree(&project_dir, &log_dir)
        .expect_err("expected replay plan to fail when one target misses CC compile unit");
    assert!(
        err.contains("missing compile units for libz object targets"),
        "unexpected error message: {}",
        err
    );
    assert!(
        err.contains("gzwrite.o"),
        "missing target should mention gzwrite.o: {}",
        err
    );
    assert!(
        log_dir.join("compile_units_manifest.txt").exists(),
        "compile units manifest should still be written for diagnosis"
    );
    assert!(
        !log_dir.join("libza_replay_plan.txt").exists(),
        "replay plan should not be written when target coverage is incomplete"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_objz_replay_build_local_fixture_success() {
    let root = unique_temp_dir("zlib_fragile_objz_replay_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, None)
        .expect("failed to create libza replay-plan project");
    let log_dir = root.join("logs");
    run_fragile_objz_replay_in_tree(&project_dir, &log_dir)
        .expect("fragile OBJZ replay should succeed");

    let manifest = fs::read_to_string(log_dir.join("fragile_objz_manifest.txt"))
        .expect("failed to read fragile_objz_manifest.txt");
    assert!(
        manifest.contains("replay_scope=OBJZ"),
        "manifest should include replay scope: {}",
        manifest
    );
    assert!(
        manifest.contains(&format!("replay_target_count={}", ZLIB_OBJZ_OBJECTS.len())),
        "manifest should include OBJZ target count: {}",
        manifest
    );
    assert!(
        manifest.contains(&format!("replayed_count={}", ZLIB_OBJZ_OBJECTS.len())),
        "manifest should include OBJZ replayed count: {}",
        manifest
    );

    for object in ZLIB_OBJZ_OBJECTS {
        let object_path = project_dir.join(object);
        assert!(
            object_path.exists(),
            "expected replayed object {}",
            object_path.display()
        );
        assert!(
            fs::metadata(&object_path)
                .expect("failed to stat replayed object")
                .len()
                > 0,
            "replayed object should be non-empty: {}",
            object_path.display()
        );
        assert!(
            manifest.contains(&format!("object={}", object)),
            "manifest should include OBJZ object {}: {}",
            object,
            manifest
        );

        let rustc_step = rustc_replay_step_name("OBJZ", object);
        assert_eq!(
            fs::read_to_string(log_dir.join(format!("{}.status", rustc_step)))
                .expect("failed to read rustc replay status")
                .trim(),
            "0",
            "rustc replay should succeed for {}",
            object
        );
    }

    for object in ZLIB_OBJG_OBJECTS {
        assert!(
            !manifest.contains(&format!("object={}", object)),
            "OBJG object {} should not appear in OBJZ replay manifest: {}",
            object,
            manifest
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_objg_replay_build_local_fixture_success() {
    let root = unique_temp_dir("zlib_fragile_objg_replay_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, None)
        .expect("failed to create libza replay-plan project");
    let log_dir = root.join("logs");
    run_fragile_objg_replay_in_tree(&project_dir, &log_dir)
        .expect("fragile OBJG replay should succeed");

    let manifest = fs::read_to_string(log_dir.join("fragile_objg_manifest.txt"))
        .expect("failed to read fragile_objg_manifest.txt");
    assert!(
        manifest.contains("replay_scope=OBJG"),
        "manifest should include replay scope: {}",
        manifest
    );
    assert!(
        manifest.contains(&format!("replay_target_count={}", ZLIB_OBJG_OBJECTS.len())),
        "manifest should include OBJG target count: {}",
        manifest
    );
    assert!(
        manifest.contains(&format!("replayed_count={}", ZLIB_OBJG_OBJECTS.len())),
        "manifest should include OBJG replayed count: {}",
        manifest
    );

    for object in ZLIB_OBJG_OBJECTS {
        let object_path = project_dir.join(object);
        assert!(
            object_path.exists(),
            "expected replayed object {}",
            object_path.display()
        );
        assert!(
            fs::metadata(&object_path)
                .expect("failed to stat replayed object")
                .len()
                > 0,
            "replayed object should be non-empty: {}",
            object_path.display()
        );
        assert!(
            manifest.contains(&format!("object={}", object)),
            "manifest should include OBJG object {}: {}",
            object,
            manifest
        );

        let rustc_step = rustc_replay_step_name("OBJG", object);
        assert_eq!(
            fs::read_to_string(log_dir.join(format!("{}.status", rustc_step)))
                .expect("failed to read rustc replay status")
                .trim(),
            "0",
            "rustc replay should succeed for {}",
            object
        );
    }

    for object in ZLIB_OBJZ_OBJECTS {
        assert!(
            !manifest.contains(&format!("object={}", object)),
            "OBJZ object {} should not appear in OBJG replay manifest: {}",
            object,
            manifest
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_objz_replay_parses_c_register_storage_class() {
    let root = unique_temp_dir("zlib_fragile_objz_replay_register_c");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, None)
        .expect("failed to create libza replay-plan project");
    fs::write(
        project_dir.join("deflate.c"),
        "int deflate_with_register(void) { register int x = 7; return x; }\n",
    )
    .expect("failed to write deflate.c fixture with register storage class");

    let log_dir = root.join("logs");
    run_fragile_objz_replay_in_tree(&project_dir, &log_dir)
        .expect("OBJZ replay should parse C register storage class");

    let rustc_step = rustc_replay_step_name("OBJZ", "deflate.o");
    assert_eq!(
        fs::read_to_string(log_dir.join(format!("{}.status", rustc_step)))
            .expect("failed to read deflate rustc replay status")
            .trim(),
        "0",
        "deflate OBJZ replay should compile after C parser-mode fix"
    );
    assert!(
        fs::read_to_string(log_dir.join("fragile_objz_manifest.txt"))
            .expect("failed to read fragile_objz_manifest.txt")
            .contains("object=deflate.o"),
        "manifest should include deflate.o replay entry"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_objz_replay_handles_typedef_struct_aggregate_initializer() {
    let root = unique_temp_dir("zlib_fragile_objz_replay_config_table_c");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, None)
        .expect("failed to create libza replay-plan project");
    fs::write(
        project_dir.join("deflate.c"),
        r#"
typedef int (*compress_func)(void);

typedef struct config_s {
    int good_length;
    int max_lazy;
    int nice_length;
    int max_chain;
    compress_func func;
} config;

int deflate_stored(void) { return 0; }
int deflate_fast(void) { return 0; }
int deflate_slow(void) { return 0; }

config configuration_table[3] = {
    {0, 0, 0, 0, deflate_stored},
    {4, 4, 8, 4, deflate_fast},
    {4, 5, 16, 8, deflate_slow},
};

int deflate_with_configuration_table(void) {
    return configuration_table[1].max_lazy + configuration_table[2].nice_length;
}
"#,
    )
    .expect("failed to write deflate.c fixture with configuration_table aggregate init");

    let log_dir = root.join("logs");
    run_fragile_objz_replay_in_tree(&project_dir, &log_dir)
        .expect("OBJZ replay should compile typedef-aggregate config table");

    let rustc_step = rustc_replay_step_name("OBJZ", "deflate.o");
    assert_eq!(
        fs::read_to_string(log_dir.join(format!("{}.status", rustc_step)))
            .expect("failed to read deflate rustc replay status")
            .trim(),
        "0",
        "deflate OBJZ replay should compile typedef struct aggregate initializer"
    );

    let transpiled = fs::read_to_string(log_dir.join("objz_deflate_o_transpiled.rs"))
        .expect("failed to read transpiled deflate artifact");
    assert!(
        transpiled.contains("config { good_length:"),
        "transpiled aggregate initializer should use named fields for config typedef: {}",
        transpiled
    );
    assert!(
        !transpiled.contains("config { 0"),
        "transpiled aggregate initializer should not use positional struct literal syntax: {}",
        transpiled
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_objz_replay_build_detects_missing_compile_unit() {
    let root = unique_temp_dir("zlib_fragile_objz_replay_missing_unit");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_libza_replay_plan_project(&root, Some("trees.o"))
        .expect("failed to create libza replay-plan project");
    let log_dir = root.join("logs");
    let err = run_fragile_objz_replay_in_tree(&project_dir, &log_dir)
        .expect_err("expected OBJZ replay to fail when one compile unit is missing");

    assert!(
        err.contains("missing compile units for libz object targets"),
        "unexpected error message: {}",
        err
    );
    assert!(
        err.contains("trees.o"),
        "missing target should mention trees.o: {}",
        err
    );
    assert!(
        log_dir.join("compile_units_manifest.txt").exists(),
        "compile units manifest should still be written on failure"
    );
    assert!(
        !log_dir.join("fragile_objz_manifest.txt").exists(),
        "OBJZ replay manifest should not be written on failure"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_single_object_build_local_fixture_success() {
    let root = unique_temp_dir("zlib_fragile_object_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_fragile_object_project(&root)
        .expect("failed to create fragile-object project");
    let log_dir = root.join("logs");
    run_fragile_single_object_in_tree(&project_dir, &log_dir, "adler32.c", "adler32.o")
        .expect("fragile single object replay should succeed");

    let object_path = project_dir.join("adler32.o");
    assert!(
        object_path.exists(),
        "expected transpiled object output at {}",
        object_path.display()
    );
    assert!(
        fs::metadata(&object_path)
            .expect("failed to stat adler32.o")
            .len()
            > 0,
        "adler32.o should be non-empty"
    );

    for rel in ZLIB_FRAGILE_ADLER32_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile-object log {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_object.status"))
            .expect("failed to read rustc_object.status")
            .trim(),
        "0"
    );
    assert!(
        fs::read_to_string(log_dir.join("compile_units_manifest.txt"))
            .expect("failed to read compile_units_manifest.txt")
            .contains("source=adler32.c object=adler32.o")
    );
    let manifest = fs::read_to_string(log_dir.join("fragile_object_manifest.txt"))
        .expect("failed to read fragile object manifest");
    assert!(
        manifest.contains("source=adler32.c"),
        "manifest should include source entry: {}",
        manifest
    );
    assert!(
        manifest.contains("object=adler32.o"),
        "manifest should include object entry: {}",
        manifest
    );
    assert!(
        manifest.contains("compile_units_count="),
        "manifest should include compile unit count: {}",
        manifest
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_single_object_build_fails_when_source_unit_missing() {
    let root = unique_temp_dir("zlib_fragile_object_missing_source_unit");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_fragile_object_project(&root)
        .expect("failed to create fragile-object project");
    let log_dir = root.join("logs");
    let err = run_fragile_single_object_in_tree(&project_dir, &log_dir, "adler32.c", "tiny.o")
        .expect_err("expected source-unit selection to fail");

    assert!(
        err.contains("compile unit for source adler32.c not found"),
        "unexpected failure reason: {}",
        err
    );
    assert!(
        log_dir.join("compile_units_manifest.txt").exists(),
        "compile units manifest should exist to aid failure diagnosis"
    );
    assert!(
        !log_dir.join("rustc_object.status").exists(),
        "rustc should not run if selected source compile unit is missing"
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
    let compile_units = fs::read_to_string(log_dir.join("compile_units_manifest.txt"))
        .expect("failed to read compile units manifest");
    assert!(
        compile_units.contains("compile_units_count="),
        "compile unit manifest should include count header: {}",
        compile_units
    );
    assert!(
        compile_units.contains("source=adler32.c"),
        "compile unit manifest should include adler32.c compile unit: {}",
        compile_units
    );
    let link_units = fs::read_to_string(log_dir.join("link_units_manifest.txt"))
        .expect("failed to read link units manifest");
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        assert!(
            link_units.contains(&format!("output={}", output)),
            "link unit manifest should include output {}: {}",
            output,
            link_units
        );
    }
}

#[test]
#[ignore = "real-world external project test (downloads and derives zlib make test command plan)"]
fn test_real_world_zlib_make_test_command_plan_generation() {
    let log_dir = run_zlib_make_test_command_plan_baseline()
        .expect("failed to generate real-world make-test command plan");
    for rel in ZLIB_MAKE_TEST_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected make-test command-plan log {}",
            log_dir.join(rel).display()
        );
    }

    assert_eq!(
        fs::read_to_string(log_dir.join("make_test_dryrun.status"))
            .expect("failed to read make_test_dryrun.status")
            .trim(),
        "0"
    );
    let manifest = fs::read_to_string(log_dir.join("make_test_commands_manifest.txt"))
        .expect("failed to read make_test_commands_manifest.txt");
    assert!(
        manifest.contains("make_test_command_count="),
        "manifest should include command count header: {}",
        manifest
    );
    for binary in ZLIB_REQUIRED_LINK_OUTPUTS {
        assert!(
            manifest.contains(&format!("./{}", binary)),
            "manifest should include required binary invocation for {}: {}",
            binary,
            manifest
        );
    }
}

#[test]
#[ignore = "real-world external project test (downloads and replays zlib make-test command subset)"]
fn test_real_world_zlib_make_test_command_subset_replay() {
    let replay_result = run_zlib_make_test_command_subset_replay_baseline();

    let log_dir = Path::new(ZLIB_MAKE_TEST_REPLAY_BASELINE_DIR).join("driver_logs");
    let err = replay_result.expect_err("make-test replay should currently fail at strict link step");
    assert!(
        err.contains("link replay failed for"),
        "unexpected make-test replay failure: {}",
        err
    );

    for rel in ZLIB_FRAGILE_LINK_REQUIRED_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected make-test replay log {}",
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
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_link_runtime_support.status"))
            .expect("failed to read rustc_link_runtime_support.status")
            .trim(),
        "0"
    );
    assert!(
        !log_dir.join("make_test_dryrun.status").exists(),
        "make-test dry-run should not execute when strict link replay fails first"
    );
    assert!(
        !log_dir.join("make_test_commands_manifest.txt").exists(),
        "make-test command plan should not be written when strict link replay fails first"
    );
    assert!(
        !log_dir.join("make_test_replay_manifest.txt").exists(),
        "make-test replay manifest should not be written when strict link replay fails first"
    );
    assert!(
        !log_dir.join("fragile_link_manifest.txt").exists(),
        "fragile link manifest should not be written on strict link replay failure"
    );

    let mut observed_link_steps: Vec<String> = Vec::new();
    let mut failing_link_steps: Vec<String> = Vec::new();
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        let link_step = format!("link_required_{}", normalize_identifier_fragment(output));
        let status_path = log_dir.join(format!("{}.status", link_step));
        if !status_path.exists() {
            break;
        }
        observed_link_steps.push(link_step.clone());
        let status = fs::read_to_string(&status_path).expect("failed to read strict link status");
        if status.trim() != "0" {
            failing_link_steps.push(link_step);
        }
    }
    assert!(
        !observed_link_steps.is_empty(),
        "strict link replay should emit at least one link-step status"
    );
    assert!(
        !failing_link_steps.is_empty(),
        "strict link replay should surface at least one failing link step"
    );
    let first_failing_step = &failing_link_steps[0];
    let stderr = fs::read_to_string(log_dir.join(format!("{}.stderr", first_failing_step)))
        .expect("failed to read first failing strict link stderr");
    assert!(
        !stderr.trim().is_empty(),
        "strict link replay failure should emit linker diagnostics for {}",
        first_failing_step
    );
    assert!(
        stderr.contains("_dist_code") || stderr.contains("_length_code"),
        "expected post-runtime-link first blocker to be unresolved C globals (_dist_code/_length_code): {}",
        stderr
    );
    assert!(
        !stderr.contains("core::panicking::panic"),
        "runtime-link leaf should clear Rust runtime unresolved-symbol diagnostics: {}",
        stderr
    );
}

#[test]
#[ignore = "real-world external project test (downloads zlib and compares native-vs-fragile make-test exit status)"]
fn test_real_world_zlib_make_test_exit_status_parity() {
    let replay_result = run_zlib_make_test_command_subset_replay_baseline();
    let replay_log_dir = Path::new(ZLIB_MAKE_TEST_REPLAY_BASELINE_DIR).join("driver_logs");

    let replay_err =
        replay_result.expect_err("fragile replay is expected to fail in current baseline");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected make-test replay failure: {}",
        replay_err
    );
    assert!(
        !replay_log_dir.join("make_test_commands_manifest.txt").exists(),
        "make-test exit status parity cannot run until strict link replay reaches make-test planning"
    );
}

#[test]
#[ignore = "real-world external project test (downloads zlib and compares native-vs-fragile make-test stdout/stderr)"]
fn test_real_world_zlib_make_test_stdout_stderr_parity() {
    let replay_result = run_zlib_make_test_command_subset_replay_baseline();
    let replay_log_dir = Path::new(ZLIB_MAKE_TEST_REPLAY_BASELINE_DIR).join("driver_logs");

    let replay_err =
        replay_result.expect_err("fragile replay is expected to fail in current baseline");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected make-test replay failure: {}",
        replay_err
    );
    assert!(
        !replay_log_dir.join("make_test_commands_manifest.txt").exists(),
        "make-test stdout/stderr parity cannot run until strict link replay reaches make-test planning"
    );
}

#[test]
#[ignore = "real-world external project test (downloads zlib and compares native-vs-fragile make-test artifact behavior)"]
fn test_real_world_zlib_make_test_artifact_behavior_parity() {
    let replay_result = run_zlib_make_test_command_subset_replay_baseline();
    let replay_log_dir = Path::new(ZLIB_MAKE_TEST_REPLAY_BASELINE_DIR).join("driver_logs");

    let replay_err =
        replay_result.expect_err("fragile replay is expected to fail in current baseline");
    assert!(
        replay_err.contains("link replay failed for"),
        "unexpected make-test replay failure: {}",
        replay_err
    );
    assert!(
        !replay_log_dir.join("make_test_commands_manifest.txt").exists(),
        "make-test artifact parity cannot run until strict link replay reaches make-test planning"
    );
}

#[test]
#[ignore = "real-world external project test (downloads, transpiles objects, and relinks zlib required binaries)"]
fn test_real_world_zlib_fragile_required_link_binaries_replay() {
    let replay_result = run_zlib_fragile_link_required_binaries_baseline();

    let log_dir = Path::new(ZLIB_FRAGILE_LINK_REQUIRED_BINARIES_BASELINE_DIR).join("driver_logs");
    replay_result.expect("strict required-link replay should succeed");

    for rel in ZLIB_FRAGILE_LINK_REQUIRED_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile required-link log {}",
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
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_link_adler32_o.status"))
            .expect("failed to read rustc_link_adler32_o.status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_link_runtime_support.status"))
            .expect("failed to read rustc_link_runtime_support.status")
            .trim(),
        "0"
    );
    let runtime_support_archive = log_dir.join("libfragile_runtime_support.a");
    assert!(
        runtime_support_archive.exists(),
        "expected runtime support archive {}",
        runtime_support_archive.display()
    );
    let runtime_support_size = fs::metadata(&runtime_support_archive)
        .expect("failed to stat runtime support archive")
        .len();
    assert!(
        runtime_support_size > 0,
        "runtime support archive should be non-empty"
    );

    let link_manifest_path = log_dir.join("fragile_link_manifest.txt");
    assert!(
        link_manifest_path.exists(),
        "strict required-link replay should write fragile link manifest on success"
    );
    let link_manifest =
        fs::read_to_string(&link_manifest_path).expect("failed to read fragile link manifest");
    assert!(
        link_manifest.contains("relinked_output_count=6"),
        "expected all required outputs to be relinked: {}",
        link_manifest
    );

    let mut observed_link_steps: Vec<String> = Vec::new();
    for output in ZLIB_REQUIRED_LINK_OUTPUTS {
        let link_step = format!("link_required_{}", normalize_identifier_fragment(output));
        let status_path = log_dir.join(format!("{}.status", link_step));
        assert!(
            status_path.exists(),
            "expected strict link replay status for {}",
            output
        );
        observed_link_steps.push(link_step.clone());
        let status = fs::read_to_string(&status_path).expect("failed to read strict link status");
        assert!(
            status.trim() == "0",
            "strict link replay should succeed for {} (status={} logs={})",
            output,
            status.trim(),
            log_dir.display()
        );
        assert!(
            link_manifest.contains(&format!("output={}", output)),
            "fragile link manifest should include output {}: {}",
            output,
            link_manifest
        );
        let stderr = fs::read_to_string(log_dir.join(format!("{}.stderr", link_step)))
            .expect("failed to read strict link stderr");
        assert!(
            stderr.trim().is_empty(),
            "successful strict link step should not emit stderr diagnostics for {}: {}",
            output,
            stderr
        );
    }
    assert!(
        observed_link_steps.len() == ZLIB_REQUIRED_LINK_OUTPUTS.len(),
        "strict required-link replay should emit status for all required outputs"
    );
}

#[test]
#[ignore = "real-world external project test (downloads and plans zlib OBJZ/OBJG replay scope)"]
fn test_real_world_zlib_libza_replay_plan_for_objz_objg_scope() {
    let log_dir =
        run_zlib_libza_replay_plan_baseline().expect("failed to run zlib libza replay-plan build");
    for rel in ZLIB_LIBZA_REPLAY_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected libza replay-plan log {}",
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
    let replay_plan = fs::read_to_string(log_dir.join("libza_replay_plan.txt"))
        .expect("failed to read replay plan");
    assert!(
        replay_plan.contains("libza_target_count=15"),
        "unexpected replay plan size: {}",
        replay_plan
    );
    for object in ZLIB_LIBZA_OBJECTS {
        assert!(
            replay_plan.contains(&format!("object={}", object)),
            "replay plan should include {}: {}",
            object,
            replay_plan
        );
    }
}

#[test]
#[ignore = "real-world external project test (downloads and transpiles zlib OBJZ objects)"]
fn test_real_world_zlib_fragile_objz_objects_replay() {
    let replay_result = run_zlib_fragile_objz_objects_baseline();

    let log_dir = Path::new(ZLIB_FRAGILE_OBJZ_OBJECTS_BASELINE_DIR).join("driver_logs");
    let worktree_dir = Path::new(ZLIB_FRAGILE_OBJZ_OBJECTS_BASELINE_DIR).join("worktree");
    for rel in ZLIB_FRAGILE_OBJZ_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile-OBJZ log {}",
            log_dir.join(rel).display()
        );
    }

    let replay_log_dir = replay_result
        .map_err(|e| format!("full OBJZ replay failed unexpectedly: {}", e))
        .expect("OBJZ replay should succeed end-to-end");
    assert_eq!(
        replay_log_dir, log_dir,
        "replay helper should return baseline OBJZ driver log directory"
    );

    let replay_plan = fs::read_to_string(log_dir.join("libza_replay_plan.txt"))
        .expect("failed to read libza_replay_plan.txt");
    assert!(
        replay_plan.contains("libza_target_count=15"),
        "replay plan should be generated before OBJZ replay: {}",
        replay_plan
    );

    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_objz_adler32_o.status"))
            .expect("failed to read adler32 replay status")
            .trim(),
        "0"
    );
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_objz_crc32_o.status"))
            .expect("failed to read crc32 replay status")
            .trim(),
        "0"
    );

    let manifest = fs::read_to_string(log_dir.join("fragile_objz_manifest.txt"))
        .expect("failed to read fragile_objz_manifest.txt");
    assert!(
        manifest.contains("replay_scope=OBJZ"),
        "manifest should include replay scope: {}",
        manifest
    );
    assert!(
        manifest.contains(&format!("replayed_count={}", ZLIB_OBJZ_OBJECTS.len())),
        "manifest should include full OBJZ replay count: {}",
        manifest
    );
    for object in ZLIB_OBJZ_OBJECTS {
        let rustc_step = rustc_replay_step_name("OBJZ", object);
        assert_eq!(
            fs::read_to_string(log_dir.join(format!("{}.status", rustc_step)))
                .expect("failed to read rustc replay status")
                .trim(),
            "0",
            "rustc replay should succeed for {}",
            object
        );

        let object_path = worktree_dir.join(object);
        assert!(
            object_path.exists(),
            "expected replayed OBJZ object {}",
            object_path.display()
        );
        assert!(
            fs::metadata(&object_path)
                .expect("failed to stat replayed OBJZ object")
                .len()
                > 0,
            "replayed OBJZ object should be non-empty: {}",
            object_path.display()
        );

        assert!(
            manifest.contains(&format!("object={}", object)),
            "manifest should include OBJZ object {}: {}",
            object,
            manifest
        );
        let manifest_line = manifest
            .lines()
            .find(|line| line.contains(&format!("object={}", object)))
            .expect("manifest should contain per-object replay line");
        let object_size_token = manifest_line
            .split_whitespace()
            .find(|token| token.starts_with("object_size="))
            .expect("manifest object line should include object_size");
        let manifest_object_size: u64 = object_size_token
            .trim_start_matches("object_size=")
            .parse()
            .expect("manifest object_size should parse as u64");
        assert!(
            manifest_object_size > 0,
            "manifest object_size should be positive for {}: {}",
            object,
            manifest_line
        );
    }
}

#[test]
#[ignore = "real-world external project test (downloads and transpiles zlib OBJG objects)"]
fn test_real_world_zlib_fragile_objg_objects_replay() {
    let replay_result = run_zlib_fragile_objg_objects_baseline();

    let log_dir = Path::new(ZLIB_FRAGILE_OBJG_OBJECTS_BASELINE_DIR).join("driver_logs");
    let worktree_dir = Path::new(ZLIB_FRAGILE_OBJG_OBJECTS_BASELINE_DIR).join("worktree");
    for rel in ZLIB_FRAGILE_OBJG_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile-OBJG log {}",
            log_dir.join(rel).display()
        );
    }

    let replay_log_dir = replay_result
        .map_err(|e| format!("full OBJG replay failed unexpectedly: {}", e))
        .expect("OBJG replay should succeed end-to-end");
    assert_eq!(
        replay_log_dir, log_dir,
        "replay helper should return baseline OBJG driver log directory"
    );

    let replay_plan = fs::read_to_string(log_dir.join("libza_replay_plan.txt"))
        .expect("failed to read libza_replay_plan.txt");
    assert!(
        replay_plan.contains("libza_target_count=15"),
        "replay plan should be generated before OBJG replay: {}",
        replay_plan
    );

    let manifest = fs::read_to_string(log_dir.join("fragile_objg_manifest.txt"))
        .expect("failed to read fragile_objg_manifest.txt");
    assert!(
        manifest.contains("replay_scope=OBJG"),
        "manifest should include replay scope: {}",
        manifest
    );
    assert!(
        manifest.contains(&format!("replayed_count={}", ZLIB_OBJG_OBJECTS.len())),
        "manifest should include full OBJG replay count: {}",
        manifest
    );
    for object in ZLIB_OBJG_OBJECTS {
        let rustc_step = rustc_replay_step_name("OBJG", object);
        assert_eq!(
            fs::read_to_string(log_dir.join(format!("{}.status", rustc_step)))
                .expect("failed to read rustc replay status")
                .trim(),
            "0",
            "rustc replay should succeed for {}",
            object
        );

        let object_path = worktree_dir.join(object);
        assert!(
            object_path.exists(),
            "expected replayed OBJG object {}",
            object_path.display()
        );
        assert!(
            fs::metadata(&object_path)
                .expect("failed to stat replayed OBJG object")
                .len()
                > 0,
            "replayed OBJG object should be non-empty: {}",
            object_path.display()
        );

        assert!(
            manifest.contains(&format!("object={}", object)),
            "manifest should include OBJG object {}: {}",
            object,
            manifest
        );
        let manifest_line = manifest
            .lines()
            .find(|line| line.contains(&format!("object={}", object)))
            .expect("manifest should contain per-object replay line");
        let object_size_token = manifest_line
            .split_whitespace()
            .find(|token| token.starts_with("object_size="))
            .expect("manifest object line should include object_size");
        let manifest_object_size: u64 = object_size_token
            .trim_start_matches("object_size=")
            .parse()
            .expect("manifest object_size should parse as u64");
        assert!(
            manifest_object_size > 0,
            "manifest object_size should be positive for {}: {}",
            object,
            manifest_line
        );
    }
}

#[test]
#[ignore = "real-world external project test (downloads and transpiles zlib adler32 object)"]
fn test_real_world_zlib_fragile_adler32_object_replay() {
    let log_dir =
        run_zlib_fragile_adler32_object_baseline().expect("failed to run adler32 object replay");
    for rel in ZLIB_FRAGILE_ADLER32_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile-object replay log {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_object.status"))
            .expect("failed to read rustc_object.status")
            .trim(),
        "0"
    );
    let manifest = fs::read_to_string(log_dir.join("fragile_object_manifest.txt"))
        .expect("failed to read fragile object manifest");
    assert!(
        manifest.contains("source=adler32.c"),
        "manifest should include adler32 source entry: {}",
        manifest
    );
    assert!(
        manifest.contains("object=adler32.o"),
        "manifest should include adler32 object entry: {}",
        manifest
    );
}
