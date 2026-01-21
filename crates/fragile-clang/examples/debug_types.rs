use fragile_clang::{ClangParser};

fn main() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(source, "template.cpp").expect("Failed to parse");

    fn print_ast(node: &fragile_clang::ClangNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        println!("{}{:?}", prefix, node.kind);
        for child in &node.children {
            print_ast(child, indent + 1);
        }
    }
    print_ast(&ast.translation_unit, 0);
}
