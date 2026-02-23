use fragile_ast_exporter::{export_ast, ASTEntryTag};
use std::collections::HashMap;
use std::path::Path;

#[test]
fn test_analyze_method_children_structure() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let test_file = test_dir.join("test_map.cpp");

    assert!(
        test_file.exists(),
        "Test file should exist: {:?}",
        test_file
    );

    let result = export_ast(&test_file, &test_dir, &[], false);

    match result {
        Ok(ctx) => {
            println!("\n=== Analyzing method children structure ===\n");

            // Focus on interesting map methods
            let interesting = [
                "find",
                "operator[]",
                "insert",
                "at",
                "erase",
                "size",
                "empty",
                "clear",
            ];

            for method_name in &interesting {
                let methods: Vec<_> = ctx
                    .ast_nodes
                    .iter()
                    .filter(|(_, n)| n.tag == ASTEntryTag::TagCXXMethodDecl)
                    .filter(|(_, n)| n.get_string(0).unwrap_or("") == *method_name)
                    .collect();

                if methods.is_empty() {
                    continue;
                }

                println!("\n=== {} ({} instances) ===", method_name, methods.len());

                // Check if ANY child is a CompoundStmt
                let with_body: Vec<_> = methods
                    .iter()
                    .filter(|(_, n)| {
                        n.children.iter().any(|c| {
                            c.and_then(|id| ctx.ast_nodes.get(&id))
                                .map(|node| node.tag == ASTEntryTag::TagCompoundStmt)
                                .unwrap_or(false)
                        })
                    })
                    .collect();

                let without_body: Vec<_> = methods
                    .iter()
                    .filter(|(_, n)| {
                        !n.children.iter().any(|c| {
                            c.and_then(|id| ctx.ast_nodes.get(&id))
                                .map(|node| node.tag == ASTEntryTag::TagCompoundStmt)
                                .unwrap_or(false)
                        })
                    })
                    .collect();

                println!("  With body (CompoundStmt anywhere): {}", with_body.len());
                println!("  Without body: {}", without_body.len());

                // Show the structure of methods WITH bodies
                if !with_body.is_empty() {
                    println!("\n  Example with body:");
                    let (id, node) = with_body[0];
                    println!("    ID {:x}, {} children:", id, node.children.len());
                    for (i, child_opt) in node.children.iter().enumerate() {
                        if let Some(child_id) = child_opt {
                            if let Some(child_node) = ctx.ast_nodes.get(child_id) {
                                let extra = match child_node.tag {
                                    ASTEntryTag::TagParmVarDecl => {
                                        format!(
                                            " name=\"{}\"",
                                            child_node.get_string(0).unwrap_or("?")
                                        )
                                    }
                                    ASTEntryTag::TagCompoundStmt => {
                                        format!(
                                            " (THIS IS THE BODY, {} statements)",
                                            child_node.children.len()
                                        )
                                    }
                                    _ => String::new(),
                                };
                                println!("      [{}] {:?}{}", i, child_node.tag, extra);
                            }
                        } else {
                            println!("      [{}] None", i);
                        }
                    }
                }

                // Show the structure of methods WITHOUT bodies
                if !without_body.is_empty() {
                    println!("\n  Example without body:");
                    let (id, node) = without_body[0];
                    println!("    ID {:x}, {} children:", id, node.children.len());
                    for (i, child_opt) in node.children.iter().enumerate() {
                        if let Some(child_id) = child_opt {
                            if let Some(child_node) = ctx.ast_nodes.get(child_id) {
                                let extra = match child_node.tag {
                                    ASTEntryTag::TagParmVarDecl => {
                                        format!(
                                            " name=\"{}\"",
                                            child_node.get_string(0).unwrap_or("?")
                                        )
                                    }
                                    _ => String::new(),
                                };
                                println!("      [{}] {:?}{}", i, child_node.tag, extra);
                            }
                        } else {
                            println!("      [{}] None (declaration without definition)", i);
                        }
                    }
                }
            }

            // Overall statistics
            println!("\n\n=== Overall Summary ===");
            let total_methods = ctx
                .ast_nodes
                .values()
                .filter(|n| n.tag == ASTEntryTag::TagCXXMethodDecl)
                .count();

            let methods_with_body = ctx
                .ast_nodes
                .values()
                .filter(|n| n.tag == ASTEntryTag::TagCXXMethodDecl)
                .filter(|n| {
                    n.children.iter().any(|c| {
                        c.and_then(|id| ctx.ast_nodes.get(&id))
                            .map(|node| node.tag == ASTEntryTag::TagCompoundStmt)
                            .unwrap_or(false)
                    })
                })
                .count();

            println!("Total method declarations: {}", total_methods);
            println!(
                "Methods with CompoundStmt body (anywhere in children): {}",
                methods_with_body
            );
            println!(
                "Methods without body (declarations only): {}",
                total_methods - methods_with_body
            );
            println!(
                "Percentage with body: {:.1}%",
                100.0 * methods_with_body as f64 / total_methods as f64
            );
        }
        Err(e) => {
            panic!("AST export failed: {}", e);
        }
    }
}
