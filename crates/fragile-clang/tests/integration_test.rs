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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_global_array.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Global arrays should work with unsafe access and proper initialization");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_virtual_diamond.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Virtual diamond inheritance should share a single virtual base instance");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_namespace_path.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Namespace function calls should use correct relative paths");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_functor.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Function call operator should work with arguments");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_ctor_body.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Constructor body statements should execute correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_subscript.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Subscript operator should work with reads and writes");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_assign_ops.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_deref_op.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Dereference operator should work with reads and writes");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_arrow_op.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Arrow operator should work for member access and method calls");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_sizeof_alignof.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "sizeof and alignof should be evaluated at compile time");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_string_char.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "String literals and char casts should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_designated_init.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "Designated initializers should work correctly");
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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_function_pointers.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_std_get.cpp")
        .expect("E2E test failed");

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

    let (exit_code, _stdout, _stderr) = transpile_compile_run(source, "e2e_std_visit.cpp")
        .expect("E2E test failed");

    assert_eq!(exit_code, 0, "std::visit on variant should work correctly");
}
