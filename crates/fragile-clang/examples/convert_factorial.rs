use fragile_clang::{ClangParser, MirConverter};
use std::path::Path;

fn main() {
    let file_path = Path::new("tests/cpp/factorial.cpp");

    let parser = ClangParser::new().expect("Failed to create parser");

    match parser.parse_file(file_path) {
        Ok(ast) => {
            println!("Successfully parsed {}", file_path.display());

            // Convert to MIR
            let mut converter = MirConverter::new();
            match converter.convert(ast) {
                Ok(module) => {
                    println!("\n=== Converted to CppModule ===");
                    println!("Functions: {}", module.functions.len());
                    for func in &module.functions {
                        println!("\n  Function: {}", func.display_name);
                        println!("    mangled: {}", func.mangled_name);
                        println!("    return_type: {:?}", func.return_type);
                        println!("    params: {:?}", func.params);
                        let body = &func.mir_body;
                        println!("    blocks: {}", body.blocks.len());
                        println!("    locals: {}", body.locals.len());
                        for (i, bb) in body.blocks.iter().enumerate() {
                            println!("    bb{}: {} statements, terminator: {}",
                                i, bb.statements.len(),
                                format!("{:?}", bb.terminator).chars().take(50).collect::<String>());
                        }
                    }
                }
                Err(e) => {
                    println!("Conversion error: {:?}", e);
                }
            }
        }
        Err(e) => println!("Parse error: {:?}", e),
    }
}
