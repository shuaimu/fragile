
use fragile_clang::LibToolingParser;
use std::fs;

fn main() {
    let cpp_path = "/tmp/tmp9j3v38xp/test.cpp";
    let libtooling_parser = LibToolingParser::new();
    let libtooling_data = libtooling_parser.parse_file(std::path::Path::new(&cpp_path))
        .expect("LibTooling parse should succeed");

    let spec_fields = fragile_clang::extract_specialization_field_types(&libtooling_data);

    println!("=== ALL SPECIALIZATIONS ===");
    let mut keys: Vec<_> = spec_fields.keys().collect();
    keys.sort();
    for key in keys {
        println!("{}", key);
    }
    println!("=== END ===");
    println!("Total: {}", spec_fields.len());
}
