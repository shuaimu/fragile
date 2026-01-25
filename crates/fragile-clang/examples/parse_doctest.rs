use fragile_clang::{ClangParser, ClangNode, ClangNodeKind};
use std::path::Path;

fn count_nodes(node: &ClangNode, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);
    match &node.kind {
        ClangNodeKind::TranslationUnit => {
            println!("{indent}TranslationUnit (children: {})", node.children.len());
        }
        ClangNodeKind::FunctionDecl { name, mangled_name, return_type, params, is_definition, .. } => {
            println!("{indent}FunctionDecl: {name} (mangled: {mangled_name:?})");
            println!("{indent}  return: {return_type:?}, params: {}", params.len());
            println!("{indent}  is_definition: {is_definition}");
        }
        ClangNodeKind::FunctionTemplateDecl { name, template_params, .. } => {
            println!("{indent}FunctionTemplateDecl: {}<{}>", name, template_params.join(", "));
        }
        ClangNodeKind::RecordDecl { name, is_class, is_definition, fields } => {
            let kind = if *is_class { "class" } else { "struct" };
            let def_marker = if *is_definition { " (definition)" } else { " (forward decl)" };
            println!("{indent}RecordDecl ({kind}): {name}{def_marker}");
            println!("{indent}  fields: {}", fields.len());
        }
        ClangNodeKind::ClassTemplateDecl { name, template_params, .. } => {
            println!("{indent}ClassTemplateDecl: {}<{}>", name, template_params.join(", "));
        }
        ClangNodeKind::NamespaceDecl { name } => {
            println!("{indent}NamespaceDecl: {}", name.as_deref().unwrap_or("(anonymous)"));
        }
        ClangNodeKind::Unknown(kind) => {
            if depth <= 2 {
                println!("{indent}Unknown: {kind}");
            }
        }
        _ => {
            // Skip detailed output for other node types
        }
    }

    for child in &node.children {
        count_nodes(child, depth + 1, max_depth);
    }
}

fn main() {
    let file_path = Path::new("tests/cpp/doctest_simple.cpp");

    let parser = ClangParser::new().expect("Failed to create parser");

    match parser.parse_file(file_path) {
        Ok(ast) => {
            println!("Successfully parsed {}", file_path.display());
            println!("\n=== AST Structure (max depth 3) ===\n");
            count_nodes(&ast.translation_unit, 0, 3);
        }
        Err(e) => println!("Parse error: {:?}", e),
    }
}
