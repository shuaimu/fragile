use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set by Cargo"),
    );
    let cpp_source = manifest_dir.join("cpp/demo.cc");
    println!("cargo:rerun-if-changed={}", cpp_source.display());

    let generated_rust = fragile_clang::transpile_cpp_to_rust(&cpp_source)
        .expect("failed to transpile C++ demo source to Rust");
    let generated_rust = strip_leading_inner_allow_attrs(&generated_rust);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set by Cargo"));
    let generated_path = out_dir.join("fragile_demo_cpp.rs");
    fs::write(&generated_path, generated_rust).expect("failed to write generated Rust source");
}

fn strip_leading_inner_allow_attrs(source: &str) -> String {
    let mut output = String::new();
    let mut skipping_prefix = true;

    for line in source.lines() {
        if skipping_prefix {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("#![allow(") && trimmed.ends_with(']') {
                continue;
            }
            skipping_prefix = false;
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}
