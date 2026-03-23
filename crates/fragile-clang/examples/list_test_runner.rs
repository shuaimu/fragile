use fragile_clang::transpile_cpp_to_rust;
use std::fs;
use std::process::Command;

fn main() {
    let cpp_source = r#"
        #include <list>

        int main() {
            std::list<int> lst;
            lst.push_back(10);
            lst.push_back(20);
            lst.push_back(30);
            return lst.size() == 3 ? 0 : 1;
        }
    "#;

    println!("=== List Test ===");

    let temp_dir = std::path::Path::new("/tmp/fragile_list_test");
    let _ = fs::create_dir_all(&temp_dir);

    // Write C++ source to temp file for LibTooling
    let cpp_path = temp_dir.join("test_list.cpp");
    fs::write(&cpp_path, cpp_source).expect("Failed to write C++ source");

    // Use LibTooling for full template method body transpilation
    let rust_code = match transpile_cpp_to_rust(&cpp_path) {
        Ok(code) => code,
        Err(e) => {
            println!("Transpilation failed: {}", e);
            std::process::exit(1);
        }
    };
    println!("Generated {} bytes of Rust code", rust_code.len());

    let rs_path = temp_dir.join("test_list.rs");
    fs::write(&rs_path, &rust_code).expect("Failed to write");

    // Compile as library
    let lib_output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-type")
        .arg("lib")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.join("libtest_list.rlib"))
        .output()
        .expect("Failed to run rustc");

    let lib_stderr = String::from_utf8_lossy(&lib_output.stderr);
    let lib_errors = lib_stderr.matches("error[E").count();
    println!("Library compilation errors: {}", lib_errors);

    if lib_errors > 0 {
        let preview: String = lib_stderr.chars().take(5000).collect();
        println!("Errors:\n{}", preview);
        std::process::exit(1);
    }

    // Compile as binary
    let bin_output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("unconditional_panic")
        .arg("-A")
        .arg("overflowing_literals")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.join("test_list"))
        .output()
        .expect("Failed to run rustc");

    if !bin_output.status.success() {
        println!("Binary compilation failed:");
        println!("{}", String::from_utf8_lossy(&bin_output.stderr));
        std::process::exit(1);
    }

    println!("Compilation successful!");

    let run_output = Command::new(temp_dir.join("test_list"))
        .output()
        .expect("Failed to run binary");

    println!("Exit code: {:?}", run_output.status.code());
    if run_output.status.success() {
        println!("✓ List test PASSED!");
    } else {
        println!("✗ List test FAILED!");
        std::process::exit(1);
    }
}
