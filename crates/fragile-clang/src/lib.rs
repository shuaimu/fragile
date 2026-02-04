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
pub use libtooling::{LibToolingParser, TemplateMethodInstantiation, MethodInfo, convert_to_clang_node, extract_method_bodies, extract_method_bodies_with_params, extract_specialization_field_types, SpecializationFieldInfo};
pub use parse::ClangParser;
pub use types::{CppType, TypeProperties, TypeTraitEvaluator, TypeTraitResult};

use miette::Result;
use std::path::Path;

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
    let parser = ClangParser::new()?;
    let ast = parser.parse_file(path)?;
    Ok(AstCodeGen::new().generate(&ast.translation_unit))
}

/// Generate Rust stubs from a C++ source file.
///
/// Stubs are function signatures with placeholder bodies,
/// useful for FFI declarations.
pub fn generate_stubs(path: &Path) -> Result<String> {
    let parser = ClangParser::new()?;
    let ast = parser.parse_file(path)?;
    Ok(AstCodeGen::new().generate_stubs(&ast.translation_unit))
}

/// Parse a C++ source file and transpile to Rust source code with LibTooling support.
///
/// This version uses LibTooling to get actual template method bodies instead of
/// generating `todo!()` stubs. This is more expensive but produces more complete output.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use fragile_clang::transpile_cpp_to_rust_with_libtooling;
///
/// let rust_code = transpile_cpp_to_rust_with_libtooling(Path::new("example.cpp"))?;
/// println!("{}", rust_code);
/// ```
pub fn transpile_cpp_to_rust_with_libtooling(path: &Path) -> Result<String> {
    // Parse with libclang (fast, gives us structure)
    let parser = ClangParser::new()?;
    let ast = parser.parse_file(path)?;

    // Parse with LibTooling (slower, gives us template bodies)
    let libtooling_parser = LibToolingParser::new();
    let libtooling_ctx = libtooling_parser.parse_file(path)?;

    // Extract method bodies with parameter info from LibTooling AST
    let method_bodies = extract_method_bodies_with_params(&libtooling_ctx);

    // Extract resolved field types for template specializations
    let field_types = extract_specialization_field_types(&libtooling_ctx);

    // Generate code with LibTooling bodies and resolved field types
    let mut codegen = AstCodeGen::new();
    codegen.set_libtooling_bodies(method_bodies);
    codegen.set_specialization_field_types(field_types);
    Ok(codegen.generate(&ast.translation_unit))
}
