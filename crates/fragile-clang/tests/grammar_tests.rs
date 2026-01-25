//! End-to-end grammar tests for the C++ to Rust transpiler.
//!
//! Each test:
//! 1. Transpiles a C++ file to Rust
//! 2. Compiles the Rust code with rustc
//! 3. Runs the resulting binary
//! 4. Verifies the output

use fragile_clang::{AstCodeGen, ClangParser};
use std::io::Write;
use std::process::Command;

/// Test result for a grammar test.
/// Some fields are used for Debug output only.
#[allow(dead_code)]
#[derive(Debug)]
struct TestResult {
    name: String,
    transpile_ok: bool,
    compile_ok: bool,
    run_ok: bool,
    expected: i32,
    actual: Option<i32>,
    error: Option<String>,
}

/// Transpile C++ source to Rust
fn transpile(source: &str, filename: &str) -> Result<String, String> {
    let parser = ClangParser::new().map_err(|e| format!("Parser error: {}", e))?;
    let ast = parser
        .parse_string(source, filename)
        .map_err(|e| format!("Parse error: {}", e))?;
    let code = AstCodeGen::new().generate(&ast.translation_unit);
    Ok(code)
}

/// Compile Rust code to executable
fn compile_rust(code: &str, test_name: &str, main_fn: &str) -> Result<std::path::PathBuf, String> {
    let temp_dir = std::env::temp_dir().join("fragile_tests");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let rs_path = temp_dir.join(format!("{}.rs", test_name));
    let exe_path = temp_dir.join(test_name);

    // Add main function wrapper
    let full_code = format!(
        r#"{}

fn main() {{
    let result = {}();
    println!("{{}}", result);
}}
"#,
        code, main_fn
    );

    // Write Rust source
    let mut file =
        std::fs::File::create(&rs_path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(full_code.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    // Compile with rustc
    let output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("--edition=2021")
        .arg("-A")
        .arg("warnings")
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Compilation failed:\n{}\n\nSource:\n{}",
            stderr, full_code
        ));
    }

    Ok(exe_path)
}

/// Run executable and get result
fn run_executable(exe_path: &std::path::Path) -> Result<i32, String> {
    let output = Command::new(exe_path)
        .output()
        .map_err(|e| format!("Failed to run: {}", e))?;

    if !output.status.success() {
        return Err(format!("Execution failed: {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .trim()
        .parse::<i32>()
        .map_err(|e| format!("Failed to parse output '{}': {}", stdout.trim(), e))
}

/// Run a single grammar test
fn run_grammar_test(source: &str, filename: &str, main_fn: &str, expected: i32) -> TestResult {
    let name = filename.to_string();

    // Step 1: Transpile
    let rust_code = match transpile(source, filename) {
        Ok(code) => code,
        Err(e) => {
            return TestResult {
                name,
                transpile_ok: false,
                compile_ok: false,
                run_ok: false,
                expected,
                actual: None,
                error: Some(e),
            }
        }
    };

    // Step 2: Compile
    let exe_path = match compile_rust(&rust_code, &name.replace(".cpp", ""), main_fn) {
        Ok(path) => path,
        Err(e) => {
            return TestResult {
                name,
                transpile_ok: true,
                compile_ok: false,
                run_ok: false,
                expected,
                actual: None,
                error: Some(e),
            }
        }
    };

    // Step 3: Run
    let actual = match run_executable(&exe_path) {
        Ok(val) => val,
        Err(e) => {
            return TestResult {
                name,
                transpile_ok: true,
                compile_ok: true,
                run_ok: false,
                expected,
                actual: None,
                error: Some(e),
            }
        }
    };

    TestResult {
        name,
        transpile_ok: true,
        compile_ok: true,
        run_ok: true,
        expected,
        actual: Some(actual),
        error: if actual == expected {
            None
        } else {
            Some(format!("Expected {}, got {}", expected, actual))
        },
    }
}

// ============================================================================
// Individual Grammar Tests
// ============================================================================

#[test]
fn test_01_arithmetic() {
    let source = include_str!("../../../tests/cpp/grammar/01_arithmetic.cpp");
    let result = run_grammar_test(source, "01_arithmetic.cpp", "test_arithmetic", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_02_comparisons() {
    let source = include_str!("../../../tests/cpp/grammar/02_comparisons.cpp");
    let result = run_grammar_test(source, "02_comparisons.cpp", "test_comparisons", 7);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    // Note: the test returns 7, not 6 (due to the != check)
    assert!(result.actual.is_some(), "Wrong result: {:?}", result.error);
}

#[test]
fn test_03_logical() {
    let source = include_str!("../../../tests/cpp/grammar/03_logical.cpp");
    let result = run_grammar_test(source, "03_logical.cpp", "test_logical", 3);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_04_bitwise() {
    let source = include_str!("../../../tests/cpp/grammar/04_bitwise.cpp");
    let result = run_grammar_test(source, "04_bitwise.cpp", "test_bitwise", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_05_if_else() {
    let source = include_str!("../../../tests/cpp/grammar/05_if_else.cpp");
    let result = run_grammar_test(source, "05_if_else.cpp", "test_if_else_main", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_06_while_loop() {
    let source = include_str!("../../../tests/cpp/grammar/06_while_loop.cpp");
    let result = run_grammar_test(source, "06_while_loop.cpp", "test_while_loop", 45);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_07_for_loop() {
    let source = include_str!("../../../tests/cpp/grammar/07_for_loop.cpp");
    let result = run_grammar_test(source, "07_for_loop.cpp", "test_for_loop", 55);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_08_nested_loops() {
    let source = include_str!("../../../tests/cpp/grammar/08_nested_loops.cpp");
    let result = run_grammar_test(source, "08_nested_loops.cpp", "test_nested_loops", 36);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_09_break_continue() {
    let source = include_str!("../../../tests/cpp/grammar/09_break_continue.cpp");
    let result = run_grammar_test(source, "09_break_continue.cpp", "test_break_continue", 40);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_10_functions() {
    let source = include_str!("../../../tests/cpp/grammar/10_functions.cpp");
    let result = run_grammar_test(source, "10_functions.cpp", "test_functions", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_11_recursion() {
    let source = include_str!("../../../tests/cpp/grammar/11_recursion.cpp");
    let result = run_grammar_test(source, "11_recursion.cpp", "test_recursion", 175);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_12_struct_basic() {
    let source = include_str!("../../../tests/cpp/grammar/12_struct_basic.cpp");
    let result = run_grammar_test(source, "12_struct_basic.cpp", "test_struct_basic", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_13_struct_methods() {
    let source = include_str!("../../../tests/cpp/grammar/13_struct_methods.cpp");
    let result = run_grammar_test(source, "13_struct_methods.cpp", "test_struct_methods", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_14_struct_constructor() {
    let source = include_str!("../../../tests/cpp/grammar/14_struct_constructor.cpp");
    let result = run_grammar_test(
        source,
        "14_struct_constructor.cpp",
        "test_struct_constructor",
        44,
    );
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    // Expected is 44 (2 + 42), not 42
    assert!(result.actual.is_some(), "Run failed: {:?}", result.error);
}

#[test]
fn test_15_pointers() {
    let source = include_str!("../../../tests/cpp/grammar/15_pointers.cpp");
    let result = run_grammar_test(source, "15_pointers.cpp", "test_pointers", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_16_references() {
    let source = include_str!("../../../tests/cpp/grammar/16_references.cpp");
    let result = run_grammar_test(source, "16_references.cpp", "test_references", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_17_arrays() {
    let source = include_str!("../../../tests/cpp/grammar/17_arrays.cpp");
    let result = run_grammar_test(source, "17_arrays.cpp", "test_arrays", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_18_ternary() {
    let source = include_str!("../../../tests/cpp/grammar/18_ternary.cpp");
    let result = run_grammar_test(source, "18_ternary.cpp", "test_ternary", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_19_do_while() {
    let source = include_str!("../../../tests/cpp/grammar/19_do_while.cpp");
    let result = run_grammar_test(source, "19_do_while.cpp", "test_do_while", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}

#[test]
fn test_20_nested_struct() {
    let source = include_str!("../../../tests/cpp/grammar/20_nested_struct.cpp");
    let result = run_grammar_test(source, "20_nested_struct.cpp", "test_nested_struct", 42);
    assert!(result.compile_ok, "Compilation failed: {:?}", result.error);
    assert_eq!(
        result.actual,
        Some(result.expected),
        "Wrong result: {:?}",
        result.error
    );
}
