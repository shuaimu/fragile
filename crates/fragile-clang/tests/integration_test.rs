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

/// Test class with access specifiers.
#[test]
fn test_class_access_specifiers() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class MyClass {
        public:
            int public_field;
        private:
            int private_field;
        protected:
            int protected_field;
        };
    "#;

    let ast = parser.parse_string(source, "access.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.name, "MyClass");
    assert!(s.is_class);
    assert_eq!(s.fields.len(), 3);

    // Check access specifiers
    let field0 = &s.fields[0];
    assert_eq!(field0.name, "public_field");
    assert_eq!(field0.access, AccessSpecifier::Public);

    let field1 = &s.fields[1];
    assert_eq!(field1.name, "private_field");
    assert_eq!(field1.access, AccessSpecifier::Private);

    let field2 = &s.fields[2];
    assert_eq!(field2.name, "protected_field");
    assert_eq!(field2.access, AccessSpecifier::Protected);
}

/// Test struct default access (public).
#[test]
fn test_struct_default_access() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        struct MyStruct {
            int field1;
            int field2;
        };
    "#;

    let ast = parser.parse_string(source, "struct_access.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.name, "MyStruct");
    assert!(!s.is_class);

    // Struct members should be public by default
    for field in &s.fields {
        assert_eq!(field.access, AccessSpecifier::Public);
    }
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

/// Test using namespace directive conversion.
#[test]
fn test_using_namespace_conversion() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace foo {
            int x;
        }
        using namespace foo;
    "#;

    let ast = parser.parse_string(source, "using.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.using_directives.len(), 1);

    let using_dir = &module.using_directives[0];
    assert_eq!(using_dir.namespace, vec!["foo"]);
    assert!(using_dir.scope.is_empty()); // Global scope
}

/// Test using namespace in nested scope.
#[test]
fn test_using_namespace_in_scope() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace foo {
            int x;
        }
        namespace bar {
            using namespace foo;
        }
    "#;

    let ast = parser.parse_string(source, "using_scope.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.using_directives.len(), 1);

    let using_dir = &module.using_directives[0];
    assert_eq!(using_dir.namespace, vec!["foo"]);
    assert_eq!(using_dir.scope, vec!["bar"]); // Inside namespace bar
}

/// Test using nested namespace.
#[test]
fn test_using_nested_namespace_conversion() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace outer {
            namespace inner {
                int x;
            }
        }
        using namespace outer::inner;
    "#;

    let ast = parser.parse_string(source, "using_nested.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.using_directives.len(), 1);

    let using_dir = &module.using_directives[0];
    assert_eq!(using_dir.namespace, vec!["outer", "inner"]);
}

/// Test class with default constructor.
#[test]
fn test_default_constructor() {
    use fragile_clang::{AccessSpecifier, ConstructorKind};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class MyClass {
        public:
            int x;
            MyClass() { x = 0; }
        };
    "#;

    let ast = parser.parse_string(source, "ctor.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.name, "MyClass");
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.kind, ConstructorKind::Default);
    assert_eq!(ctor.access, AccessSpecifier::Public);
    assert!(ctor.params.is_empty());
    assert!(ctor.mir_body.is_some()); // Has definition
}

/// Test class with copy constructor.
#[test]
fn test_copy_constructor() {
    use fragile_clang::{AccessSpecifier, ConstructorKind};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Copyable {
        public:
            int value;
            Copyable(const Copyable& other) { value = other.value; }
        };
    "#;

    let ast = parser.parse_string(source, "copy_ctor.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.kind, ConstructorKind::Copy);
    assert_eq!(ctor.access, AccessSpecifier::Public);
    assert_eq!(ctor.params.len(), 1); // const Copyable& other
}

/// Test class with move constructor.
#[test]
fn test_move_constructor() {
    use fragile_clang::{AccessSpecifier, ConstructorKind};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Movable {
        public:
            int value;
            Movable(Movable&& other) { value = other.value; }
        };
    "#;

    let ast = parser.parse_string(source, "move_ctor.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.kind, ConstructorKind::Move);
    assert_eq!(ctor.access, AccessSpecifier::Public);
    assert_eq!(ctor.params.len(), 1); // Movable&& other
}

/// Test class with parameterized constructor.
#[test]
fn test_parameterized_constructor() {
    use fragile_clang::{AccessSpecifier, ConstructorKind};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Point {
        public:
            int x;
            int y;
            Point(int a, int b) { x = a; y = b; }
        };
    "#;

    let ast = parser.parse_string(source, "param_ctor.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.kind, ConstructorKind::Other); // Parameterized
    assert_eq!(ctor.access, AccessSpecifier::Public);
    assert_eq!(ctor.params.len(), 2);
    assert_eq!(ctor.params[0].0, "a");
    assert_eq!(ctor.params[1].0, "b");
}

/// Test class with multiple constructors.
#[test]
fn test_multiple_constructors() {
    use fragile_clang::ConstructorKind;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class MultiCtor {
        public:
            int value;
            MultiCtor() { value = 0; }
            MultiCtor(int v) { value = v; }
            MultiCtor(const MultiCtor& other) { value = other.value; }
        };
    "#;

    let ast = parser.parse_string(source, "multi_ctor.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 3);

    // Check we have default, parameterized, and copy constructors
    let kinds: Vec<_> = s.constructors.iter().map(|c| c.kind).collect();
    assert!(kinds.contains(&ConstructorKind::Default));
    assert!(kinds.contains(&ConstructorKind::Copy));
    assert!(kinds.contains(&ConstructorKind::Other)); // Parameterized
}

/// Test class with destructor.
#[test]
fn test_destructor() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class WithDestructor {
        public:
            int* ptr;
            ~WithDestructor() { }
        };
    "#;

    let ast = parser.parse_string(source, "dtor.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert!(s.destructor.is_some());

    let dtor = s.destructor.as_ref().unwrap();
    assert_eq!(dtor.access, AccessSpecifier::Public);
    assert!(dtor.mir_body.is_some()); // Has definition
}

/// Test class with private constructor.
#[test]
fn test_private_constructor() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Singleton {
        private:
            Singleton() {}
        public:
            int value;
        };
    "#;

    let ast = parser.parse_string(source, "singleton.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.access, AccessSpecifier::Private);
}

/// Test class with member initializer list.
#[test]
fn test_member_initializer_list() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Point {
        public:
            int x;
            int y;
            Point(int a, int b) : x(a), y(b) { }
        };
    "#;

    let ast = parser.parse_string(source, "init_list.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.member_initializers.len(), 2);

    // Check the member names
    let init_names: Vec<_> = ctor.member_initializers.iter().map(|i| &i.member_name).collect();
    assert!(init_names.contains(&&"x".to_string()));
    assert!(init_names.contains(&&"y".to_string()));
}

/// Test class with single member initializer.
#[test]
fn test_single_member_initializer() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Counter {
        public:
            int count;
            Counter() : count(0) { }
        };
    "#;

    let ast = parser.parse_string(source, "single_init.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    assert_eq!(ctor.member_initializers.len(), 1);
    assert_eq!(ctor.member_initializers[0].member_name, "count");
    assert!(ctor.member_initializers[0].has_init);
}

/// Test constructor without member initializer list.
#[test]
fn test_no_member_initializer() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Empty {
        public:
            int x;
            Empty() { x = 0; }
        };
    "#;

    let ast = parser.parse_string(source, "no_init.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.constructors.len(), 1);

    let ctor = &s.constructors[0];
    // No member initializers - assignment happens in body
    assert_eq!(ctor.member_initializers.len(), 0);
}

/// Test class with static member variable.
#[test]
fn test_static_member_variable() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Counter {
        public:
            static int count;
            int value;
        };
    "#;

    let ast = parser.parse_string(source, "static_var.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    // Non-static fields
    assert_eq!(s.fields.len(), 1);
    assert_eq!(s.fields[0].name, "value");

    // Static fields
    assert_eq!(s.static_fields.len(), 1);
    assert_eq!(s.static_fields[0].name, "count");
}

/// Test class with static method.
#[test]
fn test_static_method() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Counter {
        public:
            int value;
            static int getZero() { return 0; }
            int getValue() { return value; }
        };
    "#;

    let ast = parser.parse_string(source, "static_method.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.methods.len(), 2);

    // Find static method
    let static_method = s.methods.iter().find(|m| m.name == "getZero");
    assert!(static_method.is_some());
    assert!(static_method.unwrap().is_static);

    // Find non-static method
    let instance_method = s.methods.iter().find(|m| m.name == "getValue");
    assert!(instance_method.is_some());
    assert!(!instance_method.unwrap().is_static);
}

/// Test class with mix of static and non-static members.
#[test]
fn test_mixed_static_members() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class MixedClass {
        public:
            static int static_count;
            int instance_value;
            static void staticMethod() { }
            void instanceMethod() { }
        private:
            static int private_static;
        };
    "#;

    let ast = parser.parse_string(source, "mixed.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];

    // Check instance fields
    assert_eq!(s.fields.len(), 1);
    assert_eq!(s.fields[0].name, "instance_value");

    // Check static fields (public and private)
    assert_eq!(s.static_fields.len(), 2);
    let static_names: Vec<_> = s.static_fields.iter().map(|f| &f.name).collect();
    assert!(static_names.contains(&&"static_count".to_string()));
    assert!(static_names.contains(&&"private_static".to_string()));

    // Check methods
    assert_eq!(s.methods.len(), 2);
    let static_method_count = s.methods.iter().filter(|m| m.is_static).count();
    let instance_method_count = s.methods.iter().filter(|m| !m.is_static).count();
    assert_eq!(static_method_count, 1);
    assert_eq!(instance_method_count, 1);
}

/// Test friend class declaration.
#[test]
fn test_friend_class() {
    use fragile_clang::CppFriend;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Foo {
            friend class Bar;
        private:
            int value;
        };
    "#;

    let ast = parser.parse_string(source, "friend_class.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.friends.len(), 1);

    match &s.friends[0] {
        CppFriend::Class { name } => assert_eq!(name, "Bar"),
        _ => panic!("Expected friend class, got function"),
    }
}

/// Test friend function declaration.
#[test]
fn test_friend_function() {
    use fragile_clang::CppFriend;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Foo {
            friend void helper(Foo& f);
        private:
            int value;
        };
    "#;

    let ast = parser.parse_string(source, "friend_func.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.friends.len(), 1);

    match &s.friends[0] {
        CppFriend::Function { name } => assert_eq!(name, "helper"),
        _ => panic!("Expected friend function, got class"),
    }
}

/// Test multiple friend declarations.
#[test]
fn test_multiple_friends() {
    use fragile_clang::CppFriend;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Foo {
            friend class Bar;
            friend class Baz;
            friend void helper(Foo& f);
        private:
            int value;
        };
    "#;

    let ast = parser.parse_string(source, "multi_friend.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.friends.len(), 3);

    // Count friend classes and functions
    let class_count = s.friends.iter().filter(|f| matches!(f, CppFriend::Class { .. })).count();
    let func_count = s.friends.iter().filter(|f| matches!(f, CppFriend::Function { .. })).count();

    assert_eq!(class_count, 2);
    assert_eq!(func_count, 1);
}

/// Test public single inheritance.
#[test]
fn test_public_inheritance() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            int base_value;
        };

        class Derived : public Base {
        public:
            int derived_value;
        };
    "#;

    let ast = parser.parse_string(source, "public_inherit.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 2);

    // Find Derived class
    let derived = module.structs.iter().find(|s| s.name == "Derived");
    assert!(derived.is_some(), "Expected Derived class");
    let derived = derived.unwrap();

    assert_eq!(derived.bases.len(), 1);
    assert_eq!(derived.bases[0].access, AccessSpecifier::Public);
    assert!(!derived.bases[0].is_virtual);
}

/// Test protected inheritance.
#[test]
fn test_protected_inheritance() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            int value;
        };

        class Derived : protected Base {
        public:
            int derived_value;
        };
    "#;

    let ast = parser.parse_string(source, "protected_inherit.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let derived = module.structs.iter().find(|s| s.name == "Derived").unwrap();
    assert_eq!(derived.bases.len(), 1);
    assert_eq!(derived.bases[0].access, AccessSpecifier::Protected);
}

/// Test private inheritance.
#[test]
fn test_private_inheritance() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            int value;
        };

        class Derived : private Base {
        public:
            int derived_value;
        };
    "#;

    let ast = parser.parse_string(source, "private_inherit.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let derived = module.structs.iter().find(|s| s.name == "Derived").unwrap();
    assert_eq!(derived.bases.len(), 1);
    assert_eq!(derived.bases[0].access, AccessSpecifier::Private);
}

/// Test virtual inheritance.
#[test]
fn test_virtual_inheritance() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            int value;
        };

        class Derived : virtual public Base {
        public:
            int derived_value;
        };
    "#;

    let ast = parser.parse_string(source, "virtual_inherit.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let derived = module.structs.iter().find(|s| s.name == "Derived").unwrap();
    assert_eq!(derived.bases.len(), 1);
    assert_eq!(derived.bases[0].access, AccessSpecifier::Public);
    assert!(derived.bases[0].is_virtual);
}
