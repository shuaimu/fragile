use fragile_clang::{AstCodeGen, ClangParser, ParserLanguage};
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
const FRAGILEC_NATIVE_ENV: &str = "FRAGILEC_NATIVE_COMPILER";
const FRAGILEC_BUILD_ID_ENV: &str = "FRAGILEC_BUILD_ID";
const FRAGILEC_ENFORCE_BUILD_ID_ENV: &str = "FRAGILEC_ENFORCE_BUILD_ID";
const FRAGILEC_REQUIRE_META_ENV: &str = "FRAGILEC_REQUIRE_META";
const FRAGILEC_KEEP_RS_ENV: &str = "FRAGILEC_KEEP_RS";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DriverMode {
    /// Always pass through to the native compiler.
    Pass,
    /// Try fragile compile path first for compile-only single-source invocations,
    /// then fall back to native compiler on failure.
    Auto,
    /// Require fragile compile path for compile-only single-source invocations.
    Strict,
}

impl DriverMode {
    fn from_env() -> Self {
        match std::env::var(FRAGILEC_MODE_ENV)
            .unwrap_or_else(|_| "pass".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "auto" => Self::Auto,
            "strict" => Self::Strict,
            _ => Self::Pass,
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedInvocation {
    args: Vec<OsString>,
    compile_only: bool,
    output: Option<PathBuf>,
    sources: Vec<PathBuf>,
    includes: Vec<String>,
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
                    includes.push(args[i + 1].to_string_lossy().to_string());
                    i += 2;
                    continue;
                }
            }
            if let Some(stripped) = cur.strip_prefix("-I") {
                if !stripped.is_empty() {
                    includes.push(stripped.to_string());
                    i += 1;
                    continue;
                }
            }
            if cur == "-isystem" || cur == "-iquote" {
                if i + 1 < args.len() {
                    includes.push(args[i + 1].to_string_lossy().to_string());
                    i += 2;
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

fn choose_native_compiler(parsed: &ParsedInvocation) -> String {
    if let Ok(native) = std::env::var(FRAGILEC_NATIVE_ENV) {
        if !native.trim().is_empty() {
            return native;
        }
    }

    if !parsed.sources.is_empty() && parsed.sources.iter().all(|s| is_c_file(s)) {
        "cc".to_string()
    } else {
        "c++".to_string()
    }
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
    writeln!(file, "cwd={}", cwd.display())
        .map_err(|e| format!("failed to append cwd record to {}: {}", log_path.display(), e))?;
    write!(file, "args=")
        .map_err(|e| format!("failed to append args prefix to {}: {}", log_path.display(), e))?;
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

fn default_object_output(source_arg: &Path, cwd: &Path) -> Result<PathBuf, String> {
    let stem = source_arg
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("cannot derive object output for source {}", source_arg.display()))?;
    Ok(cwd.join(format!("{stem}.o")))
}

fn source_language(source: &Path) -> ParserLanguage {
    if is_c_file(source) {
        ParserLanguage::C
    } else {
        ParserLanguage::Cpp
    }
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
    } else if out.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
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

fn run_native(compiler: &str, args: &[OsString]) -> Result<i32, String> {
    let status = Command::new(compiler)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run native compiler `{}`: {}", compiler, e))?;
    Ok(status.code().unwrap_or(1))
}

fn run_fragile_compile(parsed: &ParsedInvocation) -> Result<(), String> {
    if !parsed.compile_only {
        return Err("fragile compile mode only supports `-c` invocations for now".to_string());
    }
    if parsed.sources.len() != 1 {
        return Err(format!(
            "fragile compile mode currently requires exactly one source (found {})",
            parsed.sources.len()
        ));
    }

    let cwd = std::env::current_dir().map_err(|e| format!("failed to read cwd: {}", e))?;
    let source_arg = &parsed.sources[0];
    let source = resolve_path(source_arg, &cwd);
    if !source.exists() {
        return Err(format!("source file does not exist: {}", source.display()));
    }

    let out_obj = match &parsed.output {
        Some(out) => resolve_path(out, &cwd),
        None => default_object_output(source_arg, &cwd)?,
    };
    if let Some(parent) = out_obj.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create output directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let parser = ClangParser::with_paths_defines_and_language(
        parsed.includes.clone(),
        parsed.defines.clone(),
        source_language(&source),
    )
    .map_err(|e| format!("failed to create fragile parser for {}: {}", source.display(), e))?;
    let ast = parser
        .parse_file(&source)
        .map_err(|e| format!("failed to parse {}: {}", source.display(), e))?;
    let transpiled = AstCodeGen::new().generate(&ast.translation_unit);

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
        .arg(crate_name_for_source(&source))
        .arg(&transpiled_rs)
        .arg("-o")
        .arg(&out_obj)
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

    write_meta_file(&source, &out_obj, &parsed.args)?;
    Ok(())
}

fn print_help() {
    eprintln!(
        "\
fragilec - Fragile compiler driver shim

Usage:
  fragilec [compiler args...]

Environment:
  FRAGILEC_MODE=pass|auto|strict     Driver mode (default: pass)
  FRAGILEC_NATIVE_COMPILER=<bin>     Native fallback compiler (default: cc/c++)
  FRAGILEC_LOG=<path>                Append invocation log (cwd/args records)
  FRAGILEC_BUILD_ID=<id>             Build-id used for metadata writes/checks
  FRAGILEC_ENFORCE_BUILD_ID=1        Enforce build-id on .o/.a inputs during link
  FRAGILEC_REQUIRE_META=1            Require metadata sidecars for link inputs
  FRAGILEC_KEEP_RS=1                 Keep transpiled Rust sidecar next to output object
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
    let mode = DriverMode::from_env();
    let native = choose_native_compiler(&parsed);

    // Enforce link-input metadata only when we are delegating a link command.
    if !parsed.compile_only {
        if let Err(err) = enforce_build_id_for_link_inputs(&parsed.args) {
            eprintln!("[fragilec] {}", err);
            return ExitCode::from(1);
        }
    }

    let compile_candidate = parsed.compile_only && parsed.sources.len() == 1;

    if !compile_candidate {
        if mode == DriverMode::Strict {
            eprintln!(
                "[fragilec] strict mode currently supports only single-source compile-only (-c) invocations"
            );
            return ExitCode::from(2);
        }
        return match run_native(&native, &parsed.args) {
            Ok(code) => ExitCode::from(code as u8),
            Err(err) => {
                eprintln!("[fragilec] {}", err);
                ExitCode::from(1)
            }
        };
    }

    match mode {
        DriverMode::Pass => match run_native(&native, &parsed.args) {
            Ok(code) => ExitCode::from(code as u8),
            Err(err) => {
                eprintln!("[fragilec] {}", err);
                ExitCode::from(1)
            }
        },
        DriverMode::Strict => match run_fragile_compile(&parsed) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("[fragilec] {}", err);
                ExitCode::from(1)
            }
        },
        DriverMode::Auto => match run_fragile_compile(&parsed) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!(
                    "[fragilec] fragile compile failed; falling back to native compiler `{}`: {}",
                    native, err
                );
                match run_native(&native, &parsed.args) {
                    Ok(code) => ExitCode::from(code as u8),
                    Err(native_err) => {
                        eprintln!("[fragilec] {}", native_err);
                        ExitCode::from(1)
                    }
                }
            }
        },
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
        assert_eq!(parsed.includes, vec!["include".to_string()]);
        assert_eq!(parsed.defines, vec!["FOO=1".to_string()]);
        assert_eq!(parsed.output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn parse_handles_combined_flag_forms() {
        let parsed = ParsedInvocation::parse(args(&["-Iinc", "-DBAR=1", "-c", "unit.c", "-omain.o"]));
        assert_eq!(parsed.includes, vec!["inc".to_string()]);
        assert_eq!(parsed.defines, vec!["BAR=1".to_string()]);
        assert_eq!(parsed.output, Some(PathBuf::from("main.o")));
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
    fn crate_name_sanitizes_non_identifier_chars() {
        assert_eq!(crate_name_for_source(Path::new("hello-world.cpp")), "hello_world");
        assert_eq!(crate_name_for_source(Path::new("1x.c")), "fragile_1x");
    }
}
