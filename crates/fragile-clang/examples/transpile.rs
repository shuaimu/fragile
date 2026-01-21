//! Example: Transpile a C++ file to Rust.
//!
//! Usage: cargo run --example transpile -- path/to/file.cpp

use fragile_clang::transpile_cpp_to_rust;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <cpp_file>", args[0]);
        eprintln!();
        eprintln!("Example: cargo run --example transpile -- tests/cpp/add.cpp");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);

    match transpile_cpp_to_rust(path) {
        Ok(rust_code) => {
            println!("// Generated from: {}", path.display());
            println!("// ============================================");
            println!();
            println!("{}", rust_code);
        }
        Err(e) => {
            eprintln!("Error transpiling {}: {:?}", path.display(), e);
            std::process::exit(1);
        }
    }
}
