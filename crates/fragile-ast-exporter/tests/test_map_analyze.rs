use fragile_ast_exporter::{export_ast, ASTEntryTag};
use std::collections::HashMap;
use std::path::Path;

#[test]
fn test_analyze_missing_bodies() {
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
            println!("\n=== Analyzing methods without CompoundStmt bodies ===\n");

            // Group methods by name and analyze their children
            let mut methods_by_name: HashMap<String, Vec<(u64, Vec<Option<u64>>)>> = HashMap::new();

            for (id, node) in &ctx.ast_nodes {
                if node.tag == ASTEntryTag::TagCXXMethodDecl {
                    let name = node.get_string(0).unwrap_or("<unknown>").to_string();
                    methods_by_name
                        .entry(name)
                        .or_default()
                        .push((*id, node.children.clone()));
                }
            }

            // Focus on interesting map methods
            let interesting = [
                "find",
                "operator[]",
                "insert",
                "at",
                "erase",
                "count",
                "lower_bound",
                "upper_bound",
            ];

            for method_name in &interesting {
                if let Some(methods) = methods_by_name.get(*method_name) {
                    println!("\n=== {} ({} instances) ===", method_name, methods.len());

                    // Analyze each instance
                    let mut with_compound_body = 0;
                    let mut with_other_body = 0;
                    let mut no_body = 0;
                    let mut body_types: HashMap<String, usize> = HashMap::new();

                    for (id, children) in methods {
                        if children.is_empty() {
                            no_body += 1;
                        } else if let Some(first_child) = children.first().and_then(|c| *c) {
                            if let Some(child_node) = ctx.ast_nodes.get(&first_child) {
                                let tag_str = format!("{:?}", child_node.tag);
                                *body_types.entry(tag_str.clone()).or_insert(0) += 1;

                                if child_node.tag == ASTEntryTag::TagCompoundStmt {
                                    with_compound_body += 1;
                                } else {
                                    with_other_body += 1;
                                }
                            } else {
                                no_body += 1;
                            }
                        } else {
                            // First child is None
                            no_body += 1;
                        }
                    }

                    println!("  With CompoundStmt body: {}", with_compound_body);
                    println!("  With other body type: {}", with_other_body);
                    println!("  No body/declaration only: {}", no_body);
                    println!("  Body types breakdown:");
                    for (tag, count) in &body_types {
                        println!("    {}: {}", tag, count);
                    }

                    // Show a few examples without CompoundStmt body
                    if with_other_body > 0 || no_body > 0 {
                        println!("\n  Examples without CompoundStmt:");
                        let mut shown = 0;
                        for (id, children) in methods {
                            if shown >= 3 {
                                break;
                            }

                            let has_compound = children
                                .first()
                                .and_then(|c| *c)
                                .and_then(|child_id| ctx.ast_nodes.get(&child_id))
                                .map(|n| n.tag == ASTEntryTag::TagCompoundStmt)
                                .unwrap_or(false);

                            if !has_compound {
                                println!("    ID {:x}:", id);
                                println!("      Children count: {}", children.len());

                                // Show what the children are
                                for (i, child_opt) in children.iter().enumerate() {
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
                                            println!(
                                                "      Child {}: {:?}{}",
                                                i, child_node.tag, extra
                                            );
                                        }
                                    } else {
                                        println!("      Child {}: None (null body)", i);
                                    }
                                }
                                shown += 1;
                            }
                        }
                    }
                }
            }

            // Summary statistics
            println!("\n\n=== Overall Summary ===");
            let total_methods = ctx
                .ast_nodes
                .values()
                .filter(|n| n.tag == ASTEntryTag::TagCXXMethodDecl)
                .count();

            let methods_with_compound_body = ctx
                .ast_nodes
                .iter()
                .filter(|(_, n)| n.tag == ASTEntryTag::TagCXXMethodDecl)
                .filter(|(_, n)| {
                    n.children
                        .first()
                        .and_then(|c| *c)
                        .and_then(|child_id| ctx.ast_nodes.get(&child_id))
                        .map(|body| body.tag == ASTEntryTag::TagCompoundStmt)
                        .unwrap_or(false)
                })
                .count();

            let methods_declaration_only = ctx
                .ast_nodes
                .iter()
                .filter(|(_, n)| n.tag == ASTEntryTag::TagCXXMethodDecl)
                .filter(|(_, n)| {
                    n.children.is_empty() || n.children.first().and_then(|c| *c).is_none()
                })
                .count();

            println!("Total method declarations: {}", total_methods);
            println!(
                "Methods with CompoundStmt body: {}",
                methods_with_compound_body
            );
            println!(
                "Methods that are declaration-only (no body): {}",
                methods_declaration_only
            );
            println!(
                "Methods with other body types: {}",
                total_methods - methods_with_compound_body - methods_declaration_only
            );
        }
        Err(e) => {
            panic!("AST export failed: {}", e);
        }
    }
}
