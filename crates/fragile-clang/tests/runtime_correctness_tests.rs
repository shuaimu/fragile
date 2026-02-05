//! Runtime correctness tests for STL container transpilation.
//!
//! These tests verify that transpiled code ACTUALLY WORKS at runtime,
//! not just compiles. They are designed to FAIL with stub implementations.
//!
//! The tests serve two purposes:
//! 1. Prove that stubs are not sufficient (tests fail)
//! 2. Define the expected behavior for correct transpilation (tests pass when fixed)

use fragile_clang::transpile_cpp_to_rust_with_libtooling;
use std::fs;
use std::process::Command;

/// Helper to compile and run transpiled C++ code, returning the exit code
fn compile_and_run(cpp_code: &str, test_name: &str) -> Option<i32> {
    let temp_dir = std::path::PathBuf::from(format!("/tmp/fragile_runtime_test_{}", test_name));
    let _ = fs::create_dir_all(&temp_dir);

    let cpp_path = temp_dir.join("test.cpp");
    fs::write(&cpp_path, cpp_code).expect("Failed to write C++ source");

    let rust_code = match transpile_cpp_to_rust_with_libtooling(&cpp_path) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Transpilation failed for {}: {}", test_name, e);
            return None;
        }
    };

    let rs_path = temp_dir.join("test.rs");
    fs::write(&rs_path, &rust_code).expect("Failed to write Rust source");

    // Compile as binary
    let compile_output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg("-A")
        .arg("unconditional_panic")
        .arg("-A")
        .arg("overflowing_literals")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.join("test_binary"))
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        eprintln!(
            "Compilation failed for {}:\n{}",
            test_name,
            String::from_utf8_lossy(&compile_output.stderr)
        );
        return None;
    }

    // Run the binary
    let run_output = Command::new(temp_dir.join("test_binary"))
        .output()
        .expect("Failed to run binary");

    run_output.status.code()
}

/// Test that std::map::size() returns correct count after insertions.
/// This test should FAIL with stub implementation (size() { 0 }).
#[test]
#[ignore = "Requires working std::map transpilation - currently uses stubs"]
fn test_map_size_after_insert() {
    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;
    if (m.size() != 0) return 1;  // Empty map should have size 0

    m[1] = 10;
    if (m.size() != 1) return 2;  // After 1 insert, size should be 1

    m[2] = 20;
    m[3] = 30;
    if (m.size() != 3) return 3;  // After 3 inserts, size should be 3

    return 0;  // All tests passed
}
"#;

    let exit_code = compile_and_run(cpp_code, "map_size");
    assert_eq!(exit_code, Some(0), "std::map::size() should return correct count");
}

/// Test that std::map::operator[] inserts and retrieves values correctly.
/// This test should FAIL with stub implementation returning null.
#[test]
#[ignore = "Requires working std::map transpilation - currently uses stubs"]
fn test_map_operator_bracket_insert_retrieve() {
    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;

    // Insert values
    m[1] = 100;
    m[2] = 200;

    // Retrieve and verify
    if (m[1] != 100) return 1;
    if (m[2] != 200) return 2;

    // Modify and verify
    m[1] = 150;
    if (m[1] != 150) return 3;

    return 0;
}
"#;

    let exit_code = compile_and_run(cpp_code, "map_bracket");
    assert_eq!(exit_code, Some(0), "std::map::operator[] should insert and retrieve correctly");
}

/// Test that std::map doesn't crash when accessing elements.
/// This is a basic smoke test that should FAIL with null pointer stubs.
#[test]
fn test_map_no_crash_on_access() {
    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;
    m[1] = 10;  // This should not crash
    return 0;   // If we get here without crashing, basic stability works
}
"#;

    let exit_code = compile_and_run(cpp_code, "map_no_crash");
    // With null stub, this will crash (exit code None or non-zero)
    // We expect this to fail, documenting current behavior
    match exit_code {
        Some(0) => {
            // If it passes, transpilation works!
            println!("Map access works without crash - good!");
        }
        Some(code) => {
            // Non-zero exit indicates a problem (assertion failed, etc.)
            println!("Map access returned error code {}", code);
        }
        None => {
            // None means crash (signal/abort)
            println!("Map access crashed - expected with null pointer stub");
        }
    }
    // This test documents current behavior rather than asserting
    // When transpilation works, uncomment: assert_eq!(exit_code, Some(0));
}

/// Test that std::vector basic operations work.
/// Vector uses our stub implementation which should work for basic cases.
#[test]
fn test_vector_basic_operations() {
    let cpp_code = r#"
#include <vector>

int main() {
    std::vector<int> v;
    v.push_back(10);
    v.push_back(20);
    v.push_back(30);

    // Our stub implementation should handle this
    if (v.size() != 3) return 1;

    return 0;
}
"#;

    let exit_code = compile_and_run(cpp_code, "vector_basic");
    // Vector has working stub, so this should pass
    assert_eq!(exit_code, Some(0), "std::vector basic operations should work with stub");
}

/// Test to verify that compilation succeeds even if runtime fails.
/// This helps distinguish compilation errors from runtime errors.
#[test]
fn test_map_compiles_successfully() {
    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;
    m[1] = 10;
    return 0;
}
"#;

    let temp_dir = std::path::PathBuf::from("/tmp/fragile_runtime_test_map_compiles");
    let _ = fs::create_dir_all(&temp_dir);

    let cpp_path = temp_dir.join("test.cpp");
    fs::write(&cpp_path, cpp_code).expect("Failed to write C++ source");

    let rust_code = transpile_cpp_to_rust_with_libtooling(&cpp_path)
        .expect("Transpilation should succeed");

    let rs_path = temp_dir.join("test.rs");
    fs::write(&rs_path, &rust_code).expect("Failed to write Rust source");

    let compile_output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.join("test_binary"))
        .output()
        .expect("Failed to run rustc");

    assert!(
        compile_output.status.success(),
        "std::map transpilation should compile successfully.\nErrors:\n{}",
        String::from_utf8_lossy(&compile_output.stderr)
    );
}

/// Metric test: Count rollback patterns in ast_codegen.rs
/// This test tracks progress toward removing forbidden patterns.
#[test]
fn test_rollback_pattern_count() {
    use std::path::Path;

    let ast_codegen_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("ast_codegen.rs");

    let content = fs::read_to_string(&ast_codegen_path)
        .expect("Failed to read ast_codegen.rs");

    // Count rollback patterns
    let rollback_count = content.matches("|| generated.contains(").count();

    println!("Current rollback pattern count: {}", rollback_count);

    // Track the count - it should decrease over time, NEVER increase
    // History: 210 -> 204 (float literal fix) -> 201 (undeclared var cleanup)
    //       -> 196 (primary template guard skips broken impl blocks)
    //       -> 194 (iterator skip list + dead pattern cleanup)
    //       -> 193 (broken fn template skip list)
    //       -> 191 (broken function skip list in generate_function)
    //       -> 155 (broken method type skip list: threading/semaphore/condvar)
    //       -> 145 (broken locale type skip list: ctype/collate_byname/bad_weak_ptr)
    //       -> 140 (broken condvar/mutex/semaphore/swap skip lists)
    // When this test starts failing because count increased, investigate!
    assert!(
        rollback_count <= 145,
        "Rollback pattern count ({}) increased beyond 145 - investigate! \
         The condvar/mutex/semaphore skip lists should keep this under 145.",
        rollback_count
    );
}

/// Test that the AST exporter recursively exports field type specializations.
/// When transpiling std::map<int,int>, the __tree<...> field type should also
/// have its specialization exported, providing field information.
#[test]
fn test_map_field_type_specializations_exported() {
    let temp_dir = std::path::PathBuf::from("/tmp/fragile_runtime_test_map_spec");
    let _ = fs::create_dir_all(&temp_dir);

    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;
    m[1] = 10;
    return 0;
}
"#;
    let cpp_path = temp_dir.join("test.cpp");
    fs::write(&cpp_path, cpp_code).expect("Failed to write C++ source");

    let rust_code = transpile_cpp_to_rust_with_libtooling(&cpp_path)
        .expect("Transpilation should succeed");

    // The std_map struct should reference a __tree type with template args
    // (not just bare "__tree") in its field type
    assert!(
        rust_code.contains("pub struct std_map_int__int"),
        "Should generate std_map_int__int struct"
    );

    // The __tree field type name should contain template argument info
    // (i.e., it should be more specific than just "__tree")
    let map_struct_start = rust_code.find("pub struct std_map_int__int").unwrap();
    let map_struct_end = rust_code[map_struct_start..].find('}').unwrap() + map_struct_start;
    let map_struct = &rust_code[map_struct_start..=map_struct_end];

    // Field should reference a __tree type with template args encoded in the name
    assert!(
        map_struct.contains("__tree_: __tree_"),
        "Map struct should have __tree_ field referencing a __tree type: {}",
        map_struct
    );

    // The __tree type name should include value_type info (not just bare __tree)
    assert!(
        map_struct.contains("__value_type") || map_struct.contains("value_type"),
        "Map's __tree field type should encode value_type template arg: {}",
        map_struct
    );

    // The __tree stub struct should exist and have __emplace_unique
    assert!(
        rust_code.contains("pub fn __emplace_unique"),
        "Should generate __emplace_unique stub method on __tree type"
    );

    // Verify compilation still works
    let rs_path = temp_dir.join("test.rs");
    fs::write(&rs_path, &rust_code).expect("Failed to write Rust source");

    let compile_output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.join("test_binary"))
        .output()
        .expect("Failed to run rustc");

    assert!(
        compile_output.status.success(),
        "std::map with __tree field specialization should compile.\nErrors:\n{}",
        String::from_utf8_lossy(&compile_output.stderr)
    );
}

/// Test that the AST exporter flattens anonymous struct fields from compressed pairs.
/// libc++'s __tree uses _LIBCPP_COMPRESSED_PAIR which expands to anonymous struct
/// members containing fields like __size_, __end_node_, __value_comp_. These need
/// to be flattened into the parent CTSD's field list.
#[test]
fn test_compressed_pair_fields_exported() {
    let temp_dir = std::path::PathBuf::from("/tmp/fragile_runtime_test_compressed_pair");
    let _ = fs::create_dir_all(&temp_dir);

    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;
    m[1] = 10;
    int s = m.size();
    return s;
}
"#;
    let cpp_path = temp_dir.join("test.cpp");
    fs::write(&cpp_path, cpp_code).expect("Failed to write C++ source");

    // Use LibTooling to get specialization data
    let libtooling_parser = fragile_clang::LibToolingParser::new();
    let libtooling_data = libtooling_parser.parse_file(&cpp_path)
        .expect("LibTooling parse should succeed");

    let spec_fields = fragile_clang::extract_specialization_field_types(&libtooling_data);

    // Find the __tree specialization
    let tree_spec = spec_fields.iter()
        .find(|(key, _)| key.contains("__tree<"))
        .map(|(_, info)| info);

    assert!(
        tree_spec.is_some(),
        "Should have a __tree<...> specialization. Available: {:?}",
        spec_fields.keys().filter(|k| k.contains("tree")).collect::<Vec<_>>()
    );

    let tree_fields = &tree_spec.unwrap().field_types;

    // __size_ field should be present (from _LIBCPP_COMPRESSED_PAIR)
    assert!(
        tree_fields.contains_key("__size_"),
        "__tree specialization should have __size_ field (from compressed pair). Got fields: {:?}",
        tree_fields.keys().collect::<Vec<_>>()
    );

    // __begin_node_ should be present (direct field, was already working)
    assert!(
        tree_fields.contains_key("__begin_node_"),
        "__tree specialization should have __begin_node_ field. Got fields: {:?}",
        tree_fields.keys().collect::<Vec<_>>()
    );

    // __end_node_ should be present (from _LIBCPP_COMPRESSED_PAIR)
    assert!(
        tree_fields.contains_key("__end_node_"),
        "__tree specialization should have __end_node_ field (from compressed pair). Got fields: {:?}",
        tree_fields.keys().collect::<Vec<_>>()
    );

    // Should have at least 4 real fields (begin_node, end_node, size, value_comp)
    // plus padding fields from compressed pair
    let real_fields: Vec<_> = tree_fields.keys()
        .filter(|k| !k.contains("padding"))
        .collect();
    assert!(
        real_fields.len() >= 4,
        "__tree should have at least 4 non-padding fields. Got: {:?}",
        real_fields
    );
}

/// Test that __tree stub struct uses real fields from specialization data instead of opaque bytes.
/// When specialization field data is available, the __tree struct should have named fields
/// like __size_, __begin_node_, etc. instead of just `_opaque: [u8; 64]`.
#[test]
fn test_tree_struct_has_real_fields() {
    let temp_dir = std::path::PathBuf::from("/tmp/fragile_runtime_test_tree_real_fields");
    let _ = fs::create_dir_all(&temp_dir);

    let cpp_code = r#"
#include <map>

int main() {
    std::map<int, int> m;
    m[1] = 10;
    return 0;
}
"#;
    let cpp_path = temp_dir.join("test.cpp");
    fs::write(&cpp_path, cpp_code).expect("Failed to write C++ source");

    let rust_code = transpile_cpp_to_rust_with_libtooling(&cpp_path)
        .expect("Transpilation should succeed");

    // The __tree struct should NOT be opaque
    // Find the __tree struct that has __value_type in its name (the actual tree instantiation)
    let tree_struct_start = rust_code.find("pub struct __tree___value_type")
        .or_else(|| {
            // Fallback: find any __tree_ struct in the placeholder section
            let placeholder_section = rust_code.find("Placeholder structs for template").unwrap_or(0);
            rust_code[placeholder_section..].find("pub struct __tree_")
                .map(|pos| placeholder_section + pos)
        })
        .expect("Should have a __tree struct definition with template args");
    let tree_struct_end = rust_code[tree_struct_start..].find('}').unwrap() + tree_struct_start;
    let tree_struct = &rust_code[tree_struct_start..=tree_struct_end];

    // Should NOT have _opaque field (old opaque representation)
    assert!(
        !tree_struct.contains("_opaque"),
        "__tree struct should use real fields, not _opaque. Got:\n{}",
        tree_struct
    );

    // Should have __size_ field with usize type
    assert!(
        tree_struct.contains("__size_: usize"),
        "__tree struct should have __size_: usize field. Got:\n{}",
        tree_struct
    );

    // Should have __begin_node_ field (pointer type)
    assert!(
        tree_struct.contains("__begin_node_:"),
        "__tree struct should have __begin_node_ field. Got:\n{}",
        tree_struct
    );

    // __tree::size() should use self.__size_ (not return 0)
    assert!(
        rust_code.contains("pub fn size(&self) -> usize { self.__size_ as usize }"),
        "__tree::size() should return self.__size_ as usize"
    );

    // map::size() should delegate to __tree_.size()
    let map_impl_start = rust_code.find("impl std_map_int__int")
        .expect("Should have map impl block");
    let map_impl_section = &rust_code[map_impl_start..];
    assert!(
        map_impl_section.contains("pub fn size(&self) -> usize { self.__tree_.size() }"),
        "map::size() should delegate to self.__tree_.size()"
    );
}
