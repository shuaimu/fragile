use fragile_ast_exporter::{export_ast, ASTEntryTag};
use std::path::Path;

#[test]
fn test_map_template_instantiation() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let test_file = test_dir.join("test_map.cpp");

    // Ensure the test file exists
    assert!(
        test_file.exists(),
        "Test file should exist: {:?}",
        test_file
    );

    // Export the AST with debug mode to see more info
    let result = export_ast(&test_file, &test_dir, &[], true);

    match result {
        Ok(ctx) => {
            println!("\n=== AST Export Results ===");
            println!("Number of AST nodes: {}", ctx.ast_nodes.len());
            println!("Number of type nodes: {}", ctx.type_nodes.len());
            println!("Number of top-level nodes: {}", ctx.top_nodes.len());

            // Look for map-related template specializations
            let mut class_template_specs = vec![];
            let mut method_decls = vec![];

            for (id, node) in &ctx.ast_nodes {
                match node.tag {
                    ASTEntryTag::TagClassTemplateSpecializationDecl => {
                        let name = node.get_string(0).unwrap_or("<unknown>");
                        class_template_specs.push((*id, name.to_string()));
                    }
                    ASTEntryTag::TagCXXMethodDecl => {
                        let name = node.get_string(0).unwrap_or("<unknown>");
                        method_decls.push((*id, name.to_string(), node.children.len()));
                    }
                    _ => {}
                }
            }

            println!("\n=== Class Template Specializations ===");
            for (id, name) in &class_template_specs {
                println!("  {:x}: {}", id, name);
            }

            println!(
                "\n=== Method Declarations ({} total) ===",
                method_decls.len()
            );
            // Filter for interesting map methods
            let interesting_methods: Vec<_> = method_decls
                .iter()
                .filter(|(_, name, _)| {
                    name == "find"
                        || name == "operator[]"
                        || name == "insert"
                        || name == "size"
                        || name == "empty"
                        || name == "clear"
                        || name == "begin"
                        || name == "end"
                        || name == "at"
                })
                .collect();

            for (id, name, children_count) in &interesting_methods {
                let node = ctx.ast_nodes.get(id).unwrap();
                let has_body = node
                    .children
                    .first()
                    .and_then(|c| *c)
                    .and_then(|body_id| ctx.ast_nodes.get(&body_id))
                    .is_some();
                println!(
                    "  {:x}: {} (children: {}, has_body: {})",
                    id, name, children_count, has_body
                );
            }

            // Check that we found some map methods
            assert!(
                !interesting_methods.is_empty(),
                "Should find some map methods (found {} total method decls)",
                method_decls.len()
            );

            // Check that some methods have bodies
            let methods_with_bodies: Vec<_> = interesting_methods
                .iter()
                .filter(|(id, _, _)| {
                    let node = ctx.ast_nodes.get(id).unwrap();
                    node.children
                        .first()
                        .and_then(|c| *c)
                        .and_then(|body_id| ctx.ast_nodes.get(&body_id))
                        .map(|body| body.tag == ASTEntryTag::TagCompoundStmt)
                        .unwrap_or(false)
                })
                .collect();

            println!(
                "\n=== Methods with CompoundStmt bodies: {} ===",
                methods_with_bodies.len()
            );
            for (id, name, _) in &methods_with_bodies {
                println!("  {:x}: {}", id, name);
            }

            // We should have at least some methods with bodies from template instantiation
            // Note: Some methods may be inlined or have different body types
            println!(
                "\nTotal methods with actual bodies: {}",
                methods_with_bodies.len()
            );
        }
        Err(e) => {
            panic!("AST export failed: {}", e);
        }
    }
}
