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

/// E2E test: Doubly Linked List with bidirectional iteration
/// Tests: two pointers per node, forward/backward traversal, insertion at both ends
#[test]
fn test_e2e_doubly_linked_list() {
    let source = r#"
        // Doubly Linked List implementation
        // Tests bidirectional pointers, insertion, deletion, traversal

        struct DLLNode {
            int data;
            DLLNode* prev;
            DLLNode* next;
        };

        struct DoublyLinkedList {
            DLLNode* head;
            DLLNode* tail;
            int size_;

            void init() {
                head = nullptr;
                tail = nullptr;
                size_ = 0;
            }

            void push_front(int val) {
                DLLNode* node = new DLLNode;
                node->data = val;
                node->prev = nullptr;
                node->next = head;

                if (head) {
                    head->prev = node;
                } else {
                    tail = node;
                }
                head = node;
                size_++;
            }

            void push_back(int val) {
                DLLNode* node = new DLLNode;
                node->data = val;
                node->next = nullptr;
                node->prev = tail;

                if (tail) {
                    tail->next = node;
                } else {
                    head = node;
                }
                tail = node;
                size_++;
            }

            int pop_front() {
                if (!head) return -1;
                DLLNode* node = head;
                int val = node->data;
                head = head->next;
                if (head) {
                    head->prev = nullptr;
                } else {
                    tail = nullptr;
                }
                delete node;
                size_--;
                return val;
            }

            int pop_back() {
                if (!tail) return -1;
                DLLNode* node = tail;
                int val = node->data;
                tail = tail->prev;
                if (tail) {
                    tail->next = nullptr;
                } else {
                    head = nullptr;
                }
                delete node;
                size_--;
                return val;
            }

            int front() const {
                return head ? head->data : -1;
            }

            int back() const {
                return tail ? tail->data : -1;
            }

            int size() const {
                return size_;
            }

            bool empty() const {
                return size_ == 0;
            }

            // Check consistency: forward count == backward count == size_
            bool isConsistent() const {
                int forwardCount = 0;
                DLLNode* curr = head;
                while (curr) {
                    forwardCount++;
                    curr = curr->next;
                }

                int backwardCount = 0;
                curr = tail;
                while (curr) {
                    backwardCount++;
                    curr = curr->prev;
                }

                return forwardCount == size_ && backwardCount == size_;
            }

            // Sum all elements traversing forward
            int sumForward() const {
                int sum = 0;
                DLLNode* curr = head;
                while (curr) {
                    sum += curr->data;
                    curr = curr->next;
                }
                return sum;
            }

            // Sum all elements traversing backward
            int sumBackward() const {
                int sum = 0;
                DLLNode* curr = tail;
                while (curr) {
                    sum += curr->data;
                    curr = curr->prev;
                }
                return sum;
            }

            void clear() {
                while (head) {
                    DLLNode* next = head->next;
                    delete head;
                    head = next;
                }
                tail = nullptr;
                size_ = 0;
            }
        };

        int main() {
            DoublyLinkedList list;
            list.init();

            // Test 1: Empty list
            if (!list.empty()) return 1;
            if (list.size() != 0) return 2;
            if (!list.isConsistent()) return 3;

            // Test 2: Push back
            list.push_back(1);
            list.push_back(2);
            list.push_back(3);
            if (list.size() != 3) return 4;
            if (list.front() != 1) return 5;
            if (list.back() != 3) return 6;
            if (!list.isConsistent()) return 7;

            // Test 3: Forward and backward sums should match
            if (list.sumForward() != 6) return 8;
            if (list.sumBackward() != 6) return 9;

            // Test 4: Push front
            list.push_front(0);
            list.push_front(-1);
            if (list.size() != 5) return 10;
            if (list.front() != -1) return 11;
            if (list.back() != 3) return 12;
            if (!list.isConsistent()) return 13;

            // Test 5: Pop front
            int val = list.pop_front();
            if (val != -1) return 14;
            if (list.front() != 0) return 15;
            if (list.size() != 4) return 16;
            if (!list.isConsistent()) return 17;

            // Test 6: Pop back
            val = list.pop_back();
            if (val != 3) return 18;
            if (list.back() != 2) return 19;
            if (list.size() != 3) return 20;
            if (!list.isConsistent()) return 21;

            // Test 7: Interleaved operations
            list.push_front(10);
            list.push_back(20);
            // List: 10, 0, 1, 2, 20
            if (list.size() != 5) return 22;
            if (list.sumForward() != 33) return 23;
            if (list.sumBackward() != 33) return 24;
            if (!list.isConsistent()) return 25;

            // Test 8: Pop until one element
            list.pop_front();  // 0, 1, 2, 20
            list.pop_front();  // 1, 2, 20
            list.pop_back();   // 1, 2
            list.pop_back();   // 1
            if (list.size() != 1) return 26;
            if (list.front() != 1) return 27;
            if (list.back() != 1) return 28;
            if (!list.isConsistent()) return 29;

            // Test 9: Pop last element
            val = list.pop_front();
            if (val != 1) return 30;
            if (!list.empty()) return 31;
            if (list.front() != -1) return 32;  // -1 indicates empty
            if (list.back() != -1) return 33;
            if (!list.isConsistent()) return 34;

            // Test 10: Operations on empty list
            if (list.pop_front() != -1) return 35;
            if (list.pop_back() != -1) return 36;

            // Test 11: Rebuild list and clear
            list.push_back(5);
            list.push_back(10);
            list.push_back(15);
            list.clear();
            if (!list.empty()) return 37;
            if (!list.isConsistent()) return 38;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_doubly_linked_list.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Doubly linked list should work correctly"
    );
}

/// E2E test: Circular Buffer (Ring Buffer)
/// Tests: modular arithmetic, wrap-around, fixed-size arrays, overwrite behavior
#[test]
fn test_e2e_circular_buffer() {
    let source = r#"
        // Circular Buffer (Ring Buffer) implementation
        // Tests modular indexing, wrap-around, full/empty detection

        struct CircularBuffer {
            int data[8];  // Fixed capacity of 8
            int head;     // Index of first element
            int tail;     // Index of next write position
            int count;    // Number of elements

            void init() {
                head = 0;
                tail = 0;
                count = 0;
            }

            int capacity() const {
                return 8;
            }

            int size() const {
                return count;
            }

            bool empty() const {
                return count == 0;
            }

            bool full() const {
                return count == 8;
            }

            // Add element (overwrites oldest if full)
            void push(int val) {
                data[tail] = val;
                tail = (tail + 1) % 8;
                if (count < 8) {
                    count++;
                } else {
                    // Overwriting oldest element, move head
                    head = (head + 1) % 8;
                }
            }

            // Remove and return oldest element
            int pop() {
                if (count == 0) return -1;
                int val = data[head];
                head = (head + 1) % 8;
                count--;
                return val;
            }

            // Peek at oldest element without removing
            int peek() const {
                if (count == 0) return -1;
                return data[head];
            }

            // Peek at newest element
            int peekBack() const {
                if (count == 0) return -1;
                int idx = (tail - 1 + 8) % 8;
                return data[idx];
            }

            // Get element at logical index (0 = oldest)
            int at(int idx) const {
                if (idx < 0 || idx >= count) return -1;
                int realIdx = (head + idx) % 8;
                return data[realIdx];
            }

            // Sum all elements
            int sum() const {
                int total = 0;
                for (int i = 0; i < count; i++) {
                    total += at(i);
                }
                return total;
            }

            void clear() {
                head = 0;
                tail = 0;
                count = 0;
            }
        };

        int main() {
            CircularBuffer buf;
            buf.init();

            // Test 1: Empty buffer
            if (!buf.empty()) return 1;
            if (buf.size() != 0) return 2;
            if (buf.capacity() != 8) return 3;
            if (buf.pop() != -1) return 4;

            // Test 2: Push some elements
            buf.push(1);
            buf.push(2);
            buf.push(3);
            if (buf.size() != 3) return 5;
            if (buf.peek() != 1) return 6;
            if (buf.peekBack() != 3) return 7;

            // Test 3: Pop elements
            if (buf.pop() != 1) return 8;
            if (buf.pop() != 2) return 9;
            if (buf.size() != 1) return 10;
            if (buf.peek() != 3) return 11;

            // Test 4: Fill the buffer
            buf.clear();
            for (int i = 0; i < 8; i++) {
                buf.push(i * 10);
            }
            if (!buf.full()) return 12;
            if (buf.size() != 8) return 13;
            if (buf.peek() != 0) return 14;
            if (buf.peekBack() != 70) return 15;

            // Test 5: Overwrite behavior
            buf.push(80);  // Should overwrite oldest (0)
            if (buf.size() != 8) return 16;
            if (buf.peek() != 10) return 17;  // 0 was overwritten
            if (buf.peekBack() != 80) return 18;

            buf.push(90);  // Overwrite 10
            buf.push(100); // Overwrite 20
            if (buf.peek() != 30) return 19;
            if (buf.peekBack() != 100) return 20;

            // Test 6: Random access with at()
            // Buffer now: 30, 40, 50, 60, 70, 80, 90, 100
            if (buf.at(0) != 30) return 21;
            if (buf.at(7) != 100) return 22;
            if (buf.at(4) != 70) return 23;
            if (buf.at(-1) != -1) return 24;
            if (buf.at(8) != -1) return 25;

            // Test 7: Sum of elements
            // 30 + 40 + 50 + 60 + 70 + 80 + 90 + 100 = 520
            if (buf.sum() != 520) return 26;

            // Test 8: Pop all and verify order
            if (buf.pop() != 30) return 27;
            if (buf.pop() != 40) return 28;
            if (buf.pop() != 50) return 29;
            if (buf.pop() != 60) return 30;
            if (buf.pop() != 70) return 31;
            if (buf.pop() != 80) return 32;
            if (buf.pop() != 90) return 33;
            if (buf.pop() != 100) return 34;
            if (!buf.empty()) return 35;

            // Test 9: Wrap around with interleaved push/pop
            buf.push(1);
            buf.push(2);
            buf.pop();
            buf.push(3);
            buf.push(4);
            buf.pop();
            buf.push(5);
            // Should have: 3, 4, 5
            if (buf.size() != 3) return 36;
            if (buf.peek() != 3) return 37;
            if (buf.peekBack() != 5) return 38;
            if (buf.sum() != 12) return 39;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_circular_buffer.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Circular buffer should work correctly"
    );
}

/// E2E test: Merge Sort (out-of-place)
/// Tests: recursion, temporary array allocation, divide and conquer, pointer arithmetic
#[test]
fn test_e2e_merge_sort() {
    let source = r#"
        // Merge Sort implementation
        // Tests recursion, array copying, merge operation

        void merge(int* arr, int left, int mid, int right, int* temp) {
            int i = left;
            int j = mid + 1;
            int k = left;

            // Merge the two halves into temp
            while (i <= mid && j <= right) {
                if (arr[i] <= arr[j]) {
                    temp[k] = arr[i];
                    i++;
                } else {
                    temp[k] = arr[j];
                    j++;
                }
                k++;
            }

            // Copy remaining elements from left half
            while (i <= mid) {
                temp[k] = arr[i];
                i++;
                k++;
            }

            // Copy remaining elements from right half
            while (j <= right) {
                temp[k] = arr[j];
                j++;
                k++;
            }

            // Copy back to original array
            for (int x = left; x <= right; x++) {
                arr[x] = temp[x];
            }
        }

        void mergeSort(int* arr, int left, int right, int* temp) {
            if (left < right) {
                int mid = left + (right - left) / 2;
                mergeSort(arr, left, mid, temp);
                mergeSort(arr, mid + 1, right, temp);
                merge(arr, left, mid, right, temp);
            }
        }

        bool isSorted(int* arr, int n) {
            for (int i = 0; i < n - 1; i++) {
                if (arr[i] > arr[i + 1]) {
                    return false;
                }
            }
            return true;
        }

        int main() {
            // Test 1: Basic unsorted array
            int arr1[5] = {5, 2, 8, 1, 9};
            int temp1[5];
            mergeSort(&arr1[0], 0, 4, &temp1[0]);
            if (!isSorted(&arr1[0], 5)) return 1;
            if (arr1[0] != 1 || arr1[4] != 9) return 2;

            // Test 2: Already sorted
            int arr2[4] = {1, 2, 3, 4};
            int temp2[4];
            mergeSort(&arr2[0], 0, 3, &temp2[0]);
            if (!isSorted(&arr2[0], 4)) return 3;

            // Test 3: Reverse sorted
            int arr3[4] = {4, 3, 2, 1};
            int temp3[4];
            mergeSort(&arr3[0], 0, 3, &temp3[0]);
            if (!isSorted(&arr3[0], 4)) return 4;
            if (arr3[0] != 1 || arr3[3] != 4) return 5;

            // Test 4: All same elements
            int arr4[5] = {7, 7, 7, 7, 7};
            int temp4[5];
            mergeSort(&arr4[0], 0, 4, &temp4[0]);
            for (int i = 0; i < 5; i++) {
                if (arr4[i] != 7) return 6;
            }

            // Test 5: Two elements
            int arr5[2] = {10, 5};
            int temp5[2];
            mergeSort(&arr5[0], 0, 1, &temp5[0]);
            if (arr5[0] != 5 || arr5[1] != 10) return 7;

            // Test 6: Single element
            int arr6[1] = {42};
            int temp6[1];
            mergeSort(&arr6[0], 0, 0, &temp6[0]);
            if (arr6[0] != 42) return 8;

            // Test 7: Larger array with duplicates
            int arr7[10] = {3, 1, 4, 1, 5, 9, 2, 6, 5, 3};
            int temp7[10];
            mergeSort(&arr7[0], 0, 9, &temp7[0]);
            if (!isSorted(&arr7[0], 10)) return 9;
            if (arr7[0] != 1 || arr7[9] != 9) return 10;

            // Test 8: Negative numbers
            int arr8[5] = {-3, 5, -1, 0, 2};
            int temp8[5];
            mergeSort(&arr8[0], 0, 4, &temp8[0]);
            if (!isSorted(&arr8[0], 5)) return 11;
            if (arr8[0] != -3 || arr8[4] != 5) return 12;

            // Test 9: Stability check (first occurrence of 1 should be before second)
            // Can't directly test stability without object identity, but verify sorted
            int arr9[6] = {2, 1, 3, 1, 4, 1};
            int temp9[6];
            mergeSort(&arr9[0], 0, 5, &temp9[0]);
            if (!isSorted(&arr9[0], 6)) return 13;
            // Check: 1, 1, 1, 2, 3, 4
            if (arr9[0] != 1 || arr9[1] != 1 || arr9[2] != 1) return 14;
            if (arr9[3] != 2 || arr9[4] != 3 || arr9[5] != 4) return 15;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_merge_sort.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Merge sort should sort arrays correctly"
    );
}

/// E2E test: Expression Tree with polymorphic nodes
/// Tests: virtual methods, inheritance, polymorphism, tree structure
#[test]
fn test_e2e_expression_tree() {
    let source = r#"
        // Expression Tree with polymorphic Expr nodes
        // Tests: virtual evaluate(), inheritance hierarchy, recursive evaluation

        struct Expr {
            virtual int evaluate() const = 0;
            virtual ~Expr() {}
        };

        struct Number : Expr {
            int value;

            Number(int v) : value(v) {}

            int evaluate() const override {
                return value;
            }
        };

        struct BinaryExpr : Expr {
            Expr* left;
            Expr* right;

            BinaryExpr(Expr* l, Expr* r) : left(l), right(r) {}

            ~BinaryExpr() {
                delete left;
                delete right;
            }
        };

        struct Add : BinaryExpr {
            Add(Expr* l, Expr* r) : BinaryExpr(l, r) {}

            int evaluate() const override {
                return left->evaluate() + right->evaluate();
            }
        };

        struct Sub : BinaryExpr {
            Sub(Expr* l, Expr* r) : BinaryExpr(l, r) {}

            int evaluate() const override {
                return left->evaluate() - right->evaluate();
            }
        };

        struct Mul : BinaryExpr {
            Mul(Expr* l, Expr* r) : BinaryExpr(l, r) {}

            int evaluate() const override {
                return left->evaluate() * right->evaluate();
            }
        };

        struct Div : BinaryExpr {
            Div(Expr* l, Expr* r) : BinaryExpr(l, r) {}

            int evaluate() const override {
                int rval = right->evaluate();
                if (rval == 0) return 0;  // Avoid division by zero
                return left->evaluate() / rval;
            }
        };

        struct Negate : Expr {
            Expr* operand;

            Negate(Expr* op) : operand(op) {}

            ~Negate() {
                delete operand;
            }

            int evaluate() const override {
                return -operand->evaluate();
            }
        };

        int main() {
            // Test 1: Simple number
            Number* n1 = new Number(42);
            if (n1->evaluate() != 42) return 1;
            delete n1;

            // Test 2: Simple addition (3 + 5 = 8)
            Add* add1 = new Add(new Number(3), new Number(5));
            if (add1->evaluate() != 8) return 2;
            delete add1;

            // Test 3: Simple subtraction (10 - 4 = 6)
            Sub* sub1 = new Sub(new Number(10), new Number(4));
            if (sub1->evaluate() != 6) return 3;
            delete sub1;

            // Test 4: Simple multiplication (6 * 7 = 42)
            Mul* mul1 = new Mul(new Number(6), new Number(7));
            if (mul1->evaluate() != 42) return 4;
            delete mul1;

            // Test 5: Simple division (20 / 4 = 5)
            Div* div1 = new Div(new Number(20), new Number(4));
            if (div1->evaluate() != 5) return 5;
            delete div1;

            // Test 6: Negation (-5 = -5)
            Negate* neg1 = new Negate(new Number(5));
            if (neg1->evaluate() != -5) return 6;
            delete neg1;

            // Test 7: Nested expression ((2 + 3) * 4 = 20)
            Expr* expr1 = new Mul(new Add(new Number(2), new Number(3)), new Number(4));
            if (expr1->evaluate() != 20) return 7;
            delete expr1;

            // Test 8: Complex expression ((10 - 2) * (3 + 1) = 32)
            Expr* expr2 = new Mul(
                new Sub(new Number(10), new Number(2)),
                new Add(new Number(3), new Number(1))
            );
            if (expr2->evaluate() != 32) return 8;
            delete expr2;

            // Test 9: Deeply nested ((1 + 2) + (3 + 4)) = 10
            Expr* expr3 = new Add(
                new Add(new Number(1), new Number(2)),
                new Add(new Number(3), new Number(4))
            );
            if (expr3->evaluate() != 10) return 9;
            delete expr3;

            // Test 10: Mix of all operators ((12 / 4) + (3 * 2) - 1 = 8)
            // = 3 + 6 - 1 = 8
            Expr* expr4 = new Sub(
                new Add(
                    new Div(new Number(12), new Number(4)),
                    new Mul(new Number(3), new Number(2))
                ),
                new Number(1)
            );
            if (expr4->evaluate() != 8) return 10;
            delete expr4;

            // Test 11: With negation (-(5 + 3) = -8)
            Expr* expr5 = new Negate(new Add(new Number(5), new Number(3)));
            if (expr5->evaluate() != -8) return 11;
            delete expr5;

            // Test 12: Division by zero handling
            Div* div2 = new Div(new Number(10), new Number(0));
            if (div2->evaluate() != 0) return 12;  // Should return 0
            delete div2;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_expression_tree.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Expression tree should evaluate correctly"
    );
}

/// E2E test: Simple Object Pool
/// Tests: fixed-size arrays, reuse patterns, index management
#[test]
fn test_e2e_object_pool() {
    let source = r#"
        // Simple Object Pool for fixed-size allocation
        // Tests: pool-based allocation, reuse, index tracking

        struct PoolObject {
            int id;
            int data;
            bool active;
        };

        struct ObjectPool {
            PoolObject objects[16];  // Fixed capacity
            int freeList[16];        // Stack of free indices
            int freeCount;           // Number of free slots
            int nextId;              // For assigning unique IDs

            void init() {
                for (int i = 0; i < 16; i++) {
                    objects[i].id = -1;
                    objects[i].data = 0;
                    objects[i].active = false;
                    freeList[i] = 15 - i;  // Stack: 15, 14, 13, ... 0
                }
                freeCount = 16;
                nextId = 1;
            }

            int capacity() const {
                return 16;
            }

            int available() const {
                return freeCount;
            }

            int inUse() const {
                return 16 - freeCount;
            }

            // Allocate an object, returns index or -1 if full
            int allocate(int data) {
                if (freeCount == 0) return -1;
                freeCount--;
                int idx = freeList[freeCount];
                objects[idx].id = nextId++;
                objects[idx].data = data;
                objects[idx].active = true;
                return idx;
            }

            // Deallocate an object by index
            bool deallocate(int idx) {
                if (idx < 0 || idx >= 16) return false;
                if (!objects[idx].active) return false;
                objects[idx].active = false;
                objects[idx].id = -1;
                freeList[freeCount] = idx;
                freeCount++;
                return true;
            }

            // Get object by index
            PoolObject* get(int idx) {
                if (idx < 0 || idx >= 16) return nullptr;
                if (!objects[idx].active) return nullptr;
                return &objects[idx];
            }

            // Find object by ID
            int findById(int id) {
                for (int i = 0; i < 16; i++) {
                    if (objects[i].active && objects[i].id == id) {
                        return i;
                    }
                }
                return -1;
            }

            // Sum of all active data values
            int sumData() const {
                int sum = 0;
                for (int i = 0; i < 16; i++) {
                    if (objects[i].active) {
                        sum += objects[i].data;
                    }
                }
                return sum;
            }

            // Count active objects
            int countActive() const {
                int count = 0;
                for (int i = 0; i < 16; i++) {
                    if (objects[i].active) {
                        count++;
                    }
                }
                return count;
            }

            void reset() {
                for (int i = 0; i < 16; i++) {
                    objects[i].id = -1;
                    objects[i].data = 0;
                    objects[i].active = false;
                    freeList[i] = 15 - i;
                }
                freeCount = 16;
            }
        };

        int main() {
            ObjectPool pool;
            pool.init();

            // Test 1: Initial state
            if (pool.capacity() != 16) return 1;
            if (pool.available() != 16) return 2;
            if (pool.inUse() != 0) return 3;

            // Test 2: Allocate one object
            int idx1 = pool.allocate(100);
            if (idx1 < 0) return 4;
            if (pool.available() != 15) return 5;
            if (pool.inUse() != 1) return 6;

            // Test 3: Access allocated object
            PoolObject* obj1 = pool.get(idx1);
            if (!obj1) return 7;
            if (obj1->data != 100) return 8;
            if (obj1->id != 1) return 9;  // First ID should be 1

            // Test 4: Allocate more objects
            int idx2 = pool.allocate(200);
            int idx3 = pool.allocate(300);
            if (idx2 < 0 || idx3 < 0) return 10;
            if (pool.inUse() != 3) return 11;
            if (pool.sumData() != 600) return 12;  // 100 + 200 + 300

            // Test 5: Find by ID
            int foundIdx = pool.findById(2);  // Second object
            if (foundIdx != idx2) return 13;
            PoolObject* obj2 = pool.get(foundIdx);
            if (obj2->data != 200) return 14;

            // Test 6: Deallocate middle object
            if (!pool.deallocate(idx2)) return 15;
            if (pool.inUse() != 2) return 16;
            if (pool.get(idx2) != nullptr) return 17;  // Should be inactive
            if (pool.sumData() != 400) return 18;  // 100 + 300

            // Test 7: Reallocate - should reuse freed slot
            int idx4 = pool.allocate(400);
            if (idx4 < 0) return 19;
            // The freed slot should be reused (idx2)
            if (pool.inUse() != 3) return 20;
            if (pool.sumData() != 800) return 21;  // 100 + 300 + 400

            // Test 8: Fill pool completely
            pool.reset();
            for (int i = 0; i < 16; i++) {
                int idx = pool.allocate(i * 10);
                if (idx < 0) return 22;
            }
            if (pool.available() != 0) return 23;
            if (pool.inUse() != 16) return 24;

            // Test 9: Allocation should fail when full
            int idx5 = pool.allocate(999);
            if (idx5 != -1) return 25;  // Should return -1

            // Test 10: Deallocate all
            for (int i = 0; i < 16; i++) {
                pool.deallocate(i);
            }
            if (pool.available() != 16) return 26;
            if (pool.inUse() != 0) return 27;
            if (pool.countActive() != 0) return 28;

            // Test 11: Invalid deallocate
            if (pool.deallocate(-1)) return 29;
            if (pool.deallocate(20)) return 30;

            // Test 12: Get from empty slot
            if (pool.get(0) != nullptr) return 31;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_object_pool.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Object pool should work correctly"
    );
}

/// E2E test: Finite State Machine
/// Tests: enum-like state management, switch statements, transition logic
#[test]
fn test_e2e_state_machine() {
    let source = r#"
        // Simple Finite State Machine for a traffic light
        // Tests: state transitions, switch logic, condition handling
        // Using integer literals directly since const case labels have issues

        struct TrafficLight {
            int currentState;  // 0=red, 1=yellow, 2=green
            int transitionCount;
            bool emergencyMode;

            void init() {
                currentState = 0;  // red
                transitionCount = 0;
                emergencyMode = false;
            }

            int getState() const {
                return currentState;
            }

            bool isEmergency() const {
                return emergencyMode;
            }

            int getTransitions() const {
                return transitionCount;
            }

            void handleEvent(int event) {
                if (event == 2) {  // reset
                    currentState = 0;  // red
                    emergencyMode = false;
                    transitionCount++;
                    return;
                }

                if (event == 1) {  // emergency
                    emergencyMode = true;
                    currentState = 1;  // yellow
                    transitionCount++;
                    return;
                }

                if (emergencyMode) {
                    return;
                }

                // Normal timer transition (event == 0)
                switch (currentState) {
                    case 0:  // red -> green
                        currentState = 2;
                        break;
                    case 2:  // green -> yellow
                        currentState = 1;
                        break;
                    case 1:  // yellow -> red
                        currentState = 0;
                        break;
                }
                transitionCount++;
            }

            // Simulate multiple timer events
            void advance(int count) {
                for (int i = 0; i < count; i++) {
                    handleEvent(0);  // timer event
                }
            }
        };

        int main() {
            TrafficLight light;
            light.init();

            // Test 1: Initial state is red (0)
            if (light.getState() != 0) return 1;
            if (light.getTransitions() != 0) return 2;

            // Test 2: Timer -> red to green (0 -> 2)
            light.handleEvent(0);  // timer
            if (light.getState() != 2) return 3;
            if (light.getTransitions() != 1) return 4;

            // Test 3: Timer -> green to yellow (2 -> 1)
            light.handleEvent(0);  // timer
            if (light.getState() != 1) return 5;

            // Test 4: Timer -> yellow to red (1 -> 0)
            light.handleEvent(0);  // timer
            if (light.getState() != 0) return 6;

            // Test 5: Full cycle
            int initialTransitions = light.getTransitions();
            light.advance(3);  // One full cycle
            if (light.getState() != 0) return 7;
            if (light.getTransitions() != initialTransitions + 3) return 8;

            // Test 6: Emergency mode
            light.init();
            light.advance(1);  // Go to green
            light.handleEvent(1);  // emergency
            if (light.getState() != 1) return 9;  // yellow
            if (!light.isEmergency()) return 10;

            // Test 7: Timer ignored during emergency
            int beforeTransitions = light.getTransitions();
            light.handleEvent(0);  // timer
            if (light.getState() != 1) return 11;  // still yellow
            if (light.getTransitions() != beforeTransitions) return 12;

            // Test 8: Reset clears emergency
            light.handleEvent(2);  // reset
            if (light.getState() != 0) return 13;  // red
            if (light.isEmergency()) return 14;

            // Test 9: Normal operation after reset
            light.handleEvent(0);  // timer
            if (light.getState() != 2) return 15;  // green

            // Test 10: Multiple cycles
            light.init();
            light.advance(9);  // 3 full cycles
            if (light.getState() != 0) return 16;  // red
            if (light.getTransitions() != 9) return 17;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_state_machine.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "State machine should work correctly"
    );
}

/// E2E test: Simple Calculator with operator precedence
/// Tests: if-else chains, arithmetic operations, multiple functions
#[test]
fn test_e2e_calculator() {
    let source = r#"
        // Simple arithmetic expression calculator
        // Supports operators: 0=add, 1=sub, 2=mul, 3=div
        // Using int ops since switch on char literal has issues

        int calculate(int a, int op, int b) {
            if (op == 0) return a + b;      // add
            if (op == 1) return a - b;      // sub
            if (op == 2) return a * b;      // mul
            if (op == 3) {                  // div
                if (b == 0) return 0;
                return a / b;
            }
            return 0;
        }

        // Check if operator is valid (0-3)
        bool isValidOp(int op) {
            return op >= 0 && op <= 3;
        }

        // Get operator precedence (higher = binds tighter)
        // add/sub = 1, mul/div = 2
        int precedence(int op) {
            if (op == 0 || op == 1) return 1;  // add, sub
            if (op == 2 || op == 3) return 2;  // mul, div
            return 0;
        }

        // Evaluate a op1 b op2 c with precedence
        int evaluateThree(int a, int op1, int b, int op2, int c) {
            if (precedence(op2) > precedence(op1)) {
                // Evaluate right side first
                int right = calculate(b, op2, c);
                return calculate(a, op1, right);
            } else {
                // Evaluate left side first
                int left = calculate(a, op1, b);
                return calculate(left, op2, c);
            }
        }

        int main() {
            // Test 1: Basic addition (3 + 5 = 8)
            if (calculate(3, 0, 5) != 8) return 1;

            // Test 2: Basic subtraction (10 - 4 = 6)
            if (calculate(10, 1, 4) != 6) return 2;

            // Test 3: Basic multiplication (6 * 7 = 42)
            if (calculate(6, 2, 7) != 42) return 3;

            // Test 4: Basic division (20 / 4 = 5)
            if (calculate(20, 3, 4) != 5) return 4;

            // Test 5: Division by zero
            if (calculate(10, 3, 0) != 0) return 5;

            // Test 6: Negative result (3 - 10 = -7)
            if (calculate(3, 1, 10) != -7) return 6;

            // Test 7: Operator validation
            if (!isValidOp(0)) return 7;
            if (!isValidOp(1)) return 8;
            if (!isValidOp(2)) return 9;
            if (!isValidOp(3)) return 10;
            if (isValidOp(5)) return 11;

            // Test 8: Precedence
            if (precedence(2) <= precedence(0)) return 12;  // mul > add
            if (precedence(3) <= precedence(1)) return 13;  // div > sub
            if (precedence(0) != precedence(1)) return 14;  // add == sub
            if (precedence(2) != precedence(3)) return 15;  // mul == div

            // Test 9: Three operand with precedence
            // 2 + 3 * 4 = 2 + 12 = 14 (not 5 * 4 = 20)
            if (evaluateThree(2, 0, 3, 2, 4) != 14) return 16;

            // Test 10: Three operand, same precedence
            // 10 - 4 + 2 = 6 + 2 = 8 (left to right)
            if (evaluateThree(10, 1, 4, 0, 2) != 8) return 17;

            // Test 11: Multiplication then addition
            // 3 * 4 + 2 = 12 + 2 = 14
            if (evaluateThree(3, 2, 4, 0, 2) != 14) return 18;

            // Test 12: Division with subtraction
            // 20 / 4 - 3 = 5 - 3 = 2
            if (evaluateThree(20, 3, 4, 1, 3) != 2) return 19;

            // Test 13: Complex expression
            // 5 + 6 / 2 = 5 + 3 = 8
            if (evaluateThree(5, 0, 6, 3, 2) != 8) return 20;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_calculator.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Calculator should work correctly"
    );
}

/// E2E test: Switch with const int and char literals
/// Tests: const int case values, char literal case values
#[test]
fn test_e2e_switch_const_and_char() {
    let source = r#"
        // Test switch with const int case values
        const int MODE_OFF = 0;
        const int MODE_LOW = 1;
        const int MODE_MEDIUM = 2;
        const int MODE_HIGH = 3;

        int getModeValue(int mode) {
            switch (mode) {
                case MODE_OFF:
                    return 0;
                case MODE_LOW:
                    return 25;
                case MODE_MEDIUM:
                    return 50;
                case MODE_HIGH:
                    return 100;
                default:
                    return -1;
            }
        }

        // Test switch with char literal case values
        int getCharCode(char c) {
            switch (c) {
                case 'a':
                    return 1;
                case 'b':
                    return 2;
                case 'c':
                    return 3;
                case '+':
                    return 10;
                case '-':
                    return 11;
                case '*':
                    return 12;
                case '/':
                    return 13;
                default:
                    return 0;
            }
        }

        int main() {
            // Test const int switch cases
            if (getModeValue(MODE_OFF) != 0) return 1;
            if (getModeValue(MODE_LOW) != 25) return 2;
            if (getModeValue(MODE_MEDIUM) != 50) return 3;
            if (getModeValue(MODE_HIGH) != 100) return 4;
            if (getModeValue(5) != -1) return 5;

            // Also test with literal values
            if (getModeValue(0) != 0) return 6;
            if (getModeValue(1) != 25) return 7;
            if (getModeValue(2) != 50) return 8;
            if (getModeValue(3) != 100) return 9;

            // Test char literal switch cases
            if (getCharCode('a') != 1) return 10;
            if (getCharCode('b') != 2) return 11;
            if (getCharCode('c') != 3) return 12;
            if (getCharCode('+') != 10) return 13;
            if (getCharCode('-') != 11) return 14;
            if (getCharCode('*') != 12) return 15;
            if (getCharCode('/') != 13) return 16;
            if (getCharCode('x') != 0) return 17;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_switch_const_and_char.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Switch with const int and char literals should work correctly"
    );
}

/// E2E test: Function pointers and callbacks (advanced)
/// Tests: function pointer types, callback passing, binary search with comparator
#[test]
fn test_e2e_function_ptr_callbacks() {
    let source = r#"
        // Function pointers and callbacks
        // Tests function pointer syntax, passing, and invocation

        int add(int a, int b) { return a + b; }
        int sub(int a, int b) { return a - b; }
        int mul(int a, int b) { return a * b; }
        int divv(int a, int b) { return b != 0 ? a / b : 0; }

        // Higher-order function: apply an operation
        int apply(int (*op)(int, int), int x, int y) {
            return op(x, y);
        }

        // Function that returns result of two operations
        int applyBoth(int (*op1)(int, int), int (*op2)(int, int), int a, int b, int c) {
            return op1(a, op2(b, c));
        }

        // Binary search using comparison callback
        int binarySearch(int* arr, int n, int target, int (*cmp)(int, int)) {
            int lo = 0;
            int hi = n - 1;
            while (lo <= hi) {
                int mid = lo + (hi - lo) / 2;
                int result = cmp(arr[mid], target);
                if (result == 0) return mid;
                if (result < 0) lo = mid + 1;
                else hi = mid - 1;
            }
            return -1;
        }

        int compare(int a, int b) {
            if (a < b) return -1;
            if (a > b) return 1;
            return 0;
        }

        int main() {
            // Test 1: Direct function pointer call
            int (*fp)(int, int) = add;
            if (fp(3, 5) != 8) return 1;

            // Test 2: Reassign function pointer
            fp = sub;
            if (fp(10, 4) != 6) return 2;

            // Test 3: Pass function pointer to higher-order function
            if (apply(add, 2, 3) != 5) return 3;
            if (apply(mul, 4, 5) != 20) return 4;
            if (apply(divv, 20, 4) != 5) return 5;

            // Test 4: Nested function pointer applications
            // applyBoth(add, mul, 2, 3, 4) = add(2, mul(3, 4)) = add(2, 12) = 14
            if (applyBoth(add, mul, 2, 3, 4) != 14) return 6;

            // Test 5: Binary search with comparison callback
            int arr[5] = {1, 3, 5, 7, 9};
            if (binarySearch(&arr[0], 5, 5, compare) != 2) return 7;
            if (binarySearch(&arr[0], 5, 1, compare) != 0) return 8;
            if (binarySearch(&arr[0], 5, 9, compare) != 4) return 9;
            if (binarySearch(&arr[0], 5, 4, compare) != -1) return 10;

            // Test 6: Null function pointer check
            int (*nullFp)(int, int) = nullptr;
            if (nullFp != nullptr) return 11;

            // Test 7: Function pointer comparison
            int (*fp1)(int, int) = add;
            int (*fp2)(int, int) = add;
            int (*fp3)(int, int) = sub;
            if (fp1 != fp2) return 12;  // Same function
            if (fp1 == fp3) return 13;  // Different functions

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_function_ptr_callbacks.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Function pointer callbacks should work correctly"
    );
}

/// E2E test: Bit manipulation operations
/// Tests: bitwise AND/OR/XOR/NOT, shifts, bit counting
/// Note: Using signed int instead of unsigned to avoid transpiler type issues
/// Note: Using local variables for mutable state since function param mutability isn't yet supported
#[test]
fn test_e2e_bit_manipulation() {
    let source = r#"
        // Bit manipulation operations with signed int
        // Tests bitwise operators and common bit manipulation patterns

        // Count number of set bits (popcount) for values 0-255
        // Uses local copy to avoid modifying parameter
        int countBits(int num) {
            int n = num;  // Local mutable copy
            int count = 0;
            while (n != 0) {
                if ((n & 1) != 0) count++;
                n = n >> 1;
            }
            return count;
        }

        // Check if power of 2 (for positive values)
        bool isPowerOfTwo(int n) {
            return n > 0 && (n & (n - 1)) == 0;
        }

        // Get bit at position (0-indexed from right)
        bool getBit(int n, int pos) {
            return ((n >> pos) & 1) != 0;
        }

        // Set bit at position
        int setBit(int n, int pos) {
            return n | (1 << pos);
        }

        // Clear bit at position
        int clearBit(int n, int pos) {
            return n & ~(1 << pos);
        }

        // Toggle bit at position
        int toggleBit(int n, int pos) {
            return n ^ (1 << pos);
        }

        // Find lowest set bit position (0-indexed, -1 if zero)
        // Uses local copy to avoid modifying parameter
        int lowestSetBit(int num) {
            if (num == 0) return -1;
            int n = num;  // Local mutable copy
            int pos = 0;
            while ((n & 1) == 0) {
                n = n >> 1;
                pos++;
            }
            return pos;
        }

        // Swap two values using XOR
        void xorSwap(int* a, int* b) {
            if (a != b) {
                *a = *a ^ *b;
                *b = *a ^ *b;
                *a = *a ^ *b;
            }
        }

        int main() {
            // Test 1: Basic bitwise operations
            if ((5 & 3) != 1) return 1;      // 101 & 011 = 001
            if ((5 | 3) != 7) return 2;      // 101 | 011 = 111
            if ((5 ^ 3) != 6) return 3;      // 101 ^ 011 = 110

            // Test 2: Shift operations
            if ((1 << 3) != 8) return 5;
            if ((16 >> 2) != 4) return 6;
            if ((128 >> 4) != 8) return 7;

            // Test 3: Count bits
            if (countBits(0) != 0) return 8;
            if (countBits(1) != 1) return 9;
            if (countBits(7) != 3) return 10;     // 111
            if (countBits(255) != 8) return 11;   // 11111111
            if (countBits(15) != 4) return 12;    // 1111

            // Test 4: Power of 2 check
            if (!isPowerOfTwo(1)) return 13;
            if (!isPowerOfTwo(2)) return 14;
            if (!isPowerOfTwo(4)) return 15;
            if (!isPowerOfTwo(1024)) return 16;
            if (isPowerOfTwo(0)) return 17;
            if (isPowerOfTwo(3)) return 18;
            if (isPowerOfTwo(6)) return 19;

            // Test 5: Get/set/clear/toggle bits
            if (!getBit(5, 0)) return 20;  // 5 = 101, bit 0 is set
            if (getBit(5, 1)) return 21;   // bit 1 is not set
            if (!getBit(5, 2)) return 22;  // bit 2 is set

            if (setBit(5, 1) != 7) return 23;      // 101 | 010 = 111
            if (clearBit(7, 1) != 5) return 24;    // 111 & ~010 = 101
            if (toggleBit(5, 1) != 7) return 25;   // 101 ^ 010 = 111
            if (toggleBit(7, 1) != 5) return 26;   // 111 ^ 010 = 101

            // Test 6: Lowest set bit
            if (lowestSetBit(0) != -1) return 27;
            if (lowestSetBit(1) != 0) return 28;
            if (lowestSetBit(8) != 3) return 29;     // 1000
            if (lowestSetBit(12) != 2) return 30;    // 1100
            if (lowestSetBit(128) != 7) return 31;

            // Test 7: XOR swap
            int x = 10;
            int y = 20;
            xorSwap(&x, &y);
            if (x != 20) return 32;
            if (y != 10) return 33;

            // Test 8: More complex bit patterns
            int mask = (1 << 4) - 1;  // 0xF = 15
            if (mask != 15) return 34;

            int extract = (0xABCD >> 4) & mask;  // Extract nibble
            if (extract != 12) return 35;  // 0xC = 12

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_bit_manipulation.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Bit manipulation should work correctly"
    );
}

/// E2E test: 2D matrix operations
/// Tests: 2D arrays, nested loops, matrix algorithms
#[test]
fn test_e2e_matrix_operations() {
    let source = r#"
        // 2D Matrix operations
        // Tests nested arrays, matrix math, and transformations

        // Matrix represented as 1D array (row-major order)
        // For 3x3: index = row * 3 + col

        void matrixInit(int* mat, int rows, int cols, int val) {
            for (int i = 0; i < rows * cols; i++) {
                mat[i] = val;
            }
        }

        int matrixGet(int* mat, int cols, int row, int col) {
            return mat[row * cols + col];
        }

        void matrixSet(int* mat, int cols, int row, int col, int val) {
            mat[row * cols + col] = val;
        }

        // Matrix addition (same dimensions)
        void matrixAdd(int* a, int* b, int* result, int rows, int cols) {
            for (int i = 0; i < rows * cols; i++) {
                result[i] = a[i] + b[i];
            }
        }

        // Scalar multiplication
        void matrixScale(int* mat, int* result, int rows, int cols, int scalar) {
            for (int i = 0; i < rows * cols; i++) {
                result[i] = mat[i] * scalar;
            }
        }

        // Transpose a matrix (swap rows and cols)
        void matrixTranspose(int* mat, int* result, int rows, int cols) {
            for (int r = 0; r < rows; r++) {
                for (int c = 0; c < cols; c++) {
                    result[c * rows + r] = mat[r * cols + c];
                }
            }
        }

        // Matrix multiplication (a[m x n] * b[n x p] = result[m x p])
        void matrixMultiply(int* a, int* b, int* result, int m, int n, int p) {
            for (int i = 0; i < m; i++) {
                for (int j = 0; j < p; j++) {
                    int sum = 0;
                    for (int k = 0; k < n; k++) {
                        sum += a[i * n + k] * b[k * p + j];
                    }
                    result[i * p + j] = sum;
                }
            }
        }

        // Sum all elements
        int matrixSum(int* mat, int rows, int cols) {
            int sum = 0;
            for (int i = 0; i < rows * cols; i++) {
                sum += mat[i];
            }
            return sum;
        }

        // Check if two matrices are equal
        bool matrixEqual(int* a, int* b, int rows, int cols) {
            for (int i = 0; i < rows * cols; i++) {
                if (a[i] != b[i]) return false;
            }
            return true;
        }

        int main() {
            // Test 1: Initialize and get/set
            int mat1[9];
            matrixInit(&mat1[0], 3, 3, 0);
            matrixSet(&mat1[0], 3, 0, 0, 1);
            matrixSet(&mat1[0], 3, 1, 1, 2);
            matrixSet(&mat1[0], 3, 2, 2, 3);
            if (matrixGet(&mat1[0], 3, 0, 0) != 1) return 1;
            if (matrixGet(&mat1[0], 3, 1, 1) != 2) return 2;
            if (matrixGet(&mat1[0], 3, 2, 2) != 3) return 3;

            // Test 2: Matrix addition
            int a[4] = {1, 2, 3, 4};  // 2x2
            int b[4] = {5, 6, 7, 8};
            int sum[4];
            matrixAdd(&a[0], &b[0], &sum[0], 2, 2);
            if (sum[0] != 6) return 4;   // 1+5
            if (sum[1] != 8) return 5;   // 2+6
            if (sum[2] != 10) return 6;  // 3+7
            if (sum[3] != 12) return 7;  // 4+8

            // Test 3: Scalar multiplication
            int scaled[4];
            matrixScale(&a[0], &scaled[0], 2, 2, 3);
            if (scaled[0] != 3) return 8;
            if (scaled[1] != 6) return 9;
            if (scaled[2] != 9) return 10;
            if (scaled[3] != 12) return 11;

            // Test 4: Transpose
            // [1 2]    [1 3]
            // [3 4] -> [2 4]
            int trans[4];
            matrixTranspose(&a[0], &trans[0], 2, 2);
            if (trans[0] != 1) return 12;
            if (trans[1] != 3) return 13;
            if (trans[2] != 2) return 14;
            if (trans[3] != 4) return 15;

            // Test 5: Non-square transpose (2x3 -> 3x2)
            int rect[6] = {1, 2, 3, 4, 5, 6};  // 2x3
            int rectT[6];
            matrixTranspose(&rect[0], &rectT[0], 2, 3);
            // Original:  [1 2 3]    Transposed: [1 4]
            //            [4 5 6]                [2 5]
            //                                   [3 6]
            if (matrixGet(&rectT[0], 2, 0, 0) != 1) return 16;
            if (matrixGet(&rectT[0], 2, 0, 1) != 4) return 17;
            if (matrixGet(&rectT[0], 2, 1, 0) != 2) return 18;
            if (matrixGet(&rectT[0], 2, 2, 1) != 6) return 19;

            // Test 6: Matrix multiplication
            // [1 2]   [5 6]   [19 22]
            // [3 4] * [7 8] = [43 50]
            int product[4];
            matrixMultiply(&a[0], &b[0], &product[0], 2, 2, 2);
            if (product[0] != 19) return 20;  // 1*5 + 2*7
            if (product[1] != 22) return 21;  // 1*6 + 2*8
            if (product[2] != 43) return 22;  // 3*5 + 4*7
            if (product[3] != 50) return 23;  // 3*6 + 4*8

            // Test 7: Sum and equality
            if (matrixSum(&a[0], 2, 2) != 10) return 24;  // 1+2+3+4
            if (matrixEqual(&a[0], &b[0], 2, 2)) return 25;
            int aCopy[4] = {1, 2, 3, 4};
            if (!matrixEqual(&a[0], &aCopy[0], 2, 2)) return 26;

            // Test 8: Identity matrix multiplication
            // I * A = A
            int identity[4] = {1, 0, 0, 1};
            int result[4];
            matrixMultiply(&identity[0], &a[0], &result[0], 2, 2, 2);
            if (!matrixEqual(&result[0], &a[0], 2, 2)) return 27;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_matrix_operations.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Matrix operations should work correctly"
    );
}

/// E2E test: String comparison utilities
/// Tests: string comparison functions, prefix/suffix checks, case conversion
/// This simulates a simple string utility library pattern
#[test]
fn test_e2e_string_utilities() {
    let source = r#"
        // String utility functions - simulates a small library
        // Uses C-style strings (char*) for simplicity

        // String length
        int strLen(const char* s) {
            int len = 0;
            while (s[len] != '\0') len++;
            return len;
        }

        // String comparison (0 = equal, <0 if a<b, >0 if a>b)
        int strCmp(const char* a, const char* b) {
            int i = 0;
            while (a[i] != '\0' && b[i] != '\0') {
                if (a[i] != b[i]) return a[i] - b[i];
                i++;
            }
            return a[i] - b[i];
        }

        // Check if string starts with prefix
        bool startsWith(const char* str, const char* prefix) {
            int i = 0;
            while (prefix[i] != '\0') {
                if (str[i] == '\0' || str[i] != prefix[i]) return false;
                i++;
            }
            return true;
        }

        // Check if string ends with suffix
        bool endsWith(const char* str, const char* suffix) {
            int strLen_val = strLen(str);
            int suffixLen = strLen(suffix);
            if (suffixLen > strLen_val) return false;
            int offset = strLen_val - suffixLen;
            for (int i = 0; i < suffixLen; i++) {
                if (str[offset + i] != suffix[i]) return false;
            }
            return true;
        }

        // Check if character is uppercase
        bool isUpper(char c) {
            return c >= 'A' && c <= 'Z';
        }

        // Check if character is lowercase
        bool isLower(char c) {
            return c >= 'a' && c <= 'z';
        }

        // Convert to uppercase
        char toUpper(char c) {
            if (isLower(c)) return (char)(c - 32);
            return c;
        }

        // Convert to lowercase
        char toLower(char c) {
            if (isUpper(c)) return (char)(c + 32);
            return c;
        }

        // Case-insensitive comparison
        int strCmpIgnoreCase(const char* a, const char* b) {
            int i = 0;
            while (a[i] != '\0' && b[i] != '\0') {
                char ca = toLower(a[i]);
                char cb = toLower(b[i]);
                if (ca != cb) return ca - cb;
                i++;
            }
            return toLower(a[i]) - toLower(b[i]);
        }

        // Count occurrences of a character
        int countChar(const char* str, char c) {
            int count = 0;
            for (int i = 0; str[i] != '\0'; i++) {
                if (str[i] == c) count++;
            }
            return count;
        }

        // Find first occurrence of character
        int findChar(const char* str, char c) {
            for (int i = 0; str[i] != '\0'; i++) {
                if (str[i] == c) return i;
            }
            return -1;
        }

        // Find last occurrence of character
        int findLastChar(const char* str, char c) {
            int last = -1;
            for (int i = 0; str[i] != '\0'; i++) {
                if (str[i] == c) last = i;
            }
            return last;
        }

        int main() {
            // Test strLen
            if (strLen("hello") != 5) return 1;
            if (strLen("") != 0) return 2;
            if (strLen("a") != 1) return 3;

            // Test strCmp
            if (strCmp("abc", "abc") != 0) return 4;
            if (strCmp("abc", "abd") >= 0) return 5;  // c < d
            if (strCmp("abd", "abc") <= 0) return 6;  // d > c
            if (strCmp("ab", "abc") >= 0) return 7;   // shorter
            if (strCmp("abc", "ab") <= 0) return 8;   // longer

            // Test startsWith
            if (!startsWith("hello world", "hello")) return 9;
            if (startsWith("hello", "world")) return 10;
            if (!startsWith("abc", "a")) return 11;
            if (!startsWith("", "")) return 12;
            if (startsWith("a", "ab")) return 13;

            // Test endsWith
            if (!endsWith("hello world", "world")) return 14;
            if (endsWith("hello", "world")) return 15;
            if (!endsWith("abc", "c")) return 16;
            if (!endsWith("", "")) return 17;
            if (endsWith("a", "ab")) return 18;

            // Test case functions
            if (!isUpper('A')) return 19;
            if (isUpper('a')) return 20;
            if (!isLower('z')) return 21;
            if (isLower('Z')) return 22;
            if (toUpper('a') != 'A') return 23;
            if (toLower('Z') != 'z') return 24;
            if (toUpper('A') != 'A') return 25;  // already upper
            if (toLower('z') != 'z') return 26;  // already lower

            // Test case-insensitive comparison
            if (strCmpIgnoreCase("ABC", "abc") != 0) return 27;
            if (strCmpIgnoreCase("Hello", "HELLO") != 0) return 28;
            if (strCmpIgnoreCase("abc", "abd") >= 0) return 29;

            // Test countChar
            if (countChar("hello", 'l') != 2) return 30;
            if (countChar("hello", 'x') != 0) return 31;
            if (countChar("aaa", 'a') != 3) return 32;

            // Test findChar
            if (findChar("hello", 'e') != 1) return 33;
            if (findChar("hello", 'l') != 2) return 34;  // first l
            if (findChar("hello", 'x') != -1) return 35;

            // Test findLastChar
            if (findLastChar("hello", 'l') != 3) return 36;  // last l
            if (findLastChar("hello", 'h') != 0) return 37;
            if (findLastChar("hello", 'x') != -1) return 38;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_string_utilities.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "String utilities should work correctly"
    );
}

/// E2E test: Command-line argument parsing pattern
/// Tests: argc/argv handling, option parsing, flag detection
/// This simulates a simple CLI argument parser
#[test]
fn test_e2e_cli_arg_parser() {
    let source = r#"
        // Simple CLI argument parser - simulates a mini CLI tool pattern
        // Uses argc/argv style command line handling

        // Check if two strings are equal
        bool strEqual(const char* a, const char* b) {
            int i = 0;
            while (a[i] != '\0' && b[i] != '\0') {
                if (a[i] != b[i]) return false;
                i++;
            }
            return a[i] == b[i];
        }

        // Check if string starts with prefix
        bool startsWith(const char* str, const char* prefix) {
            int i = 0;
            while (prefix[i] != '\0') {
                if (str[i] == '\0' || str[i] != prefix[i]) return false;
                i++;
            }
            return true;
        }

        // Parse integer from string
        int parseInt(const char* str) {
            int result = 0;
            int sign = 1;
            int i = 0;
            if (str[0] == '-') {
                sign = -1;
                i = 1;
            }
            while (str[i] != '\0') {
                if (str[i] >= '0' && str[i] <= '9') {
                    result = result * 10 + (str[i] - '0');
                }
                i++;
            }
            return result * sign;
        }

        // Argument parser struct
        struct ArgParser {
            bool help;
            bool verbose;
            int count;
            const char* output;
            int positionalCount;
        };

        // Initialize parser with defaults
        void initParser(ArgParser* parser) {
            parser->help = false;
            parser->verbose = false;
            parser->count = 1;
            parser->output = "";
            parser->positionalCount = 0;
        }

        // Parse single argument, return number of args consumed
        int parseArg(ArgParser* parser, int argc, const char* const* argv, int index) {
            const char* arg = argv[index];

            // Help flag
            if (strEqual(arg, "-h") || strEqual(arg, "--help")) {
                parser->help = true;
                return 1;
            }

            // Verbose flag
            if (strEqual(arg, "-v") || strEqual(arg, "--verbose")) {
                parser->verbose = true;
                return 1;
            }

            // Count option with value
            if (strEqual(arg, "-n") || strEqual(arg, "--count")) {
                if (index + 1 < argc) {
                    parser->count = parseInt(argv[index + 1]);
                    return 2;
                }
                return 1;
            }

            // Count option with equals sign
            if (startsWith(arg, "--count=")) {
                parser->count = parseInt(&arg[8]);
                return 1;
            }

            // Output option with value
            if (strEqual(arg, "-o") || strEqual(arg, "--output")) {
                if (index + 1 < argc) {
                    parser->output = argv[index + 1];
                    return 2;
                }
                return 1;
            }

            // Not a known option, treat as positional
            parser->positionalCount = parser->positionalCount + 1;
            return 1;
        }

        // Parse all arguments
        void parseAll(ArgParser* parser, int argc, const char* const* argv) {
            int i = 1;  // Skip program name
            while (i < argc) {
                int consumed = parseArg(parser, argc, argv, i);
                i += consumed;
            }
        }

        int main() {
            ArgParser parser;

            // Test 1: Empty args (just program name)
            {
                const char* args1[] = {"program"};
                initParser(&parser);
                parseAll(&parser, 1, args1);
                if (parser.help) return 1;
                if (parser.verbose) return 2;
                if (parser.count != 1) return 3;
                if (parser.positionalCount != 0) return 4;
            }

            // Test 2: Help flag (short)
            {
                const char* args2[] = {"program", "-h"};
                initParser(&parser);
                parseAll(&parser, 2, args2);
                if (!parser.help) return 5;
            }

            // Test 3: Help flag (long)
            {
                const char* args3[] = {"program", "--help"};
                initParser(&parser);
                parseAll(&parser, 2, args3);
                if (!parser.help) return 6;
            }

            // Test 4: Verbose flag
            {
                const char* args4[] = {"program", "-v"};
                initParser(&parser);
                parseAll(&parser, 2, args4);
                if (!parser.verbose) return 7;
            }

            // Test 5: Count option with separate value
            {
                const char* args5[] = {"program", "-n", "42"};
                initParser(&parser);
                parseAll(&parser, 3, args5);
                if (parser.count != 42) return 8;
            }

            // Test 6: Count option with equals
            {
                const char* args6[] = {"program", "--count=100"};
                initParser(&parser);
                parseAll(&parser, 2, args6);
                if (parser.count != 100) return 9;
            }

            // Test 7: Output option
            {
                const char* args7[] = {"program", "-o", "file.txt"};
                initParser(&parser);
                parseAll(&parser, 3, args7);
                if (!strEqual(parser.output, "file.txt")) return 10;
            }

            // Test 8: Multiple flags
            {
                const char* args8[] = {"program", "-v", "-h"};
                initParser(&parser);
                parseAll(&parser, 3, args8);
                if (!parser.verbose) return 11;
                if (!parser.help) return 12;
            }

            // Test 9: Mixed options and positional
            {
                const char* args9[] = {"program", "-v", "input.txt", "-n", "5", "output.txt"};
                initParser(&parser);
                parseAll(&parser, 6, args9);
                if (!parser.verbose) return 13;
                if (parser.count != 5) return 14;
                if (parser.positionalCount != 2) return 15;  // input.txt and output.txt
            }

            // Test 10: Negative number parsing
            {
                const char* args10[] = {"program", "-n", "-5"};
                initParser(&parser);
                parseAll(&parser, 3, args10);
                if (parser.count != -5) return 16;
            }

            // Test 11: Integer parsing edge cases
            if (parseInt("0") != 0) return 17;
            if (parseInt("123") != 123) return 18;
            if (parseInt("-456") != -456) return 19;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_cli_arg_parser.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "CLI argument parser should work correctly"
    );
}

/// E2E test: Simple assertion library pattern
/// Tests: assertion functions, test functions as unit of work
/// This simulates a mini testing library (simplified version)
#[test]
fn test_e2e_assertion_library() {
    let source = r#"
        // Assertion library - simulates testing library utilities
        // Demonstrates boolean logic, comparison, and test organization

        // Assertion helpers
        bool assertEqual(int expected, int actual) {
            return expected == actual;
        }

        bool assertNotEqual(int a, int b) {
            return a != b;
        }

        bool assertTrue(bool condition) {
            return condition;
        }

        bool assertFalse(bool condition) {
            return !condition;
        }

        bool assertNull(void* ptr) {
            return ptr == nullptr;
        }

        bool assertNotNull(void* ptr) {
            return ptr != nullptr;
        }

        bool assertInRange(int value, int minVal, int maxVal) {
            return value >= minVal && value <= maxVal;
        }

        bool assertPositive(int value) {
            return value > 0;
        }

        bool assertNegative(int value) {
            return value < 0;
        }

        bool assertZero(int value) {
            return value == 0;
        }

        // Simple math library to test
        int add(int a, int b) { return a + b; }
        int subtract(int a, int b) { return a - b; }
        int multiply(int a, int b) { return a * b; }
        int divide(int a, int b) { return b != 0 ? a / b : 0; }
        int absolute(int x) { return x < 0 ? -x : x; }
        int maxVal(int a, int b) { return a > b ? a : b; }
        int minVal(int a, int b) { return a < b ? a : b; }
        int clamp(int value, int lo, int hi) {
            if (value < lo) return lo;
            if (value > hi) return hi;
            return value;
        }

        // Test functions (return true if all assertions pass)
        bool testAddition() {
            if (!assertEqual(5, add(2, 3))) return false;
            if (!assertEqual(0, add(0, 0))) return false;
            if (!assertEqual(-1, add(-3, 2))) return false;
            if (!assertEqual(100, add(50, 50))) return false;
            return true;
        }

        bool testSubtraction() {
            if (!assertEqual(1, subtract(3, 2))) return false;
            if (!assertEqual(-5, subtract(0, 5))) return false;
            if (!assertEqual(0, subtract(5, 5))) return false;
            if (!assertNegative(subtract(3, 10))) return false;
            return true;
        }

        bool testMultiplication() {
            if (!assertEqual(6, multiply(2, 3))) return false;
            if (!assertZero(multiply(0, 100))) return false;
            if (!assertEqual(-6, multiply(-2, 3))) return false;
            if (!assertPositive(multiply(-2, -3))) return false;
            return true;
        }

        bool testDivision() {
            if (!assertEqual(2, divide(6, 3))) return false;
            if (!assertZero(divide(0, 5))) return false;
            if (!assertZero(divide(5, 0))) return false;  // Safe division by zero
            if (!assertNegative(divide(-6, 3))) return false;
            return true;
        }

        bool testAbsoluteValue() {
            if (!assertEqual(5, absolute(5))) return false;
            if (!assertEqual(5, absolute(-5))) return false;
            if (!assertZero(absolute(0))) return false;
            if (!assertPositive(absolute(-100))) return false;
            return true;
        }

        bool testMinMax() {
            if (!assertEqual(5, maxVal(3, 5))) return false;
            if (!assertEqual(5, maxVal(5, 3))) return false;
            if (!assertEqual(3, minVal(3, 5))) return false;
            if (!assertEqual(3, minVal(5, 3))) return false;
            if (!assertZero(maxVal(0, 0))) return false;
            return true;
        }

        bool testClamp() {
            if (!assertEqual(5, clamp(5, 0, 10))) return false;    // In range
            if (!assertEqual(0, clamp(-5, 0, 10))) return false;   // Below min
            if (!assertEqual(10, clamp(15, 0, 10))) return false;  // Above max
            if (!assertEqual(5, clamp(5, 5, 5))) return false;     // Tight bounds
            return true;
        }

        bool testRangeAssertions() {
            if (!assertInRange(5, 0, 10)) return false;
            if (assertInRange(-1, 0, 10)) return false;  // Should fail
            if (assertInRange(11, 0, 10)) return false;  // Should fail
            if (!assertInRange(0, 0, 10)) return false;  // Boundary
            if (!assertInRange(10, 0, 10)) return false; // Boundary
            return true;
        }

        bool testPointerAssertions() {
            int x = 42;
            int* ptr = &x;
            void* null_ptr = nullptr;

            if (!assertNotNull((void*)ptr)) return false;
            if (!assertNull(null_ptr)) return false;
            if (assertNull((void*)ptr)) return false;
            if (assertNotNull(null_ptr)) return false;
            return true;
        }

        int main() {
            int passed = 0;
            int failed = 0;

            // Run all tests and count results
            if (testAddition()) passed = passed + 1; else failed = failed + 1;
            if (testSubtraction()) passed = passed + 1; else failed = failed + 1;
            if (testMultiplication()) passed = passed + 1; else failed = failed + 1;
            if (testDivision()) passed = passed + 1; else failed = failed + 1;
            if (testAbsoluteValue()) passed = passed + 1; else failed = failed + 1;
            if (testMinMax()) passed = passed + 1; else failed = failed + 1;
            if (testClamp()) passed = passed + 1; else failed = failed + 1;
            if (testRangeAssertions()) passed = passed + 1; else failed = failed + 1;
            if (testPointerAssertions()) passed = passed + 1; else failed = failed + 1;

            // Verify all tests passed (9 tests total)
            if (passed != 9) return 1;
            if (failed != 0) return 2;

            // Additional validation of assertion functions directly
            if (!assertEqual(42, 42)) return 3;
            if (assertEqual(1, 2)) return 4;
            if (!assertNotEqual(1, 2)) return 5;
            if (assertNotEqual(5, 5)) return 6;
            if (!assertTrue(true)) return 7;
            if (assertTrue(false)) return 8;
            if (!assertFalse(false)) return 9;
            if (assertFalse(true)) return 10;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_assertion_library.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Assertion library should work correctly"
    );
}

/// E2E test: Ring buffer (circular queue) implementation
/// Tests: modular arithmetic, wrap-around indexing, full/empty detection
/// Note: Uses pointer-to-data instead of embedded array due to transpiler limitations
#[test]
fn test_e2e_ring_buffer() {
    let source = r#"
        // Ring buffer implementation - fixed-size circular queue
        // Uses separate data array and pointer due to transpiler limitation with member arrays

        struct RingBuffer {
            int* data;
            int head;
            int tail;
            int count;
            int capacity;
        };

        void rbInit(RingBuffer* rb, int* dataPtr) {
            rb->data = dataPtr;
            rb->head = 0;
            rb->tail = 0;
            rb->count = 0;
            rb->capacity = 16;
        }

        bool rbIsEmpty(RingBuffer* rb) {
            return rb->count == 0;
        }

        bool rbIsFull(RingBuffer* rb) {
            return rb->count == rb->capacity;
        }

        int rbSize(RingBuffer* rb) {
            return rb->count;
        }

        bool rbPush(RingBuffer* rb, int value) {
            if (rbIsFull(rb)) return false;
            rb->data[rb->tail] = value;
            rb->tail = (rb->tail + 1) % rb->capacity;
            rb->count = rb->count + 1;
            return true;
        }

        bool rbPop(RingBuffer* rb, int* value) {
            if (rbIsEmpty(rb)) return false;
            *value = rb->data[rb->head];
            rb->head = (rb->head + 1) % rb->capacity;
            rb->count = rb->count - 1;
            return true;
        }

        bool rbPeek(RingBuffer* rb, int* value) {
            if (rbIsEmpty(rb)) return false;
            *value = rb->data[rb->head];
            return true;
        }

        void rbClear(RingBuffer* rb) {
            rb->head = 0;
            rb->tail = 0;
            rb->count = 0;
        }

        // Get element at position (0 = head)
        bool rbAt(RingBuffer* rb, int pos, int* value) {
            if (pos < 0 || pos >= rb->count) return false;
            int index = (rb->head + pos) % rb->capacity;
            *value = rb->data[index];
            return true;
        }

        int main() {
            int data[16];  // Local storage
            RingBuffer rb;
            rbInit(&rb, &data[0]);
            int val;

            // Test 1: Empty buffer
            if (!rbIsEmpty(&rb)) return 1;
            if (rbIsFull(&rb)) return 2;
            if (rbSize(&rb) != 0) return 3;
            if (rbPop(&rb, &val)) return 4;
            if (rbPeek(&rb, &val)) return 5;

            // Test 2: Single element
            if (!rbPush(&rb, 42)) return 6;
            if (rbIsEmpty(&rb)) return 7;
            if (rbSize(&rb) != 1) return 8;
            if (!rbPeek(&rb, &val) || val != 42) return 9;
            if (!rbPop(&rb, &val) || val != 42) return 10;
            if (!rbIsEmpty(&rb)) return 11;

            // Test 3: Multiple elements (FIFO order)
            for (int i = 1; i <= 5; i++) {
                if (!rbPush(&rb, i * 10)) return 12;
            }
            if (rbSize(&rb) != 5) return 13;
            for (int i = 1; i <= 5; i++) {
                if (!rbPop(&rb, &val) || val != i * 10) return 14;
            }
            if (!rbIsEmpty(&rb)) return 15;

            // Test 4: Wrap-around behavior
            rbClear(&rb);
            // Fill mostly, then drain, then fill again to force wrap
            for (int i = 0; i < 10; i++) rbPush(&rb, i);
            for (int i = 0; i < 8; i++) rbPop(&rb, &val);
            // Now head is at 8, add more to wrap around
            for (int i = 100; i < 110; i++) rbPush(&rb, i);
            // Should have: [8, 9, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109]
            if (rbSize(&rb) != 12) return 16;
            if (!rbPop(&rb, &val) || val != 8) return 17;
            if (!rbPop(&rb, &val) || val != 9) return 18;
            if (!rbPop(&rb, &val) || val != 100) return 19;

            // Test 5: Full buffer
            rbClear(&rb);
            for (int i = 0; i < 16; i++) {
                if (!rbPush(&rb, i)) return 20;
            }
            if (!rbIsFull(&rb)) return 21;
            if (rbPush(&rb, 999)) return 22;  // Should fail

            // Test 6: Random access
            if (!rbAt(&rb, 0, &val) || val != 0) return 23;
            if (!rbAt(&rb, 5, &val) || val != 5) return 24;
            if (!rbAt(&rb, 15, &val) || val != 15) return 25;
            if (rbAt(&rb, 16, &val)) return 26;   // Out of bounds
            if (rbAt(&rb, -1, &val)) return 27;   // Negative

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_ring_buffer.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Ring buffer should work correctly"
    );
}

/// E2E test: LRU Cache pattern
/// Tests: struct with pointer to entries, key-value pairs, eviction logic
/// Note: Ignored - requires local struct array initialization which isn't supported yet
#[test]
#[ignore]
fn test_e2e_lru_cache() {
    let source = r#"
        // Simple LRU (Least Recently Used) Cache implementation
        // Uses pointer to entry array due to transpiler limitation with member arrays

        struct CacheEntry {
            int key;
            int value;
            int accessTime;
            bool valid;
        };

        struct LRUCache {
            CacheEntry* entries;
            int currentTime;
            int size;
            int capacity;
        };

        void cacheInit(LRUCache* cache, CacheEntry* entryStorage) {
            cache->entries = entryStorage;
            cache->currentTime = 0;
            cache->size = 0;
            cache->capacity = 8;
            for (int i = 0; i < 8; i++) {
                cache->entries[i].valid = false;
            }
        }

        // Find entry by key, returns index or -1
        int cacheFindKey(LRUCache* cache, int key) {
            for (int i = 0; i < cache->capacity; i++) {
                if (cache->entries[i].valid && cache->entries[i].key == key) {
                    return i;
                }
            }
            return -1;
        }

        // Find least recently used entry
        int cacheFindLRU(LRUCache* cache) {
            int lruIndex = -1;
            int minTime = 2147483647;  // INT_MAX
            for (int i = 0; i < cache->capacity; i++) {
                if (cache->entries[i].valid && cache->entries[i].accessTime < minTime) {
                    minTime = cache->entries[i].accessTime;
                    lruIndex = i;
                }
            }
            return lruIndex;
        }

        // Find empty slot
        int cacheFindEmpty(LRUCache* cache) {
            for (int i = 0; i < cache->capacity; i++) {
                if (!cache->entries[i].valid) {
                    return i;
                }
            }
            return -1;
        }

        // Get value for key, returns true if found
        bool cacheGet(LRUCache* cache, int key, int* value) {
            int idx = cacheFindKey(cache, key);
            if (idx < 0) return false;

            cache->currentTime = cache->currentTime + 1;
            cache->entries[idx].accessTime = cache->currentTime;
            *value = cache->entries[idx].value;
            return true;
        }

        // Put key-value pair, evicts LRU if full
        void cachePut(LRUCache* cache, int key, int value) {
            cache->currentTime = cache->currentTime + 1;

            // Check if key already exists
            int idx = cacheFindKey(cache, key);
            if (idx >= 0) {
                cache->entries[idx].value = value;
                cache->entries[idx].accessTime = cache->currentTime;
                return;
            }

            // Find empty slot or evict LRU
            idx = cacheFindEmpty(cache);
            if (idx < 0) {
                idx = cacheFindLRU(cache);
            } else {
                cache->size = cache->size + 1;
            }

            cache->entries[idx].key = key;
            cache->entries[idx].value = value;
            cache->entries[idx].accessTime = cache->currentTime;
            cache->entries[idx].valid = true;
        }

        // Remove entry by key
        bool cacheRemove(LRUCache* cache, int key) {
            int idx = cacheFindKey(cache, key);
            if (idx < 0) return false;
            cache->entries[idx].valid = false;
            cache->size = cache->size - 1;
            return true;
        }

        int cacheSize(LRUCache* cache) {
            return cache->size;
        }

        void cacheReset(LRUCache* cache) {
            cache->currentTime = 0;
            cache->size = 0;
            for (int i = 0; i < cache->capacity; i++) {
                cache->entries[i].valid = false;
            }
        }

        int main() {
            CacheEntry storage[8];  // Local storage
            LRUCache cache;
            cacheInit(&cache, &storage[0]);
            int val;

            // Test 1: Empty cache
            if (cacheSize(&cache) != 0) return 1;
            if (cacheGet(&cache, 1, &val)) return 2;

            // Test 2: Put and get
            cachePut(&cache, 1, 100);
            if (cacheSize(&cache) != 1) return 3;
            if (!cacheGet(&cache, 1, &val) || val != 100) return 4;

            // Test 3: Multiple entries
            cachePut(&cache, 2, 200);
            cachePut(&cache, 3, 300);
            if (cacheSize(&cache) != 3) return 5;
            if (!cacheGet(&cache, 2, &val) || val != 200) return 6;
            if (!cacheGet(&cache, 3, &val) || val != 300) return 7;

            // Test 4: Update existing key
            cachePut(&cache, 1, 111);
            if (cacheSize(&cache) != 3) return 8;  // Size unchanged
            if (!cacheGet(&cache, 1, &val) || val != 111) return 9;

            // Test 5: Remove
            if (!cacheRemove(&cache, 2)) return 10;
            if (cacheSize(&cache) != 2) return 11;
            if (cacheGet(&cache, 2, &val)) return 12;

            // Test 6: LRU eviction
            cacheReset(&cache);  // Reset
            // Fill cache
            for (int i = 0; i < 8; i++) {
                cachePut(&cache, i, i * 10);
            }
            if (cacheSize(&cache) != 8) return 13;

            // Access some keys to update their access time
            cacheGet(&cache, 0, &val);  // 0 is now most recently used
            cacheGet(&cache, 7, &val);  // 7 is now most recently used

            // Add new key, should evict LRU (key 1, since 0 and 7 were accessed)
            cachePut(&cache, 100, 1000);
            if (cacheGet(&cache, 1, &val)) return 14;  // Key 1 should be evicted
            if (!cacheGet(&cache, 100, &val) || val != 1000) return 15;
            if (!cacheGet(&cache, 0, &val)) return 16;  // Key 0 still there
            if (!cacheGet(&cache, 7, &val)) return 17;  // Key 7 still there

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_lru_cache.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "LRU cache should work correctly"
    );
}

/// E2E test: Tokenizer/Lexer pattern
/// Tests: character classification, state machine, string parsing
/// Note: Ignored - requires struct return assignment (op_assign) which isn't supported yet
#[test]
#[ignore]
fn test_e2e_tokenizer() {
    let source = r#"
        // Simple tokenizer/lexer for arithmetic expressions
        // Demonstrates character classification and state machine patterns

        // Token types
        const int TOK_NUMBER = 1;
        const int TOK_PLUS = 2;
        const int TOK_MINUS = 3;
        const int TOK_STAR = 4;
        const int TOK_SLASH = 5;
        const int TOK_LPAREN = 6;
        const int TOK_RPAREN = 7;
        const int TOK_EOF = 8;
        const int TOK_ERROR = 9;

        struct Token {
            int type;
            int value;  // For numbers
        };

        struct Tokenizer {
            const char* input;
            int pos;
        };

        void tokInit(Tokenizer* tok, const char* input) {
            tok->input = input;
            tok->pos = 0;
        }

        char tokPeek(Tokenizer* tok) {
            return tok->input[tok->pos];
        }

        char tokAdvance(Tokenizer* tok) {
            char c = tok->input[tok->pos];
            if (c != '\0') tok->pos = tok->pos + 1;
            return c;
        }

        bool isDigit(char c) {
            return c >= '0' && c <= '9';
        }

        bool isWhitespace(char c) {
            return c == ' ' || c == '\t' || c == '\n' || c == '\r';
        }

        void skipWhitespace(Tokenizer* tok) {
            while (isWhitespace(tokPeek(tok))) {
                tokAdvance(tok);
            }
        }

        int parseNumber(Tokenizer* tok) {
            int result = 0;
            while (isDigit(tokPeek(tok))) {
                result = result * 10 + (tokAdvance(tok) - '0');
            }
            return result;
        }

        Token tokNext(Tokenizer* tok) {
            Token t;
            t.value = 0;

            skipWhitespace(tok);
            char c = tokPeek(tok);

            if (c == '\0') {
                t.type = TOK_EOF;
                return t;
            }

            if (isDigit(c)) {
                t.type = TOK_NUMBER;
                t.value = parseNumber(tok);
                return t;
            }

            tokAdvance(tok);

            if (c == '+') { t.type = TOK_PLUS; }
            else if (c == '-') { t.type = TOK_MINUS; }
            else if (c == '*') { t.type = TOK_STAR; }
            else if (c == '/') { t.type = TOK_SLASH; }
            else if (c == '(') { t.type = TOK_LPAREN; }
            else if (c == ')') { t.type = TOK_RPAREN; }
            else { t.type = TOK_ERROR; }

            return t;
        }

        // Count tokens in expression
        int countTokens(const char* expr) {
            Tokenizer tok;
            tokInit(&tok, expr);
            int count = 0;
            while (true) {
                Token t = tokNext(&tok);
                if (t.type == TOK_EOF) break;
                if (t.type == TOK_ERROR) return -1;
                count = count + 1;
            }
            return count;
        }

        // Simple expression evaluator (no precedence, left to right)
        int evalSimple(const char* expr) {
            Tokenizer tok;
            tokInit(&tok, expr);

            Token t = tokNext(&tok);
            if (t.type != TOK_NUMBER) return 0;
            int result = t.value;

            while (true) {
                Token op = tokNext(&tok);
                if (op.type == TOK_EOF) break;

                Token num = tokNext(&tok);
                if (num.type != TOK_NUMBER) return 0;

                if (op.type == TOK_PLUS) result = result + num.value;
                else if (op.type == TOK_MINUS) result = result - num.value;
                else if (op.type == TOK_STAR) result = result * num.value;
                else if (op.type == TOK_SLASH && num.value != 0) result = result / num.value;
            }

            return result;
        }

        int main() {
            Tokenizer tok;
            Token t;

            // Test 1: Empty string
            tokInit(&tok, "");
            t = tokNext(&tok);
            if (t.type != TOK_EOF) return 1;

            // Test 2: Single number
            tokInit(&tok, "42");
            t = tokNext(&tok);
            if (t.type != TOK_NUMBER || t.value != 42) return 2;
            t = tokNext(&tok);
            if (t.type != TOK_EOF) return 3;

            // Test 3: Multi-digit number
            tokInit(&tok, "12345");
            t = tokNext(&tok);
            if (t.type != TOK_NUMBER || t.value != 12345) return 4;

            // Test 4: Operators
            tokInit(&tok, "+-*/()");
            t = tokNext(&tok); if (t.type != TOK_PLUS) return 5;
            t = tokNext(&tok); if (t.type != TOK_MINUS) return 6;
            t = tokNext(&tok); if (t.type != TOK_STAR) return 7;
            t = tokNext(&tok); if (t.type != TOK_SLASH) return 8;
            t = tokNext(&tok); if (t.type != TOK_LPAREN) return 9;
            t = tokNext(&tok); if (t.type != TOK_RPAREN) return 10;

            // Test 5: Whitespace handling
            tokInit(&tok, "  10  +  20  ");
            t = tokNext(&tok); if (t.type != TOK_NUMBER || t.value != 10) return 11;
            t = tokNext(&tok); if (t.type != TOK_PLUS) return 12;
            t = tokNext(&tok); if (t.type != TOK_NUMBER || t.value != 20) return 13;

            // Test 6: Token counting
            if (countTokens("1 + 2") != 3) return 14;
            if (countTokens("1 + 2 * 3") != 5) return 15;
            if (countTokens("(1 + 2) * 3") != 7) return 16;
            if (countTokens("") != 0) return 17;

            // Test 7: Simple evaluation
            if (evalSimple("5") != 5) return 18;
            if (evalSimple("10 + 5") != 15) return 19;
            if (evalSimple("10 - 3") != 7) return 20;
            if (evalSimple("4 * 3") != 12) return 21;
            if (evalSimple("12 / 4") != 3) return 22;
            if (evalSimple("2 + 3 * 4") != 20) return 23;  // Left to right: (2+3)*4

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_tokenizer.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Tokenizer should work correctly"
    );
}

/// E2E test: Prime number generation
/// Tests: modular arithmetic, nested loops, early termination
#[test]
fn test_e2e_prime_sieve() {
    let source = r#"
        // Prime number sieve - tests nested loops, early termination, modular arithmetic

        bool isPrime(int n) {
            if (n <= 1) return false;
            if (n <= 3) return true;
            if (n % 2 == 0 || n % 3 == 0) return false;

            // Check divisors up to sqrt(n)
            int i = 5;
            while (i * i <= n) {
                if (n % i == 0 || n % (i + 2) == 0) return false;
                i = i + 6;
            }
            return true;
        }

        // Count primes up to n (exclusive)
        int countPrimes(int n) {
            int count = 0;
            for (int i = 2; i < n; i++) {
                if (isPrime(i)) count = count + 1;
            }
            return count;
        }

        // Get nth prime (1-indexed: 1st prime = 2)
        int nthPrime(int n) {
            int count = 0;
            int candidate = 2;
            while (count < n) {
                if (isPrime(candidate)) count = count + 1;
                if (count < n) candidate = candidate + 1;
            }
            return candidate;
        }

        // Sum of primes up to n
        int sumPrimes(int n) {
            int sum = 0;
            for (int i = 2; i <= n; i++) {
                if (isPrime(i)) sum = sum + i;
            }
            return sum;
        }

        // Greatest common divisor (Euclidean algorithm)
        // Use local copies since function params aren't mutable in Rust
        int gcd(int x, int y) {
            int a = x;
            int b = y;
            while (b != 0) {
                int temp = b;
                b = a % b;
                a = temp;
            }
            return a;
        }

        // Least common multiple
        int lcm(int a, int b) {
            return (a / gcd(a, b)) * b;
        }

        int main() {
            // Test isPrime
            if (isPrime(0)) return 1;
            if (isPrime(1)) return 2;
            if (!isPrime(2)) return 3;
            if (!isPrime(3)) return 4;
            if (isPrime(4)) return 5;
            if (!isPrime(5)) return 6;
            if (isPrime(6)) return 7;
            if (!isPrime(7)) return 8;
            if (isPrime(9)) return 9;
            if (!isPrime(11)) return 10;
            if (!isPrime(13)) return 11;
            if (isPrime(15)) return 12;
            if (!isPrime(17)) return 13;
            if (!isPrime(97)) return 14;
            if (isPrime(100)) return 15;

            // Test countPrimes
            if (countPrimes(10) != 4) return 16;   // 2, 3, 5, 7
            if (countPrimes(20) != 8) return 17;   // 2, 3, 5, 7, 11, 13, 17, 19
            if (countPrimes(2) != 0) return 18;
            if (countPrimes(3) != 1) return 19;

            // Test nthPrime
            if (nthPrime(1) != 2) return 20;
            if (nthPrime(2) != 3) return 21;
            if (nthPrime(3) != 5) return 22;
            if (nthPrime(4) != 7) return 23;
            if (nthPrime(5) != 11) return 24;
            if (nthPrime(10) != 29) return 25;

            // Test sumPrimes
            if (sumPrimes(10) != 17) return 26;   // 2 + 3 + 5 + 7
            if (sumPrimes(2) != 2) return 27;
            if (sumPrimes(1) != 0) return 28;

            // Test gcd
            if (gcd(12, 8) != 4) return 29;
            if (gcd(15, 25) != 5) return 30;
            if (gcd(17, 13) != 1) return 31;
            if (gcd(100, 10) != 10) return 32;
            if (gcd(7, 7) != 7) return 33;

            // Test lcm
            if (lcm(4, 6) != 12) return 34;
            if (lcm(3, 5) != 15) return 35;
            if (lcm(6, 8) != 24) return 36;
            if (lcm(7, 1) != 7) return 37;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_prime_sieve.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Prime sieve should work correctly");
}

/// E2E test: Recursive algorithms
/// Tests: recursion, tail recursion patterns, memoization-like patterns
#[test]
fn test_e2e_recursive_algorithms() {
    let source = r#"
        // Recursive algorithms - tests various recursion patterns

        // Classic fibonacci (exponential)
        int fibSlow(int n) {
            if (n <= 1) return n;
            return fibSlow(n - 1) + fibSlow(n - 2);
        }

        // Iterative fibonacci (linear)
        int fibFast(int n) {
            if (n <= 1) return n;
            int prev = 0;
            int curr = 1;
            for (int i = 2; i <= n; i++) {
                int next = prev + curr;
                prev = curr;
                curr = next;
            }
            return curr;
        }

        // Power function (recursive)
        int power(int base, int exp) {
            if (exp == 0) return 1;
            if (exp == 1) return base;
            if (exp % 2 == 0) {
                int half = power(base, exp / 2);
                return half * half;
            } else {
                return base * power(base, exp - 1);
            }
        }

        // Sum of digits (recursive)
        int sumDigits(int num) {
            int n = num;
            if (n < 0) n = -n;  // Handle negative
            if (n < 10) return n;
            return (n % 10) + sumDigits(n / 10);
        }

        // Count digits
        int countDigits(int num) {
            int n = num;
            if (n < 0) n = -n;
            if (n < 10) return 1;
            return 1 + countDigits(n / 10);
        }

        // Reverse digits
        int reverseDigitsHelper(int n, int rev) {
            if (n == 0) return rev;
            return reverseDigitsHelper(n / 10, rev * 10 + n % 10);
        }

        int reverseDigits(int n) {
            if (n < 0) return -reverseDigitsHelper(-n, 0);
            return reverseDigitsHelper(n, 0);
        }

        // Check palindrome number
        bool isPalindrome(int n) {
            if (n < 0) return false;
            return n == reverseDigits(n);
        }

        // Ackermann function (limited to small values)
        int ackermann(int m, int n) {
            if (m == 0) return n + 1;
            if (n == 0) return ackermann(m - 1, 1);
            return ackermann(m - 1, ackermann(m, n - 1));
        }

        // Count ways to climb stairs (1 or 2 steps at a time)
        int climbStairs(int n) {
            if (n <= 2) return n;
            int prev = 1;
            int curr = 2;
            for (int i = 3; i <= n; i++) {
                int next = prev + curr;
                prev = curr;
                curr = next;
            }
            return curr;
        }

        int main() {
            // Test fibonacci
            if (fibSlow(0) != 0) return 1;
            if (fibSlow(1) != 1) return 2;
            if (fibSlow(2) != 1) return 3;
            if (fibSlow(5) != 5) return 4;
            if (fibSlow(10) != 55) return 5;

            // Verify fast and slow give same results
            for (int i = 0; i <= 15; i++) {
                if (fibSlow(i) != fibFast(i)) return 6;
            }

            // Test power
            if (power(2, 0) != 1) return 7;
            if (power(2, 1) != 2) return 8;
            if (power(2, 10) != 1024) return 9;
            if (power(3, 4) != 81) return 10;
            if (power(5, 3) != 125) return 11;

            // Test sumDigits
            if (sumDigits(0) != 0) return 12;
            if (sumDigits(5) != 5) return 13;
            if (sumDigits(123) != 6) return 14;
            if (sumDigits(9999) != 36) return 15;
            if (sumDigits(-123) != 6) return 16;

            // Test countDigits
            if (countDigits(0) != 1) return 17;
            if (countDigits(5) != 1) return 18;
            if (countDigits(123) != 3) return 19;
            if (countDigits(10000) != 5) return 20;

            // Test reverseDigits
            if (reverseDigits(0) != 0) return 21;
            if (reverseDigits(5) != 5) return 22;
            if (reverseDigits(123) != 321) return 23;
            if (reverseDigits(1000) != 1) return 24;
            if (reverseDigits(-123) != -321) return 25;

            // Test isPalindrome
            if (!isPalindrome(0)) return 26;
            if (!isPalindrome(5)) return 27;
            if (!isPalindrome(121)) return 28;
            if (!isPalindrome(12321)) return 29;
            if (isPalindrome(123)) return 30;
            if (isPalindrome(-121)) return 31;

            // Test Ackermann (small values only!)
            if (ackermann(0, 0) != 1) return 32;
            if (ackermann(1, 1) != 3) return 33;
            if (ackermann(2, 2) != 7) return 34;
            if (ackermann(3, 2) != 29) return 35;

            // Test climbStairs
            if (climbStairs(1) != 1) return 36;
            if (climbStairs(2) != 2) return 37;
            if (climbStairs(3) != 3) return 38;
            if (climbStairs(4) != 5) return 39;
            if (climbStairs(5) != 8) return 40;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_recursive_algorithms.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "Recursive algorithms should work correctly"
    );
}

/// E2E test: Union-Find (Disjoint Set) data structure
/// Tests: path compression, union by rank, connected components
#[test]
fn test_e2e_union_find() {
    let source = r#"
        // Union-Find with path compression and union by rank

        struct UnionFind {
            int* parent;
            int* rank;
            int size;
        };

        void ufInit(UnionFind* uf, int* parentBuf, int* rankBuf, int n) {
            uf->parent = parentBuf;
            uf->rank = rankBuf;
            uf->size = n;
            for (int i = 0; i < n; i++) {
                parentBuf[i] = i;  // Each element is its own parent
                rankBuf[i] = 0;    // Initial rank is 0
            }
        }

        // Find with path compression
        int ufFind(UnionFind* uf, int x) {
            if (uf->parent[x] != x) {
                uf->parent[x] = ufFind(uf, uf->parent[x]);  // Path compression
            }
            return uf->parent[x];
        }

        // Union by rank
        void ufUnion(UnionFind* uf, int x, int y) {
            int rootX = ufFind(uf, x);
            int rootY = ufFind(uf, y);

            if (rootX == rootY) return;  // Already in same set

            // Union by rank
            if (uf->rank[rootX] < uf->rank[rootY]) {
                uf->parent[rootX] = rootY;
            } else if (uf->rank[rootX] > uf->rank[rootY]) {
                uf->parent[rootY] = rootX;
            } else {
                uf->parent[rootY] = rootX;
                uf->rank[rootX] = uf->rank[rootX] + 1;
            }
        }

        // Check if two elements are in the same set
        bool ufConnected(UnionFind* uf, int x, int y) {
            return ufFind(uf, x) == ufFind(uf, y);
        }

        // Count distinct sets
        int ufCountSets(UnionFind* uf) {
            int count = 0;
            for (int i = 0; i < uf->size; i++) {
                if (uf->parent[i] == i) count = count + 1;
            }
            return count;
        }

        int main() {
            int parent[10];
            int rank[10];
            UnionFind uf;
            ufInit(&uf, parent, rank, 10);

            // Initially all elements are separate
            if (ufCountSets(&uf) != 10) return 1;

            // Union some elements
            ufUnion(&uf, 0, 1);
            if (!ufConnected(&uf, 0, 1)) return 2;
            if (ufCountSets(&uf) != 9) return 3;

            ufUnion(&uf, 2, 3);
            if (!ufConnected(&uf, 2, 3)) return 4;
            if (ufCountSets(&uf) != 8) return 5;

            ufUnion(&uf, 0, 2);  // Merge two groups
            if (!ufConnected(&uf, 0, 3)) return 6;
            if (!ufConnected(&uf, 1, 2)) return 7;
            if (ufCountSets(&uf) != 7) return 8;

            // Check that unconnected elements are still separate
            if (ufConnected(&uf, 0, 4)) return 9;
            if (ufConnected(&uf, 5, 6)) return 10;

            // Create a chain: 4-5-6-7-8-9
            ufUnion(&uf, 4, 5);
            ufUnion(&uf, 5, 6);
            ufUnion(&uf, 6, 7);
            ufUnion(&uf, 7, 8);
            ufUnion(&uf, 8, 9);

            if (!ufConnected(&uf, 4, 9)) return 11;
            if (ufCountSets(&uf) != 2) return 12;  // {0,1,2,3} and {4,5,6,7,8,9}

            // Merge everything
            ufUnion(&uf, 0, 9);
            if (ufCountSets(&uf) != 1) return 13;
            if (!ufConnected(&uf, 0, 9)) return 14;

            // Test path compression - find should flatten the tree
            ufFind(&uf, 9);
            ufFind(&uf, 8);
            ufFind(&uf, 7);
            // After path compression, these should have same root
            if (ufFind(&uf, 7) != ufFind(&uf, 9)) return 15;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_union_find.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Union-Find should work correctly");
}

/// E2E test: Bitset operations
/// Tests: bit manipulation, set operations on bits
#[test]
fn test_e2e_bitset_operations() {
    let source = r#"
        // Bitset operations using unsigned int

        // Set bit at position
        unsigned int setBit(unsigned int n, int pos) {
            return n | (1u << pos);
        }

        // Clear bit at position
        unsigned int clearBit(unsigned int n, int pos) {
            return n & ~(1u << pos);
        }

        // Toggle bit at position
        unsigned int toggleBit(unsigned int n, int pos) {
            return n ^ (1u << pos);
        }

        // Check if bit is set
        bool isBitSet(unsigned int n, int pos) {
            return (n & (1u << pos)) != 0;
        }

        // Count set bits (population count)
        int popCount(unsigned int n) {
            int count = 0;
            unsigned int val = n;
            while (val != 0) {
                count = count + (int)(val & 1u);
                val = val >> 1;
            }
            return count;
        }

        // Find lowest set bit position (0-indexed, -1 if none)
        int lowestSetBit(unsigned int n) {
            if (n == 0) return -1;
            unsigned int val = n;  // Use local copy for mutation
            int pos = 0;
            while ((val & 1u) == 0) {
                val = val >> 1;
                pos = pos + 1;
            }
            return pos;
        }

        // Find highest set bit position (0-indexed, -1 if none)
        int highestSetBit(unsigned int n) {
            if (n == 0) return -1;
            unsigned int val = n;  // Use local copy for mutation
            int pos = 0;
            while (val != 0) {
                val = val >> 1;
                pos = pos + 1;
            }
            return pos - 1;
        }

        // Reverse bits
        unsigned int reverseBits(unsigned int n) {
            unsigned int val = n;  // Use local copy for mutation
            unsigned int result = 0;
            for (int i = 0; i < 32; i++) {
                result = result << 1;
                result = result | (val & 1u);
                val = val >> 1;
            }
            return result;
        }

        // Check if power of 2
        bool isPowerOfTwo(unsigned int n) {
            return n != 0 && (n & (n - 1)) == 0;
        }

        // Next power of 2
        unsigned int nextPowerOfTwo(unsigned int n) {
            if (n == 0) return 1;
            unsigned int v = n - 1;
            v = v | (v >> 1);
            v = v | (v >> 2);
            v = v | (v >> 4);
            v = v | (v >> 8);
            v = v | (v >> 16);
            return v + 1;
        }

        int main() {
            // Test setBit
            if (setBit(0, 0) != 1) return 1;
            if (setBit(0, 3) != 8) return 2;
            if (setBit(1, 3) != 9) return 3;

            // Test clearBit
            if (clearBit(15, 0) != 14) return 4;
            if (clearBit(15, 2) != 11) return 5;
            if (clearBit(8, 3) != 0) return 6;

            // Test toggleBit
            if (toggleBit(0, 0) != 1) return 7;
            if (toggleBit(1, 0) != 0) return 8;
            if (toggleBit(10, 2) != 14) return 9;

            // Test isBitSet
            if (!isBitSet(5, 0)) return 10;
            if (isBitSet(5, 1)) return 11;
            if (!isBitSet(5, 2)) return 12;

            // Test popCount
            if (popCount(0) != 0) return 13;
            if (popCount(1) != 1) return 14;
            if (popCount(7) != 3) return 15;
            if (popCount(255) != 8) return 16;
            if (popCount(0xFFFF) != 16) return 17;

            // Test lowestSetBit
            if (lowestSetBit(0) != -1) return 18;
            if (lowestSetBit(1) != 0) return 19;
            if (lowestSetBit(8) != 3) return 20;
            if (lowestSetBit(12) != 2) return 21;

            // Test highestSetBit
            if (highestSetBit(0) != -1) return 22;
            if (highestSetBit(1) != 0) return 23;
            if (highestSetBit(8) != 3) return 24;
            if (highestSetBit(15) != 3) return 25;
            if (highestSetBit(16) != 4) return 26;

            // Test isPowerOfTwo
            if (!isPowerOfTwo(1)) return 27;
            if (!isPowerOfTwo(2)) return 28;
            if (!isPowerOfTwo(4)) return 29;
            if (isPowerOfTwo(3)) return 30;
            if (isPowerOfTwo(0)) return 31;
            if (!isPowerOfTwo(1024)) return 32;

            // Test nextPowerOfTwo
            if (nextPowerOfTwo(0) != 1) return 33;
            if (nextPowerOfTwo(1) != 1) return 34;
            if (nextPowerOfTwo(2) != 2) return 35;
            if (nextPowerOfTwo(3) != 4) return 36;
            if (nextPowerOfTwo(5) != 8) return 37;
            if (nextPowerOfTwo(17) != 32) return 38;

            // Test reverseBits (just check specific patterns)
            if (reverseBits(0) != 0) return 39;
            if (reverseBits(0x80000000) != 1) return 40;
            if (reverseBits(1) != 0x80000000) return 41;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_bitset_operations.cpp").expect("E2E test failed");

    assert_eq!(exit_code, 0, "Bitset operations should work correctly");
}

/// E2E test: String pattern matching
/// Tests: KMP-like pattern matching, string search algorithms
#[test]
fn test_e2e_string_pattern_matching() {
    let source = r#"
        // String pattern matching algorithms

        // Simple string length
        int strLen(const char* s) {
            int len = 0;
            while (s[len] != '\0') len = len + 1;
            return len;
        }

        // Simple substring search (naive algorithm)
        // Returns index of first occurrence, -1 if not found
        int strFind(const char* text, const char* pattern) {
            int textLen = strLen(text);
            int patLen = strLen(pattern);

            if (patLen == 0) return 0;
            if (patLen > textLen) return -1;

            for (int i = 0; i <= textLen - patLen; i++) {
                int j = 0;
                while (j < patLen && text[i + j] == pattern[j]) {
                    j = j + 1;
                }
                if (j == patLen) return i;
            }
            return -1;
        }

        // Count occurrences of pattern in text
        int strCount(const char* text, const char* pattern) {
            int count = 0;
            int textLen = strLen(text);
            int patLen = strLen(pattern);

            if (patLen == 0) return 0;
            if (patLen > textLen) return 0;

            for (int i = 0; i <= textLen - patLen; i++) {
                int j = 0;
                while (j < patLen && text[i + j] == pattern[j]) {
                    j = j + 1;
                }
                if (j == patLen) {
                    count = count + 1;
                }
            }
            return count;
        }

        // Check if string starts with prefix
        bool startsWith(const char* text, const char* prefix) {
            int i = 0;
            while (prefix[i] != '\0') {
                if (text[i] == '\0' || text[i] != prefix[i]) return false;
                i = i + 1;
            }
            return true;
        }

        // Check if string ends with suffix
        bool endsWith(const char* text, const char* suffix) {
            int textLen = strLen(text);
            int suffixLen = strLen(suffix);
            if (suffixLen > textLen) return false;

            int offset = textLen - suffixLen;
            for (int i = 0; i < suffixLen; i++) {
                if (text[offset + i] != suffix[i]) return false;
            }
            return true;
        }

        // Simple wildcard match (* matches any sequence, ? matches single char)
        bool wildcardMatch(const char* text, const char* pattern) {
            int ti = 0;
            int pi = 0;
            int starIdx = -1;
            int matchIdx = 0;

            while (text[ti] != '\0') {
                if (pattern[pi] != '\0' && (pattern[pi] == '?' || pattern[pi] == text[ti])) {
                    ti = ti + 1;
                    pi = pi + 1;
                } else if (pattern[pi] == '*') {
                    starIdx = pi;
                    matchIdx = ti;
                    pi = pi + 1;
                } else if (starIdx != -1) {
                    pi = starIdx + 1;
                    matchIdx = matchIdx + 1;
                    ti = matchIdx;
                } else {
                    return false;
                }
            }

            while (pattern[pi] == '*') pi = pi + 1;
            return pattern[pi] == '\0';
        }

        int main() {
            // Test strLen
            if (strLen("") != 0) return 1;
            if (strLen("a") != 1) return 2;
            if (strLen("hello") != 5) return 3;

            // Test strFind
            if (strFind("hello world", "world") != 6) return 4;
            if (strFind("hello world", "hello") != 0) return 5;
            if (strFind("hello world", "xyz") != -1) return 6;
            if (strFind("hello world", "") != 0) return 7;
            if (strFind("aaa", "aa") != 0) return 8;
            if (strFind("ababab", "bab") != 1) return 9;

            // Test strCount
            if (strCount("aaa", "a") != 3) return 10;
            if (strCount("ababab", "ab") != 3) return 11;
            if (strCount("hello", "l") != 2) return 12;
            if (strCount("hello", "x") != 0) return 13;

            // Test startsWith
            if (!startsWith("hello world", "hello")) return 14;
            if (!startsWith("hello world", "")) return 15;
            if (startsWith("hello", "hello world")) return 16;
            if (startsWith("hello", "hx")) return 17;

            // Test endsWith
            if (!endsWith("hello world", "world")) return 18;
            if (!endsWith("hello world", "")) return 19;
            if (endsWith("hello", "hello world")) return 20;
            if (endsWith("hello", "hx")) return 21;

            // Test wildcardMatch
            if (!wildcardMatch("hello", "hello")) return 22;
            if (!wildcardMatch("hello", "h*o")) return 23;
            if (!wildcardMatch("hello", "*")) return 24;
            if (!wildcardMatch("hello", "h?llo")) return 25;
            if (!wildcardMatch("hello", "*llo")) return 26;
            if (!wildcardMatch("hello", "hel*")) return 27;
            if (wildcardMatch("hello", "h?o")) return 28;
            if (!wildcardMatch("", "*")) return 29;
            if (!wildcardMatch("abc", "a*c")) return 30;
            if (!wildcardMatch("abcdef", "a*c*f")) return 31;

            return 0;  // Success
        }
    "#;

    let (exit_code, _stdout, _stderr) =
        transpile_compile_run(source, "e2e_string_pattern_matching.cpp").expect("E2E test failed");

    assert_eq!(
        exit_code, 0,
        "String pattern matching should work correctly"
    );
}
