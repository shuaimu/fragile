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
const FRAGILEC_BUILD_ID_ENV: &str = "FRAGILEC_BUILD_ID";
const FRAGILEC_ENFORCE_BUILD_ID_ENV: &str = "FRAGILEC_ENFORCE_BUILD_ID";
const FRAGILEC_REQUIRE_META_ENV: &str = "FRAGILEC_REQUIRE_META";
const FRAGILEC_KEEP_RS_ENV: &str = "FRAGILEC_KEEP_RS";
const FRAGILEC_LINKER_ENV: &str = "FRAGILEC_LINKER";

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
    includes: Vec<String>,
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

fn strict_compile_source_to_object(
    source_arg: &Path,
    out_obj: &Path,
    includes: &[String],
    defines: &[String],
    args_for_meta: &[OsString],
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

    let parser = ClangParser::with_paths_defines_and_language(
        includes.to_vec(),
        defines.to_vec(),
        source_language(&source),
    )
    .map_err(|e| {
        format!(
            "failed to create fragile parser for {}: {}",
            source.display(),
            e
        )
    })?;
    let ast = parser
        .parse_file(&source)
        .map_err(|e| format!("failed to parse {}: {}", source.display(), e))?;
    let transpiled =
        normalize_transpiled_main_entry(AstCodeGen::new().generate(&ast.translation_unit));

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
    for source_arg in &parsed.sources {
        let out_obj = if parsed.sources.len() == 1 {
            match &parsed.output {
                Some(out) => resolve_path(out, &cwd),
                None => default_object_output(source_arg, &cwd)?,
            }
        } else {
            default_object_output(source_arg, &cwd)?
        };

        strict_compile_source_to_object(
            source_arg,
            &out_obj,
            &parsed.includes,
            &parsed.defines,
            &parsed.args,
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
    for (idx, source_arg) in parsed.sources.iter().enumerate() {
        let stem = source_arg
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unit");
        let out_obj = temp_root.join(format!("{idx}_{stem}.o"));
        strict_compile_source_to_object(
            source_arg,
            &out_obj,
            &parsed.includes,
            &parsed.defines,
            &parsed.args,
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

    if defining_objects.is_empty() && link_requires_program_main(parsed) {
        return Err(format!(
            "strict link requires a real `main` symbol for executable outputs\n{}",
            main_symbol_diag
        ));
    }

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
        return Err(format!(
            "strict link failed via `{}`\nstdout:\n{}\nstderr:\n{}\n{}",
            driver,
            String::from_utf8_lossy(&link_output.stdout),
            String::from_utf8_lossy(&link_output.stderr),
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
  FRAGILEC_LOG=<path>                Append invocation log (cwd/args records)
  FRAGILEC_BUILD_ID=<id>             Build-id used for metadata writes/checks
  FRAGILEC_ENFORCE_BUILD_ID=1        Enforce build-id on .o/.a inputs during link
  FRAGILEC_REQUIRE_META=1            Require metadata sidecars for link inputs
  FRAGILEC_KEEP_RS=1                 Keep transpiled Rust sidecar next to output object
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
        assert_eq!(parsed.includes, vec!["include".to_string()]);
        assert_eq!(parsed.defines, vec!["FOO=1".to_string()]);
        assert_eq!(parsed.output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn parse_handles_combined_flag_forms() {
        let parsed =
            ParsedInvocation::parse(args(&["-Iinc", "-DBAR=1", "-c", "unit.c", "-omain.o"]));
        assert_eq!(parsed.includes, vec!["inc".to_string()]);
        assert_eq!(parsed.defines, vec!["BAR=1".to_string()]);
        assert_eq!(parsed.output, Some(PathBuf::from("main.o")));
        assert_eq!(parsed.source_indices, vec![3usize]);
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
        fs::write(
            &source,
            "int helper() { return 1; }\nint main() { return helper() - 1; }\n",
        )
        .expect("failed to write source");

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
}
