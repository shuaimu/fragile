use fragile_ast_exporter::{export_ast, get_clang_version, ASTEntryTag};
use std::path::Path;

#[test]
fn test_clang_version_exists() {
    let version = get_clang_version();
    assert!(version.is_some(), "Should get Clang version");
    let version = version.unwrap();
    println!("Clang version: {}", version);
    assert!(!version.is_empty());
}

#[test]
fn test_template_instantiation_export() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let test_file = test_dir.join("test_template.cpp");

    // Ensure the test file exists
    assert!(test_file.exists(), "Test file should exist: {:?}", test_file);

    // Export the AST
    let result = export_ast(&test_file, &test_dir, &[], false);

    match result {
        Ok(ctx) => {
            println!("AST export successful!");
            println!("Number of AST nodes: {}", ctx.ast_nodes.len());
            println!("Number of type nodes: {}", ctx.type_nodes.len());
            println!("Number of top-level nodes: {}", ctx.top_nodes.len());
            println!("Number of files: {}", ctx.files.len());

            // Check that we have some nodes
            assert!(!ctx.ast_nodes.is_empty(), "Should have AST nodes");

            // Look for template instantiation-related nodes
            let interesting_tags = [
                ASTEntryTag::TagFunctionDecl,
                ASTEntryTag::TagCXXMethodDecl,
                ASTEntryTag::TagCXXRecordDecl,
                ASTEntryTag::TagClassTemplateDecl,
                ASTEntryTag::TagClassTemplateSpecializationDecl,
            ];

            // Count how many "double_value" methods we find (should be multiple instantiations)
            let mut double_value_methods = vec![];
            let mut get_methods = vec![];
            let mut set_methods = vec![];

            for (id, node) in &ctx.ast_nodes {
                if node.tag == ASTEntryTag::TagCXXMethodDecl {
                    let name = node.get_string(0).unwrap_or("");
                    match name {
                        "double_value" => double_value_methods.push(*id),
                        "get" => get_methods.push(*id),
                        "set" => set_methods.push(*id),
                        _ => {}
                    }
                }
            }

            println!("\nFound {} double_value methods", double_value_methods.len());
            println!("Found {} get methods", get_methods.len());
            println!("Found {} set methods", set_methods.len());

            // We should have at least 2 instantiations of each method
            // (one for Container<int>, one for Container<double>, plus the template definition)
            assert!(double_value_methods.len() >= 2,
                "Should have at least 2 double_value methods, found {}",
                double_value_methods.len());

            // Check that instantiated methods have body children
            // The key difference from libclang is that LibTooling gives us actual method bodies
            for method_id in &double_value_methods {
                let node = ctx.ast_nodes.get(method_id).unwrap();
                println!("\nMethod double_value (id {:x}):", method_id);
                println!("  Children: {:?}", node.children);

                // Method should have at least one child (the body)
                // In template instantiations, the body should be non-null
                if !node.children.is_empty() {
                    if let Some(body_id) = node.children.get(0).and_then(|c| *c) {
                        if let Some(body) = ctx.ast_nodes.get(&body_id) {
                            println!("  Body node tag: {:?}", body.tag);
                            println!("  Body has {} children", body.children.len());

                            // The body should be a CompoundStmt and have children (the return statement)
                            assert_eq!(body.tag, ASTEntryTag::TagCompoundStmt,
                                "Method body should be a CompoundStmt");
                            assert!(!body.children.is_empty(),
                                "Method body should have statements");
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
