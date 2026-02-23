use fragile_clang::{LibToolingParser, extract_method_bodies_with_params};
use std::path::Path;

fn main() {
    let test_file = Path::new("/tmp/test_map_operator.cpp");
    
    let parser = LibToolingParser::new();
    let ctx = parser.parse_file(test_file).expect("Failed to parse");
    
    let method_bodies = extract_method_bodies_with_params(&ctx);
    
    println!("=== Method bodies ===");
    for ((class, method), infos) in method_bodies.iter() {
        println!("Class: '{}', Method: '{}', Bodies: {}", class, method, infos.len());
        for info in infos {
            println!("  Params: {:?}", info.param_names);
            println!("  Body kind: {:?}", info.body.kind);
        }
    }
}
