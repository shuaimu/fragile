use fragile_clang::{ClangParser, MirConverter};

fn main() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <vector>

        void test() {
            std::vector<int> v;
            v.push_back(42);
        }
    "#;

    match parser.parse_string(code, "test.cpp") {
        Ok(ast) => {
            println!("AST parsed successfully");
            let converter = MirConverter::new();
            match converter.convert(ast) {
                Ok(module) => {
                    println!("Module converted successfully");
                    println!("Functions: {}", module.functions.len());
                    println!("Structs: {}", module.structs.len());
                    println!("Class templates: {}", module.class_templates.len());
                    println!("Type aliases: {}", module.type_aliases.len());
                }
                Err(e) => println!("Conversion failed: {}", e),
            }
        }
        Err(e) => println!("Parse failed: {}", e),
    }
}
