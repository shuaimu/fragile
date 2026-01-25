use fragile_clang::ClangParser;

fn main() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser
        .parse_string(source, "template.cpp")
        .expect("Failed to parse");

    // Print all function template children to see params
    fn visit(node: &fragile_clang::ClangNode, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}{:?}", indent, node.kind);
        for child in &node.children {
            visit(child, depth + 1);
        }
    }

    for child in &ast.translation_unit.children {
        if let fragile_clang::ClangNodeKind::FunctionTemplateDecl { .. } = &child.kind {
            visit(child, 0);
        }
    }
}
