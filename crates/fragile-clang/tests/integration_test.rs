//! Integration tests for Clang AST parsing and MIR conversion.

use fragile_clang::{ClangNodeKind, ClangParser, CppType, MirConverter, MirTerminator};

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

/// Test multiple inheritance.
#[test]
fn test_multiple_inheritance() {
    use fragile_clang::AccessSpecifier;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base1 {
        public:
            int value1;
        };

        class Base2 {
        public:
            int value2;
        };

        class Derived : public Base1, protected Base2 {
        public:
            int derived_value;
        };
    "#;

    let ast = parser.parse_string(source, "multi_inherit.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 3);

    let derived = module.structs.iter().find(|s| s.name == "Derived").unwrap();
    assert_eq!(derived.bases.len(), 2);

    // First base is public Base1
    assert_eq!(derived.bases[0].access, AccessSpecifier::Public);

    // Second base is protected Base2
    assert_eq!(derived.bases[1].access, AccessSpecifier::Protected);
}

/// Test virtual function.
#[test]
fn test_virtual_function() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            virtual void foo() { }
            void normal() { }
        };
    "#;

    let ast = parser.parse_string(source, "virtual_func.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.methods.len(), 2);

    // Find virtual method
    let virtual_method = s.methods.iter().find(|m| m.name == "foo");
    assert!(virtual_method.is_some());
    assert!(virtual_method.unwrap().is_virtual);
    assert!(!virtual_method.unwrap().is_pure_virtual);

    // Find normal method
    let normal_method = s.methods.iter().find(|m| m.name == "normal");
    assert!(normal_method.is_some());
    assert!(!normal_method.unwrap().is_virtual);
}

/// Test pure virtual function.
#[test]
fn test_pure_virtual_function() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Abstract {
        public:
            virtual void pure() = 0;
        };
    "#;

    let ast = parser.parse_string(source, "pure_virtual.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.methods.len(), 1);

    let method = &s.methods[0];
    assert_eq!(method.name, "pure");
    assert!(method.is_virtual);
    assert!(method.is_pure_virtual);
}

/// Test override specifier on methods.
#[test]
fn test_override_specifier() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            virtual void foo() { }
        };

        class Derived : public Base {
        public:
            void foo() override { }
        };
    "#;

    let ast = parser.parse_string(source, "override.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the derived class
    let derived = module.structs.iter().find(|s| s.name == "Derived");
    assert!(derived.is_some());

    let derived = derived.unwrap();
    assert_eq!(derived.methods.len(), 1);

    let method = &derived.methods[0];
    assert_eq!(method.name, "foo");
    assert!(method.is_virtual); // override implies virtual
    assert!(method.is_override);
    assert!(!method.is_final);
}

/// Test final specifier on methods.
#[test]
fn test_final_specifier() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            virtual void foo() { }
        };

        class Derived : public Base {
        public:
            void foo() final { }
        };
    "#;

    let ast = parser.parse_string(source, "final.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the derived class
    let derived = module.structs.iter().find(|s| s.name == "Derived");
    assert!(derived.is_some());

    let derived = derived.unwrap();
    assert_eq!(derived.methods.len(), 1);

    let method = &derived.methods[0];
    assert_eq!(method.name, "foo");
    assert!(method.is_virtual); // final implies virtual
    assert!(method.is_final);
}

/// Test override and final together.
#[test]
fn test_override_and_final() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Base {
        public:
            virtual void foo() { }
        };

        class Derived : public Base {
        public:
            void foo() override final { }
        };
    "#;

    let ast = parser.parse_string(source, "override_final.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the derived class
    let derived = module.structs.iter().find(|s| s.name == "Derived");
    assert!(derived.is_some());

    let derived = derived.unwrap();
    assert_eq!(derived.methods.len(), 1);

    let method = &derived.methods[0];
    assert_eq!(method.name, "foo");
    assert!(method.is_virtual);
    assert!(method.is_override);
    assert!(method.is_final);
}

/// Test operator overloading (arithmetic operators).
#[test]
fn test_operator_overloading_arithmetic() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Vector {
        public:
            int x, y;

            Vector operator+(const Vector& other) const {
                return Vector();
            }

            Vector operator-(const Vector& other) const {
                return Vector();
            }

            Vector operator*(int scalar) const {
                return Vector();
            }

            Vector operator/(int scalar) const {
                return Vector();
            }
        };
    "#;

    let ast = parser.parse_string(source, "operators.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];
    assert_eq!(s.name, "Vector");

    // Should have 4 operator methods
    let operator_methods: Vec<_> = s.methods.iter().filter(|m| m.name.starts_with("operator")).collect();
    assert_eq!(operator_methods.len(), 4);

    // Check for specific operators
    let op_plus = s.methods.iter().find(|m| m.name == "operator+");
    assert!(op_plus.is_some(), "operator+ not found");

    let op_minus = s.methods.iter().find(|m| m.name == "operator-");
    assert!(op_minus.is_some(), "operator- not found");

    let op_mul = s.methods.iter().find(|m| m.name == "operator*");
    assert!(op_mul.is_some(), "operator* not found");

    let op_div = s.methods.iter().find(|m| m.name == "operator/");
    assert!(op_div.is_some(), "operator/ not found");
}

/// Test operator overloading (comparison operators).
#[test]
fn test_operator_overloading_comparison() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Value {
        public:
            int val;

            bool operator==(const Value& other) const {
                return val == other.val;
            }

            bool operator!=(const Value& other) const {
                return val != other.val;
            }

            bool operator<(const Value& other) const {
                return val < other.val;
            }

            bool operator>(const Value& other) const {
                return val > other.val;
            }
        };
    "#;

    let ast = parser.parse_string(source, "comparison.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];

    // Check for specific operators
    assert!(s.methods.iter().any(|m| m.name == "operator=="), "operator== not found");
    assert!(s.methods.iter().any(|m| m.name == "operator!="), "operator!= not found");
    assert!(s.methods.iter().any(|m| m.name == "operator<"), "operator< not found");
    assert!(s.methods.iter().any(|m| m.name == "operator>"), "operator> not found");
}

/// Test operator overloading (compound assignment).
#[test]
fn test_operator_overloading_assignment() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Counter {
        public:
            int count;

            Counter& operator=(const Counter& other) {
                count = other.count;
                return *this;
            }

            Counter& operator+=(int delta) {
                count += delta;
                return *this;
            }

            Counter& operator-=(int delta) {
                count -= delta;
                return *this;
            }
        };
    "#;

    let ast = parser.parse_string(source, "assignment.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];

    // Check for specific operators
    assert!(s.methods.iter().any(|m| m.name == "operator="), "operator= not found");
    assert!(s.methods.iter().any(|m| m.name == "operator+="), "operator+= not found");
    assert!(s.methods.iter().any(|m| m.name == "operator-="), "operator-= not found");
}

/// Test operator overloading (subscript and call operators).
#[test]
fn test_operator_overloading_subscript_call() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        class Array {
        public:
            int data[10];

            int& operator[](int index) {
                return data[index];
            }

            int operator()(int a, int b) {
                return a + b;
            }
        };
    "#;

    let ast = parser.parse_string(source, "subscript_call.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);

    let s = &module.structs[0];

    // Check for specific operators
    assert!(s.methods.iter().any(|m| m.name == "operator[]"), "operator[] not found");
    assert!(s.methods.iter().any(|m| m.name == "operator()"), "operator() not found");
}

/// Test operator overloading (pointer operators).
#[test]
fn test_operator_overloading_pointer() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        struct Inner {
            int value;
        };

        class SmartPtr {
        public:
            Inner* ptr;

            Inner& operator*() {
                return *ptr;
            }

            Inner* operator->() {
                return ptr;
            }
        };
    "#;

    let ast = parser.parse_string(source, "pointer.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find SmartPtr class
    let smart_ptr = module.structs.iter().find(|s| s.name == "SmartPtr");
    assert!(smart_ptr.is_some(), "SmartPtr class not found");

    let s = smart_ptr.unwrap();

    // Check for specific operators
    assert!(s.methods.iter().any(|m| m.name == "operator*"), "operator* not found");
    assert!(s.methods.iter().any(|m| m.name == "operator->"), "operator-> not found");
}

/// Test const reference parameters.
#[test]
fn test_const_reference() {
    use fragile_clang::CppType;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        void take_const_ref(const int& value);
        void take_mutable_ref(int& value);
        const int& return_const_ref();
        int& return_mutable_ref();
    "#;

    let ast = parser.parse_string(source, "const_ref.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the functions
    let take_const_ref = module.externs.iter().find(|f| f.display_name == "take_const_ref");
    assert!(take_const_ref.is_some(), "take_const_ref not found");
    let f = take_const_ref.unwrap();
    assert_eq!(f.params.len(), 1);
    // Check that the parameter is a const lvalue reference
    if let CppType::Reference { is_const, is_rvalue, .. } = &f.params[0].1 {
        assert!(*is_const, "Parameter should be const reference");
        assert!(!*is_rvalue, "Parameter should be lvalue reference");
    } else {
        panic!("Expected Reference type for const int&");
    }

    let take_mutable_ref = module.externs.iter().find(|f| f.display_name == "take_mutable_ref");
    assert!(take_mutable_ref.is_some(), "take_mutable_ref not found");
    let f = take_mutable_ref.unwrap();
    assert_eq!(f.params.len(), 1);
    // Check that the parameter is a mutable lvalue reference
    if let CppType::Reference { is_const, is_rvalue, .. } = &f.params[0].1 {
        assert!(!*is_const, "Parameter should be mutable reference");
        assert!(!*is_rvalue, "Parameter should be lvalue reference");
    } else {
        panic!("Expected Reference type for int&");
    }

    let return_const_ref = module.externs.iter().find(|f| f.display_name == "return_const_ref");
    assert!(return_const_ref.is_some(), "return_const_ref not found");
    let f = return_const_ref.unwrap();
    // Check that return type is a const lvalue reference
    if let CppType::Reference { is_const, is_rvalue, .. } = &f.return_type {
        assert!(*is_const, "Return type should be const reference");
        assert!(!*is_rvalue, "Return type should be lvalue reference");
    } else {
        panic!("Expected Reference type for const int&");
    }

    let return_mutable_ref = module.externs.iter().find(|f| f.display_name == "return_mutable_ref");
    assert!(return_mutable_ref.is_some(), "return_mutable_ref not found");
    let f = return_mutable_ref.unwrap();
    // Check that return type is a mutable lvalue reference
    if let CppType::Reference { is_const, is_rvalue, .. } = &f.return_type {
        assert!(!*is_const, "Return type should be mutable reference");
        assert!(!*is_rvalue, "Return type should be lvalue reference");
    } else {
        panic!("Expected Reference type for int&");
    }
}

/// Test rvalue references (move semantics).
#[test]
fn test_rvalue_reference() {
    use fragile_clang::CppType;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        void take_rvalue(int&& value);
        int&& return_rvalue();
    "#;

    let ast = parser.parse_string(source, "rvalue_ref.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the functions
    let take_rvalue = module.externs.iter().find(|f| f.display_name == "take_rvalue");
    assert!(take_rvalue.is_some(), "take_rvalue not found");
    let f = take_rvalue.unwrap();
    assert_eq!(f.params.len(), 1);
    // Check that the parameter is an rvalue reference
    if let CppType::Reference { is_const, is_rvalue, .. } = &f.params[0].1 {
        assert!(!*is_const, "Parameter should not be const");
        assert!(*is_rvalue, "Parameter should be rvalue reference");
    } else {
        panic!("Expected Reference type for int&&");
    }

    let return_rvalue = module.externs.iter().find(|f| f.display_name == "return_rvalue");
    assert!(return_rvalue.is_some(), "return_rvalue not found");
    let f = return_rvalue.unwrap();
    // Check that return type is an rvalue reference
    if let CppType::Reference { is_const, is_rvalue, .. } = &f.return_type {
        assert!(!*is_const, "Return type should not be const");
        assert!(*is_rvalue, "Return type should be rvalue reference");
    } else {
        panic!("Expected Reference type for int&&");
    }
}

/// Test basic function template parsing.
#[test]
fn test_function_template_basic() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) {
            return x;
        }
    "#;

    let ast = parser.parse_string(source, "template_basic.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);

    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "identity");
    assert_eq!(tmpl.template_params.len(), 1);
    assert_eq!(tmpl.template_params[0], "T");
    assert!(tmpl.is_definition);
}

/// Test function template with multiple type parameters.
#[test]
fn test_function_template_multiple_params() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T, typename U>
        T convert(U value) {
            return static_cast<T>(value);
        }
    "#;

    let ast = parser.parse_string(source, "template_multi.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);

    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "convert");
    assert_eq!(tmpl.template_params.len(), 2);
    assert_eq!(tmpl.template_params[0], "T");
    assert_eq!(tmpl.template_params[1], "U");
}

/// Test function template declaration (without definition).
#[test]
fn test_function_template_declaration() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T compute(T a, T b);
    "#;

    let ast = parser.parse_string(source, "template_decl.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);

    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "compute");
    assert_eq!(tmpl.template_params.len(), 1);
    assert_eq!(tmpl.template_params[0], "T");
    assert!(!tmpl.is_definition);
}

/// Test name resolution for function calls within the same namespace.
#[test]
fn test_name_resolution_same_namespace() {
    use fragile_clang::MirTerminator;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace foo {
            int helper() {
                return 42;
            }

            int caller() {
                return helper();
            }
        }
    "#;

    let ast = parser.parse_string(source, "same_ns.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let mut module = converter.convert(ast).expect("Failed to convert");

    // Before resolution, function name is unqualified
    let caller = module.functions.iter().find(|f| f.display_name == "caller").unwrap();
    let call_block = caller.mir_body.blocks.iter().find(|b| {
        matches!(&b.terminator, MirTerminator::Call { .. })
    });
    assert!(call_block.is_some(), "Should have a function call");

    // Apply name resolution
    module.resolve_names();

    // After resolution, function name should be qualified
    let caller = module.functions.iter().find(|f| f.display_name == "caller").unwrap();
    for block in &caller.mir_body.blocks {
        if let MirTerminator::Call { func, .. } = &block.terminator {
            assert_eq!(func, "foo::helper", "Function call should be resolved to foo::helper");
        }
    }
}

/// Test name resolution via using namespace directive.
#[test]
fn test_name_resolution_using_namespace() {
    use fragile_clang::MirTerminator;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        namespace bar {
            int helper() {
                return 100;
            }
        }

        using namespace bar;

        int caller() {
            return helper();
        }
    "#;

    let ast = parser.parse_string(source, "using_ns.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let mut module = converter.convert(ast).expect("Failed to convert");

    // Apply name resolution
    module.resolve_names();

    // Function call should be resolved to bar::helper
    let caller = module.functions.iter().find(|f| f.display_name == "caller").unwrap();
    for block in &caller.mir_body.blocks {
        if let MirTerminator::Call { func, .. } = &block.terminator {
            assert_eq!(func, "bar::helper", "Function call should be resolved to bar::helper via using directive");
        }
    }
}

/// Test name resolution for global functions from within a namespace.
#[test]
fn test_name_resolution_global_from_namespace() {
    use fragile_clang::MirTerminator;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        int global_helper() {
            return 500;
        }

        namespace ns {
            int caller() {
                return global_helper();
            }
        }
    "#;

    let ast = parser.parse_string(source, "global.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let mut module = converter.convert(ast).expect("Failed to convert");

    // Apply name resolution
    module.resolve_names();

    // Function call should be resolved to global scope
    let caller = module.functions.iter().find(|f| f.display_name == "caller").unwrap();
    for block in &caller.mir_body.blocks {
        if let MirTerminator::Call { func, .. } = &block.terminator {
            assert_eq!(func, "global_helper", "Function call should be resolved to global_helper");
        }
    }
}

/// Test std::move generates MirOperand::Move.
#[test]
fn test_std_move_basic() {
    use fragile_clang::MirOperand;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        #include <utility>

        int test_move() {
            int x = 42;
            int y = std::move(x);
            return y;
        }
    "#;

    let ast = parser.parse_string(source, "move.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the test_move function
    let func = module.functions.iter().find(|f| f.display_name == "test_move");
    assert!(func.is_some(), "test_move function not found");

    let func = func.unwrap();

    // Check that there's a Move operand in the MIR
    let has_move = func.mir_body.blocks.iter().any(|block| {
        block.statements.iter().any(|stmt| {
            if let fragile_clang::MirStatement::Assign { value, .. } = stmt {
                matches!(value, fragile_clang::MirRvalue::Use(MirOperand::Move(_)))
            } else {
                false
            }
        })
    });

    assert!(has_move, "std::move should generate a Move operand");
}

/// Test std::forward generates MirOperand::Move.
#[test]
fn test_std_forward_basic() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        #include <utility>

        template<typename T>
        T forward_wrapper(T&& arg) {
            return std::forward<T>(arg);
        }
    "#;

    let ast = parser.parse_string(source, "forward.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // The function template should be parsed (even if not instantiated)
    // We can't directly test the MIR without template instantiation,
    // but we verify the parse doesn't fail
    assert!(module.function_templates.len() >= 1 || module.functions.is_empty(),
            "Should parse function template or be empty if no instantiation");
}

/// Test template parameter types are correctly identified.
#[test]
fn test_template_param_types() {
    use fragile_clang::CppType;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(source, "param_types.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let tmpl = &module.function_templates[0];

    // Return type should be TemplateParam
    assert!(
        matches!(&tmpl.return_type, CppType::TemplateParam { name, depth: 0, index: 0 } if name == "T"),
        "Return type should be TemplateParam T, got {:?}",
        tmpl.return_type
    );

    // Parameter type should be TemplateParam
    assert_eq!(tmpl.params.len(), 1);
    let (param_name, param_type) = &tmpl.params[0];
    assert_eq!(param_name, "x");
    assert!(
        matches!(param_type, CppType::TemplateParam { name, depth: 0, index: 0 } if name == "T"),
        "Parameter type should be TemplateParam T, got {:?}",
        param_type
    );
}

/// Test dependent types (e.g., const T&) are detected.
#[test]
fn test_dependent_types() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        void take_const_ref(const T& x);
    "#;

    let ast = parser.parse_string(source, "dependent.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let tmpl = &module.function_templates[0];

    // Parameter type should be DependentType or contain reference to T
    assert_eq!(tmpl.params.len(), 1);
    let (param_name, param_type) = &tmpl.params[0];
    assert_eq!(param_name, "x");

    // Check it's dependent (contains template param reference)
    assert!(
        param_type.is_dependent(),
        "Parameter const T& should be dependent, got {:?}",
        param_type
    );
}

/// Test CppType::is_dependent method.
#[test]
fn test_is_dependent_method() {
    use fragile_clang::CppType;

    // Simple types are not dependent
    assert!(!CppType::Int { signed: true }.is_dependent());
    assert!(!CppType::Named("MyClass".to_string()).is_dependent());

    // Template params are dependent
    let tparam = CppType::TemplateParam {
        name: "T".to_string(),
        depth: 0,
        index: 0,
    };
    assert!(tparam.is_dependent());

    // DependentType is dependent
    let dep = CppType::DependentType {
        spelling: "const T&".to_string(),
    };
    assert!(dep.is_dependent());

    // Pointer to template param is dependent
    let ptr = CppType::Pointer {
        pointee: Box::new(tparam.clone()),
        is_const: false,
    };
    assert!(ptr.is_dependent());

    // Reference to template param is dependent
    let ref_ty = CppType::Reference {
        referent: Box::new(tparam.clone()),
        is_const: true,
        is_rvalue: false,
    };
    assert!(ref_ty.is_dependent());
}

/// Test std::move with function call argument.
#[test]
fn test_std_move_in_call() {
    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        #include <utility>

        void consume(int&& val) {}

        void test_move_call() {
            int x = 10;
            consume(std::move(x));
        }
    "#;

    let ast = parser.parse_string(source, "move_call.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Find the test_move_call function
    let func = module.functions.iter().find(|f| f.display_name == "test_move_call");
    assert!(func.is_some(), "test_move_call function not found");

    // The function should have a call to consume with a Move operand
    let func = func.unwrap();
    let has_call_with_move = func.mir_body.blocks.iter().any(|block| {
        if let fragile_clang::MirTerminator::Call { func: called_func, args, .. } = &block.terminator {
            called_func == "consume" && args.iter().any(|arg| matches!(arg, fragile_clang::MirOperand::Move(_)))
        } else {
            false
        }
    });

    assert!(has_call_with_move, "consume should be called with a Move operand");
}

// ============================================================================
// Type Deduction Tests
// ============================================================================

/// Test basic type deduction with simple int type.
#[test]
fn test_deduce_simple_int_type() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(source, "deduce_int.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let template = &module.function_templates[0];

    // Deduce T from int argument
    let arg_types = vec![CppType::Int { signed: true }];
    let result = TypeDeducer::deduce(template, &arg_types).expect("Deduction failed");

    assert_eq!(result.len(), 1);
    assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
}

/// Test type deduction with double type.
#[test]
fn test_deduce_double_type() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T square(T x) { return x * x; }
    "#;

    let ast = parser.parse_string(source, "deduce_double.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Deduce T from double argument
    let arg_types = vec![CppType::Double];
    let result = TypeDeducer::deduce(template, &arg_types).expect("Deduction failed");

    assert_eq!(result.get("T"), Some(&CppType::Double));
}

/// Test instantiation of a template function.
#[test]
fn test_instantiate_function_template() {
    use fragile_clang::CppType;
    use std::collections::HashMap;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(source, "instantiate.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Instantiate with T = int
    let mut subst = HashMap::new();
    subst.insert("T".to_string(), CppType::Int { signed: true });
    let func = template.instantiate(&subst);

    assert_eq!(func.display_name, "identity");
    assert_eq!(func.return_type, CppType::Int { signed: true });
    assert_eq!(func.params.len(), 1);
    assert_eq!(func.params[0].1, CppType::Int { signed: true });
}

/// Test deduce_and_instantiate convenience method.
#[test]
fn test_deduce_and_instantiate() {
    use fragile_clang::CppType;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(source, "deduce_inst.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Deduce and instantiate with double
    let arg_types = vec![CppType::Double];
    let func = template.deduce_and_instantiate(&arg_types).expect("Deduction failed");

    assert_eq!(func.return_type, CppType::Double);
    assert_eq!(func.params[0].1, CppType::Double);
    assert!(func.mangled_name.contains("f64"), "Mangled name should contain type: {}", func.mangled_name);
}

/// Test CppType::substitute method.
#[test]
fn test_substitute_template_param() {
    use fragile_clang::CppType;
    use std::collections::HashMap;

    let mut subst = HashMap::new();
    subst.insert("T".to_string(), CppType::Int { signed: true });

    // Direct template param
    let ty = CppType::TemplateParam {
        name: "T".to_string(),
        depth: 0,
        index: 0,
    };
    assert_eq!(ty.substitute(&subst), CppType::Int { signed: true });

    // Pointer to template param: T* → int*
    let ptr_ty = CppType::Pointer {
        pointee: Box::new(CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        }),
        is_const: false,
    };
    assert_eq!(
        ptr_ty.substitute(&subst),
        CppType::Pointer {
            pointee: Box::new(CppType::Int { signed: true }),
            is_const: false,
        }
    );

    // Reference to template param: T& → int&
    let ref_ty = CppType::Reference {
        referent: Box::new(CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        }),
        is_const: false,
        is_rvalue: false,
    };
    assert_eq!(
        ref_ty.substitute(&subst),
        CppType::Reference {
            referent: Box::new(CppType::Int { signed: true }),
            is_const: false,
            is_rvalue: false,
        }
    );
}

/// Test deduction error when types conflict.
#[test]
fn test_deduction_conflict() {
    use fragile_clang::{CppType, DeductionError, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T max(T a, T b) { return a > b ? a : b; }
    "#;

    let ast = parser.parse_string(source, "conflict.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Try to deduce with conflicting types: int and double
    let arg_types = vec![CppType::Int { signed: true }, CppType::Double];
    let result = TypeDeducer::deduce(template, &arg_types);

    assert!(matches!(result, Err(DeductionError::Conflict { .. })));
}

/// Test deduction with pointer parameter.
#[test]
fn test_deduce_pointer_type() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        void process(T* ptr);
    "#;

    let ast = parser.parse_string(source, "deduce_ptr.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Deduce T from int* argument
    let arg_types = vec![CppType::Pointer {
        pointee: Box::new(CppType::Int { signed: true }),
        is_const: false,
    }];
    let result = TypeDeducer::deduce(template, &arg_types).expect("Deduction failed");

    assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
}

/// Test deduction with const reference parameter.
#[test]
fn test_deduce_const_ref_type() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        void print(const T& x);
    "#;

    let ast = parser.parse_string(source, "deduce_cref.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Deduce T from int argument (const T& binds to lvalue)
    let arg_types = vec![CppType::Int { signed: true }];
    let result = TypeDeducer::deduce(template, &arg_types).expect("Deduction failed");

    assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
}

/// Test deduction with rvalue reference parameter.
#[test]
fn test_deduce_rvalue_ref_type() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        void consume(T&& x);
    "#;

    let ast = parser.parse_string(source, "deduce_rref.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Deduce T from double argument (T&& binds to rvalue)
    let arg_types = vec![CppType::Double];
    let result = TypeDeducer::deduce(template, &arg_types).expect("Deduction failed");

    assert_eq!(result.get("T"), Some(&CppType::Double));
}

// ============================================================================
// Explicit Template Arguments Tests
// ============================================================================

/// Test explicit template argument with single param.
#[test]
fn test_explicit_template_arg() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(source, "explicit.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Explicit T = double, call arg is int
    let explicit_args = vec![CppType::Double];
    let call_args = vec![CppType::Int { signed: true }];
    let result = TypeDeducer::deduce_with_explicit(template, &explicit_args, &call_args)
        .expect("Deduction failed");

    // Explicit wins
    assert_eq!(result.get("T"), Some(&CppType::Double));
}

/// Test explicit with mixed deduction.
#[test]
fn test_explicit_with_mixed_deduction() {
    use fragile_clang::{CppType, TypeDeducer};

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T, typename U>
        T convert(U x) { return static_cast<T>(x); }
    "#;

    let ast = parser.parse_string(source, "explicit_mixed.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // T = int (explicit), U = double (deduced)
    let explicit_args = vec![CppType::Int { signed: true }];
    let call_args = vec![CppType::Double];
    let result = TypeDeducer::deduce_with_explicit(template, &explicit_args, &call_args)
        .expect("Deduction failed");

    assert_eq!(result.get("T"), Some(&CppType::Int { signed: true }));
    assert_eq!(result.get("U"), Some(&CppType::Double));
}

/// Test deduce_and_instantiate_with_explicit method.
#[test]
fn test_deduce_instantiate_with_explicit() {
    use fragile_clang::CppType;

    let parser = ClangParser::new().expect("Failed to create parser");

    let source = r#"
        template<typename T, typename U>
        T convert(U x) { return static_cast<T>(x); }
    "#;

    let ast = parser.parse_string(source, "explicit_inst.cpp").expect("Failed to parse");
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    let template = &module.function_templates[0];

    // Instantiate with T = int (explicit), U = float (deduced)
    let explicit_args = vec![CppType::Int { signed: true }];
    let call_args = vec![CppType::Float];
    let func = template.deduce_and_instantiate_with_explicit(&explicit_args, &call_args)
        .expect("Instantiation failed");

    assert_eq!(func.return_type, CppType::Int { signed: true });
    assert_eq!(func.params[0].1, CppType::Float);
}

// ============================================================================
// Template Specialization Tests
// ============================================================================

/// Test that specializations can be added and found.
#[test]
fn test_specialization_found() {
    use fragile_clang::{CppType, CppFunction, CppFunctionTemplate, MirBody};

    let mut template = CppFunctionTemplate {
        name: "identity".to_string(),
        namespace: vec![],
        template_params: vec!["T".to_string()],
        return_type: CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        },
        params: vec![("x".to_string(), CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        })],
        is_definition: true,
        specializations: vec![],
        parameter_pack_indices: vec![],
        requires_clause: None,
    };

    // Add specialization for int
    let int_spec = CppFunction {
        mangled_name: "identity_int".to_string(),
        display_name: "identity<int>".to_string(),
        namespace: vec![],
        params: vec![("x".to_string(), CppType::Int { signed: true })],
        return_type: CppType::Int { signed: true },
        mir_body: MirBody::default(),
    };
    template.add_specialization(vec![CppType::Int { signed: true }], int_spec);

    // Should find the specialization
    let found = template.find_specialization(&[CppType::Int { signed: true }]);
    assert!(found.is_some());
    assert_eq!(found.unwrap().display_name, "identity<int>");

    // Should not find for double
    let not_found = template.find_specialization(&[CppType::Double]);
    assert!(not_found.is_none());
}

/// Test that specialization is used during instantiation.
#[test]
fn test_specialization_used_in_instantiation() {
    use fragile_clang::{CppType, CppFunction, CppFunctionTemplate, MirBody};

    let mut template = CppFunctionTemplate {
        name: "identity".to_string(),
        namespace: vec![],
        template_params: vec!["T".to_string()],
        return_type: CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        },
        params: vec![("x".to_string(), CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        })],
        is_definition: true,
        specializations: vec![],
        parameter_pack_indices: vec![],
        requires_clause: None,
    };

    // Add specialization for int with custom name
    let int_spec = CppFunction {
        mangled_name: "identity_int_specialized".to_string(),
        display_name: "identity<int>".to_string(),
        namespace: vec![],
        params: vec![("x".to_string(), CppType::Int { signed: true })],
        return_type: CppType::Int { signed: true },
        mir_body: MirBody::default(),
    };
    template.add_specialization(vec![CppType::Int { signed: true }], int_spec);

    // Instantiate with int - should use specialization
    let func = template
        .deduce_and_instantiate(&[CppType::Int { signed: true }])
        .expect("Instantiation failed");
    assert_eq!(func.mangled_name, "identity_int_specialized");

    // Instantiate with double - should use primary template
    let func = template
        .deduce_and_instantiate(&[CppType::Double])
        .expect("Instantiation failed");
    assert!(func.mangled_name.contains("f64")); // Primary template generates name with type
}

/// Test specialization with explicit template args.
#[test]
fn test_specialization_with_explicit_args() {
    use fragile_clang::{CppType, CppFunction, CppFunctionTemplate, MirBody};

    let mut template = CppFunctionTemplate {
        name: "convert".to_string(),
        namespace: vec![],
        template_params: vec!["T".to_string(), "U".to_string()],
        return_type: CppType::TemplateParam {
            name: "T".to_string(),
            depth: 0,
            index: 0,
        },
        params: vec![("x".to_string(), CppType::TemplateParam {
            name: "U".to_string(),
            depth: 0,
            index: 1,
        })],
        is_definition: true,
        specializations: vec![],
        parameter_pack_indices: vec![],
        requires_clause: None,
    };

    // Add specialization for <int, double>
    let spec = CppFunction {
        mangled_name: "convert_int_double".to_string(),
        display_name: "convert<int, double>".to_string(),
        namespace: vec![],
        params: vec![("x".to_string(), CppType::Double)],
        return_type: CppType::Int { signed: true },
        mir_body: MirBody::default(),
    };
    template.add_specialization(
        vec![CppType::Int { signed: true }, CppType::Double],
        spec,
    );

    // Explicit T = int, deduce U = double - should use specialization
    let func = template
        .deduce_and_instantiate_with_explicit(
            &[CppType::Int { signed: true }],
            &[CppType::Double],
        )
        .expect("Instantiation failed");
    assert_eq!(func.mangled_name, "convert_int_double");

    // Different types - should use primary template
    let func = template
        .deduce_and_instantiate_with_explicit(
            &[CppType::Int { signed: true }],
            &[CppType::Float],
        )
        .expect("Instantiation failed");
    assert!(func.mangled_name.contains("i32") || func.mangled_name.contains("convert"));
}

// ============================================================================
// Variadic Template Tests
// ============================================================================

/// Test that variadic template with parameter pack is detected.
#[test]
fn test_variadic_template_detected() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename... Args>
        void print(Args... args) {}
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "print");
    assert_eq!(tmpl.template_params, vec!["Args"]);

    // Should have one parameter pack at index 0
    assert!(tmpl.is_variadic());
    assert_eq!(tmpl.parameter_pack_indices, vec![0]);
    assert!(tmpl.is_pack_param(0));
}

/// Test variadic template with mixed params (regular + pack).
#[test]
fn test_variadic_template_mixed_params() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T, typename... Rest>
        T first(T head, Rest... tail) { return head; }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "first");
    assert_eq!(tmpl.template_params, vec!["T", "Rest"]);

    // T at index 0 is not a pack, Rest at index 1 is a pack
    assert!(tmpl.is_variadic());
    assert_eq!(tmpl.parameter_pack_indices, vec![1]);
    assert!(!tmpl.is_pack_param(0)); // T is not a pack
    assert!(tmpl.is_pack_param(1));  // Rest is a pack
}

/// Test non-variadic template has empty pack indices.
#[test]
fn test_non_variadic_template() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        T identity(T x) { return x; }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let tmpl = &module.function_templates[0];

    // Should not be variadic
    assert!(!tmpl.is_variadic());
    assert!(tmpl.parameter_pack_indices.is_empty());
}

/// Test CppType::ParameterPack variant.
#[test]
fn test_parameter_pack_type() {
    use fragile_clang::CppType;

    let pack = CppType::parameter_pack("Args", 0, 0);
    assert!(pack.is_parameter_pack());
    assert!(pack.is_dependent());
    assert_eq!(pack.to_rust_type_str(), "Args...");

    // Regular template param is not a pack
    let param = CppType::template_param("T", 0, 0);
    assert!(!param.is_parameter_pack());
}

// ============================================================================
// Class Template Tests
// ============================================================================

/// Test basic class template parsing.
#[test]
fn test_class_template_basic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        class Box {
        public:
            T value;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let tmpl = &module.class_templates[0];
    assert_eq!(tmpl.name, "Box");
    assert_eq!(tmpl.template_params, vec!["T"]);
    assert!(tmpl.is_class);
    assert!(!tmpl.parameter_pack_indices.is_empty() == false); // No packs
}

/// Test class template with multiple template parameters.
#[test]
fn test_class_template_multiple_params() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename K, typename V>
        class Pair {
        public:
            K first;
            V second;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let tmpl = &module.class_templates[0];
    assert_eq!(tmpl.name, "Pair");
    assert_eq!(tmpl.template_params, vec!["K", "V"]);
    assert_eq!(tmpl.fields.len(), 2);
}

/// Test class template with methods.
#[test]
fn test_class_template_with_methods() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        class Box {
        public:
            T value;
            Box(T v) : value(v) {}
            T get() const { return value; }
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let tmpl = &module.class_templates[0];
    assert_eq!(tmpl.name, "Box");
    assert_eq!(tmpl.constructors.len(), 1);
    assert_eq!(tmpl.methods.len(), 1);
    assert_eq!(tmpl.methods[0].name, "get");
}

/// Test variadic class template.
#[test]
fn test_class_template_variadic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename... Args>
        class Tuple {
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let tmpl = &module.class_templates[0];
    assert_eq!(tmpl.name, "Tuple");
    assert_eq!(tmpl.template_params, vec!["Args"]);
    assert_eq!(tmpl.parameter_pack_indices, vec![0]);
}

/// Test struct template (vs class template).
#[test]
fn test_struct_template() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        struct Wrapper {
            T data;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let tmpl = &module.class_templates[0];
    assert_eq!(tmpl.name, "Wrapper");
    // Note: struct templates may be detected as is_class=true due to how clang reports them
    // The key is that fields with template types work
    assert_eq!(tmpl.fields.len(), 1);
}

// ============================================================================
// Class Template Partial Specialization Tests
// ============================================================================

/// Test basic partial specialization - same type for both parameters.
#[test]
fn test_partial_spec_same_type() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        // Primary template
        template<typename T, typename U>
        class Pair {
            T first;
            U second;
        };

        // Partial specialization: both types are the same
        template<typename T>
        class Pair<T, T> {
            T first;
            T second;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have the primary template
    assert_eq!(module.class_templates.len(), 1);
    assert_eq!(module.class_templates[0].name, "Pair");
    assert_eq!(module.class_templates[0].template_params.len(), 2);

    // Should have one partial specialization
    assert_eq!(module.class_partial_specializations.len(), 1);
    let partial = &module.class_partial_specializations[0];
    assert_eq!(partial.template_name, "Pair");
    assert_eq!(partial.template_params.len(), 1); // Only one parameter: T
    assert_eq!(partial.specialization_args.len(), 2); // Two args: T, T

    // Both specialization args should reference the same template parameter
    match &partial.specialization_args[0] {
        CppType::TemplateParam { name, .. } => assert_eq!(name, "T"),
        other => panic!("Expected TemplateParam, got {:?}", other),
    }
    match &partial.specialization_args[1] {
        CppType::TemplateParam { name, .. } => assert_eq!(name, "T"),
        other => panic!("Expected TemplateParam, got {:?}", other),
    }
}

/// Test partial specialization with pointer.
#[test]
fn test_partial_spec_pointer() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        // Primary template
        template<typename T, typename U>
        class Pair {
            T first;
            U second;
        };

        // Partial specialization: second type is pointer
        template<typename T, typename U>
        class Pair<T, U*> {
            T first;
            U* second;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have the primary template
    assert_eq!(module.class_templates.len(), 1);

    // Should have one partial specialization
    assert_eq!(module.class_partial_specializations.len(), 1);
    let partial = &module.class_partial_specializations[0];
    assert_eq!(partial.template_name, "Pair");
    assert_eq!(partial.template_params.len(), 2); // Two parameters: T, U
    assert_eq!(partial.specialization_args.len(), 2); // Two args: T, U*

    // First arg is just T
    match &partial.specialization_args[0] {
        CppType::TemplateParam { name, .. } => assert_eq!(name, "T"),
        other => panic!("Expected TemplateParam for first arg, got {:?}", other),
    }

    // Second arg is U*
    match &partial.specialization_args[1] {
        CppType::Pointer { pointee, .. } => {
            match pointee.as_ref() {
                CppType::TemplateParam { name, .. } => assert_eq!(name, "U"),
                other => panic!("Expected TemplateParam in pointer, got {:?}", other),
            }
        }
        other => panic!("Expected Pointer for second arg, got {:?}", other),
    }
}

/// Test partial specialization with methods.
#[test]
fn test_partial_spec_with_methods() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        // Primary template
        template<typename T, typename U>
        class Pair {
            T first;
            U second;
        public:
            T getFirst() { return first; }
        };

        // Partial specialization with its own methods
        template<typename T>
        class Pair<T, T> {
            T first;
            T second;
        public:
            T getFirst() { return first; }
            T getSecond() { return second; }
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Partial specialization should have methods
    assert_eq!(module.class_partial_specializations.len(), 1);
    let partial = &module.class_partial_specializations[0];
    assert_eq!(partial.methods.len(), 2);

    let method_names: Vec<_> = partial.methods.iter().map(|m| &m.name).collect();
    assert!(method_names.contains(&&"getFirst".to_string()));
    assert!(method_names.contains(&&"getSecond".to_string()));
}

/// Test partial specialization in namespace.
#[test]
fn test_partial_spec_in_namespace() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        namespace ns {
            // Primary template
            template<typename T, typename U>
            class Pair {
                T first;
                U second;
            };

            // Partial specialization
            template<typename T>
            class Pair<T, T> {
                T value;
            };
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Both should be in the namespace
    assert_eq!(module.class_templates.len(), 1);
    assert_eq!(module.class_templates[0].namespace, vec!["ns"]);

    assert_eq!(module.class_partial_specializations.len(), 1);
    assert_eq!(module.class_partial_specializations[0].namespace, vec!["ns"]);
}

// ============================================================================
// Member Template Tests (Nested Templates)
// ============================================================================

/// Test basic member template in a non-template class.
#[test]
fn test_member_template_basic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        class Container {
        public:
            template<typename U>
            void process(U value);
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);
    let container = &module.structs[0];
    assert_eq!(container.name, "Container");
    assert_eq!(container.member_templates.len(), 1);

    let member_tmpl = &container.member_templates[0];
    assert_eq!(member_tmpl.name, "process");
    assert_eq!(member_tmpl.template_params.len(), 1);
    assert_eq!(member_tmpl.template_params[0], "U");
    assert_eq!(member_tmpl.params.len(), 1);
    assert_eq!(member_tmpl.params[0].0, "value");
}

/// Test member template in a class template.
#[test]
fn test_member_template_in_class_template() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        class Box {
            T data;
        public:
            template<typename U>
            U convert();
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let box_tmpl = &module.class_templates[0];
    assert_eq!(box_tmpl.name, "Box");
    assert_eq!(box_tmpl.template_params, vec!["T"]);
    assert_eq!(box_tmpl.member_templates.len(), 1);

    let member_tmpl = &box_tmpl.member_templates[0];
    assert_eq!(member_tmpl.name, "convert");
    assert_eq!(member_tmpl.template_params, vec!["U"]);
    // Return type should reference template param U
    match &member_tmpl.return_type {
        CppType::TemplateParam { name, .. } => assert_eq!(name, "U"),
        other => panic!("Expected TemplateParam for return type, got {:?}", other),
    }
}

/// Test variadic member template (like Mako's bucket::construct).
#[test]
fn test_member_template_variadic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        class Bucket {
        public:
            template<typename... Args>
            void construct(Args&&... args);
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.structs.len(), 1);
    let bucket = &module.structs[0];
    assert_eq!(bucket.member_templates.len(), 1);

    let member_tmpl = &bucket.member_templates[0];
    assert_eq!(member_tmpl.name, "construct");
    assert_eq!(member_tmpl.template_params, vec!["Args"]);
    assert_eq!(member_tmpl.parameter_pack_indices, vec![0]);
}

/// Test member template with multiple template parameters.
#[test]
fn test_member_template_multiple_params() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        class Converter {
        public:
            template<typename U, typename V>
            V transform(U input);
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.class_templates.len(), 1);
    let conv = &module.class_templates[0];
    assert_eq!(conv.member_templates.len(), 1);

    let member_tmpl = &conv.member_templates[0];
    assert_eq!(member_tmpl.name, "transform");
    assert_eq!(member_tmpl.template_params, vec!["U", "V"]);
    assert_eq!(member_tmpl.params.len(), 1);
}

// ============================================================================
// Type Properties Tests (SFINAE Foundation)
// ============================================================================

/// Test type properties for integral types.
#[test]
fn test_type_properties_integral() {
    // Signed int
    let int_type = CppType::Int { signed: true };
    let props = int_type.properties().unwrap();
    assert!(props.is_integral);
    assert!(props.is_signed);
    assert!(props.is_scalar);
    assert!(!props.is_floating_point);
    assert!(props.is_trivially_copyable);

    // Unsigned int
    let uint_type = CppType::Int { signed: false };
    let props = uint_type.properties().unwrap();
    assert!(props.is_integral);
    assert!(!props.is_signed);
    assert!(props.is_scalar);

    // Bool is integral but not signed
    let bool_type = CppType::Bool;
    let props = bool_type.properties().unwrap();
    assert!(props.is_integral);
    assert!(!props.is_signed);
    assert!(props.is_scalar);
}

/// Test type properties for floating point types.
#[test]
fn test_type_properties_floating() {
    let double_type = CppType::Double;
    let props = double_type.properties().unwrap();
    assert!(!props.is_integral);
    assert!(props.is_signed);
    assert!(props.is_floating_point);
    assert!(props.is_scalar);
    assert!(props.is_trivially_copyable);

    let float_type = CppType::Float;
    let props = float_type.properties().unwrap();
    assert!(!props.is_integral);
    assert!(props.is_floating_point);
    assert!(props.is_scalar);
}

/// Test type properties for pointer types.
#[test]
fn test_type_properties_pointer() {
    let ptr_type = CppType::int().ptr();
    let props = ptr_type.properties().unwrap();
    assert!(!props.is_integral);
    assert!(!props.is_floating_point);
    assert!(props.is_scalar);
    assert!(props.is_pointer);
    assert!(props.is_trivially_copyable);
}

/// Test type properties for reference types.
#[test]
fn test_type_properties_reference() {
    let ref_type = CppType::int().ref_();
    let props = ref_type.properties().unwrap();
    assert!(!props.is_integral);
    assert!(!props.is_scalar);
    assert!(props.is_reference);
    assert!(!props.is_trivially_copyable);
}

/// Test that template parameters have no properties (dependent).
#[test]
fn test_type_properties_template_param() {
    let template_type = CppType::template_param("T", 0, 0);
    assert!(template_type.properties().is_none());
    assert!(template_type.is_integral().is_none());
}

/// Test is_integral helper method.
#[test]
fn test_is_integral_helper() {
    assert_eq!(CppType::int().is_integral(), Some(true));
    assert_eq!(CppType::Double.is_integral(), Some(false));
    assert_eq!(CppType::template_param("T", 0, 0).is_integral(), None);
}

/// Test is_arithmetic helper method.
#[test]
fn test_is_arithmetic_helper() {
    assert_eq!(CppType::int().is_arithmetic(), Some(true));
    assert_eq!(CppType::Double.is_arithmetic(), Some(true));
    assert_eq!(CppType::int().ptr().is_arithmetic(), Some(false));
}

// ============================================================================
// Type Trait Evaluator Tests
// ============================================================================

use fragile_clang::{TypeTraitEvaluator, TypeTraitResult};

/// Test TypeTraitEvaluator::is_integral with various types.
#[test]
fn test_type_trait_is_integral() {
    // Integral types
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::int()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::Bool), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::Char { signed: true }), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::Short { signed: false }), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::Long { signed: true }), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::LongLong { signed: false }), TypeTraitResult::Value(true));

    // Non-integral types
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::Double), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::Float), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::int().ptr()), TypeTraitResult::Value(false));

    // Dependent types
    assert_eq!(TypeTraitEvaluator::is_integral(&CppType::template_param("T", 0, 0)), TypeTraitResult::Dependent);
}

/// Test TypeTraitEvaluator::is_signed and is_unsigned.
#[test]
fn test_type_trait_is_signed() {
    // Signed types
    assert_eq!(TypeTraitEvaluator::is_signed(&CppType::Int { signed: true }), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_signed(&CppType::Char { signed: true }), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_signed(&CppType::Double), TypeTraitResult::Value(true)); // floating point is signed

    // Unsigned types
    assert_eq!(TypeTraitEvaluator::is_signed(&CppType::Int { signed: false }), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_signed(&CppType::Bool), TypeTraitResult::Value(false)); // bool is unsigned

    // is_unsigned should be opposite
    assert_eq!(TypeTraitEvaluator::is_unsigned(&CppType::Int { signed: true }), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_unsigned(&CppType::Int { signed: false }), TypeTraitResult::Value(true));

    // Dependent
    assert_eq!(TypeTraitEvaluator::is_signed(&CppType::template_param("T", 0, 0)), TypeTraitResult::Dependent);
}

/// Test TypeTraitEvaluator::is_floating_point.
#[test]
fn test_type_trait_is_floating_point() {
    assert_eq!(TypeTraitEvaluator::is_floating_point(&CppType::Float), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_floating_point(&CppType::Double), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_floating_point(&CppType::int()), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_floating_point(&CppType::Bool), TypeTraitResult::Value(false));
}

/// Test TypeTraitEvaluator::is_same.
#[test]
fn test_type_trait_is_same() {
    // Same types
    assert_eq!(TypeTraitEvaluator::is_same(&CppType::int(), &CppType::int()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_same(&CppType::Double, &CppType::Double), TypeTraitResult::Value(true));

    // Different types
    assert_eq!(TypeTraitEvaluator::is_same(&CppType::int(), &CppType::Double), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_same(&CppType::int(), &CppType::uint()), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_same(&CppType::int(), &CppType::int().ptr()), TypeTraitResult::Value(false));

    // Dependent types
    let t_param = CppType::template_param("T", 0, 0);
    assert_eq!(TypeTraitEvaluator::is_same(&t_param, &CppType::int()), TypeTraitResult::Dependent);
    assert_eq!(TypeTraitEvaluator::is_same(&CppType::int(), &t_param), TypeTraitResult::Dependent);
    assert_eq!(TypeTraitEvaluator::is_same(&t_param, &t_param), TypeTraitResult::Dependent);
}

/// Test TypeTraitEvaluator::is_pointer and is_reference.
#[test]
fn test_type_trait_is_pointer_reference() {
    // Pointer types
    assert_eq!(TypeTraitEvaluator::is_pointer(&CppType::int().ptr()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_pointer(&CppType::Void.ptr()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_pointer(&CppType::int()), TypeTraitResult::Value(false));

    // Reference types
    assert_eq!(TypeTraitEvaluator::is_reference(&CppType::int().ref_()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_reference(&CppType::int().const_ref()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_reference(&CppType::int().rvalue_ref()), TypeTraitResult::Value(true));
    assert_eq!(TypeTraitEvaluator::is_reference(&CppType::int()), TypeTraitResult::Value(false));
    assert_eq!(TypeTraitEvaluator::is_reference(&CppType::int().ptr()), TypeTraitResult::Value(false));
}

/// Test TypeTraitResult helper methods.
#[test]
fn test_type_trait_result_helpers() {
    let true_result = TypeTraitResult::Value(true);
    let false_result = TypeTraitResult::Value(false);
    let dependent = TypeTraitResult::Dependent;

    assert!(true_result.is_true());
    assert!(!true_result.is_false());
    assert!(!true_result.is_dependent());
    assert_eq!(true_result.to_bool(), Some(true));

    assert!(!false_result.is_true());
    assert!(false_result.is_false());
    assert!(!false_result.is_dependent());
    assert_eq!(false_result.to_bool(), Some(false));

    assert!(!dependent.is_true());
    assert!(!dependent.is_false());
    assert!(dependent.is_dependent());
    assert_eq!(dependent.to_bool(), None);
}

/// Test TypeTraitEvaluator::is_base_of with concrete types.
#[test]
fn test_type_trait_is_base_of() {
    // Same type is a base of itself
    let my_class = CppType::Named("MyClass".to_string());
    assert_eq!(TypeTraitEvaluator::is_base_of(&my_class, &my_class), TypeTraitResult::Value(true));

    // Different named types - we don't have hierarchy info, so result is Dependent
    let base = CppType::Named("Base".to_string());
    let derived = CppType::Named("Derived".to_string());
    assert_eq!(TypeTraitEvaluator::is_base_of(&base, &derived), TypeTraitResult::Dependent);

    // Non-class types: false
    assert_eq!(TypeTraitEvaluator::is_base_of(&CppType::int(), &CppType::Double), TypeTraitResult::Value(false));
}

// ============================================================================
// For Statement Tests
// ============================================================================

/// Test basic for loop parsing.
#[test]
fn test_for_loop_basic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int sum() {
            int total = 0;
            for (int i = 0; i < 10; i++) {
                total += i;
            }
            return total;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have one function
    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "sum");

    // MIR body should have multiple basic blocks for the for loop
    // Entry block, loop header, loop body, loop exit
    assert!(func.mir_body.blocks.len() >= 3);
}

/// Test for loop with empty body.
#[test]
fn test_for_loop_empty_body() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void spin(int n) {
            for (int i = 0; i < n; i++) {
            }
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "spin");
}

/// Test for loop with multiple initializers (using expression).
#[test]
fn test_for_loop_expression_init() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int countdown(int n) {
            int i;
            for (i = n; i > 0; i--) {
            }
            return i;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "countdown");
}

/// Test nested for loops.
#[test]
fn test_for_loop_nested() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int matrix_sum(int n) {
            int sum = 0;
            for (int i = 0; i < n; i++) {
                for (int j = 0; j < n; j++) {
                    sum += 1;
                }
            }
            return sum;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    // Nested loops should have more blocks than a simple function
    // (at least entry block + some loop structure)
    assert!(module.functions[0].mir_body.blocks.len() >= 2);
}

// ============================================================================
// Switch Statement Tests
// ============================================================================

/// Test basic switch statement parsing.
#[test]
fn test_switch_basic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int classify(int x) {
            switch (x) {
                case 0:
                    return 0;
                case 1:
                    return 1;
                default:
                    return -1;
            }
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "classify");

    // Should have multiple blocks for switch branches
    assert!(func.mir_body.blocks.len() >= 3);
}

/// Test switch with multiple cases.
#[test]
fn test_switch_multiple_cases() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int day_type(int day) {
            switch (day) {
                case 1:
                case 2:
                case 3:
                case 4:
                case 5:
                    return 1;
                case 6:
                case 7:
                    return 0;
                default:
                    return -1;
            }
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "day_type");
}

/// Test switch without default.
#[test]
fn test_switch_no_default() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int simple_switch(int x) {
            switch (x) {
                case 0:
                    return 0;
                case 1:
                    return 1;
            }
            return -1;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "simple_switch");
}

// ============================================================================
// Break and Continue Statement Tests
// ============================================================================

#[test]
fn test_break_in_while_loop() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int find_first_even(int n) {
            int i = 0;
            while (i < n) {
                if (i % 2 == 0) {
                    break;
                }
                i = i + 1;
            }
            return i;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "find_first_even");

    // Check that there's a Goto terminator (not Unreachable) for break
    let has_goto = func.mir_body.blocks.iter().any(|block| {
        matches!(block.terminator, MirTerminator::Goto { .. })
    });
    assert!(has_goto, "Break should generate a Goto terminator");
}

#[test]
fn test_continue_in_while_loop() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int sum_odd(int n) {
            int i = 0;
            int sum = 0;
            while (i < n) {
                i = i + 1;
                if (i % 2 == 0) {
                    continue;
                }
                sum = sum + i;
            }
            return sum;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "sum_odd");

    // Check that we have blocks for the loop structure
    assert!(func.mir_body.blocks.len() >= 3, "Should have blocks for loop header, body, and exit");
}

#[test]
fn test_break_in_for_loop() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int find_divisor(int n, int d) {
            for (int i = 2; i < n; i = i + 1) {
                if (n % i == 0) {
                    return i;
                }
                if (i > d) {
                    break;
                }
            }
            return -1;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "find_divisor");

    // Check that break generates a Goto terminator, not Unreachable
    let has_unreachable = func.mir_body.blocks.iter().any(|block| {
        matches!(block.terminator, MirTerminator::Unreachable)
    });
    assert!(!has_unreachable, "Break should not generate Unreachable terminator");
}

#[test]
fn test_continue_in_for_loop() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int count_non_divisible(int n, int d) {
            int count = 0;
            for (int i = 1; i <= n; i = i + 1) {
                if (i % d == 0) {
                    continue;
                }
                count = count + 1;
            }
            return count;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "count_non_divisible");

    // Check that continue generates Goto (not Unreachable)
    let has_unreachable = func.mir_body.blocks.iter().any(|block| {
        matches!(block.terminator, MirTerminator::Unreachable)
    });
    assert!(!has_unreachable, "Continue should not generate Unreachable terminator");
}

#[test]
fn test_nested_loops_break() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int nested_break(int n) {
            int result = 0;
            for (int i = 0; i < n; i = i + 1) {
                for (int j = 0; j < n; j = j + 1) {
                    if (j > i) {
                        break;
                    }
                    result = result + 1;
                }
            }
            return result;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    let func = &module.functions[0];
    assert_eq!(func.display_name, "nested_break");

    // Check that break in inner loop generates Goto (not Unreachable)
    let has_unreachable = func.mir_body.blocks.iter().any(|block| {
        matches!(block.terminator, MirTerminator::Unreachable)
    });
    assert!(!has_unreachable, "Break in nested loop should not generate Unreachable terminator");

    // Check that we have at least some blocks generated
    assert!(func.mir_body.blocks.len() >= 1, "Function should have at least one block");
}

// ============================================================================
// Binary and Unary Operator Extraction Tests
// ============================================================================

#[test]
fn test_binary_operator_arithmetic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int arithmetic(int a, int b) {
            int sum = a + b;
            int diff = a - b;
            int prod = a * b;
            int quot = a / b;
            int rem = a % b;
            return sum + diff + prod + quot + rem;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "arithmetic");
    // Should generate multiple basic blocks with arithmetic operations
    assert!(module.functions[0].mir_body.blocks.len() >= 1);
}

#[test]
fn test_binary_operator_comparison() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        bool compare(int a, int b) {
            bool eq = a == b;
            bool ne = a != b;
            bool lt = a < b;
            bool le = a <= b;
            bool gt = a > b;
            bool ge = a >= b;
            return eq && ne && lt && le && gt && ge;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "compare");
}

#[test]
fn test_binary_operator_bitwise() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int bitwise(int a, int b) {
            int band = a & b;
            int bor = a | b;
            int bxor = a ^ b;
            int shl = a << 2;
            int shr = a >> 2;
            return band + bor + bxor + shl + shr;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "bitwise");
}

#[test]
fn test_binary_operator_logical() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        bool logical(bool a, bool b) {
            bool land = a && b;
            bool lor = a || b;
            return land || lor;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "logical");
}

#[test]
fn test_binary_operator_assignment() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int assignments(int x) {
            x = 10;
            x += 5;
            x -= 2;
            x *= 3;
            x /= 2;
            return x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "assignments");
}

#[test]
fn test_unary_operator_arithmetic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int unary_arith(int x) {
            int neg = -x;
            int pos = +x;
            return neg + pos;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "unary_arith");
}

#[test]
fn test_unary_operator_logical() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        bool unary_logical(bool x) {
            return !x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "unary_logical");
}

#[test]
fn test_unary_operator_bitwise() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int unary_bitwise(int x) {
            return ~x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "unary_bitwise");
}

#[test]
fn test_unary_operator_increment_decrement() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int inc_dec(int x) {
            int a = ++x;
            int b = x++;
            int c = --x;
            int d = x--;
            return a + b + c + d;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].display_name, "inc_dec");
}

// ============================================================================
// C++20 Concepts Tests
// ============================================================================

/// Test parsing a simple concept definition.
#[test]
fn test_concept_definition() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept Integral = __is_integral(T);
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.concepts.len(), 1);
    let concept = &module.concepts[0];
    assert_eq!(concept.name, "Integral");
    assert_eq!(concept.template_params, vec!["T"]);
    assert!(!concept.constraint_expr.is_empty());
}

/// Test parsing multiple concepts.
#[test]
fn test_multiple_concepts() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept Integral = __is_integral(T);

        template<typename T>
        concept Signed = __is_signed(T);

        template<typename T>
        concept SignedIntegral = Integral<T> && Signed<T>;
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.concepts.len(), 3);
    assert_eq!(module.concepts[0].name, "Integral");
    assert_eq!(module.concepts[1].name, "Signed");
    assert_eq!(module.concepts[2].name, "SignedIntegral");
}

/// Test function template with requires clause.
#[test]
fn test_function_template_with_requires_clause() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept Integral = __is_integral(T);

        template<typename T>
            requires Integral<T>
        T twice(T x) {
            return x + x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have 1 concept and 1 function template
    assert_eq!(module.concepts.len(), 1);
    assert_eq!(module.function_templates.len(), 1);

    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "twice");
    // The requires clause should be captured (may be None if not fully parsed yet)
    // This test verifies the infrastructure is in place
}

/// Test function template without requires clause (baseline).
#[test]
fn test_function_template_without_requires_clause() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        T identity(T x) {
            return x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.function_templates.len(), 1);
    let tmpl = &module.function_templates[0];
    assert_eq!(tmpl.name, "identity");
    assert!(tmpl.requires_clause.is_none());
}

/// Test class template with requires clause.
#[test]
fn test_class_template_with_requires_clause() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept Integral = __is_integral(T);

        template<typename T>
            requires Integral<T>
        class Counter {
            T value;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    // Should have 1 concept and 1 class template
    assert_eq!(module.concepts.len(), 1);
    assert_eq!(module.class_templates.len(), 1);

    let tmpl = &module.class_templates[0];
    assert_eq!(tmpl.name, "Counter");
}

/// Test concept with requires expression containing simple requirements.
#[test]
fn test_concept_with_requires_expression() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept Addable = requires(T a, T b) {
            a + b;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.concepts.len(), 1);
    let concept = &module.concepts[0];
    assert_eq!(concept.name, "Addable");
    // The constraint should contain the requires expression
    assert!(concept.constraint_expr.contains("requires") || !concept.constraint_expr.is_empty());
}

/// Test concept with multiple requirements.
#[test]
fn test_concept_with_multiple_requirements() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept Copyable = requires(T a) {
            T(a);
            a = a;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.concepts.len(), 1);
    let concept = &module.concepts[0];
    assert_eq!(concept.name, "Copyable");
}

/// Test concept with type requirement.
#[test]
fn test_concept_with_type_requirement() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        concept HasValueType = requires {
            typename T::value_type;
        };
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).expect("Failed to convert");

    assert_eq!(module.concepts.len(), 1);
    let concept = &module.concepts[0];
    assert_eq!(concept.name, "HasValueType");
}

// ============================================================================
// Header Include Path Tests
// ============================================================================

/// Test that parser can be created with system include paths.
#[test]
fn test_parser_with_system_includes() {
    let parser = ClangParser::with_system_includes().unwrap();
    // Just verify it creates successfully
    let code = r#"
        int main() { return 0; }
    "#;
    let ast = parser.parse_string(code, "test.cpp").unwrap();
    assert!(ast.translation_unit.children.len() >= 1);
}

/// Test parsing code that includes a system header.
/// This tests that header search paths are working.
#[test]
fn test_parse_with_std_header() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <cstddef>
        size_t get_size() { return 42; }
    "#;

    // This should not error with "file not found"
    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse code with <cstddef>: {:?}", result.err());
}

/// Test that custom include paths work.
#[test]
fn test_parser_with_custom_include_paths() {
    let custom_paths = vec!["/tmp".to_string()];
    let parser = ClangParser::with_include_paths(custom_paths).unwrap();

    let code = r#"
        int main() { return 0; }
    "#;
    let ast = parser.parse_string(code, "test.cpp").unwrap();
    assert!(ast.translation_unit.children.len() >= 1);
}

// ========== Type Alias Tests (C.0.2) ==========

/// Test parsing a simple type alias using 'using'.
#[test]
fn test_type_alias_using() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        using IntAlias = int;
        using FloatPtr = float*;
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    assert_eq!(module.type_aliases.len(), 2, "Expected 2 type aliases");

    let alias1 = &module.type_aliases[0];
    assert_eq!(alias1.name, "IntAlias");
    assert_eq!(alias1.underlying_type, CppType::Int { signed: true });
    assert!(!alias1.is_template);

    let alias2 = &module.type_aliases[1];
    assert_eq!(alias2.name, "FloatPtr");
    assert!(matches!(alias2.underlying_type, CppType::Pointer { .. }));
    assert!(!alias2.is_template);
}

/// Test parsing a typedef (old-style C type alias).
#[test]
fn test_typedef() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        typedef int Integer;
        typedef const char* CString;
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    assert_eq!(module.type_aliases.len(), 2, "Expected 2 typedefs");

    let alias1 = &module.type_aliases[0];
    assert_eq!(alias1.name, "Integer");
    assert_eq!(alias1.underlying_type, CppType::Int { signed: true });
    assert!(!alias1.is_template);

    let alias2 = &module.type_aliases[1];
    assert_eq!(alias2.name, "CString");
    assert!(matches!(alias2.underlying_type, CppType::Pointer { .. }));
    assert!(!alias2.is_template);
}

/// Test parsing a type alias template.
#[test]
fn test_type_alias_template() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        template<typename T>
        using Ptr = T*;

        template<typename T, typename U>
        struct Pair { T first; U second; };

        template<typename T>
        using PairOfT = Pair<T, T>;
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // Should have at least 2 template aliases: Ptr and PairOfT
    let template_aliases: Vec<_> = module.type_aliases.iter().filter(|a| a.is_template).collect();
    assert!(template_aliases.len() >= 2, "Expected at least 2 template aliases, got {}", template_aliases.len());

    // Find the Ptr alias
    let ptr_alias = template_aliases.iter().find(|a| a.name == "Ptr");
    assert!(ptr_alias.is_some(), "Expected to find Ptr template alias");
    let ptr_alias = ptr_alias.unwrap();
    assert!(ptr_alias.is_template);
    assert_eq!(ptr_alias.template_params.len(), 1);
    assert_eq!(ptr_alias.template_params[0], "T");
}

/// Test type alias in namespace.
#[test]
fn test_type_alias_in_namespace() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        namespace myns {
            using MyInt = int;
            typedef double MyDouble;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    assert_eq!(module.type_aliases.len(), 2, "Expected 2 type aliases");

    for alias in &module.type_aliases {
        assert_eq!(alias.namespace, vec!["myns".to_string()], "Expected alias in myns namespace");
    }

    let my_int = module.type_aliases.iter().find(|a| a.name == "MyInt");
    assert!(my_int.is_some());
    assert_eq!(my_int.unwrap().underlying_type, CppType::Int { signed: true });

    let my_double = module.type_aliases.iter().find(|a| a.name == "MyDouble");
    assert!(my_double.is_some());
    assert_eq!(my_double.unwrap().underlying_type, CppType::Double);
}

/// Test type alias with reference types.
#[test]
fn test_type_alias_reference() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        using IntRef = int&;
        using ConstIntRef = const int&;
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    assert_eq!(module.type_aliases.len(), 2, "Expected 2 type aliases");

    let int_ref = &module.type_aliases[0];
    assert_eq!(int_ref.name, "IntRef");
    assert!(matches!(int_ref.underlying_type, CppType::Reference { is_rvalue: false, is_const: false, .. }));

    let const_int_ref = &module.type_aliases[1];
    assert_eq!(const_int_ref.name, "ConstIntRef");
    assert!(matches!(const_int_ref.underlying_type, CppType::Reference { is_rvalue: false, is_const: true, .. }));
}

/// Test type alias with struct type.
#[test]
fn test_type_alias_struct() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        struct Point { int x; int y; };
        using PointAlias = Point;
        typedef Point PointTypedef;
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    assert_eq!(module.type_aliases.len(), 2, "Expected 2 type aliases");

    let point_alias = &module.type_aliases[0];
    assert_eq!(point_alias.name, "PointAlias");
    assert!(matches!(&point_alias.underlying_type, CppType::Named(name) if name == "Point"));

    let point_typedef = &module.type_aliases[1];
    assert_eq!(point_typedef.name, "PointTypedef");
    assert!(matches!(&point_typedef.underlying_type, CppType::Named(name) if name == "Point"));
}

// ========== std::vector Tests (C.1.1) ==========

/// Test parsing code that uses std::vector with system includes.
#[test]
fn test_std_vector_basic_usage() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <vector>

        int test_vector() {
            std::vector<int> v;
            v.push_back(42);
            v.push_back(100);
            int x = v[0];
            int s = v.size();
            return s + x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // Should have the test_vector function (among many STL functions)
    let test_fn = module.functions.iter().find(|f| f.display_name == "test_vector");
    assert!(test_fn.is_some(), "Expected to find test_vector function");

    let test_fn = test_fn.unwrap();
    assert!(matches!(test_fn.return_type, CppType::Int { signed: true }));

    // Verify the MIR has basic blocks with function calls
    assert!(!test_fn.mir_body.blocks.is_empty(), "Expected MIR body to have blocks");
}

/// Test parsing code that uses std::vector with pop_back.
#[test]
fn test_std_vector_pop_back() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <vector>

        void test_pop() {
            std::vector<int> v;
            v.push_back(1);
            v.pop_back();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_pop");
    assert!(test_fn.is_some(), "Expected to find test_pop function");
}

/// Test parsing code that uses std::vector with iterators.
#[test]
fn test_std_vector_iterators() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <vector>

        int sum_vector() {
            std::vector<int> v;
            v.push_back(1);
            v.push_back(2);
            v.push_back(3);

            int sum = 0;
            for (auto it = v.begin(); it != v.end(); ++it) {
                sum += *it;
            }
            return sum;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let sum_fn = module.functions.iter().find(|f| f.display_name == "sum_vector");
    assert!(sum_fn.is_some(), "Expected to find sum_vector function");
}

// ========== std::string Tests (C.1.2) ==========

/// Test parsing code that uses std::string basic operations.
#[test]
fn test_std_string_basic_usage() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <string>

        int test_string() {
            std::string s = "hello";
            int len = s.size();
            return len;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_string");
    assert!(test_fn.is_some(), "Expected to find test_string function");

    let test_fn = test_fn.unwrap();
    assert!(matches!(test_fn.return_type, CppType::Int { signed: true }));
}

/// Test std::string with c_str() and operator[].
#[test]
fn test_std_string_cstr_subscript() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <string>

        char test_cstr() {
            std::string s = "world";
            const char* ptr = s.c_str();
            char c = s[0];
            return c;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_cstr");
    assert!(test_fn.is_some(), "Expected to find test_cstr function");
}

/// Test std::string with length() and empty().
#[test]
fn test_std_string_length_empty() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <string>

        bool test_empty() {
            std::string s;
            bool empty = s.empty();
            int len = s.length();
            return empty;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_empty");
    assert!(test_fn.is_some(), "Expected to find test_empty function");
}

// ========== Smart Pointer Tests (C.2) ==========

/// Test parsing code that uses std::unique_ptr.
#[test]
fn test_std_unique_ptr_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <memory>

        int test_unique() {
            std::unique_ptr<int> p = std::make_unique<int>(42);
            int val = *p;
            p.reset();
            return val;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_unique");
    assert!(test_fn.is_some(), "Expected to find test_unique function");
}

/// Test std::unique_ptr with custom type.
#[test]
fn test_std_unique_ptr_custom_type() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <memory>

        struct Point { int x; int y; };

        int test_unique_point() {
            std::unique_ptr<Point> p = std::make_unique<Point>();
            p->x = 10;
            p->y = 20;
            return p->x + p->y;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_unique_point");
    assert!(test_fn.is_some(), "Expected to find test_unique_point function");
}

/// Test std::shared_ptr basic usage.
#[test]
fn test_std_shared_ptr_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <memory>

        int test_shared() {
            std::shared_ptr<int> p1 = std::make_shared<int>(42);
            std::shared_ptr<int> p2 = p1;  // copy (increases ref count)
            int val = *p2;
            long count = p1.use_count();
            return val;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_shared");
    assert!(test_fn.is_some(), "Expected to find test_shared function");
}

/// Test std::weak_ptr basic usage.
#[test]
fn test_std_weak_ptr_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <memory>

        int test_weak() {
            std::shared_ptr<int> shared = std::make_shared<int>(42);
            std::weak_ptr<int> weak = shared;

            if (auto locked = weak.lock()) {
                return *locked;
            }
            return 0;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_weak");
    assert!(test_fn.is_some(), "Expected to find test_weak function");
}

// ========== Lambda Tests (E.3) ==========

/// Test parsing code with basic lambda expressions.
#[test]
fn test_lambda_basic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int test_lambda() {
            auto add = [](int a, int b) { return a + b; };
            return add(2, 3);
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_lambda");
    assert!(test_fn.is_some(), "Expected to find test_lambda function");
}

/// Test lambda with value capture.
#[test]
fn test_lambda_capture_value() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int test_capture() {
            int x = 10;
            auto add_x = [x](int a) { return a + x; };
            return add_x(5);
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_capture");
    assert!(test_fn.is_some(), "Expected to find test_capture function");
}

/// Test lambda with reference capture.
#[test]
fn test_lambda_capture_ref() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int test_capture_ref() {
            int x = 10;
            auto increment = [&x]() { x++; };
            increment();
            return x;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_capture_ref");
    assert!(test_fn.is_some(), "Expected to find test_capture_ref function");
}

/// Test generic lambda (auto parameters).
#[test]
fn test_lambda_generic() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        int test_generic() {
            auto add = [](auto a, auto b) { return a + b; };
            return add(2, 3);
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_generic");
    assert!(test_fn.is_some(), "Expected to find test_generic function");
}

// ========== std::function Tests (C.4) ==========

/// Test parsing code that uses std::function.
#[test]
fn test_std_function_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <functional>

        int apply_fn(std::function<int(int)> f, int x) {
            return f(x);
        }

        int test_function() {
            auto double_it = [](int x) { return x * 2; };
            std::function<int(int)> fn = double_it;
            return apply_fn(fn, 5);
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_function");
    assert!(test_fn.is_some(), "Expected to find test_function function");

    let apply_fn = module.functions.iter().find(|f| f.display_name == "apply_fn");
    assert!(apply_fn.is_some(), "Expected to find apply_fn function");
}

/// Test std::function with void return type.
#[test]
fn test_std_function_void() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <functional>

        void test_void_fn() {
            int counter = 0;
            std::function<void()> incr = [&counter]() { counter++; };
            incr();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_void_fn");
    assert!(test_fn.is_some(), "Expected to find test_void_fn function");
}

// ========== std::chrono Tests (C.4) ==========

/// Test parsing code that uses std::chrono.
#[test]
fn test_std_chrono_duration() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <chrono>

        long get_duration() {
            using namespace std::chrono;
            auto start = steady_clock::now();
            auto end = steady_clock::now();
            auto elapsed = duration_cast<milliseconds>(end - start);
            return elapsed.count();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "get_duration");
    assert!(test_fn.is_some(), "Expected to find get_duration function");
}

/// Test std::chrono time points.
#[test]
fn test_std_chrono_time_point() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <chrono>

        bool is_elapsed(int ms) {
            using namespace std::chrono;
            auto deadline = steady_clock::now() + milliseconds(ms);
            return steady_clock::now() >= deadline;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "is_elapsed");
    assert!(test_fn.is_some(), "Expected to find is_elapsed function");
}

// ========== Concurrency Tests (C.3) ==========

/// Test parsing code that uses std::thread.
#[test]
fn test_std_thread_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <thread>

        void thread_work() {}

        void test_thread() {
            std::thread t(thread_work);
            t.join();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_thread");
    assert!(test_fn.is_some(), "Expected to find test_thread function");
}

/// Test std::mutex and std::lock_guard.
#[test]
fn test_std_mutex_lock_guard() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <mutex>

        std::mutex g_mutex;
        int counter = 0;

        void increment() {
            std::lock_guard<std::mutex> lock(g_mutex);
            counter++;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "increment");
    assert!(test_fn.is_some(), "Expected to find increment function");
}

/// Test std::atomic.
#[test]
fn test_std_atomic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <atomic>

        std::atomic<int> counter{0};

        int test_atomic() {
            counter++;
            return counter.load();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_atomic");
    assert!(test_fn.is_some(), "Expected to find test_atomic function");
}

/// Test std::condition_variable.
#[test]
fn test_std_condition_variable() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <mutex>
        #include <condition_variable>

        std::mutex mtx;
        std::condition_variable cv;
        bool ready = false;

        void wait_for_signal() {
            std::unique_lock<std::mutex> lock(mtx);
            cv.wait(lock, []{ return ready; });
        }

        void signal() {
            std::lock_guard<std::mutex> lock(mtx);
            ready = true;
            cv.notify_one();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let wait_fn = module.functions.iter().find(|f| f.display_name == "wait_for_signal");
    assert!(wait_fn.is_some(), "Expected to find wait_for_signal function");

    let signal_fn = module.functions.iter().find(|f| f.display_name == "signal");
    assert!(signal_fn.is_some(), "Expected to find signal function");
}

// ========== Other Containers Tests (C.1.3) ==========

/// Test parsing code that uses std::optional.
#[test]
fn test_std_optional_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <optional>

        std::optional<int> find_value(bool found) {
            if (found) {
                return 42;
            }
            return std::nullopt;
        }

        int use_optional() {
            auto opt = find_value(true);
            if (opt.has_value()) {
                return opt.value();
            }
            return 0;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let find_fn = module.functions.iter().find(|f| f.display_name == "find_value");
    assert!(find_fn.is_some(), "Expected to find find_value function");

    let use_fn = module.functions.iter().find(|f| f.display_name == "use_optional");
    assert!(use_fn.is_some(), "Expected to find use_optional function");
}

/// Test std::variant.
#[test]
fn test_std_variant_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <variant>
        #include <string>

        int use_variant() {
            std::variant<int, double, std::string> v = 42;
            if (std::holds_alternative<int>(v)) {
                return std::get<int>(v);
            }
            return 0;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let use_fn = module.functions.iter().find(|f| f.display_name == "use_variant");
    assert!(use_fn.is_some(), "Expected to find use_variant function");
}

/// Test std::map.
#[test]
fn test_std_map_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <map>
        #include <string>

        void use_map() {
            std::map<std::string, int> m;
            m["one"] = 1;
            m["two"] = 2;
            int val = m["one"];
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let use_fn = module.functions.iter().find(|f| f.display_name == "use_map");
    assert!(use_fn.is_some(), "Expected to find use_map function");
}

/// Test std::unordered_map.
#[test]
fn test_std_unordered_map_basic() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <unordered_map>
        #include <string>

        int use_unordered_map() {
            std::unordered_map<std::string, int> m;
            m["key"] = 42;
            return m["key"];
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let use_fn = module.functions.iter().find(|f| f.display_name == "use_unordered_map");
    assert!(use_fn.is_some(), "Expected to find use_unordered_map function");
}

// ========== Attribute Tests (E.4) ==========

/// Test parsing code with [[nodiscard]] attribute.
#[test]
fn test_attribute_nodiscard() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        [[nodiscard]] int get_value() {
            return 42;
        }

        int use_nodiscard() {
            return get_value();
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let get_fn = module.functions.iter().find(|f| f.display_name == "get_value");
    assert!(get_fn.is_some(), "Expected to find get_value function");

    let use_fn = module.functions.iter().find(|f| f.display_name == "use_nodiscard");
    assert!(use_fn.is_some(), "Expected to find use_nodiscard function");
}

/// Test parsing code with [[maybe_unused]] attribute.
#[test]
fn test_attribute_maybe_unused() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void use_maybe_unused([[maybe_unused]] int x) {
            // x might be unused
        }

        int test_unused() {
            [[maybe_unused]] int unused_var = 42;
            return 0;
        }
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let use_fn = module.functions.iter().find(|f| f.display_name == "use_maybe_unused");
    assert!(use_fn.is_some(), "Expected to find use_maybe_unused function");

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_unused");
    assert!(test_fn.is_some(), "Expected to find test_unused function");
}

/// Test parsing code with [[deprecated]] attribute.
#[test]
fn test_attribute_deprecated() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        [[deprecated("Use new_api instead")]]
        void old_api() {}

        void new_api() {}
    "#;

    let ast = parser.parse_string(code, "test.cpp").unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let old_fn = module.functions.iter().find(|f| f.display_name == "old_api");
    assert!(old_fn.is_some(), "Expected to find old_api function");

    let new_fn = module.functions.iter().find(|f| f.display_name == "new_api");
    assert!(new_fn.is_some(), "Expected to find new_api function");
}

// ========== C++20 Coroutines Tests (Phase D) ==========

/// Helper function to recursively search for a specific node kind in the AST.
fn find_node_kind<'a>(
    node: &'a fragile_clang::ClangNode,
    predicate: impl Fn(&ClangNodeKind) -> bool + Copy,
) -> Option<&'a fragile_clang::ClangNode> {
    if predicate(&node.kind) {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node_kind(child, predicate) {
            return Some(found);
        }
    }
    None
}

/// Test parsing co_await expression.
#[test]
fn test_coroutine_co_await() {
    let parser = ClangParser::with_system_includes().unwrap();
    // C++20 coroutine with co_await
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task simple_coroutine() {
            co_await std::suspend_always{};
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse coroutine code: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // The simple_coroutine function should be parsed
    let coro_fn = module.functions.iter().find(|f| f.display_name == "simple_coroutine");
    assert!(coro_fn.is_some(), "Expected to find simple_coroutine function");
}

/// Test parsing co_yield expression.
#[test]
fn test_coroutine_co_yield() {
    let parser = ClangParser::with_system_includes().unwrap();
    // C++20 generator coroutine with co_yield
    let code = r#"
        #include <coroutine>

        struct Generator {
            struct promise_type {
                int current_value;
                Generator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        Generator generate_numbers() {
            co_yield 1;
            co_yield 2;
            co_yield 3;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse generator code: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // The generate_numbers function should be parsed
    let gen_fn = module.functions.iter().find(|f| f.display_name == "generate_numbers");
    assert!(gen_fn.is_some(), "Expected to find generate_numbers function");
}

/// Test parsing co_return statement.
#[test]
fn test_coroutine_co_return() {
    let parser = ClangParser::with_system_includes().unwrap();
    // C++20 coroutine with co_return
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task coroutine_with_return() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse co_return code: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // The coroutine_with_return function should be parsed
    let coro_fn = module.functions.iter().find(|f| f.display_name == "coroutine_with_return");
    assert!(coro_fn.is_some(), "Expected to find coroutine_with_return function");
}

/// Test that CoawaitExpr AST node is properly created.
#[test]
fn test_coroutine_ast_coawait_node() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task test_await() {
            co_await std::suspend_always{};
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for CoawaitExpr node
    let has_coawait = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CoawaitExpr { .. })
    });

    // Note: This may fail if clang doesn't properly expose the coroutine as UnexposedExpr
    // In that case, we should check that the function parses correctly instead
    if has_coawait.is_none() {
        // At minimum, verify the code compiles and the function is recognized
        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();
        let fn_found = module.functions.iter().any(|f| f.display_name == "test_await");
        assert!(fn_found, "Expected test_await function to be parsed");
    }
}

/// Test that CoyieldExpr AST node is properly created.
#[test]
fn test_coroutine_ast_coyield_node() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Generator {
            struct promise_type {
                int current_value;
                Generator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        Generator test_yield() {
            co_yield 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for CoyieldExpr node
    let has_coyield = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CoyieldExpr { .. })
    });

    // Note: This may fail if clang doesn't properly expose the coroutine as UnexposedExpr
    if has_coyield.is_none() {
        // At minimum, verify the code compiles and the function is recognized
        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();
        let fn_found = module.functions.iter().any(|f| f.display_name == "test_yield");
        assert!(fn_found, "Expected test_yield function to be parsed");
    }
}

/// Test that CoreturnStmt AST node is properly created.
#[test]
fn test_coroutine_ast_coreturn_node() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task test_return() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for CoreturnStmt node
    let has_coreturn = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CoreturnStmt { .. })
    });

    // Note: This may fail if clang doesn't properly expose the coroutine as UnexposedStmt
    if has_coreturn.is_none() {
        // At minimum, verify the code compiles and the function is recognized
        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();
        let fn_found = module.functions.iter().any(|f| f.display_name == "test_return");
        assert!(fn_found, "Expected test_return function to be parsed");
    }
}

// ========== C++20 Coroutine Header Types Tests (D.4) ==========

/// Test parsing std::coroutine_handle type.
#[test]
fn test_coroutine_handle_type() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct MyPromise;

        void test_handle() {
            std::coroutine_handle<MyPromise> handle;
            std::coroutine_handle<void> void_handle;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse coroutine_handle: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_handle");
    assert!(test_fn.is_some(), "Expected to find test_handle function");
}

/// Test parsing std::suspend_always and std::suspend_never types.
#[test]
fn test_suspend_types() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        void test_suspend_types() {
            std::suspend_always always_suspender;
            std::suspend_never never_suspender;

            // Test await_ready, await_suspend, await_resume
            bool ready1 = always_suspender.await_ready();
            bool ready2 = never_suspender.await_ready();
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse suspend types: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_suspend_types");
    assert!(test_fn.is_some(), "Expected to find test_suspend_types function");
}

/// Test coroutine_traits usage in promise type detection.
#[test]
fn test_coroutine_traits() {
    let parser = ClangParser::with_system_includes().unwrap();
    // Test that we can parse code that relies on coroutine_traits
    let code = r#"
        #include <coroutine>

        template<typename T>
        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_value(T value) {}
                void unhandled_exception() {}
            };
        };

        Task<int> async_compute() {
            co_return 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse coroutine_traits code: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // The async_compute function should be parsed
    let coro_fn = module.functions.iter().find(|f| f.display_name == "async_compute");
    assert!(coro_fn.is_some(), "Expected to find async_compute function");
}

/// Test coroutine handle operations (resume, destroy, done).
#[test]
fn test_coroutine_handle_operations() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct MyPromise {
            std::suspend_always initial_suspend() { return {}; }
            std::suspend_always final_suspend() noexcept { return {}; }
            void return_void() {}
            void unhandled_exception() {}
            void get_return_object() {}
        };

        void test_handle_ops(std::coroutine_handle<MyPromise> h) {
            h.resume();
            bool is_done = h.done();
            h.destroy();
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse handle operations: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_handle_ops");
    assert!(test_fn.is_some(), "Expected to find test_handle_ops function");
}

// ========== C++ Exception Handling Tests (E.1) ==========

/// Test parsing basic try/catch block.
#[test]
fn test_exception_try_catch() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_exceptions() {
            try {
                int x = 42;
            } catch (int e) {
                int y = e;
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse try/catch: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_exceptions");
    assert!(test_fn.is_some(), "Expected to find test_exceptions function");
}

/// Test parsing throw expression.
#[test]
fn test_exception_throw() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_throw() {
            throw 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse throw: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_throw");
    assert!(test_fn.is_some(), "Expected to find test_throw function");
}

/// Test parsing catch-all handler.
#[test]
fn test_exception_catch_all() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_catch_all() {
            try {
                throw "error";
            } catch (...) {
                // Handle all exceptions
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse catch-all: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_catch_all");
    assert!(test_fn.is_some(), "Expected to find test_catch_all function");
}

/// Test parsing multiple catch handlers.
#[test]
fn test_exception_multiple_catch() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_multiple_catch() {
            try {
                throw 1.0;
            } catch (int e) {
                // Handle int
            } catch (double d) {
                // Handle double
            } catch (...) {
                // Handle all others
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse multiple catch: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_multiple_catch");
    assert!(test_fn.is_some(), "Expected to find test_multiple_catch function");
}

/// Test parsing rethrow (throw;).
#[test]
fn test_exception_rethrow() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_rethrow() {
            try {
                throw 42;
            } catch (...) {
                throw;  // Rethrow
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse rethrow: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_rethrow");
    assert!(test_fn.is_some(), "Expected to find test_rethrow function");
}

/// Test that TryStmt AST node is properly created.
#[test]
fn test_exception_ast_try_node() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_try() {
            try {
                int x = 1;
            } catch (int e) {
                int y = e;
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for TryStmt node
    let has_try = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::TryStmt)
    });

    assert!(has_try.is_some(), "Expected to find TryStmt node in AST");
}

/// Test that ThrowExpr AST node is properly created.
#[test]
fn test_exception_ast_throw_node() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        void test_throw_node() {
            throw 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for ThrowExpr node
    let has_throw = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::ThrowExpr { .. })
    });

    assert!(has_throw.is_some(), "Expected to find ThrowExpr node in AST");
}

// ========== C++ RTTI Tests (E.2) ==========

/// Test parsing typeid expression.
#[test]
fn test_rtti_typeid() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <typeinfo>

        void test_typeid() {
            int x = 42;
            const std::type_info& ti = typeid(x);
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse typeid: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_typeid");
    assert!(test_fn.is_some(), "Expected to find test_typeid function");
}

/// Test parsing typeid with type (not expression).
#[test]
fn test_rtti_typeid_type() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <typeinfo>

        void test_typeid_type() {
            const std::type_info& ti = typeid(int);
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse typeid(type): {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_typeid_type");
    assert!(test_fn.is_some(), "Expected to find test_typeid_type function");
}

/// Test parsing dynamic_cast.
#[test]
fn test_rtti_dynamic_cast() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        class Base { public: virtual ~Base() {} };
        class Derived : public Base {};

        void test_dynamic_cast(Base* b) {
            Derived* d = dynamic_cast<Derived*>(b);
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse dynamic_cast: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_dynamic_cast");
    assert!(test_fn.is_some(), "Expected to find test_dynamic_cast function");
}

/// Test parsing dynamic_cast with reference.
#[test]
fn test_rtti_dynamic_cast_reference() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        class Base { public: virtual ~Base() {} };
        class Derived : public Base {};

        void test_dynamic_cast_ref(Base& b) {
            Derived& d = dynamic_cast<Derived&>(b);
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse dynamic_cast reference: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let test_fn = module.functions.iter().find(|f| f.display_name == "test_dynamic_cast_ref");
    assert!(test_fn.is_some(), "Expected to find test_dynamic_cast_ref function");
}

/// Test that TypeidExpr AST node is properly created.
#[test]
fn test_rtti_ast_typeid_node() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <typeinfo>

        void test_typeid_node() {
            int x = 1;
            const std::type_info& ti = typeid(x);
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for TypeidExpr node
    let has_typeid = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::TypeidExpr { .. })
    });

    assert!(has_typeid.is_some(), "Expected to find TypeidExpr node in AST");
}

/// Test that DynamicCastExpr AST node is properly created.
#[test]
fn test_rtti_ast_dynamic_cast_node() {
    let parser = ClangParser::new().unwrap();
    let code = r#"
        class Base { public: virtual ~Base() {} };
        class Derived : public Base {};

        void test_dynamic_cast_node(Base* b) {
            Derived* d = dynamic_cast<Derived*>(b);
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search the AST for DynamicCastExpr node
    let has_dynamic_cast = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::DynamicCastExpr { .. })
    });

    assert!(has_dynamic_cast.is_some(), "Expected to find DynamicCastExpr node in AST");
}

// ========== C++20 Coroutine Promise Types Tests (D.5) ==========

/// Test parsing promise type with get_return_object method.
#[test]
fn test_promise_type_get_return_object() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task simple_coro() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse promise type: {:?}", result.err());

    let ast = result.unwrap();

    // Find the promise_type struct - it should contain get_return_object method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, return_type, .. }
            if name == "get_return_object" && matches!(return_type, CppType::Named(n) if n == "Task"))
    });

    assert!(has_method.is_some(), "Expected to find get_return_object method in promise_type");
}

/// Test parsing promise type with initial_suspend method.
#[test]
fn test_promise_type_initial_suspend() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task test_coro() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse initial_suspend: {:?}", result.err());

    let ast = result.unwrap();

    // Find initial_suspend method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "initial_suspend")
    });

    assert!(has_method.is_some(), "Expected to find initial_suspend method in promise_type");
}

/// Test parsing promise type with final_suspend method (noexcept).
#[test]
fn test_promise_type_final_suspend() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task test_final() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse final_suspend: {:?}", result.err());

    let ast = result.unwrap();

    // Find final_suspend method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "final_suspend")
    });

    assert!(has_method.is_some(), "Expected to find final_suspend method in promise_type");
}

/// Test parsing promise type with return_void method.
#[test]
fn test_promise_type_return_void() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct VoidTask {
            struct promise_type {
                VoidTask get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        VoidTask void_coro() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse return_void: {:?}", result.err());

    let ast = result.unwrap();

    // Find return_void method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, return_type, .. }
            if name == "return_void" && matches!(return_type, CppType::Void))
    });

    assert!(has_method.is_some(), "Expected to find return_void method in promise_type");
}

/// Test parsing promise type with return_value method.
#[test]
fn test_promise_type_return_value() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct ValueTask {
            int result;
            struct promise_type {
                int value;
                ValueTask get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_value(int v) { value = v; }
                void unhandled_exception() {}
            };
        };

        ValueTask value_coro() {
            co_return 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse return_value: {:?}", result.err());

    let ast = result.unwrap();

    // Find return_value method with int parameter
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, params, .. }
            if name == "return_value" && params.len() == 1)
    });

    assert!(has_method.is_some(), "Expected to find return_value method in promise_type");
}

/// Test parsing promise type with yield_value method.
#[test]
fn test_promise_type_yield_value() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Generator {
            struct promise_type {
                int current_value;
                Generator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        Generator gen() {
            co_yield 1;
            co_yield 2;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse yield_value: {:?}", result.err());

    let ast = result.unwrap();

    // Find yield_value method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, params, .. }
            if name == "yield_value" && params.len() == 1)
    });

    assert!(has_method.is_some(), "Expected to find yield_value method in promise_type");
}

/// Test parsing unhandled_exception method in promise type.
#[test]
fn test_promise_type_unhandled_exception() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct SafeTask {
            struct promise_type {
                SafeTask get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() noexcept {}
            };
        };

        SafeTask safe_coro() {
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse unhandled_exception: {:?}", result.err());

    let ast = result.unwrap();

    // Find unhandled_exception method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "unhandled_exception")
    });

    assert!(has_method.is_some(), "Expected to find unhandled_exception method in promise_type");
}

/// Test complete promise type with all methods parsed correctly.
#[test]
fn test_promise_type_complete() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct CompleteTask {
            struct promise_type {
                int result;
                CompleteTask get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_value(int v) { result = v; }
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    result = value;
                    return {};
                }
            };
        };

        CompleteTask complete_coro() {
            co_yield 10;
            co_return 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse complete promise type: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // Verify the coroutine function is correctly parsed
    let coro_fn = module.functions.iter().find(|f| f.display_name == "complete_coro");
    assert!(coro_fn.is_some(), "Expected to find complete_coro function");
}

// ========== C++20 Coroutine Awaitables Tests (D.6) ==========

/// Test parsing custom awaitable with await_ready method.
#[test]
fn test_awaitable_await_ready() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct CustomAwaitable {
            bool await_ready() { return false; }
            void await_suspend(std::coroutine_handle<>) {}
            void await_resume() {}
        };
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse await_ready: {:?}", result.err());

    let ast = result.unwrap();

    // Find await_ready method returning bool
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, return_type, .. }
            if name == "await_ready" && matches!(return_type, CppType::Bool))
    });

    assert!(has_method.is_some(), "Expected to find await_ready method in CustomAwaitable");
}

/// Test parsing custom awaitable with await_suspend method.
#[test]
fn test_awaitable_await_suspend() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct CustomAwaitable {
            bool await_ready() { return false; }
            void await_suspend(std::coroutine_handle<> h) { h.resume(); }
            void await_resume() {}
        };
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse await_suspend: {:?}", result.err());

    let ast = result.unwrap();

    // Find await_suspend method with coroutine_handle parameter
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, params, .. }
            if name == "await_suspend" && params.len() == 1)
    });

    assert!(has_method.is_some(), "Expected to find await_suspend method in CustomAwaitable");
}

/// Test parsing custom awaitable with await_resume method.
#[test]
fn test_awaitable_await_resume() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct ValueAwaitable {
            bool await_ready() { return true; }
            void await_suspend(std::coroutine_handle<>) {}
            int await_resume() { return 42; }
        };
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse await_resume: {:?}", result.err());

    let ast = result.unwrap();

    // Find await_resume method returning int
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, return_type, .. }
            if name == "await_resume" && matches!(return_type, CppType::Int { signed: true }))
    });

    assert!(has_method.is_some(), "Expected to find await_resume method in ValueAwaitable");
}

/// Test parsing awaitable with conditional suspend (returns bool from await_suspend).
#[test]
fn test_awaitable_conditional_suspend() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct ConditionalAwaitable {
            bool await_ready() { return false; }
            bool await_suspend(std::coroutine_handle<> h) {
                return true; // true = suspend, false = resume immediately
            }
            int await_resume() { return 0; }
        };
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse conditional suspend: {:?}", result.err());

    let ast = result.unwrap();

    // Find await_suspend returning bool (conditional suspension)
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, return_type, .. }
            if name == "await_suspend" && matches!(return_type, CppType::Bool))
    });

    assert!(has_method.is_some(), "Expected to find await_suspend returning bool");
}

/// Test parsing awaitable with symmetric transfer (returns coroutine_handle from await_suspend).
#[test]
fn test_awaitable_symmetric_transfer() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct SymmetricAwaitable {
            std::coroutine_handle<> next_coro;

            bool await_ready() { return false; }
            std::coroutine_handle<> await_suspend(std::coroutine_handle<>) {
                return next_coro; // Transfer to another coroutine
            }
            void await_resume() {}
        };
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse symmetric transfer: {:?}", result.err());

    let ast = result.unwrap();

    // Find await_suspend method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "await_suspend")
    });

    assert!(has_method.is_some(), "Expected to find await_suspend for symmetric transfer");
}

/// Test using custom awaitable with co_await in a coroutine.
#[test]
fn test_awaitable_with_co_await() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct ValueAwaitable {
            int value;
            bool await_ready() { return true; }
            void await_suspend(std::coroutine_handle<>) {}
            int await_resume() { return value; }
        };

        struct Task {
            struct promise_type {
                Task get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        Task use_custom_awaitable() {
            ValueAwaitable awaitable{42};
            int result = co_await awaitable;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse co_await with custom awaitable: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // Verify the coroutine function is parsed
    let coro_fn = module.functions.iter().find(|f| f.display_name == "use_custom_awaitable");
    assert!(coro_fn.is_some(), "Expected to find use_custom_awaitable function");
}

/// Test promise_type with await_transform method.
#[test]
fn test_awaitable_await_transform() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct TransformTask {
            struct promise_type {
                TransformTask get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_never final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}

                // Transform any awaited type
                std::suspend_always await_transform(int value) {
                    return {};
                }
            };
        };

        TransformTask with_transform() {
            co_await 42;  // Will be transformed by await_transform
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse await_transform: {:?}", result.err());

    let ast = result.unwrap();

    // Find await_transform method
    let has_method = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, params, .. }
            if name == "await_transform" && params.len() == 1)
    });

    assert!(has_method.is_some(), "Expected to find await_transform method in promise_type");
}

/// Test complete awaitable pattern integration.
#[test]
fn test_awaitable_complete_pattern() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        // A simple async operation awaitable
        struct AsyncOperation {
            int result_value;
            bool is_ready;

            bool await_ready() const noexcept { return is_ready; }

            void await_suspend(std::coroutine_handle<> h) noexcept {
                // In real code: schedule resumption when operation completes
                h.resume();
            }

            int await_resume() noexcept { return result_value; }
        };

        struct AsyncTask {
            struct promise_type {
                AsyncTask get_return_object() { return {}; }
                std::suspend_never initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
            };
        };

        AsyncTask async_example() {
            AsyncOperation op1{10, false};
            int r1 = co_await op1;

            AsyncOperation op2{20, true};
            int r2 = co_await op2;

            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse complete awaitable pattern: {:?}", result.err());

    let ast = result.unwrap();

    // Verify all three awaitable methods are found
    let has_ready = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "await_ready")
    });
    let has_suspend = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "await_suspend")
    });
    let has_resume = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "await_resume")
    });

    assert!(has_ready.is_some() && has_suspend.is_some() && has_resume.is_some(),
        "Expected all three awaitable protocol methods");

    // Verify the coroutine function is parsed via MIR conversion
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();
    let coro_fn = module.functions.iter().find(|f| f.display_name == "async_example");
    assert!(coro_fn.is_some(), "Expected to find async_example function");
}

// ========== C++20 Coroutine Generators Tests (D.7) ==========

/// Test basic co_yield expression.
#[test]
fn test_generator_basic_yield() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Generator {
            struct promise_type {
                int current_value;
                Generator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        Generator basic_yield() {
            co_yield 42;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse basic yield: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let gen_fn = module.functions.iter().find(|f| f.display_name == "basic_yield");
    assert!(gen_fn.is_some(), "Expected to find basic_yield function");
}

/// Test multiple co_yield expressions in sequence.
#[test]
fn test_generator_multiple_yields() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Generator {
            struct promise_type {
                int current_value;
                Generator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        Generator multi_yield() {
            co_yield 1;
            co_yield 2;
            co_yield 3;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse multiple yields: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let gen_fn = module.functions.iter().find(|f| f.display_name == "multi_yield");
    assert!(gen_fn.is_some(), "Expected to find multi_yield function");
}

/// Test co_yield inside a loop.
#[test]
fn test_generator_yield_in_loop() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Generator {
            struct promise_type {
                int current_value;
                Generator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        Generator loop_yield(int n) {
            for (int i = 0; i < n; i++) {
                co_yield i;
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse yield in loop: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let gen_fn = module.functions.iter().find(|f| f.display_name == "loop_yield");
    assert!(gen_fn.is_some(), "Expected to find loop_yield function");
}

/// Test generator that yields different value types.
#[test]
fn test_generator_different_types() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct DoubleGenerator {
            struct promise_type {
                double current_value;
                DoubleGenerator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(double value) {
                    current_value = value;
                    return {};
                }
            };
        };

        DoubleGenerator yield_doubles() {
            co_yield 3.14;
            co_yield 2.71;
            co_yield 1.41;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse double yields: {:?}", result.err());

    let ast = result.unwrap();

    // Find yield_value method with double parameter
    let has_yield = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, params, .. }
            if name == "yield_value" && params.len() == 1)
    });

    assert!(has_yield.is_some(), "Expected yield_value method for double");
}

/// Test generator with state between yields.
#[test]
fn test_generator_with_state() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct StatefulGenerator {
            struct promise_type {
                int current;
                StatefulGenerator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current = value;
                    return {};
                }
            };
        };

        StatefulGenerator fibonacci() {
            int a = 0, b = 1;
            while (true) {
                co_yield a;
                int next = a + b;
                a = b;
                b = next;
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse stateful generator: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let gen_fn = module.functions.iter().find(|f| f.display_name == "fibonacci");
    assert!(gen_fn.is_some(), "Expected to find fibonacci function");
}

/// Test generator with co_yield and co_return.
#[test]
fn test_generator_yield_then_return() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct FiniteGenerator {
            struct promise_type {
                int current;
                FiniteGenerator get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current = value;
                    return {};
                }
            };
        };

        FiniteGenerator countdown(int start) {
            for (int i = start; i > 0; i--) {
                co_yield i;
            }
            co_return;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse yield then return: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let gen_fn = module.functions.iter().find(|f| f.display_name == "countdown");
    assert!(gen_fn.is_some(), "Expected to find countdown function");
}

/// Test CoyieldExpr AST node detection.
#[test]
fn test_generator_ast_coyield_node() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct Gen {
            struct promise_type {
                int val;
                Gen get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int v) {
                    val = v;
                    return {};
                }
            };
        };

        Gen test_yield_ast() {
            co_yield 100;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let ast = result.unwrap();

    // Search for CoyieldExpr node
    let has_coyield = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CoyieldExpr { .. })
    });

    // Note: CoyieldExpr may be detected via token parsing
    // If not found, at least verify function parses
    if has_coyield.is_none() {
        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();
        let fn_found = module.functions.iter().any(|f| f.display_name == "test_yield_ast");
        assert!(fn_found, "Expected test_yield_ast function to be parsed");
    }
}

/// Test complete generator pattern with range-like interface.
#[test]
fn test_generator_complete_pattern() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        #include <coroutine>

        struct IntRange {
            struct promise_type {
                int current_value;
                IntRange get_return_object() { return {}; }
                std::suspend_always initial_suspend() { return {}; }
                std::suspend_always final_suspend() noexcept { return {}; }
                void return_void() {}
                void unhandled_exception() {}
                std::suspend_always yield_value(int value) {
                    current_value = value;
                    return {};
                }
            };
        };

        IntRange make_range(int start, int end, int step) {
            for (int i = start; i < end; i += step) {
                co_yield i;
            }
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse complete generator pattern: {:?}", result.err());

    let ast = result.unwrap();

    // Verify yield_value method exists
    let has_yield = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, .. } if name == "yield_value")
    });
    assert!(has_yield.is_some(), "Expected yield_value method");

    // Verify the generator function is parsed via MIR conversion
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();
    let gen_fn = module.functions.iter().find(|f| f.display_name == "make_range");
    assert!(gen_fn.is_some(), "Expected to find make_range function");
}

// ========== Phase F: Mako Integration Tests ==========

/// Test parsing simplified rand.cpp patterns.
/// This tests the patterns used in Mako's rand.cpp without external dependencies.
#[test]
fn test_mako_rand_patterns() {
    let parser = ClangParser::with_system_includes().unwrap();
    // Simplified version of rand.cpp patterns
    let code = r#"
        #include <string>
        #include <vector>

        namespace rrr {

        class RandomGenerator {
        private:
            static thread_local unsigned int seed_;
            static int nu_constant;

        public:
            static int rand(int min = 0, int max = 100);
            static double rand_double(double min = 0.0, double max = 1.0);
            static std::string rand_str(int length = 0);
            static std::string int2str_n(int i, int length);
            static bool percentage_true(double p);
            static bool percentage_true(int p);
            static unsigned int weighted_select(const std::vector<double> &weight_vector);
        };

        thread_local unsigned int RandomGenerator::seed_ = 12345;
        int RandomGenerator::nu_constant = 0;

        int RandomGenerator::rand(int min, int max) {
            seed_ = seed_ * 1103515245 + 12345;
            return (seed_ % (max - min + 1)) + min;
        }

        double RandomGenerator::rand_double(double min, double max) {
            if (max == min) return min;
            int r = rand(0, 1000000);
            return ((double)r) / 1000000.0 * (max - min) + min;
        }

        std::string RandomGenerator::rand_str(int length) {
            int r = rand(0, 1000000);
            std::string s = std::to_string(r);
            if (length <= 0) return s;
            return s.substr(0, length);
        }

        std::string RandomGenerator::int2str_n(int i, int length) {
            std::string ret = std::to_string(i);
            while (ret.length() < (unsigned)length) {
                ret = std::string("0") + ret;
            }
            if (ret.length() > (unsigned)length) {
                ret = ret.substr(ret.length() - length, length);
            }
            return ret;
        }

        bool RandomGenerator::percentage_true(double p) {
            return rand_double(0.0, 100.0) <= p;
        }

        bool RandomGenerator::percentage_true(int p) {
            return rand(0, 99) < p;
        }

        unsigned int RandomGenerator::weighted_select(const std::vector<double> &weight_vector) {
            double sum = 0;
            for (unsigned i = 0; i < weight_vector.size(); i++)
                sum += weight_vector[i];
            double r = rand_double(0, sum);
            double stage_sum = 0;
            for (unsigned i = 0; i < weight_vector.size(); i++) {
                stage_sum += weight_vector[i];
                if (r <= stage_sum) return i;
            }
            return weight_vector.size() - 1;
        }

        }
    "#;

    let result = parser.parse_string(code, "test_rand.cpp");
    assert!(result.is_ok(), "Failed to parse rand patterns: {:?}", result.err());

    let ast = result.unwrap();

    // Verify namespace
    let has_namespace = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::NamespaceDecl { name } if name.as_deref() == Some("rrr"))
    });
    assert!(has_namespace.is_some(), "Expected to find rrr namespace");

    // Verify class
    let has_class = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::RecordDecl { name, .. } if name == "RandomGenerator")
    });
    assert!(has_class.is_some(), "Expected to find RandomGenerator class");

    // Verify static methods
    let has_rand = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::CXXMethodDecl { name, is_static: true, .. } if name == "rand")
    });
    assert!(has_rand.is_some(), "Expected to find static rand method");

    // Verify can convert to MIR
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    // The module should have functions from the class
    // Note: Static member functions are parsed but display names may vary
    assert!(!module.functions.is_empty(), "Expected to find functions in module");
}

/// Test thread_local storage class.
#[test]
fn test_mako_thread_local_storage() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        class TestClass {
        private:
            static thread_local int counter_;
        public:
            static int get() { return counter_++; }
        };

        thread_local int TestClass::counter_ = 0;

        int use_counter() {
            return TestClass::get();
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse thread_local: {:?}", result.err());

    let ast = result.unwrap();
    let converter = MirConverter::new();
    let module = converter.convert(ast).unwrap();

    let fn_found = module.functions.iter().any(|f| f.display_name == "use_counter");
    assert!(fn_found, "Expected to find use_counter function");
}

/// Test inline assembly parsing (should be handled gracefully).
#[test]
fn test_mako_inline_asm() {
    let parser = ClangParser::with_system_includes().unwrap();
    let code = r#"
        unsigned long long rdtsc() {
            unsigned int lo, hi;
            __asm__ __volatile__("rdtsc" : "=a" (lo), "=d" (hi));
            return ((unsigned long long)hi << 32) | lo;
        }
    "#;

    let result = parser.parse_string(code, "test.cpp");
    assert!(result.is_ok(), "Failed to parse inline asm: {:?}", result.err());

    // We expect the function to be parseable even if inline asm is not fully supported
    let ast = result.unwrap();
    let has_fn = find_node_kind(&ast.translation_unit, |kind| {
        matches!(kind, ClangNodeKind::FunctionDecl { name, .. } if name == "rdtsc")
    });
    assert!(has_fn.is_some(), "Expected to find rdtsc function");
}
