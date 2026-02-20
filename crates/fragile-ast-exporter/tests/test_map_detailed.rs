use fragile_ast_exporter::{export_ast, ASTEntryTag};
use std::collections::HashSet;
use std::path::Path;

#[test]
fn test_map_method_body_details() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let test_file = test_dir.join("test_map.cpp");

    assert!(test_file.exists(), "Test file should exist: {:?}", test_file);

    let result = export_ast(&test_file, &test_dir, &[], false);

    match result {
        Ok(ctx) => {
            println!("\n=== Inspecting method bodies ===\n");

            // Find a size() method with a body and inspect it
            for (id, node) in &ctx.ast_nodes {
                if node.tag == ASTEntryTag::TagCXXMethodDecl {
                    let name = node.get_string(0).unwrap_or("");
                    if name == "size" && !node.children.is_empty() {
                        if let Some(body_id) = node.children.first().and_then(|c| *c) {
                            if let Some(body) = ctx.ast_nodes.get(&body_id) {
                                if body.tag == ASTEntryTag::TagCompoundStmt {
                                    println!("=== size() method body at {:x} ===", id);
                                    let mut visited = HashSet::new();
                                    print_node_tree(&ctx, body_id, 0, &mut visited);
                                    println!();
                                    // Just show one example
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // Find an empty() method with a body
            for (id, node) in &ctx.ast_nodes {
                if node.tag == ASTEntryTag::TagCXXMethodDecl {
                    let name = node.get_string(0).unwrap_or("");
                    if name == "empty" && !node.children.is_empty() {
                        if let Some(body_id) = node.children.first().and_then(|c| *c) {
                            if let Some(body) = ctx.ast_nodes.get(&body_id) {
                                if body.tag == ASTEntryTag::TagCompoundStmt {
                                    println!("=== empty() method body at {:x} ===", id);
                                    let mut visited = HashSet::new();
                                    print_node_tree(&ctx, body_id, 0, &mut visited);
                                    println!();
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // Find a clear() method with a body
            for (id, node) in &ctx.ast_nodes {
                if node.tag == ASTEntryTag::TagCXXMethodDecl {
                    let name = node.get_string(0).unwrap_or("");
                    if name == "clear" && !node.children.is_empty() {
                        if let Some(body_id) = node.children.first().and_then(|c| *c) {
                            if let Some(body) = ctx.ast_nodes.get(&body_id) {
                                if body.tag == ASTEntryTag::TagCompoundStmt {
                                    println!("=== clear() method body at {:x} ===", id);
                                    let mut visited = HashSet::new();
                                    print_node_tree(&ctx, body_id, 0, &mut visited);
                                    println!();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            panic!("AST export failed: {}", e);
        }
    }
}

fn print_node_tree(
    ctx: &fragile_ast_exporter::clang_ast::AstContext,
    node_id: u64,
    depth: usize,
    visited: &mut HashSet<u64>,
) {
    let indent = "  ".repeat(depth);
    if depth > 128 {
        println!("{}<max-depth {} reached>", indent, node_id);
        return;
    }
    if !visited.insert(node_id) {
        println!("{}<cycle {}>", indent, node_id);
        return;
    }
    if let Some(node) = ctx.ast_nodes.get(&node_id) {
        let extra_info = match node.tag {
            ASTEntryTag::TagDeclRefExpr => {
                format!(" name=\"{}\"", node.get_string(0).unwrap_or("?"))
            }
            ASTEntryTag::TagMemberExpr => {
                format!(" member=\"{}\"", node.get_string(0).unwrap_or("?"))
            }
            ASTEntryTag::TagIntegerLiteral => {
                format!(" value={}", node.get_int(0).unwrap_or(0))
            }
            ASTEntryTag::TagCXXMethodDecl | ASTEntryTag::TagFunctionDecl => {
                format!(" name=\"{}\"", node.get_string(0).unwrap_or("?"))
            }
            ASTEntryTag::TagBinaryOperator => {
                // The operator kind is usually in extras
                format!(" op={:?}", node.extras.first())
            }
            _ => String::new()
        };

        println!("{}{:?}{}", indent, node.tag, extra_info);

        for child_opt in &node.children {
            if let Some(child_id) = child_opt {
                print_node_tree(ctx, *child_id, depth + 1, visited);
            }
        }
    }
    visited.remove(&node_id);
}
