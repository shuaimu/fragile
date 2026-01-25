//! Debug example to understand virtual method call AST structure.

use fragile_clang::ClangParser;

fn print_ast(node: &fragile_clang::ClangNode, indent: usize) {
    let prefix = "  ".repeat(indent);
    println!("{}{:?}", prefix, node.kind);
    for child in &node.children {
        print_ast(child, indent + 1);
    }
}

fn main() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let code = r#"
        class Animal {
        public:
            virtual void speak() {}
            void non_virtual() {}
        };

        void test() {
            Animal a;
            a.speak();        // virtual call
            a.non_virtual();  // non-virtual call

            Animal* p = &a;
            p->speak();       // virtual call through pointer
        }
    "#;

    let ast = parser
        .parse_string(code, "test.cpp")
        .expect("Failed to parse");

    println!("=== Full AST ===");
    print_ast(&ast.translation_unit, 0);
}
