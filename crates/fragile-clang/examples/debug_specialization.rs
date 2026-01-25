use fragile_clang::{ClangNode, ClangParser};

fn main() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        // Primary template
        template<typename T>
        T identity(T x) { return x; }

        // Explicit specialization for int
        template<>
        int identity<int>(int x) { return x + 1; }
    "#;

    let ast = parser
        .parse_string(source, "spec.cpp")
        .expect("Failed to parse");

    // Print AST structure
    fn visit(node: &ClangNode, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}{:?}", indent, node.kind);
        for child in &node.children {
            visit(child, depth + 1);
        }
    }

    for child in &ast.translation_unit.children {
        visit(child, 0);
        println!("---");
    }
}
