//! Integration tests for Clang AST parsing and Rust code generation.

use fragile_clang::{ClangParser, AstCodeGen};

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

/// Test generating Rust code from C++ source.
#[test]
fn test_generate_rust_code() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int add(int a, int b) {
            return a + b;
        }
    "#;

    let ast = parser.parse_string(source, "add.cpp").expect("Failed to parse");
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

    let ast = parser.parse_string(source, "test.cpp").expect("Failed to parse");
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

    let ast = parser.parse_string(source, "math.cpp").expect("Failed to parse");
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

    let ast = parser.parse_string(source, "ns.cpp").expect("Failed to parse");
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

    let ast = parser.parse_string(source, "max.cpp").expect("Failed to parse");
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

    let ast = parser.parse_string(source, "sum.cpp").expect("Failed to parse");
    let code = AstCodeGen::new().generate(&ast.translation_unit);

    // Check while loop is preserved
    assert!(code.contains("while i <= n"));
    assert!(code.contains("return sum"));
}

// ============================================================================
// End-to-End Tests: Transpile -> Compile -> Run
// ============================================================================

use std::fs;
use std::process::Command;

/// Helper function to transpile C++ source, compile with rustc, and run.
/// Returns (exit_code, stdout, stderr).
fn transpile_compile_run(cpp_source: &str, filename: &str) -> Result<(i32, String, String), String> {
    let parser = ClangParser::new().map_err(|e| format!("Failed to create parser: {}", e))?;

    // Parse and generate Rust code
    let ast = parser.parse_string(cpp_source, filename).map_err(|e| format!("Failed to parse: {}", e))?;
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_add.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_factorial.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_while.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_for.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_struct.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_arrays.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_pointers.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_references.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_prime.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_constructor.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_nullptr.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_casts.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_new_delete.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_array_new.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "new int[5] should create array that can be indexed");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_inheritance.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Inheritance should embed base struct as __base field");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_destructor.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Struct with destructor should compile and run");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_copy_ctor.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Struct with copy constructor should compile and run");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_exception.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_namespace.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_virtual_override.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Virtual method override should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_base_constructor.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Base class constructor delegation should work");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_operator_overloading.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_dynamic_dispatch.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Dynamic dispatch should correctly call derived class methods");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_function_returning_struct.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Function returning struct should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_multiple_inheritance.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Multiple inheritance should work correctly with access to both base classes");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_enum_class.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Enum class should work correctly with scoped access");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_static_members.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_lambda_basic.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Basic lambda expressions should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_lambda_captures.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_generic_lambda.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Generic lambdas with single type usage should work");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_range_for.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Range-based for loop should iterate over array");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_increment_decrement.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Increment/decrement operators should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_default_params.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Default function parameters should be evaluated correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_const_methods.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Const methods should use &self, non-const should use &mut self");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_switch.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Switch statements with fallthrough should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_comma_operator.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Comma operator should evaluate both expressions and return the last");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_typedef.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_global_var.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Global variables should work with unsafe access");
}
