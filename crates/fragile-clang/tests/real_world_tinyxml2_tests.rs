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

use fragile_clang::{AstCodeGen, ClangParser, ParserLanguage};

const TINYXML2_REPO_URL: &str = "https://github.com/leethomason/tinyxml2.git";
const TINYXML2_PINNED_COMMIT: &str = "9148bdf719e997d1f474be6bcc7943881046dba1"; // 11.0.0
const TINYXML2_CACHE_DIR: &str = "/tmp/fragile_real_world_tinyxml2";
const TINYXML2_NATIVE_BASELINE_DIR: &str = "/tmp/fragile_real_world_tinyxml2_native_baseline";
const TINYXML2_MAKE_TEST_COMMAND_PLAN_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_make_test_command_plan";
const TINYXML2_MAKE_TEST_REPLAY_NATIVE_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_make_test_replay_native";
const TINYXML2_MAKE_TEST_REPLAY_FRAGILE_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_make_test_replay_fragile";
const TINYXML2_CXX_DRIVER_XMLTEST_DIR: &str = "/tmp/fragile_real_world_tinyxml2_cxx_driver_xmltest";
const TINYXML2_FRAGILE_XMLTEST_BUILD_DIR: &str =
    "/tmp/fragile_real_world_tinyxml2_fragile_xmltest_build";
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
const TINYXML2_FRAGILE_XMLTEST_LOG_FILES: &[&str] = &[
    "make_clean_driver.status",
    "make_clean_driver.stdout",
    "make_clean_driver.stderr",
    "make_xmltest_driver.status",
    "make_xmltest_driver.stdout",
    "make_xmltest_driver.stderr",
    "cxx_driver.log",
    "cxx_driver_manifest.txt",
    "compile_units_manifest.txt",
    "link_fragile_xmltest.status",
    "link_fragile_xmltest.stdout",
    "link_fragile_xmltest.stderr",
    "fragile_xmltest_manifest.txt",
    "rustc_fragile_runtime_support.status",
    "rustc_fragile_runtime_support.stdout",
    "rustc_fragile_runtime_support.stderr",
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

#[derive(Clone, Debug)]
struct CompileCommand {
    source_rel: String,
    object_rel: String,
    command_cwd: PathBuf,
    command_tokens: Vec<String>,
}

#[derive(Clone, Debug)]
struct LinkCommand {
    output_rel: String,
    input_paths: Vec<String>,
    command_cwd: PathBuf,
    command_tokens: Vec<String>,
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

fn parse_compile_commands_from_cxx_driver_log(
    driver_log: &str,
    source_dir: &Path,
) -> Result<Vec<CompileCommand>, String> {
    let mut commands: std::collections::BTreeMap<(String, String), CompileCommand> =
        std::collections::BTreeMap::new();
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
        if tokens.is_empty() {
            continue;
        }
        if !tokens.iter().any(|tok| *tok == "-c" || tok.starts_with("-c")) {
            continue;
        }

        let Some((object_raw, object_consumed_idx)) = extract_arg_value(&tokens, "-o") else {
            continue;
        };
        let Some(source_raw) = extract_compile_source_token(&tokens, &object_raw, object_consumed_idx)
        else {
            continue;
        };

        if !is_c_family_source_token(&source_raw) {
            continue;
        }

        let source_rel = normalize_path_for_manifest(&source_raw, &command_cwd, source_dir);
        let object_rel = normalize_path_for_manifest(&object_raw, &command_cwd, source_dir);
        let key = (source_rel.clone(), object_rel.clone());
        commands.entry(key).or_insert_with(|| CompileCommand {
            source_rel,
            object_rel,
            command_cwd: command_cwd.clone(),
            command_tokens: tokens.iter().map(|token| (*token).to_string()).collect(),
        });
    }

    if commands.is_empty() {
        return Err("no compile units found in cxx_driver.log".to_string());
    }
    Ok(commands.into_values().collect())
}

fn parse_compile_units_from_cxx_driver_log(
    driver_log: &str,
    source_dir: &Path,
) -> Result<Vec<(String, String)>, String> {
    let commands = parse_compile_commands_from_cxx_driver_log(driver_log, source_dir)?;
    Ok(commands
        .into_iter()
        .map(|cmd| (cmd.source_rel, cmd.object_rel))
        .collect())
}

fn parse_link_commands_from_cxx_driver_log(
    driver_log: &str,
    source_dir: &Path,
) -> Result<Vec<LinkCommand>, String> {
    let mut commands: std::collections::BTreeMap<String, LinkCommand> =
        std::collections::BTreeMap::new();
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
        if tokens.is_empty() {
            continue;
        }
        if tokens.iter().any(|tok| *tok == "-c" || tok.starts_with("-c")) {
            continue;
        }

        let Some((output_raw, output_consumed_idx)) = extract_arg_value(&tokens, "-o") else {
            continue;
        };
        let output_rel = normalize_path_for_manifest(&output_raw, &command_cwd, source_dir);
        let mut input_paths: Vec<String> = Vec::new();
        let mut skip_next = false;
        for (idx, token) in tokens.iter().enumerate() {
            if skip_next {
                skip_next = false;
                continue;
            }
            if *token == "-o" {
                skip_next = true;
                continue;
            }
            if output_consumed_idx.is_some_and(|i| i == idx) {
                continue;
            }
            if token.starts_with('-') {
                continue;
            }
            let normalized = normalize_path_for_manifest(token, &command_cwd, source_dir);
            if normalized == output_rel {
                continue;
            }
            if !input_paths.contains(&normalized) {
                input_paths.push(normalized);
            }
        }

        if input_paths.is_empty() {
            continue;
        }

        commands.entry(output_rel.clone()).or_insert_with(|| LinkCommand {
            output_rel,
            input_paths,
            command_cwd: command_cwd.clone(),
            command_tokens: tokens.iter().map(|token| (*token).to_string()).collect(),
        });
    }

    if commands.is_empty() {
        return Err("no link commands found in cxx_driver.log".to_string());
    }
    Ok(commands.into_values().collect())
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
    let mut crate_name = String::from("tinyxml2_");
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
    if crate_name == "tinyxml2" || crate_name == "tinyxml2_" {
        "tinyxml2_unit".to_string()
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
        .arg("opt-level=3")
        .arg("-C")
        .arg("debuginfo=2")
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
    let runtime_source_path = log_dir.join("fragile_runtime_support.rs");
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
        .arg("fragile_runtime_support")
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
    write_command_capture(log_dir, "rustc_fragile_runtime_support", &rustc_output)?;
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

fn sanitize_for_path_component(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn rustc_fragile_step_name(object_rel: &str) -> String {
    format!("rustc_fragile_{}", sanitize_for_path_component(object_rel))
}

fn append_c_main_export_shim_if_present(transpiled: &mut String) {
    if transpiled.contains("export_name = \"main\"") {
        return;
    }

    let mut main_params: Option<String> = None;
    let mut main_returns_i32 = false;
    for line in transpiled.lines() {
        let Some(main_idx) = line.find("fn main(") else {
            continue;
        };
        let sig_tail = &line[main_idx + "fn main(".len()..];
        let Some(close_idx) = sig_tail.find(')') else {
            continue;
        };
        main_params = Some(sig_tail[..close_idx].trim().to_string());
        main_returns_i32 = sig_tail[close_idx + 1..].contains("-> i32");
        break;
    }

    let Some(main_params) = main_params else {
        return;
    };

    let param_count = main_params
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .count();
    let call_expr = match param_count {
        0 => "main()",
        1 => "main(argc)",
        _ => "main(argc, argv)",
    };
    if main_returns_i32 {
        transpiled.push_str(&format!(
            "\n#[export_name = \"main\"]\npub extern \"C\" fn fragile_exported_main(argc: i32, argv: *mut *const i8) -> i32 {{\n    {}\n}}\n",
            call_expr
        ));
    } else {
        transpiled.push_str(&format!(
            "\n#[export_name = \"main\"]\npub extern \"C\" fn fragile_exported_main(argc: i32, argv: *mut *const i8) -> i32 {{\n    {};\n    0\n}}\n",
            call_expr
        ));
    }
}

fn select_link_command_for_output(
    link_commands: &[LinkCommand],
    output_name: &str,
) -> Result<LinkCommand, String> {
    for command in link_commands {
        if Path::new(&command.output_rel)
            .file_name()
            .and_then(|s| s.to_str())
            == Some(output_name)
        {
            return Ok(command.clone());
        }
    }
    Err(format!(
        "link command for output {} not found in cxx_driver.log",
        output_name
    ))
}

fn write_fragile_xmltest_manifest(
    log_dir: &Path,
    source_dir: &Path,
    runtime_support: &RustRuntimeSupportInputs,
    staged_binary_path: &Path,
    replay_binary_path: &Path,
    replayed_entries: &[(String, String, PathBuf, PathBuf, u64)],
) -> Result<(), String> {
    let head = read_head(source_dir).unwrap_or_else(|| "unknown".to_string());
    let staged_binary_size = fs::metadata(staged_binary_path)
        .map_err(|e| format!("failed to stat staged binary {}: {}", staged_binary_path.display(), e))?
        .len();
    let replay_binary_size = fs::metadata(replay_binary_path)
        .map_err(|e| format!("failed to stat replay binary {}: {}", replay_binary_path.display(), e))?
        .len();
    let mut manifest = format!(
        "source_dir={}\ncommit={}\nstaged_binary={}\nstaged_binary_size={}\nreplay_binary={}\nreplay_binary_size={}\nruntime_support_archive={}\nruntime_support_archive_size={}\nruntime_support_native_static_libs={}\nreplayed_compile_unit_count={}\n",
        source_dir.display(),
        head.trim(),
        staged_binary_path.display(),
        staged_binary_size,
        replay_binary_path.display(),
        replay_binary_size,
        runtime_support.archive_path.display(),
        runtime_support.archive_size,
        runtime_support.native_static_libs.join(" "),
        replayed_entries.len(),
    );
    for (source_rel, object_rel, transpiled_rs_path, staged_object, object_size) in replayed_entries {
        manifest.push_str(&format!(
            "source={} object={} transpiled_rust={} staged_object={} object_size={}\n",
            source_rel,
            object_rel,
            transpiled_rs_path.display(),
            staged_object.display(),
            object_size
        ));
    }
    fs::write(log_dir.join("fragile_xmltest_manifest.txt"), manifest).map_err(|e| {
        format!(
            "failed to write fragile xmltest manifest at {}: {}",
            log_dir.display(),
            e
        )
    })
}

fn stage_compile_command_object(
    source_dir: &Path,
    log_dir: &Path,
    stage_object_root: &Path,
    command: &CompileCommand,
) -> Result<(PathBuf, PathBuf, u64), String> {
    let source_path = source_dir.join(&command.source_rel);
    if !source_path.exists() {
        return Err(format!(
            "compile source {} does not exist under {}",
            command.source_rel,
            source_dir.display()
        ));
    }

    let mut transpiled = transpile_source_with_driver_command(
        &source_path,
        &command.command_cwd,
        &command.command_tokens,
    )?;
    append_c_main_export_shim_if_present(&mut transpiled);
    let transpiled_rs_path = log_dir.join(format!(
        "fragile_{}_transpiled.rs",
        sanitize_for_path_component(&command.object_rel)
    ));
    fs::write(&transpiled_rs_path, transpiled).map_err(|e| {
        format!(
            "failed to write transpiled source {}: {}",
            transpiled_rs_path.display(),
            e
        )
    })?;

    let staged_object_path = stage_object_root.join(&command.object_rel);
    let compile_output = compile_rust_source_to_object(
        &transpiled_rs_path,
        &staged_object_path,
        &crate_name_from_source(&command.source_rel),
    )?;
    let rustc_step = rustc_fragile_step_name(&command.object_rel);
    write_command_capture(log_dir, &rustc_step, &compile_output)?;
    if !compile_output.status.success() {
        return Err(format!(
            "fragile rustc object build failed for {} with status {} (logs: {})",
            command.object_rel,
            status_code(&compile_output),
            log_dir.display()
        ));
    }

    let object_size = fs::metadata(&staged_object_path)
        .map_err(|e| {
            format!(
                "failed to stat staged object {}: {}",
                staged_object_path.display(),
                e
            )
        })?
        .len();
    if object_size == 0 {
        return Err(format!(
            "staged object {} is empty",
            staged_object_path.display()
        ));
    }

    Ok((transpiled_rs_path, staged_object_path, object_size))
}

fn run_fragile_xmltest_build_from_cxx_driver_plan_in_tree(
    source_dir: &Path,
    log_dir: &Path,
) -> Result<(), String> {
    run_cxx_driver_xmltest_baseline_in_tree(source_dir, log_dir)?;

    let driver_log_path = log_dir.join("cxx_driver.log");
    let driver_log = fs::read_to_string(&driver_log_path)
        .map_err(|e| format!("failed to read {}: {}", driver_log_path.display(), e))?;
    let compile_commands = parse_compile_commands_from_cxx_driver_log(&driver_log, source_dir)?;
    let link_commands = parse_link_commands_from_cxx_driver_log(&driver_log, source_dir)?;
    let link_command = select_link_command_for_output(&link_commands, "xmltest")?;

    let mut compile_by_object: std::collections::BTreeMap<String, CompileCommand> =
        std::collections::BTreeMap::new();
    let mut compile_by_source: std::collections::BTreeMap<String, CompileCommand> =
        std::collections::BTreeMap::new();
    for command in compile_commands {
        compile_by_source.insert(command.source_rel.clone(), command.clone());
        compile_by_object.insert(command.object_rel.clone(), command);
    }

    let runtime_support = build_rust_runtime_support_inputs(log_dir)?;
    let stage_root = log_dir.join("fragile_stage");
    let stage_object_root = stage_root.join("objects");
    let stage_archive_root = stage_root.join("archives");
    fs::create_dir_all(&stage_object_root).map_err(|e| {
        format!(
            "failed to create fragile stage object dir {}: {}",
            stage_object_root.display(),
            e
        )
    })?;
    fs::create_dir_all(&stage_archive_root).map_err(|e| {
        format!(
            "failed to create fragile stage archive dir {}: {}",
            stage_archive_root.display(),
            e
        )
    })?;

    let mut staged_objects_by_object: std::collections::BTreeMap<String, PathBuf> =
        std::collections::BTreeMap::new();
    let mut staged_objects_by_source: std::collections::BTreeMap<String, PathBuf> =
        std::collections::BTreeMap::new();
    let mut staged_archives: std::collections::BTreeMap<String, PathBuf> =
        std::collections::BTreeMap::new();
    let mut replayed_entries: Vec<(String, String, PathBuf, PathBuf, u64)> = Vec::new();
    for command in compile_by_object.values() {
        let (transpiled_rs_path, staged_object_path, object_size) =
            stage_compile_command_object(source_dir, log_dir, &stage_object_root, command)?;
        staged_objects_by_object.insert(command.object_rel.clone(), staged_object_path.clone());
        staged_objects_by_source.insert(command.source_rel.clone(), staged_object_path.clone());
        replayed_entries.push((
            command.source_rel.clone(),
            command.object_rel.clone(),
            transpiled_rs_path,
            staged_object_path,
            object_size,
        ));
    }

    let staged_binary_path = stage_root.join("xmltest_fragile");
    if let Some(parent) = staged_binary_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create staged binary parent {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let mut link_args: Vec<String> = Vec::new();
    let mut idx = 0usize;
    while idx < link_command.command_tokens.len() {
        let token = &link_command.command_tokens[idx];
        if token == "-o" {
            link_args.push("-o".to_string());
            link_args.push(staged_binary_path.to_string_lossy().to_string());
            idx += 2;
            continue;
        }
        if token.starts_with("-o") && token.len() > 2 {
            link_args.push("-o".to_string());
            link_args.push(staged_binary_path.to_string_lossy().to_string());
            idx += 1;
            continue;
        }
        if !token.starts_with('-') {
            let normalized =
                normalize_path_for_manifest(token, &link_command.command_cwd, source_dir);
            if let Some(staged_object) = staged_objects_by_source.get(&normalized) {
                link_args.push(staged_object.to_string_lossy().to_string());
                idx += 1;
                continue;
            }
            if let Some(staged_object) = staged_objects_by_object.get(&normalized) {
                link_args.push(staged_object.to_string_lossy().to_string());
                idx += 1;
                continue;
            }
            if normalized == link_command.output_rel {
                idx += 1;
                continue;
            }
            if is_c_family_source_token(&normalized) {
                let source_rel = normalized.clone();
                let source_stem = Path::new(&source_rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unit");
                let object_rel = format!("{}.o", source_stem);
                let command = if let Some(existing) = compile_by_source.get(&source_rel) {
                    existing.clone()
                } else {
                    CompileCommand {
                        source_rel: source_rel.clone(),
                        object_rel: object_rel.clone(),
                        command_cwd: link_command.command_cwd.clone(),
                        command_tokens: link_command.command_tokens.clone(),
                    }
                };
                let (transpiled_rs_path, staged_object_path, object_size) =
                    stage_compile_command_object(source_dir, log_dir, &stage_object_root, &command)?;
                staged_objects_by_object.insert(command.object_rel.clone(), staged_object_path.clone());
                staged_objects_by_source.insert(command.source_rel.clone(), staged_object_path.clone());
                replayed_entries.push((
                    command.source_rel.clone(),
                    command.object_rel.clone(),
                    transpiled_rs_path,
                    staged_object_path.clone(),
                    object_size,
                ));
                link_args.push(staged_object_path.to_string_lossy().to_string());
                idx += 1;
                continue;
            }
            if normalized.ends_with(".a") {
                if let Some(existing_archive) = staged_archives.get(&normalized) {
                    link_args.push(existing_archive.to_string_lossy().to_string());
                    idx += 1;
                    continue;
                }
                let archive_stem = Path::new(&normalized)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if let Some(lib_stem) = archive_stem.strip_prefix("lib") {
                    let candidate_object = format!("{}.o", lib_stem);
                    if let Some(staged_object) = staged_objects_by_object.get(&candidate_object) {
                        let archive_name = Path::new(&normalized)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("libfragile.a");
                        let staged_archive = stage_archive_root.join(archive_name);
                        let ar_output = Command::new("ar")
                            .arg("cr")
                            .arg(&staged_archive)
                            .arg(staged_object)
                            .current_dir(source_dir)
                            .output()
                            .map_err(|e| {
                                format!(
                                    "failed to run ar for staged archive {}: {}",
                                    staged_archive.display(),
                                    e
                                )
                            })?;
                        let ar_step = format!("ar_{}", sanitize_for_path_component(&normalized));
                        write_command_capture(log_dir, &ar_step, &ar_output)?;
                        if !ar_output.status.success() {
                            return Err(format!(
                                "failed to build staged archive {} with status {} (logs: {})",
                                staged_archive.display(),
                                status_code(&ar_output),
                                log_dir.display()
                            ));
                        }
                        let ranlib_output = Command::new("ranlib")
                            .arg(&staged_archive)
                            .current_dir(source_dir)
                            .output()
                            .map_err(|e| {
                                format!(
                                    "failed to run ranlib for staged archive {}: {}",
                                    staged_archive.display(),
                                    e
                                )
                            })?;
                        let ranlib_step =
                            format!("ranlib_{}", sanitize_for_path_component(&normalized));
                        write_command_capture(log_dir, &ranlib_step, &ranlib_output)?;
                        if !ranlib_output.status.success() {
                            return Err(format!(
                                "failed to index staged archive {} with status {} (logs: {})",
                                staged_archive.display(),
                                status_code(&ranlib_output),
                                log_dir.display()
                            ));
                        }
                        staged_archives.insert(normalized.clone(), staged_archive.clone());
                        link_args.push(staged_archive.to_string_lossy().to_string());
                        idx += 1;
                        continue;
                    }
                }
            }
            if normalized.ends_with(".o") {
                return Err(format!(
                    "missing compile units for link input objects: {}",
                    normalized
                ));
            }
        }
        link_args.push(token.clone());
        idx += 1;
    }
    link_args.push(runtime_support.archive_path.to_string_lossy().to_string());
    for lib_flag in &runtime_support.native_static_libs {
        link_args.push(lib_flag.clone());
    }

    let link_output = Command::new("c++")
        .args(&link_args)
        .current_dir(source_dir)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .output()
        .map_err(|e| format!("failed to run fragile xmltest link command: {}", e))?;
    write_command_capture(log_dir, "link_fragile_xmltest", &link_output)?;
    if !link_output.status.success() {
        return Err(format!(
            "fragile xmltest link failed with status {} (logs: {})",
            status_code(&link_output),
            log_dir.display()
        ));
    }

    let staged_binary_size = fs::metadata(&staged_binary_path)
        .map_err(|e| {
            format!(
                "failed to stat staged fragile binary {}: {}",
                staged_binary_path.display(),
                e
            )
        })?
        .len();
    if staged_binary_size == 0 {
        return Err(format!(
            "staged fragile binary {} is empty",
            staged_binary_path.display()
        ));
    }

    let replay_binary_path = source_dir.join("xmltest");
    fs::copy(&staged_binary_path, &replay_binary_path).map_err(|e| {
        format!(
            "failed to stage fragile xmltest binary at {}: {}",
            replay_binary_path.display(),
            e
        )
    })?;
    make_executable(&replay_binary_path)?;
    write_fragile_xmltest_manifest(
        log_dir,
        source_dir,
        &runtime_support,
        &staged_binary_path,
        &replay_binary_path,
        &replayed_entries,
    )?;
    Ok(())
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
        .env("CXXLD", cxx_driver_str.as_str())
        .env("LINK", cxx_driver_str.as_str())
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
        .env("CXXLD", cxx_driver_str.as_str())
        .env("LINK", cxx_driver_str.as_str())
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
    run_fragile_xmltest_build_from_cxx_driver_plan_in_tree(source_dir, log_dir)?;
    run_make_test_command_replay_in_tree(source_dir, log_dir)?;
    Ok(())
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

fn run_tinyxml2_make_test_command_replay_fragile() -> Result<PathBuf, String> {
    let checkout_dir = ensure_tinyxml2_checkout()?;
    let baseline_root = PathBuf::from(TINYXML2_MAKE_TEST_REPLAY_FRAGILE_DIR);
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
            "make-test replay fragile worktree expected commit {} but got {}",
            TINYXML2_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("replay_logs");
    run_tinyxml2_fragile_make_test_command_replay_in_tree(&worktree_dir, &log_dir)?;
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

fn run_tinyxml2_fragile_xmltest_build_baseline() -> Result<PathBuf, String> {
    let checkout_dir = ensure_tinyxml2_checkout()?;
    let baseline_root = PathBuf::from(TINYXML2_FRAGILE_XMLTEST_BUILD_DIR);
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
            "fragile xmltest build worktree expected commit {} but got {}",
            TINYXML2_PINNED_COMMIT, actual_head
        ));
    }

    let log_dir = baseline_root.join("fragile_build_logs");
    run_fragile_xmltest_build_from_cxx_driver_plan_in_tree(&worktree_dir, &log_dir)?;
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

fn collect_fail_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| line.starts_with("[fail] "))
        .collect()
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
        "#include \"tinyxml2.h\"\nint main(void) { return 0; }\n",
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
fn test_append_c_main_export_shim_handles_pub_main_with_argv() {
    let mut transpiled = "\npub fn main(argc: i32, argv: *mut *const i8) -> i32 {\n    argc + if argv.is_null() { 0 } else { 1 }\n}\n".to_string();
    append_c_main_export_shim_if_present(&mut transpiled);
    assert!(
        transpiled.contains("#[export_name = \"main\"]"),
        "expected exported C main shim to be appended"
    );
    assert!(
        transpiled.contains("fragile_exported_main(argc: i32, argv: *mut *const i8) -> i32"),
        "expected shim signature with argc/argv, got:\n{}",
        transpiled
    );
    assert!(
        transpiled.contains("main(argc, argv)"),
        "expected argv-aware shim to forward args to transpiled main, got:\n{}",
        transpiled
    );
}

#[test]
fn test_append_c_main_export_shim_handles_zero_arg_main() {
    let mut transpiled = "\nfn main() -> i32 {\n    7\n}\n".to_string();
    append_c_main_export_shim_if_present(&mut transpiled);
    assert!(
        transpiled.contains("#[export_name = \"main\"]"),
        "expected exported C main shim to be appended"
    );
    assert!(
        transpiled.contains("main()"),
        "expected zero-arg shim to call transpiled main() directly, got:\n{}",
        transpiled
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
fn test_parse_link_commands_from_cxx_driver_log_normalizes_and_deduplicates() {
    let source_dir = Path::new("/tmp/tinyxml2_driver_link_parse");
    let driver_log = "\
cwd=/tmp/tinyxml2_driver_link_parse\n\
args=-std=c++11 -O2 xmltest.o tinyxml2.o -o xmltest \n\
cwd=/tmp/tinyxml2_driver_link_parse\n\
args=-std=c++11 -O2 ./xmltest.o ./tinyxml2.o -o ./xmltest \n";

    let link_commands = parse_link_commands_from_cxx_driver_log(driver_log, source_dir)
        .expect("CXX-driver parse should capture link commands");
    assert_eq!(
        link_commands.len(),
        1,
        "link command parser should deduplicate by output path"
    );
    assert_eq!(link_commands[0].output_rel, "xmltest".to_string());
    assert_eq!(
        link_commands[0].input_paths,
        vec!["xmltest.o".to_string(), "tinyxml2.o".to_string()],
        "link command parser should normalize object inputs"
    );
}

#[test]
fn test_parse_link_commands_from_cxx_driver_log_reports_missing_link_commands() {
    let source_dir = Path::new("/tmp/tinyxml2_driver_link_parse_empty");
    let driver_log = "\
cwd=/tmp/tinyxml2_driver_link_parse_empty\n\
args=-std=c++11 -O2 -c xmltest.cpp -o xmltest.o \n";

    let err = parse_link_commands_from_cxx_driver_log(driver_log, source_dir)
        .expect_err("parser should fail when CXX driver log has no link commands");
    assert!(
        err.contains("no link commands found in cxx_driver.log"),
        "missing-link error should be explicit, got: {}",
        err
    );
}

#[test]
fn test_parse_link_commands_from_cxx_driver_log_keeps_source_and_archive_inputs() {
    let source_dir = Path::new("/tmp/tinyxml2_driver_link_parse_src_archive");
    let driver_log = "\
cwd=/tmp/tinyxml2_driver_link_parse_src_archive\n\
args=-D_FILE_OFFSET_BITS=64 -fPIC xmltest.cpp libtinyxml2.a -o xmltest \n";

    let link_commands = parse_link_commands_from_cxx_driver_log(driver_log, source_dir)
        .expect("CXX-driver parse should capture source+archive link command");
    assert_eq!(link_commands.len(), 1);
    assert_eq!(
        link_commands[0].input_paths,
        vec!["xmltest.cpp".to_string(), "libtinyxml2.a".to_string()],
        "link command parser should preserve source and archive inputs"
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
fn test_fragile_xmltest_build_from_cxx_driver_plan_local_fixture_success() {
    let root = unique_temp_dir("tinyxml2_fragile_xmltest_build_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_tinyxml2_cxx_driver_project(&root)
        .expect("failed to create local tinyxml2 CXX-driver project");
    let log_dir = root.join("fragile_build_logs");
    run_fragile_xmltest_build_from_cxx_driver_plan_in_tree(&project_dir, &log_dir)
        .expect("fragile xmltest build should succeed for local fixture");

    for rel in TINYXML2_FRAGILE_XMLTEST_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile xmltest build artifact {}",
            log_dir.join(rel).display()
        );
    }
    for object_rel in ["xmltest.o", "tinyxml2.o"] {
        let step = rustc_fragile_step_name(object_rel);
        assert_eq!(
            read_status_file(&log_dir.join(format!("{}.status", step)))
                .expect("failed to read fragile rustc replay status"),
            0,
            "fragile rustc replay should succeed for {}",
            object_rel
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("link_fragile_xmltest.status"))
            .expect("failed to read link_fragile_xmltest.status"),
        0,
        "fragile xmltest link should succeed"
    );

    let staged_binary = project_dir.join("xmltest");
    assert!(
        staged_binary.exists(),
        "staged fragile replay binary should exist at {}",
        staged_binary.display()
    );
    let staged_status = Command::new("./xmltest")
        .current_dir(&project_dir)
        .output()
        .expect("failed to execute staged fragile xmltest")
        .status;
    assert!(
        staged_status.success(),
        "staged fragile xmltest binary should execute successfully"
    );

    let manifest = fs::read_to_string(log_dir.join("fragile_xmltest_manifest.txt"))
        .expect("failed to read fragile_xmltest_manifest.txt");
    assert!(
        manifest.contains("replayed_compile_unit_count=2"),
        "manifest should record compile unit replay count, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("source=xmltest.cpp object=xmltest.o"),
        "manifest should include xmltest compile unit mapping, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("source=tinyxml2.cpp object=tinyxml2.o"),
        "manifest should include tinyxml2 compile unit mapping, got:\n{}",
        manifest
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_fragile_xmltest_build_reports_missing_link_command() {
    let root = unique_temp_dir("tinyxml2_fragile_xmltest_build_missing_link");
    fs::create_dir_all(&root).expect("failed to create test root");
    let project_dir = root.join("tinyxml2_missing_link_project");
    fs::create_dir_all(&project_dir).expect("failed to create project dir");
    fs::write(
        project_dir.join("tinyxml2.h"),
        "#pragma once\nint tinyxml2_fixture_value(void);\n",
    )
    .expect("failed to write tinyxml2.h");
    fs::write(
        project_dir.join("tinyxml2.cpp"),
        "#include \"tinyxml2.h\"\nint tinyxml2_fixture_value(void) { return 7; }\n",
    )
    .expect("failed to write tinyxml2.cpp");
    fs::write(
        project_dir.join("xmltest.cpp"),
        "#include \"tinyxml2.h\"\nint main(void) { return tinyxml2_fixture_value() == 7 ? 0 : 1; }\n",
    )
    .expect("failed to write xmltest.cpp");
    fs::write(
        project_dir.join("Makefile"),
        "\
CXX ?= c++\n\
CXXFLAGS ?= -std=c++11 -O2\n\
\n\
xmltest:\n\
\t$(CXX) $(CXXFLAGS) -c xmltest.cpp -o xmltest.o\n\
\t$(CXX) $(CXXFLAGS) -c tinyxml2.cpp -o tinyxml2.o\n\
\t@printf '%s\\n' '#!/bin/sh' 'exit 0' > xmltest\n\
\t@chmod +x xmltest\n\
\n\
clean:\n\
\t$(RM) xmltest xmltest.o tinyxml2.o\n",
    )
    .expect("failed to write fixture Makefile");

    let log_dir = root.join("fragile_build_logs");
    let err = run_fragile_xmltest_build_from_cxx_driver_plan_in_tree(&project_dir, &log_dir)
        .expect_err("fragile build should fail when cxx driver log has no link command");
    assert!(
        err.contains("no link commands found in cxx_driver.log"),
        "missing-link diagnostic should be explicit, got: {}",
        err
    );
    assert!(
        !log_dir.join("link_fragile_xmltest.status").exists(),
        "fragile link stage should not run when link command is missing"
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
fn test_collect_fail_lines_keeps_order() {
    let sample = "\
[pass] A\n\
[fail] First problem\n\
[pass] B\n\
[fail] Second problem\n";
    let fails = collect_fail_lines(sample);
    assert_eq!(fails, vec!["[fail] First problem", "[fail] Second problem"]);
}

#[test]
fn test_make_test_command_replay_local_fixture_fragile_runner_builds_and_replays_successfully() {
    let root = unique_temp_dir("tinyxml2_make_test_replay_fragile_success");
    fs::create_dir_all(&root).expect("failed to create test root");

    let project_dir = create_local_tinyxml2_cxx_driver_project(&root)
        .expect("failed to create local tinyxml2 CXX-driver project fixture");

    let log_dir = root.join("replay_logs_fragile");
    run_tinyxml2_fragile_make_test_command_replay_in_tree(&project_dir, &log_dir)
        .expect("fragile replay runner should build/stage xmltest and replay successfully");

    for rel in TINYXML2_FRAGILE_XMLTEST_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile build artifact {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("link_fragile_xmltest.status"))
            .expect("failed to read link_fragile_xmltest.status"),
        0,
        "fragile replay runner should stage xmltest from transpiled build before replay"
    );

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
        "fragile replay command should succeed after automatic fragile build/stage"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_make_test_command_replay_local_fixture_fragile_runner_reports_missing_compile_coverage() {
    let root = unique_temp_dir("tinyxml2_make_test_replay_fragile_missing_compile_coverage");
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
        "test: xmltest\n\t@./xmltest\n\nxmltest:\n\t@printf '%s\\n' '#!/bin/sh' 'echo \"xmltest fixture: Pass 1, Fail 0\"' > xmltest\n\t@chmod +x xmltest\n\nclean:\n\t@rm -f xmltest\n",
    )
    .expect("failed to update missing-coverage fixture Makefile with clean target");

    let log_dir = root.join("replay_logs_fragile");
    let err = run_tinyxml2_fragile_make_test_command_replay_in_tree(&checkout_dir, &log_dir)
        .expect_err("fragile replay runner should fail when compile coverage for xmltest build is unavailable");
    assert!(
        err.contains("no compile units found in cxx_driver.log"),
        "missing compile coverage should fail during fragile build staging, got: {}",
        err
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_xmltest_driver.status"))
            .expect("failed to read make_xmltest_driver.status"),
        0,
        "fixture make xmltest step should still run successfully before compile-coverage validation"
    );
    assert!(
        !log_dir.join("make_test_replay_01.status").exists(),
        "replay should not start when fragile xmltest build staging fails"
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
#[ignore = "real-world external project test (builds/stages fragile tinyxml2 xmltest from captured compile/link plan)"]
fn test_real_world_tinyxml2_fragile_xmltest_build_from_cxx_driver_plan() {
    let log_dir = run_tinyxml2_fragile_xmltest_build_baseline()
        .expect("failed to run tinyxml2 fragile xmltest build baseline");
    for rel in TINYXML2_FRAGILE_XMLTEST_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile xmltest build artifact {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("link_fragile_xmltest.status"))
            .expect("failed to read link_fragile_xmltest.status"),
        0,
        "real-world fragile xmltest link should succeed"
    );

    let manifest = fs::read_to_string(log_dir.join("fragile_xmltest_manifest.txt"))
        .expect("failed to read fragile_xmltest_manifest.txt");
    assert!(
        manifest.contains("replayed_compile_unit_count="),
        "manifest should record replayed compile unit count, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("source=tinyxml2.cpp object=tinyxml2.o"),
        "manifest should include tinyxml2 compile unit mapping, got:\n{}",
        manifest
    );
    assert!(
        manifest.contains("source=xmltest.cpp object=xmltest.o"),
        "manifest should include xmltest compile unit mapping, got:\n{}",
        manifest
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

#[test]
#[ignore = "real-world external project test (captures current tinyxml2 fragile replay runtime blocker)"]
fn test_real_world_tinyxml2_make_test_command_subset_replay_fragile() {
    let err = run_tinyxml2_make_test_command_replay_fragile()
        .expect_err("fragile replay is expected to fail at command 1 until runtime blocker is resolved");
    assert!(
        err.contains("make-test command replay failed at command 1 with status 56"),
        "expected command-1 non-crashing blocker message, got: {}",
        err
    );
    let log_dir = PathBuf::from(TINYXML2_MAKE_TEST_REPLAY_FRAGILE_DIR).join("replay_logs");
    for rel in TINYXML2_MAKE_TEST_COMMAND_PLAN_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected replay prerequisite file {}",
            log_dir.join(rel).display()
        );
    }
    for rel in TINYXML2_FRAGILE_XMLTEST_LOG_FILES {
        assert!(
            log_dir.join(rel).exists(),
            "expected fragile xmltest build artifact {}",
            log_dir.join(rel).display()
        );
    }
    assert_eq!(
        read_status_file(&log_dir.join("link_fragile_xmltest.status"))
            .expect("failed to read link_fragile_xmltest.status"),
        0,
        "fragile replay should build and stage xmltest before command replay"
    );
    assert_eq!(
        read_status_file(&log_dir.join("make_test_replay_01.status"))
            .expect("failed to read make_test_replay_01.status"),
        56,
        "current blocker should surface as non-crashing status 56 on replay command 1"
    );
    let replay_stderr = fs::read_to_string(log_dir.join("make_test_replay_01.stderr"))
        .expect("failed to read make_test_replay_01.stderr");
    assert!(
        !replay_stderr.contains("Segmentation fault"),
        "command-1 blocker should be non-crashing; got stderr:\n{}",
        replay_stderr
    );
    let replay_stdout = fs::read_to_string(log_dir.join("make_test_replay_01.stdout"))
        .expect("failed to read make_test_replay_01.stdout");
    let fail_lines = collect_fail_lines(&replay_stdout);
    assert!(
        !fail_lines.is_empty(),
        "expected deterministic fail signatures in replay stdout"
    );
    assert_eq!(
        fail_lines[0],
        "[fail] Document Clear()'s [true][false]",
        "unexpected first fail signature; got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Ill formed XML [true][false]"),
        "ill-formed-XML parse-error signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] IntText() test [][]"),
        "IntText parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] UnsignedText() test [][]"),
        "UnsignedText parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Int64Text() test [][]"),
        "Int64Text parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] DoubleText() test [][]"),
        "DoubleText parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] FloatText()) test [][]"),
        "FloatText parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] FloatText()) test [true][false]"),
        "BoolText parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] IntText() hex value test [][]"),
        "IntText hex parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] UnsignedText() hex value test [][]"),
        "UnsignedText hex parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Int64Text() hex value test [][]"),
        "Int64Text hex parse/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Infinite loop in shallow equal. [true][false]"),
        "infinite-loop shallow-equal signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] ShallowEqual() test [false][true]"),
        "ShallowEqual mismatch signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Element names with lead digit fail to parse. [true][false]"),
        "lead-digit element-name parse-error signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] NextSiblingElement() test [true][false]"),
        "NextSiblingElement sibling-navigation signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines.iter().any(|line| *line == "[fail] QueryIntText"),
        "QueryIntText signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] QueryUnsignedText"),
        "QueryUnsignedText signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] QueryFloatText"),
        "QueryFloatText signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] QueryDoubleText"),
        "QueryDoubleText signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines.iter().any(|line| *line == "[fail] QueryBoolText"),
        "QueryBoolText signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] BOM and default declaration"),
        "BOM/default-declaration signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines.iter().any(|line| *line == "[fail] CStrSize"),
        "BOM declaration CStrSize signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Handle, non-const, element name matches [sub][text]"),
        "handle nested-element name-match signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Handle, non-const, element not found [true][false]"),
        "handle non-const missing-element signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Handle, const, element not found [true][false]"),
        "handle const missing-element signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Clone and Equal [][]"),
        "Clone and Equal sibling parity signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] No closing element [true][false]"),
        "no-closing-element signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Comments iterate correctly. [][]"),
        "comment-iteration signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Missing end tag at end of input [true][false]"),
        "missing-end-tag signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Missing end tag with trailing whitespace [true][false]"),
        "missing-end-tag-with-trailing-whitespace signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Throw error with bad end quotes. [true][false]"),
        "malformed trailing-quote parse-error signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Low entities. [\u{000e}][(null)]"),
        "low-entity decoding/value signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Embedded null throws error. [true][false]"),
        "embedded-null parse-error signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Entity with one digit."),
        "one-digit entity signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Correct value of unknown. [DOCTYPE PLAY SYSTEM 'play.dtd'][DOCTYPE PLAY SYSTEM \"play.dtd\"]"),
        "unknown-node value quoting signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Entity transformation: write. "),
        "entity transformation write signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] dot in names [a.elem][root]"),
        "dot-in-names element-name signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] dot in names [2.0][(null)]"),
        "dot-in-names attribute signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Entity transformation: read.  [Line 5 has \"quotation marks\" and 'apostrophe marks'. It also has <, >, and &, as well as a fake copyright ©.][(null)]"),
        "entity transformation read signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] No entity parsing. [Line 5 has &quot;quotation marks&quot; and &apos;apostrophe marks&apos;.][(null)]"),
        "no-entity parsing context signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] No entity parsing. [Crazy &ttk;][Tinyxml2]"),
        "no-entity parsing text signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines.iter().any(|line| *line == "[fail] CDATA parse."),
        "CDATA parse signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] CDATA parse. [ tixml1:1480107 ]"),
        "secondary CDATA parse signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Broken CDATA [true][false]"),
        "broken CDATA error signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] PushDeclaration() test [version = '1.0' encoding = 'utf-8'][xml version=\"1.0\"]"),
        "PushDeclaration declaration-parity signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Programmatic DOM [][]"),
        "programmatic DOM first-fail signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines.iter().any(|line| *line == "[fail] Compact mode"),
        "compact-mode failure signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| line.starts_with("[fail] Formatted error string ")),
        "formatted error-string failure signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Query attribute: int as double [][]"),
        "query-attribute conversion first-fail signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Query char attribute [strValue][(null)]"),
        "query-string/attribute-round-trip first-fail signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] UTF-8: Russian value. [ценность][(null)]"),
        "UTF-8 Russian attribute-value first-fail signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] UTF-8: Browsing russian element name. [][<имеет>]"),
        "UTF-8 russian element-name browsing signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] UTF-8: Open utf8testout.xml [true][false]"),
        "UTF-8 utf8testout artifact-open signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] UTF-8: Verified multi-language round trip. [true][false]"),
        "UTF-8 multi-language round-trip signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] SetText() normal use (open/close). [darkness.][(null)]"),
        "SetText open/close signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] Bool true is '1' [1][true]"),
        "bool serialization signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] GetText() normal use. [This is  text][text]"),
        "GetText normal-use signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        !fail_lines
            .iter()
            .any(|line| *line == "[fail] GetText() contained element. [false][true]"),
        "GetText contained-element signature should be resolved, got:\n{}",
        replay_stdout
    );
    assert!(
        replay_stdout.contains("Pass 417, Fail 56"),
        "current blocker signature should report failing xmltest parity count, got:\n{}",
        replay_stdout
    );
}
