use fragile_clang::{
    IncludeDirective, IncludeDirectiveKind, ParserBackend as ClangParserBackend,
    ParserLanguage as ClangParserLanguage, TemplateParsingMode, TranspileOptions,
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
const FRAGILEC_PARSER_CORE_MANIFEST_DIR_ENV: &str = "FRAGILEC_PARSER_CORE_MANIFEST_DIR";
const FRAGILEC_SKIP_SYSTEM_HEADERS_ENV: &str = "FRAGILEC_SKIP_SYSTEM_HEADERS";
const FRAGILEC_TRANSPILE_STAGE_TIMING_PATH_ENV: &str = "FRAGILEC_TRANSPILE_STAGE_TIMING_PATH";
const FRAGILEC_RUSTC_BIN_ENV: &str = "FRAGILEC_RUSTC_BIN";
const FRAGILEC_RUSTC_WRAPPER_ENV: &str = "FRAGILEC_RUSTC_WRAPPER";
const FRAGILEC_RUNTIME_LINK_CACHE_DIR_ENV: &str = "FRAGILEC_RUNTIME_LINK_CACHE_DIR";
const FRAGILEC_DUMP_UNRESOLVED_RS_ENV: &str = "FRAGILEC_DUMP_UNRESOLVED_RS";
const FRAGILEC_NATIVE_FALLBACK_CXX_ENV: &str = "FRAGILEC_NATIVE_FALLBACK_CXX";

#[derive(Debug, Clone, PartialEq, Eq)]
enum StrictParserBackend {
    Libtooling,
    ParserCore { backend_id: String },
}

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

fn normalized_nonempty(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn rustc_bin_from_value(raw: Option<&str>) -> String {
    normalized_nonempty(raw).unwrap_or_else(|| "rustc".to_string())
}

fn rustc_wrapper_from_value(raw: Option<&str>) -> Option<String> {
    normalized_nonempty(raw)
}

fn rustc_bin_from_env() -> String {
    rustc_bin_from_value(std::env::var(FRAGILEC_RUSTC_BIN_ENV).ok().as_deref())
}

fn rustc_wrapper_from_env() -> Option<String> {
    rustc_wrapper_from_value(std::env::var(FRAGILEC_RUSTC_WRAPPER_ENV).ok().as_deref())
        .or_else(|| rustc_wrapper_from_value(std::env::var("RUSTC_WRAPPER").ok().as_deref()))
}

fn rustc_command() -> Command {
    let rustc_bin = rustc_bin_from_env();
    if let Some(wrapper) = rustc_wrapper_from_env() {
        let mut cmd = Command::new(wrapper);
        cmd.arg(rustc_bin);
        cmd
    } else {
        Command::new(rustc_bin)
    }
}

fn rustc_invocation_fingerprint() -> String {
    let rustc_bin = rustc_bin_from_env();
    let wrapper = rustc_wrapper_from_env().unwrap_or_else(|| "<none>".to_string());
    format!(
        "fragilec={} rustc_bin={} rustc_wrapper={} os={} arch={} toolchain={}",
        env!("CARGO_PKG_VERSION"),
        rustc_bin,
        wrapper,
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_else(|_| "<default>".to_string()),
    )
}

fn env_flag_is_true(name: &str) -> bool {
    std::env::var(name)
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn skip_system_headers_from_env() -> bool {
    match std::env::var(FRAGILEC_SKIP_SYSTEM_HEADERS_ENV) {
        Ok(raw) => matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

fn native_fallback_cxx_from_env() -> String {
    normalized_nonempty(std::env::var(FRAGILEC_NATIVE_FALLBACK_CXX_ENV).ok().as_deref())
        .unwrap_or_else(|| "/usr/bin/clang++".to_string())
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
    if !skip_system_headers_from_env() {
        return kind;
    }
    if kind == IncludeDirectiveKind::Include
        && include_path_is_external_test_framework(resolved_path)
    {
        IncludeDirectiveKind::System
    } else {
        kind
    }
}

fn maybe_promote_frontend_include_flag(flag: &str, resolved_path: &str) -> String {
    if !skip_system_headers_from_env() {
        return flag.to_string();
    }
    if flag == "-I" && include_path_is_external_test_framework(resolved_path) {
        "-isystem".to_string()
    } else {
        flag.to_string()
    }
}

fn should_retry_with_system_headers_on_failure(err: &str) -> bool {
    const RETRY_NEEDLES: &[&str] = &[
        "cannot find function `gai_strerror`",
        "cannot find function `connect`",
        "cannot find function `setsockopt`",
        "cannot find function `getaddrinfo`",
        "cannot find function `freeaddrinfo`",
        "cannot find type `sockaddr`",
        "cannot find type `addrinfo`",
        "no field `ai_family`",
        "no field `ai_socktype`",
        "no field `ai_protocol`",
        "no field `ai_addr`",
        "no field `ai_addrlen`",
        "no field `ai_next`",
    ];
    RETRY_NEEDLES.iter().any(|needle| err.contains(needle))
}

fn should_native_fallback_on_failure(err: &str) -> bool {
    const NATIVE_FALLBACK_NEEDLES: &[&str] = &[
        "no method named `transition_to`",
        "cannot find function `error` in this scope",
        "no function or associated item named `make_ref_std_sync_Arc_rrr_PollThread`",
        "attempted to take value of method `borrow_mut`",
        "attempted to take value of method `upgrade`",
        "no method named `op_index` found for type `u128`",
    ];
    NATIVE_FALLBACK_NEEDLES
        .iter()
        .any(|needle| err.contains(needle))
}

fn compile_with_native_fallback(args: &[OsString], source: &Path) -> Result<(), String> {
    let compiler = native_fallback_cxx_from_env();
    let mut cmd = Command::new(&compiler);
    cmd.args(args);
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run native fallback compiler `{}`: {}", compiler, e))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "native fallback compile failed for {} via `{}`\nstdout:\n{}\nstderr:\n{}",
        source.display(),
        compiler,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmitRustCppHeaderInvocation {
    rust_source: PathBuf,
    output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustStructDef {
    name: String,
    fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustFnExport {
    name: String,
    params: Vec<(String, String)>,
    ret: Option<String>,
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

fn parse_emit_rust_cpp_header_invocation(
    args: &[OsString],
) -> Result<Option<EmitRustCppHeaderInvocation>, String> {
    let has_mode_flag = args.iter().any(|arg| {
        let token = arg.to_string_lossy();
        token == "--emit-rust-cpp-header" || token.starts_with("--emit-rust-cpp-header=")
    });
    if !has_mode_flag {
        return Ok(None);
    }

    let mut rust_source: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        let token = args[i].to_string_lossy();
        let cur = token.as_ref();
        if cur == "--emit-rust-cpp-header" {
            if i + 1 >= args.len() {
                return Err(
                    "missing source path after --emit-rust-cpp-header; expected a Rust source file"
                        .to_string(),
                );
            }
            rust_source = Some(PathBuf::from(args[i + 1].to_string_lossy().to_string()));
            i += 2;
            continue;
        }
        if let Some(path) = cur.strip_prefix("--emit-rust-cpp-header=") {
            if path.trim().is_empty() {
                return Err(
                    "empty source path in --emit-rust-cpp-header=<path>; expected a Rust source file"
                        .to_string(),
                );
            }
            rust_source = Some(PathBuf::from(path));
            i += 1;
            continue;
        }
        if cur == "-o" || cur == "--output" {
            if i + 1 >= args.len() {
                return Err(format!("missing output path after {}", cur));
            }
            output = Some(PathBuf::from(args[i + 1].to_string_lossy().to_string()));
            i += 2;
            continue;
        }
        if let Some(path) = cur.strip_prefix("-o") {
            if !path.trim().is_empty() {
                output = Some(PathBuf::from(path));
                i += 1;
                continue;
            }
        }
        if let Some(path) = cur.strip_prefix("--output=") {
            if path.trim().is_empty() {
                return Err("empty path in --output=<path>".to_string());
            }
            output = Some(PathBuf::from(path));
            i += 1;
            continue;
        }
        if cur == "--fragilec-help" || cur == "--help" {
            i += 1;
            continue;
        }

        return Err(format!(
            "unsupported argument `{}` for --emit-rust-cpp-header mode",
            cur
        ));
    }

    let rust_source = rust_source.ok_or_else(|| {
        "missing --emit-rust-cpp-header <source>; expected a Rust source file".to_string()
    })?;
    let output = output.ok_or_else(|| {
        "missing output path; use -o <header.hpp> with --emit-rust-cpp-header".to_string()
    })?;
    Ok(Some(EmitRustCppHeaderInvocation {
        rust_source,
        output,
    }))
}

fn strip_inline_comment(line: &str) -> &str {
    if let Some(idx) = line.find("//") {
        &line[..idx]
    } else {
        line
    }
}

fn canonical_rust_ident(name: &str) -> String {
    name.trim().trim_start_matches("r#").to_string()
}

fn split_top_level_comma_list(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_angle = 0usize;
    let mut depth_bracket = 0usize;
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '<' => depth_angle += 1,
            '>' => depth_angle = depth_angle.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            ',' if depth_paren == 0 && depth_angle == 0 && depth_bracket == 0 => {
                out.push(input[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

fn find_matching_close_paren(input: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in input[open_idx..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open_idx + idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_rust_exported_function_signature(signature: &str) -> Result<RustFnExport, String> {
    let signature = signature.split('{').next().unwrap_or(signature).trim();
    let signature = signature
        .strip_prefix("pub ")
        .unwrap_or(signature)
        .trim_start();
    let signature = signature
        .strip_prefix("unsafe ")
        .unwrap_or(signature)
        .trim_start();
    let signature = signature
        .strip_prefix("fn ")
        .ok_or_else(|| format!("invalid Rust function signature: `{}`", signature))?;
    let open_idx = signature.find('(').ok_or_else(|| {
        format!(
            "missing parameter list in Rust function signature: `{}`",
            signature
        )
    })?;
    let close_idx = find_matching_close_paren(signature, open_idx).ok_or_else(|| {
        format!(
            "unbalanced parameter list in Rust function signature: `{}`",
            signature
        )
    })?;
    let name = canonical_rust_ident(signature[..open_idx].trim());
    if name.is_empty() {
        return Err(format!(
            "missing function name in Rust function signature: `{}`",
            signature
        ));
    }
    let params_src = &signature[open_idx + 1..close_idx];
    let mut params: Vec<(String, String)> = Vec::new();
    for param in split_top_level_comma_list(params_src) {
        if param.is_empty() || param == "self" || param == "&self" || param == "&mut self" {
            continue;
        }
        let Some((lhs, rhs)) = param.split_once(':') else {
            continue;
        };
        let mut param_name = lhs.trim();
        if let Some(stripped) = param_name.strip_prefix("mut ") {
            param_name = stripped.trim();
        }
        if let Some(last) = param_name.split_whitespace().last() {
            param_name = last.trim();
        }
        let param_name = canonical_rust_ident(param_name);
        if param_name.is_empty() {
            continue;
        }
        let param_ty = rhs.trim().to_string();
        if param_ty.is_empty() {
            continue;
        }
        params.push((param_name, param_ty));
    }
    let mut ret: Option<String> = None;
    let tail = signature[close_idx + 1..].trim();
    if let Some(after_arrow) = tail.strip_prefix("->") {
        let ret_raw = after_arrow.trim();
        let ret_clean = ret_raw
            .split(" where ")
            .next()
            .unwrap_or(ret_raw)
            .trim()
            .to_string();
        if !ret_clean.is_empty() {
            ret = Some(ret_clean);
        }
    }
    Ok(RustFnExport { name, params, ret })
}

fn parse_rust_struct_name_from_line(trimmed_line: &str) -> Option<String> {
    if !trimmed_line.starts_with("pub struct ") || !trimmed_line.ends_with('{') {
        return None;
    }
    let body = trimmed_line["pub struct ".len()..trimmed_line.len() - 1].trim();
    if body.is_empty() || body.contains('<') {
        return None;
    }
    let name: String = body
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '#')
        .collect();
    let name = canonical_rust_ident(&name);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn parse_rust_struct_field_line(trimmed_line: &str) -> Option<(String, String)> {
    let line = trimmed_line.trim_end_matches(',').trim();
    let (lhs, rhs) = line.split_once(':')?;
    let rhs = rhs.trim();
    if rhs.is_empty() {
        return None;
    }
    let mut lhs = lhs.trim();
    if let Some(stripped) = lhs.strip_prefix("pub ") {
        lhs = stripped.trim();
    } else if lhs.starts_with("pub(") {
        if let Some(idx) = lhs.find(')') {
            lhs = lhs[idx + 1..].trim();
        }
    }
    let field_name = canonical_rust_ident(lhs);
    if field_name.is_empty() {
        return None;
    }
    Some((field_name, rhs.to_string()))
}

fn parse_rust_source_for_cpp_header(
    source: &str,
) -> Result<(Vec<RustStructDef>, Vec<RustFnExport>), String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut structs: Vec<RustStructDef> = Vec::new();
    let mut exports: Vec<RustFnExport> = Vec::new();
    let mut in_struct: Option<RustStructDef> = None;
    let mut pending_no_mangle = false;
    let mut i = 0usize;

    while i < lines.len() {
        let line = strip_inline_comment(lines[i]).trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        if let Some(current) = in_struct.as_mut() {
            if line.starts_with('}') {
                structs.push(in_struct.take().unwrap());
                pending_no_mangle = false;
                i += 1;
                continue;
            }
            if let Some(field) = parse_rust_struct_field_line(line) {
                current.fields.push(field);
            }
            i += 1;
            continue;
        }

        if line == "#[no_mangle]" {
            pending_no_mangle = true;
            i += 1;
            continue;
        }

        if let Some(struct_name) = parse_rust_struct_name_from_line(line) {
            in_struct = Some(RustStructDef {
                name: struct_name,
                fields: Vec::new(),
            });
            pending_no_mangle = false;
            i += 1;
            continue;
        }

        if pending_no_mangle && (line.starts_with("pub fn ") || line.starts_with("pub unsafe fn "))
        {
            let mut signature = line.to_string();
            while !signature.contains('{') && i + 1 < lines.len() {
                i += 1;
                let continuation = strip_inline_comment(lines[i]).trim();
                if continuation.is_empty() {
                    continue;
                }
                signature.push(' ');
                signature.push_str(continuation);
            }
            exports.push(parse_rust_exported_function_signature(&signature)?);
            pending_no_mangle = false;
            i += 1;
            continue;
        }

        if !line.starts_with("#[") {
            pending_no_mangle = false;
        }
        i += 1;
    }

    if let Some(current) = in_struct {
        structs.push(current);
    }

    Ok((structs, exports))
}

fn rust_type_to_cpp_type(raw_ty: &str) -> String {
    let ty = raw_ty.trim();

    if let Some(inner) = ty.strip_prefix("*mut ") {
        return format!("{}*", rust_type_to_cpp_type(inner));
    }
    if let Some(inner) = ty.strip_prefix("*const ") {
        return format!("const {}*", rust_type_to_cpp_type(inner));
    }
    if let Some(inner) = ty.strip_prefix("&mut ") {
        return format!("{}*", rust_type_to_cpp_type(inner));
    }
    if let Some(inner) = ty.strip_prefix('&') {
        return format!("const {}*", rust_type_to_cpp_type(inner));
    }

    match ty {
        "i8" => "std::int8_t".to_string(),
        "u8" => "std::uint8_t".to_string(),
        "i16" => "std::int16_t".to_string(),
        "u16" => "std::uint16_t".to_string(),
        "i32" => "std::int32_t".to_string(),
        "u32" => "std::uint32_t".to_string(),
        "i64" => "std::int64_t".to_string(),
        "u64" => "std::uint64_t".to_string(),
        "isize" => "std::ptrdiff_t".to_string(),
        "usize" => "std::size_t".to_string(),
        "f32" => "float".to_string(),
        "f64" => "double".to_string(),
        "bool" => "bool".to_string(),
        "()" => "void".to_string(),
        other => canonical_rust_ident(other),
    }
}

fn render_rust_cpp_header(
    source: &Path,
    structs: &[RustStructDef],
    exports: &[RustFnExport],
) -> String {
    let mut out = String::new();
    out.push_str("#pragma once\n\n");
    out.push_str("// Auto-generated by fragilec.\n");
    out.push_str(&format!("// Source: {}\n", source.display()));
    out.push_str("// Do not edit manually.\n\n");
    out.push_str("#include <cstddef>\n");
    out.push_str("#include <cstdint>\n\n");

    for st in structs {
        out.push_str(&format!("struct {} {{\n", st.name));
        for (field_name, field_ty) in &st.fields {
            out.push_str(&format!(
                "  {} {};\n",
                rust_type_to_cpp_type(field_ty),
                canonical_rust_ident(field_name)
            ));
        }
        out.push_str("};\n\n");
    }

    for exported_fn in exports {
        let ret = exported_fn
            .ret
            .as_deref()
            .map(rust_type_to_cpp_type)
            .unwrap_or_else(|| "void".to_string());
        let params = exported_fn
            .params
            .iter()
            .map(|(name, ty)| {
                format!(
                    "{} {}",
                    rust_type_to_cpp_type(ty),
                    canonical_rust_ident(name)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "extern {} {}({}) asm(\"{}\");\n",
            ret, exported_fn.name, params, exported_fn.name
        ));
    }
    if !exports.is_empty() {
        out.push('\n');
    }

    out
}

fn run_emit_rust_cpp_header(invocation: &EmitRustCppHeaderInvocation) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let source = resolve_path(&invocation.rust_source, &cwd);
    if !source.exists() {
        return Err(format!("Rust source does not exist: {}", source.display()));
    }
    let output = resolve_path(&invocation.output, &cwd);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    let content = fs::read_to_string(&source)
        .map_err(|e| format!("failed to read Rust source {}: {}", source.display(), e))?;
    let (structs, exports) = parse_rust_source_for_cpp_header(&content)?;
    let header = render_rust_cpp_header(&source, &structs, &exports);
    fs::write(&output, header)
        .map_err(|e| format!("failed to write header {}: {}", output.display(), e))
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

fn filter_transpile_frontend_args(args: &[String]) -> Vec<String> {
    // Parser-only normalization: avoid forcing a specific C++ stdlib header set.
    // In this workspace, forwarding `-stdlib=...` to libtooling can explode
    // header load (notably libc++) and trigger extreme memory/runtime spikes.
    let mut filtered = Vec::with_capacity(args.len());
    let mut i = 0usize;
    while i < args.len() {
        let token = args[i].as_str();
        if token == "-stdlib" {
            i += 1;
            if i < args.len() {
                i += 1;
            }
            continue;
        }
        if token.starts_with("-stdlib=") {
            i += 1;
            continue;
        }
        filtered.push(args[i].clone());
        i += 1;
    }
    filtered
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
        None => Ok(StrictParserBackend::Libtooling),
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
) -> Result<(), String> {
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
        Ok(())
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
    if !env_flag_is_true(FRAGILEC_ENFORCE_BUILD_ID_ENV) {
        return Ok(());
    }

    let required_build_id = build_id();
    let require_meta = env_flag_is_true(FRAGILEC_REQUIRE_META_ENV);

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

fn safe_source_filename_for_temp(source: &Path) -> String {
    source
        .file_name()
        .and_then(|name| name.to_str())
        .map(|raw| {
            raw.chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "source".to_string())
}

fn deterministic_transpiled_rs_path(source: &Path, out_obj: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    source.display().to_string().hash(&mut hasher);
    out_obj.display().to_string().hash(&mut hasher);
    let key = hasher.finish();
    let filename = safe_source_filename_for_temp(source);
    std::env::temp_dir()
        .join("fragilec_transpiled")
        .join(format!(
            "{}_{:016x}_{}.rs",
            filename,
            key,
            crate_name_for_source(source)
        ))
}

fn runtime_link_cache_root() -> PathBuf {
    if let Some(path) = normalized_nonempty(
        std::env::var(FRAGILEC_RUNTIME_LINK_CACHE_DIR_ENV)
            .ok()
            .as_deref(),
    ) {
        return PathBuf::from(path);
    }
    std::env::temp_dir().join("fragilec_runtime_link_cache")
}

fn runtime_link_cache_paths() -> (PathBuf, PathBuf) {
    let fingerprint = rustc_invocation_fingerprint();
    let mut hasher = DefaultHasher::new();
    fingerprint.hash(&mut hasher);
    let key = format!("{:016x}", hasher.finish());
    let cache_dir = runtime_link_cache_root().join(key);
    (
        cache_dir.join("libfragile_runtime_support.a"),
        cache_dir.join("native_static_libs.txt"),
    )
}

fn parse_native_static_libs_from_stream(stream: &[u8], out: &mut Vec<OsString>) {
    let text = String::from_utf8_lossy(stream);
    for line in text.lines() {
        if let Some(rest) = line.split("native-static-libs:").nth(1) {
            for token in rest.split_whitespace() {
                out.push(OsString::from(token));
            }
        }
    }
}

fn parse_native_static_libs_from_output(output: &std::process::Output) -> Vec<OsString> {
    let mut native_libs = Vec::new();
    parse_native_static_libs_from_stream(&output.stdout, &mut native_libs);
    parse_native_static_libs_from_stream(&output.stderr, &mut native_libs);
    native_libs
}

fn read_native_static_libs_file(path: &Path) -> Result<Vec<OsString>, String> {
    let text = fs::read_to_string(path).map_err(|e| {
        format!(
            "failed to read native static libs cache {}: {}",
            path.display(),
            e
        )
    })?;
    let mut out = Vec::new();
    for line in text.lines() {
        let token = line.trim();
        if token.is_empty() {
            continue;
        }
        out.push(OsString::from(token));
    }
    Ok(out)
}

fn write_native_static_libs_file(path: &Path, libs: &[OsString]) -> Result<(), String> {
    let mut serialized = String::new();
    for lib in libs {
        serialized.push_str(lib.to_string_lossy().as_ref());
        serialized.push('\n');
    }
    fs::write(path, serialized).map_err(|e| {
        format!(
            "failed to write native static libs cache {}: {}",
            path.display(),
            e
        )
    })
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
        &parser_backend,
    )
}

#[allow(dead_code)]
fn strict_compile_source_to_object_with_backend(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[IncludeDirective],
    defines: &[String],
    args_for_meta: &[OsString],
    parser_backend: ClangParserBackend,
) -> Result<(), String> {
    let parser_backend = strict_parser_backend_from_legacy_backend(parser_backend)?;
    strict_compile_source_to_object_with_frontend_args_and_backend(
        source_arg,
        out_obj,
        includes,
        defines,
        &[],
        args_for_meta,
        &parser_backend,
    )
}

fn strict_compile_source_to_object_with_frontend_args_and_backend(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[IncludeDirective],
    defines: &[String],
    frontend_args: &[String],
    args_for_meta: &[OsString],
    parser_backend: &StrictParserBackend,
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
    if let StrictParserBackend::ParserCore { backend_id } = parser_backend {
        run_parser_core_backend_parse(
            &source,
            language,
            includes,
            defines,
            frontend_args,
            backend_id.as_str(),
            &cwd,
        )?;
        return Err(format!(
            "parser backend `{}` is wired through fragile-parser-core, but transpile codegen cutover is not implemented yet; use FRAGILEC_PARSER_BACKEND=libtooling",
            backend_id
        ));
    }
    let keep_rs = env_flag_is_true(FRAGILEC_KEEP_RS_ENV);
    let transpiled_rs = if keep_rs {
        out_obj.with_extension("fragile.rs")
    } else {
        deterministic_transpiled_rs_path(&source, out_obj)
    };
    if let Some(parent) = transpiled_rs.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create transpiled source directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let compile_once = |skip_system_headers: bool,
                        template_parsing_mode: TemplateParsingMode|
     -> Result<(), String> {
        let transpile_frontend_args = filter_transpile_frontend_args(frontend_args);
        let transpile_options = TranspileOptions {
            include_paths: Vec::new(),
            include_directives: includes.to_vec(),
            frontend_args: transpile_frontend_args,
            defines: defines.to_vec(),
            language: language.clone(),
            language_standard: language_standard.clone(),
            ignored_error_patterns: strict_parser_ignored_error_patterns(language.clone()),
            backend: ClangParserBackend::Libtooling,
            template_parsing_mode,
            libtooling_skip_system_headers: skip_system_headers,
            stage_timing_trace_path: stage_timing_trace_path.clone(),
        };
        let transpiled =
            fragile_clang::transpile_cpp_to_rust_with_options(&source, &transpile_options)
                .map_err(|e| {
                    format!(
                        "failed to transpile {} with parser backend {} (skip_system_headers={}, template_parsing_mode={:?}): {}",
                        source.display(),
                        strict_parser_backend_label(parser_backend),
                        skip_system_headers,
                        template_parsing_mode,
                        e
                    )
                })?;
        let transpiled = normalize_transpiled_main_entry(transpiled);
        enforce_unresolved_type_invariant(&source, &transpiled)?;

        fs::write(&transpiled_rs, transpiled).map_err(|e| {
            format!(
                "failed to write transpiled source {}: {}",
                transpiled_rs.display(),
                e
            )
        })?;

        let mut rustc_cmd = rustc_command();
        rustc_cmd
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
            .arg(out_obj);
        let rustc = rustc_cmd
            .output()
            .map_err(|e| format!("failed to run rustc for {}: {}", source.display(), e))?;

        if !rustc.status.success() {
            return Err(format!(
                "fragile rustc object compile failed for {} (skip_system_headers={}, template_parsing_mode={:?})\nstdout:\n{}\nstderr:\n{}",
                source.display(),
                skip_system_headers,
                template_parsing_mode,
                String::from_utf8_lossy(&rustc.stdout),
                String::from_utf8_lossy(&rustc.stderr)
            ));
        }
        Ok(())
    };

    let preferred_skip = skip_system_headers_from_env();
    match compile_once(preferred_skip, TemplateParsingMode::Auto) {
        Ok(()) => {}
        Err(primary_err) => {
            if !preferred_skip {
                return Err(primary_err);
            }
            if should_native_fallback_on_failure(&primary_err) {
                eprintln!(
                    "[fragilec] using native fallback compiler for {} after strict compile failure",
                    source.display()
                );
                compile_with_native_fallback(args_for_meta, source.as_path())?;
                if !keep_rs {
                    let _ = fs::remove_file(&transpiled_rs);
                }
                write_meta_file(&source, out_obj, args_for_meta)?;
                return Ok(());
            }
            if !should_retry_with_system_headers_on_failure(&primary_err) {
                return Err(primary_err);
            }
            eprintln!(
                "[fragilec] retrying {} with system headers enabled after strict compile failure",
                source.display()
            );
            if let Err(retry_err) = compile_once(false, TemplateParsingMode::Delayed) {
                return Err(format!(
                    "{primary_err}\n\n[fragilec] retry with skip_system_headers=false also failed:\n{retry_err}"
                ));
            }
        }
    }

    if !keep_rs {
        let _ = fs::remove_file(&transpiled_rs);
    }

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
    ) || name.starts_with("__atomic_base_")
        || name.starts_with("__pthread_")
        || name.starts_with("reverse_iterator_")
        || (name.contains("iterator") && name.ends_with("_value_type"))
}

fn maybe_dump_unresolved_transpiled_rs(source: &Path, transpiled: &str) -> Option<PathBuf> {
    if !env_flag_is_true(FRAGILEC_DUMP_UNRESOLVED_RS_ENV) {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    source.display().to_string().hash(&mut hasher);
    let hash = hasher.finish();
    let file_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unit");
    let out_path = std::env::temp_dir()
        .join("fragilec_unresolved")
        .join(format!("{}_{}.rs", file_stem, hash));
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&out_path, transpiled).is_ok() {
        Some(out_path)
    } else {
        None
    }
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
    let dump_note = maybe_dump_unresolved_transpiled_rs(source, transpiled)
        .map(|path| format!(" [dumped transpiled Rust to {}]", path.display()))
        .unwrap_or_default();
    Err(format!(
        "fragile unresolved-type invariant failed for {}: {}{}",
        source.display(),
        detail,
        dump_note
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
            &parser_backend,
        )?;
    }

    Ok(())
}

fn link_driver_command_from_value(linker_env: Option<&str>) -> (String, Vec<OsString>) {
    if let Some(raw) = linker_env {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), Vec::new());
        }
    }

    ("clang++".to_string(), Vec::new())
}

fn link_driver_command() -> (String, Vec<OsString>) {
    let linker_env = std::env::var(FRAGILEC_LINKER_ENV).ok();
    link_driver_command_from_value(linker_env.as_deref())
}

fn build_rust_runtime_link_support(temp_root: &Path) -> Result<(PathBuf, Vec<OsString>), String> {
    let (cached_archive, cached_libs_file) = runtime_link_cache_paths();
    if cached_archive.is_file() && cached_libs_file.is_file() {
        let cached_libs = read_native_static_libs_file(&cached_libs_file)?;
        return Ok((cached_archive, cached_libs));
    }

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

    let mut rustc_cmd = rustc_command();
    rustc_cmd
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
        .arg(&runtime_archive);
    let output = rustc_cmd
        .output()
        .map_err(|e| format!("failed to build rust runtime support archive: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "rust runtime support build failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let native_libs = parse_native_static_libs_from_output(&output);

    if let Some(cache_dir) = cached_archive.parent() {
        if fs::create_dir_all(cache_dir).is_ok() {
            if !cached_archive.exists() {
                let _ = fs::copy(&runtime_archive, &cached_archive);
            }
            if !cached_libs_file.exists() {
                let _ = write_native_static_libs_file(&cached_libs_file, &native_libs);
            }
        }
    }

    if cached_archive.is_file() && cached_libs_file.is_file() {
        let cached_libs = read_native_static_libs_file(&cached_libs_file)?;
        return Ok((cached_archive, cached_libs));
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

fn run_fragile_link(parsed: &ParsedInvocation) -> Result<(), String> {
    if parsed.compile_only {
        return Err("internal error: run_fragile_link called for compile-only command".to_string());
    }

    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let keep_rs = env_flag_is_true(FRAGILEC_KEEP_RS_ENV);
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
            &parser_backend,
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

    // Do not reject early when direct object inputs lack `main`: linked archives/shared
    // libraries (for example gtest_main) can legally provide the program entrypoint.
    // Let the platform linker perform the full symbol resolution decision.

    let (runtime_archive, native_libs) = build_rust_runtime_link_support(&temp_root)?;
    link_args.push(OsString::from(
        runtime_archive.to_string_lossy().to_string(),
    ));
    link_args.extend(native_libs);

    let (driver, driver_args) = link_driver_command();
    let link_output = Command::new(&driver)
        .args(&driver_args)
        .args(&link_args)
        .output()
        .map_err(|e| format!("failed to run strict link driver `{}`: {}", driver, e))?;
    if !link_output.status.success() {
        let link_stdout = String::from_utf8_lossy(&link_output.stdout).to_string();
        let link_stderr = String::from_utf8_lossy(&link_output.stderr).to_string();
        let stderr_lower = link_stderr.to_ascii_lowercase();
        let missing_main_after_resolution = defining_objects.is_empty()
            && link_requires_program_main(parsed)
            && (stderr_lower.contains("undefined reference to `main`")
                || stderr_lower.contains("undefined symbol: _main")
                || stderr_lower.contains("undefined symbol main")
                || (stderr_lower.contains("symbol(s) not found")
                    && stderr_lower.contains("_main")));
        let maybe_main_hint = if missing_main_after_resolution {
            format!(
                "\nstrict link missing `main` (after full linker resolution)\n{}",
                main_symbol_diag
            )
        } else {
            String::new()
        };
        return Err(format!(
            "strict link failed via `{}`\nstdout:\n{}\nstderr:\n{}\n{}{}",
            driver, link_stdout, link_stderr, main_symbol_diag, maybe_main_hint
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
  fragilec --emit-rust-cpp-header <source.rs> -o <generated.hpp>

Compile flag:
  (none; fragilec is strict-only)

Environment:
  FRAGILEC_MODE=strict               Optional; strict-only mode (default: strict)
  FRAGILEC_PARSER_BACKEND=<name>     Parser backend: libtooling or fragile-parser-clang
  FRAGILEC_PARSER_CORE_MANIFEST_DIR=<path>
                                     Optional parser-core parse summary output directory
  FRAGILEC_SKIP_SYSTEM_HEADERS=<0|1> Skip system/header-unit AST export (default: disabled)
  FRAGILEC_LOG=<path>                Append invocation log (cwd/args records)
  FRAGILEC_BUILD_ID=<id>             Build-id used for metadata writes/checks
  FRAGILEC_ENFORCE_BUILD_ID=1        Enforce build-id on .o/.a inputs during link
  FRAGILEC_REQUIRE_META=1            Require metadata sidecars for link inputs
  FRAGILEC_KEEP_RS=1                 Keep transpiled Rust sidecar next to output object
  FRAGILEC_RUSTC_BIN=<path>          rustc binary for strict compile/link helper steps
  FRAGILEC_RUSTC_WRAPPER=<path>      rustc wrapper (for example sccache); falls back to RUSTC_WRAPPER
  FRAGILEC_RUNTIME_LINK_CACHE_DIR=<path>
                                     Cache dir for runtime link-support archive/native-static-libs
  FRAGILEC_TRANSPILE_STAGE_TIMING_PATH=<path>
                                     Write transpile stage timing trace (parse/export/enrichment/codegen)
  FRAGILEC_LINKER=<path>             Link-driver executable for strict link (default: clang++)
"
    );
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.iter().any(|a| a == "--fragilec-help") || (args.len() == 1 && args[0] == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    match parse_emit_rust_cpp_header_invocation(&args) {
        Ok(Some(invocation)) => match run_emit_rust_cpp_header(&invocation) {
            Ok(()) => return ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("[fragilec] {}", err);
                return ExitCode::from(1);
            }
        },
        Ok(None) => {}
        Err(err) => {
            eprintln!("[fragilec] {}", err);
            return ExitCode::from(2);
        }
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
    use fragile_parser_core::ParserLanguage as CoreParserLanguage;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(|s| OsString::from(*s)).collect()
    }

    #[test]
    fn parse_emit_rust_cpp_header_invocation_supports_split_and_equals_forms() {
        let split = parse_emit_rust_cpp_header_invocation(&args(&[
            "--emit-rust-cpp-header",
            "src/math.rs",
            "-o",
            "build/rust_math.hpp",
        ]))
        .expect("split-form invocation should parse")
        .expect("mode should be detected");
        assert_eq!(split.rust_source, PathBuf::from("src/math.rs"));
        assert_eq!(split.output, PathBuf::from("build/rust_math.hpp"));

        let equals = parse_emit_rust_cpp_header_invocation(&args(&[
            "--emit-rust-cpp-header=src/math.rs",
            "--output=build/rust_math.hpp",
        ]))
        .expect("equals-form invocation should parse")
        .expect("mode should be detected");
        assert_eq!(equals.rust_source, PathBuf::from("src/math.rs"));
        assert_eq!(equals.output, PathBuf::from("build/rust_math.hpp"));
    }

    #[test]
    fn parse_rust_source_for_cpp_header_extracts_struct_and_exported_functions() {
        let source = r#"
pub struct RustAccumulator {
    total: i64,
    scale: i64,
}

#[no_mangle]
pub fn rust_accumulator_init(ptr: *mut RustAccumulator, seed: i64, scale: i64) -> bool {
    true
}

#[no_mangle]
pub fn rust_accumulator_drop(ptr: *mut RustAccumulator) {
}
"#;
        let (structs, exports) =
            parse_rust_source_for_cpp_header(source).expect("parse should succeed");
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "RustAccumulator");
        assert_eq!(
            structs[0].fields,
            vec![
                ("total".to_string(), "i64".to_string()),
                ("scale".to_string(), "i64".to_string())
            ]
        );
        assert_eq!(exports.len(), 2);
        assert_eq!(exports[0].name, "rust_accumulator_init");
        assert_eq!(
            exports[0].params,
            vec![
                ("ptr".to_string(), "*mut RustAccumulator".to_string()),
                ("seed".to_string(), "i64".to_string()),
                ("scale".to_string(), "i64".to_string()),
            ]
        );
        assert_eq!(exports[0].ret.as_deref(), Some("bool"));
        assert_eq!(exports[1].name, "rust_accumulator_drop");
        assert_eq!(exports[1].ret, None);
    }

    #[test]
    fn render_rust_cpp_header_maps_types_and_exports_symbols() {
        let structs = vec![RustStructDef {
            name: "RustAccumulator".to_string(),
            fields: vec![
                ("total".to_string(), "i64".to_string()),
                ("scale".to_string(), "i64".to_string()),
            ],
        }];
        let exports = vec![
            RustFnExport {
                name: "rust_accumulator_init".to_string(),
                params: vec![
                    ("ptr".to_string(), "*mut RustAccumulator".to_string()),
                    ("seed".to_string(), "i64".to_string()),
                    ("scale".to_string(), "i64".to_string()),
                ],
                ret: Some("bool".to_string()),
            },
            RustFnExport {
                name: "rust_accumulator_drop".to_string(),
                params: vec![("ptr".to_string(), "*mut RustAccumulator".to_string())],
                ret: None,
            },
        ];
        let header = render_rust_cpp_header(Path::new("src/math.rs"), &structs, &exports);
        assert!(
            header.contains("struct RustAccumulator"),
            "rendered header should include struct declaration, got:\n{}",
            header
        );
        assert!(
            header.contains("std::int64_t total;") && header.contains("std::int64_t scale;"),
            "rendered header should map i64 fields to std::int64_t, got:\n{}",
            header
        );
        assert!(
            header.contains(
                "extern bool rust_accumulator_init(RustAccumulator* ptr, std::int64_t seed, std::int64_t scale) asm(\"rust_accumulator_init\");"
            ),
            "rendered header should include exported init declaration with asm symbol binding, got:\n{}",
            header
        );
        assert!(
            header.contains(
                "extern void rust_accumulator_drop(RustAccumulator* ptr) asm(\"rust_accumulator_drop\");"
            ),
            "rendered header should include exported void declaration, got:\n{}",
            header
        );
    }

    #[test]
    fn link_driver_command_defaults_to_clang() {
        let (driver, driver_args) = link_driver_command_from_value(None);
        assert_eq!(driver, "clang++");
        assert!(driver_args.is_empty());

        let (driver, driver_args) = link_driver_command_from_value(Some("   "));
        assert_eq!(driver, "clang++");
        assert!(driver_args.is_empty());
    }

    #[test]
    fn link_driver_command_honors_env_override_without_default_flags() {
        let (driver, driver_args) =
            link_driver_command_from_value(Some("/opt/toolchains/custom++"));
        assert_eq!(driver, "/opt/toolchains/custom++");
        assert!(
            driver_args.is_empty(),
            "custom linker override should not get implicit default args"
        );
    }

    #[test]
    fn rustc_bin_defaults_and_override() {
        assert_eq!(rustc_bin_from_value(None), "rustc");
        assert_eq!(rustc_bin_from_value(Some("  ")), "rustc");
        assert_eq!(
            rustc_bin_from_value(Some("/opt/toolchains/rustc-custom")),
            "/opt/toolchains/rustc-custom"
        );
    }

    #[test]
    fn rustc_wrapper_parsing_trims_and_handles_empty() {
        assert_eq!(rustc_wrapper_from_value(None), None);
        assert_eq!(rustc_wrapper_from_value(Some("  ")), None);
        assert_eq!(
            rustc_wrapper_from_value(Some(" /usr/bin/sccache ")),
            Some("/usr/bin/sccache".to_string())
        );
    }

    #[test]
    fn deterministic_transpiled_path_is_stable_for_same_inputs() {
        let source = Path::new("/tmp/demo.cc");
        let out_obj = Path::new("/tmp/build/demo.o");
        let first = deterministic_transpiled_rs_path(source, out_obj);
        let second = deterministic_transpiled_rs_path(source, out_obj);
        assert_eq!(first, second);
        let path = first.to_string_lossy();
        assert!(path.ends_with(".rs"));
        assert!(
            path.contains("demo.cc_"),
            "deterministic path should include source filename for readability: {}",
            path
        );
    }

    #[test]
    fn parse_native_static_libs_extracts_from_both_streams() {
        let mut libs = Vec::new();
        parse_native_static_libs_from_stream(
            b"note: native-static-libs: -lgcc_s -lutil",
            &mut libs,
        );
        parse_native_static_libs_from_stream(
            b"warning: native-static-libs: -lrt -lpthread",
            &mut libs,
        );
        assert_eq!(
            libs,
            vec![
                OsString::from("-lgcc_s"),
                OsString::from("-lutil"),
                OsString::from("-lrt"),
                OsString::from("-lpthread"),
            ]
        );
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
        let cpp = strict_parser_ignored_error_patterns(ClangParserLanguage::Cpp);
        assert!(
            cpp.is_empty(),
            "cpp strict parser ignore list should be empty"
        );

        let c = strict_parser_ignored_error_patterns(ClangParserLanguage::C);
        assert!(c.is_empty(), "c strict parser ignore list should be empty");
    }

    #[test]
    fn strict_parser_backend_validation_accepts_supported_values() {
        assert_eq!(
            strict_parser_backend_from_legacy_backend(ClangParserBackend::Libtooling)
                .expect("legacy libtooling alias should map"),
            StrictParserBackend::Libtooling
        );
        strict_parser_backend_from_legacy_backend(ClangParserBackend::Libclang)
            .expect_err("legacy libclang alias should be rejected");
        assert_eq!(
            parse_parser_backend_value("LIBTOOLING").expect("libtooling backend should parse"),
            StrictParserBackend::Libtooling
        );
        assert_eq!(
            strict_parser_backend_from_value(None).expect("missing backend should default"),
            StrictParserBackend::Libtooling
        );
        assert_eq!(
            strict_parser_backend_from_value(Some("")).expect("empty backend should default"),
            StrictParserBackend::Libtooling
        );
        assert_eq!(
            strict_parser_backend_from_value(Some(" fragile-parser-clang "))
                .expect("parser-core backend should parse"),
            StrictParserBackend::ParserCore {
                backend_id: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string()
            }
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
            err.contains("libtooling") && err.contains(FRAGILE_PARSER_CLANG_BACKEND_ID),
            "error should list supported backend values, got: {}",
            err
        );
    }

    #[test]
    fn strict_compile_parser_core_backend_reports_cutover_boundary() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragilec_parser_core_backend_{}", stamp));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("program.c");
        let out_obj = temp_dir.join("program.o");
        fs::write(&source, "int main(void) { return 0; }\n").expect("failed to write source");

        let err = strict_compile_source_to_object_with_frontend_args_and_backend(
            &source,
            &out_obj,
            &[],
            &[],
            &[],
            &[],
            &StrictParserBackend::ParserCore {
                backend_id: FRAGILE_PARSER_CLANG_BACKEND_ID.to_string(),
            },
        )
        .expect_err("parser-core backend should currently stop at codegen cutover boundary");
        assert!(
            err.contains("fragile-parser-core")
                && err.contains("codegen cutover is not implemented yet")
                && err.contains(FRAGILE_PARSER_CLANG_BACKEND_ID),
            "unexpected cutover error: {}",
            err
        );
        assert!(
            !out_obj.exists(),
            "object output should not be produced on parser-core cutover boundary"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn parser_core_manifest_writer_emits_deterministic_summary() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragilec_parser_manifest_{}", stamp));
        let manifest_dir = temp_dir.join("manifests");
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("unit.c");
        fs::write(
            &source,
            "int mul(int a, int b) { return a * b; }\nint main(void) { return mul(2, 3); }\n",
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
    fn extract_language_standard_supports_split_and_equals_forms() {
        let split_form = extract_language_standard(
            &args(&["-O2", "-std", "c++11", "-c", "unit.cpp"]),
            ClangParserLanguage::Cpp,
        );
        assert_eq!(split_form.as_deref(), Some("c++11"));

        let equals_form = extract_language_standard(
            &args(&["-Wall", "-std=gnu++17", "-c", "unit.cpp"]),
            ClangParserLanguage::Cpp,
        );
        assert_eq!(equals_form.as_deref(), Some("gnu++17"));
    }

    #[test]
    fn extract_language_standard_prefers_last_matching_flag() {
        let detected = extract_language_standard(
            &args(&["-std=c++11", "-std=gnu++17", "-c", "unit.cpp"]),
            ClangParserLanguage::Cpp,
        );
        assert_eq!(detected.as_deref(), Some("gnu++17"));
    }

    #[test]
    fn extract_language_standard_ignores_mismatched_language_family() {
        let c_from_cpp =
            extract_language_standard(&args(&["-std=c++20", "-c", "unit.c"]), ClangParserLanguage::C);
        assert_eq!(c_from_cpp, None);

        let cpp_from_c =
            extract_language_standard(&args(&["-std=c11", "-c", "unit.cpp"]), ClangParserLanguage::Cpp);
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
            ClangParserBackend::Libtooling,
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
            ClangParserBackend::Libtooling,
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
            ClangParserBackend::Libtooling,
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
            ClangParserBackend::Libtooling,
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
            ClangParserBackend::Libtooling,
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
