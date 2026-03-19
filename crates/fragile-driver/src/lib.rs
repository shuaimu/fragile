use fragile_clang::{
    IncludeDirective, IncludeDirectiveKind, ParserBackend as ClangParserBackend,
    ParserLanguage as ClangParserLanguage, ParserOutputCodegenOptions, TemplateParsingMode,
    TranspileOptions,
};
use fragile_parser_clang::{FragileParserClangBackend, FRAGILE_PARSER_CLANG_BACKEND_ID};
use fragile_parser_core::{
    BackendRegistry, IncludeDirective as CoreIncludeDirective,
    IncludeDirectiveKind as CoreIncludeDirectiveKind, ParseRequest, ParserOutputV1,
    ParserLanguage as CoreParserLanguage, PARSER_OUTPUT_SCHEMA_VERSION_V1,
};
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const FRAGILEC_BUILD_ID_ENV: &str = "FRAGILEC_BUILD_ID";
pub const FRAGILEC_KEEP_RS_ENV: &str = "FRAGILEC_KEEP_RS";
pub const FRAGILEC_PARSER_BACKEND_ENV: &str = "FRAGILEC_PARSER_BACKEND";
pub const FRAGILEC_PARSER_CORE_MANIFEST_DIR_ENV: &str = "FRAGILEC_PARSER_CORE_MANIFEST_DIR";
pub const FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV: &str =
    "FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH";
pub const FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV: &str = "FRAGILEC_TRANSPILE_STAGE_TIMING_PATH";
pub const FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV: &str = "FRAGILEC_ESCAPE_HATCH_LOG_PATH";

/// Hardening window expiry: escape hatches are deprecated immediately and will
/// be rejected after this date (YYYY-MM-DD).  The window gives downstream users
/// one release cycle to migrate away from `FRAGILEC_PARSER_BACKEND=libtooling`
/// and `FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling`.
pub const ESCAPE_HATCH_HARDENING_EXPIRY: &str = "2026-04-18";

#[derive(Debug, Clone, PartialEq, Eq)]
enum StrictParserBackend {
    Libtooling,
    ParserCore { backend_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserCoreCodegenEscapeHatch {
    Libtooling,
}

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

fn source_language(source: &Path) -> ClangParserLanguage {
    if is_c_file(source) {
        ClangParserLanguage::C
    } else {
        ClangParserLanguage::Cpp
    }
}

fn normalize_language_standard(raw: &str, language: ClangParserLanguage) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let is_cpp_standard = lower.contains("++");
    let is_c_standard = !is_cpp_standard && (lower.starts_with('c') || lower.starts_with("gnu"));
    match language {
        ClangParserLanguage::Cpp => {
            if is_cpp_standard {
                Some(trimmed.to_string())
            } else {
                None
            }
        }
        ClangParserLanguage::C => {
            if is_c_standard {
                Some(trimmed.to_string())
            } else {
                None
            }
        }
    }
}

fn extract_language_standard(args: &[OsString], language: ClangParserLanguage) -> Option<String> {
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

fn strict_parser_ignored_error_patterns(language: ClangParserLanguage) -> Vec<String> {
    let _ = language;
    Vec::new()
}

fn supported_parser_backend_values_message() -> String {
    format!("libtooling, {}", FRAGILE_PARSER_CLANG_BACKEND_ID)
}

fn parse_parser_backend_value(backend: &str) -> Result<StrictParserBackend, String> {
    let normalized = backend.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "libtooling" => Ok(StrictParserBackend::Libtooling),
        FRAGILE_PARSER_CLANG_BACKEND_ID => Ok(StrictParserBackend::ParserCore {
            backend_id: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string(),
        }),
        other => Err(format!(
            "unsupported FRAGILEC_PARSER_BACKEND value `{}`; expected one of: {}",
            other,
            supported_parser_backend_values_message()
        )),
    }
}

fn strict_parser_backend_from_value(raw: Option<&str>) -> Result<StrictParserBackend, String> {
    match raw.map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(backend) => parse_parser_backend_value(backend),
        None => Ok(StrictParserBackend::ParserCore {
            backend_id: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string(),
        }),
    }
}

fn strict_parser_backend_from_env() -> Result<StrictParserBackend, String> {
    let raw = std::env::var(FRAGILEC_PARSER_BACKEND_ENV).ok();
    strict_parser_backend_from_value(raw.as_deref())
}

fn strict_parser_backend_label(backend: &StrictParserBackend) -> &str {
    match backend {
        StrictParserBackend::Libtooling => "libtooling",
        StrictParserBackend::ParserCore { backend_id } => backend_id.as_str(),
    }
}

fn supported_parser_core_codegen_escape_hatch_values_message() -> &'static str {
    "libtooling"
}

fn parse_parser_core_codegen_escape_hatch_value(
    raw: &str,
) -> Result<ParserCoreCodegenEscapeHatch, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "libtooling" => Ok(ParserCoreCodegenEscapeHatch::Libtooling),
        other => Err(format!(
            "unsupported FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH value `{}`; expected one of: {}",
            other,
            supported_parser_core_codegen_escape_hatch_values_message()
        )),
    }
}

fn parser_core_codegen_escape_hatch_from_value(
    raw: Option<&str>,
) -> Result<Option<ParserCoreCodegenEscapeHatch>, String> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => parse_parser_core_codegen_escape_hatch_value(value).map(Some),
        None => Ok(None),
    }
}

fn parser_core_codegen_escape_hatch_from_env(
) -> Result<Option<ParserCoreCodegenEscapeHatch>, String> {
    let raw = std::env::var(FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH_ENV).ok();
    parser_core_codegen_escape_hatch_from_value(raw.as_deref())
}

/// Returns true if today's date is past the hardening window expiry.
pub fn escape_hatch_hardening_expired() -> bool {
    escape_hatch_hardening_expired_as_of(today_date_string().as_str())
}

/// Testable variant: returns true if `today` (YYYY-MM-DD) is strictly after the
/// hardening expiry date.
pub fn escape_hatch_hardening_expired_as_of(today: &str) -> bool {
    today > ESCAPE_HATCH_HARDENING_EXPIRY
}

fn today_date_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 86400 seconds per day; epoch is 1970-01-01 (Thursday).
    let days = secs / 86400;
    // Simple Gregorian calendar conversion.
    let (year, month, day) = days_since_epoch_to_ymd(days);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn days_since_epoch_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Emit a deprecation warning to stderr for escape hatch usage.
pub fn emit_escape_hatch_deprecation_warning(escape_kind: &str, source: &str) {
    eprintln!(
        "[fragilec] DEPRECATION WARNING: {} escape hatch is deprecated and will be \
         removed after {}. Migrate to the default fragile-parser-clang backend. \
         (source: {})",
        escape_kind, ESCAPE_HATCH_HARDENING_EXPIRY, source
    );
}

/// Log escape hatch usage to the file at `FRAGILEC_ESCAPE_HATCH_LOG_PATH` if set.
pub fn log_escape_hatch_usage(escape_kind: &str, source: &str) {
    let log_path = match std::env::var(FRAGILEC_ESCAPE_HATCH_LOG_PATH_ENV).ok() {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p.trim()),
        _ => return,
    };
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let entry = format!(
        "timestamp={} escape_kind={} source={} pid={}\n",
        timestamp,
        escape_kind,
        source,
        std::process::id()
    );
    // Append; ignore errors (best-effort telemetry).
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(entry.as_bytes())
        });
}

/// Enforce escape hatch policy: emit deprecation warning, log usage, and reject
/// if the hardening window has expired.  Returns `Ok(())` if the escape hatch
/// is still allowed (within the hardening window) or `Err` if expired.
pub fn enforce_escape_hatch_policy(escape_kind: &str, source: &str) -> Result<(), String> {
    emit_escape_hatch_deprecation_warning(escape_kind, source);
    log_escape_hatch_usage(escape_kind, source);
    if escape_hatch_hardening_expired() {
        return Err(format!(
            "escape hatch `{}` rejected: hardening window expired on {}. \
             Remove the escape hatch environment variable and use the default \
             fragile-parser-clang backend. (source: {})",
            escape_kind, ESCAPE_HATCH_HARDENING_EXPIRY, source
        ));
    }
    Ok(())
}

/// Testable variant with explicit date.
pub fn enforce_escape_hatch_policy_as_of(
    escape_kind: &str,
    source: &str,
    today: &str,
) -> Result<(), String> {
    emit_escape_hatch_deprecation_warning(escape_kind, source);
    log_escape_hatch_usage(escape_kind, source);
    if escape_hatch_hardening_expired_as_of(today) {
        return Err(format!(
            "escape hatch `{}` rejected: hardening window expired on {}. \
             Remove the escape hatch environment variable and use the default \
             fragile-parser-clang backend. (source: {})",
            escape_kind, ESCAPE_HATCH_HARDENING_EXPIRY, source
        ));
    }
    Ok(())
}

#[cfg(test)]
fn strict_parser_backend_from_legacy_backend(
    backend: ClangParserBackend,
) -> Result<StrictParserBackend, String> {
    match backend {
        ClangParserBackend::Libtooling => Ok(StrictParserBackend::Libtooling),
        ClangParserBackend::Libclang | ClangParserBackend::Hybrid => Err(format!(
            "legacy parser backend alias `{}` is unsupported in strict mode; expected one of: {}",
            match backend {
                ClangParserBackend::Libclang => "libclang",
                ClangParserBackend::Hybrid => "hybrid",
                ClangParserBackend::Libtooling => "libtooling",
            },
            supported_parser_backend_values_message()
        )),
    }
}

fn parser_core_language(language: ClangParserLanguage) -> CoreParserLanguage {
    match language {
        ClangParserLanguage::C => CoreParserLanguage::C,
        ClangParserLanguage::Cpp => CoreParserLanguage::Cpp,
    }
}

fn parser_core_include_directives(
    include_directives: &[IncludeDirective],
) -> Vec<CoreIncludeDirective> {
    include_directives
        .iter()
        .map(|directive| CoreIncludeDirective {
            kind: match directive.kind {
                IncludeDirectiveKind::Include => CoreIncludeDirectiveKind::Include,
                IncludeDirectiveKind::System => CoreIncludeDirectiveKind::System,
                IncludeDirectiveKind::Quote => CoreIncludeDirectiveKind::Quote,
            },
            path: directive.path.clone(),
        })
        .collect()
}

fn parser_core_manifest_dir_from_env() -> Option<PathBuf> {
    std::env::var(FRAGILEC_PARSER_CORE_MANIFEST_DIR_ENV)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .map(PathBuf::from)
}

fn parser_core_manifest_path_for_source(manifest_dir: &Path, source: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    source.display().to_string().hash(&mut hasher);
    let hash = hasher.finish();
    let file_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unit");
    manifest_dir.join(format!("{}_{}.parser_core_manifest.txt", file_stem, hash))
}

fn parser_core_language_label(language: CoreParserLanguage) -> &'static str {
    match language {
        CoreParserLanguage::C => "c",
        CoreParserLanguage::Cpp => "cpp",
    }
}

fn parser_core_parse_manifest(output: &ParserOutputV1) -> String {
    let mut node_kind_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for node in &output.nodes {
        *node_kind_counts.entry(node.node_kind.as_str()).or_insert(0) += 1;
    }

    let mut manifest = String::new();
    manifest.push_str(&format!("schema_version={}\n", output.schema_version));
    manifest.push_str(&format!(
        "parser_backend={}\n",
        output.translation_unit.parser_backend
    ));
    manifest.push_str(&format!(
        "source={}\n",
        output.translation_unit.source_path.display()
    ));
    manifest.push_str(&format!(
        "language={}\n",
        parser_core_language_label(output.translation_unit.language)
    ));
    manifest.push_str(&format!(
        "frontend_arg_count={}\n",
        output.translation_unit.frontend_args.len()
    ));
    manifest.push_str(&format!(
        "define_count={}\n",
        output.translation_unit.defines.len()
    ));
    manifest.push_str(&format!(
        "include_directive_count={}\n",
        output.translation_unit.include_directives.len()
    ));
    manifest.push_str(&format!("node_count={}\n", output.nodes.len()));
    manifest.push_str(&format!("diagnostic_count={}\n", output.diagnostics.len()));
    if let Some(first) = output.nodes.first() {
        manifest.push_str(&format!("first_node_id={}\n", first.node_id));
    }
    if let Some(last) = output.nodes.last() {
        manifest.push_str(&format!("last_node_id={}\n", last.node_id));
    }
    for (kind, count) in node_kind_counts {
        manifest.push_str(&format!("node_kind.{}={}\n", kind, count));
    }
    manifest
}

fn write_parser_core_parse_manifest(
    manifest_dir: Option<&Path>,
    source: &Path,
    output: &ParserOutputV1,
) -> Result<Option<PathBuf>, String> {
    let Some(manifest_dir) = manifest_dir else {
        return Ok(None);
    };
    fs::create_dir_all(manifest_dir).map_err(|err| {
        format!(
            "failed to create parser-core manifest directory {}: {}",
            manifest_dir.display(),
            err
        )
    })?;
    let manifest_path = parser_core_manifest_path_for_source(manifest_dir, source);
    fs::write(&manifest_path, parser_core_parse_manifest(output)).map_err(|err| {
        format!(
            "failed to write parser-core manifest {}: {}",
            manifest_path.display(),
            err
        )
    })?;
    Ok(Some(manifest_path))
}

fn maybe_write_parser_core_parse_manifest(
    source: &Path,
    output: &ParserOutputV1,
) -> Result<(), String> {
    let manifest_dir = parser_core_manifest_dir_from_env();
    write_parser_core_parse_manifest(manifest_dir.as_deref(), source, output)?;
    Ok(())
}

fn run_parser_core_backend_parse(
    source: &Path,
    language: ClangParserLanguage,
    includes: &[IncludeDirective],
    defines: &[String],
    frontend_args: &[String],
    backend_id: &str,
    cwd: &Path,
) -> Result<ParserOutputV1, String> {
    let mut registry = BackendRegistry::new();
    registry
        .register(FragileParserClangBackend)
        .map_err(|err| format!("failed to register parser backend `{}`: {}", backend_id, err))?;
    let request = ParseRequest {
        source_path: source.to_path_buf(),
        language: parser_core_language(language),
        frontend_args: frontend_args.to_vec(),
        defines: defines.to_vec(),
        include_directives: parser_core_include_directives(includes),
    };
    with_current_dir(cwd, || {
        let output = registry.parse_with(backend_id, &request).map_err(|err| {
            format!(
                "parser backend `{}` preflight parse failed for {}: {}",
                backend_id,
                source.display(),
                err
            )
        })?;
        if output.schema_version != PARSER_OUTPUT_SCHEMA_VERSION_V1 {
            return Err(format!(
                "parser backend `{}` returned schema version `{}`; expected `{}`",
                backend_id, output.schema_version, PARSER_OUTPUT_SCHEMA_VERSION_V1
            ));
        }
        maybe_write_parser_core_parse_manifest(source, &output)?;
        Ok(output)
    })
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
    parser_backend: &StrictParserBackend,
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
    let parser_core_codegen_escape_hatch = parser_core_codegen_escape_hatch_from_env()?;
    let mut use_libtooling_codegen_escape_hatch = false;
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

    let compile_transpiled_text = |transpiled: String, context: &str| -> Result<(), String> {
        let transpiled = normalize_transpiled_main_entry(transpiled);
        enforce_unresolved_type_invariant(&source, &transpiled)?;

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
                "fragile rustc object compile failed for {} ({})\nstdout:\n{}\nstderr:\n{}",
                source.display(),
                context,
                String::from_utf8_lossy(&rustc.stdout),
                String::from_utf8_lossy(&rustc.stderr)
            ));
        }
        Ok(())
    };

    // Enforce escape hatch policy for explicit libtooling backend override.
    if matches!(parser_backend, StrictParserBackend::Libtooling) {
        enforce_escape_hatch_policy(
            "FRAGILEC_PARSER_BACKEND=libtooling",
            &source.display().to_string(),
        )?;
    }

    if let StrictParserBackend::ParserCore { backend_id } = parser_backend {
        let parser_output = run_parser_core_backend_parse(
            &source,
            language,
            includes,
            defines,
            frontend_args,
            backend_id.as_str(),
            cwd,
        )?;
        if matches!(
            parser_core_codegen_escape_hatch,
            Some(ParserCoreCodegenEscapeHatch::Libtooling)
        ) {
            // Enforce escape hatch policy for codegen escape hatch.
            enforce_escape_hatch_policy(
                "FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH=libtooling",
                &source.display().to_string(),
            )?;
            use_libtooling_codegen_escape_hatch = true;
        } else {
            let transpiled = with_current_dir(cwd, || {
                fragile_clang::transpile_parser_output_to_rust_with_options(
                    &parser_output,
                    &ParserOutputCodegenOptions {
                        ignored_error_patterns: strict_parser_ignored_error_patterns(language),
                        stage_timing_trace_path: stage_timing_trace_path.clone(),
                    },
                )
                .map_err(|e| {
                    format!(
                        "failed parser-output handoff transpile for {} with parser backend {}: {}",
                        source.display(),
                        strict_parser_backend_label(parser_backend),
                        e
                    )
                })
            })?;
            compile_transpiled_text(transpiled, "parser-output-handoff")?;
            if !keep_rs {
                let _ = fs::remove_file(&transpiled_rs);
            }
            write_meta_file(&source, out_obj, args_for_meta)?;
            return Ok(());
        }
    }

    let codegen_backend_label = if use_libtooling_codegen_escape_hatch {
        format!(
            "{}+libtooling-escape-hatch",
            strict_parser_backend_label(parser_backend)
        )
    } else {
        strict_parser_backend_label(parser_backend).to_string()
    };

    with_current_dir(cwd, || {
        let transpile_options = TranspileOptions {
            include_paths: Vec::new(),
            include_directives: includes.to_vec(),
            frontend_args: frontend_args.to_vec(),
            defines: defines.to_vec(),
            language,
            language_standard,
            ignored_error_patterns: strict_parser_ignored_error_patterns(language),
            backend: ClangParserBackend::Libtooling,
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
                        "failed to transpile {} with parser backend {}: {}",
                        source.display(),
                        codegen_backend_label,
                        e
                    )
                })?;
        compile_transpiled_text(
            transpiled,
            format!("backend={}", codegen_backend_label).as_str(),
        )?;

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
            &parser_backend,
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
    use fragile_parser_core::ParserLanguage as CoreParserLanguage;

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

    #[test]
    fn strict_parser_backend_validation_accepts_parser_core_backend() {
        assert_eq!(
            strict_parser_backend_from_legacy_backend(ClangParserBackend::Libtooling)
                .expect("legacy libtooling alias should map"),
            StrictParserBackend::Libtooling
        );
        strict_parser_backend_from_legacy_backend(ClangParserBackend::Libclang)
            .expect_err("legacy libclang alias should be rejected");
        assert_eq!(
            parse_parser_backend_value("libtooling").expect("libtooling should parse"),
            StrictParserBackend::Libtooling
        );
        assert_eq!(
            parse_parser_backend_value("FRAGILE-PARSER-CLANG")
                .expect("parser-core backend should parse"),
            StrictParserBackend::ParserCore {
                backend_id: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string()
            }
        );
        strict_parser_backend_from_value(Some(" libclang "))
            .expect_err("legacy libclang alias should be rejected");
        strict_parser_backend_from_value(Some(" hybrid "))
            .expect_err("legacy hybrid alias should be rejected");
    }

    #[test]
    fn strict_parser_backend_validation_error_lists_supported_values() {
        let err = parse_parser_backend_value("unsupported")
            .expect_err("unsupported backend value should be rejected");
        assert!(
            err.contains("unsupported FRAGILEC_PARSER_BACKEND value"),
            "unexpected error: {}",
            err
        );
        assert!(
            err.contains("libtooling") && err.contains(FRAGILE_PARSER_CLANG_BACKEND_ID),
            "error should list supported backend values: {}",
            err
        );
    }

    #[test]
    fn parser_core_codegen_escape_hatch_validation() {
        assert_eq!(
            parser_core_codegen_escape_hatch_from_value(None).expect("missing value should disable"),
            None
        );
        assert_eq!(
            parser_core_codegen_escape_hatch_from_value(Some("  "))
                .expect("empty value should disable"),
            None
        );
        assert_eq!(
            parser_core_codegen_escape_hatch_from_value(Some("LIBTOOLING"))
                .expect("libtooling value should parse"),
            Some(ParserCoreCodegenEscapeHatch::Libtooling)
        );
        let err = parser_core_codegen_escape_hatch_from_value(Some("unsupported"))
            .expect_err("unsupported escape hatch should fail");
        assert!(
            err.contains("FRAGILEC_PARSER_CORE_CODEGEN_ESCAPE_HATCH")
                && err.contains("libtooling"),
            "unexpected escape hatch validation error: {}",
            err
        );
    }

    #[test]
    fn parser_core_backend_routes_through_parser_output_handoff() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragile_driver_parser_core_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("program.c");
        let out_obj = temp_dir.join("program.o");
        fs::write(&source, "int main(void) { return 0; }\n").expect("failed to write source");

        strict_compile_source_to_object_with_frontend_args_and_backend(
            &source,
            &out_obj,
            &[],
            &[],
            &[],
            &[],
            &StrictParserBackend::ParserCore {
                backend_id: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string(),
            },
            &temp_dir,
        )
        .expect("parser-core backend should compile through parser-output handoff");
        assert!(
            out_obj.exists(),
            "object should be generated on parser-core handoff compile"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn parser_core_manifest_writer_emits_deterministic_summary() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragile_driver_manifest_{}", stamp));
        let manifest_dir = temp_dir.join("manifests");
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("unit.c");
        fs::write(
            &source,
            "int add(int a, int b) { return a + b; }\nint main(void) { return add(1, 2); }\n",
        )
        .expect("failed to write source fixture");

        let mut registry = BackendRegistry::new();
        registry
            .register(FragileParserClangBackend)
            .expect("failed to register parser-core backend");
        let request = ParseRequest {
            source_path: source.clone(),
            language: CoreParserLanguage::C,
            frontend_args: Vec::new(),
            defines: Vec::new(),
            include_directives: Vec::new(),
        };
        let output = registry
            .parse_with(FRAGILE_PARSER_CLANG_BACKEND_ID, &request)
            .expect("parser-core parse should succeed");

        let manifest_path = write_parser_core_parse_manifest(Some(&manifest_dir), &source, &output)
            .expect("manifest write should succeed")
            .expect("manifest path should be present");
        let first = fs::read_to_string(&manifest_path).expect("failed to read first manifest");
        let second_path = write_parser_core_parse_manifest(Some(&manifest_dir), &source, &output)
            .expect("second manifest write should succeed")
            .expect("second manifest path should be present");
        let second = fs::read_to_string(&second_path).expect("failed to read second manifest");

        assert_eq!(manifest_path, second_path, "manifest path should be deterministic");
        assert_eq!(first, second, "manifest content should be deterministic");
        assert!(
            first.contains("schema_version=1.0.0")
                && first.contains("language=c")
                && first.contains("node_kind.function_decl=")
                && first.contains("node_count="),
            "manifest missing required summary fields:\n{}",
            first
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn escape_hatch_hardening_expiry_within_window() {
        assert!(!escape_hatch_hardening_expired_as_of("2026-03-18"));
        assert!(!escape_hatch_hardening_expired_as_of("2026-04-18"));
    }

    #[test]
    fn escape_hatch_hardening_expiry_after_window() {
        assert!(escape_hatch_hardening_expired_as_of("2026-04-19"));
        assert!(escape_hatch_hardening_expired_as_of("2027-01-01"));
    }

    #[test]
    fn enforce_escape_hatch_policy_as_of_ok_within_window() {
        let result = enforce_escape_hatch_policy_as_of(
            "FRAGILEC_PARSER_BACKEND=libtooling",
            "test.cpp",
            "2026-03-20",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn enforce_escape_hatch_policy_as_of_err_after_window() {
        let result = enforce_escape_hatch_policy_as_of(
            "FRAGILEC_PARSER_BACKEND=libtooling",
            "test.cpp",
            "2026-04-19",
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("rejected"));
        assert!(err.contains(ESCAPE_HATCH_HARDENING_EXPIRY));
    }

    #[test]
    fn today_date_string_is_valid_format() {
        let today = today_date_string();
        assert_eq!(today.len(), 10);
        assert_eq!(&today[4..5], "-");
        assert_eq!(&today[7..8], "-");
    }
}
