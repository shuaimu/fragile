use fragile_clang::{
    IncludeDirective, IncludeDirectiveKind, ParserBackend, ParserLanguage, TemplateParsingMode,
    TranspileOptions,
};
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

const FRAGILEC_LOG_ENV: &str = "FRAGILEC_LOG";
const FRAGILEC_MODE_ENV: &str = "FRAGILEC_MODE";
const FRAGILEC_BUILD_ID_ENV: &str = "FRAGILEC_BUILD_ID";
const FRAGILEC_ENFORCE_BUILD_ID_ENV: &str = "FRAGILEC_ENFORCE_BUILD_ID";
const FRAGILEC_REQUIRE_META_ENV: &str = "FRAGILEC_REQUIRE_META";
const FRAGILEC_KEEP_RS_ENV: &str = "FRAGILEC_KEEP_RS";
const FRAGILEC_LINKER_ENV: &str = "FRAGILEC_LINKER";
const FRAGILEC_PARSER_BACKEND_ENV: &str = "FRAGILEC_PARSER_BACKEND";
const FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV: &str = "FRAGILEC_TRANSPILE_STAGE_TIMING_PATH";

fn validate_strict_mode_value(mode: &str) -> Result<(), String> {
    match mode.to_ascii_lowercase().as_str() {
        "strict" => Ok(()),
        "auto" => {
            Err("FRAGILEC_MODE=auto has been removed; fragilec is strict-only now".to_string())
        }
        "pass" => {
            Err("FRAGILEC_MODE=pass has been removed; fragilec is strict-only now".to_string())
        }
        other => Err(format!(
            "unsupported FRAGILEC_MODE value `{}`; fragilec is strict-only",
            other
        )),
    }
}

fn validate_strict_mode_env() -> Result<(), String> {
    let mode = std::env::var(FRAGILEC_MODE_ENV).unwrap_or_else(|_| "strict".to_string());
    validate_strict_mode_value(mode.as_str())
}

#[derive(Debug, Clone)]
struct ParsedInvocation {
    args: Vec<OsString>,
    compile_only: bool,
    output: Option<PathBuf>,
    sources: Vec<PathBuf>,
    source_indices: Vec<usize>,
    includes: Vec<IncludeDirective>,
    defines: Vec<String>,
}

impl ParsedInvocation {
    fn parse(args: Vec<OsString>) -> Self {
        let mut compile_only = false;
        let mut output = None;
        let mut sources = Vec::new();
        let mut source_indices = Vec::new();
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
                source_indices.push(i);
            }
            i += 1;
        }

        Self {
            args,
            compile_only,
            output,
            sources,
            source_indices,
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

fn append_invocation_log(args: &[OsString]) -> Result<(), String> {
    let log_path = match std::env::var(FRAGILEC_LOG_ENV) {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => return Ok(()),
    };
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create fragilec log parent dir {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    let cwd = std::env::current_dir()
        .map_err(|e| format!("failed to resolve current dir for fragilec logging: {}", e))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("failed to open fragilec log {}: {}", log_path.display(), e))?;
    writeln!(file, "cwd={}", cwd.display()).map_err(|e| {
        format!(
            "failed to append cwd record to {}: {}",
            log_path.display(),
            e
        )
    })?;
    write!(file, "args=").map_err(|e| {
        format!(
            "failed to append args prefix to {}: {}",
            log_path.display(),
            e
        )
    })?;
    for arg in args {
        write!(file, "{} ", arg.to_string_lossy())
            .map_err(|e| format!("failed to append args to {}: {}", log_path.display(), e))?;
    }
    writeln!(file).map_err(|e| format!("failed to append newline to {}: {}", log_path.display(), e))
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
            IncludeDirective {
                kind: directive.kind,
                path: resolved_path.to_string_lossy().to_string(),
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
        // normal header-search rules (-I/-isystem/etc.) instead of forcing an
        // absolute path anchored at the current working directory.
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
                collected.push(cur.to_string());
                let value = args[i + 1].to_string_lossy();
                if matches!(cur, "-include" | "-imacros" | "-include-pch") {
                    collected.push(resolve_forced_include_value(value.as_ref(), cwd));
                } else {
                    collected.push(resolve_frontend_path_value(value.as_ref(), cwd));
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
            collected.push("-I".to_string());
            collected.push(resolve_frontend_path_value(rest, cwd));
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

fn is_cmake_cxx_probe_source_name(name: &str) -> bool {
    matches!(
        name,
        "CMakeCXXCompilerId.cpp" | "CMakeCXXCompilerABI.cpp" | "testCXXCompiler.cxx"
    )
}

fn is_cmake_probe_working_dir(cwd: &Path) -> bool {
    let cwd = cwd.to_string_lossy();
    (cwd.contains("/CMakeFiles/") && cwd.contains("/CompilerIdCXX"))
        || cwd.contains("/CMakeFiles/CMakeScratch/TryCompile-")
}

fn should_passthrough_cmake_compiler_probe(
    parsed: &ParsedInvocation,
    args: &[OsString],
    cwd: &Path,
) -> bool {
    if is_cmake_probe_working_dir(cwd) {
        return true;
    }

    for source in &parsed.sources {
        if let Some(name) = source.file_name().and_then(|s| s.to_str()) {
            if is_cmake_cxx_probe_source_name(name) {
                return true;
            }
        }
    }

    for arg in args {
        let token = arg.to_string_lossy();
        if token.starts_with('-') {
            continue;
        }
        if let Some(name) = Path::new(token.as_ref())
            .file_name()
            .and_then(|s| s.to_str())
        {
            if is_cmake_cxx_probe_source_name(name) {
                return true;
            }
        }
    }

    false
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

fn read_build_id_from_meta(meta_path: &Path) -> Result<Option<String>, String> {
    let content = match fs::read_to_string(meta_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "failed to read metadata file {}: {}",
                meta_path.display(),
                e
            ));
        }
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("build_id=") {
            return Ok(Some(rest.trim().to_string()));
        }
    }
    Ok(None)
}

fn enforce_build_id_for_link_inputs(args: &[OsString]) -> Result<(), String> {
    if std::env::var(FRAGILEC_ENFORCE_BUILD_ID_ENV)
        .map(|v| v == "1")
        .unwrap_or(false)
        == false
    {
        return Ok(());
    }

    let required_build_id = build_id();
    let require_meta = std::env::var(FRAGILEC_REQUIRE_META_ENV)
        .map(|v| v == "1")
        .unwrap_or(false);

    for arg in args {
        let token = arg.to_string_lossy();
        if token.starts_with('-') {
            continue;
        }
        let input = Path::new(token.as_ref());
        let is_obj_or_archive = matches!(
            input.extension().and_then(|e| e.to_str()),
            Some("o") | Some("a")
        );
        if !is_obj_or_archive || !input.exists() {
            continue;
        }

        let meta = meta_path(input);
        match read_build_id_from_meta(&meta)? {
            Some(found) => {
                if found != required_build_id {
                    return Err(format!(
                        "build-id mismatch for {}: expected {} but found {} in {}",
                        input.display(),
                        required_build_id,
                        found,
                        meta.display()
                    ));
                }
            }
            None => {
                if require_meta {
                    return Err(format!(
                        "missing fragile metadata for link input {} (expected {})",
                        input.display(),
                        meta.display()
                    ));
                }
            }
        }
    }

    Ok(())
}

fn crate_name_for_unit(source: &Path, out_obj: &Path) -> String {
    let base = crate_name_for_source(source);
    let mut hasher = DefaultHasher::new();
    source.display().to_string().hash(&mut hasher);
    out_obj.display().to_string().hash(&mut hasher);
    let suffix = hasher.finish() as u32;
    format!("{}_{}", base, format!("{suffix:08x}"))
}

#[allow(dead_code)]
fn strict_compile_source_to_object(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[IncludeDirective],
    defines: &[String],
    args_for_meta: &[OsString],
) -> Result<(), String> {
    let parser_backend = strict_parser_backend_from_env()?;
    strict_compile_source_to_object_with_frontend_args_and_backend(
        source_arg,
        out_obj,
        includes,
        defines,
        &[],
        args_for_meta,
        parser_backend,
    )
}

#[allow(dead_code)]
fn strict_compile_source_to_object_with_backend(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[IncludeDirective],
    defines: &[String],
    args_for_meta: &[OsString],
    parser_backend: ParserBackend,
) -> Result<(), String> {
    strict_compile_source_to_object_with_frontend_args_and_backend(
        source_arg,
        out_obj,
        includes,
        defines,
        &[],
        args_for_meta,
        parser_backend,
    )
}

fn strict_compile_source_to_object_with_frontend_args_and_backend(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[IncludeDirective],
    defines: &[String],
    frontend_args: &[String],
    args_for_meta: &[OsString],
    parser_backend: ParserBackend,
) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let source = resolve_path(source_arg, &cwd);
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
    let transpile_options = TranspileOptions {
        include_paths: Vec::new(),
        include_directives: includes.to_vec(),
        frontend_args: frontend_args.to_vec(),
        defines: defines.to_vec(),
        language,
        language_standard,
        ignored_error_patterns: strict_parser_ignored_error_patterns(language),
        backend: parser_backend,
        template_parsing_mode: TemplateParsingMode::Auto,
        libtooling_skip_system_headers: true,
        stage_timing_trace_path,
    };
    let transpiled = fragile_clang::transpile_cpp_to_rust_with_options(&source, &transpile_options)
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

    write_meta_file(&source, out_obj, args_for_meta)?;
    Ok(())
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

fn enforce_unresolved_type_invariant(source: &Path, transpiled: &str) -> Result<(), String> {
    let unresolved = fragile_clang::AstCodeGen::unresolved_named_type_references(transpiled);
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

fn run_fragile_compile(parsed: &ParsedInvocation) -> Result<(), String> {
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

    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let resolved_includes = resolve_include_directives(&parsed.includes, &cwd);
    let resolved_frontend_args = collect_resolved_frontend_args(&parsed.args, &cwd);
    let parser_backend = strict_parser_backend_from_env()?;
    for source_arg in &parsed.sources {
        let out_obj = if parsed.sources.len() == 1 {
            match &parsed.output {
                Some(out) => resolve_path(out, &cwd),
                None => default_object_output(source_arg, &cwd)?,
            }
        } else {
            default_object_output(source_arg, &cwd)?
        };

        strict_compile_source_to_object_with_frontend_args_and_backend(
            source_arg,
            &out_obj,
            &resolved_includes,
            &parsed.defines,
            &resolved_frontend_args,
            &parsed.args,
            parser_backend,
        )?;
    }

    Ok(())
}

fn link_driver() -> String {
    match std::env::var(FRAGILEC_LINKER_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => "c++".to_string(),
    }
}

fn build_rust_runtime_link_support(temp_root: &Path) -> Result<(PathBuf, Vec<OsString>), String> {
    let runtime_rs = temp_root.join("fragile_runtime_support.rs");
    let runtime_archive = temp_root.join("libfragile_runtime_support.a");
    fs::write(
        &runtime_rs,
        "#[no_mangle]\npub extern \"C\" fn __fragile_runtime_support_anchor() {}\n",
    )
    .map_err(|e| {
        format!(
            "failed to write runtime support source {}: {}",
            runtime_rs.display(),
            e
        )
    })?;

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--print")
        .arg("native-static-libs")
        .arg(&runtime_rs)
        .arg("-o")
        .arg(&runtime_archive)
        .output()
        .map_err(|e| format!("failed to build rust runtime support archive: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "rust runtime support build failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut native_libs: Vec<OsString> = Vec::new();
    for stream in [&output.stdout, &output.stderr] {
        let text = String::from_utf8_lossy(stream);
        for line in text.lines() {
            if let Some(rest) = line.split("native-static-libs:").nth(1) {
                for token in rest.split_whitespace() {
                    native_libs.push(OsString::from(token));
                }
            }
        }
    }

    Ok((runtime_archive, native_libs))
}

fn object_defines_main_symbol(obj: &Path) -> Result<bool, String> {
    let output = Command::new("nm")
        .arg("-g")
        .arg(obj)
        .output()
        .map_err(|e| format!("failed to inspect symbols for {}: {}", obj.display(), e))?;
    if !output.status.success() {
        return Err(format!(
            "nm failed for {}\nstdout:\n{}\nstderr:\n{}",
            obj.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let symbol = tokens[tokens.len() - 1];
        if symbol != "main" {
            continue;
        }
        let kind = if tokens.len() >= 2 {
            tokens[tokens.len() - 2]
        } else {
            ""
        };
        // `U main` is undefined/imported, not a definition.
        if kind == "U" {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

fn push_unique_object_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if paths.iter().all(|existing| existing != &path) {
        paths.push(path);
    }
}

fn collect_link_input_objects_for_main_scan(
    parsed: &ParsedInvocation,
    compiled_positions: &[(usize, PathBuf)],
    cwd: &Path,
) -> Vec<PathBuf> {
    let mut objects = Vec::new();
    for (_, out_obj) in compiled_positions {
        push_unique_object_path(&mut objects, out_obj.clone());
    }
    for arg in &parsed.args {
        let arg_str = arg.to_string_lossy();
        if !arg_str.ends_with(".o") {
            continue;
        }
        let obj = resolve_path(Path::new(arg_str.as_ref()), cwd);
        if !obj.exists() {
            continue;
        }
        push_unique_object_path(&mut objects, obj);
    }
    objects
}

fn scan_main_defining_objects(objects: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut defining = Vec::new();
    for obj in objects {
        if object_defines_main_symbol(obj)? {
            defining.push(obj.clone());
        }
    }
    Ok(defining)
}

fn format_main_symbol_diagnostic(
    inspected_objects: &[PathBuf],
    defining_objects: &[PathBuf],
) -> String {
    fn format_paths(paths: &[PathBuf]) -> String {
        if paths.is_empty() {
            return "<none>".to_string();
        }
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    format!(
        "main symbol diagnostic:\n  defining objects: {}\n  inspected objects: {}",
        format_paths(defining_objects),
        format_paths(inspected_objects)
    )
}

fn output_is_non_executable_artifact(output: Option<&Path>) -> bool {
    let Some(output) = output else {
        return false;
    };
    let normalized = output.to_string_lossy().to_ascii_lowercase();
    normalized.ends_with(".a")
        || normalized.ends_with(".o")
        || normalized.ends_with(".obj")
        || normalized.ends_with(".lo")
        || normalized.ends_with(".so")
        || normalized.contains(".so.")
        || normalized.ends_with(".dylib")
        || normalized.ends_with(".dll")
}

fn linker_flag_disables_main_requirement(flag: &str) -> bool {
    matches!(
        flag,
        "-shared"
            | "-dynamiclib"
            | "-r"
            | "--relocatable"
            | "-nostdlib"
            | "-nostartfiles"
            | "-e"
            | "--entry"
    )
}

fn linker_tokens_disable_main_requirement(tokens: &[&str]) -> bool {
    tokens.iter().any(|token| {
        matches!(
            *token,
            "-shared"
                | "--shared"
                | "-dylib"
                | "--dylib"
                | "-r"
                | "--relocatable"
                | "-e"
                | "--entry"
        ) || token.starts_with("--entry=")
    })
}

fn link_requires_program_main(parsed: &ParsedInvocation) -> bool {
    if output_is_non_executable_artifact(parsed.output.as_deref()) {
        return false;
    }

    let mut skip_next = false;
    for arg in &parsed.args {
        let arg = arg.to_string_lossy();
        let arg = arg.as_ref();

        if skip_next {
            skip_next = false;
            if linker_flag_disables_main_requirement(arg) {
                return false;
            }
        }
        if arg == "-Xlinker" {
            skip_next = true;
            continue;
        }
        if linker_flag_disables_main_requirement(arg) {
            return false;
        }
        if let Some(rest) = arg.strip_prefix("-Wl,") {
            let tokens: Vec<&str> = rest.split(',').collect();
            if linker_tokens_disable_main_requirement(&tokens) {
                return false;
            }
        }
    }

    true
}

fn linker_output_reports_missing_main(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("undefined reference to `main'")
        || lower.contains("undefined reference to 'main'")
        || lower.contains("undefined symbol: main")
        || lower.contains("undefined symbol: _main")
        || lower.contains("unresolved external symbol main")
}

fn run_fragile_link(parsed: &ParsedInvocation) -> Result<(), String> {
    if parsed.compile_only {
        return Err("internal error: run_fragile_link called for compile-only command".to_string());
    }

    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let keep_rs = std::env::var(FRAGILEC_KEEP_RS_ENV)
        .map(|v| v == "1")
        .unwrap_or(false);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("failed to read wall clock: {}", e))?
        .as_nanos();
    let temp_root =
        std::env::temp_dir().join(format!("fragilec_link_{}_{}", std::process::id(), stamp));
    fs::create_dir_all(&temp_root).map_err(|e| {
        format!(
            "failed to create strict-link temp dir {}: {}",
            temp_root.display(),
            e
        )
    })?;

    let mut compiled_positions: Vec<(usize, PathBuf)> = Vec::with_capacity(parsed.sources.len());
    let resolved_includes = resolve_include_directives(&parsed.includes, &cwd);
    let resolved_frontend_args = collect_resolved_frontend_args(&parsed.args, &cwd);
    let parser_backend = strict_parser_backend_from_env()?;
    for (idx, source_arg) in parsed.sources.iter().enumerate() {
        let stem = source_arg
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unit");
        let out_obj = temp_root.join(format!("{idx}_{stem}.o"));
        strict_compile_source_to_object_with_frontend_args_and_backend(
            source_arg,
            &out_obj,
            &resolved_includes,
            &parsed.defines,
            &resolved_frontend_args,
            &parsed.args,
            parser_backend,
        )?;
        let source_pos = parsed
            .source_indices
            .get(idx)
            .copied()
            .ok_or_else(|| "internal parse error: missing source index".to_string())?;
        compiled_positions.push((source_pos, out_obj));
    }

    let mut link_args = parsed.args.clone();
    for (source_pos, out_obj) in &compiled_positions {
        if *source_pos >= link_args.len() {
            return Err("internal parse error: source index out of bounds".to_string());
        }
        let replaced = out_obj.strip_prefix(&cwd).unwrap_or(out_obj.as_path());
        link_args[*source_pos] = OsString::from(replaced.to_string_lossy().to_string());
    }

    let inspected_objects =
        collect_link_input_objects_for_main_scan(parsed, &compiled_positions, &cwd);
    let defining_objects = scan_main_defining_objects(&inspected_objects)?;
    let main_symbol_diag = format_main_symbol_diagnostic(&inspected_objects, &defining_objects);
    let requires_main = link_requires_program_main(parsed);
    let has_main_in_objects = !defining_objects.is_empty();

    let (runtime_archive, native_libs) = build_rust_runtime_link_support(&temp_root)?;
    link_args.push(OsString::from(
        runtime_archive.to_string_lossy().to_string(),
    ));
    link_args.extend(native_libs);

    let driver = link_driver();
    let link_output = Command::new(&driver)
        .args(&link_args)
        .output()
        .map_err(|e| format!("failed to run strict link driver `{}`: {}", driver, e))?;
    if !link_output.status.success() {
        let stderr_text = String::from_utf8_lossy(&link_output.stderr);
        if requires_main && !has_main_in_objects && linker_output_reports_missing_main(&stderr_text)
        {
            return Err(format!(
                "strict link requires a real `main` symbol for executable outputs\n{}",
                main_symbol_diag
            ));
        }
        return Err(format!(
            "strict link failed via `{}`\nstdout:\n{}\nstderr:\n{}\n{}",
            driver,
            String::from_utf8_lossy(&link_output.stdout),
            stderr_text,
            main_symbol_diag
        ));
    }

    if !keep_rs {
        let _ = fs::remove_dir_all(&temp_root);
    }

    Ok(())
}

fn print_help() {
    eprintln!(
        "\
fragilec - Fragile compiler driver shim

Usage:
  fragilec [compiler args...]

Environment:
  FRAGILEC_MODE=strict               Optional; strict-only mode (default: strict)
  FRAGILEC_PARSER_BACKEND=<name>     Parser backend: libtooling
  FRAGILEC_LOG=<path>                Append invocation log (cwd/args records)
  FRAGILEC_BUILD_ID=<id>             Build-id used for metadata writes/checks
  FRAGILEC_ENFORCE_BUILD_ID=1        Enforce build-id on .o/.a inputs during link
  FRAGILEC_REQUIRE_META=1            Require metadata sidecars for link inputs
  FRAGILEC_KEEP_RS=1                 Keep transpiled Rust sidecar next to output object
  FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=<path>
                                     Write transpile stage timing trace (parse/export/enrichment/codegen)
  FRAGILEC_LINKER=<path>             Link-driver executable for strict link (default: c++)
"
    );
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.iter().any(|a| a == "--fragilec-help") || (args.len() == 1 && args[0] == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    if let Err(err) = append_invocation_log(&args) {
        eprintln!("[fragilec] warning: {}", err);
    }

    let parsed = ParsedInvocation::parse(args.clone());
    if let Err(err) = validate_strict_mode_env() {
        eprintln!("[fragilec] {}", err);
        return ExitCode::from(2);
    }

    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("[fragilec] failed to read cwd: {}", err);
            return ExitCode::from(1);
        }
    };
    if should_passthrough_cmake_compiler_probe(&parsed, &args, &cwd) {
        let status = Command::new("c++").args(&args).status();
        return match status {
            Ok(status) if status.success() => ExitCode::SUCCESS,
            Ok(_) => ExitCode::from(1),
            Err(err) => {
                eprintln!(
                    "[fragilec] failed to run cmake compiler-probe passthrough via `c++`: {}",
                    err
                );
                ExitCode::from(1)
            }
        };
    }

    // Enforce link-input metadata only when we are delegating a link command.
    if !parsed.compile_only {
        if let Err(err) = enforce_build_id_for_link_inputs(&parsed.args) {
            eprintln!("[fragilec] {}", err);
            return ExitCode::from(1);
        }
    }

    let run_result = if parsed.compile_only {
        run_fragile_compile(&parsed)
    } else {
        run_fragile_link(&parsed)
    };
    match run_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[fragilec] {}", err);
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(|s| OsString::from(*s)).collect()
    }

    #[test]
    fn parse_compile_invocation_collects_sources_flags_and_output() {
        let parsed = ParsedInvocation::parse(args(&[
            "-I",
            "include",
            "-D",
            "FOO=1",
            "-c",
            "src/main.cpp",
            "-o",
            "main.o",
        ]));
        assert!(parsed.compile_only);
        assert_eq!(parsed.sources, vec![PathBuf::from("src/main.cpp")]);
        assert_eq!(parsed.source_indices, vec![5usize]);
        assert_eq!(
            parsed.includes,
            vec![IncludeDirective {
                kind: IncludeDirectiveKind::Include,
                path: "include".to_string(),
            }]
        );
        assert_eq!(parsed.defines, vec!["FOO=1".to_string()]);
        assert_eq!(parsed.output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn parse_handles_combined_flag_forms() {
        let parsed = ParsedInvocation::parse(args(&[
            "-Iinc",
            "-isystemsys",
            "-iquotequote",
            "-DBAR=1",
            "-c",
            "unit.c",
            "-omain.o",
        ]));
        assert_eq!(
            parsed.includes,
            vec![
                IncludeDirective {
                    kind: IncludeDirectiveKind::Include,
                    path: "inc".to_string(),
                },
                IncludeDirective {
                    kind: IncludeDirectiveKind::System,
                    path: "sys".to_string(),
                },
                IncludeDirective {
                    kind: IncludeDirectiveKind::Quote,
                    path: "quote".to_string(),
                },
            ]
        );
        assert_eq!(parsed.defines, vec!["BAR=1".to_string()]);
        assert_eq!(parsed.output, Some(PathBuf::from("main.o")));
        assert_eq!(parsed.source_indices, vec![5usize]);
    }

    #[test]
    fn parse_preserves_include_directive_order_and_kinds() {
        let parsed = ParsedInvocation::parse(args(&[
            "-I", "inc1", "-isystem", "sys1", "-iquote", "quote1", "-Iinc2", "unit.cpp",
        ]));
        assert_eq!(
            parsed.includes,
            vec![
                IncludeDirective {
                    kind: IncludeDirectiveKind::Include,
                    path: "inc1".to_string(),
                },
                IncludeDirective {
                    kind: IncludeDirectiveKind::System,
                    path: "sys1".to_string(),
                },
                IncludeDirective {
                    kind: IncludeDirectiveKind::Quote,
                    path: "quote1".to_string(),
                },
                IncludeDirective {
                    kind: IncludeDirectiveKind::Include,
                    path: "inc2".to_string(),
                },
            ]
        );
    }

    #[test]
    fn resolve_include_directives_interprets_relative_paths_from_invocation_cwd() {
        let cwd = Path::new("/tmp/fragilec_include_resolution_base");
        let resolved = resolve_include_directives(
            &[IncludeDirective {
                kind: IncludeDirectiveKind::Include,
                path: "../relative/include".to_string(),
            }],
            cwd,
        );
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].kind, IncludeDirectiveKind::Include);
        assert_eq!(
            resolved[0].path,
            cwd.join("../relative/include")
                .to_string_lossy()
                .to_string()
        );
    }

    #[test]
    fn collect_resolved_frontend_args_preserves_order_and_resolves_supported_flags() {
        let cwd = Path::new("/tmp/fragilec_frontend_arg_resolution_base");
        let collected = collect_resolved_frontend_args(
            &args(&[
                "-Iinc",
                "-isystem",
                "sys",
                "-iquotequote",
                "-idirafter",
                "after",
                "-Ffw",
                "-iframework",
                "ifw",
                "--sysroot=rootfs",
                "-isysroot",
                "sdk",
                "-resource-dir",
                "resource",
                "-nostdinc",
                "-nostdinc++",
                "-stdlib=libstdc++",
                "-include",
                "force/config.hpp",
                "-D",
                "FOO=1",
                "-DBAR=2",
                "unit.cpp",
            ]),
            cwd,
        );

        assert_eq!(
            collected,
            vec![
                "-I".to_string(),
                cwd.join("inc").to_string_lossy().to_string(),
                "-isystem".to_string(),
                cwd.join("sys").to_string_lossy().to_string(),
                "-iquote".to_string(),
                cwd.join("quote").to_string_lossy().to_string(),
                "-idirafter".to_string(),
                cwd.join("after").to_string_lossy().to_string(),
                "-F".to_string(),
                cwd.join("fw").to_string_lossy().to_string(),
                "-iframework".to_string(),
                cwd.join("ifw").to_string_lossy().to_string(),
                format!("--sysroot={}", cwd.join("rootfs").to_string_lossy()),
                "-isysroot".to_string(),
                cwd.join("sdk").to_string_lossy().to_string(),
                "-resource-dir".to_string(),
                cwd.join("resource").to_string_lossy().to_string(),
                "-nostdinc".to_string(),
                "-nostdinc++".to_string(),
                "-stdlib=libstdc++".to_string(),
                "-include".to_string(),
                "force/config.hpp".to_string(),
                "-D".to_string(),
                "FOO=1".to_string(),
                "-DBAR=2".to_string(),
            ]
        );
    }

    #[test]
    fn collect_resolved_frontend_args_ignores_unrelated_flags() {
        let cwd = Path::new("/tmp/fragilec_frontend_arg_ignore_base");
        let collected = collect_resolved_frontend_args(
            &args(&[
                "-Winvalid-pch",
                "-Winvalid-offsetof",
                "-Wl,-rpath,/tmp/lib",
                "unit.cpp",
            ]),
            cwd,
        );
        assert!(
            collected.is_empty(),
            "unexpected passthrough flags collected: {:?}",
            collected
        );
    }

    #[test]
    fn parse_tracks_multiple_source_positions() {
        let parsed = ParsedInvocation::parse(args(&["-O2", "a.cpp", "-DMODE=1", "b.cc"]));
        assert_eq!(
            parsed.sources,
            vec![PathBuf::from("a.cpp"), PathBuf::from("b.cc")]
        );
        assert_eq!(parsed.source_indices, vec![1usize, 3usize]);
    }

    #[test]
    fn default_output_uses_cwd_and_source_stem() {
        let cwd = Path::new("/tmp/work");
        let out = default_object_output(Path::new("src/foo.c"), cwd).expect("default output");
        assert_eq!(out, PathBuf::from("/tmp/work/foo.o"));
    }

    #[test]
    fn source_kind_detection_works() {
        assert!(is_source_file_token("x.c"));
        assert!(is_source_file_token("x.cpp"));
        assert!(is_source_file_token("x.cxx"));
        assert!(!is_source_file_token("x.o"));
        assert!(!is_source_file_token("-Wl,--as-needed"));
    }

    #[test]
    fn strict_parser_ignored_patterns_are_empty_by_default() {
        let cpp = strict_parser_ignored_error_patterns(ParserLanguage::Cpp);
        assert!(
            cpp.is_empty(),
            "cpp strict parser ignore list should be empty"
        );

        let c = strict_parser_ignored_error_patterns(ParserLanguage::C);
        assert!(c.is_empty(), "c strict parser ignore list should be empty");
    }

    #[test]
    fn strict_parser_backend_validation_accepts_supported_values() {
        assert_eq!(
            parse_parser_backend_value("LIBTOOLING").expect("libtooling backend should parse"),
            ParserBackend::Libtooling
        );
        assert_eq!(
            strict_parser_backend_from_value(None).expect("missing backend should default"),
            ParserBackend::Libtooling
        );
        assert_eq!(
            strict_parser_backend_from_value(Some("")).expect("empty backend should default"),
            ParserBackend::Libtooling
        );
        strict_parser_backend_from_value(Some(" libclang "))
            .expect_err("legacy libclang backend value should be rejected");
        strict_parser_backend_from_value(Some(" hybrid "))
            .expect_err("legacy hybrid backend value should be rejected");
    }

    #[test]
    fn strict_parser_backend_validation_rejects_unsupported_values() {
        let err = parse_parser_backend_value("unsupported")
            .expect_err("unsupported backend value must be rejected");
        assert!(
            err.contains("unsupported FRAGILEC_PARSER_BACKEND value"),
            "unexpected error: {}",
            err
        );
        assert!(
            err.contains("expected: libtooling"),
            "error should list the only supported backend value, got: {}",
            err
        );
    }

    #[test]
    fn extract_language_standard_supports_split_and_equals_forms() {
        let split_form = extract_language_standard(
            &args(&["-O2", "-std", "c++11", "-c", "unit.cpp"]),
            ParserLanguage::Cpp,
        );
        assert_eq!(split_form.as_deref(), Some("c++11"));

        let equals_form = extract_language_standard(
            &args(&["-Wall", "-std=gnu++17", "-c", "unit.cpp"]),
            ParserLanguage::Cpp,
        );
        assert_eq!(equals_form.as_deref(), Some("gnu++17"));
    }

    #[test]
    fn extract_language_standard_prefers_last_matching_flag() {
        let detected = extract_language_standard(
            &args(&["-std=c++11", "-std=gnu++17", "-c", "unit.cpp"]),
            ParserLanguage::Cpp,
        );
        assert_eq!(detected.as_deref(), Some("gnu++17"));
    }

    #[test]
    fn extract_language_standard_ignores_mismatched_language_family() {
        let c_from_cpp =
            extract_language_standard(&args(&["-std=c++20", "-c", "unit.c"]), ParserLanguage::C);
        assert_eq!(c_from_cpp, None);

        let cpp_from_c =
            extract_language_standard(&args(&["-std=c11", "-c", "unit.cpp"]), ParserLanguage::Cpp);
        assert_eq!(cpp_from_c, None);
    }

    #[test]
    fn strict_mode_validation_accepts_strict() {
        assert!(
            validate_strict_mode_value("strict").is_ok(),
            "strict mode should be accepted"
        );
    }

    #[test]
    fn strict_mode_validation_rejects_auto_and_pass() {
        let auto_err = validate_strict_mode_value("auto").expect_err("auto mode must be rejected");
        assert!(
            auto_err.contains("removed"),
            "unexpected error: {}",
            auto_err
        );

        let pass_err = validate_strict_mode_value("pass").expect_err("pass mode must be rejected");
        assert!(
            pass_err.contains("removed"),
            "unexpected error: {}",
            pass_err
        );
    }

    #[test]
    fn crate_name_sanitizes_non_identifier_chars() {
        assert_eq!(
            crate_name_for_source(Path::new("hello-world.cpp")),
            "hello_world"
        );
        assert_eq!(crate_name_for_source(Path::new("1x.c")), "fragile_1x");
    }

    #[test]
    fn strict_compile_rejects_multi_source_single_output() {
        let parsed = ParsedInvocation::parse(args(&["-c", "a.cpp", "b.cpp", "-o", "out.o"]));
        let err =
            run_fragile_compile(&parsed).expect_err("multi-source single -o must be rejected");
        assert!(
            err.contains("multiple sources") && err.contains("single `-o`"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn link_requires_main_for_default_executable_shape() {
        let parsed = ParsedInvocation::parse(args(&["program.cpp", "-o", "program"]));
        assert!(
            link_requires_program_main(&parsed),
            "plain executable links should require a real main"
        );
    }

    #[test]
    fn link_does_not_require_main_for_shared_link_flag() {
        let parsed = ParsedInvocation::parse(args(&["-shared", "unit.cpp", "-o", "libunit.so"]));
        assert!(
            !link_requires_program_main(&parsed),
            "shared library links should not require a program main"
        );
    }

    #[test]
    fn link_does_not_require_main_for_non_executable_output_suffixes() {
        let lib = ParsedInvocation::parse(args(&["unit.o", "-o", "libunit.so.1.2"]));
        assert!(
            !link_requires_program_main(&lib),
            "versioned shared-object outputs should not require main"
        );

        let archive = ParsedInvocation::parse(args(&["unit.o", "-o", "libunit.a"]));
        assert!(
            !link_requires_program_main(&archive),
            "archive outputs should not require main"
        );
    }

    #[test]
    fn link_does_not_require_main_when_custom_entrypoint_is_set() {
        let parsed = ParsedInvocation::parse(args(&[
            "start.o",
            "-Wl,-e,_custom_entry",
            "-o",
            "kernel_like_binary",
        ]));
        assert!(
            !link_requires_program_main(&parsed),
            "custom linker entrypoint should disable default main requirement"
        );
    }

    #[test]
    fn linker_output_reports_missing_main_detects_common_linker_messages() {
        assert!(linker_output_reports_missing_main(
            "/usr/bin/ld: foo.o: undefined reference to `main'\ncollect2: error: ld returned 1 exit status"
        ));
        assert!(linker_output_reports_missing_main(
            "ld.lld: error: undefined symbol: _main"
        ));
        assert!(linker_output_reports_missing_main(
            "LINK : error LNK2001: unresolved external symbol main"
        ));
    }

    #[test]
    fn linker_output_reports_missing_main_ignores_unrelated_link_failures() {
        assert!(!linker_output_reports_missing_main(
            "/usr/bin/ld: foo.o: undefined reference to `puts'"
        ));
    }

    #[test]
    fn collect_link_input_objects_for_main_scan_dedupes_and_filters_missing_files() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragilec_main_scan_test_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let compiled_obj = temp_dir.join("compiled.o");
        let arg_obj = temp_dir.join("arg.o");
        let missing_obj = temp_dir.join("missing.o");
        fs::write(&compiled_obj, b"").expect("failed to create compiled object marker");
        fs::write(&arg_obj, b"").expect("failed to create arg object marker");

        let parsed = ParsedInvocation::parse(vec![
            OsString::from(arg_obj.to_string_lossy().to_string()),
            OsString::from(arg_obj.to_string_lossy().to_string()),
            OsString::from(missing_obj.to_string_lossy().to_string()),
            OsString::from("-o"),
            OsString::from(temp_dir.join("out_bin").to_string_lossy().to_string()),
        ]);
        let compiled_positions = vec![
            (0usize, compiled_obj.clone()),
            (1usize, compiled_obj.clone()),
        ];
        let objects =
            collect_link_input_objects_for_main_scan(&parsed, &compiled_positions, Path::new("/"));

        assert_eq!(
            objects,
            vec![compiled_obj, arg_obj],
            "main scan should keep unique existing object inputs in stable order"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn format_main_symbol_diagnostic_reports_defining_and_inspected_sets() {
        let inspected = vec![PathBuf::from("a.o"), PathBuf::from("b.o")];
        let defining = vec![PathBuf::from("b.o")];
        let diag = format_main_symbol_diagnostic(&inspected, &defining);
        assert!(
            diag.contains("defining objects: b.o"),
            "diagnostic should report defining objects, got:\n{}",
            diag
        );
        assert!(
            diag.contains("inspected objects: a.o, b.o"),
            "diagnostic should report inspected objects, got:\n{}",
            diag
        );
    }

    #[test]
    fn format_main_symbol_diagnostic_reports_none_for_empty_sets() {
        let diag = format_main_symbol_diagnostic(&[], &[]);
        assert!(
            diag.contains("defining objects: <none>"),
            "diagnostic should include explicit empty defining set, got:\n{}",
            diag
        );
        assert!(
            diag.contains("inspected objects: <none>"),
            "diagnostic should include explicit empty inspected set, got:\n{}",
            diag
        );
    }

    #[test]
    fn strict_compile_source_with_main_exports_main_symbol() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragilec_main_export_test_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("program.cpp");
        let out_obj = temp_dir.join("program.o");
        fs::write(&source, "int main() { return 0; }\n").expect("failed to write source");

        strict_compile_source_to_object(&source, &out_obj, &[], &[], &[])
            .expect("strict compile should succeed");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );
        assert!(
            object_defines_main_symbol(&out_obj).expect("failed to inspect object symbols"),
            "strict-compiled object should define main symbol: {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_via_driver_resolves_relative_include_paths_from_invocation_cwd() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let cwd = std::env::current_dir().expect("failed to read current directory");
        let project_rel = PathBuf::from("target")
            .join(format!("fragilec_relative_include_resolution_test_{stamp}"));
        let temp_dir = cwd.join(&project_rel);
        let include_dir = temp_dir.join("include");
        let source_dir = temp_dir.join("src");
        let source = source_dir.join("program.cpp");
        let out_obj = temp_dir.join("program.o");

        fs::create_dir_all(&include_dir).expect("failed to create include dir");
        fs::create_dir_all(&source_dir).expect("failed to create source dir");
        fs::write(
            include_dir.join("foo.h"),
            "#pragma once\n#define FOO_VALUE 0\n",
        )
        .expect("failed to write header");
        fs::write(
            &source,
            "#include \"foo.h\"\nint main() { return FOO_VALUE; }\n",
        )
        .expect("failed to write source");

        let include_rel = project_rel.join("include");
        let parsed = ParsedInvocation::parse(vec![
            OsString::from("-c"),
            OsString::from(source.to_string_lossy().to_string()),
            OsString::from("-I"),
            OsString::from(include_rel.to_string_lossy().to_string()),
            OsString::from("-o"),
            OsString::from(out_obj.to_string_lossy().to_string()),
        ]);

        run_fragile_compile(&parsed)
            .expect("strict driver compile should resolve relative -I against invocation cwd");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_via_driver_resolves_relative_forced_include_paths_from_invocation_cwd() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let cwd = std::env::current_dir().expect("failed to read current directory");
        let project_rel =
            PathBuf::from("target").join(format!("fragilec_relative_forced_include_test_{stamp}"));
        let temp_dir = cwd.join(&project_rel);
        let force_dir = temp_dir.join("force");
        let source_dir = temp_dir.join("src");
        let source = source_dir.join("program.cpp");
        let out_obj = temp_dir.join("program.o");

        fs::create_dir_all(&force_dir).expect("failed to create force include dir");
        fs::create_dir_all(&source_dir).expect("failed to create source dir");
        fs::write(
            force_dir.join("config.hpp"),
            "#pragma once\n#define FORCED_VALUE 7\n",
        )
        .expect("failed to write forced include header");
        fs::write(
            &source,
            "int main() { return FORCED_VALUE == 7 ? 0 : 1; }\n",
        )
        .expect("failed to write source");

        let forced_include_rel = project_rel.join("force/config.hpp");
        let parsed = ParsedInvocation::parse(vec![
            OsString::from("-c"),
            OsString::from(source.to_string_lossy().to_string()),
            OsString::from("-include"),
            OsString::from(forced_include_rel.to_string_lossy().to_string()),
            OsString::from("-o"),
            OsString::from(out_obj.to_string_lossy().to_string()),
        ]);

        run_fragile_compile(&parsed).expect(
            "strict driver compile should resolve relative -include against invocation cwd",
        );
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_source_with_explicit_libtooling_backend_exports_main_symbol() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("fragilec_libtooling_backend_test_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("program.cpp");
        let out_obj = temp_dir.join("program.o");
        fs::write(&source, "int main() { return 0; }\n").expect("failed to write source");

        strict_compile_source_to_object_with_backend(
            &source,
            &out_obj,
            &[],
            &[],
            &[],
            ParserBackend::Libtooling,
        )
        .expect("strict compile should succeed with explicit libtooling backend");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );
        assert!(
            object_defines_main_symbol(&out_obj).expect("failed to inspect object symbols"),
            "strict-compiled object should define main symbol when using explicit libtooling backend: {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_source_with_explicit_libtooling_backend_repeated_compile_exports_main_symbol()
    {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("fragilec_libtooling_backend_repeat_test_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("program.cpp");
        let out_obj = temp_dir.join("program.o");
        fs::write(&source, "int main() { return 0; }\n").expect("failed to write source");

        strict_compile_source_to_object_with_backend(
            &source,
            &out_obj,
            &[],
            &[],
            &[],
            ParserBackend::Libtooling,
        )
        .expect("strict compile should succeed with explicit libtooling backend");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );
        assert!(
            object_defines_main_symbol(&out_obj).expect("failed to inspect object symbols"),
            "strict-compiled object should define main symbol when using explicit libtooling backend: {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_degraded_main_shape_still_exports_main_symbol() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("fragilec_main_degraded_shape_test_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("program.cpp");
        let out_obj = temp_dir.join("program.o");
        fs::write(
            &source,
            r#"
int main(int argc, char** argv) {
    return (argc ? 0 : 0) + (argv ? 0 : 0);
}
"#,
        )
        .expect("failed to write source");

        strict_compile_source_to_object_with_backend(
            &source,
            &out_obj,
            &[],
            &[],
            &[],
            ParserBackend::Libtooling,
        )
        .expect("strict compile should preserve degraded main body shapes");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );
        assert!(
            object_defines_main_symbol(&out_obj).expect("failed to inspect object symbols"),
            "strict-compiled degraded-shape main should define main symbol: {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_ignores_rapidjson_const_assignment_parser_diagnostic() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "fragilec_rapidjson_const_assign_diag_test_{}",
            stamp
        ));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let include_dir = temp_dir.join("include/rapidjson");
        fs::create_dir_all(&include_dir).expect("failed to create include dir");
        let header = include_dir.join("document.h");
        let source = temp_dir.join("rapidjson_like.cpp");
        let out_obj = temp_dir.join("rapidjson_like.o");
        fs::write(
            &header,
            r#"
typedef unsigned int SizeType;
template<typename CharType>
struct GenericStringRef {
    const CharType* const s;
    const SizeType length;
    GenericStringRef(const CharType* str, SizeType len) : s(str), length(len) {}
    GenericStringRef& operator=(const GenericStringRef& rhs) { s = rhs.s; length = rhs.length; return *this; }
};
"#,
        )
        .expect("failed to write header");
        fs::write(
            &source,
            r#"
#include "rapidjson/document.h"
int main() { return 0; }
"#,
        )
        .expect("failed to write source");

        strict_compile_source_to_object(
            &source,
            &out_obj,
            &[IncludeDirective {
                kind: IncludeDirectiveKind::Include,
                path: temp_dir.join("include").to_string_lossy().to_string(),
            }],
            &[],
            &[],
        )
        .expect(
            "strict compile should tolerate known rapidjson const-member assignment parse diagnostic",
        );
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_libtooling_ignores_rapidjson_const_assignment_parser_diagnostic() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "fragilec_rapidjson_const_assign_diag_libtooling_test_{}",
            stamp
        ));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let include_dir = temp_dir.join("include/rapidjson");
        fs::create_dir_all(&include_dir).expect("failed to create include dir");
        let header = include_dir.join("document.h");
        let source = temp_dir.join("rapidjson_like.cpp");
        let out_obj = temp_dir.join("rapidjson_like.o");
        fs::write(
            &header,
            r#"
typedef unsigned int SizeType;
template<typename CharType>
struct GenericStringRef {
    const CharType* const s;
    const SizeType length;
    GenericStringRef(const CharType* str, SizeType len) : s(str), length(len) {}
    GenericStringRef& operator=(const GenericStringRef& rhs) { s = rhs.s; length = rhs.length; return *this; }
};
"#,
        )
        .expect("failed to write header");
        fs::write(
            &source,
            r#"
#include "rapidjson/document.h"
int main() { return 0; }
"#,
        )
        .expect("failed to write source");

        strict_compile_source_to_object_with_backend(
            &source,
            &out_obj,
            &[IncludeDirective {
                kind: IncludeDirectiveKind::Include,
                path: temp_dir.join("include").to_string_lossy().to_string(),
            }],
            &[],
            &[],
            ParserBackend::Libtooling,
        )
        .expect("strict compile should tolerate known rapidjson diagnostic in libtooling mode");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_non_rapidjson_const_assignment_diagnostic_is_non_fatal() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "fragilec_non_rapidjson_const_assign_diag_test_{}",
            stamp
        ));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("rapidjson_like.cpp");
        let out_obj = temp_dir.join("rapidjson_like.o");
        fs::write(
            &source,
            r#"
typedef unsigned int SizeType;
template<typename CharType>
struct GenericStringRef {
    const CharType* const s;
    const SizeType length;
    GenericStringRef(const CharType* str, SizeType len) : s(str), length(len) {}
    GenericStringRef& operator=(const GenericStringRef& rhs) { s = rhs.s; length = rhs.length; return *this; }
};
int main() { return 0; }
"#,
        )
        .expect("failed to write source");

        strict_compile_source_to_object(&source, &out_obj, &[], &[], &[])
            .expect("libtooling-only strict compile should continue without libclang precheck");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn strict_compile_libtooling_non_rapidjson_const_assignment_diagnostic_is_non_fatal() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "fragilec_non_rapidjson_const_assign_diag_libtooling_test_{}",
            stamp
        ));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("rapidjson_like.cpp");
        let out_obj = temp_dir.join("rapidjson_like.o");
        fs::write(
            &source,
            r#"
typedef unsigned int SizeType;
template<typename CharType>
struct GenericStringRef {
    const CharType* const s;
    const SizeType length;
    GenericStringRef(const CharType* str, SizeType len) : s(str), length(len) {}
    GenericStringRef& operator=(const GenericStringRef& rhs) { s = rhs.s; length = rhs.length; return *this; }
};
int main() { return 0; }
"#,
        )
        .expect("failed to write source");

        strict_compile_source_to_object_with_backend(
            &source,
            &out_obj,
            &[],
            &[],
            &[],
            ParserBackend::Libtooling,
        )
        .expect("libtooling strict compile should continue without libclang precheck");
        assert!(
            out_obj.exists(),
            "expected object output at {}",
            out_obj.display()
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn cmake_probe_passthrough_detects_probe_source_names() {
        let parsed = ParsedInvocation::parse(args(&["CMakeCXXCompilerId.cpp"]));
        assert!(
            should_passthrough_cmake_compiler_probe(&parsed, &parsed.args, Path::new("/tmp/work")),
            "compiler-id source file should trigger cmake-probe passthrough"
        );

        let parsed =
            ParsedInvocation::parse(args(&["/usr/share/cmake/Modules/CMakeCXXCompilerABI.cpp"]));
        assert!(
            should_passthrough_cmake_compiler_probe(&parsed, &parsed.args, Path::new("/tmp/work")),
            "compiler-abi source file should trigger cmake-probe passthrough"
        );
    }

    #[test]
    fn cmake_probe_passthrough_detects_trycompile_working_dir_without_source_tokens() {
        let parsed = ParsedInvocation::parse(args(&[
            "CMakeFiles/cmTC_123.dir/testCXXCompiler.cxx.o",
            "-o",
            "cmTC_123",
        ]));
        assert!(
            should_passthrough_cmake_compiler_probe(
                &parsed,
                &parsed.args,
                Path::new("/tmp/build/CMakeFiles/CMakeScratch/TryCompile-abc123")
            ),
            "try-compile working dir should trigger cmake-probe passthrough even when invocation has no source token"
        );
    }

    #[test]
    fn cmake_probe_passthrough_detects_compiler_id_working_dir_for_empty_invocation() {
        let parsed = ParsedInvocation::parse(Vec::new());
        assert!(
            should_passthrough_cmake_compiler_probe(
                &parsed,
                &parsed.args,
                Path::new("/tmp/build/CMakeFiles/3.31.6/CompilerIdCXX")
            ),
            "compiler-id working dir should trigger cmake-probe passthrough for empty probe invocations"
        );
    }

    #[test]
    fn cmake_probe_passthrough_ignores_regular_project_compilation() {
        let parsed = ParsedInvocation::parse(args(&["-c", "src/main.cpp", "-o", "main.o"]));
        assert!(
            !should_passthrough_cmake_compiler_probe(
                &parsed,
                &parsed.args,
                Path::new("/tmp/build/example/CMakeFiles/condense.dir")
            ),
            "regular project compile invocations should remain on strict fragile path"
        );
    }
}
