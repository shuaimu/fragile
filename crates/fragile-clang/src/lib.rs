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
mod parse;
mod types;

pub use ast::{
    AccessSpecifier, BinaryOp, ClangAst, ClangNode, ClangNodeKind, ConstructorKind, Requirement,
    TypeTraitKind, UnaryOp,
};
pub use ast_codegen::AstCodeGen;
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
