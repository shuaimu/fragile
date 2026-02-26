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
    TypeTraitKind, UnaryOp,
};
pub use ast_codegen::AstCodeGen;
pub use libtooling::{
    convert_to_clang_node, extract_method_bodies, extract_method_bodies_with_params,
    extract_specialization_field_types, extract_specialization_method_signatures, LibToolingParser,
    MethodInfo, MethodSignature, SpecializationFieldInfo, TemplateMethodInstantiation,
};
pub use parse::{ClangParser, ParserLanguage};
pub use types::{CppType, TypeProperties, TypeTraitEvaluator, TypeTraitResult};

use fragile_ast_exporter::clang_ast::AstContext;
use miette::Result;
use std::path::Path;

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
    pub ignored_error_patterns: Vec<String>,
    pub backend: ParserBackend,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            include_paths: Vec::new(),
            defines: Vec::new(),
            language: ParserLanguage::Cpp,
            ignored_error_patterns: Vec::new(),
            backend: ParserBackend::Libclang,
        }
    }
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
            extra_args.push("-std=c++20".to_string());
        }
        ParserLanguage::C => {
            extra_args.push("-x".to_string());
            extra_args.push("c".to_string());
            extra_args.push("-std=gnu11".to_string());
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
    parser
}

fn parse_libtooling_context(path: &Path, options: &TranspileOptions) -> Result<AstContext> {
    let parser = libtooling_parser_for_path(path, options);
    parser.parse_file(path)
}

fn translation_unit_from_libtooling_context(ctx: &AstContext) -> ClangNode {
    let children = ctx
        .top_nodes
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
    let mut codegen = AstCodeGen::new();
    let translation_unit = match options.backend {
        ParserBackend::Libclang => {
            let parser = parser_for_path_with_options(path, options)?;
            let ast = parser.parse_file(path)?;
            ast.translation_unit
        }
        ParserBackend::Hybrid => {
            let parser = parser_for_path_with_options(path, options)?;
            let ast = parser.parse_file(path)?;
            let ctx = parse_libtooling_context(path, options)?;
            apply_libtooling_enrichment(&mut codegen, &ctx);
            ast.translation_unit
        }
        ParserBackend::Libtooling => {
            let ctx = parse_libtooling_context(path, options)?;
            apply_libtooling_enrichment(&mut codegen, &ctx);
            translation_unit_from_libtooling_context(&ctx)
        }
    };
    Ok(codegen.generate(&translation_unit))
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
