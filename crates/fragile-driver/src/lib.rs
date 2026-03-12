use fragile_clang::{
    IncludeDirective, IncludeDirectiveKind, ParserBackend, ParserLanguage, TemplateParsingMode,
    TranspileOptions,
};
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const FRAGILEC_BUILD_ID_ENV: &str = "FRAGILEC_BUILD_ID";
pub const FRAGILEC_KEEP_RS_ENV: &str = "FRAGILEC_KEEP_RS";
pub const FRAGILEC_PARSER_BACKEND_ENV: &str = "FRAGILEC_PARSER_BACKEND";
pub const FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV: &str = "FRAGILEC_TRANSPILE_STAGE_TIMING_PATH";

#[derive(Debug, Clone)]
struct ParsedInvocation {
    args: Vec<OsString>,
    compile_only: bool,
    output: Option<PathBuf>,
    sources: Vec<PathBuf>,
    includes: Vec<IncludeDirective>,
    defines: Vec<String>,
}

impl ParsedInvocation {
    fn parse(args: Vec<OsString>) -> Self {
        let mut compile_only = false;
        let mut output = None;
        let mut sources = Vec::new();
        let mut includes = Vec::new();
        let mut defines = Vec::new();

        let mut i = 0usize;
        while i < args.len() {
            let current = args[i].to_string_lossy();
            let cur = current.as_ref();
            if cur == "-c" {
                compile_only = true;
                i += 1;
                continue;
            }
            if cur == "-o" {
                if i + 1 < args.len() {
                    output = Some(PathBuf::from(args[i + 1].to_string_lossy().to_string()));
                    i += 2;
                    continue;
                }
            }
            if let Some(stripped) = cur.strip_prefix("-o") {
                if !stripped.is_empty() {
                    output = Some(PathBuf::from(stripped));
                    i += 1;
                    continue;
                }
            }
            if cur == "-I" {
                if i + 1 < args.len() {
                    includes.push(IncludeDirective {
                        kind: IncludeDirectiveKind::Include,
                        path: args[i + 1].to_string_lossy().to_string(),
                    });
                    i += 2;
                    continue;
                }
            }
            if let Some(stripped) = cur.strip_prefix("-I") {
                if !stripped.is_empty() {
                    includes.push(IncludeDirective {
                        kind: IncludeDirectiveKind::Include,
                        path: stripped.to_string(),
                    });
                    i += 1;
                    continue;
                }
            }
            if cur == "-isystem" {
                if i + 1 < args.len() {
                    includes.push(IncludeDirective {
                        kind: IncludeDirectiveKind::System,
                        path: args[i + 1].to_string_lossy().to_string(),
                    });
                    i += 2;
                    continue;
                }
            }
            if let Some(stripped) = cur.strip_prefix("-isystem") {
                if !stripped.is_empty() {
                    includes.push(IncludeDirective {
                        kind: IncludeDirectiveKind::System,
                        path: stripped.to_string(),
                    });
                    i += 1;
                    continue;
                }
            }
            if cur == "-iquote" {
                if i + 1 < args.len() {
                    includes.push(IncludeDirective {
                        kind: IncludeDirectiveKind::Quote,
                        path: args[i + 1].to_string_lossy().to_string(),
                    });
                    i += 2;
                    continue;
                }
            }
            if let Some(stripped) = cur.strip_prefix("-iquote") {
                if !stripped.is_empty() {
                    includes.push(IncludeDirective {
                        kind: IncludeDirectiveKind::Quote,
                        path: stripped.to_string(),
                    });
                    i += 1;
                    continue;
                }
            }
            if cur == "-D" {
                if i + 1 < args.len() {
                    defines.push(args[i + 1].to_string_lossy().to_string());
                    i += 2;
                    continue;
                }
            }
            if let Some(stripped) = cur.strip_prefix("-D") {
                if !stripped.is_empty() {
                    defines.push(stripped.to_string());
                    i += 1;
                    continue;
                }
            }
            if !cur.starts_with('-') && is_source_file_token(cur) {
                sources.push(PathBuf::from(cur));
            }
            i += 1;
        }

        Self {
            args,
            compile_only,
            output,
            sources,
            includes,
            defines,
        }
    }
}

fn is_c_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("c"))
        .unwrap_or(false)
}

fn is_source_file_token(token: &str) -> bool {
    let path = Path::new(token);
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("c") | Some("cc") | Some("cpp") | Some("cxx") | Some("C") | Some("cp") | Some("c++")
    )
}

fn include_path_is_external_test_framework(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.contains("/googletest/")
        || normalized.ends_with("/googletest")
        || normalized.contains("/googlemock/")
        || normalized.ends_with("/googlemock")
        || normalized.contains("/gtest/")
        || normalized.ends_with("/gtest")
        || normalized.contains("/gmock/")
        || normalized.ends_with("/gmock")
}

fn maybe_promote_include_kind_to_system(
    kind: IncludeDirectiveKind,
    resolved_path: &str,
) -> IncludeDirectiveKind {
    if kind == IncludeDirectiveKind::Include
        && include_path_is_external_test_framework(resolved_path)
    {
        IncludeDirectiveKind::System
    } else {
        kind
    }
}

fn maybe_promote_frontend_include_flag(flag: &str, resolved_path: &str) -> String {
    if flag == "-I" && include_path_is_external_test_framework(resolved_path) {
        "-isystem".to_string()
    } else {
        flag.to_string()
    }
}

fn resolve_path(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn resolve_include_directives(
    include_directives: &[IncludeDirective],
    cwd: &Path,
) -> Vec<IncludeDirective> {
    include_directives
        .iter()
        .map(|directive| {
            let include_path = Path::new(directive.path.as_str());
            let resolved_path = if include_path.is_absolute() {
                include_path.to_path_buf()
            } else {
                let joined = cwd.join(include_path);
                if joined.exists() {
                    joined.canonicalize().unwrap_or(joined)
                } else {
                    joined
                }
            };
            let resolved = resolved_path.to_string_lossy().to_string();
            IncludeDirective {
                kind: maybe_promote_include_kind_to_system(directive.kind, &resolved),
                path: resolved,
            }
        })
        .collect()
}

fn resolve_frontend_path_value(raw: &str, cwd: &Path) -> String {
    let path = Path::new(raw);
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }
    let joined = cwd.join(path);
    if joined.exists() {
        joined
            .canonicalize()
            .unwrap_or(joined)
            .to_string_lossy()
            .to_string()
    } else {
        joined.to_string_lossy().to_string()
    }
}

fn resolve_forced_include_value(raw: &str, cwd: &Path) -> String {
    let path = Path::new(raw);
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }

    let joined = cwd.join(path);
    if joined.exists() {
        joined
            .canonicalize()
            .unwrap_or(joined)
            .to_string_lossy()
            .to_string()
    } else {
        // Keep unresolved include-like paths relative so Clang can still apply
        // normal header-search rules (-I/-isystem/etc.).
        raw.to_string()
    }
}

fn split_joined_path_flag<'a>(token: &'a str, flag: &str) -> Option<&'a str> {
    token.strip_prefix(flag).and_then(|rest| {
        if rest.is_empty() || rest.starts_with('-') {
            None
        } else {
            Some(rest)
        }
    })
}

fn collect_resolved_frontend_args(args: &[OsString], cwd: &Path) -> Vec<String> {
    let mut collected = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        let token = args[i].to_string_lossy();
        let cur = token.as_ref();

        if matches!(
            cur,
            "-I" | "-isystem"
                | "-iquote"
                | "-idirafter"
                | "-F"
                | "-iframework"
                | "-iframeworkwithsysroot"
                | "-iprefix"
                | "-iwithprefix"
                | "-iwithprefixbefore"
                | "-isysroot"
                | "--sysroot"
                | "-resource-dir"
                | "-include"
                | "-imacros"
                | "-include-pch"
                | "-ivfsoverlay"
        ) {
            if i + 1 < args.len() {
                let value = args[i + 1].to_string_lossy();
                if matches!(cur, "-include" | "-imacros" | "-include-pch") {
                    collected.push(cur.to_string());
                    collected.push(resolve_forced_include_value(value.as_ref(), cwd));
                } else {
                    let resolved = resolve_frontend_path_value(value.as_ref(), cwd);
                    collected.push(maybe_promote_frontend_include_flag(cur, &resolved));
                    collected.push(resolved);
                }
                i += 2;
                continue;
            }
        }

        if cur == "-D" {
            if i + 1 < args.len() {
                collected.push("-D".to_string());
                collected.push(args[i + 1].to_string_lossy().to_string());
                i += 2;
                continue;
            }
        }
        if let Some(rest) = cur.strip_prefix("-D") {
            if !rest.is_empty() {
                collected.push(cur.to_string());
                i += 1;
                continue;
            }
        }

        if cur == "-stdlib" {
            if i + 1 < args.len() {
                collected.push("-stdlib".to_string());
                collected.push(args[i + 1].to_string_lossy().to_string());
                i += 2;
                continue;
            }
        }
        if cur.starts_with("-stdlib=") {
            collected.push(cur.to_string());
            i += 1;
            continue;
        }

        if matches!(cur, "-nostdinc" | "-nostdinc++" | "-nostdlibinc") {
            collected.push(cur.to_string());
            i += 1;
            continue;
        }

        if let Some(rest) = split_joined_path_flag(cur, "-iwithprefixbefore") {
            collected.push("-iwithprefixbefore".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-iwithprefix") {
            collected.push("-iwithprefix".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-iframeworkwithsysroot") {
            collected.push("-iframeworkwithsysroot".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-iframework") {
            collected.push("-iframework".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-idirafter") {
            collected.push("-idirafter".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-ivfsoverlay") {
            collected.push("-ivfsoverlay".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-include-pch") {
            collected.push("-include-pch".to_string());
            collected.push(resolve_forced_include_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-imacros") {
            collected.push("-imacros".to_string());
            collected.push(resolve_forced_include_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-include") {
            collected.push("-include".to_string());
            collected.push(resolve_forced_include_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = cur.strip_prefix("--sysroot=") {
            if !rest.is_empty() {
                collected.push(format!(
                    "--sysroot={}",
                    resolve_frontend_path_value(rest, cwd)
                ));
                i += 1;
                continue;
            }
        }
        if let Some(rest) = split_joined_path_flag(cur, "-isysroot") {
            collected.push("-isysroot".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = cur.strip_prefix("-resource-dir=") {
            if !rest.is_empty() {
                collected.push(format!(
                    "-resource-dir={}",
                    resolve_frontend_path_value(rest, cwd)
                ));
                i += 1;
                continue;
            }
        }
        if let Some(rest) = split_joined_path_flag(cur, "-resource-dir") {
            collected.push("-resource-dir".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-iprefix") {
            collected.push("-iprefix".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-F") {
            collected.push("-F".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-isystem") {
            collected.push("-isystem".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-iquote") {
            collected.push("-iquote".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
            i += 1;
            continue;
        }
        if let Some(rest) = split_joined_path_flag(cur, "-I") {
            let resolved = resolve_frontend_path_value(rest, cwd);
            collected.push(maybe_promote_frontend_include_flag("-I", &resolved));
            collected.push(resolved);
            i += 1;
            continue;
        }

        i += 1;
    }

    collected
}

fn default_object_output(source_arg: &Path, cwd: &Path) -> Result<PathBuf, String> {
    let stem = source_arg
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            format!(
                "cannot derive object output for source {}",
                source_arg.display()
            )
        })?;
    Ok(cwd.join(format!("{stem}.o")))
}

fn source_language(source: &Path) -> ParserLanguage {
    if is_c_file(source) {
        ParserLanguage::C
    } else {
        ParserLanguage::Cpp
    }
}

fn normalize_language_standard(raw: &str, language: ParserLanguage) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let is_cpp_standard = lower.contains("++");
    let is_c_standard = !is_cpp_standard && (lower.starts_with('c') || lower.starts_with("gnu"));
    match language {
        ParserLanguage::Cpp => {
            if is_cpp_standard {
                Some(trimmed.to_string())
            } else {
                None
            }
        }
        ParserLanguage::C => {
            if is_c_standard {
                Some(trimmed.to_string())
            } else {
                None
            }
        }
    }
}

fn extract_language_standard(args: &[OsString], language: ParserLanguage) -> Option<String> {
    let mut detected: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        let token = arg.as_ref();
        if token == "-std" {
            if i + 1 < args.len() {
                let value = args[i + 1].to_string_lossy().to_string();
                if let Some(normalized) = normalize_language_standard(value.as_str(), language) {
                    detected = Some(normalized);
                }
                i += 2;
                continue;
            }
        } else if let Some(value) = token.strip_prefix("-std=") {
            if let Some(normalized) = normalize_language_standard(value, language) {
                detected = Some(normalized);
            }
        }
        i += 1;
    }
    detected
}

fn strict_parser_ignored_error_patterns(language: ParserLanguage) -> Vec<String> {
    let _ = language;
    Vec::new()
}

fn parse_parser_backend_value(backend: &str) -> Result<ParserBackend, String> {
    match backend.to_ascii_lowercase().as_str() {
        "libtooling" => Ok(ParserBackend::Libtooling),
        other => Err(format!(
            "unsupported FRAGILEC_PARSER_BACKEND value `{}`; expected: libtooling",
            other
        )),
    }
}

fn strict_parser_backend_from_value(raw: Option<&str>) -> Result<ParserBackend, String> {
    match raw.map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(backend) => parse_parser_backend_value(backend),
        None => Ok(ParserBackend::Libtooling),
    }
}

fn strict_parser_backend_from_env() -> Result<ParserBackend, String> {
    let raw = std::env::var(FRAGILEC_PARSER_BACKEND_ENV).ok();
    strict_parser_backend_from_value(raw.as_deref())
}

fn crate_name_for_source(source: &Path) -> String {
    let raw = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("fragile_unit");
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "fragile_unit".to_string()
    } else if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("fragile_{out}")
    } else {
        out
    }
}

fn hash_args(args: &[OsString]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for arg in args {
        arg.to_string_lossy().hash(&mut hasher);
    }
    hasher.finish()
}

fn build_id() -> String {
    if let Ok(id) = std::env::var(FRAGILEC_BUILD_ID_ENV) {
        if !id.trim().is_empty() {
            return id;
        }
    }
    "default".to_string()
}

fn meta_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("output");
    path.with_file_name(format!("{file_name}.fragile-meta"))
}

fn write_meta_file(source: &Path, output_obj: &Path, args: &[OsString]) -> Result<(), String> {
    if output_obj == Path::new("/dev/null") {
        return Ok(());
    }
    let meta = meta_path(output_obj);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("failed to read wall clock: {}", e))?
        .as_secs();
    let manifest = format!(
        "build_id={}\nsource={}\noutput={}\nargs_hash={:016x}\ntimestamp={}\nstrategy=fragile\n",
        build_id(),
        source.display(),
        output_obj.display(),
        hash_args(args),
        timestamp
    );
    fs::write(&meta, manifest)
        .map_err(|e| format!("failed to write metadata {}: {}", meta.display(), e))
}

fn crate_name_for_unit(source: &Path, out_obj: &Path) -> String {
    let base = crate_name_for_source(source);
    let mut hasher = DefaultHasher::new();
    source.display().to_string().hash(&mut hasher);
    out_obj.display().to_string().hash(&mut hasher);
    let suffix = hasher.finish() as u32;
    format!("{}_{suffix:08x}", base)
}

fn strict_compile_source_to_object_with_frontend_args_and_backend(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[IncludeDirective],
    defines: &[String],
    frontend_args: &[String],
    args_for_meta: &[OsString],
    parser_backend: ParserBackend,
    cwd: &Path,
) -> Result<(), String> {
    let source = resolve_path(source_arg, cwd);
    if !source.exists() {
        return Err(format!("source file does not exist: {}", source.display()));
    }

    if let Some(parent) = out_obj.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let language = source_language(&source);
    let language_standard = extract_language_standard(args_for_meta, language);
    let stage_timing_trace_path = std::env::var(FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .map(PathBuf::from);
    with_current_dir(cwd, || {
        let transpile_options = TranspileOptions {
            include_paths: Vec::new(),
            include_directives: includes.to_vec(),
            frontend_args: frontend_args.to_vec(),
            defines: defines.to_vec(),
            language,
            language_standard,
            ignored_error_patterns: strict_parser_ignored_error_patterns(language),
            backend: parser_backend,
            template_parsing_mode: TemplateParsingMode::Standard,
            // Keep system headers visible by default so libc/kernel symbols
            // (e.g. socket/epoll/netdb) retain full declaration surfaces.
            libtooling_skip_system_headers: false,
            stage_timing_trace_path,
        };
        let transpiled =
            fragile_clang::transpile_cpp_to_rust_with_options(&source, &transpile_options)
                .map_err(|e| {
                    format!(
                        "failed to transpile {} with parser backend {:?}: {}",
                        source.display(),
                        parser_backend,
                        e
                    )
                })?;
        let transpiled = normalize_transpiled_main_entry(transpiled);
        enforce_unresolved_type_invariant(&source, &transpiled)?;

        let keep_rs = std::env::var(FRAGILEC_KEEP_RS_ENV)
            .map(|v| v == "1")
            .unwrap_or(false);
        let transpiled_rs = if keep_rs {
            out_obj.with_extension("fragile.rs")
        } else {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| format!("failed to read wall clock: {}", e))?
                .as_nanos();
            std::env::temp_dir().join(format!(
                "fragilec_{}_{}_{}.rs",
                std::process::id(),
                stamp,
                crate_name_for_source(&source)
            ))
        };
        fs::write(&transpiled_rs, transpiled).map_err(|e| {
            format!(
                "failed to write transpiled source {}: {}",
                transpiled_rs.display(),
                e
            )
        })?;

        let rustc = Command::new("rustc")
            .arg("--edition")
            .arg("2021")
            .arg("-A")
            .arg("warnings")
            .arg("--crate-type")
            .arg("lib")
            .arg("--emit=obj")
            .arg("--crate-name")
            .arg(crate_name_for_unit(&source, out_obj))
            .arg(&transpiled_rs)
            .arg("-o")
            .arg(out_obj)
            .output()
            .map_err(|e| format!("failed to run rustc for {}: {}", source.display(), e))?;

        if !rustc.status.success() {
            return Err(format!(
                "fragile rustc object compile failed for {}\nstdout:\n{}\nstderr:\n{}",
                source.display(),
                String::from_utf8_lossy(&rustc.stdout),
                String::from_utf8_lossy(&rustc.stderr)
            ));
        }

        if !keep_rs {
            let _ = fs::remove_file(&transpiled_rs);
        }

        Ok(())
    })?;

    write_meta_file(&source, out_obj, args_for_meta)?;
    Ok(())
}

fn with_current_dir<T, F>(dir: &Path, f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let original =
        std::env::current_dir().map_err(|e| format!("failed to read current dir: {}", e))?;
    std::env::set_current_dir(dir)
        .map_err(|e| format!("failed to switch to {}: {}", dir.display(), e))?;
    let result = f();
    let restore = std::env::set_current_dir(&original).map_err(|e| {
        format!(
            "failed to restore current dir back to {}: {}",
            original.display(),
            e
        )
    });
    match (result, restore) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(err), Ok(())) => Err(err),
        (Ok(_), Err(restore_err)) => Err(restore_err),
        (Err(err), Err(restore_err)) => Err(format!("{err}\n{restore_err}")),
    }
}

fn normalize_transpiled_main_entry(transpiled: String) -> String {
    if !transpiled.contains("cpp_main(") {
        return transpiled;
    }
    let promoted = if transpiled.contains("pub extern \"C\" fn cpp_main(") {
        transpiled.replacen(
            "pub extern \"C\" fn cpp_main(",
            "pub extern \"C\" fn main(",
            1,
        )
    } else if transpiled.contains("pub unsafe extern \"C\" fn cpp_main(") {
        transpiled.replacen(
            "pub unsafe extern \"C\" fn cpp_main(",
            "pub unsafe extern \"C\" fn main(",
            1,
        )
    } else {
        transpiled.replacen(
            "pub fn cpp_main(",
            "#[no_mangle]\npub extern \"C\" fn main(",
            1,
        )
    };
    promoted.replace(
        "\nfn main() {\n    std::process::exit(cpp_main());\n}\n",
        "\n",
    )
}

fn has_fragile_runtime_glob_import(transpiled: &str) -> bool {
    transpiled.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.contains("::*")
            && (trimmed.starts_with("use fragile_stl::")
                || trimmed.starts_with("pub use fragile_stl::")
                || trimmed.starts_with("pub(crate) use fragile_stl::")
                || trimmed.starts_with("pub(super) use fragile_stl::")
                || trimmed.starts_with("use fragile_runtime::")
                || trimmed.starts_with("pub use fragile_runtime::")
                || trimmed.starts_with("pub(crate) use fragile_runtime::")
                || trimmed.starts_with("pub(super) use fragile_runtime::"))
    })
}

fn is_runtime_glob_import_resolved_type_name(name: &str) -> bool {
    matches!(
        name,
        "FragileCStrDisplay"
            | "error_category"
            | "fpos_mbstate_t"
            | "std_thread"
            | "string_view"
            | "wstring_view"
            | "atomic_bool"
            | "atomic_int"
            | "atomic_long"
            | "atomic_ulong"
            | "atomic_llong"
            | "atomic_ullong"
            | "atomic_long_long"
            | "atomic_unsigned_long"
            | "atomic_unsigned_long_long"
    ) || name.starts_with("__atomic_base_")
        || name.starts_with("__pthread_")
        || name.starts_with("reverse_iterator_")
        || (name.contains("iterator") && name.ends_with("_value_type"))
}

fn enforce_unresolved_type_invariant(source: &Path, transpiled: &str) -> Result<(), String> {
    let mut unresolved = fragile_clang::AstCodeGen::unresolved_named_type_references(transpiled);
    if has_fragile_runtime_glob_import(transpiled) {
        unresolved.retain(|name| !is_runtime_glob_import_resolved_type_name(name));
    }
    if unresolved.is_empty() {
        return Ok(());
    }

    let preview: Vec<String> = unresolved.iter().take(8).cloned().collect();
    let mut detail = preview.join(", ");
    if unresolved.len() > preview.len() {
        detail.push_str(&format!(" (and {} more)", unresolved.len() - preview.len()));
    }
    Err(format!(
        "fragile unresolved-type invariant failed for {}: {}",
        source.display(),
        detail
    ))
}

fn run_fragile_compile_in_dir(parsed: &ParsedInvocation, cwd: &Path) -> Result<(), String> {
    if !parsed.compile_only {
        return Err("fragile compile mode only supports `-c` invocations".to_string());
    }
    if parsed.sources.is_empty() {
        return Err("fragile compile mode requires at least one source input".to_string());
    }
    if parsed.sources.len() > 1 && parsed.output.is_some() {
        return Err(
            "fragile strict compile does not support `-c` with multiple sources and a single `-o` output"
                .to_string(),
        );
    }

    let resolved_includes = resolve_include_directives(&parsed.includes, cwd);
    let resolved_frontend_args = collect_resolved_frontend_args(&parsed.args, cwd);
    let parser_backend = strict_parser_backend_from_env()?;
    for source_arg in &parsed.sources {
        let out_obj = if parsed.sources.len() == 1 {
            match &parsed.output {
                Some(out) => resolve_path(out, cwd),
                None => default_object_output(source_arg, cwd)?,
            }
        } else {
            default_object_output(source_arg, cwd)?
        };

        strict_compile_source_to_object_with_frontend_args_and_backend(
            source_arg,
            &out_obj,
            &resolved_includes,
            &parsed.defines,
            &resolved_frontend_args,
            &parsed.args,
            parser_backend,
            cwd,
        )?;
    }

    Ok(())
}

pub fn compile_invocation_in_dir(args: &[OsString], cwd: &Path) -> Result<(), String> {
    let parsed = ParsedInvocation::parse(args.to_vec());
    run_fragile_compile_in_dir(&parsed, cwd)
}

pub fn compile_invocation(args: &[OsString]) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    compile_invocation_in_dir(args, &cwd)
}

pub fn compile_unit_with_flags_in_dir(
    source: &Path,
    output_obj: &Path,
    flags: &[&str],
    cwd: &Path,
) -> Result<(), String> {
    let mut args: Vec<OsString> = flags.iter().map(|flag| OsString::from(*flag)).collect();
    args.push(OsString::from("-c"));
    args.push(OsString::from(source.to_string_lossy().to_string()));
    args.push(OsString::from("-o"));
    args.push(OsString::from(output_obj.to_string_lossy().to_string()));
    compile_invocation_in_dir(&args, cwd)
}

pub fn compile_unit_with_flags(
    source: &Path,
    output_obj: &Path,
    flags: &[&str],
) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    compile_unit_with_flags_in_dir(source, output_obj, flags, &cwd)
}

pub fn compile_unit_with_fragilec_in_dir(
    source: &Path,
    output_obj: &Path,
    flags: &[&str],
    cwd: &Path,
) -> Result<(), String> {
    let mut last_spawn_err: Option<String> = None;
    for fragilec in fragilec_candidates() {
        let mut cmd = Command::new(&fragilec);
        cmd.current_dir(cwd);
        if std::env::var_os("FRAGILEC_MODE").is_none() {
            cmd.env("FRAGILEC_MODE", "strict");
        }
        cmd.args(flags);
        cmd.arg("-c");
        cmd.arg(source);
        cmd.arg("-o");
        cmd.arg(output_obj);
        let output = match cmd.output() {
            Ok(output) => output,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                last_spawn_err = Some(format!(
                    "candidate `{}` not found ({})",
                    fragilec.display(),
                    err
                ));
                continue;
            }
            Err(err) => {
                return Err(format!("failed to spawn `{}`: {}", fragilec.display(), err));
            }
        };
        if !output.status.success() {
            return Err(format!(
                "`{}` compile failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                fragilec.display(),
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        return Ok(());
    }
    Err(format!(
        "unable to locate `fragilec`; set FRAGILEC_BIN or ensure fragilec is on PATH{}",
        last_spawn_err
            .map(|e| format!(" ({e})"))
            .unwrap_or_default()
    ))
}

pub fn compile_unit_with_fragilec(
    source: &Path,
    output_obj: &Path,
    flags: &[&str],
) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    compile_unit_with_fragilec_in_dir(source, output_obj, flags, &cwd)
}

fn fragilec_candidates() -> Vec<PathBuf> {
    if let Ok(explicit) = std::env::var("FRAGILEC_BIN") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return vec![PathBuf::from(trimmed)];
        }
    }

    let mut candidates = vec![PathBuf::from("fragilec")];
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(repo_root) = manifest_dir.parent().and_then(|p| p.parent()) {
        candidates.push(repo_root.join("target/release/fragilec"));
        candidates.push(repo_root.join("target/debug/fragilec"));
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_collects_sources_and_compile_output() {
        let parsed = ParsedInvocation::parse(vec![
            OsString::from("-I"),
            OsString::from("inc"),
            OsString::from("-c"),
            OsString::from("test.cc"),
            OsString::from("-o"),
            OsString::from("test.o"),
        ]);

        assert!(parsed.compile_only);
        assert_eq!(parsed.sources, vec![PathBuf::from("test.cc")]);
        assert_eq!(parsed.output, Some(PathBuf::from("test.o")));
        assert_eq!(parsed.includes.len(), 1);
    }

    #[test]
    fn compile_unit_helper_adds_compile_args() {
        let cwd = Path::new("/");
        let source = Path::new("src/main.cc");
        let out = Path::new("out/main.o");
        let mut args: Vec<OsString> = vec![OsString::from("-Iinclude")];
        args.push(OsString::from("-c"));
        args.push(OsString::from(source.to_string_lossy().to_string()));
        args.push(OsString::from("-o"));
        args.push(OsString::from(out.to_string_lossy().to_string()));

        let parsed = ParsedInvocation::parse(args);
        assert!(parsed.compile_only);
        assert_eq!(parsed.sources, vec![PathBuf::from("src/main.cc")]);
        assert_eq!(parsed.output, Some(PathBuf::from("out/main.o")));
        assert_eq!(
            resolve_path(&parsed.sources[0], cwd),
            PathBuf::from("/src/main.cc")
        );
    }
}
