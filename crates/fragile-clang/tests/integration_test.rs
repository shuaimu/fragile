//! Integration tests for Clang AST parsing and MIR conversion.

use fragile_clang::{ClangParser, MirConverter, CppType};

/// Test parsing a simple add function.
#[test]
fn test_parse_add_function() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int add(int a, int b) {
            return a + b;
        }
    "#;

    let ast = parser.parse_string(source, "add.cpp").expect("Failed to parse");

    // Check that we got a translation unit
    assert!(matches!(
        ast.translation_unit.kind,
        fragile_clang::ClangNodeKind::TranslationUnit
    ));

    // Should have at least one child (the function)
    assert!(!ast.translation_unit.children.is_empty());
}

/// Test converting a simple add function to MIR.
#[test]
fn test_convert_add_function() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int add(int a, int b) {
            return a + b;
        }
    "#;

    let ast = parser.parse_string(source, "add.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have one function
    assert_eq!(module.functions.len(), 1);

    let func = &module.functions[0];
    assert_eq!(func.display_name, "add");
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.params[0].0, "a");
    assert_eq!(func.params[1].0, "b");

    // Return type should be int
    assert_eq!(func.return_type, CppType::int());

    // MIR body should have at least one basic block
    assert!(!func.mir_body.blocks.is_empty());
}

/// Test generating Rust stubs for C++ code.
#[test]
fn test_generate_stubs() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        struct Point {
            int x;
            int y;
        };
    "#;

    let ast = parser.parse_string(source, "test.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let stubs = fragile_rustc_driver::generate_rust_stubs(&[module]);

    // Check that the stubs contain the function declaration
    assert!(stubs.contains("fn add"));
    assert!(stubs.contains("a: i32"));
    assert!(stubs.contains("b: i32"));
    assert!(stubs.contains("-> i32"));

    // Check struct definition (if struct was parsed)
    // Note: struct parsing may not fully work yet
}

/// Test the full end-to-end flow.
#[test]
fn test_end_to_end() {
    use fragile_rustc_driver::{FragileDriver, generate_rust_stubs};

    let parser = ClangParser::new().expect("Failed to create parser");

    // Parse C++ source
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int multiply(int x, int y) {
            return x * y;
        }
    "#;

    let ast = parser.parse_string(source, "math.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have two functions
    assert_eq!(module.functions.len(), 2);

    // Create driver and register module
    let driver = FragileDriver::new();
    driver.register_cpp_module(&module);

    // Generate stubs
    let stubs = generate_rust_stubs(&[module]);

    // Verify stubs contain both functions
    assert!(stubs.contains("fn add"));
    assert!(stubs.contains("fn multiply"));

    // Note: Actually running the driver requires nightly + rustc-dev
    // For now, we just verify the flow works up to stub generation
}

/// Test parsing and converting namespaced functions.
#[test]
fn test_namespace_function() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace rrr {
            int compute(int x) {
                return x * 2;
            }
        }
    "#;

    let ast = parser.parse_string(source, "ns.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have one function
    assert_eq!(module.functions.len(), 1);

    let func = &module.functions[0];
    assert_eq!(func.display_name, "compute");
    assert_eq!(func.namespace, vec!["rrr"]);
}

/// Test nested namespaces.
#[test]
fn test_nested_namespace() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace outer {
            namespace inner {
                int nested_func() {
                    return 42;
                }
            }
        }
    "#;

    let ast = parser.parse_string(source, "nested.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);

    let func = &module.functions[0];
    assert_eq!(func.display_name, "nested_func");
    assert_eq!(func.namespace, vec!["outer", "inner"]);
}

/// Test struct in namespace.
#[test]
fn test_namespace_struct() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace mako {
            struct Point {
                int x;
                int y;
            };
        }
    "#;

    let ast = parser.parse_string(source, "ns_struct.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.name, "Point");
    assert_eq!(s.namespace, vec!["mako"]);
    assert_eq!(s.fields.len(), 2);
}

/// Test anonymous namespace.
#[test]
fn test_anonymous_namespace() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace {
            int internal_func() {
                return 0;
            }
        }
    "#;

    let ast = parser.parse_string(source, "anon.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);

    let func = &module.functions[0];
    assert_eq!(func.display_name, "internal_func");
    // Anonymous namespace results in empty namespace path
    assert!(func.namespace.is_empty());
}
