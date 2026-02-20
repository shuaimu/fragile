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

use fragile_clang::{AstCodeGen, ClangParser};

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
    let parser = ClangParser::with_paths_and_defines(include_paths, defines).map_err(|e| {
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
    let err = run_zlib_fragile_objz_objects_baseline()
        .expect_err("OBJZ replay is expected to fail until crc32 codegen typing is fixed");
    assert!(
        err.contains("crc32.o"),
        "failure should surface crc32.o replay blocker: {}",
        err
    );

    let log_dir = Path::new(ZLIB_FRAGILE_OBJZ_OBJECTS_BASELINE_DIR).join("driver_logs");
    for rel in ZLIB_FRAGILE_OBJZ_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile-OBJZ log {}",
            log_dir.join(rel).display()
        );
    }

    let manifest = fs::read_to_string(log_dir.join("libza_replay_plan.txt"))
        .expect("failed to read libza_replay_plan.txt");
    assert!(
        manifest.contains("libza_target_count=15"),
        "replay plan should still be generated before OBJZ replay failure: {}",
        manifest
    );

    assert_eq!(
        fs::read_to_string(log_dir.join("rustc_objz_adler32_o.status"))
            .expect("failed to read adler32 replay status")
            .trim(),
        "0"
    );
    assert_ne!(
        fs::read_to_string(log_dir.join("rustc_objz_crc32_o.status"))
            .expect("failed to read crc32 replay status")
            .trim(),
        "0"
    );
    assert!(
        fs::read_to_string(log_dir.join("rustc_objz_crc32_o.stderr"))
            .expect("failed to read crc32 replay stderr")
            .contains("wrapping_shl"),
        "crc32 failure should capture wrapping_shl typing issue"
    );
    assert!(
        !log_dir.join("fragile_objz_manifest.txt").exists(),
        "OBJZ replay manifest should not be written when replay aborts on crc32 failure"
    );
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
