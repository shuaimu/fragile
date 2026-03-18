//! Clang AST parsing and Rust code generation for the Fragile polyglot compiler.
//!
//! This crate provides:
//! - C++ source parsing via LibTooling AST export
//! - Clang AST traversal and extraction
//! - Direct AST-to-Rust source code generation
//!
//! # Architecture
//!
//! ```text
//! C++ Source → LibTooling AST export → Clang AST model → Rust Source (via AstCodeGen)
//! ```

mod ast;
mod ast_codegen;
mod libtooling;
mod parse;
mod types;

pub use ast::{
    AccessSpecifier, BinaryOp, ClangAst, ClangNode, ClangNodeKind, ConstructorKind, Requirement,
    TemplateSpecializationKind, TypeTraitKind, UnaryOp,
};
pub use ast_codegen::AstCodeGen;
pub use libtooling::{
    convert_to_clang_node, extract_method_bodies, extract_method_bodies_with_params,
    extract_specialization_field_types, extract_specialization_method_signatures, LibToolingParser,
    MethodInfo, MethodSignature, SpecializationFieldInfo, TemplateMethodInstantiation,
};
pub use parse::{ClangParser, ParserLanguage};
pub use types::{CppType, TypeProperties, TypeTraitEvaluator, TypeTraitResult};

use fragile_ast_exporter::{clang_ast::AstContext, clang_ast::SrcSpan, ASTEntryTag};
use fragile_parser_core::{
    IncludeDirective as ParserCoreIncludeDirective,
    ParserLanguage as ParserCoreLanguage,
    ParserOutputV1,
    PARSER_OUTPUT_SCHEMA_VERSION_V1,
};
use fragile_stl::layout_contract::pre_generated_stl_family_contract_entry_v1;
use miette::Result;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Parser backend selection for transpilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserBackend {
    /// Legacy alias retained for compatibility; transpilation uses LibTooling.
    Libclang,
    /// Parse translation unit with LibTooling conversion only.
    Libtooling,
    /// Legacy alias retained for compatibility; transpilation uses LibTooling.
    Hybrid,
}

/// Header search directive kinds supported by transpile options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeDirectiveKind {
    Include,
    System,
    Quote,
}

/// Template parsing strategy used for LibTooling export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateParsingMode {
    /// Try standard parsing first, then retry with delayed template parsing.
    Auto,
    /// Use standard template parsing only.
    Standard,
    /// Force delayed template parsing.
    Delayed,
}

/// Ordered include directive forwarded to the parser frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    pub kind: IncludeDirectiveKind,
    pub path: String,
}

/// Transpile configuration.
#[derive(Debug, Clone)]
pub struct TranspileOptions {
    /// Legacy include-path list interpreted as `-I`.
    /// Prefer `include_directives` when preserving include-kind semantics matters.
    pub include_paths: Vec<String>,
    /// Ordered include directives preserving `-I` / `-isystem` / `-iquote`.
    pub include_directives: Vec<IncludeDirective>,
    /// Ordered frontend flags forwarded verbatim to Clang/LibTooling.
    /// When non-empty, these are used instead of include/directive/define legacy fields.
    pub frontend_args: Vec<String>,
    pub defines: Vec<String>,
    pub language: ParserLanguage,
    pub language_standard: Option<String>,
    pub ignored_error_patterns: Vec<String>,
    pub backend: ParserBackend,
    pub template_parsing_mode: TemplateParsingMode,
    pub libtooling_skip_system_headers: bool,
    pub stage_timing_trace_path: Option<PathBuf>,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            include_paths: Vec::new(),
            include_directives: Vec::new(),
            frontend_args: Vec::new(),
            defines: Vec::new(),
            language: ParserLanguage::Cpp,
            language_standard: None,
            ignored_error_patterns: Vec::new(),
            backend: ParserBackend::Libtooling,
            template_parsing_mode: TemplateParsingMode::Auto,
            libtooling_skip_system_headers: false,
            stage_timing_trace_path: None,
        }
    }
}

/// Parser-output handoff configuration for codegen-only transpile entry points.
#[derive(Debug, Clone, Default)]
pub struct ParserOutputCodegenOptions {
    /// Error patterns to ignore while reparsing source with libclang.
    pub ignored_error_patterns: Vec<String>,
    /// Optional stage timing trace output path.
    pub stage_timing_trace_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TranspileStageTimings {
    pub parse: Duration,
    pub export: Duration,
    pub enrichment: Duration,
    pub codegen: Duration,
}

impl TranspileStageTimings {
    fn total(self) -> Duration {
        self.parse + self.export + self.enrichment + self.codegen
    }
}

const TRANSPILE_STAGE_PARSE: &str = "parse";
const TRANSPILE_STAGE_EXPORT: &str = "export";
const TRANSPILE_STAGE_ENRICHMENT: &str = "enrichment";
const TRANSPILE_STAGE_CODEGEN: &str = "codegen";
const PARSER_OUTPUT_HANDOFF_BACKEND_LABEL: &str = "parser-output-handoff";
const STL_PLACEHOLDER_KIND_FAMILY_MAP: &[(&str, &str)] = &[
    ("stl_vector_placeholder", "vector"),
    ("stl_map_placeholder", "map"),
    ("stl_unordered_map_placeholder", "unordered_map"),
    ("stl_string_placeholder", "string"),
    ("stl_optional_placeholder", "optional"),
    ("stl_variant_placeholder", "variant"),
    ("stl_tuple_placeholder", "tuple"),
    ("stl_shared_ptr_placeholder", "shared_ptr"),
    ("stl_unique_ptr_placeholder", "unique_ptr"),
];

fn parser_backend_label(backend: ParserBackend) -> &'static str {
    match backend {
        ParserBackend::Libclang => "libclang",
        ParserBackend::Libtooling => "libtooling",
        ParserBackend::Hybrid => "hybrid",
    }
}

fn append_stage_trace_line(trace_path: Option<&Path>, line: &str) {
    let Some(path) = trace_path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn sanitize_stage_trace_message(message: &str) -> String {
    message.trim().replace('\r', "").replace('\n', "\\n")
}

fn initialize_stage_trace_with_backend_label(
    trace_path: Option<&Path>,
    source: &Path,
    backend_label: &str,
) {
    let Some(path) = trace_path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut initial = String::new();
    initial.push_str(&format!("source={}\n", source.display()));
    initial.push_str(&format!("backend={backend_label}\n"));
    initial.push_str("status=started\n");
    let _ = fs::write(path, initial);
}

fn initialize_stage_trace(trace_path: Option<&Path>, source: &Path, backend: ParserBackend) {
    initialize_stage_trace_with_backend_label(trace_path, source, parser_backend_label(backend));
}

fn stage_start(trace_path: Option<&Path>, stage: &str) {
    append_stage_trace_line(
        trace_path,
        format!("event=stage_start stage={stage}").as_str(),
    );
}

fn stage_end(
    trace_path: Option<&Path>,
    stage: &str,
    elapsed: Duration,
    status: &str,
    error: Option<&str>,
) {
    let mut line = format!(
        "event=stage_end stage={} status={} elapsed_ms={}",
        stage,
        status,
        elapsed.as_millis()
    );
    if let Some(error_message) = error {
        line.push_str(&format!(
            " error={}",
            sanitize_stage_trace_message(error_message)
        ));
    }
    append_stage_trace_line(trace_path, line.as_str());
}

fn finalize_stage_trace(
    trace_path: Option<&Path>,
    timings: &TranspileStageTimings,
    status: &str,
    error: Option<&str>,
) {
    append_stage_trace_line(
        trace_path,
        format!(
            "summary parse_ms={} export_ms={} enrichment_ms={} codegen_ms={} total_ms={}",
            timings.parse.as_millis(),
            timings.export.as_millis(),
            timings.enrichment.as_millis(),
            timings.codegen.as_millis(),
            timings.total().as_millis()
        )
        .as_str(),
    );
    append_stage_trace_line(trace_path, format!("status={status}").as_str());
    if let Some(error_message) = error {
        append_stage_trace_line(
            trace_path,
            format!(
                "status_error={}",
                sanitize_stage_trace_message(error_message)
            )
            .as_str(),
        );
    }
}

fn trace_stage<F, T>(
    trace_path: Option<&Path>,
    stage: &str,
    slot: &mut Duration,
    action: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    stage_start(trace_path, stage);
    let started = Instant::now();
    let result = action();
    let elapsed = started.elapsed();
    *slot = elapsed;
    if let Err(err) = &result {
        let err_message = err.to_string();
        stage_end(
            trace_path,
            stage,
            elapsed,
            "error",
            Some(err_message.as_str()),
        );
    } else {
        stage_end(trace_path, stage, elapsed, "ok", None);
    }
    result
}

fn path_specific_defines(path: &Path) -> Vec<String> {
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if file_name == "xxh_x86dispatch.c" {
        return vec![
            "XXH_DISPATCH_AVX2=0".to_string(),
            "XXH_DISPATCH_AVX512=0".to_string(),
        ];
    }
    Vec::new()
}

fn merged_defines(path: &Path, defines: &[String]) -> Vec<String> {
    let mut merged = defines.to_vec();
    for define in path_specific_defines(path) {
        if !merged.iter().any(|existing| existing == &define) {
            merged.push(define);
        }
    }
    merged
}

fn frontend_args_contains_define(frontend_args: &[String], define: &str) -> bool {
    let mut i = 0usize;
    while i < frontend_args.len() {
        let arg = frontend_args[i].as_str();
        if arg == "-D" {
            if i + 1 < frontend_args.len() && frontend_args[i + 1] == define {
                return true;
            }
            i += 2;
            continue;
        }
        if let Some(rest) = arg.strip_prefix("-D") {
            if !rest.is_empty() && rest == define {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn frontend_args_has_template_parsing_override(frontend_args: &[String]) -> bool {
    frontend_args
        .iter()
        .any(|arg| arg == "-fdelayed-template-parsing" || arg == "-fno-delayed-template-parsing")
}

fn libtooling_parser_for_path(
    path: &Path,
    options: &TranspileOptions,
    delayed_template_parsing: bool,
) -> LibToolingParser {
    let mut extra_args = Vec::new();
    match options.language {
        ParserLanguage::Cpp => {
            extra_args.push("-x".to_string());
            extra_args.push("c++".to_string());
            let std = options
                .language_standard
                .as_deref()
                .unwrap_or("c++17")
                .to_string();
            extra_args.push(format!("-std={std}"));
        }
        ParserLanguage::C => {
            extra_args.push("-x".to_string());
            extra_args.push("c".to_string());
            let std = options
                .language_standard
                .as_deref()
                .unwrap_or("gnu11")
                .to_string();
            extra_args.push(format!("-std={std}"));
        }
    }
    if options.frontend_args.is_empty() {
        for include in &options.include_directives {
            match include.kind {
                IncludeDirectiveKind::Include => {
                    extra_args.push("-I".to_string());
                    extra_args.push(include.path.clone());
                }
                IncludeDirectiveKind::System => {
                    extra_args.push("-isystem".to_string());
                    extra_args.push(include.path.clone());
                }
                IncludeDirectiveKind::Quote => {
                    extra_args.push("-iquote".to_string());
                    extra_args.push(include.path.clone());
                }
            }
        }
        for include in &options.include_paths {
            extra_args.push("-I".to_string());
            extra_args.push(include.clone());
        }
        for define in merged_defines(path, &options.defines) {
            extra_args.push(format!("-D{}", define));
        }
    } else {
        extra_args.extend(options.frontend_args.iter().cloned());
        for define in path_specific_defines(path) {
            if !frontend_args_contains_define(&options.frontend_args, &define) {
                extra_args.push(format!("-D{}", define));
            }
        }
    }

    if options.language == ParserLanguage::Cpp
        && delayed_template_parsing
        && !frontend_args_has_template_parsing_override(&extra_args)
    {
        // Delayed template parsing is only enabled on explicit delayed attempts.
        extra_args.push("-fdelayed-template-parsing".to_string());
    }

    let mut parser = LibToolingParser::new().with_extra_args(extra_args);
    if let Ok(cwd) = std::env::current_dir() {
        parser = parser.with_compile_commands_dir(&cwd.to_string_lossy());
    } else if let Some(parent) = path.parent() {
        parser = parser.with_compile_commands_dir(&parent.to_string_lossy());
    }
    if options.libtooling_skip_system_headers {
        parser = parser.with_skip_system_headers(true);
    }
    parser
}

fn template_parsing_attempts(
    language: ParserLanguage,
    mode: TemplateParsingMode,
) -> &'static [bool] {
    const STANDARD_ONLY: [bool; 1] = [false];
    const DELAYED_ONLY: [bool; 1] = [true];
    const AUTO_CPP: [bool; 2] = [false, true];

    if language != ParserLanguage::Cpp {
        return &STANDARD_ONLY;
    }
    match mode {
        TemplateParsingMode::Auto => &AUTO_CPP,
        TemplateParsingMode::Standard => &STANDARD_ONLY,
        TemplateParsingMode::Delayed => &DELAYED_ONLY,
    }
}

fn template_parsing_label(delayed: bool) -> &'static str {
    if delayed {
        "delayed"
    } else {
        "standard"
    }
}

fn parser_output_to_parser_language(language: ParserCoreLanguage) -> ParserLanguage {
    match language {
        ParserCoreLanguage::C => ParserLanguage::C,
        ParserCoreLanguage::Cpp => ParserLanguage::Cpp,
    }
}

fn parser_output_frontend_include_paths(frontend_args: &[String]) -> Vec<String> {
    let mut include_paths = Vec::new();
    let mut index = 0usize;
    while index < frontend_args.len() {
        let arg = frontend_args[index].as_str();
        if matches!(arg, "-I" | "-isystem" | "-iquote") {
            if let Some(next) = frontend_args.get(index + 1) {
                if let Some(path) = sanitize_parser_output_frontend_value(next) {
                    include_paths.push(path);
                }
            }
            index += 2;
            continue;
        }
        if let Some(path) = arg
            .strip_prefix("-I")
            .or_else(|| arg.strip_prefix("-isystem"))
            .or_else(|| arg.strip_prefix("-iquote"))
            .and_then(sanitize_parser_output_frontend_value)
        {
            include_paths.push(path);
        }
        index += 1;
    }
    include_paths
}

fn parser_output_frontend_defines(frontend_args: &[String]) -> Vec<String> {
    let mut defines = Vec::new();
    let mut index = 0usize;
    while index < frontend_args.len() {
        let arg = frontend_args[index].as_str();
        if arg == "-D" {
            if let Some(next) = frontend_args.get(index + 1) {
                if let Some(define) = sanitize_parser_output_frontend_value(next) {
                    defines.push(define);
                }
            }
            index += 2;
            continue;
        }
        if let Some(define) = arg
            .strip_prefix("-D")
            .and_then(sanitize_parser_output_frontend_value)
        {
            defines.push(define);
        }
        index += 1;
    }
    defines
}

fn sanitize_parser_output_frontend_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('=').unwrap_or(trimmed).trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn dedupe_parser_output_values<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }
        let normalized = normalized.to_string();
        if seen.insert(normalized.clone()) {
            deduped.push(normalized);
        }
    }
    deduped
}

fn parser_output_effective_include_paths(parser_output: &ParserOutputV1) -> Vec<String> {
    let requested = parser_output
        .translation_unit
        .include_directives
        .iter()
        .map(|directive: &ParserCoreIncludeDirective| directive.path.clone());
    let frontend =
        parser_output_frontend_include_paths(&parser_output.translation_unit.frontend_args)
            .into_iter();
    dedupe_parser_output_values(requested.chain(frontend))
}

fn parser_output_effective_defines(parser_output: &ParserOutputV1) -> Vec<String> {
    let requested = parser_output.translation_unit.defines.iter().cloned();
    let frontend =
        parser_output_frontend_defines(&parser_output.translation_unit.frontend_args).into_iter();
    dedupe_parser_output_values(requested.chain(frontend))
}

fn stl_placeholder_family_from_node_kind(node_kind: &str) -> Option<&'static str> {
    STL_PLACEHOLDER_KIND_FAMILY_MAP
        .iter()
        .find_map(|(kind, family)| (*kind == node_kind).then_some(*family))
}

fn supported_stl_placeholder_node_kinds() -> String {
    let mut kinds = STL_PLACEHOLDER_KIND_FAMILY_MAP
        .iter()
        .map(|(kind, _)| *kind)
        .collect::<Vec<_>>();
    kinds.sort();
    kinds.join(", ")
}

fn resolve_parser_output_stl_placeholder_mappings(
    parser_output: &ParserOutputV1,
) -> Result<BTreeMap<String, String>> {
    let mut mappings = BTreeMap::new();
    let supported_kinds = supported_stl_placeholder_node_kinds();
    for node in &parser_output.nodes {
        let node_kind = node.node_kind.trim();
        if !(node_kind.starts_with("stl_") && node_kind.ends_with("_placeholder")) {
            continue;
        }
        let Some(family) = stl_placeholder_family_from_node_kind(node_kind) else {
            return Err(miette::miette!(
                "unsupported parser-output STL placeholder node kind `{}` (supported: {})",
                node_kind,
                supported_kinds
            ));
        };
        let Some(contract_entry) = pre_generated_stl_family_contract_entry_v1(family) else {
            return Err(miette::miette!(
                "missing pre-generated STL contract mapping for placeholder family `{}`",
                family
            ));
        };
        mappings
            .entry(node_kind.to_string())
            .or_insert_with(|| contract_entry.canonical_type_prefix.to_string());
    }
    Ok(mappings)
}

fn extract_missing_header_name_from_line(line: &str) -> Option<String> {
    let marker_idx = line.find("file not found")?;
    let prefix = &line[..marker_idx];

    for quote in ['\'', '"'] {
        if let Some(end) = prefix.rfind(quote) {
            if let Some(start) = prefix[..end].rfind(quote) {
                let candidate = prefix[start + 1..end].trim();
                if !candidate.is_empty() {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    None
}

fn collect_missing_header_names(parse_error: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut headers: Vec<String> = Vec::new();
    for line in parse_error.lines() {
        let Some(header) = extract_missing_header_name_from_line(line) else {
            continue;
        };
        if seen.insert(header.clone()) {
            headers.push(header);
        }
    }
    headers
}

fn sanitize_missing_header_rel_path(header: &str) -> Option<PathBuf> {
    let raw = header
        .trim()
        .trim_matches('<')
        .trim_matches('>')
        .trim_matches('"')
        .trim_matches('\'');
    if raw.is_empty() {
        return None;
    }

    let path = Path::new(raw);
    if path.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => return None,
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }

    if normalized.as_os_str().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn create_missing_header_stub_dir(headers: &[String]) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let stub_dir = std::env::temp_dir().join(format!(
        "fragile_missing_headers_{}_{}",
        std::process::id(),
        ts
    ));
    fs::create_dir_all(&stub_dir).map_err(|err| {
        miette::miette!(
            "failed to create missing-header stub directory {}: {}",
            stub_dir.display(),
            err
        )
    })?;

    for header in headers {
        let Some(rel_path) = sanitize_missing_header_rel_path(header) else {
            continue;
        };
        let stub_path = stub_dir.join(rel_path);
        if let Some(parent) = stub_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                miette::miette!(
                    "failed to create missing-header stub parent {}: {}",
                    parent.display(),
                    err
                )
            })?;
        }
        fs::write(
            &stub_path,
            format!(
                "// Auto-generated fragile missing-header stub for {}\n#pragma once\n",
                header
            ),
        )
        .map_err(|err| {
            miette::miette!(
                "failed to write missing-header stub {}: {}",
                stub_path.display(),
                err
            )
        })?;
    }

    Ok(stub_dir)
}

fn add_missing_header_stub_search_path(
    options: &TranspileOptions,
    stub_dir: &Path,
) -> TranspileOptions {
    let mut adjusted = options.clone();
    let stub_path = stub_dir.to_string_lossy().to_string();

    if adjusted.frontend_args.is_empty() {
        adjusted.include_directives.push(IncludeDirective {
            kind: IncludeDirectiveKind::Include,
            path: stub_path,
        });
    } else {
        adjusted.frontend_args.push("-I".to_string());
        adjusted.frontend_args.push(stub_path);
    }

    adjusted
}

fn parse_libtooling_context(path: &Path, options: &TranspileOptions) -> Result<AstContext> {
    let mut errors: Vec<String> = Vec::new();
    for &delayed_template_parsing in
        template_parsing_attempts(options.language, options.template_parsing_mode)
    {
        let parser = libtooling_parser_for_path(path, options, delayed_template_parsing);
        match parser.parse_file(path) {
            Ok(ctx) => return Ok(ctx),
            Err(err) => {
                errors.push(format!(
                    "{}: {}",
                    template_parsing_label(delayed_template_parsing),
                    err
                ));
            }
        }
    }

    let combined_errors = errors.join(" | ");
    let missing_headers = collect_missing_header_names(&combined_errors);
    if !missing_headers.is_empty() {
        match create_missing_header_stub_dir(&missing_headers) {
            Ok(stub_dir) => {
                let stubbed_options = add_missing_header_stub_search_path(options, &stub_dir);
                for &delayed_template_parsing in
                    template_parsing_attempts(options.language, options.template_parsing_mode)
                {
                    let parser = libtooling_parser_for_path(
                        path,
                        &stubbed_options,
                        delayed_template_parsing,
                    );
                    match parser.parse_file(path) {
                        Ok(ctx) => {
                            let _ = fs::remove_dir_all(&stub_dir);
                            return Ok(ctx);
                        }
                        Err(err) => {
                            errors.push(format!(
                                "missing-header-stub {}: {}",
                                template_parsing_label(delayed_template_parsing),
                                err
                            ));
                        }
                    }
                }
                let _ = fs::remove_dir_all(&stub_dir);
            }
            Err(err) => {
                errors.push(format!("missing-header-stub setup: {err}"));
            }
        }
    }

    Err(miette::miette!(
        "LibTooling parse failed for {} after template parsing attempt(s): {}",
        path.display(),
        errors.join(" | ")
    ))
}

fn function_param_count(
    node: &fragile_ast_exporter::clang_ast::AstNode,
    ctx: &AstContext,
) -> usize {
    node.children
        .iter()
        .flatten()
        .filter(|child_id| {
            ctx.ast_nodes
                .get(child_id)
                .is_some_and(|child| child.tag == ASTEntryTag::TagParmVarDecl)
        })
        .count()
}

fn function_has_body(node: &fragile_ast_exporter::clang_ast::AstNode, ctx: &AstContext) -> bool {
    node.children.iter().flatten().any(|child_id| {
        ctx.ast_nodes
            .get(child_id)
            .is_some_and(|child| child.tag == ASTEntryTag::TagCompoundStmt)
    })
}

fn function_identity_key(
    node: &fragile_ast_exporter::clang_ast::AstNode,
    ctx: &AstContext,
) -> Option<String> {
    if node.tag != ASTEntryTag::TagFunctionDecl {
        return None;
    }

    if let Some(canonical) = node.get_u64(9).filter(|id| *id != 0) {
        return Some(format!("fn:canon:{canonical}"));
    }

    if let Some(mangled) = node.get_string(6).filter(|s| !s.is_empty()) {
        return Some(format!("fn:mangled:{mangled}"));
    }

    let arity = function_param_count(node, ctx);
    if let Some(qualified) = node.get_string(7).filter(|s| !s.is_empty()) {
        return Some(format!("fn:qualified:{qualified}:{arity}"));
    }

    let name = node.get_string(0).unwrap_or("");
    if name.is_empty() {
        None
    } else {
        Some(format!("fn:name:{name}:{arity}"))
    }
}

fn should_replace_function_candidate(
    current: &fragile_ast_exporter::clang_ast::AstNode,
    candidate: &fragile_ast_exporter::clang_ast::AstNode,
    ctx: &AstContext,
) -> bool {
    let current_has_body = function_has_body(current, ctx);
    let candidate_has_body = function_has_body(candidate, ctx);
    if current_has_body != candidate_has_body {
        return candidate_has_body;
    }

    let current_has_mangled = current
        .get_string(6)
        .is_some_and(|s| !s.is_empty() && s != current.get_string(0).unwrap_or(""));
    let candidate_has_mangled = candidate
        .get_string(6)
        .is_some_and(|s| !s.is_empty() && s != candidate.get_string(0).unwrap_or(""));
    if current_has_mangled != candidate_has_mangled {
        return candidate_has_mangled;
    }

    false
}

fn dedup_function_roots(ctx: &AstContext, root_ids: Vec<u64>) -> Vec<u64> {
    let mut deduped: Vec<u64> = Vec::new();
    let mut key_to_index: HashMap<String, usize> = HashMap::new();

    for id in root_ids {
        let Some(node) = ctx.ast_nodes.get(&id) else {
            continue;
        };
        if node.tag != ASTEntryTag::TagFunctionDecl {
            deduped.push(id);
            continue;
        }

        let Some(key) = function_identity_key(node, ctx) else {
            deduped.push(id);
            continue;
        };

        if let Some(existing_index) = key_to_index.get(&key).copied() {
            let Some(existing_node) = ctx.ast_nodes.get(&deduped[existing_index]) else {
                deduped[existing_index] = id;
                continue;
            };
            if should_replace_function_candidate(existing_node, node, ctx) {
                deduped[existing_index] = id;
            }
            continue;
        }

        key_to_index.insert(key, deduped.len());
        deduped.push(id);
    }

    deduped
}

fn has_decl_context_parent(
    ctx: &AstContext,
    parent_map: &HashMap<u64, Vec<u64>>,
    node_id: u64,
) -> bool {
    parent_map.get(&node_id).is_some_and(|parents| {
        parents.iter().any(|parent_id| {
            ctx.ast_nodes.get(parent_id).is_some_and(|parent| {
                matches!(
                    parent.tag,
                    ASTEntryTag::TagNamespaceDecl
                        | ASTEntryTag::TagCXXRecordDecl
                        | ASTEntryTag::TagClassTemplateDecl
                        | ASTEntryTag::TagClassTemplateSpecializationDecl
                        | ASTEntryTag::TagFunctionTemplateDecl
                        | ASTEntryTag::TagFunctionDecl
                        | ASTEntryTag::TagCXXMethodDecl
                        | ASTEntryTag::TagCXXConstructorDecl
                        | ASTEntryTag::TagCXXDestructorDecl
                        | ASTEntryTag::TagEnumDecl
                        | ASTEntryTag::TagDeclStmt
                )
            })
        })
    })
}

fn is_promotable_decl_tag(tag: ASTEntryTag) -> bool {
    matches!(
        tag,
        ASTEntryTag::TagFunctionDecl
            | ASTEntryTag::TagVarDecl
            | ASTEntryTag::TagCXXRecordDecl
            | ASTEntryTag::TagClassTemplateDecl
            | ASTEntryTag::TagClassTemplateSpecializationDecl
            | ASTEntryTag::TagFunctionTemplateDecl
            | ASTEntryTag::TagNamespaceDecl
            | ASTEntryTag::TagTypedefDecl
            | ASTEntryTag::TagTypeAliasDecl
            | ASTEntryTag::TagEnumDecl
            | ASTEntryTag::TagUsingDecl
            | ASTEntryTag::TagUsingDirectiveDecl
    )
}

fn span_has_bounds(span: &SrcSpan) -> bool {
    span.begin_line > 0
        && span.end_line > 0
        && ((span.end_line > span.begin_line)
            || (span.end_line == span.begin_line && span.end_column >= span.begin_column))
}

fn span_pos_leq(line_a: u64, col_a: u64, line_b: u64, col_b: u64) -> bool {
    line_a < line_b || (line_a == line_b && col_a <= col_b)
}

fn span_contains(outer: &SrcSpan, inner: &SrcSpan) -> bool {
    if !span_has_bounds(outer) || !span_has_bounds(inner) {
        return false;
    }
    if outer.file_id != 0 && inner.file_id != 0 && outer.file_id != inner.file_id {
        return false;
    }
    span_pos_leq(
        outer.begin_line,
        outer.begin_column,
        inner.begin_line,
        inner.begin_column,
    ) && span_pos_leq(
        inner.end_line,
        inner.end_column,
        outer.end_line,
        outer.end_column,
    )
}

fn function_like_spans(ctx: &AstContext) -> Vec<SrcSpan> {
    ctx.ast_nodes
        .values()
        .filter(|node| {
            matches!(
                node.tag,
                ASTEntryTag::TagFunctionDecl
                    | ASTEntryTag::TagCXXMethodDecl
                    | ASTEntryTag::TagCXXConstructorDecl
                    | ASTEntryTag::TagCXXDestructorDecl
                    | ASTEntryTag::TagFunctionTemplateDecl
            )
        })
        .map(|node| node.loc)
        .filter(span_has_bounds)
        .collect()
}

fn should_keep_root_var_decl(
    node: &fragile_ast_exporter::clang_ast::AstNode,
    fn_spans: &[SrcSpan],
) -> bool {
    // Exporter payload for VarDecl extras:
    // [name, isStaticLocal, isConstexpr, hasExternalStorage, isStaticStorage, isExternStorage, qualifiedName, namespacePath, canonicalId]
    if node.get_bool(1).unwrap_or(false) {
        return false;
    }

    // Clang formats function-scope qualified names as `foo()::x`.
    if node
        .get_string(6)
        .is_some_and(|qname| qname.contains(")::"))
    {
        return false;
    }

    // Guard against malformed parent edges: local variables can be surfaced as
    // roots in LibTooling export when statement links are pruned.
    !fn_spans.iter().any(|span| span_contains(span, &node.loc))
}

fn translation_unit_from_libtooling_context(ctx: &AstContext) -> ClangNode {
    let mut root_ids = ctx.top_nodes.clone();
    let mut parent_map: HashMap<u64, Vec<u64>> = HashMap::new();
    for (parent_id, node) in &ctx.ast_nodes {
        for child_id in node.children.iter().flatten() {
            parent_map.entry(*child_id).or_default().push(*parent_id);
        }
    }
    let fn_spans = function_like_spans(ctx);
    root_ids.retain(|id| {
        let Some(node) = ctx.ast_nodes.get(id) else {
            return false;
        };
        if node.tag == ASTEntryTag::TagParmVarDecl {
            return false;
        }
        if node.tag != ASTEntryTag::TagVarDecl {
            return true;
        }
        if has_decl_context_parent(ctx, &parent_map, node.id) {
            return false;
        }
        should_keep_root_var_decl(node, &fn_spans)
    });
    let mut seen_roots: HashSet<u64> = root_ids.iter().copied().collect();

    // Promote declaration nodes that can be pruned from `top_nodes` because
    // they are referenced by expression edges (for example DeclRefExpr).
    let mut candidate_ids: Vec<u64> = ctx
        .ast_nodes
        .iter()
        .filter_map(|(id, node)| is_promotable_decl_tag(node.tag).then_some(*id))
        .collect();
    candidate_ids.sort_unstable();

    for node_id in candidate_ids {
        let Some(node) = ctx.ast_nodes.get(&node_id) else {
            continue;
        };

        if seen_roots.contains(&node.id) {
            continue;
        }
        if has_decl_context_parent(ctx, &parent_map, node.id) {
            continue;
        }

        if node.tag == ASTEntryTag::TagFunctionDecl {
            let has_reference_parent = parent_map
                .get(&node.id)
                .is_some_and(|parents| !parents.is_empty());
            if !function_has_body(node, ctx) && !has_reference_parent {
                continue;
            }
            let name = node.get_string(0).unwrap_or("");
            if name.is_empty() || name.starts_with("__") {
                continue;
            }
        }
        if node.tag == ASTEntryTag::TagVarDecl && !node.children.iter().any(|c| c.is_some()) {
            continue;
        }
        if node.tag == ASTEntryTag::TagVarDecl && !should_keep_root_var_decl(node, &fn_spans) {
            continue;
        }

        root_ids.push(node.id);
        seen_roots.insert(node.id);
    }

    root_ids = dedup_function_roots(ctx, root_ids);

    let children = root_ids
        .iter()
        .filter_map(|id| convert_to_clang_node(ctx, *id))
        .collect();
    ClangNode::new(ClangNodeKind::TranslationUnit).with_children(children)
}

fn apply_libtooling_enrichment(codegen: &mut AstCodeGen, ctx: &AstContext) {
    let method_bodies = extract_method_bodies_with_params(ctx);
    if !method_bodies.is_empty() {
        codegen.set_libtooling_bodies(method_bodies);
    }

    let field_types = extract_specialization_field_types(ctx);
    if !field_types.is_empty() {
        codegen.set_specialization_field_types(field_types);
    }

    let method_sigs = extract_specialization_method_signatures(ctx);
    if !method_sigs.is_empty() {
        codegen.set_specialization_method_signatures(method_sigs);
    }
}

/// Parse a C++ source file and transpile to Rust source code.
///
/// This is the main entry point for the C++ to Rust transpiler.
/// Uses direct AST-to-Rust code generation for clean output.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use fragile_clang::transpile_cpp_to_rust;
///
/// let rust_code = transpile_cpp_to_rust(Path::new("example.cpp"))?;
/// println!("{}", rust_code);
/// ```
pub fn transpile_cpp_to_rust(path: &Path) -> Result<String> {
    transpile_cpp_to_rust_with_backend(path, ParserBackend::Libtooling)
}

/// Parse a C++ source file and transpile to Rust source code with the selected parser backend.
pub fn transpile_cpp_to_rust_with_backend(path: &Path, backend: ParserBackend) -> Result<String> {
    let options = TranspileOptions {
        backend,
        ..TranspileOptions::default()
    };
    transpile_cpp_to_rust_with_options(path, &options)
}

/// Parse a source file and transpile with explicit parser + codegen options.
pub fn transpile_cpp_to_rust_with_options(
    path: &Path,
    options: &TranspileOptions,
) -> Result<String> {
    let trace_path = options.stage_timing_trace_path.as_deref();
    initialize_stage_trace(trace_path, path, options.backend);
    let mut timings = TranspileStageTimings::default();
    let mut codegen = AstCodeGen::new();
    let transpile_result: Result<String> = (|| {
        let ctx = trace_stage(
            trace_path,
            TRANSPILE_STAGE_EXPORT,
            &mut timings.export,
            || parse_libtooling_context(path, options),
        )?;
        let translation_unit = trace_stage(
            trace_path,
            TRANSPILE_STAGE_PARSE,
            &mut timings.parse,
            || Ok(translation_unit_from_libtooling_context(&ctx)),
        )?;
        trace_stage(
            trace_path,
            TRANSPILE_STAGE_ENRICHMENT,
            &mut timings.enrichment,
            || {
                apply_libtooling_enrichment(&mut codegen, &ctx);
                Ok(())
            },
        )?;
        trace_stage(
            trace_path,
            TRANSPILE_STAGE_CODEGEN,
            &mut timings.codegen,
            || Ok(codegen.generate(&translation_unit)),
        )
    })();

    if let Err(err) = &transpile_result {
        let err_message = err.to_string();
        finalize_stage_trace(trace_path, &timings, "error", Some(err_message.as_str()));
    } else {
        finalize_stage_trace(trace_path, &timings, "completed", None);
    }
    transpile_result
}

/// Transpile via parser-output handoff without invoking LibTooling parse/export.
///
/// This consumes `ParserOutputV1` metadata and performs codegen using a libclang parse.
pub fn transpile_parser_output_to_rust(parser_output: &ParserOutputV1) -> Result<String> {
    transpile_parser_output_to_rust_with_options(
        parser_output,
        &ParserOutputCodegenOptions::default(),
    )
}

/// Transpile via parser-output handoff with explicit codegen options.
pub fn transpile_parser_output_to_rust_with_options(
    parser_output: &ParserOutputV1,
    options: &ParserOutputCodegenOptions,
) -> Result<String> {
    if parser_output.schema_version != PARSER_OUTPUT_SCHEMA_VERSION_V1 {
        return Err(miette::miette!(
            "unsupported parser output schema `{}` (expected `{}`)",
            parser_output.schema_version,
            PARSER_OUTPUT_SCHEMA_VERSION_V1
        ));
    }

    let source = &parser_output.translation_unit.source_path;
    let trace_path = options.stage_timing_trace_path.as_deref();
    initialize_stage_trace_with_backend_label(
        trace_path,
        source,
        PARSER_OUTPUT_HANDOFF_BACKEND_LABEL,
    );
    let mut timings = TranspileStageTimings::default();
    let transpile_result: Result<String> = (|| {
        let placeholder_mappings = resolve_parser_output_stl_placeholder_mappings(parser_output)?;
        let parser = ClangParser::with_paths_defines_language_and_ignored_errors(
            parser_output_effective_include_paths(parser_output),
            parser_output_effective_defines(parser_output),
            parser_output_to_parser_language(parser_output.translation_unit.language),
            options.ignored_error_patterns.clone(),
        )
        .map_err(|err| {
            miette::miette!(
                "failed to initialize parser-output handoff parser for {} (backend={}): {}",
                source.display(),
                parser_output.translation_unit.parser_backend,
                err
            )
        })?;

        let ast = trace_stage(trace_path, TRANSPILE_STAGE_PARSE, &mut timings.parse, || {
            parser.parse_file(source)
        })?;

        trace_stage(
            trace_path,
            TRANSPILE_STAGE_CODEGEN,
            &mut timings.codegen,
            || {
                let mut codegen = AstCodeGen::new();
                codegen.set_parser_output_stl_placeholder_mappings(placeholder_mappings.clone());
                Ok(codegen.generate(&ast.translation_unit))
            },
        )
    })();

    if let Err(err) = &transpile_result {
        let err_message = err.to_string();
        finalize_stage_trace(trace_path, &timings, "error", Some(err_message.as_str()));
    } else {
        finalize_stage_trace(trace_path, &timings, "completed", None);
    }
    transpile_result
}

/// Generate Rust stubs from a C++ source file.
///
/// Stubs are function signatures with placeholder bodies,
/// useful for FFI declarations.
pub fn generate_stubs(path: &Path) -> Result<String> {
    let options = TranspileOptions::default();
    let ctx = parse_libtooling_context(path, &options)?;
    let translation_unit = translation_unit_from_libtooling_context(&ctx);
    Ok(AstCodeGen::new().generate_stubs(&translation_unit))
}

/// Parse a C++ source file and transpile to Rust source code with LibTooling.
pub fn transpile_cpp_to_rust_with_libtooling(path: &Path) -> Result<String> {
    transpile_cpp_to_rust_with_backend(path, ParserBackend::Libtooling)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fragile_ast_exporter::{
        clang_ast::{AstNode, SrcFile},
        CborValue,
    };
    use fragile_parser_core::{
        IncludeDirective as ParserCoreIncludeDirective,
        IncludeDirectiveKind as ParserCoreIncludeDirectiveKind,
        ParserDiagnostic,
        ParserLanguage as ParserCoreLanguage,
        ParserNode,
        ParserOutputV1,
        ParserTranslationUnit,
    };

    fn span(
        file_id: u64,
        begin_line: u64,
        begin_column: u64,
        end_line: u64,
        end_column: u64,
    ) -> SrcSpan {
        SrcSpan {
            file_id,
            begin_line,
            begin_column,
            end_line,
            end_column,
        }
    }

    fn ast_node(
        id: u64,
        tag: ASTEntryTag,
        children: Vec<Option<u64>>,
        loc: SrcSpan,
        extras: Vec<CborValue>,
    ) -> AstNode {
        AstNode {
            id,
            tag,
            children,
            loc,
            type_id: None,
            extras,
        }
    }

    fn parser_output_fixture(
        source_path: PathBuf,
        language: ParserCoreLanguage,
        frontend_args: Vec<String>,
        defines: Vec<String>,
        include_directives: Vec<ParserCoreIncludeDirective>,
    ) -> ParserOutputV1 {
        ParserOutputV1 {
            schema_version: PARSER_OUTPUT_SCHEMA_VERSION_V1.to_string(),
            translation_unit: ParserTranslationUnit {
                source_path,
                language,
                parser_backend: "fragile-parser-clang".to_string(),
                frontend_args,
                defines,
                include_directives,
            },
            nodes: vec![ParserNode {
                node_id: "n0".to_string(),
                parent_id: None,
                node_kind: "translation_unit".to_string(),
                name: Some("unit".to_string()),
                cpp_type: None,
            }],
            diagnostics: Vec::<ParserDiagnostic>::new(),
        }
    }

    fn parser_node_fixture(node_id: &str, node_kind: &str) -> ParserNode {
        ParserNode {
            node_id: node_id.to_string(),
            parent_id: None,
            node_kind: node_kind.to_string(),
            name: None,
            cpp_type: None,
        }
    }

    #[test]
    fn test_translation_unit_filters_function_local_vardecl_roots() {
        let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
        ast_nodes.insert(
            1,
            ast_node(
                1,
                ASTEntryTag::TagFunctionDecl,
                vec![Some(2)],
                span(1, 1, 1, 20, 1),
                vec![CborValue::Text("f".to_string())],
            ),
        );
        ast_nodes.insert(
            2,
            ast_node(
                2,
                ASTEntryTag::TagCompoundStmt,
                vec![Some(3)],
                span(1, 1, 10, 20, 1),
                vec![],
            ),
        );
        ast_nodes.insert(
            3,
            ast_node(
                3,
                ASTEntryTag::TagDeclStmt,
                vec![Some(4)],
                span(1, 5, 3, 5, 12),
                vec![],
            ),
        );
        ast_nodes.insert(
            4,
            ast_node(
                4,
                ASTEntryTag::TagVarDecl,
                vec![],
                span(1, 5, 5, 5, 9),
                vec![
                    CborValue::Text("arg".to_string()),
                    CborValue::Bool(false), // isStaticLocal
                    CborValue::Bool(false),
                    CborValue::Bool(false),
                    CborValue::Bool(false), // isStaticStorage
                    CborValue::Bool(false), // isExternStorage
                    CborValue::Text("f()::arg".to_string()),
                ],
            ),
        );

        let ctx = AstContext {
            ast_nodes,
            type_nodes: HashMap::new(),
            top_nodes: vec![1, 4],
            files: vec![SrcFile {
                path: None,
                include_loc: None,
            }],
        };

        let tu = translation_unit_from_libtooling_context(&ctx);
        assert!(
            tu.children
                .iter()
                .all(|child| !matches!(child.kind, ClangNodeKind::VarDecl { .. })),
            "function-local VarDecl roots should be filtered out"
        );
    }

    #[test]
    fn test_translation_unit_keeps_true_global_vardecl_roots() {
        let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
        ast_nodes.insert(
            10,
            ast_node(
                10,
                ASTEntryTag::TagFunctionDecl,
                vec![Some(11)],
                span(1, 1, 1, 8, 1),
                vec![CborValue::Text("f".to_string())],
            ),
        );
        ast_nodes.insert(
            11,
            ast_node(
                11,
                ASTEntryTag::TagCompoundStmt,
                vec![],
                span(1, 1, 10, 8, 1),
                vec![],
            ),
        );
        ast_nodes.insert(
            12,
            ast_node(
                12,
                ASTEntryTag::TagVarDecl,
                vec![Some(13)],
                span(1, 20, 1, 20, 14),
                vec![
                    CborValue::Text("g_value".to_string()),
                    CborValue::Bool(false), // isStaticLocal
                    CborValue::Bool(false),
                    CborValue::Bool(false),
                    CborValue::Bool(false), // isStaticStorage
                    CborValue::Bool(false), // isExternStorage
                    CborValue::Text("g_value".to_string()),
                ],
            ),
        );
        ast_nodes.insert(
            13,
            ast_node(
                13,
                ASTEntryTag::TagIntegerLiteral,
                vec![],
                span(1, 20, 12, 20, 12),
                vec![CborValue::Integer(1.into())],
            ),
        );

        let ctx = AstContext {
            ast_nodes,
            type_nodes: HashMap::new(),
            top_nodes: vec![10, 12],
            files: vec![SrcFile {
                path: None,
                include_loc: None,
            }],
        };

        let tu = translation_unit_from_libtooling_context(&ctx);
        assert!(
            tu.children.iter().any(|child| {
                matches!(
                    &child.kind,
                    ClangNodeKind::VarDecl { name, .. } if name == "g_value"
                )
            }),
            "true global VarDecl roots should remain in the translation unit"
        );
    }

    #[test]
    fn test_translation_unit_filters_static_local_vardecl_roots() {
        let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
        ast_nodes.insert(
            20,
            ast_node(
                20,
                ASTEntryTag::TagVarDecl,
                vec![],
                span(0, 0, 0, 0, 0),
                vec![
                    CborValue::Text("cache".to_string()),
                    CborValue::Bool(true), // isStaticLocal
                    CborValue::Bool(false),
                    CborValue::Bool(false),
                    CborValue::Bool(true),  // isStaticStorage
                    CborValue::Bool(false), // isExternStorage
                    CborValue::Text("f()::cache".to_string()),
                ],
            ),
        );

        let ctx = AstContext {
            ast_nodes,
            type_nodes: HashMap::new(),
            top_nodes: vec![20],
            files: Vec::new(),
        };

        let tu = translation_unit_from_libtooling_context(&ctx);
        assert!(
            tu.children.is_empty(),
            "static-local VarDecl roots should be filtered from translation-unit roots"
        );
    }

    #[test]
    fn test_translation_unit_filters_parm_vardecl_roots() {
        let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
        ast_nodes.insert(
            30,
            ast_node(
                30,
                ASTEntryTag::TagParmVarDecl,
                vec![],
                span(0, 0, 0, 0, 0),
                vec![CborValue::Text("dst".to_string())],
            ),
        );

        let ctx = AstContext {
            ast_nodes,
            type_nodes: HashMap::new(),
            top_nodes: vec![30],
            files: Vec::new(),
        };

        let tu = translation_unit_from_libtooling_context(&ctx);
        assert!(
            tu.children.is_empty(),
            "ParmVarDecl roots should not be promoted into translation-unit declarations"
        );
    }

    #[test]
    fn test_translation_unit_filters_root_vardecl_with_declstmt_parent() {
        let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
        ast_nodes.insert(
            40,
            ast_node(
                40,
                ASTEntryTag::TagFunctionDecl,
                vec![Some(41)],
                span(1, 1, 1, 8, 1),
                vec![CborValue::Text("f".to_string())],
            ),
        );
        ast_nodes.insert(
            41,
            ast_node(
                41,
                ASTEntryTag::TagCompoundStmt,
                vec![Some(42)],
                span(1, 1, 10, 8, 1),
                vec![],
            ),
        );
        ast_nodes.insert(
            42,
            ast_node(
                42,
                ASTEntryTag::TagDeclStmt,
                vec![Some(43)],
                span(1, 5, 2, 5, 9),
                vec![],
            ),
        );
        ast_nodes.insert(
            43,
            ast_node(
                43,
                ASTEntryTag::TagVarDecl,
                vec![Some(44)],
                span(1, 50, 12, 50, 50),
                vec![
                    CborValue::Text("src".to_string()),
                    CborValue::Bool(false), // isStaticLocal
                    CborValue::Bool(false),
                    CborValue::Bool(false),
                    CborValue::Bool(false), // isStaticStorage
                    CborValue::Bool(false), // isExternStorage
                    CborValue::Text("src".to_string()),
                ],
            ),
        );
        ast_nodes.insert(
            44,
            ast_node(
                44,
                ASTEntryTag::TagIntegerLiteral,
                vec![],
                span(1, 56, 12, 56, 56),
                vec![CborValue::Integer(0.into())],
            ),
        );

        let ctx = AstContext {
            ast_nodes,
            type_nodes: HashMap::new(),
            top_nodes: vec![40, 43],
            files: vec![SrcFile {
                path: None,
                include_loc: None,
            }],
        };

        let tu = translation_unit_from_libtooling_context(&ctx);
        assert!(
            !tu.children.iter().any(
                |child| matches!(&child.kind, ClangNodeKind::VarDecl { name, .. } if name == "src")
            ),
            "root VarDecl with DeclStmt parent should be filtered as function-local"
        );
    }

    #[test]
    fn test_translation_unit_promotes_referenced_declaration_only_function_roots() {
        let mut ast_nodes: HashMap<u64, AstNode> = HashMap::new();
        ast_nodes.insert(
            100,
            ast_node(
                100,
                ASTEntryTag::TagFunctionDecl,
                vec![Some(101)],
                span(1, 1, 1, 20, 1),
                vec![CborValue::Text("main".to_string())],
            ),
        );
        ast_nodes.insert(
            101,
            ast_node(
                101,
                ASTEntryTag::TagCompoundStmt,
                vec![Some(102)],
                span(1, 2, 1, 18, 1),
                vec![],
            ),
        );
        ast_nodes.insert(
            102,
            ast_node(
                102,
                ASTEntryTag::TagCallExpr,
                vec![Some(200)],
                span(1, 3, 1, 17, 1),
                vec![],
            ),
        );
        ast_nodes.insert(
            200,
            ast_node(
                200,
                ASTEntryTag::TagFunctionDecl,
                vec![],
                span(1, 25, 1, 25, 30),
                vec![CborValue::Text("ext".to_string())],
            ),
        );

        let ctx = AstContext {
            ast_nodes,
            type_nodes: HashMap::new(),
            top_nodes: vec![100],
            files: vec![SrcFile {
                path: None,
                include_loc: None,
            }],
        };

        let tu = translation_unit_from_libtooling_context(&ctx);
        assert!(
            tu.children.iter().any(
                |child| matches!(&child.kind, ClangNodeKind::FunctionDecl { name, is_definition, .. } if name == "ext" && !is_definition)
            ),
            "referenced declaration-only FunctionDecl roots should be promoted into the translation unit"
        );
    }

    #[test]
    fn template_parsing_attempts_cpp_auto_prefers_standard_then_delayed() {
        let attempts = template_parsing_attempts(ParserLanguage::Cpp, TemplateParsingMode::Auto);
        assert_eq!(attempts, &[false, true]);
    }

    #[test]
    fn template_parsing_attempts_cpp_standard_is_single_attempt() {
        let attempts =
            template_parsing_attempts(ParserLanguage::Cpp, TemplateParsingMode::Standard);
        assert_eq!(attempts, &[false]);
    }

    #[test]
    fn template_parsing_attempts_cpp_delayed_is_single_attempt() {
        let attempts = template_parsing_attempts(ParserLanguage::Cpp, TemplateParsingMode::Delayed);
        assert_eq!(attempts, &[true]);
    }

    #[test]
    fn template_parsing_attempts_c_ignores_mode_override() {
        let auto_attempts = template_parsing_attempts(ParserLanguage::C, TemplateParsingMode::Auto);
        let delayed_attempts =
            template_parsing_attempts(ParserLanguage::C, TemplateParsingMode::Delayed);
        assert_eq!(auto_attempts, &[false]);
        assert_eq!(delayed_attempts, &[false]);
    }

    #[test]
    fn frontend_args_template_parsing_override_detection() {
        assert!(!frontend_args_has_template_parsing_override(&[]));
        assert!(frontend_args_has_template_parsing_override(&[
            "-fdelayed-template-parsing".to_string()
        ]));
        assert!(frontend_args_has_template_parsing_override(&[
            "-fno-delayed-template-parsing".to_string()
        ]));
    }

    #[test]
    fn extract_missing_header_name_from_line_parses_common_diagnostics() {
        let single = "/tmp/foo.cc:10:10: fatal error: 'rcc_rpc.h' file not found";
        let double = "/tmp/foo.cc:10:10: fatal error: \"foo/bar/baz.hpp\" file not found";
        assert_eq!(
            extract_missing_header_name_from_line(single).as_deref(),
            Some("rcc_rpc.h")
        );
        assert_eq!(
            extract_missing_header_name_from_line(double).as_deref(),
            Some("foo/bar/baz.hpp")
        );
    }

    #[test]
    fn collect_missing_header_names_deduplicates_headers() {
        let error = "fatal error: 'a.h' file not found\nfatal error: 'a.h' file not found\nfatal error: 'b/c.h' file not found";
        let headers = collect_missing_header_names(error);
        assert_eq!(headers, vec!["a.h".to_string(), "b/c.h".to_string()]);
    }

    #[test]
    fn create_missing_header_stub_dir_creates_relative_stub_files() {
        let headers = vec!["rcc_rpc.h".to_string(), "foo/bar.hpp".to_string()];
        let stub_dir =
            create_missing_header_stub_dir(&headers).expect("failed to create stub directory");
        assert!(stub_dir.join("rcc_rpc.h").exists());
        assert!(stub_dir.join("foo/bar.hpp").exists());
        let _ = fs::remove_dir_all(stub_dir);
    }

    #[test]
    fn parser_output_stl_placeholder_mapping_resolves_known_placeholder_kinds() {
        let mut parser_output = parser_output_fixture(
            PathBuf::from("fixture.cc"),
            ParserCoreLanguage::Cpp,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        parser_output.nodes = vec![
            parser_node_fixture("n0", "translation_unit"),
            parser_node_fixture("n1", "stl_vector_placeholder"),
            parser_node_fixture("n2", "stl_map_placeholder"),
            parser_node_fixture("n3", "stl_unordered_map_placeholder"),
            parser_node_fixture("n4", "stl_map_placeholder"),
        ];

        let mappings = resolve_parser_output_stl_placeholder_mappings(&parser_output)
            .expect("known placeholders should resolve via contract");
        assert_eq!(mappings.get("stl_vector_placeholder"), Some(&"std_vector".to_string()));
        assert_eq!(mappings.get("stl_map_placeholder"), Some(&"std_map".to_string()));
        assert_eq!(
            mappings.get("stl_unordered_map_placeholder"),
            Some(&"std_unordered_map".to_string())
        );
        assert_eq!(mappings.len(), 3);
    }

    #[test]
    fn parser_output_stl_placeholder_mapping_rejects_unknown_placeholder_kind() {
        let mut parser_output = parser_output_fixture(
            PathBuf::from("fixture.cc"),
            ParserCoreLanguage::Cpp,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        parser_output.nodes = vec![
            parser_node_fixture("n0", "translation_unit"),
            parser_node_fixture("n1", "stl_deque_placeholder"),
        ];

        let err = resolve_parser_output_stl_placeholder_mappings(&parser_output)
            .expect_err("unknown placeholder kind should fail fast");
        let err_text = err.to_string();
        assert!(
            err_text.contains("stl_deque_placeholder")
                && err_text.contains("unsupported parser-output STL placeholder node kind"),
            "unexpected error for unknown placeholder kind: {err_text}"
        );
    }

    #[test]
    fn parser_output_codegen_rejects_unknown_schema_version() {
        let parser_output = ParserOutputV1 {
            schema_version: "0.0.0".to_string(),
            ..parser_output_fixture(
                PathBuf::from("fixture.c"),
                ParserCoreLanguage::C,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        };

        let err = transpile_parser_output_to_rust(&parser_output)
            .expect_err("schema mismatch should return an error");
        let err_text = err.to_string();
        assert!(
            err_text.contains("unsupported parser output schema")
                && err_text.contains("1.0.0"),
            "unexpected schema validation error: {err_text}"
        );
    }

    #[test]
    fn parser_output_codegen_rejects_unknown_stl_placeholder_kind_before_parse() {
        let mut parser_output = parser_output_fixture(
            PathBuf::from("this_file_should_not_exist_for_placeholder_validation.cc"),
            ParserCoreLanguage::Cpp,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        parser_output.nodes = vec![
            parser_node_fixture("n0", "translation_unit"),
            parser_node_fixture("n1", "stl_fake_placeholder"),
        ];

        let err = transpile_parser_output_to_rust(&parser_output)
            .expect_err("unknown placeholder should fail before parse");
        let err_text = err.to_string();
        assert!(
            err_text.contains("stl_fake_placeholder")
                && err_text.contains("unsupported parser-output STL placeholder node kind"),
            "unexpected parser-output placeholder validation error: {err_text}"
        );
    }

    #[test]
    fn parser_output_codegen_uses_handoff_metadata_without_libtooling_export() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("fragile_parser_output_codegen_{stamp}"));
        let include_dir = temp_dir.join("include");
        fs::create_dir_all(&include_dir).expect("failed to create include dir");
        fs::write(include_dir.join("value.h"), "#define HEADER_VALUE 41\n")
            .expect("failed to write header");
        let source = temp_dir.join("unit.c");
        fs::write(
            &source,
            "#include \"value.h\"\nint compute(void) { return HEADER_VALUE + FRONTEND_FLAG + REQUEST_FLAG; }\n",
        )
        .expect("failed to write source");
        let parser_output = parser_output_fixture(
            source.clone(),
            ParserCoreLanguage::C,
            vec!["-DFRONTEND_FLAG=3".to_string()],
            vec!["REQUEST_FLAG=7".to_string()],
            vec![ParserCoreIncludeDirective {
                kind: ParserCoreIncludeDirectiveKind::Include,
                path: include_dir.to_string_lossy().to_string(),
            }],
        );
        let stage_trace_path = temp_dir.join("stage_trace.txt");
        let transpiled = transpile_parser_output_to_rust_with_options(
            &parser_output,
            &ParserOutputCodegenOptions {
                ignored_error_patterns: Vec::new(),
                stage_timing_trace_path: Some(stage_trace_path.clone()),
            },
        )
        .expect("parser-output handoff transpile should succeed");
        assert!(
            transpiled.contains("fn compute("),
            "transpiled output should contain compute function:\n{}",
            transpiled
        );

        let trace =
            fs::read_to_string(&stage_trace_path).expect("failed to read stage trace output");
        assert!(
            trace.contains("backend=parser-output-handoff")
                && trace.contains("event=stage_end stage=parse status=ok")
                && trace.contains("event=stage_end stage=codegen status=ok")
                && trace.contains("status=completed"),
            "stage trace missing parser-output handoff markers:\n{}",
            trace
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn parser_output_codegen_active_handoff_blocks_legacy_associative_std_collections_alias_lanes(
    ) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let temp_dir =
            std::env::temp_dir().join(format!("fragile_parser_output_mapping_lanes_{stamp}"));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
        let source = temp_dir.join("assoc_lane_probe.cc");
        fs::write(
            &source,
            "class map_unsigned_int__bool;\n\
             class unordered_map_unsigned_int__bool;\n\
             struct Holder {\n\
             \tmap_unsigned_int__bool* ordered;\n\
             \tunordered_map_unsigned_int__bool* unordered;\n\
             };\n\
             int probe(Holder* h) { return h == 0 ? 0 : 1; }\n",
        )
        .expect("failed to write source");

        let unmapped_parser_output = parser_output_fixture(
            source.clone(),
            ParserCoreLanguage::Cpp,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let unmapped = transpile_parser_output_to_rust(&unmapped_parser_output)
            .expect("unmapped parser-output handoff transpile should succeed");
        assert!(
            unmapped.contains("// parser_output_stl_placeholder_mapping_manifest_v1:")
                && unmapped.contains("// parser_output_mapping_context_enabled=true")
                && unmapped.contains("// parser_output_observed_family_count=0")
                && unmapped.contains("// parser_output_observed_families=<none>"),
            "active parser-output handoff run should emit deterministic empty observed-family mapping manifest in handoff context:\n{}",
            unmapped
        );
        assert!(
            !unmapped.contains(
                "pub type map_unsigned_int__bool = std::collections::BTreeMap<u32, bool>;"
            ),
            "active parser-output handoff run should block legacy ordered-map std::collections alias lane even when no placeholder mapping nodes are present:\n{}",
            unmapped
        );
        assert!(
            !unmapped.contains(
                "pub type unordered_map_unsigned_int__bool = std::collections::HashMap<u32, bool>;"
            ),
            "active parser-output handoff run should block legacy unordered-map std::collections alias lane even when no placeholder mapping nodes are present:\n{}",
            unmapped
        );
        assert!(
            unmapped.contains("pub struct map_unsigned_int__bool {")
                && unmapped.contains("pub struct unordered_map_unsigned_int__bool {"),
            "active parser-output handoff run should keep unresolved mapped associative families explicit when no concrete mapping lane resolves:\n{}",
            unmapped
        );

        let mut mapped_parser_output = parser_output_fixture(
            source,
            ParserCoreLanguage::Cpp,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        mapped_parser_output.nodes.push(parser_node_fixture(
            "n1",
            "stl_map_placeholder",
        ));
        mapped_parser_output.nodes.push(parser_node_fixture(
            "n2",
            "stl_unordered_map_placeholder",
        ));
        let mapped = transpile_parser_output_to_rust(&mapped_parser_output)
            .expect("mapped parser-output handoff transpile should succeed");
        assert!(
            mapped.contains("// parser_output_stl_placeholder_mapping_manifest_v1:")
                && mapped.contains("// parser_output_mapping_context_enabled=true")
                && mapped.contains("// parser_output_observed_family_count=2"),
            "mapped active parser-output handoff run should emit deterministic observed-family mapping manifest summary:\n{}",
            mapped
        );
        let map_manifest_line =
            "// parser_output_observed_family.map.placeholder_kind=stl_map_placeholder";
        let unordered_manifest_line = "// parser_output_observed_family.unordered_map.placeholder_kind=stl_unordered_map_placeholder";
        assert!(
            mapped.contains(map_manifest_line)
                && mapped.contains(
                    "// parser_output_observed_family.map.canonical_type_prefix=std_map"
                )
                && mapped.contains(unordered_manifest_line)
                && mapped.contains(
                    "// parser_output_observed_family.unordered_map.canonical_type_prefix=std_unordered_map"
                ),
            "mapped active parser-output handoff run should emit observed-family mapping entries with canonical prefixes:\n{}",
            mapped
        );
        let map_pos = mapped
            .find(map_manifest_line)
            .expect("missing map mapping manifest line");
        let unordered_pos = mapped
            .find(unordered_manifest_line)
            .expect("missing unordered_map mapping manifest line");
        assert!(
            map_pos < unordered_pos,
            "mapped active parser-output manifest entries should be deterministic and key-ordered:\n{}",
            mapped
        );
        assert!(
            !mapped.contains(
                "pub type map_unsigned_int__bool = std::collections::BTreeMap<u32, bool>;"
            ),
            "mapped active parser-output run should not use legacy ordered-map std::collections alias lane:\n{}",
            mapped
        );
        assert!(
            !mapped.contains(
                "pub type unordered_map_unsigned_int__bool = std::collections::HashMap<u32, bool>;"
            ),
            "mapped active parser-output run should not use legacy unordered-map std::collections alias lane:\n{}",
            mapped
        );
        assert!(
            mapped.contains("pub struct map_unsigned_int__bool {")
                && mapped.contains("pub struct unordered_map_unsigned_int__bool {"),
            "mapped active parser-output run should keep unsupported mapped associative families explicit as unresolved placeholders:\n{}",
            mapped
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
