//! Debug AST structure for constructor calls.

use fragile_clang::{ClangParser, ClangNode, ClangNodeKind};
use std::env;
use std::path::Path;

fn print_node(node: &ClangNode, indent: usize) {
    let prefix = "  ".repeat(indent);

    // Print node kind with key details
    match &node.kind {
        ClangNodeKind::Unknown(name) => {
            println!("{}Unknown(\"{}\")", prefix, name);
        }
        ClangNodeKind::FunctionDecl { name, .. } => {
            println!("{}FunctionDecl {{ name: \"{}\" }}", prefix, name);
        }
        ClangNodeKind::VarDecl { name, ty, has_init } => {
            println!("{}VarDecl {{ name: \"{}\", ty: {:?}, has_init: {} }}", prefix, name, ty, has_init);
        }
        ClangNodeKind::DeclStmt => {
            println!("{}DeclStmt", prefix);
        }
        ClangNodeKind::CallExpr { ty } => {
            println!("{}CallExpr {{ ty: {:?} }}", prefix, ty);
        }
        ClangNodeKind::MemberExpr { member_name, is_arrow, ty, declaring_class, is_static } => {
            println!("{}MemberExpr {{ member_name: \"{}\", is_arrow: {}, ty: {:?}, declaring_class: {:?}, is_static: {} }}", prefix, member_name, is_arrow, ty, declaring_class, is_static);
        }
        other => {
            println!("{}{:?}", prefix, other);
        }
    }

    // Recurse into children
    for child in &node.children {
        print_node(child, indent + 1);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <cpp_file>", args[0]);
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    let parser = ClangParser::new().expect("Failed to create parser");

    match parser.parse_file(path) {
        Ok(ast) => {
            println!("AST for {}:", path.display());
            println!("================");
            print_node(&ast.translation_unit, 0);
        }
        Err(e) => {
            eprintln!("Error parsing {}: {:?}", path.display(), e);
            std::process::exit(1);
        }
    }
}
