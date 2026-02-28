//! Clang AST parsing and Rust code generation for the Fragile polyglot compiler.
//!
//! This crate provides:
//! - C++ source parsing via libclang
//! - Clang AST traversal and extraction
//! - Direct AST-to-Rust source code generation
//!
//! # Architecture
//!
//! ```text
//! C++ Source → libclang → Clang AST → Rust Source (via AstCodeGen)
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

use fragile_ast_exporter::{clang_ast::AstContext, ASTEntryTag, CborValue};
use miette::Result;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Parser backend selection for transpilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserBackend {
    /// Parse translation unit with libclang only.
    Libclang,
    /// Parse translation unit with LibTooling conversion only.
    Libtooling,
    /// Parse translation unit with libclang and enrich template/specialization data with LibTooling.
    Hybrid,
}

/// Transpile configuration.
#[derive(Debug, Clone)]
pub struct TranspileOptions {
    pub include_paths: Vec<String>,
    pub defines: Vec<String>,
    pub language: ParserLanguage,
    pub language_standard: Option<String>,
    pub ignored_error_patterns: Vec<String>,
    pub backend: ParserBackend,
    pub libtooling_skip_system_headers: bool,
    pub stage_timing_trace_path: Option<PathBuf>,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            include_paths: Vec::new(),
            defines: Vec::new(),
            language: ParserLanguage::Cpp,
            language_standard: None,
            ignored_error_patterns: Vec::new(),
            backend: ParserBackend::Libclang,
            libtooling_skip_system_headers: false,
            stage_timing_trace_path: None,
        }
    }
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

fn initialize_stage_trace(trace_path: Option<&Path>, source: &Path, backend: ParserBackend) {
    let Some(path) = trace_path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut initial = String::new();
    initial.push_str(&format!("source={}\n", source.display()));
    initial.push_str(&format!("backend={}\n", parser_backend_label(backend)));
    initial.push_str("status=started\n");
    let _ = fs::write(path, initial);
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

fn stage_skip(trace_path: Option<&Path>, stage: &str, reason: &str) {
    append_stage_trace_line(
        trace_path,
        format!(
            "event=stage_skip stage={} elapsed_ms=0 reason={}",
            stage, reason
        )
        .as_str(),
    );
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

fn parser_for_path_with_options(path: &Path, options: &TranspileOptions) -> Result<ClangParser> {
    ClangParser::with_paths_defines_language_and_ignored_errors(
        options.include_paths.clone(),
        merged_defines(path, &options.defines),
        options.language,
        options.ignored_error_patterns.clone(),
    )
}

fn parser_for_path(path: &Path) -> Result<ClangParser> {
    parser_for_path_with_options(path, &TranspileOptions::default())
}

fn libtooling_parser_for_path(path: &Path, options: &TranspileOptions) -> LibToolingParser {
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
            // Keep LibTooling aligned with libclang's tolerant template parsing for
            // known upstream headers (e.g. RapidJSON GenericStringRef assignment form)
            // that are semantically diagnosed only when eagerly parsing template bodies.
            extra_args.push("-fdelayed-template-parsing".to_string());
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
    for include in &options.include_paths {
        extra_args.push(format!("-I{}", include));
    }
    for define in merged_defines(path, &options.defines) {
        extra_args.push(format!("-D{}", define));
    }

    let mut parser = LibToolingParser::new().with_extra_args(extra_args);
    if let Some(parent) = path.parent() {
        let compile_dir = parent.to_string_lossy().to_string();
        parser = parser.with_compile_commands_dir(&compile_dir);
    }
    if options.libtooling_skip_system_headers {
        parser = parser.with_skip_system_headers(true);
    }
    parser
}

fn parse_libtooling_context(path: &Path, options: &TranspileOptions) -> Result<AstContext> {
    // Preserve strict diagnostic semantics from the libclang parser
    // (including scoped RapidJSON const-member assignment tolerance)
    // before consuming the richer LibTooling AST export.
    let _ = parser_for_path_with_options(path, options)?.parse_file(path)?;

    let parser = libtooling_parser_for_path(path, options);
    parser.parse_file(path)
}

fn translation_unit_from_libtooling_context(ctx: &AstContext) -> ClangNode {
    let mut root_ids = ctx.top_nodes.clone();
    let mut seen_roots: std::collections::HashSet<u64> = root_ids.iter().copied().collect();

    // Promote concrete function-template instantiation decls that can be nested
    // in the exported graph and therefore omitted from computed top nodes.
    for node in ctx.ast_nodes.values() {
        if node.tag != ASTEntryTag::TagFunctionDecl || seen_roots.contains(&node.id) {
            continue;
        }

        let has_body = node.children.iter().flatten().any(|child_id| {
            ctx.ast_nodes
                .get(child_id)
                .is_some_and(|child| child.tag == ASTEntryTag::TagCompoundStmt)
        });
        if !has_body {
            continue;
        }

        let has_template_args = node
            .extras
            .get(5)
            .is_some_and(|extra| matches!(extra, CborValue::Array(args) if !args.is_empty()));
        if !node.get_bool(4).unwrap_or(false) && !has_template_args {
            continue;
        }

        let name = node.get_string(0).unwrap_or("");
        if name.is_empty() || name.starts_with("__") {
            continue;
        }

        root_ids.push(node.id);
        seen_roots.insert(node.id);
    }

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
    transpile_cpp_to_rust_with_backend(path, ParserBackend::Libclang)
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
        let translation_unit = match options.backend {
            ParserBackend::Libclang => {
                let ast = trace_stage(
                    trace_path,
                    TRANSPILE_STAGE_PARSE,
                    &mut timings.parse,
                    || {
                        let parser = parser_for_path_with_options(path, options)?;
                        parser.parse_file(path)
                    },
                )?;
                timings.export = Duration::ZERO;
                stage_skip(trace_path, TRANSPILE_STAGE_EXPORT, "backend_without_export");
                timings.enrichment = Duration::ZERO;
                stage_skip(
                    trace_path,
                    TRANSPILE_STAGE_ENRICHMENT,
                    "backend_without_enrichment",
                );
                ast.translation_unit
            }
            ParserBackend::Hybrid => {
                let ast = trace_stage(
                    trace_path,
                    TRANSPILE_STAGE_PARSE,
                    &mut timings.parse,
                    || {
                        let parser = parser_for_path_with_options(path, options)?;
                        parser.parse_file(path)
                    },
                )?;
                let ctx = trace_stage(
                    trace_path,
                    TRANSPILE_STAGE_EXPORT,
                    &mut timings.export,
                    || parse_libtooling_context(path, options),
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
                ast.translation_unit
            }
            ParserBackend::Libtooling => {
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
                translation_unit
            }
        };
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

/// Generate Rust stubs from a C++ source file.
///
/// Stubs are function signatures with placeholder bodies,
/// useful for FFI declarations.
pub fn generate_stubs(path: &Path) -> Result<String> {
    let parser = parser_for_path(path)?;
    let ast = parser.parse_file(path)?;
    Ok(AstCodeGen::new().generate_stubs(&ast.translation_unit))
}

/// Parse a C++ source file and transpile to Rust source code with hybrid LibTooling enrichment.
///
/// This version uses LibTooling to get template method bodies and specialization details
/// while keeping libclang as the primary AST source.
pub fn transpile_cpp_to_rust_with_libtooling(path: &Path) -> Result<String> {
    transpile_cpp_to_rust_with_backend(path, ParserBackend::Hybrid)
}
