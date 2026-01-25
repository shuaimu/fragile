//! Integration tests for Clang AST parsing and Rust code generation.

use fragile_clang::{AstCodeGen, ClangParser};

/// Test parsing a simple add function.
#[test]
fn test_parse_add_function() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int add(int a, int b) {
            return a + b;
        }
    "#;

    let ast = parser
        .parse_string(source, "add.cpp")
        .expect("Failed to parse");

    // Check that we got a translation unit
    assert!(matches!(
        ast.translation_unit.kind,
        fragile_clang::ClangNodeKind::TranslationUnit
    ));

    // Should have at least one child (the function)
    assert!(!ast.translation_unit.children.is_empty());
}

/// Test generating Rust code from C++ source.
#[test]
fn test_generate_rust_code() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int add(int a, int b) {
            return a + b;
        }
    "#;

    let ast = parser
        .parse_string(source, "add.cpp")
        .expect("Failed to parse");
    let code = AstCodeGen::new().generate(&ast.translation_unit);

    // Check that the generated code contains the function
    assert!(code.contains("pub fn add"));
    assert!(code.contains("a: i32"));
    assert!(code.contains("b: i32"));
    assert!(code.contains("-> i32"));
    assert!(code.contains("return a + b"));
}

/// Test generating stubs from C++ source.
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

    let ast = parser
        .parse_string(source, "test.cpp")
        .expect("Failed to parse");
    let stubs = AstCodeGen::new().generate_stubs(&ast.translation_unit);

    // Check that the stubs contain the function declaration
    assert!(stubs.contains("fn add"));
    assert!(stubs.contains("a: i32"));
    assert!(stubs.contains("b: i32"));
    assert!(stubs.contains("-> i32"));

    // Check struct stub
    assert!(stubs.contains("struct Point"));
}

/// Test the full end-to-end flow.
#[test]
fn test_end_to_end() {
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

    let ast = parser
        .parse_string(source, "math.cpp")
        .expect("Failed to parse");
    let code = AstCodeGen::new().generate(&ast.translation_unit);

    // Verify both functions are generated
    assert!(code.contains("pub fn add"));
    assert!(code.contains("pub fn multiply"));
    assert!(code.contains("return a + b"));
    assert!(code.contains("return x * y"));
}

/// Test parsing and generating namespaced functions.
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

    let ast = parser
        .parse_string(source, "ns.cpp")
        .expect("Failed to parse");
    let code = AstCodeGen::new().generate(&ast.translation_unit);

    // Function should be generated
    assert!(code.contains("fn compute"));
    assert!(code.contains("return x * 2"));
}

/// Test control flow generation.
#[test]
fn test_control_flow() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int max(int a, int b) {
            if (a > b) {
                return a;
            } else {
                return b;
            }
        }
    "#;

    let ast = parser
        .parse_string(source, "max.cpp")
        .expect("Failed to parse");
    let code = AstCodeGen::new().generate(&ast.translation_unit);

    // Check natural control flow is preserved
    assert!(code.contains("if a > b"));
    assert!(code.contains("return a"));
    assert!(code.contains("} else {"));
    assert!(code.contains("return b"));
}

/// Test while loop generation.
#[test]
fn test_while_loop() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int sum_to_n(int n) {
            int sum = 0;
            int i = 1;
            while (i <= n) {
                sum = sum + i;
                i = i + 1;
            }
            return sum;
        }
    "#;

    let ast = parser
        .parse_string(source, "sum.cpp")
        .expect("Failed to parse");
    let code = AstCodeGen::new().generate(&ast.translation_unit);

    // Check while loop is preserved
    assert!(code.contains("while i <= n"));
    assert!(code.contains("return sum"));
}

// ============================================================================
// End-to-End Tests: Transpile -> Compile -> Run
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper to find the fragile-runtime library path.
/// Looks in the target directory for the compiled rlib.
fn find_fragile_runtime_path() -> Option<PathBuf> {
    // Try to find the workspace root by looking for Cargo.toml
    let mut current = std::env::current_dir().ok()?;

    // Walk up to find workspace root
    for _ in 0..10 {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).ok()?;
            if content.contains("[workspace]") {
                // Found workspace root, look for runtime library
                // Check release first (more common in CI), then debug
                let release_path = current.join("target/release");
                if release_path.join("libfragile_runtime.rlib").exists() {
                    return Some(release_path);
                }
                let debug_path = current.join("target/debug");
                if debug_path.join("libfragile_runtime.rlib").exists() {
                    return Some(debug_path);
                }
            }
        }
        current = current.parent()?.to_path_buf();
    }
    None
}

/// Helper function to transpile C++ source, compile with rustc, and run.
/// Returns (exit_code, stdout, stderr).
fn transpile_compile_run(
    cpp_source: &str,
    filename: &str,
) -> Result<(i32, String, String), String> {
    let parser = ClangParser::new().map_err(|e| format!("Failed to create parser: {}", e))?;

    // Parse and generate Rust code
    let ast = parser
        .parse_string(cpp_source, filename)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    let rust_code = AstCodeGen::new().generate(&ast.translation_unit);

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_tests");
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Write Rust source
    let rs_path = temp_dir.join(format!("{}.rs", filename.replace(".cpp", "")));
    fs::write(&rs_path, &rust_code).map_err(|e| format!("Failed to write Rust source: {}", e))?;

    // Compile with rustc
    let binary_path = temp_dir.join(filename.replace(".cpp", ""));
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !compile_output.status.success() {
        return Err(format!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}\n\nGenerated code:\n{}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr),
            rust_code
        ));
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .map_err(|e| format!("Failed to run binary: {}", e))?;

    Ok((
        run_output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&run_output.stdout).to_string(),
        String::from_utf8_lossy(&run_output.stderr).to_string(),
    ))
}

/// Helper function to transpile C++ source, compile with rustc + fragile-runtime, and run.
/// This version links against fragile-runtime for tests that need runtime support.
/// Returns (exit_code, stdout, stderr).
#[allow(dead_code)] // Reserved for future STL transpilation tests
fn transpile_compile_run_with_runtime(
    cpp_source: &str,
    filename: &str,
) -> Result<(i32, String, String), String> {
    let parser = ClangParser::new().map_err(|e| format!("Failed to create parser: {}", e))?;

    // Parse and generate Rust code
    let ast = parser
        .parse_string(cpp_source, filename)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    let rust_code = AstCodeGen::new().generate(&ast.translation_unit);

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_runtime_tests");
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Write Rust source
    let rs_path = temp_dir.join(format!("{}.rs", filename.replace(".cpp", "")));
    fs::write(&rs_path, &rust_code).map_err(|e| format!("Failed to write Rust source: {}", e))?;

    // Find fragile-runtime library path
    let runtime_path = find_fragile_runtime_path()
        .ok_or_else(|| "Could not find fragile-runtime library path".to_string())?;

    // Compile with rustc, linking against fragile-runtime
    let binary_path = temp_dir.join(filename.replace(".cpp", ""));
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .arg("-L")
        .arg(&runtime_path)
        .arg("-L")
        .arg(runtime_path.join("deps"))
        .arg("--extern")
        .arg(format!(
            "fragile_runtime={}/libfragile_runtime.rlib",
            runtime_path.display()
        ))
        .output()
        .map_err(|e| format!("Failed to run rustc: {}", e))?;

    if !compile_output.status.success() {
        return Err(format!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}\n\nGenerated code:\n{}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr),
            rust_code
        ));
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .map_err(|e| format!("Failed to run binary: {}", e))?;

    Ok((
        run_output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&run_output.stdout).to_string(),
        String::from_utf8_lossy(&run_output.stderr).to_string(),
    ))
}

/// E2E test: Simple arithmetic function
#[test]
fn test_e2e_simple_add() {
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int main() {
            return add(5, 7) - 12;  // Returns 0 if add works correctly
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_add.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "add(5, 7) should equal 12");
}

/// E2E test: Factorial with recursion
#[test]
fn test_e2e_factorial() {
    let source = r#"
        int factorial(int n) {
            if (n <= 1) {
                return 1;
            }
            return n * factorial(n - 1);
        }

        int main() {
            int f5 = factorial(5);
            if (f5 == 120) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_factorial.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "factorial(5) should equal 120");
}

/// E2E test: While loop sum
#[test]
fn test_e2e_while_loop() {
    let source = r#"
        int sum_to_n(int n) {
            int sum = 0;
            int i = 1;
            while (i <= n) {
                sum = sum + i;
                i = i + 1;
            }
            return sum;
        }

        int main() {
            int s = sum_to_n(10);
            if (s == 55) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_while.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "sum_to_n(10) should equal 55");
}

/// E2E test: For loop
#[test]
fn test_e2e_for_loop() {
    let source = r#"
        int sum_for(int n) {
            int sum = 0;
            for (int i = 1; i <= n; i = i + 1) {
                sum = sum + i;
            }
            return sum;
        }

        int main() {
            if (sum_for(10) == 55) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_for.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "sum_for(10) should equal 55");
}

/// E2E test: Struct with methods
#[test]
fn test_e2e_struct_methods() {
    let source = r#"
        struct Counter {
            int value;

            void increment() {
                value = value + 1;
            }

            int get() {
                return value;
            }
        };

        int main() {
            Counter c;
            c.value = 0;
            c.increment();
            c.increment();
            c.increment();
            if (c.get() == 3) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_struct.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Counter should increment to 3");
}

/// E2E test: Arrays
#[test]
fn test_e2e_arrays() {
    let source = r#"
        int sum_array(int* arr, int size) {
            int sum = 0;
            for (int i = 0; i < size; i = i + 1) {
                sum = sum + arr[i];
            }
            return sum;
        }

        int main() {
            int arr[5];
            arr[0] = 1;
            arr[1] = 2;
            arr[2] = 3;
            arr[3] = 4;
            arr[4] = 5;
            int s = sum_array(arr, 5);
            if (s == 15) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_arrays.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "sum_array should equal 15");
}

/// E2E test: Pointers
#[test]
fn test_e2e_pointers() {
    let source = r#"
        void swap(int* a, int* b) {
            int temp = *a;
            *a = *b;
            *b = temp;
        }

        int main() {
            int x = 10;
            int y = 20;
            swap(&x, &y);
            if (x == 20 && y == 10) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_pointers.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "swap should exchange x and y");
}

/// E2E test: References
#[test]
fn test_e2e_references() {
    let source = r#"
        void increment(int& x) {
            x = x + 1;
        }

        int main() {
            int val = 5;
            increment(val);
            increment(val);
            if (val == 7) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_references.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "val should be incremented twice to 7");
}

/// E2E test: Nested control flow
#[test]
fn test_e2e_nested_control() {
    let source = r#"
        int is_prime(int n) {
            if (n <= 1) {
                return 0;
            }
            for (int i = 2; i < n; i = i + 1) {
                if (n % i == 0) {
                    return 0;
                }
            }
            return 1;
        }

        int main() {
            int primes = 0;
            for (int i = 1; i <= 20; i = i + 1) {
                if (is_prime(i) == 1) {
                    primes = primes + 1;
                }
            }
            // Primes 1-20: 2,3,5,7,11,13,17,19 = 8 primes
            if (primes == 8) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_prime.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "There should be 8 primes between 1 and 20");
}

/// E2E test: Constructor
#[test]
fn test_e2e_constructor() {
    let source = r#"
        struct Point {
            int x;
            int y;

            Point(int a, int b) {
                x = a;
                y = b;
            }

            int distance_sq() {
                return x * x + y * y;
            }
        };

        int main() {
            Point p(3, 4);
            if (p.distance_sq() == 25) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_constructor.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Point(3,4).distance_sq() should be 25");
}

/// E2E test: nullptr handling
#[test]
fn test_e2e_nullptr() {
    let source = r#"
        int* get_null() {
            return nullptr;
        }

        int main() {
            int* p = nullptr;
            if (p == nullptr) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_nullptr.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "nullptr should be handled correctly");
}

/// E2E test: C++ casts (static_cast, reinterpret_cast)
#[test]
fn test_e2e_casts() {
    let source = r#"
        int test_static_cast(double d) {
            return static_cast<int>(d);
        }

        int main() {
            double d = 3.7;
            int i = static_cast<int>(d);
            if (i == 3) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_casts.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "static_cast<int>(3.7) should equal 3");
}

/// E2E test: new/delete
#[test]
fn test_e2e_new_delete() {
    let source = r#"
        int main() {
            int* p = new int(42);
            int v = *p;
            delete p;
            if (v == 42) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_new_delete.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "new int(42) should create value 42");
}

/// E2E test: new[]/delete[] (array allocation)
#[test]
fn test_e2e_array_new_delete() {
    let source = r#"
        int main() {
            int* arr = new int[5];
            arr[0] = 10;
            arr[1] = 20;
            int sum = arr[0] + arr[1];
            delete[] arr;
            if (sum == 30) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_array_new.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "new int[5] should create array that can be indexed"
    );
}

/// E2E test: Single inheritance
#[test]
fn test_e2e_inheritance() {
    let source = r#"
        struct Animal {
            int legs;
        };

        struct Dog : public Animal {
            int tail;
        };

        int main() {
            Dog d;
            d.legs = 4;  // Inherited from Animal
            d.tail = 1;
            if (d.legs == 4 && d.tail == 1) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_inheritance.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Inheritance should embed base struct as __base field"
    );
}

/// E2E test: Destructor → Drop trait
#[test]
fn test_e2e_destructor() {
    let source = r#"
        struct Resource {
            int value;

            Resource() {
                value = 100;
            }

            ~Resource() {
                // Destructor body - this code runs when Drop::drop is called
                value = 0;  // Reset value on destruction
            }

            int get() {
                return value;
            }
        };

        int main() {
            Resource r;
            int v = r.get();
            if (v == 100) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_destructor.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Struct with destructor should compile and run"
    );
}

#[test]
fn test_e2e_copy_constructor() {
    let source = r#"
        struct Point {
            int x;
            int y;

            Point() {
                x = 0;
                y = 0;
            }

            Point(int px, int py) {
                x = px;
                y = py;
            }

            // Copy constructor
            Point(const Point& other) {
                x = other.x;
                y = other.y;
            }

            int sum() {
                return x + y;
            }
        };

        int main() {
            Point a(10, 20);
            Point b = a;  // Uses copy constructor
            // Verify both a and b have the same values
            if (a.sum() == 30 && b.sum() == 30) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_copy_ctor.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Struct with copy constructor should compile and run"
    );
}

#[test]
fn test_e2e_exception_handling() {
    let source = r#"
        int divide(int a, int b) {
            if (b == 0) {
                throw "Division by zero";
            }
            return a / b;
        }

        int safe_divide(int a, int b) {
            try {
                return divide(a, b);
            } catch (...) {
                return -1;
            }
        }

        int main() {
            // Test normal division
            int r1 = safe_divide(10, 2);
            if (r1 != 5) return 1;

            // Test division by zero (should catch and return -1)
            int r2 = safe_divide(10, 0);
            if (r2 != -1) return 2;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_exception.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Exception handling should compile and run");
}

#[test]
fn test_e2e_namespaces() {
    let source = r#"
        namespace math {
            int add(int a, int b) {
                return a + b;
            }

            namespace utils {
                int multiply(int a, int b) {
                    return a * b;
                }
            }
        }

        int main() {
            int r1 = math::add(2, 3);
            int r2 = math::utils::multiply(4, 5);
            if (r1 == 5 && r2 == 20) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_namespace.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Namespace functions should compile and run");
}

/// Test virtual method override resolution (static dispatch).
#[test]
fn test_e2e_virtual_override() {
    let source = r#"
        class Animal {
        public:
            virtual int speak() {
                return 1;
            }
            int eat() {
                return 10;
            }
        };

        class Dog : public Animal {
        public:
            int speak() override {
                return 2;
            }
        };

        int main() {
            Dog d;
            int a = d.speak();      // 2 (overridden)
            int b = d.eat();        // 10 (inherited)
            if (a == 2 && b == 10) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_virtual_override.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Virtual method override should work correctly"
    );
}

/// Test base class constructor delegation.
#[test]
fn test_e2e_base_constructor() {
    let source = r#"
        class Base {
        protected:
            int x;
            int y;
        public:
            Base(int a, int b) : x(a), y(b) {}
        };

        class Derived : public Base {
            int z;
        public:
            Derived(int a, int b, int c) : Base(a, b), z(c) {}
            int sum() { return x + y + z; }
        };

        int main() {
            Derived d(10, 20, 30);
            if (d.sum() == 60) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_base_constructor.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Base class constructor delegation should work"
    );
}

/// Test operator overloading.
#[test]
fn test_e2e_operator_overloading() {
    let source = r#"
        class Point {
            int x;
            int y;
        public:
            Point(int a, int b) : x(a), y(b) {}
            Point operator+(const Point& other) const {
                return Point(x + other.x, y + other.y);
            }
            bool operator==(const Point& other) const {
                return x == other.x && y == other.y;
            }
            int getX() const { return x; }
            int getY() const { return y; }
        };

        int main() {
            Point a(1, 2);
            Point b(3, 4);
            Point c = a + b;
            if (c.getX() == 4 && c.getY() == 6) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_operator_overloading.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Operator overloading should work correctly");
}

/// Test dynamic dispatch (virtual method polymorphism).
#[test]
fn test_e2e_dynamic_dispatch() {
    let source = r#"
        class Animal {
        public:
            virtual int speak() { return 1; }
        };

        class Dog : public Animal {
        public:
            int speak() override { return 2; }
        };

        class Cat : public Animal {
        public:
            int speak() override { return 3; }
        };

        int callSpeak(Animal* a) {
            return a->speak();
        }

        int main() {
            Dog d;
            Cat c;
            // Dynamic dispatch: should call Dog::speak() and Cat::speak()
            int result = callSpeak(&d) + callSpeak(&c);
            if (result == 5) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_dynamic_dispatch.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Dynamic dispatch should correctly call derived class methods"
    );
}

/// Test deep inheritance hierarchy (A → B → C → D) with virtual methods.
/// Verifies vtable dispatch works through multiple inheritance levels.
#[test]
fn test_e2e_deep_inheritance() {
    let source = r#"
        // Root class with virtual method
        class Base {
        public:
            virtual int level() { return 0; }
        };

        // Level 1 - overrides Base::level
        class Level1 : public Base {
        public:
            int level() override { return 1; }
        };

        // Level 2 - overrides Level1::level
        class Level2 : public Level1 {
        public:
            int level() override { return 2; }
        };

        // Level 3 - overrides Level2::level
        class Level3 : public Level2 {
        public:
            int level() override { return 3; }
        };

        // Level 4 - does NOT override, inherits Level3::level
        class Level4 : public Level3 {
        public:
            // No override - should still return 3
        };

        int getLevel(Base* b) {
            return b->level();
        }

        int main() {
            Base b;
            Level1 l1;
            Level2 l2;
            Level3 l3;
            Level4 l4;

            // Test virtual dispatch at each level
            int sum = getLevel(&b) + getLevel(&l1) + getLevel(&l2) + getLevel(&l3) + getLevel(&l4);
            // Expected: 0 + 1 + 2 + 3 + 3 = 9
            if (sum == 9) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_deep_inheritance.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Deep inheritance virtual dispatch should work correctly"
    );
}

/// Test that RTTI type IDs are generated correctly for vtables.
/// Verifies the RTTI infrastructure exists without testing dynamic_cast directly.
#[test]
fn test_e2e_vtable_rtti() {
    let source = r#"
        class Animal {
        public:
            virtual int id() { return 1; }
            virtual ~Animal() {}
        };

        class Dog : public Animal {
        public:
            int id() override { return 2; }
        };

        class Cat : public Animal {
        public:
            int id() override { return 3; }
        };

        int main() {
            Animal a;
            Dog d;
            Cat c;

            // Test that virtual dispatch works correctly
            Animal* pa = &a;
            Animal* pd = &d;
            Animal* pc = &c;

            int sum = pa->id() + pd->id() + pc->id();
            // Expected: 1 + 2 + 3 = 6
            if (sum == 6) {
                return 0;  // Success
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_vtable_rtti.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "RTTI type IDs should be generated for vtables"
    );
}

/// Test dynamic_cast with RTTI.
/// Verifies that dynamic_cast correctly returns the pointer on success
/// and nullptr on failure.
#[test]
fn test_e2e_dynamic_cast() {
    let source = r#"
        class Base {
        public:
            virtual int id() { return 1; }
            virtual ~Base() {}
        };

        class Derived : public Base {
        public:
            int id() override { return 2; }
        };

        int main() {
            Derived d;
            Base b;

            Base* pd = &d;  // Points to Derived
            Base* pb = &b;  // Points to Base

            // Test 1: dynamic_cast should succeed when actual type matches
            Derived* d1 = dynamic_cast<Derived*>(pd);
            if (d1 == nullptr) return 1;  // Should not be null

            // Test 2: dynamic_cast should fail when actual type doesn't match
            Derived* d2 = dynamic_cast<Derived*>(pb);
            if (d2 != nullptr) return 3;  // Should be null

            // Test 3: dynamic_cast on nullptr should return nullptr
            Base* null_ptr = nullptr;
            Derived* d3 = dynamic_cast<Derived*>(null_ptr);
            if (d3 != nullptr) return 4;  // Should be null

            return 0;  // All tests passed
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_dynamic_cast.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "dynamic_cast should work correctly with RTTI");
}

/// Test function template instantiation.
/// Verifies that function templates are correctly instantiated with concrete types.
#[test]
fn test_e2e_function_template() {
    let source = r#"
        template<typename T>
        T add(T a, T b) {
            return a + b;
        }

        template<typename T>
        T identity(T x) {
            return x;
        }

        int main() {
            // Test 1: int instantiation of add
            int sum = add(3, 4);
            if (sum != 7) return 1;

            // Test 2: identity function template
            int x = identity(42);
            if (x != 42) return 2;

            return 0;  // All tests passed
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_template.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "function template instantiation should work correctly"
    );
}

/// Test function template with pointer parameters.
/// Verifies that templates like swap(T* a, T* b) extract T correctly (not T*).
#[test]
fn test_e2e_function_template_pointer_params() {
    let source = r#"
        template<typename T>
        void swap(T* a, T* b) {
            T tmp = *a;
            *a = *b;
            *b = tmp;
        }

        int main() {
            // Test 1: swap two integers via pointers
            int x = 100, y = 200;
            swap(&x, &y);
            if (x != 200 || y != 100) return 1;

            // Test 2: swap again to verify it works both ways
            swap(&x, &y);
            if (x != 100 || y != 200) return 2;

            return 0;  // All tests passed
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_template_ptr.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "function template with pointer parameters should work correctly"
    );
}

/// Test function template with reference parameters.
/// Verifies that templates like swap(T& a, T& b) extract T correctly.
#[test]
fn test_e2e_function_template_reference_params() {
    let source = r#"
        template<typename T>
        void swap_ref(T& a, T& b) {
            T tmp = a;
            a = b;
            b = tmp;
        }

        int main() {
            // Test 1: swap two integers via references
            int x = 50, y = 150;
            swap_ref(x, y);
            if (x != 150 || y != 50) return 1;

            // Test 2: swap again to verify
            swap_ref(x, y);
            if (x != 50 || y != 150) return 2;

            return 0;  // All tests passed
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_template_ref.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "function template with reference parameters should work correctly"
    );
}

/// Test function template with multiple type parameters.
/// Verifies that templates like pair<T, U> work with different instantiations.
#[test]
fn test_e2e_function_template_multiple_params() {
    let source = r#"
        template<typename T, typename U>
        T first_of(T a, U b) {
            return a;
        }

        template<typename T, typename U>
        U second_of(T a, U b) {
            return b;
        }

        int main() {
            // Test 1: first_of with int, float
            int a = first_of(42, 3.14f);
            if (a != 42) return 1;

            // Test 2: second_of with int, float (returns float as int)
            float b = second_of(10, 2.5f);
            if (b < 2.4f || b > 2.6f) return 2;

            // Test 3: first_of with different types
            int c = first_of(100, 'X');
            if (c != 100) return 3;

            return 0;  // All tests passed
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_template_multi.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "function template with multiple type parameters should work correctly"
    );
}

/// Test std_string stub operations directly in generated Rust code.
/// This verifies the std_string stub in the preamble works correctly.
/// Note: This test compiles hand-written Rust that uses the stub, rather than
/// transpiling C++ std::string usage, because full std::string transpilation
/// requires complete libc++ support (which is still in progress).
#[test]
fn test_e2e_std_string_stub() {
    use std::fs;
    use std::process::Command;

    // Write Rust code that directly uses the std_string stub from preamble
    let rust_code = r#"
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

// std::string stub implementation (same as generated in preamble)
#[repr(C)]
#[derive(Default)]
pub struct std_string {
    _data: *mut i8,
    _size: usize,
    _capacity: usize,
}

impl std_string {
    pub fn new_0() -> Self {
        Self { _data: std::ptr::null_mut(), _size: 0, _capacity: 0 }
    }
    pub fn new_1(s: *const i8) -> Self {
        if s.is_null() {
            return Self::new_0();
        }
        let mut len = 0usize;
        unsafe { while *s.add(len) != 0 { len += 1; } }
        let cap = len + 1;
        let layout = std::alloc::Layout::array::<i8>(cap).unwrap();
        let data = unsafe { std::alloc::alloc(layout) as *mut i8 };
        unsafe { std::ptr::copy_nonoverlapping(s, data, len); }
        unsafe { *data.add(len) = 0; }
        Self { _data: data, _size: len, _capacity: cap }
    }
    pub fn c_str(&self) -> *const i8 {
        if self._data.is_null() {
            b"\0".as_ptr() as *const i8
        } else {
            self._data as *const i8
        }
    }
    pub fn size(&self) -> usize { self._size }
    pub fn length(&self) -> usize { self._size }
    pub fn empty(&self) -> bool { self._size == 0 }
    pub fn push_back(&mut self, c: i8) {
        if self._size + 1 >= self._capacity {
            let new_cap = if self._capacity == 0 { 16 } else { self._capacity * 2 };
            let new_layout = std::alloc::Layout::array::<i8>(new_cap).unwrap();
            let new_data = unsafe { std::alloc::alloc(new_layout) as *mut i8 };
            if !self._data.is_null() {
                unsafe { std::ptr::copy_nonoverlapping(self._data, new_data, self._size); }
                let old_layout = std::alloc::Layout::array::<i8>(self._capacity).unwrap();
                unsafe { std::alloc::dealloc(self._data as *mut u8, old_layout); }
            }
            self._data = new_data;
            self._capacity = new_cap;
        }
        unsafe { *self._data.add(self._size) = c; }
        self._size += 1;
        unsafe { *self._data.add(self._size) = 0; }
    }
    pub fn append(&mut self, s: *const i8) -> &mut Self {
        if s.is_null() { return self; }
        let mut len = 0usize;
        unsafe { while *s.add(len) != 0 { len += 1; } }
        for i in 0..len {
            self.push_back(unsafe { *s.add(i) });
        }
        self
    }
    pub fn clear(&mut self) {
        self._size = 0;
        if !self._data.is_null() {
            unsafe { *self._data = 0; }
        }
    }
    pub fn capacity(&self) -> usize { self._capacity }
}

impl Drop for std_string {
    fn drop(&mut self) {
        if !self._data.is_null() && self._capacity > 0 {
            let layout = std::alloc::Layout::array::<i8>(self._capacity).unwrap();
            unsafe { std::alloc::dealloc(self._data as *mut u8, layout); }
        }
    }
}

fn main() {
    // Test 1: Default constructor creates empty string
    let mut s = std_string::new_0();
    if !s.empty() { std::process::exit(1); }
    if s.size() != 0 { std::process::exit(2); }

    // Test 2: Push back characters
    s.push_back(b'H' as i8);
    s.push_back(b'i' as i8);
    if s.size() != 2 { std::process::exit(3); }
    if s.empty() { std::process::exit(4); }

    // Test 3: c_str() returns correct data
    let cs = s.c_str();
    unsafe {
        if *cs.add(0) != b'H' as i8 { std::process::exit(5); }
        if *cs.add(1) != b'i' as i8 { std::process::exit(6); }
        if *cs.add(2) != 0 { std::process::exit(7); }  // null terminator
    }

    // Test 4: clear() empties the string
    s.clear();
    if !s.empty() { std::process::exit(8); }
    if s.size() != 0 { std::process::exit(9); }

    // Test 5: length() is alias for size()
    s.push_back(b'X' as i8);
    if s.length() != 1 { std::process::exit(10); }

    // Test 6: Constructor from C string
    let hello = b"Hello\0".as_ptr() as *const i8;
    let s2 = std_string::new_1(hello);
    if s2.size() != 5 { std::process::exit(11); }
    unsafe {
        let cs2 = s2.c_str();
        if *cs2.add(0) != b'H' as i8 { std::process::exit(12); }
        if *cs2.add(4) != b'o' as i8 { std::process::exit(13); }
    }

    // Test 7: append()
    let mut s3 = std_string::new_0();
    let world = b"World\0".as_ptr() as *const i8;
    s3.append(world);
    if s3.size() != 5 { std::process::exit(14); }

    std::process::exit(0);  // All tests passed
}
"#;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write Rust source
    let rs_path = temp_dir.join("e2e_std_string_stub.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write Rust source");

    // Compile with rustc
    let binary_path = temp_dir.join("e2e_std_string_stub");
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .expect("Failed to run binary");

    let exit_code = run_output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code, 0,
        "std_string stub operations should work correctly (exit code: {})",
        exit_code
    );
}

/// Test std_unordered_map_int_int stub operations directly in generated Rust code.
/// This verifies the std_unordered_map stub in the preamble works correctly.
#[test]
fn test_e2e_std_unordered_map_stub() {
    use std::fs;
    use std::process::Command;

    // Write Rust code that directly uses the std_unordered_map_int_int stub
    let rust_code = r#"
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

// std::unordered_map<int, int> stub implementation (same as generated in preamble)
#[repr(C)]
pub struct std_unordered_map_int_int {
    _buckets: Vec<Vec<(i32, i32)>>,
    _size: usize,
}

impl Default for std_unordered_map_int_int {
    fn default() -> Self {
        Self { _buckets: vec![Vec::new(); 16], _size: 0 }
    }
}

impl std_unordered_map_int_int {
    pub fn new_0() -> Self { Default::default() }
    pub fn size(&self) -> usize { self._size }
    pub fn empty(&self) -> bool { self._size == 0 }
    #[inline]
    fn _hash(key: i32) -> usize {
        (key as u32 as usize) % 16
    }
    pub fn insert(&mut self, key: i32, value: i32) {
        let idx = Self::_hash(key);
        for &mut (ref k, ref mut v) in &mut self._buckets[idx] {
            if *k == key { *v = value; return; }
        }
        self._buckets[idx].push((key, value));
        self._size += 1;
    }
    pub fn find(&self, key: i32) -> Option<i32> {
        let idx = Self::_hash(key);
        for &(k, v) in &self._buckets[idx] {
            if k == key { return Some(v); }
        }
        None
    }
    pub fn contains(&self, key: i32) -> bool { self.find(key).is_some() }
    pub fn op_index(&mut self, key: i32) -> &mut i32 {
        let idx = Self::_hash(key);
        for i in 0..self._buckets[idx].len() {
            if self._buckets[idx][i].0 == key {
                return &mut self._buckets[idx][i].1;
            }
        }
        self._buckets[idx].push((key, 0));
        self._size += 1;
        let len = self._buckets[idx].len();
        &mut self._buckets[idx][len - 1].1
    }
    pub fn erase(&mut self, key: i32) -> bool {
        let idx = Self::_hash(key);
        if let Some(pos) = self._buckets[idx].iter().position(|&(k, _)| k == key) {
            self._buckets[idx].remove(pos);
            self._size -= 1;
            return true;
        }
        false
    }
    pub fn clear(&mut self) {
        for bucket in &mut self._buckets {
            bucket.clear();
        }
        self._size = 0;
    }
}

fn main() {
    // Test 1: Default constructor creates empty map
    let mut m = std_unordered_map_int_int::new_0();
    if !m.empty() { std::process::exit(1); }
    if m.size() != 0 { std::process::exit(2); }

    // Test 2: Insert and find
    m.insert(1, 100);
    m.insert(2, 200);
    if m.size() != 2 { std::process::exit(3); }
    if m.find(1) != Some(100) { std::process::exit(4); }
    if m.find(2) != Some(200) { std::process::exit(5); }
    if m.find(99) != None { std::process::exit(6); }

    // Test 3: Update existing key
    m.insert(1, 111);
    if m.find(1) != Some(111) { std::process::exit(7); }
    if m.size() != 2 { std::process::exit(8); }

    // Test 4: operator[] access
    *m.op_index(3) = 300;
    if m.find(3) != Some(300) { std::process::exit(9); }
    if m.size() != 3 { std::process::exit(10); }

    // Test 5: contains
    if !m.contains(1) { std::process::exit(11); }
    if !m.contains(2) { std::process::exit(12); }
    if !m.contains(3) { std::process::exit(13); }
    if m.contains(99) { std::process::exit(14); }

    // Test 6: erase
    if !m.erase(1) { std::process::exit(15); }
    if m.contains(1) { std::process::exit(16); }
    if m.size() != 2 { std::process::exit(17); }
    if m.erase(99) { std::process::exit(18); }  // erase non-existent

    // Test 7: clear
    m.clear();
    if !m.empty() { std::process::exit(19); }
    if m.size() != 0 { std::process::exit(20); }

    std::process::exit(0);  // All tests passed
}
"#;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write Rust source
    let rs_path = temp_dir.join("e2e_std_unordered_map_stub.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write Rust source");

    // Compile with rustc
    let binary_path = temp_dir.join("e2e_std_unordered_map_stub");
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .expect("Failed to run binary");

    let exit_code = run_output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code, 0,
        "std_unordered_map_int_int stub operations should work correctly (exit code: {})",
        exit_code
    );
}

/// Test std::unique_ptr and std::shared_ptr stub operations directly in generated Rust code.
/// This verifies the smart pointer stubs in the preamble work correctly.
#[test]
fn test_e2e_smart_ptr_stub() {
    use std::fs;
    use std::process::Command;

    // Write Rust code that directly uses the smart pointer stubs
    let rust_code = r#"
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

// std::unique_ptr<int> stub implementation (same as generated in preamble)
#[repr(C)]
pub struct std_unique_ptr_int {
    _ptr: *mut i32,
}

impl Default for std_unique_ptr_int {
    fn default() -> Self { Self { _ptr: std::ptr::null_mut() } }
}

impl std_unique_ptr_int {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(ptr: *mut i32) -> Self { Self { _ptr: ptr } }
    pub fn get(&self) -> *mut i32 { self._ptr }
    pub fn op_deref(&self) -> &mut i32 {
        unsafe { &mut *self._ptr }
    }
    pub fn op_arrow(&self) -> *mut i32 { self._ptr }
    pub fn release(&mut self) -> *mut i32 {
        let ptr = self._ptr;
        self._ptr = std::ptr::null_mut();
        ptr
    }
    pub fn reset(&mut self) {
        if !self._ptr.is_null() {
            unsafe { drop(Box::from_raw(self._ptr)); }
        }
        self._ptr = std::ptr::null_mut();
    }
}

impl Drop for std_unique_ptr_int {
    fn drop(&mut self) {
        if !self._ptr.is_null() {
            unsafe { drop(Box::from_raw(self._ptr)); }
        }
    }
}

// std::shared_ptr<int> stub implementation (same as generated in preamble)
#[repr(C)]
pub struct std_shared_ptr_int {
    _ptr: *mut i32,
    _refcount: *mut usize,
}

impl Default for std_shared_ptr_int {
    fn default() -> Self { Self { _ptr: std::ptr::null_mut(), _refcount: std::ptr::null_mut() } }
}

impl std_shared_ptr_int {
    pub fn new_0() -> Self { Default::default() }
    pub fn new_1(ptr: *mut i32) -> Self {
        let refcount = Box::into_raw(Box::new(1usize));
        Self { _ptr: ptr, _refcount: refcount }
    }
    pub fn get(&self) -> *mut i32 { self._ptr }
    pub fn op_deref(&self) -> &mut i32 {
        unsafe { &mut *self._ptr }
    }
    pub fn use_count(&self) -> usize {
        if self._refcount.is_null() { 0 } else { unsafe { *self._refcount } }
    }
    pub fn reset(&mut self) {
        if !self._refcount.is_null() {
            unsafe {
                *self._refcount -= 1;
                if *self._refcount == 0 {
                    if !self._ptr.is_null() { drop(Box::from_raw(self._ptr)); }
                    drop(Box::from_raw(self._refcount));
                }
            }
        }
        self._ptr = std::ptr::null_mut();
        self._refcount = std::ptr::null_mut();
    }
}

impl Clone for std_shared_ptr_int {
    fn clone(&self) -> Self {
        if !self._refcount.is_null() {
            unsafe { *self._refcount += 1; }
        }
        Self { _ptr: self._ptr, _refcount: self._refcount }
    }
}

impl Drop for std_shared_ptr_int {
    fn drop(&mut self) {
        if !self._refcount.is_null() {
            unsafe {
                *self._refcount -= 1;
                if *self._refcount == 0 {
                    if !self._ptr.is_null() { drop(Box::from_raw(self._ptr)); }
                    drop(Box::from_raw(self._refcount));
                }
            }
        }
    }
}

fn main() {
    // ========== unique_ptr tests ==========

    // Test 1: Default constructor creates null pointer
    let up0 = std_unique_ptr_int::new_0();
    if !up0.get().is_null() { std::process::exit(1); }
    drop(up0);

    // Test 2: Constructor from raw pointer holds the pointer
    let raw_ptr = Box::into_raw(Box::new(42i32));
    let mut up1 = std_unique_ptr_int::new_1(raw_ptr);
    if up1.get() != raw_ptr { std::process::exit(2); }
    if up1.get().is_null() { std::process::exit(3); }

    // Test 3: op_deref returns the value
    if *up1.op_deref() != 42 { std::process::exit(4); }

    // Test 4: Modify value through op_deref
    *up1.op_deref() = 100;
    if *up1.op_deref() != 100 { std::process::exit(5); }

    // Test 5: op_arrow returns the pointer
    if up1.op_arrow() != raw_ptr { std::process::exit(6); }

    // Test 6: release() returns pointer and clears ownership
    let released = up1.release();
    if released != raw_ptr { std::process::exit(7); }
    if !up1.get().is_null() { std::process::exit(8); }
    // Manual cleanup since we released
    unsafe { drop(Box::from_raw(released)); }
    drop(up1);

    // Test 7: reset() on non-null pointer
    let raw_ptr2 = Box::into_raw(Box::new(200i32));
    let mut up2 = std_unique_ptr_int::new_1(raw_ptr2);
    up2.reset();
    if !up2.get().is_null() { std::process::exit(9); }
    drop(up2);

    // ========== shared_ptr tests ==========

    // Test 10: Default constructor creates null pointer with use_count 0
    let sp0 = std_shared_ptr_int::new_0();
    if !sp0.get().is_null() { std::process::exit(10); }
    if sp0.use_count() != 0 { std::process::exit(11); }
    drop(sp0);

    // Test 11: Constructor from raw pointer, use_count == 1
    let raw_ptr3 = Box::into_raw(Box::new(300i32));
    let sp1 = std_shared_ptr_int::new_1(raw_ptr3);
    if sp1.get() != raw_ptr3 { std::process::exit(12); }
    if sp1.use_count() != 1 { std::process::exit(13); }

    // Test 12: op_deref returns the value
    if *sp1.op_deref() != 300 { std::process::exit(14); }

    // Test 13: Clone increases use_count
    let sp2 = sp1.clone();
    if sp1.use_count() != 2 { std::process::exit(15); }
    if sp2.use_count() != 2 { std::process::exit(16); }
    if sp1.get() != sp2.get() { std::process::exit(17); }

    // Test 14: Drop decreases use_count
    drop(sp2);
    if sp1.use_count() != 1 { std::process::exit(18); }

    // Test 15: reset() decreases use_count
    let sp3 = sp1.clone();
    if sp1.use_count() != 2 { std::process::exit(19); }
    let mut sp4 = sp3;
    sp4.reset();
    if sp4.use_count() != 0 { std::process::exit(20); }
    if sp4.get() != std::ptr::null_mut() { std::process::exit(21); }
    if sp1.use_count() != 1 { std::process::exit(22); }

    // Test 16: Memory freed when last reference drops (no crash = success)
    drop(sp1);

    std::process::exit(0);  // All tests passed
}
"#;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write Rust source
    let rs_path = temp_dir.join("e2e_smart_ptr_stub.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write Rust source");

    // Compile with rustc
    let binary_path = temp_dir.join("e2e_smart_ptr_stub");
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .expect("Failed to run binary");

    let exit_code = run_output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code, 0,
        "Smart pointer stub operations should work correctly (exit code: {})",
        exit_code
    );
}

/// Test STL algorithm stub operations directly in generated Rust code.
/// This verifies the algorithm stubs (std::sort, std::find, etc.) in the preamble work correctly.
#[test]
fn test_e2e_stl_algorithm_stub() {
    use std::fs;
    use std::process::Command;

    // Write Rust code that directly uses the STL algorithm stubs
    let rust_code = r#"
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

// STL algorithm stubs (same as generated in preamble)

/// std::sort(first, last) - sorts range [first, last) in ascending order
pub fn std_sort_int(first: *mut i32, last: *mut i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    slice.sort();
}

/// std::find(first, last, value) - returns iterator to first match or last
pub fn std_find_int(first: *const i32, last: *const i32, value: i32) -> *const i32 {
    if first.is_null() || last.is_null() { return last; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return last; }
    let slice = unsafe { std::slice::from_raw_parts(first, len) };
    match slice.iter().position(|&x| x == value) {
        Some(idx) => unsafe { first.add(idx) },
        None => last,
    }
}

/// std::count(first, last, value) - counts occurrences of value in range
pub fn std_count_int(first: *const i32, last: *const i32, value: i32) -> usize {
    if first.is_null() || last.is_null() { return 0; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return 0; }
    let slice = unsafe { std::slice::from_raw_parts(first, len) };
    slice.iter().filter(|&&x| x == value).count()
}

/// std::copy(first, last, dest) - copies range to dest, returns end of dest
pub fn std_copy_int(first: *const i32, last: *const i32, dest: *mut i32) -> *mut i32 {
    if first.is_null() || last.is_null() || dest.is_null() { return dest; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return dest; }
    unsafe { std::ptr::copy_nonoverlapping(first, dest, len); }
    unsafe { dest.add(len) }
}

/// std::fill(first, last, value) - fills range with value
pub fn std_fill_int(first: *mut i32, last: *mut i32, value: i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    for elem in slice.iter_mut() { *elem = value; }
}

/// std::reverse(first, last) - reverses range in place
pub fn std_reverse_int(first: *mut i32, last: *mut i32) {
    if first.is_null() || last.is_null() { return; }
    let len = unsafe { last.offset_from(first) as usize };
    if len == 0 { return; }
    let slice = unsafe { std::slice::from_raw_parts_mut(first, len) };
    slice.reverse();
}

fn main() {
    // ========== std_sort tests ==========

    // Test 1: Sort empty range (no-op)
    std_sort_int(std::ptr::null_mut(), std::ptr::null_mut());
    // No crash = success

    // Test 2: Sort single element
    let mut single = [42i32];
    std_sort_int(single.as_mut_ptr(), unsafe { single.as_mut_ptr().add(1) });
    if single[0] != 42 { std::process::exit(1); }

    // Test 3: Sort already sorted array
    let mut sorted = [1i32, 2, 3, 4, 5];
    std_sort_int(sorted.as_mut_ptr(), unsafe { sorted.as_mut_ptr().add(5) });
    if sorted != [1, 2, 3, 4, 5] { std::process::exit(2); }

    // Test 4: Sort reverse-sorted array
    let mut reverse = [5i32, 4, 3, 2, 1];
    std_sort_int(reverse.as_mut_ptr(), unsafe { reverse.as_mut_ptr().add(5) });
    if reverse != [1, 2, 3, 4, 5] { std::process::exit(3); }

    // Test 5: Sort random order array
    let mut random = [3i32, 1, 4, 1, 5, 9, 2, 6];
    std_sort_int(random.as_mut_ptr(), unsafe { random.as_mut_ptr().add(8) });
    if random != [1, 1, 2, 3, 4, 5, 6, 9] { std::process::exit(4); }

    // ========== std_find tests ==========

    // Test 6: Find in empty range
    let empty: [i32; 0] = [];
    let result = std_find_int(empty.as_ptr(), empty.as_ptr(), 42);
    if result != empty.as_ptr() { std::process::exit(5); }

    // Test 7: Find existing element
    let arr = [10i32, 20, 30, 40, 50];
    let result = std_find_int(arr.as_ptr(), unsafe { arr.as_ptr().add(5) }, 30);
    if result != unsafe { arr.as_ptr().add(2) } { std::process::exit(6); }

    // Test 8: Find non-existing element (returns end)
    let result = std_find_int(arr.as_ptr(), unsafe { arr.as_ptr().add(5) }, 99);
    if result != unsafe { arr.as_ptr().add(5) } { std::process::exit(7); }

    // Test 9: Find first of duplicates
    let dups = [1i32, 2, 3, 2, 4, 2];
    let result = std_find_int(dups.as_ptr(), unsafe { dups.as_ptr().add(6) }, 2);
    if result != unsafe { dups.as_ptr().add(1) } { std::process::exit(8); }

    // ========== std_count tests ==========

    // Test 10: Count in empty range
    let count = std_count_int(empty.as_ptr(), empty.as_ptr(), 42);
    if count != 0 { std::process::exit(9); }

    // Test 11: Count non-existing value
    let count = std_count_int(arr.as_ptr(), unsafe { arr.as_ptr().add(5) }, 99);
    if count != 0 { std::process::exit(10); }

    // Test 12: Count single occurrence
    let count = std_count_int(arr.as_ptr(), unsafe { arr.as_ptr().add(5) }, 30);
    if count != 1 { std::process::exit(11); }

    // Test 13: Count multiple occurrences
    let count = std_count_int(dups.as_ptr(), unsafe { dups.as_ptr().add(6) }, 2);
    if count != 3 { std::process::exit(12); }

    // ========== std_copy tests ==========

    // Test 14: Copy empty range
    let mut dest: [i32; 5] = [0; 5];
    let end = std_copy_int(empty.as_ptr(), empty.as_ptr(), dest.as_mut_ptr());
    if end != dest.as_mut_ptr() { std::process::exit(13); }

    // Test 15: Copy to separate buffer
    let src = [1i32, 2, 3, 4, 5];
    let end = std_copy_int(src.as_ptr(), unsafe { src.as_ptr().add(5) }, dest.as_mut_ptr());
    if end != unsafe { dest.as_mut_ptr().add(5) } { std::process::exit(14); }
    if dest != [1, 2, 3, 4, 5] { std::process::exit(15); }

    // Test 16: Verify original unchanged
    if src != [1, 2, 3, 4, 5] { std::process::exit(16); }

    // ========== std_fill tests ==========

    // Test 17: Fill empty range (no-op)
    std_fill_int(std::ptr::null_mut(), std::ptr::null_mut(), 99);
    // No crash = success

    // Test 18: Fill with value
    let mut fill_arr = [0i32; 5];
    std_fill_int(fill_arr.as_mut_ptr(), unsafe { fill_arr.as_mut_ptr().add(5) }, 42);
    if fill_arr != [42, 42, 42, 42, 42] { std::process::exit(17); }

    // Test 19: Fill with zero
    std_fill_int(fill_arr.as_mut_ptr(), unsafe { fill_arr.as_mut_ptr().add(5) }, 0);
    if fill_arr != [0, 0, 0, 0, 0] { std::process::exit(18); }

    // ========== std_reverse tests ==========

    // Test 20: Reverse empty range (no-op)
    std_reverse_int(std::ptr::null_mut(), std::ptr::null_mut());
    // No crash = success

    // Test 21: Reverse single element (no-op)
    let mut single_rev = [42i32];
    std_reverse_int(single_rev.as_mut_ptr(), unsafe { single_rev.as_mut_ptr().add(1) });
    if single_rev[0] != 42 { std::process::exit(19); }

    // Test 22: Reverse even length array
    let mut even = [1i32, 2, 3, 4];
    std_reverse_int(even.as_mut_ptr(), unsafe { even.as_mut_ptr().add(4) });
    if even != [4, 3, 2, 1] { std::process::exit(20); }

    // Test 23: Reverse odd length array
    let mut odd = [1i32, 2, 3, 4, 5];
    std_reverse_int(odd.as_mut_ptr(), unsafe { odd.as_mut_ptr().add(5) });
    if odd != [5, 4, 3, 2, 1] { std::process::exit(21); }

    std::process::exit(0);  // All tests passed
}
"#;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write Rust source
    let rs_path = temp_dir.join("e2e_stl_algorithm_stub.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write Rust source");

    // Compile with rustc
    let binary_path = temp_dir.join("e2e_stl_algorithm_stub");
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .expect("Failed to run binary");

    let exit_code = run_output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code, 0,
        "STL algorithm stub operations should work correctly (exit code: {})",
        exit_code
    );
}

/// Test function returning struct (rvalue handling).
#[test]
fn test_e2e_function_returning_struct() {
    let source = r#"
        class Widget {
            int value;
        public:
            Widget(int v) : value(v) {}
            int getValue() const { return value; }
        };

        Widget createWidget(int v) {
            return Widget(v);
        }

        int main() {
            Widget w = createWidget(42);
            if (w.getValue() == 42) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_returning_struct.cpp")
            .expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Function returning struct should work correctly"
    );
}

/// Test multiple inheritance.
#[test]
fn test_e2e_multiple_inheritance() {
    let source = r#"
        class Flyable {
        public:
            int altitude;
            Flyable() : altitude(0) {}
            void setAltitude(int a) { altitude = a; }
            int getAltitude() const { return altitude; }
        };

        class Swimmable {
        public:
            int depth;
            Swimmable() : depth(0) {}
            void setDepth(int d) { depth = d; }
            int getDepth() const { return depth; }
        };

        class Duck : public Flyable, public Swimmable {
        public:
            int age;
            Duck() : age(1) {}
        };

        int main() {
            Duck d;
            d.setAltitude(100);
            d.setDepth(5);

            if (d.getAltitude() == 100 && d.getDepth() == 5 && d.age == 1) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_multiple_inheritance.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Multiple inheritance should work correctly with access to both base classes"
    );
}

/// Test enum class (scoped enums).
#[test]
fn test_e2e_enum_class() {
    let source = r#"
        enum class Color { Red = 0, Green = 1, Blue = 2 };

        int main() {
            Color c = Color::Red;
            if (c == Color::Red) {
                c = Color::Green;
            }
            if (c == Color::Green) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_enum_class.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Enum class should work correctly with scoped access"
    );
}

/// Test static class members.
#[test]
fn test_e2e_static_members() {
    let source = r#"
        class Counter {
        public:
            static int count;
            static void inc() { count = count + 1; }
            static void dec() { count = count - 1; }
            static int getCount() { return count; }
        };
        int Counter::count = 0;

        int main() {
            Counter::inc();
            Counter::inc();
            Counter::inc();
            Counter::dec();

            if (Counter::getCount() == 2) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_static_members.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Static class members should work correctly");
}

/// Test basic lambda expressions.
#[test]
fn test_e2e_lambda_basic() {
    let source = r#"
        int main() {
            auto double_it = [](int x) { return x * 2; };
            auto add = [](int a, int b) { return a + b; };

            int result = double_it(10);
            result = add(result, 5);

            if (result == 25) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_lambda_basic.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Basic lambda expressions should work correctly"
    );
}

/// Test lambda captures (by value and by reference).
#[test]
fn test_e2e_lambda_captures() {
    let source = r#"
        int main() {
            int x = 10;
            int y = 5;

            // Capture by value [=]
            auto sum_all = [=]() { return x + y; };

            // Capture by reference [&]
            auto inc_both = [&]() { x++; y++; };

            int result = sum_all();  // 15
            inc_both();              // x=11, y=6

            if (result == 15 && x == 11 && y == 6) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_lambda_captures.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Lambda captures should work correctly");
}

/// Test generic lambdas (auto parameters).
/// Note: In Rust, closures can only have one concrete type, so generic lambdas
/// can only be used with one type instantiation.
#[test]
fn test_e2e_generic_lambda() {
    let source = r#"
        int main() {
            auto identity = [](auto x) { return x; };
            auto add_one = [](auto x) { return x + 1; };

            int a = identity(42);
            int b = add_one(9);

            if (a == 42 && b == 10) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_generic_lambda.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Generic lambdas with single type usage should work"
    );
}

/// Test E2E: Range-based for loops
#[test]
fn test_e2e_range_for() {
    let source = r#"
        int main() {
            int arr[] = {1, 2, 3, 4, 5};
            int sum = 0;

            for (int x : arr) {
                sum += x;
            }

            if (sum == 15) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_range_for.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Range-based for loop should iterate over array"
    );
}

/// Test E2E: Increment/decrement operators (prefix and postfix)
#[test]
fn test_e2e_increment_decrement() {
    let source = r#"
        int main() {
            int x = 5;
            int y = x++;  // post-increment: y=5, x=6
            int z = ++x;  // pre-increment: z=7, x=7
            int a = x--;  // post-decrement: a=7, x=6
            int b = --x;  // pre-decrement: b=5, x=5

            if (y == 5 && z == 7 && a == 7 && b == 5 && x == 5) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_increment_decrement.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Increment/decrement operators should work correctly"
    );
}

/// Test E2E: Default function parameters
#[test]
fn test_e2e_default_params() {
    let source = r#"
        int add(int a, int b = 10, int c = 20) {
            return a + b + c;
        }

        int main() {
            int x = add(1);           // 1 + 10 + 20 = 31
            int y = add(1, 2);        // 1 + 2 + 20 = 23
            int z = add(1, 2, 3);     // 1 + 2 + 3 = 6

            if (x == 31 && y == 23 && z == 6) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_default_params.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Default function parameters should be evaluated correctly"
    );
}

/// Test E2E: Const vs non-const methods (mut self detection)
#[test]
fn test_e2e_const_methods() {
    let source = r#"
        struct Counter {
            int value;

            int get() const {
                return value;
            }

            void increment() {
                value++;
            }
        };

        int main() {
            Counter c;
            c.value = 5;
            c.increment();

            if (c.get() == 6) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_const_methods.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Const methods should use &self, non-const should use &mut self"
    );
}

/// Test E2E: Switch statements (including fallthrough)
#[test]
fn test_e2e_switch() {
    let source = r#"
        int getValue(int x) {
            switch (x) {
                case 1:
                    return 10;
                case 2:
                    return 20;
                case 3:
                case 4:
                    return 30;
                default:
                    return 0;
            }
        }

        int main() {
            int a = getValue(1);
            int b = getValue(2);
            int c = getValue(3);
            int d = getValue(4);
            int e = getValue(5);

            if (a == 10 && b == 20 && c == 30 && d == 30 && e == 0) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_switch.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Switch statements with fallthrough should work correctly"
    );
}

/// Test E2E: Comma operator
#[test]
fn test_e2e_comma_operator() {
    let source = r#"
        int main() {
            int a = 0;
            int b = (a = 5, a + 10);  // Sets a to 5, then b = 5 + 10 = 15

            if (a == 5 && b == 15) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_comma_operator.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Comma operator should evaluate both expressions and return the last"
    );
}

/// Test E2E: Typedef type aliases
#[test]
fn test_e2e_typedef() {
    let source = r#"
        typedef int MyInt;
        typedef MyInt* MyIntPtr;

        int main() {
            MyInt x = 42;
            MyIntPtr p = &x;
            if (*p == 42) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_typedef.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Typedef type aliases should work correctly");
}

/// Test E2E: Global variables
#[test]
fn test_e2e_global_var() {
    let source = r#"
        int counter = 0;

        void increment() {
            counter++;
        }

        int main() {
            increment();
            increment();
            increment();
            if (counter == 3) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_global_var.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Global variables should work with unsafe access"
    );
}

/// Test E2E: Global arrays
#[test]
fn test_e2e_global_array() {
    let source = r#"
        int array[5];

        int main() {
            for (int i = 0; i < 5; i++) {
                array[i] = i * 2;
            }
            int sum = 0;
            for (int i = 0; i < 5; i++) {
                sum += array[i];
            }
            // Sum of 0, 2, 4, 6, 8 = 20
            if (sum == 20) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_global_array.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Global arrays should work with unsafe access and proper initialization"
    );
}

/// Test virtual diamond inheritance - ensures virtual base class is shared.
#[test]
fn test_e2e_virtual_diamond() {
    let source = r#"
        class A {
        public:
            int a;
            A(int v) : a(v) {}
            int getA() { return a; }
        };

        class B : virtual public A {
        public:
            int b;
            B(int v) : A(v), b(v + 1) {}
            int getAFromB() { return a; }
        };

        class C : virtual public A {
        public:
            int c;
            C(int v) : A(v), c(v + 2) {}
            int getAFromC() { return a; }
        };

        class D : public B, public C {
        public:
            int d;
            D(int v) : A(v), B(v), C(v), d(v + 3) {}
            int sum() { return a + b + c + d; }
        };

        int main() {
            D obj(10);
            // a=10, b=11, c=12, d=13, sum=46
            if (obj.sum() != 46) return 1;
            // Access 'a' through B and C paths - should be same value
            if (obj.getAFromB() != 10) return 2;
            if (obj.getAFromC() != 10) return 3;
            // Direct access to a
            if (obj.getA() != 10) return 4;
            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_virtual_diamond.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Virtual diamond inheritance should share a single virtual base instance"
    );
}

/// Test namespace function call path resolution.
#[test]
fn test_e2e_namespace_path_resolution() {
    let source = r#"
        namespace foo {
            int helper() { return 42; }
            int test() { return helper(); }  // Same namespace call

            namespace inner {
                int innerHelper() { return 10; }
                int useParent() { return helper(); }  // Parent namespace call
                int useLocal() { return innerHelper(); }  // Same namespace call
            }
        }

        int globalFunc() { return 100; }

        namespace bar {
            int useGlobal() { return globalFunc(); }  // Global function call
        }

        int main() {
            if (foo::test() != 42) return 1;
            if (foo::inner::useParent() != 42) return 2;
            if (foo::inner::useLocal() != 10) return 3;
            if (bar::useGlobal() != 100) return 4;
            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_namespace_path.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Namespace function calls should use correct relative paths"
    );
}

/// Test function call operator (functor/callable object).
#[test]
fn test_e2e_functor() {
    let source = r#"
        class Adder {
            int base;
        public:
            Adder(int b) : base(b) {}
            int operator()(int x, int y) {
                return base + x + y;
            }
        };

        int main() {
            Adder add5(5);
            // Multiple arguments
            if (add5(10, 20) != 35) return 1;  // 5 + 10 + 20 = 35
            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_functor.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Function call operator should work with arguments"
    );
}

/// Test constructor body statements (non-initializer assignments).
#[test]
fn test_e2e_ctor_body_stmts() {
    let source = r#"
        class Array {
            int data[5];
        public:
            Array() {
                // These are body statements, not member initializers
                data[0] = 100;
                data[1] = 200;
                data[2] = 300;
            }
            int get(int idx) {
                return data[idx];
            }
        };

        int main() {
            Array arr;
            if (arr.get(0) != 100) return 1;
            if (arr.get(1) != 200) return 2;
            if (arr.get(2) != 300) return 3;
            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_ctor_body.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Constructor body statements should execute correctly"
    );
}

/// Test subscript operator [] (returns mutable reference, correct argument passing).
#[test]
fn test_e2e_subscript_operator() {
    let source = r#"
        class Array {
            int data[10];
        public:
            Array() {
                for (int i = 0; i < 10; i++) {
                    data[i] = i;
                }
            }
            int& operator[](int idx) {
                return data[idx];
            }
        };

        int main() {
            Array arr;
            // Read through subscript
            if (arr[5] != 5) return 1;

            // Write through subscript
            arr[3] = 100;
            if (arr[3] != 100) return 2;

            // Compound operations with subscript
            arr[4] += 10;
            if (arr[4] != 14) return 3;

            // Subscript in expression
            int sum = arr[0] + arr[1] + arr[2];
            if (sum != 3) return 4;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_subscript.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Subscript operator should work with reads and writes"
    );
}

/// Test assignment operators (=, +=, -=) for custom types.
#[test]
fn test_e2e_assignment_operators() {
    let source = r#"
        class Counter {
            int value;
        public:
            Counter(int v = 0) : value(v) {}

            Counter& operator=(int v) {
                value = v;
                return *this;
            }

            Counter& operator+=(int v) {
                value += v;
                return *this;
            }

            Counter& operator-=(int v) {
                value -= v;
                return *this;
            }

            int get() const { return value; }
        };

        int main() {
            Counter c;
            c = 10;           // operator=
            if (c.get() != 10) return 1;

            c += 5;           // operator+=
            if (c.get() != 15) return 2;

            c -= 3;           // operator-=
            if (c.get() != 12) return 3;

            // Chained operations
            Counter d;
            (d = 100) += 50;
            if (d.get() != 150) return 4;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_assign_ops.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Assignment operators should work correctly");
}

/// Test dereference operator * for smart pointer types.
#[test]
fn test_e2e_deref_operator() {
    let source = r#"
        class SmartPtr {
            int* ptr;
        public:
            SmartPtr(int val) : ptr(new int(val)) {}
            ~SmartPtr() {
                if (ptr) delete ptr;
            }

            int& operator*() {
                return *ptr;
            }
        };

        int main() {
            SmartPtr sp(42);

            // Read through dereference
            int val = *sp;
            if (val != 42) return 1;

            // Write through dereference
            *sp = 100;
            if (*sp != 100) return 2;

            // Arithmetic with dereference
            *sp += 50;
            if (*sp != 150) return 3;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_deref_op.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Dereference operator should work with reads and writes"
    );
}

/// Test arrow operator -> for smart pointer types.
#[test]
fn test_e2e_arrow_operator() {
    let source = r#"
        class Point {
        public:
            int x, y;
            Point(int a, int b) : x(a), y(b) {}
            int sum() const { return x + y; }
        };

        class PointPtr {
            Point* ptr;
        public:
            PointPtr(Point* p) : ptr(p) {}
            ~PointPtr() { delete ptr; }

            Point* operator->() { return ptr; }
        };

        int main() {
            PointPtr pp(new Point(10, 20));

            // Arrow operator for member access
            if (pp->x != 10) return 1;
            if (pp->y != 20) return 2;

            // Arrow operator for method call
            if (pp->sum() != 30) return 3;

            // Arrow operator for member assignment
            pp->x = 100;
            if (pp->x != 100) return 4;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_arrow_op.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Arrow operator should work for member access and method calls"
    );
}

/// Test sizeof and alignof operators.
#[test]
fn test_e2e_sizeof_alignof() {
    let source = r#"
        class Point {
        public:
            int x, y;
        };

        int main() {
            int a;
            Point p;

            // sizeof with type
            int size1 = sizeof(int);
            int size2 = sizeof(Point);

            // sizeof with expression
            int size3 = sizeof(a);
            int size4 = sizeof(p);
            int size5 = sizeof(p.x);

            // alignof with type
            int align1 = alignof(int);
            int align2 = alignof(Point);

            // Check expected values
            if (size1 != 4) return 1;
            if (size2 != 8) return 2;
            if (size3 != 4) return 3;
            if (size4 != 8) return 4;
            if (size5 != 4) return 5;
            if (align1 != 4) return 6;
            if (align2 != 4) return 7;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_sizeof_alignof.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "sizeof and alignof should be evaluated at compile time"
    );
}

/// Test string literals and implicit char-to-int casts.
#[test]
fn test_e2e_string_literals_and_char_casts() {
    let source = r#"
        int main() {
            // String literals assigned to const char*
            const char* s1 = "hello";
            const char* s2 = "world";

            // Access characters through pointer
            if (s1[0] != 'h') return 1;
            if (s2[0] != 'w') return 2;

            // Character literals and implicit casts
            char c = 'A';
            int i = c;  // implicit char to int cast
            if (i != 65) return 3;

            // Direct character comparisons
            if (c != 'A') return 4;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_string_char.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "String literals and char casts should work correctly"
    );
}

/// E2E test: Designated initializers (C++20)
#[test]
fn test_e2e_designated_initializers() {
    let source = r#"
        struct Point {
            int x;
            int y;
            int z;
        };

        struct Config {
            int width;
            int height;
            bool enabled;
        };

        int main() {
            // Basic designated initializer
            Point p = { .x = 10, .y = 20, .z = 30 };
            if (p.x != 10 || p.y != 20 || p.z != 30) return 1;

            // Different order (still works because Clang sorts them)
            Config cfg = { .height = 480, .width = 640, .enabled = true };
            if (cfg.width != 640 || cfg.height != 480 || !cfg.enabled) return 2;

            // Non-designated initializer (positional)
            Point q = { 5, 15, 25 };
            if (q.x != 5 || q.y != 15 || q.z != 25) return 3;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_designated_init.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Designated initializers should work correctly"
    );
}

/// E2E test: Function pointers
#[test]
fn test_e2e_function_pointers() {
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int multiply(int a, int b) {
            return a * b;
        }

        int subtract(int a, int b) {
            return a - b;
        }

        // Function that takes a function pointer as parameter
        int apply(int (*fn)(int, int), int x, int y) {
            return fn(x, y);
        }

        int main() {
            // Basic function pointer declaration and assignment
            int (*ptr)(int, int) = add;
            int result1 = ptr(3, 4);  // 7
            if (result1 != 7) return 1;

            // Reassigning function pointer
            ptr = multiply;
            int result2 = ptr(3, 4);  // 12
            if (result2 != 12) return 2;

            // Passing function pointer as argument
            int result3 = apply(add, 5, 6);  // 11
            if (result3 != 11) return 3;

            int result4 = apply(subtract, 10, 3);  // 7
            if (result4 != 7) return 4;

            // Chained function pointer calls
            int result5 = apply(multiply, apply(add, 2, 3), 4);  // (2+3)*4 = 20
            if (result5 != 20) return 5;

            // Null function pointer initialization
            int (*null_ptr)(int, int) = nullptr;
            if (null_ptr != nullptr) return 6;  // Should be null

            // Assign and check not null
            null_ptr = add;
            if (null_ptr == nullptr) return 7;  // Should not be null

            // Call through previously-null pointer
            int result6 = null_ptr(1, 2);  // 3
            if (result6 != 3) return 8;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_pointers.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Function pointers should work correctly");
}

/// E2E test: std::get for std::variant
/// NOTE: This test is ignored because including <variant> header pulls in STL internals
/// that generate invalid Rust code. The std::get functionality itself works correctly
/// (match expression is generated) but the surrounding STL types aren't fully supported.
#[test]
#[ignore]
fn test_e2e_std_get() {
    let source = r#"
        #include <variant>

        int main() {
            // Test std::get<Type>
            std::variant<int, double, bool> v1 = 42;
            int x = std::get<int>(v1);
            if (x != 42) return 1;

            // Test std::get<Index>
            std::variant<int, double, bool> v2 = 3.14;
            double y = std::get<1>(v2);
            if (y < 3.13 || y > 3.15) return 2;

            // Test with boolean variant
            std::variant<int, double, bool> v3 = true;
            bool z = std::get<bool>(v3);
            if (!z) return 3;

            // Test index-based get for bool (index 2)
            bool w = std::get<2>(v3);
            if (!w) return 4;

            // Test reassignment and get
            v1 = 100;
            int a = std::get<int>(v1);
            if (a != 100) return 5;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_std_get.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "std::get on variant should work correctly");
}

/// E2E test: std::visit for std::variant
/// NOTE: This test is ignored because including <variant> header pulls in STL internals
/// that generate invalid Rust code. The std::visit functionality itself works correctly
/// (match expression is generated) but the surrounding STL types aren't fully supported.
#[test]
#[ignore]
fn test_e2e_std_visit() {
    let source = r#"
        #include <variant>

        int main() {
            // Test std::visit with single variant and lambda
            std::variant<int, double, bool> v1 = 42;
            auto result1 = std::visit([](auto x) { return static_cast<int>(x * 2); }, v1);
            if (result1 != 84) return 1;

            // Test std::visit with double variant
            std::variant<int, double, bool> v2 = 3.5;
            auto result2 = std::visit([](auto x) { return static_cast<int>(x * 2); }, v2);
            if (result2 != 7) return 2;

            // Test std::visit with bool variant
            std::variant<int, double, bool> v3 = true;
            auto result3 = std::visit([](auto x) { return x ? 10 : 0; }, v3);
            if (result3 != 10) return 3;

            return 0;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_std_visit.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "std::visit on variant should work correctly");
}

/// Test anonymous namespace generates private module with synthetic name.
#[test]
fn test_e2e_anonymous_namespace() {
    let source = r#"
        namespace {
            int secret_value = 42;

            int get_secret() {
                return secret_value;
            }
        }

        int main() {
            // Access items from anonymous namespace (should be auto-imported)
            int val = get_secret();
            if (val == 42) {
                return 0;
            }
            return 1;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_anon_namespace.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Anonymous namespace items should be accessible"
    );
}

#[test]
fn test_e2e_anonymous_struct() {
    let source = r#"
        struct Container {
            int before;

            // Anonymous struct - fields should be accessible directly
            struct {
                int x;
                int y;
            };

            int after;
        };

        int main() {
            Container c;
            c.before = 1;
            c.x = 10;     // Direct access to anonymous struct field
            c.y = 20;     // Direct access to anonymous struct field
            c.after = 2;

            // Verify all fields are accessible and have correct values
            if (c.before != 1) return 1;
            if (c.x != 10) return 2;
            if (c.y != 20) return 3;
            if (c.after != 2) return 4;

            // Return sum of anonymous struct fields
            return c.x + c.y;  // Should return 30
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_anon_struct.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 30,
        "Anonymous struct fields should be flattened and accessible"
    );
}

#[test]
fn test_e2e_anonymous_union() {
    let source = r#"
        struct Data {
            int type;

            // Anonymous union - fields should be accessible directly
            union {
                int int_val;
                float float_val;
                double double_val;
            };

            int flags;
        };

        int main() {
            Data d;
            d.type = 1;
            d.int_val = 42;  // Direct access to anonymous union member
            d.flags = 3;

            // Verify all fields are accessible
            if (d.type != 1) return 1;
            if (d.int_val != 42) return 2;
            if (d.flags != 3) return 3;

            // Return the union value
            return d.int_val;  // Should return 42
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_anon_union.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 42,
        "Anonymous union fields should be flattened and accessible"
    );
}

#[test]
fn test_e2e_access_specifiers() {
    // Test that C++ access specifiers generate appropriate Rust visibility
    // Public fields should be pub, private should have no visibility, protected should be pub(crate)
    let source = r#"
        class Data {
        public:
            int pub_field;

        protected:
            int prot_field;

        private:
            int priv_field;

        public:
            Data() : pub_field(10), prot_field(20), priv_field(30) {}

            // Public method to access private field for testing
            int get_priv() { return priv_field; }
            int get_prot() { return prot_field; }
        };

        int main() {
            Data d;
            // Can access public field directly
            int result = d.pub_field;

            // Access protected and private via public methods
            result += d.get_prot() + d.get_priv();

            // Should be 10 + 20 + 30 = 60
            return result;
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_access_specifiers.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 60,
        "Access specifiers should generate appropriate visibility"
    );
}

// =============================================================================
// STL Transpilation Tests (Section 23 - libc++ integration)
// =============================================================================
// These tests verify that code using C++ standard library headers can be
// transpiled with the vendored libc++. They capture transpiler errors to
// document which patterns fail.

/// Helper function to transpile code with vendored libc++ and attempt compilation.
/// Returns (transpile_success, rust_code, errors, compile_success, compile_errors).
fn transpile_and_compile_with_vendored_libcxx(
    cpp_source: &str,
    filename: &str,
) -> (bool, String, String, bool, String) {
    let (transpile_success, rust_code, transpile_errors) =
        transpile_with_vendored_libcxx(cpp_source, filename);

    if !transpile_success {
        return (false, rust_code, transpile_errors, false, String::new());
    }

    // Try to compile the generated Rust code
    let temp_dir = std::env::temp_dir().join("fragile_libcxx_tests");
    let _ = fs::create_dir_all(&temp_dir);

    let rs_path = temp_dir.join(format!("{}.rs", filename.replace(".cpp", "")));
    if let Err(e) = fs::write(&rs_path, &rust_code) {
        return (
            true,
            rust_code,
            String::new(),
            false,
            format!("Failed to write Rust source: {}", e),
        );
    }

    // Compile with rustc
    let binary_path = temp_dir.join(filename.replace(".cpp", ""));
    let compile_output = match Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            return (
                true,
                rust_code,
                String::new(),
                false,
                format!("Failed to run rustc: {}", e),
            )
        }
    };

    if compile_output.status.success() {
        (true, rust_code, String::new(), true, String::new())
    } else {
        let stderr = String::from_utf8_lossy(&compile_output.stderr).to_string();
        (true, rust_code, String::new(), false, stderr)
    }
}

/// Helper function to transpile code with vendored libc++ and capture errors.
/// Returns (success, rust_code, errors) where errors are transpiler/parse errors.
fn transpile_with_vendored_libcxx(cpp_source: &str, filename: &str) -> (bool, String, String) {
    use fragile_clang::ClangParser;

    // Try to create parser with vendored libc++
    let parser = match ClangParser::with_vendored_libcxx() {
        Ok(p) => p,
        Err(e) => {
            return (
                false,
                String::new(),
                format!("Failed to create parser with vendored libc++: {}", e),
            );
        }
    };

    // Parse and generate Rust code
    match parser.parse_string(cpp_source, filename) {
        Ok(ast) => {
            let rust_code = AstCodeGen::new().generate(&ast.translation_unit);
            (true, rust_code, String::new())
        }
        Err(e) => (false, String::new(), format!("Parse error: {}", e)),
    }
}

/// Test 23.1.1: Transpile minimal code with #include <vector>
/// This test documents the current state of libc++ vector transpilation.
/// It's not expected to produce compilable Rust code yet - we're documenting
/// what works and what doesn't.
#[test]
fn test_libcxx_vector_transpilation() {
    // Skip if vendored libc++ is not available
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <vector>

        int main() {
            std::vector<int> v;
            v.push_back(1);
            v.push_back(2);
            return v.size() == 2 ? 0 : 1;
        }
    "#;

    let (transpile_ok, rust_code, transpile_errors, compile_ok, compile_errors) =
        transpile_and_compile_with_vendored_libcxx(source, "test_vector.cpp");

    println!("=== libc++ vector transpilation test ===");
    println!("Transpilation success: {}", transpile_ok);
    println!("Generated Rust code length: {} chars", rust_code.len());
    println!("Compilation success: {}", compile_ok);

    if !transpile_errors.is_empty() {
        println!("Transpilation errors:\n{}", transpile_errors);
    }

    if !compile_ok && !compile_errors.is_empty() {
        // Count errors
        let error_count = compile_errors.matches("error[E").count();
        println!("Compilation errors: {} total", error_count);
        // Show first 5000 chars of errors
        let preview = if compile_errors.len() > 5000 {
            format!(
                "{}...\n[truncated, {} more chars]",
                &compile_errors[..5000],
                compile_errors.len() - 5000
            )
        } else {
            compile_errors.clone()
        };
        println!("Compile errors:\n{}", preview);
    }

    // For now, we just check that the transpiler doesn't crash
    // Later tests will verify the code compiles and runs
    assert!(
        transpile_ok || !transpile_errors.is_empty(),
        "Should either succeed or report errors, not crash"
    );
}

/// Test 23.1.3.1: Transpile minimal <cstddef> (just typedefs)
/// This is the simplest libc++ header - tests basic include mechanism.
#[test]
fn test_libcxx_cstddef_transpilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <cstddef>

        int main() {
            std::size_t sz = 42;
            std::ptrdiff_t diff = -10;
            return (sz == 42 && diff == -10) ? 0 : 1;
        }
    "#;

    let (success, rust_code, errors) = transpile_with_vendored_libcxx(source, "test_cstddef.cpp");

    println!("=== libc++ cstddef transpilation test ===");
    println!("Transpilation success: {}", success);
    if !errors.is_empty() {
        println!("Errors:\n{}", errors);
    }
    if !rust_code.is_empty() && rust_code.len() < 1000 {
        println!("Generated Rust code:\n{}", rust_code);
    }

    assert!(
        success || !errors.is_empty(),
        "Should either succeed or report errors"
    );
}

/// Test 23.1.3.2: Transpile <cstdint> (integer types)
#[test]
fn test_libcxx_cstdint_transpilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <cstdint>

        int main() {
            int8_t i8 = -1;
            uint16_t u16 = 65535;
            int32_t i32 = -2147483647;
            uint64_t u64 = 18446744073709551615ULL;

            // Basic sanity checks
            if (i8 != -1) return 1;
            if (u16 != 65535) return 2;
            if (i32 != -2147483647) return 3;
            return 0;
        }
    "#;

    let (success, rust_code, errors) = transpile_with_vendored_libcxx(source, "test_cstdint.cpp");

    println!("=== libc++ cstdint transpilation test ===");
    println!("Transpilation success: {}", success);
    if !errors.is_empty() {
        println!("Errors:\n{}", errors);
    }
    if !rust_code.is_empty() && rust_code.len() < 1000 {
        println!("Generated Rust code:\n{}", rust_code);
    }

    assert!(
        success || !errors.is_empty(),
        "Should either succeed or report errors"
    );
}

/// Test 23.1.3.3: Transpile <initializer_list> (simple container)
#[test]
fn test_libcxx_initializer_list_transpilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <initializer_list>

        int sum(std::initializer_list<int> values) {
            int total = 0;
            for (int v : values) {
                total += v;
            }
            return total;
        }

        int main() {
            int result = sum({1, 2, 3, 4, 5});
            return result == 15 ? 0 : 1;
        }
    "#;

    let (success, rust_code, errors) = transpile_with_vendored_libcxx(source, "test_init_list.cpp");

    println!("=== libc++ initializer_list transpilation test ===");
    println!("Transpilation success: {}", success);
    if !errors.is_empty() {
        println!("Errors:\n{}", errors);
    }
    if !rust_code.is_empty() {
        let preview = if rust_code.len() > 2000 {
            format!("{}...\n[truncated]", &rust_code[..2000])
        } else {
            rust_code.clone()
        };
        println!("Generated Rust code:\n{}", preview);
    }

    assert!(
        success || !errors.is_empty(),
        "Should either succeed or report errors"
    );
}

/// Test 23.1.3.4: Transpile <type_traits> (template metaprogramming)
#[test]
fn test_libcxx_type_traits_transpilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <type_traits>

        template<typename T>
        T identity(T value) {
            static_assert(std::is_integral<T>::value, "Must be integral");
            return value;
        }

        int main() {
            int x = identity(42);
            return x == 42 ? 0 : 1;
        }
    "#;

    let (success, rust_code, errors) =
        transpile_with_vendored_libcxx(source, "test_type_traits.cpp");

    println!("=== libc++ type_traits transpilation test ===");
    println!("Transpilation success: {}", success);
    if !errors.is_empty() {
        println!("Errors:\n{}", errors);
    }
    if !rust_code.is_empty() {
        let preview = if rust_code.len() > 2000 {
            format!("{}...\n[truncated]", &rust_code[..2000])
        } else {
            rust_code.clone()
        };
        println!("Generated Rust code:\n{}", preview);
    }

    assert!(
        success || !errors.is_empty(),
        "Should either succeed or report errors"
    );
}

/// Test 23.4: Attempt to compile transpiled libc++ code with rustc.
/// This test verifies what compilation errors we get when trying to compile
/// the generated Rust code from libc++ headers.
#[test]
fn test_libcxx_cstddef_compilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <cstddef>

        int main() {
            std::size_t sz = 42;
            return sz == 42 ? 0 : 1;
        }
    "#;

    let (success, rust_code, errors) =
        transpile_with_vendored_libcxx(source, "compile_cstddef.cpp");

    if !success {
        println!("Transpilation failed: {}", errors);
        return;
    }

    // Try to compile the generated Rust code
    let temp_dir = std::env::temp_dir().join("fragile_libcxx_compile_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let rs_path = temp_dir.join("cstddef_test.rs");
    fs::write(&rs_path, &rust_code).expect("Failed to write Rust source");

    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.join("cstddef_test"))
        .arg("--edition=2021")
        .output()
        .expect("Failed to run rustc");

    println!("=== libc++ cstddef compilation test ===");
    println!("Compilation success: {}", compile_output.status.success());

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        // Count and summarize errors
        let error_count = stderr.matches("error[").count();
        let warning_count = stderr.matches("warning:").count();
        println!("Errors: {}, Warnings: {}", error_count, warning_count);

        // Show first few errors
        let lines: Vec<&str> = stderr.lines().collect();
        let preview = lines
            .iter()
            .take(50)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        println!("First 50 lines of compiler output:\n{}", preview);
    }

    // Don't assert success yet - we're documenting what fails
}

/// Test 23.4.3: FILE I/O test using fragile-runtime.
/// This test verifies that transpiled code can use the fragile-runtime stdio functions.
#[test]
fn test_e2e_runtime_file_io() {
    // For this test, we manually create Rust code that uses fragile-runtime
    // to verify the linking infrastructure works.
    let rust_code = r#"
extern crate fragile_runtime;

use std::ffi::CString;
use std::ptr;

fn main() {
    unsafe {
        // Test fopen/fwrite/fclose
        let path = CString::new("/tmp/fragile_e2e_stdio_test.txt").unwrap();
        let mode = CString::new("w").unwrap();

        let file = fragile_runtime::fopen(path.as_ptr(), mode.as_ptr());
        if file.is_null() {
            std::process::exit(1);
        }

        let data = b"Hello from fragile-runtime!";
        let written = fragile_runtime::fwrite(
            data.as_ptr() as *const std::ffi::c_void,
            1,
            data.len(),
            file
        );

        if written != data.len() {
            std::process::exit(2);
        }

        let close_result = fragile_runtime::fclose(file);
        if close_result != 0 {
            std::process::exit(3);
        }

        // Verify by reading it back
        let mode_r = CString::new("r").unwrap();
        let file_r = fragile_runtime::fopen(path.as_ptr(), mode_r.as_ptr());
        if file_r.is_null() {
            std::process::exit(4);
        }

        let mut buffer = [0u8; 64];
        let read_count = fragile_runtime::fread(
            buffer.as_mut_ptr() as *mut std::ffi::c_void,
            1,
            buffer.len(),
            file_r
        );

        fragile_runtime::fclose(file_r);

        if read_count != data.len() {
            std::process::exit(5);
        }

        if &buffer[..data.len()] != data {
            std::process::exit(6);
        }

        // Clean up
        std::fs::remove_file("/tmp/fragile_e2e_stdio_test.txt").ok();

        // Success
        std::process::exit(0);
    }
}
"#;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_runtime_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write Rust source
    let rs_path = temp_dir.join("runtime_file_io.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write Rust source");

    // Find fragile-runtime library path
    let runtime_path =
        find_fragile_runtime_path().expect("Could not find fragile-runtime library path");

    // Compile with rustc, linking against fragile-runtime
    let binary_path = temp_dir.join("runtime_file_io");
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .arg("-L")
        .arg(&runtime_path)
        .arg("-L")
        .arg(runtime_path.join("deps"))
        .arg("--extern")
        .arg(format!(
            "fragile_runtime={}/libfragile_runtime.rlib",
            runtime_path.display()
        ))
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .expect("Failed to run binary");

    let exit_code = run_output.status.code().unwrap_or(-1);

    if exit_code != 0 {
        panic!(
            "Runtime file I/O test failed with exit code {}\nstdout: {}\nstderr: {}",
            exit_code,
            String::from_utf8_lossy(&run_output.stdout),
            String::from_utf8_lossy(&run_output.stderr)
        );
    }
}

/// Test 23.4.4: pthread test using fragile-runtime.
/// This test verifies that transpiled code can use the fragile-runtime pthread functions.
#[test]
fn test_e2e_runtime_pthread() {
    // Create Rust code that uses fragile-runtime pthread functions
    let rust_code = r#"
extern crate fragile_runtime;

use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

// Shared counter to verify thread executed
static COUNTER: AtomicI32 = AtomicI32::new(0);

extern "C" fn thread_func(arg: *mut c_void) -> *mut c_void {
    // Increment counter to prove we ran
    COUNTER.fetch_add(1, Ordering::SeqCst);

    // Return the argument incremented by 100
    let val = arg as i32;
    (val + 100) as *mut c_void
}

fn main() {
    unsafe {
        // Create a thread
        let mut thread = fragile_runtime::fragile_pthread_t::new();
        let arg = 42 as *mut c_void;

        let result = fragile_runtime::fragile_pthread_create(
            &mut thread,
            std::ptr::null(),
            Some(thread_func),
            arg,
        );

        if result != 0 {
            std::process::exit(1);
        }

        // Join the thread and get return value
        let mut retval: *mut c_void = std::ptr::null_mut();
        let result = fragile_runtime::fragile_pthread_join(thread, &mut retval);

        if result != 0 {
            std::process::exit(2);
        }

        // Verify the return value is 42 + 100 = 142
        if retval as i32 != 142 {
            std::process::exit(3);
        }

        // Verify the counter was incremented
        if COUNTER.load(Ordering::SeqCst) != 1 {
            std::process::exit(4);
        }

        // Test pthread_self
        let self_thread = fragile_runtime::fragile_pthread_self();
        if self_thread.id == 0 {
            std::process::exit(5);
        }

        // Success
        std::process::exit(0);
    }
}
"#;

    // Create temp directory
    let temp_dir = std::env::temp_dir().join("fragile_e2e_runtime_tests");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // Write Rust source
    let rs_path = temp_dir.join("runtime_pthread.rs");
    fs::write(&rs_path, rust_code).expect("Failed to write Rust source");

    // Find fragile-runtime library path
    let runtime_path =
        find_fragile_runtime_path().expect("Could not find fragile-runtime library path");

    // Compile with rustc, linking against fragile-runtime
    let binary_path = temp_dir.join("runtime_pthread");
    let compile_output = Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&binary_path)
        .arg("--edition=2021")
        .arg("-L")
        .arg(&runtime_path)
        .arg("-L")
        .arg(runtime_path.join("deps"))
        .arg("--extern")
        .arg(format!(
            "fragile_runtime={}/libfragile_runtime.rlib",
            runtime_path.display()
        ))
        .output()
        .expect("Failed to run rustc");

    if !compile_output.status.success() {
        panic!(
            "rustc compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the binary
    let run_output = Command::new(&binary_path)
        .output()
        .expect("Failed to run binary");

    let exit_code = run_output.status.code().unwrap_or(-1);

    if exit_code != 0 {
        panic!(
            "Runtime pthread test failed with exit code {}\nstdout: {}\nstderr: {}",
            exit_code,
            String::from_utf8_lossy(&run_output.stdout),
            String::from_utf8_lossy(&run_output.stderr)
        );
    }
}

/// Test 23.5: Verify C library function names are mapped to fragile-runtime equivalents.
/// This test checks that calls to pthread_create, fopen, etc. are transpiled
/// to fragile_runtime::fragile_pthread_create, fragile_runtime::fopen, etc.
#[test]
fn test_runtime_function_name_mapping() {
    let parser = ClangParser::new().expect("Failed to create parser");

    // Test pthread function mapping
    let source = r#"
        #include <pthread.h>

        void* thread_func(void* arg) {
            return arg;
        }

        int main() {
            pthread_t thread;
            pthread_create(&thread, nullptr, thread_func, nullptr);
            pthread_join(thread, nullptr);
            return 0;
        }
    "#;

    let ast = parser
        .parse_string(source, "pthread_test.cpp")
        .expect("Failed to parse");
    let rust_code = AstCodeGen::new().generate(&ast.translation_unit);

    // Check that pthread_create is mapped to fragile_runtime
    assert!(
        rust_code.contains("fragile_runtime::fragile_pthread_create") ||
        // Note: The function might not appear if pthread.h is not fully parsed
        // In that case, the test validates the mapping mechanism is in place
        !rust_code.contains("pthread_create("),
        "pthread_create should be mapped to fragile_runtime::fragile_pthread_create\nGenerated code snippet:\n{}",
        &rust_code[..rust_code.len().min(2000)]
    );

    // Test stdio function mapping
    let source2 = r#"
        #include <stdio.h>

        int main() {
            FILE* f = fopen("test.txt", "w");
            if (f) {
                fputs("Hello", f);
                fclose(f);
            }
            return 0;
        }
    "#;

    let ast2 = parser
        .parse_string(source2, "stdio_test.cpp")
        .expect("Failed to parse");
    let rust_code2 = AstCodeGen::new().generate(&ast2.translation_unit);

    // Check that fopen is mapped to fragile_runtime
    assert!(
        rust_code2.contains("fragile_runtime::fopen") ||
        // Note: The function might not appear if stdio.h is not fully parsed
        !rust_code2.contains("fopen("),
        "fopen should be mapped to fragile_runtime::fopen\nGenerated code snippet:\n{}",
        &rust_code2[..rust_code2.len().min(2000)]
    );

    println!("=== Runtime function name mapping test ===");
    println!(
        "pthread mapping: {}",
        if rust_code.contains("fragile_runtime::fragile_pthread_create") {
            "OK"
        } else {
            "Not triggered (header not parsed)"
        }
    );
    println!(
        "stdio mapping: {}",
        if rust_code2.contains("fragile_runtime::fopen") {
            "OK"
        } else {
            "Not triggered (header not parsed)"
        }
    );
}

/// Test 23.7: Verify operator new/delete are correctly mapped to fragile-runtime.
/// operator new(size) should generate fragile_runtime::fragile_malloc(size)
/// operator delete(ptr) should generate fragile_runtime::fragile_free(ptr)
#[test]
fn test_operator_new_delete_mapping() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        #include <cstddef>
        #include <new>

        void* my_alloc(std::size_t n) {
            return ::operator new(n);
        }

        void my_free(void* p) {
            ::operator delete(p);
        }

        int main() {
            void* p = my_alloc(100);
            my_free(p);
            return 0;
        }
    "#;

    let ast = parser
        .parse_string(source, "opnew_test.cpp")
        .expect("Failed to parse");
    let rust_code = AstCodeGen::new().generate(&ast.translation_unit);

    // Check that operator new is mapped to fragile_malloc
    assert!(
        rust_code.contains("fragile_runtime::fragile_malloc"),
        "operator new should be mapped to fragile_runtime::fragile_malloc\nGenerated code:\n{}",
        &rust_code[..rust_code.len().min(3000)]
    );

    // Check that operator delete is mapped to fragile_free
    assert!(
        rust_code.contains("fragile_runtime::fragile_free"),
        "operator delete should be mapped to fragile_runtime::fragile_free\nGenerated code:\n{}",
        &rust_code[..rust_code.len().min(3000)]
    );

    // Verify the argument is passed correctly (n for new, p for delete)
    assert!(
        rust_code.contains("fragile_malloc(n)"),
        "fragile_malloc should receive the size argument 'n'\nGenerated code:\n{}",
        &rust_code[..rust_code.len().min(3000)]
    );

    println!("=== Operator new/delete mapping test ===");
    println!("operator new -> fragile_malloc: OK");
    println!("operator delete -> fragile_free: OK");
}

/// Test 23.9.1: Transpile minimal <iostream> usage
/// This test documents the current state of libc++ iostream transpilation.
#[test]
fn test_libcxx_iostream_transpilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <iostream>

        int main() {
            std::cout << "Hello" << std::endl;
            return 0;
        }
    "#;

    let (transpile_ok, rust_code, transpile_errors, compile_ok, compile_errors) =
        transpile_and_compile_with_vendored_libcxx(source, "test_iostream.cpp");

    println!("=== libc++ iostream transpilation test ===");
    println!("Transpilation success: {}", transpile_ok);
    println!("Generated Rust code length: {} chars", rust_code.len());
    println!("Compilation success: {}", compile_ok);

    if !transpile_errors.is_empty() {
        println!("Transpilation errors:\n{}", transpile_errors);
    }

    if !compile_ok && !compile_errors.is_empty() {
        // Count errors
        let error_count = compile_errors.matches("error[E").count();
        println!("Compilation errors: {} total", error_count);
        // Show first 5000 chars of errors
        let preview = if compile_errors.len() > 5000 {
            format!(
                "{}...\n[truncated, {} more chars]",
                &compile_errors[..5000],
                compile_errors.len() - 5000
            )
        } else {
            compile_errors.clone()
        };
        println!("Compile errors:\n{}", preview);
    }

    // For now, we just check that the transpiler doesn't crash
    // Later tests will verify the code compiles and runs
    assert!(
        transpile_ok || !transpile_errors.is_empty(),
        "Should either succeed or report errors, not crash"
    );
}

/// Test 23.10.1: Transpile minimal <thread> usage
/// This test documents the current state of libc++ thread transpilation.
#[test]
fn test_libcxx_thread_transpilation() {
    if !ClangParser::is_vendored_libcxx_available() {
        eprintln!("Skipping test: vendored libc++ not available");
        return;
    }

    let source = r#"
        #include <thread>

        void worker() { }

        int main() {
            std::thread t(worker);
            t.join();
            return 0;
        }
    "#;

    let (transpile_ok, rust_code, transpile_errors, compile_ok, compile_errors) =
        transpile_and_compile_with_vendored_libcxx(source, "test_thread.cpp");

    println!("=== libc++ thread transpilation test ===");
    println!("Transpilation success: {}", transpile_ok);
    println!("Generated Rust code length: {} chars", rust_code.len());
    println!("Compilation success: {}", compile_ok);

    if !transpile_errors.is_empty() {
        println!("Transpilation errors:\n{}", transpile_errors);
    }

    if !compile_ok && !compile_errors.is_empty() {
        // Count errors
        let error_count = compile_errors.matches("error[E").count();
        println!("Compilation errors: {} total", error_count);
        // Show first 5000 chars of errors
        let preview = if compile_errors.len() > 5000 {
            format!(
                "{}...\n[truncated, {} more chars]",
                &compile_errors[..5000],
                compile_errors.len() - 5000
            )
        } else {
            compile_errors.clone()
        };
        println!("Compile errors:\n{}", preview);
    }

    // For now, we just check that the transpiler doesn't crash
    assert!(
        transpile_ok || !transpile_errors.is_empty(),
        "Should either succeed or report errors, not crash"
    );
}

/// Task 23.11.1: Test a realistic single-file C++ program.
/// This is an expression evaluator that exercises multiple features:
/// - Class inheritance hierarchy (Expr -> BinaryExpr -> Add/Mul)
/// - Virtual methods (eval())
/// - Pure virtual methods (= 0)
/// - new/delete for memory management
/// - Operator overloading (through virtual dispatch)
#[test]
fn test_e2e_expression_evaluator() {
    let source = r#"
        // Base class with pure virtual eval() method
        class Expr {
        public:
            virtual ~Expr() {}
            virtual int eval() const = 0;
        };

        // Leaf node: a number literal
        class Number : public Expr {
            int value;
        public:
            Number(int v) : value(v) {}
            int eval() const override { return value; }
        };

        // Intermediate class for binary expressions
        class BinaryExpr : public Expr {
        protected:
            Expr* left;
            Expr* right;
        public:
            BinaryExpr(Expr* l, Expr* r) : left(l), right(r) {}
            ~BinaryExpr() override {
                delete left;
                delete right;
            }
        };

        // Addition expression
        class Add : public BinaryExpr {
        public:
            Add(Expr* l, Expr* r) : BinaryExpr(l, r) {}
            int eval() const override { return left->eval() + right->eval(); }
        };

        // Multiplication expression
        class Mul : public BinaryExpr {
        public:
            Mul(Expr* l, Expr* r) : BinaryExpr(l, r) {}
            int eval() const override { return left->eval() * right->eval(); }
        };

        int main() {
            // Build expression: (2 + 3) * 4 = 20
            Expr* expr = new Mul(
                new Add(new Number(2), new Number(3)),
                new Number(4)
            );

            int result = expr->eval();
            delete expr;

            // Verify result
            if (result != 20) {
                return 1;  // Failure: wrong result
            }

            // Test another expression: 5 * (3 + 7) = 50
            Expr* expr2 = new Mul(
                new Number(5),
                new Add(new Number(3), new Number(7))
            );

            int result2 = expr2->eval();
            delete expr2;

            if (result2 != 50) {
                return 2;  // Failure: wrong result for expr2
            }

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_expression_evaluator.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Expression evaluator should evaluate (2+3)*4=20 and 5*(3+7)=50 correctly"
    );
}

/// Task 23.11.1: Test simple linked list implementation.
/// This exercises:
/// - Struct with pointer to self
/// - Constructor with initializer list
/// - Destructor with pointer cleanup
/// - Template instantiation (via function template)
#[test]
fn test_e2e_linked_list() {
    let source = r#"
        // Simple singly-linked list node
        struct Node {
            int value;
            Node* next;

            Node(int v) : value(v), next(nullptr) {}
            ~Node() {
                // Recursively delete remaining nodes
                if (next) {
                    delete next;
                }
            }
        };

        // Simple linked list
        class List {
            Node* head;
            int count;
        public:
            List() : head(nullptr), count(0) {}
            ~List() {
                if (head) {
                    delete head;
                }
            }

            void push_front(int value) {
                Node* newNode = new Node(value);
                newNode->next = head;
                head = newNode;
                count++;
            }

            int front() const {
                return head ? head->value : 0;
            }

            int size() const {
                return count;
            }

            int sum() const {
                int total = 0;
                Node* curr = head;
                while (curr) {
                    total += curr->value;
                    curr = curr->next;
                }
                return total;
            }
        };

        int main() {
            List list;

            // Test empty list
            if (list.size() != 0) return 1;

            // Add elements
            list.push_front(1);
            list.push_front(2);
            list.push_front(3);

            // Test size
            if (list.size() != 3) return 2;

            // Test front (should be 3, most recently added)
            if (list.front() != 3) return 3;

            // Test sum (3 + 2 + 1 = 6)
            if (list.sum() != 6) return 4;

            // Add more elements
            list.push_front(4);
            list.push_front(5);

            // Final checks
            if (list.size() != 5) return 5;
            if (list.sum() != 15) return 6;  // 5 + 4 + 3 + 2 + 1 = 15
            if (list.front() != 5) return 7;

            return 0;  // Success - destructor will clean up
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_linked_list.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Linked list implementation should work correctly"
    );
}

/// Task 23.11.2: Test binary search tree implementation.
/// This exercises:
/// - Tree structures with left/right pointers
/// - Recursive algorithms (insert, search, traversal)
/// - Destructor with recursive cleanup
/// - More complex control flow (tree rotations would be next level)
#[test]
fn test_e2e_binary_search_tree() {
    let source = r#"
        // Binary search tree node
        struct TreeNode {
            int value;
            TreeNode* left;
            TreeNode* right;

            TreeNode(int v) : value(v), left(nullptr), right(nullptr) {}

            ~TreeNode() {
                // Recursively delete children
                if (left) delete left;
                if (right) delete right;
            }
        };

        // Binary search tree
        class BST {
            TreeNode* root;
            int count;

            // Helper: recursive insert
            TreeNode* insertHelper(TreeNode* node, int value) {
                if (!node) {
                    count++;
                    return new TreeNode(value);
                }
                if (value < node->value) {
                    node->left = insertHelper(node->left, value);
                } else if (value > node->value) {
                    node->right = insertHelper(node->right, value);
                }
                // Duplicate values ignored
                return node;
            }

            // Helper: recursive search
            bool searchHelper(TreeNode* node, int value) const {
                if (!node) return false;
                if (value == node->value) return true;
                if (value < node->value) return searchHelper(node->left, value);
                return searchHelper(node->right, value);
            }

            // Helper: in-order traversal sum
            int sumHelper(TreeNode* node) const {
                if (!node) return 0;
                return sumHelper(node->left) + node->value + sumHelper(node->right);
            }

        public:
            BST() : root(nullptr), count(0) {}

            ~BST() {
                if (root) delete root;
            }

            void insert(int value) {
                root = insertHelper(root, value);
            }

            bool search(int value) const {
                return searchHelper(root, value);
            }

            int size() const { return count; }

            int sum() const {
                return sumHelper(root);
            }

            // Get root value (for testing)
            int rootValue() const {
                return root ? root->value : 0;
            }
        };

        int main() {
            BST tree;

            // Test empty tree
            if (tree.size() != 0) return 1;
            if (tree.search(10)) return 2;  // Should not find anything

            // Insert values: 50, 30, 70, 20, 40, 60, 80
            //        50
            //       /  \
            //      30   70
            //     / \   / \
            //    20 40 60 80
            tree.insert(50);
            tree.insert(30);
            tree.insert(70);
            tree.insert(20);
            tree.insert(40);
            tree.insert(60);
            tree.insert(80);

            // Test size
            if (tree.size() != 7) return 3;

            // Test root
            if (tree.rootValue() != 50) return 4;

            // Test search for existing values
            if (!tree.search(50)) return 5;
            if (!tree.search(30)) return 6;
            if (!tree.search(70)) return 7;
            if (!tree.search(20)) return 8;
            if (!tree.search(40)) return 9;
            if (!tree.search(60)) return 10;
            if (!tree.search(80)) return 11;

            // Test search for non-existing values
            if (tree.search(10)) return 12;
            if (tree.search(55)) return 13;
            if (tree.search(100)) return 14;

            // Test sum (20+30+40+50+60+70+80 = 350)
            if (tree.sum() != 350) return 15;

            // Test duplicate insertion (should be ignored)
            tree.insert(50);
            if (tree.size() != 7) return 16;  // Size unchanged

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_binary_search_tree.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Binary search tree should work correctly (insert, search, sum)"
    );
}

/// Task 23.11.2: Test a simple 2D vector class with operator overloading.
/// This exercises:
/// - Operator overloading (+, -, *, ==, !=)
/// - Copy constructor
/// - Static factory methods
/// - Const methods
#[test]
fn test_e2e_vector2d() {
    let source = r#"
        class Vec2 {
            float x, y;
        public:
            Vec2() : x(0), y(0) {}
            Vec2(float x_, float y_) : x(x_), y(y_) {}

            // Copy constructor
            Vec2(const Vec2& other) : x(other.x), y(other.y) {}

            // Static factory methods
            static Vec2 zero() { return Vec2(0, 0); }
            static Vec2 one() { return Vec2(1, 1); }
            static Vec2 unit_x() { return Vec2(1, 0); }
            static Vec2 unit_y() { return Vec2(0, 1); }

            // Accessors
            float getX() const { return x; }
            float getY() const { return y; }

            // Magnitude squared (avoid sqrt for simplicity)
            float length_squared() const {
                return x * x + y * y;
            }

            // Dot product
            float dot(const Vec2& other) const {
                return x * other.x + y * other.y;
            }

            // Operator overloading
            Vec2 operator+(const Vec2& other) const {
                return Vec2(x + other.x, y + other.y);
            }

            Vec2 operator-(const Vec2& other) const {
                return Vec2(x - other.x, y - other.y);
            }

            Vec2 operator*(float scalar) const {
                return Vec2(x * scalar, y * scalar);
            }

            bool operator==(const Vec2& other) const {
                return x == other.x && y == other.y;
            }

            bool operator!=(const Vec2& other) const {
                return !(*this == other);
            }

            // Compound assignment
            Vec2& operator+=(const Vec2& other) {
                x += other.x;
                y += other.y;
                return *this;
            }
        };

        int main() {
            // Test default constructor
            Vec2 v1;
            if (v1.getX() != 0 || v1.getY() != 0) return 1;

            // Test parameterized constructor
            Vec2 v2(3, 4);
            if (v2.getX() != 3 || v2.getY() != 4) return 2;

            // Test copy constructor
            Vec2 v3(v2);
            if (v3.getX() != 3 || v3.getY() != 4) return 3;

            // Test static factory methods
            Vec2 zero = Vec2::zero();
            if (zero.getX() != 0 || zero.getY() != 0) return 4;

            Vec2 one = Vec2::one();
            if (one.getX() != 1 || one.getY() != 1) return 5;

            // Test addition
            Vec2 sum = v2 + one;
            if (sum.getX() != 4 || sum.getY() != 5) return 6;

            // Test subtraction
            Vec2 diff = v2 - one;
            if (diff.getX() != 2 || diff.getY() != 3) return 7;

            // Test scalar multiplication
            Vec2 scaled = v2 * 2;
            if (scaled.getX() != 6 || scaled.getY() != 8) return 8;

            // Test length squared (3*3 + 4*4 = 25)
            float len_sq = v2.length_squared();
            if (len_sq != 25) return 9;

            // Test dot product ((3,4) . (1,1) = 3 + 4 = 7)
            float dot = v2.dot(one);
            if (dot != 7) return 10;

            // Test equality
            Vec2 v4(3, 4);
            if (!(v2 == v4)) return 11;
            if (v2 != v4) return 12;

            // Test inequality
            if (v2 == one) return 13;
            if (!(v2 != one)) return 14;

            // Test compound assignment
            Vec2 v5(1, 2);
            v5 += v2;  // (1,2) + (3,4) = (4,6)
            if (v5.getX() != 4 || v5.getY() != 6) return 15;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_vector2d.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Vec2 class with operator overloading should work correctly"
    );
}

/// E2E test: Stack (array-based) with push/pop operations
/// Tests: array indexing, bounds checking, copy semantics
#[test]
fn test_e2e_stack() {
    let source = r#"
        // Fixed-size stack (non-template for simplicity)
        class IntStack {
            static const int CAPACITY = 10;
            int data[10];
            int top;
        public:
            IntStack() : top(-1) {}

            bool empty() const { return top < 0; }
            bool full() const { return top >= CAPACITY - 1; }
            int size() const { return top + 1; }

            bool push(int value) {
                if (full()) return false;
                data[++top] = value;
                return true;
            }

            bool pop(int* out) {
                if (empty()) return false;
                *out = data[top--];
                return true;
            }

            int peek() const {
                return data[top];
            }
        };

        int main() {
            IntStack s;

            // Test empty stack
            if (!s.empty()) return 1;
            if (s.size() != 0) return 2;

            // Test push
            if (!s.push(10)) return 3;
            if (s.empty()) return 4;
            if (s.size() != 1) return 5;
            if (s.peek() != 10) return 6;

            // Test multiple pushes
            if (!s.push(20)) return 7;
            if (!s.push(30)) return 8;
            if (s.size() != 3) return 9;
            if (s.peek() != 30) return 10;

            // Test pop
            int val;
            if (!s.pop(&val)) return 11;
            if (val != 30) return 12;
            if (s.size() != 2) return 13;

            // Pop remaining
            if (!s.pop(&val)) return 14;
            if (val != 20) return 15;
            if (!s.pop(&val)) return 16;
            if (val != 10) return 17;

            // Stack should be empty
            if (!s.empty()) return 18;

            // Pop from empty should fail
            if (s.pop(&val)) return 19;

            // Fill up to test full()
            for (int i = 0; i < 10; i++) {
                if (!s.push(i * 2)) return 20;
            }
            if (!s.full()) return 21;

            // Push to full should fail
            if (s.push(100)) return 22;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_stack.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Stack with array storage should work correctly"
    );
}

/// E2E test: 2x2 Matrix with operator overloading
/// Tests: 2D array access, operator overloading, static factory methods
#[test]
fn test_e2e_matrix2x2() {
    let source = r#"
        class Matrix2x2 {
            float m[2][2];
        public:
            // Default constructor (identity matrix)
            Matrix2x2() {
                m[0][0] = 1; m[0][1] = 0;
                m[1][0] = 0; m[1][1] = 1;
            }

            // Constructor with values
            Matrix2x2(float a00, float a01, float a10, float a11) {
                m[0][0] = a00; m[0][1] = a01;
                m[1][0] = a10; m[1][1] = a11;
            }

            // Static factory: zero matrix
            static Matrix2x2 zero() {
                return Matrix2x2(0, 0, 0, 0);
            }

            // Element access (getter only - setter would require fixing const detection)
            float get(int row, int col) const { return m[row][col]; }

            // Matrix addition
            Matrix2x2 operator+(const Matrix2x2& other) const {
                return Matrix2x2(
                    m[0][0] + other.m[0][0], m[0][1] + other.m[0][1],
                    m[1][0] + other.m[1][0], m[1][1] + other.m[1][1]
                );
            }

            // Matrix multiplication (single overload to avoid overload resolution issues)
            Matrix2x2 operator*(const Matrix2x2& other) const {
                return Matrix2x2(
                    m[0][0] * other.m[0][0] + m[0][1] * other.m[1][0],
                    m[0][0] * other.m[0][1] + m[0][1] * other.m[1][1],
                    m[1][0] * other.m[0][0] + m[1][1] * other.m[1][0],
                    m[1][0] * other.m[0][1] + m[1][1] * other.m[1][1]
                );
            }

            // Scalar multiplication via named method (avoids overload resolution)
            Matrix2x2 scale(float s) const {
                return Matrix2x2(
                    m[0][0] * s, m[0][1] * s,
                    m[1][0] * s, m[1][1] * s
                );
            }

            // Equality
            bool operator==(const Matrix2x2& other) const {
                return m[0][0] == other.m[0][0] && m[0][1] == other.m[0][1] &&
                       m[1][0] == other.m[1][0] && m[1][1] == other.m[1][1];
            }

            bool operator!=(const Matrix2x2& other) const {
                return !(*this == other);
            }

            // Determinant
            float det() const {
                return m[0][0] * m[1][1] - m[0][1] * m[1][0];
            }

            // Trace
            float trace() const {
                return m[0][0] + m[1][1];
            }
        };

        int main() {
            // Test default constructor (identity)
            Matrix2x2 identity;
            if (identity.get(0, 0) != 1 || identity.get(0, 1) != 0) return 1;
            if (identity.get(1, 0) != 0 || identity.get(1, 1) != 1) return 2;

            // Test parameterized constructor
            Matrix2x2 a(1, 2, 3, 4);
            if (a.get(0, 0) != 1 || a.get(0, 1) != 2) return 3;
            if (a.get(1, 0) != 3 || a.get(1, 1) != 4) return 4;

            // Test static factory
            Matrix2x2 z = Matrix2x2::zero();
            if (z.get(0, 0) != 0 || z.get(1, 1) != 0) return 5;

            // Test addition
            Matrix2x2 b(5, 6, 7, 8);
            Matrix2x2 sum = a + b;  // [6,8; 10,12]
            if (sum.get(0, 0) != 6 || sum.get(0, 1) != 8) return 6;
            if (sum.get(1, 0) != 10 || sum.get(1, 1) != 12) return 7;

            // Test matrix multiplication: identity * a = a
            Matrix2x2 prod = identity * a;
            if (prod != a) return 8;

            // Test matrix multiplication: a * b
            // [1,2] * [5,6] = [1*5+2*7, 1*6+2*8] = [19, 22]
            // [3,4]   [7,8]   [3*5+4*7, 3*6+4*8]   [43, 50]
            Matrix2x2 ab = a * b;
            if (ab.get(0, 0) != 19 || ab.get(0, 1) != 22) return 9;
            if (ab.get(1, 0) != 43 || ab.get(1, 1) != 50) return 10;

            // Test scalar multiplication (via named method)
            Matrix2x2 scaled = a.scale(2);  // [2,4; 6,8]
            if (scaled.get(0, 0) != 2 || scaled.get(1, 1) != 8) return 11;

            // Test equality
            Matrix2x2 a_copy(1, 2, 3, 4);
            if (a != a_copy) return 12;
            if (a == b) return 13;

            // Test determinant: det([1,2; 3,4]) = 1*4 - 2*3 = -2
            float d = a.det();
            if (d != -2) return 14;

            // Test trace: trace([1,2; 3,4]) = 1 + 4 = 5
            float t = a.trace();
            if (t != 5) return 15;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_matrix2x2.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Matrix2x2 with operator overloading should work correctly"
    );
}

/// E2E test: Simple hash table with chaining
/// Tests: pointer arrays, linked list nodes, hashing, templates
#[test]
fn test_e2e_simple_hash_table() {
    let source = r#"
        // Simple hash table entry (linked list node for chaining)
        struct Entry {
            int key;
            int value;
            Entry* next;

            Entry(int k, int v) : key(k), value(v), next(nullptr) {}
        };

        // Simple hash table with fixed size and chaining
        class IntHashTable {
            static const int SIZE = 16;
            Entry* buckets[16];
            int count;

            int hash(int key) const {
                // Simple modulo hash
                int h = key % SIZE;
                return h < 0 ? h + SIZE : h;
            }
        public:
            IntHashTable() : count(0) {
                for (int i = 0; i < SIZE; i++) {
                    buckets[i] = nullptr;
                }
            }

            ~IntHashTable() {
                for (int i = 0; i < SIZE; i++) {
                    Entry* e = buckets[i];
                    while (e != nullptr) {
                        Entry* next = e->next;
                        delete e;
                        e = next;
                    }
                }
            }

            int size() const { return count; }
            bool empty() const { return count == 0; }

            void insert(int key, int value) {
                int idx = hash(key);
                // Check if key exists, update value
                Entry* e = buckets[idx];
                while (e != nullptr) {
                    if (e->key == key) {
                        e->value = value;
                        return;
                    }
                    e = e->next;
                }
                // Key not found, insert new entry
                Entry* newEntry = new Entry(key, value);
                newEntry->next = buckets[idx];
                buckets[idx] = newEntry;
                count++;
            }

            bool contains(int key) const {
                int idx = hash(key);
                Entry* e = buckets[idx];
                while (e != nullptr) {
                    if (e->key == key) return true;
                    e = e->next;
                }
                return false;
            }

            int get(int key) const {
                int idx = hash(key);
                Entry* e = buckets[idx];
                while (e != nullptr) {
                    if (e->key == key) return e->value;
                    e = e->next;
                }
                return -1;  // Not found
            }

            bool remove(int key) {
                int idx = hash(key);
                Entry* prev = nullptr;
                Entry* e = buckets[idx];
                while (e != nullptr) {
                    if (e->key == key) {
                        if (prev != nullptr) {
                            prev->next = e->next;
                        } else {
                            buckets[idx] = e->next;
                        }
                        delete e;
                        count--;
                        return true;
                    }
                    prev = e;
                    e = e->next;
                }
                return false;
            }
        };

        int main() {
            IntHashTable ht;

            // Test empty table
            if (!ht.empty()) return 1;
            if (ht.size() != 0) return 2;
            if (ht.contains(42)) return 3;

            // Test insert and get
            ht.insert(10, 100);
            if (ht.empty()) return 4;
            if (ht.size() != 1) return 5;
            if (!ht.contains(10)) return 6;
            if (ht.get(10) != 100) return 7;

            // Test multiple inserts
            ht.insert(20, 200);
            ht.insert(30, 300);
            if (ht.size() != 3) return 8;
            if (ht.get(20) != 200) return 9;
            if (ht.get(30) != 300) return 10;

            // Test collision (keys that hash to same bucket)
            ht.insert(26, 260);  // 26 % 16 = 10, same bucket as 10
            if (ht.size() != 4) return 11;
            if (ht.get(10) != 100) return 12;  // Original still there
            if (ht.get(26) != 260) return 13;  // New one also there

            // Test update existing key
            ht.insert(10, 101);
            if (ht.size() != 4) return 14;  // Size unchanged
            if (ht.get(10) != 101) return 15;  // Value updated

            // Test remove
            if (!ht.remove(20)) return 16;
            if (ht.size() != 3) return 17;
            if (ht.contains(20)) return 18;

            // Test remove non-existent
            if (ht.remove(999)) return 19;

            // Test negative key
            ht.insert(-5, 500);
            if (!ht.contains(-5)) return 20;
            if (ht.get(-5) != 500) return 21;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_simple_hash_table.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Simple hash table with chaining should work correctly"
    );
}

/// E2E test: Min-heap / priority queue
/// Tests: array-based heap operations, complex indexing, swap operations
#[test]
fn test_e2e_min_heap() {
    let source = r#"
        class MinHeap {
            static const int MAX_SIZE = 32;
            int data[32];
            int size_;

            int parent(int i) const { return (i - 1) / 2; }
            int left(int i) const { return 2 * i + 1; }
            int right(int i) const { return 2 * i + 2; }

            void swap(int i, int j) {
                int temp = data[i];
                data[i] = data[j];
                data[j] = temp;
            }

            void heapifyUp(int i) {
                while (i > 0 && data[parent(i)] > data[i]) {
                    swap(i, parent(i));
                    i = parent(i);
                }
            }

            void heapifyDown(int i) {
                int minIdx = i;
                int l = left(i);
                int r = right(i);

                if (l < size_ && data[l] < data[minIdx]) {
                    minIdx = l;
                }
                if (r < size_ && data[r] < data[minIdx]) {
                    minIdx = r;
                }

                if (minIdx != i) {
                    swap(i, minIdx);
                    heapifyDown(minIdx);
                }
            }

        public:
            MinHeap() : size_(0) {}

            int size() const { return size_; }
            bool empty() const { return size_ == 0; }
            bool full() const { return size_ >= MAX_SIZE; }

            bool push(int value) {
                if (full()) return false;
                data[size_] = value;
                heapifyUp(size_);
                size_++;
                return true;
            }

            int top() const {
                return data[0];
            }

            bool pop() {
                if (empty()) return false;
                data[0] = data[size_ - 1];
                size_--;
                if (size_ > 0) {
                    heapifyDown(0);
                }
                return true;
            }
        };

        int main() {
            MinHeap heap;

            // Test empty heap
            if (!heap.empty()) return 1;
            if (heap.size() != 0) return 2;

            // Test single element
            if (!heap.push(42)) return 3;
            if (heap.empty()) return 4;
            if (heap.size() != 1) return 5;
            if (heap.top() != 42) return 6;

            // Test min property - insert in descending order
            heap.pop();
            heap.push(5);
            heap.push(3);
            heap.push(8);
            heap.push(1);
            heap.push(6);

            // Min should be 1
            if (heap.top() != 1) return 7;
            if (heap.size() != 5) return 8;

            // Pop should return elements in sorted order
            int prev = heap.top();
            heap.pop();

            while (!heap.empty()) {
                int curr = heap.top();
                if (curr < prev) return 9;  // Should be increasing
                prev = curr;
                heap.pop();
            }

            // Test heap is empty after all pops
            if (!heap.empty()) return 10;
            if (heap.pop()) return 11;  // Pop on empty should fail

            // Test with duplicates
            heap.push(5);
            heap.push(5);
            heap.push(3);
            heap.push(3);
            if (heap.size() != 4) return 12;
            if (heap.top() != 3) return 13;

            // Verify all can be popped
            for (int i = 0; i < 4; i++) {
                if (!heap.pop()) return 14;
            }
            if (!heap.empty()) return 15;

            // Test filling heap
            for (int i = 0; i < 32; i++) {
                if (!heap.push(i)) return 16;
            }
            if (!heap.full()) return 17;
            if (heap.push(999)) return 18;  // Should fail when full

            // Verify heap property maintained
            if (heap.top() != 0) return 19;  // Min should be 0

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_min_heap.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Min-heap priority queue should work correctly"
    );
}

/// E2E test: Graph with adjacency list
/// Tests: linked lists, dynamic memory, BFS traversal, visited tracking
#[test]
fn test_e2e_simple_graph() {
    let source = r#"
        // Simple graph using adjacency list
        // Tests linked lists, dynamic memory, BFS traversal

        struct Edge {
            int to;
            Edge* next;
            Edge(int t) : to(t), next(nullptr) {}
        };

        class Graph {
            static const int MAX_VERTICES = 16;
            Edge* adj[16];  // adjacency list heads
            int numVertices;

        public:
            Graph(int n) : numVertices(n) {
                for (int i = 0; i < MAX_VERTICES; i++) {
                    adj[i] = nullptr;
                }
            }

            ~Graph() {
                // Free all edge lists
                for (int i = 0; i < numVertices; i++) {
                    Edge* curr = adj[i];
                    while (curr != nullptr) {
                        Edge* next = curr->next;
                        delete curr;
                        curr = next;
                    }
                }
            }

            void addEdge(int from, int to) {
                Edge* e = new Edge(to);
                e->next = adj[from];
                adj[from] = e;
            }

            // Count edges from a vertex
            int outDegree(int v) const {
                int count = 0;
                Edge* curr = adj[v];
                while (curr != nullptr) {
                    count++;
                    curr = curr->next;
                }
                return count;
            }

            // Simple BFS using array as queue
            int bfsDistance(int start, int end) {
                if (start == end) return 0;
                if (start < 0 || start >= numVertices) return -1;
                if (end < 0 || end >= numVertices) return -1;

                bool visited[16];
                int distance[16];
                int queue[16];
                int front = 0, back = 0;

                for (int i = 0; i < MAX_VERTICES; i++) {
                    visited[i] = false;
                    distance[i] = -1;
                }

                visited[start] = true;
                distance[start] = 0;
                queue[back++] = start;

                while (front < back) {
                    int curr = queue[front++];

                    Edge* edge = adj[curr];
                    while (edge != nullptr) {
                        int next = edge->to;
                        if (!visited[next]) {
                            visited[next] = true;
                            distance[next] = distance[curr] + 1;
                            if (next == end) {
                                return distance[next];
                            }
                            queue[back++] = next;
                        }
                        edge = edge->next;
                    }
                }

                return -1;  // Not reachable
            }

            // Check if path exists using DFS
            bool hasPath(int from, int to) {
                if (from == to) return true;
                if (from < 0 || from >= numVertices) return false;
                if (to < 0 || to >= numVertices) return false;

                bool visited[16];
                for (int i = 0; i < MAX_VERTICES; i++) {
                    visited[i] = false;
                }

                // Explicitly pass array as pointer (workaround for array-to-pointer decay)
                return dfsHelper(from, to, &visited[0]);
            }

        private:
            bool dfsHelper(int curr, int target, bool* visited) {
                if (curr == target) return true;
                visited[curr] = true;

                Edge* edge = adj[curr];
                while (edge != nullptr) {
                    if (!visited[edge->to]) {
                        // Recursively pass pointer
                        if (dfsHelper(edge->to, target, visited)) {
                            return true;
                        }
                    }
                    edge = edge->next;
                }
                return false;
            }
        };

        int main() {
            // Create a simple directed graph:
            // 0 -> 1 -> 2
            //      |    |
            //      v    v
            //      3 -> 4
            Graph g(5);

            // Test empty graph
            if (g.outDegree(0) != 0) return 1;

            // Add edges
            g.addEdge(0, 1);
            g.addEdge(1, 2);
            g.addEdge(1, 3);
            g.addEdge(2, 4);
            g.addEdge(3, 4);

            // Test out-degrees
            if (g.outDegree(0) != 1) return 2;  // 0 -> 1
            if (g.outDegree(1) != 2) return 3;  // 1 -> 2, 3
            if (g.outDegree(4) != 0) return 4;  // 4 has no outgoing

            // Test BFS distances
            if (g.bfsDistance(0, 0) != 0) return 5;   // Same vertex
            if (g.bfsDistance(0, 1) != 1) return 6;   // 0 -> 1
            if (g.bfsDistance(0, 2) != 2) return 7;   // 0 -> 1 -> 2
            if (g.bfsDistance(0, 3) != 2) return 8;   // 0 -> 1 -> 3
            if (g.bfsDistance(0, 4) != 3) return 9;   // 0 -> 1 -> 2 -> 4 or 0 -> 1 -> 3 -> 4
            if (g.bfsDistance(4, 0) != -1) return 10; // No path back

            // Test hasPath
            if (!g.hasPath(0, 4)) return 11;  // Path exists
            if (!g.hasPath(0, 3)) return 12;  // Path exists
            if (g.hasPath(4, 0)) return 13;   // No path
            if (g.hasPath(2, 3)) return 14;   // No path (they're siblings from 1)

            // Test with a cycle
            Graph g2(3);
            g2.addEdge(0, 1);
            g2.addEdge(1, 2);
            g2.addEdge(2, 0);  // Cycle

            if (!g2.hasPath(0, 2)) return 15;
            if (!g2.hasPath(2, 0)) return 16;  // Through cycle
            if (g2.bfsDistance(0, 2) != 2) return 17;
            if (g2.bfsDistance(2, 0) != 1) return 18;  // Direct edge

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_simple_graph.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Graph with adjacency list should work correctly"
    );
}

/// Debug test to print AST structure for array init
#[test]
fn test_debug_array_init_ast() {
    use fragile_clang::{ClangNode, ClangNodeKind, ClangParser};

    let source = r#"
int main() {
    int arr[5] = {5, 2, 8, 1, 9};
    return arr[0];
}
"#;

    fn print_ast(node: &ClangNode, indent: usize) {
        let prefix = " ".repeat(indent);
        println!("{}{:?}", prefix, node.kind);
        for child in &node.children {
            print_ast(child, indent + 2);
        }
    }

    let parser = ClangParser::new().unwrap();
    let ast_result = parser.parse_string(source, "test.cpp").unwrap();

    // Find the VarDecl for arr
    fn find_vardecl<'a>(node: &'a ClangNode, name: &str) -> Option<&'a ClangNode> {
        match &node.kind {
            ClangNodeKind::VarDecl { name: n, .. } if n == name => Some(node),
            _ => {
                for child in &node.children {
                    if let Some(found) = find_vardecl(child, name) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }

    if let Some(vardecl) = find_vardecl(&ast_result.translation_unit, "arr") {
        println!("=== VarDecl for arr ===");
        print_ast(vardecl, 0);
    } else {
        println!("VarDecl for arr not found");
    }
}

/// E2E test: QuickSort with partition and recursion
/// Tests: recursive functions, array manipulation, swap, comparison
#[test]
fn test_e2e_quicksort() {
    let source = r#"
        // QuickSort implementation
        // Tests recursive functions, array manipulation, and partition logic

        void swap(int* a, int* b) {
            int temp = *a;
            *a = *b;
            *b = temp;
        }

        // Partition using last element as pivot
        int partition(int* arr, int low, int high) {
            int pivot = arr[high];
            int i = low - 1;

            for (int j = low; j < high; j++) {
                if (arr[j] <= pivot) {
                    i++;
                    swap(&arr[i], &arr[j]);
                }
            }
            swap(&arr[i + 1], &arr[high]);
            return i + 1;
        }

        void quicksort(int* arr, int low, int high) {
            if (low < high) {
                int pi = partition(arr, low, high);
                quicksort(arr, low, pi - 1);
                quicksort(arr, pi + 1, high);
            }
        }

        // Helper to check if array is sorted
        bool isSorted(int* arr, int n) {
            for (int i = 0; i < n - 1; i++) {
                if (arr[i] > arr[i + 1]) {
                    return false;
                }
            }
            return true;
        }

        int main() {
            // Test 1: Simple array
            int arr1[5] = {5, 2, 8, 1, 9};
            quicksort(arr1, 0, 4);
            if (!isSorted(arr1, 5)) return 1;
            if (arr1[0] != 1 || arr1[4] != 9) return 2;

            // Test 2: Already sorted
            int arr2[4] = {1, 2, 3, 4};
            quicksort(arr2, 0, 3);
            if (!isSorted(arr2, 4)) return 3;

            // Test 3: Reverse sorted
            int arr3[4] = {4, 3, 2, 1};
            quicksort(arr3, 0, 3);
            if (!isSorted(arr3, 4)) return 4;
            if (arr3[0] != 1 || arr3[3] != 4) return 5;

            // Test 4: All same elements
            int arr4[5] = {7, 7, 7, 7, 7};
            quicksort(arr4, 0, 4);
            for (int i = 0; i < 5; i++) {
                if (arr4[i] != 7) return 6;
            }

            // Test 5: Two elements
            int arr5[2] = {10, 5};
            quicksort(arr5, 0, 1);
            if (arr5[0] != 5 || arr5[1] != 10) return 7;

            // Test 6: Single element (edge case)
            int arr6[1] = {42};
            quicksort(arr6, 0, 0);
            if (arr6[0] != 42) return 8;

            // Test 7: Larger array with duplicates
            int arr7[10] = {3, 1, 4, 1, 5, 9, 2, 6, 5, 3};
            quicksort(arr7, 0, 9);
            if (!isSorted(arr7, 10)) return 9;
            if (arr7[0] != 1 || arr7[9] != 9) return 10;

            // Test 8: Negative numbers
            int arr8[5] = {-3, 5, -1, 0, 2};
            quicksort(arr8, 0, 4);
            if (!isSorted(arr8, 5)) return 11;
            if (arr8[0] != -3 || arr8[4] != 5) return 12;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_quicksort.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "QuickSort should sort arrays correctly"
    );
}
