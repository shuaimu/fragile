//! Direct AST to Rust source code generation.
//!
//! This module generates Rust source code directly from the Clang AST,
//! without going through an intermediate MIR representation.
//! This produces cleaner, more idiomatic Rust code.

use crate::ast::{ClangNode, ClangNodeKind, BinaryOp, UnaryOp, CastKind, ConstructorKind};
use crate::types::CppType;
use std::collections::HashSet;

/// Rust reserved keywords that need raw identifier syntax.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn",
    "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
    "abstract", "become", "box", "do", "final", "macro", "override",
    "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Rust code generator that works directly with Clang AST.
pub struct AstCodeGen {
    output: String,
    indent: usize,
    /// Track variable names that are declared as reference types
    ref_vars: HashSet<String>,
    /// Track variable names that are declared as pointer types
    ptr_vars: HashSet<String>,
    /// Track variable names that are declared as array types
    arr_vars: HashSet<String>,
}

impl AstCodeGen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            ref_vars: HashSet::new(),
            ptr_vars: HashSet::new(),
            arr_vars: HashSet::new(),
        }
    }

    /// Generate Rust source code from a Clang AST.
    pub fn generate(mut self, ast: &ClangNode) -> String {
        // File header
        self.writeln("#![allow(dead_code)]");
        self.writeln("#![allow(unused_variables)]");
        self.writeln("#![allow(unused_mut)]");
        self.writeln("#![allow(non_camel_case_types)]");
        self.writeln("#![allow(non_snake_case)]");
        self.writeln("");

        // Process translation unit
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            for child in &ast.children {
                self.generate_top_level(child);
            }
        }

        self.output
    }

    /// Generate Rust stubs (signatures only, no bodies) from a Clang AST.
    /// This is useful for FFI declarations and header generation.
    pub fn generate_stubs(mut self, ast: &ClangNode) -> String {
        // File header
        self.writeln("// Auto-generated Rust stubs from C++ code");
        self.writeln("#![allow(dead_code)]");
        self.writeln("#![allow(unused_variables)]");
        self.writeln("");

        // Process translation unit
        if let ClangNodeKind::TranslationUnit = &ast.kind {
            for child in &ast.children {
                self.generate_stub_top_level(child);
            }
        }

        self.output
    }

    /// Generate a top-level stub declaration (signatures only).
    fn generate_stub_top_level(&mut self, node: &ClangNode) {
        match &node.kind {
            ClangNodeKind::FunctionDecl { name, mangled_name, return_type, params, is_definition, .. } => {
                if *is_definition {
                    self.generate_function_stub(name, mangled_name, return_type, params);
                }
            }
            ClangNodeKind::RecordDecl { name, is_class, .. } => {
                self.generate_struct_stub(name, *is_class, &node.children);
            }
            ClangNodeKind::NamespaceDecl { .. } => {
                for child in &node.children {
                    self.generate_stub_top_level(child);
                }
            }
            _ => {}
        }
    }

    /// Generate a function stub (signature with placeholder body).
    fn generate_function_stub(&mut self, name: &str, mangled_name: &str, return_type: &CppType,
                              params: &[(String, CppType)]) {
        self.writeln(&format!("/// @fragile_cpp_mangled: {}", mangled_name));
        self.writeln(&format!("#[export_name = \"{}\"]", mangled_name));

        let params_str = params.iter()
            .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
            .collect::<Vec<_>>()
            .join(", ");

        let ret_str = if *return_type == CppType::Void {
            String::new()
        } else {
            format!(" -> {}", return_type.to_rust_type_str())
        };

        self.writeln(&format!("pub extern \"C\" fn {}({}){} {{", sanitize_identifier(name), params_str, ret_str));
        self.indent += 1;
        self.writeln("// Stub body - replaced by MIR injection at compile time");
        self.writeln("unreachable!(\"Fragile: C++ MIR should be injected\")");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a struct stub (fields only).
    fn generate_struct_stub(&mut self, name: &str, is_class: bool, children: &[ClangNode]) {
        let kind = if is_class { "class" } else { "struct" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));
        self.writeln("#[repr(C)]");
        self.writeln(&format!("pub struct {} {{", name));
        self.indent += 1;

        // First, embed base classes as fields
        for child in children {
            if let ClangNodeKind::CXXBaseSpecifier { base_type, access, .. } = &child.kind {
                if !matches!(access, crate::ast::AccessSpecifier::Private) {
                    let base_name = base_type.to_rust_type_str();
                    self.writeln(&format!("pub __base: {},", base_name));
                }
            }
        }

        // Then add derived class fields
        for child in children {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, .. } = &child.kind {
                let field_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                self.writeln(&format!("pub {}: {},", field_name, ty.to_rust_type_str()));
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    /// Generate a top-level declaration.
    fn generate_top_level(&mut self, node: &ClangNode) {
        match &node.kind {
            ClangNodeKind::FunctionDecl { name, mangled_name, return_type, params, is_definition, .. } => {
                if *is_definition {
                    self.generate_function(name, mangled_name, return_type, params, &node.children);
                }
            }
            ClangNodeKind::RecordDecl { name, is_class, .. } => {
                self.generate_struct(name, *is_class, &node.children);
            }
            ClangNodeKind::NamespaceDecl { .. } => {
                // Process namespace contents
                for child in &node.children {
                    self.generate_top_level(child);
                }
            }
            _ => {}
        }
    }

    /// Generate a function definition.
    fn generate_function(&mut self, name: &str, mangled_name: &str, return_type: &CppType,
                         params: &[(String, CppType)], children: &[ClangNode]) {
        // Special handling for C++ main function
        let is_main = name == "main" && params.is_empty();
        let func_name = if is_main { "cpp_main" } else { name };

        // Doc comment
        self.writeln(&format!("/// C++ function `{}`", name));
        self.writeln(&format!("/// Mangled: `{}`", mangled_name));

        // Track reference, pointer, and array parameters - clear any from previous function
        self.ref_vars.clear();
        self.ptr_vars.clear();
        self.arr_vars.clear();
        for (param_name, param_type) in params {
            if matches!(param_type, CppType::Reference { .. }) {
                self.ref_vars.insert(param_name.clone());
            }
            // Unsized arrays in function parameters are actually pointers in C++
            // (int arr[] is equivalent to int* arr)
            if matches!(param_type, CppType::Pointer { .. })
                || matches!(param_type, CppType::Array { size: None, .. }) {
                self.ptr_vars.insert(param_name.clone());
            }
            // Only track sized arrays as arrays
            if matches!(param_type, CppType::Array { size: Some(_), .. }) {
                self.arr_vars.insert(param_name.clone());
            }
        }

        // Function signature
        let params_str = params.iter()
            .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
            .collect::<Vec<_>>()
            .join(", ");

        let ret_str = if *return_type == CppType::Void {
            String::new()
        } else {
            format!(" -> {}", return_type.to_rust_type_str())
        };

        self.writeln(&format!("pub fn {}({}){} {{", sanitize_identifier(func_name), params_str, ret_str));
        self.indent += 1;

        // Find the compound statement (function body)
        for child in children {
            if let ClangNodeKind::CompoundStmt = &child.kind {
                self.generate_block_contents(&child.children, return_type);
            }
        }

        self.indent -= 1;
        self.writeln("}");
        self.writeln("");

        // Generate Rust main wrapper for C++ main
        if is_main {
            self.writeln("fn main() {");
            self.indent += 1;
            self.writeln("std::process::exit(cpp_main());");
            self.indent -= 1;
            self.writeln("}");
            self.writeln("");
        }
    }

    /// Generate struct definition.
    fn generate_struct(&mut self, name: &str, is_class: bool, children: &[ClangNode]) {
        let kind = if is_class { "class" } else { "struct" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));
        self.writeln("#[repr(C)]");
        self.writeln("#[derive(Default)]");
        self.writeln(&format!("pub struct {} {{", name));
        self.indent += 1;

        // First, embed base classes as fields (for single inheritance)
        // Base classes must come first to maintain C++ memory layout
        for child in children {
            if let ClangNodeKind::CXXBaseSpecifier { base_type, access, .. } = &child.kind {
                // Only include public/protected bases (private inheritance is more complex)
                if !matches!(access, crate::ast::AccessSpecifier::Private) {
                    let base_name = base_type.to_rust_type_str();
                    self.writeln(&format!("/// Inherited from `{}`", base_name));
                    self.writeln(&format!("pub __base: {},", base_name));
                }
            }
        }

        // Then collect derived class fields
        for child in children {
            if let ClangNodeKind::FieldDecl { name: field_name, ty, .. } = &child.kind {
                let field_name = if field_name.is_empty() {
                    "_field".to_string()
                } else {
                    sanitize_identifier(field_name)
                };
                self.writeln(&format!("pub {}: {},", field_name, ty.to_rust_type_str()));
            }
        }

        self.indent -= 1;
        self.writeln("}");

        // Check if there's an explicit default constructor (0 params)
        let has_default_ctor = children.iter().any(|c| {
            matches!(&c.kind, ClangNodeKind::ConstructorDecl { params, is_definition: true, .. } if params.is_empty())
        });

        // Generate impl block for methods
        let methods: Vec<_> = children.iter().filter(|c| {
            matches!(&c.kind, ClangNodeKind::CXXMethodDecl { is_definition: true, .. } |
                              ClangNodeKind::ConstructorDecl { is_definition: true, .. })
        }).collect();

        // Always generate impl block if we need new_0 or have other methods
        if !methods.is_empty() || !has_default_ctor {
            self.writeln("");
            self.writeln(&format!("impl {} {{", name));
            self.indent += 1;

            // Generate default new_0() if no explicit default constructor
            if !has_default_ctor {
                self.writeln("pub fn new_0() -> Self {");
                self.indent += 1;
                self.writeln("Default::default()");
                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }

            for method in methods {
                self.generate_method(method, name);
            }

            self.indent -= 1;
            self.writeln("}");
        }

        // Generate Drop impl if there's a destructor
        for child in children {
            if let ClangNodeKind::DestructorDecl { is_definition: true, .. } = &child.kind {
                self.writeln("");
                self.writeln(&format!("impl Drop for {} {{", name));
                self.indent += 1;
                self.writeln("fn drop(&mut self) {");
                self.indent += 1;
                // Find the destructor body
                for dtor_child in &child.children {
                    if let ClangNodeKind::CompoundStmt = &dtor_child.kind {
                        self.generate_block_contents(&dtor_child.children, &CppType::Void);
                    }
                }
                self.indent -= 1;
                self.writeln("}");
                self.indent -= 1;
                self.writeln("}");
                break; // Only one destructor per class
            }
        }

        // Generate Clone impl if there's a copy constructor
        for child in children {
            if let ClangNodeKind::ConstructorDecl { ctor_kind: ConstructorKind::Copy, is_definition: true, .. } = &child.kind {
                self.writeln("");
                self.writeln(&format!("impl Clone for {} {{", name));
                self.indent += 1;
                self.writeln("fn clone(&self) -> Self {");
                self.indent += 1;
                // Copy constructor is always new_1 (takes one argument: const T&)
                self.writeln("Self::new_1(self)");
                self.indent -= 1;
                self.writeln("}");
                self.indent -= 1;
                self.writeln("}");
                break; // Only one copy constructor per class
            }
        }

        self.writeln("");
    }

    /// Check if a method modifies self (has assignments to member fields).
    fn method_modifies_self(node: &ClangNode) -> bool {
        // Check if this node is an assignment to a member
        if let ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } = &node.kind {
            // Left side of assignment - check if it's a member expression
            if !node.children.is_empty() {
                if Self::is_member_access(&node.children[0]) {
                    return true;
                }
            }
        }
        // Also check compound assignment operators
        if let ClangNodeKind::BinaryOperator { op, .. } = &node.kind {
            match op {
                BinaryOp::AddAssign | BinaryOp::SubAssign | BinaryOp::MulAssign |
                BinaryOp::DivAssign | BinaryOp::RemAssign | BinaryOp::AndAssign |
                BinaryOp::OrAssign | BinaryOp::XorAssign | BinaryOp::ShlAssign |
                BinaryOp::ShrAssign => {
                    if !node.children.is_empty() {
                        if Self::is_member_access(&node.children[0]) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        // Recursively check children
        for child in &node.children {
            if Self::method_modifies_self(child) {
                return true;
            }
        }
        false
    }

    /// Check if a node is a member access (directly or through implicit this).
    fn is_member_access(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::MemberExpr { .. } => true,
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Check through implicit casts
                !node.children.is_empty() && Self::is_member_access(&node.children[0])
            }
            _ => false,
        }
    }

    /// Extract member assignments from a constructor body.
    /// Looks for patterns like `this->field = value;` or `field = value;`
    fn extract_member_assignments(node: &ClangNode, initializers: &mut Vec<(String, String)>, codegen: &AstCodeGen) {
        for child in &node.children {
            // Look for ExprStmt containing BinaryOperator with Assign
            if let ClangNodeKind::ExprStmt = &child.kind {
                if !child.children.is_empty() {
                    Self::extract_assignment(&child.children[0], initializers, codegen);
                }
            } else if let ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } = &child.kind {
                Self::extract_assignment(child, initializers, codegen);
            }
            // Recursively check compound statements
            if let ClangNodeKind::CompoundStmt = &child.kind {
                Self::extract_member_assignments(child, initializers, codegen);
            }
        }
    }

    /// Extract a single member assignment from a BinaryOperator node.
    fn extract_assignment(node: &ClangNode, initializers: &mut Vec<(String, String)>, codegen: &AstCodeGen) {
        if let ClangNodeKind::BinaryOperator { op: BinaryOp::Assign, .. } = &node.kind {
            if node.children.len() >= 2 {
                // Get member name from left side
                if let Some(member_name) = Self::get_member_name(&node.children[0]) {
                    // Get value from right side
                    let value = codegen.expr_to_string(&node.children[1]);
                    initializers.push((member_name, value));
                }
            }
        }
    }

    /// Get member name from a member expression (possibly wrapped in casts).
    fn get_member_name(node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::MemberExpr { member_name, .. } => Some(member_name.clone()),
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    Self::get_member_name(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if a node is a pointer dereference (possibly wrapped in casts).
    fn is_pointer_deref(node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::UnaryOperator { op: UnaryOp::Deref, .. } => true,
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && Self::is_pointer_deref(&node.children[0])
            }
            _ => false,
        }
    }

    /// Check if a node is an array subscript on a pointer (needs unsafe for assignment).
    fn is_pointer_subscript(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                if !node.children.is_empty() {
                    // Check if the array expression is a pointer type
                    let arr_type = Self::get_expr_type(&node.children[0]);
                    matches!(arr_type, Some(CppType::Pointer { .. }))
                        || matches!(arr_type, Some(CppType::Array { size: None, .. }))
                        || self.is_ptr_var_expr(&node.children[0])
                } else {
                    false
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                !node.children.is_empty() && self.is_pointer_subscript(&node.children[0])
            }
            _ => false,
        }
    }

    /// Get the raw identifier for a reference variable expression (without dereferencing).
    /// Returns None if not a reference variable expression.
    fn get_ref_var_ident(&self, node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => {
                if self.ref_vars.contains(name) {
                    Some(sanitize_identifier(name))
                } else {
                    None
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                if !node.children.is_empty() {
                    self.get_ref_var_ident(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if an expression is a pointer variable (parameter or local with pointer type).
    fn is_ptr_var_expr(&self, node: &ClangNode) -> bool {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => {
                self.ptr_vars.contains(name)
            }
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look through casts and unknown wrappers
                !node.children.is_empty() && self.is_ptr_var_expr(&node.children[0])
            }
            _ => {
                // Also check all children recursively for cases where the structure differs
                node.children.iter().any(|c| self.is_ptr_var_expr(c))
            }
        }
    }

    /// Check if an expression is an array variable and get its identifier.
    fn get_array_var_ident(&self, node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, ty, .. } => {
                // Check both the type from AST and our tracked arrays
                if matches!(ty, CppType::Array { .. }) || self.arr_vars.contains(name) {
                    Some(sanitize_identifier(name))
                } else {
                    None
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::Unknown(_) => {
                // Look through casts and unknown wrappers
                if !node.children.is_empty() {
                    self.get_array_var_ident(&node.children[0])
                } else {
                    None
                }
            }
            _ => {
                // Also check children recursively
                for child in &node.children {
                    if let Some(ident) = self.get_array_var_ident(child) {
                        return Some(ident);
                    }
                }
                None
            }
        }
    }

    /// Get the type of an expression node.
    fn get_expr_type(node: &ClangNode) -> Option<CppType> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::BinaryOperator { ty, .. } => Some(ty.clone()),
            ClangNodeKind::UnaryOperator { ty, .. } => Some(ty.clone()),
            ClangNodeKind::MemberExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::CallExpr { ty } => Some(ty.clone()),
            ClangNodeKind::ImplicitCastExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::CastExpr { ty, .. } => Some(ty.clone()),
            ClangNodeKind::ArraySubscriptExpr { ty } => Some(ty.clone()),
            ClangNodeKind::ParmVarDecl { ty, .. } => Some(ty.clone()),
            // For unknown or wrapper nodes, look through to children
            ClangNodeKind::Unknown(_) | ClangNodeKind::ParenExpr { .. } => {
                if !node.children.is_empty() {
                    Self::get_expr_type(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Extract the class name from a type, handling const qualifiers, references, and pointers.
    /// For example, "const Point" -> "Point", Reference { pointee: Named("Point") } -> "Point"
    fn extract_class_name(ty: &Option<CppType>) -> Option<String> {
        ty.as_ref().and_then(|t| Self::extract_class_name_from_type(t))
    }

    /// Helper to extract class name from a CppType.
    fn extract_class_name_from_type(ty: &CppType) -> Option<String> {
        match ty {
            CppType::Named(name) => {
                // Strip "const " prefix if present
                let stripped = name.strip_prefix("const ").unwrap_or(name);
                Some(stripped.to_string())
            }
            CppType::Reference { referent, .. } => Self::extract_class_name_from_type(referent),
            CppType::Pointer { pointee, .. } => Self::extract_class_name_from_type(pointee),
            _ => None,
        }
    }

    /// Get function parameter types from a function reference node.
    fn get_function_param_types(node: &ClangNode) -> Option<Vec<CppType>> {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { ty, .. } => {
                if let CppType::Function { params, .. } = ty {
                    Some(params.clone())
                } else {
                    None
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Look through casts (e.g., FunctionToPointerDecay)
                if !node.children.is_empty() {
                    Self::get_function_param_types(&node.children[0])
                } else {
                    None
                }
            }
            ClangNodeKind::Unknown(_) => {
                // Unknown nodes (like UnexposedExpr) may wrap DeclRefExpr, recurse
                if !node.children.is_empty() {
                    Self::get_function_param_types(&node.children[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Generate a method or constructor.
    fn generate_method(&mut self, node: &ClangNode, _struct_name: &str) {
        match &node.kind {
            ClangNodeKind::CXXMethodDecl { name, return_type, params, is_static, .. } => {
                let self_param = if *is_static {
                    "".to_string()
                } else {
                    // Check if method modifies self
                    let modifies_self = Self::method_modifies_self(node);
                    if modifies_self { "&mut self, ".to_string() } else { "&self, ".to_string() }
                };
                let params_str = params.iter()
                    .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
                    .collect::<Vec<_>>()
                    .join(", ");

                let ret_str = if *return_type == CppType::Void {
                    String::new()
                } else {
                    format!(" -> {}", return_type.to_rust_type_str())
                };

                self.writeln(&format!("pub fn {}({}{}){} {{",
                    sanitize_identifier(name), self_param, params_str, ret_str));
                self.indent += 1;

                // Find body
                for child in &node.children {
                    if let ClangNodeKind::CompoundStmt = &child.kind {
                        self.generate_block_contents(&child.children, return_type);
                    }
                }

                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
            ClangNodeKind::ConstructorDecl { params, .. } => {
                // Always use new_N format (new_0, new_1, new_2) for consistency
                let fn_name = format!("new_{}", params.len());

                let params_str = params.iter()
                    .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
                    .collect::<Vec<_>>()
                    .join(", ");

                self.writeln(&format!("pub fn {}({}) -> Self {{", fn_name, params_str));
                self.indent += 1;

                // Extract member initializers from constructor children
                // Pattern 1: MemberRef { name } followed by initialization expression (member initializer list)
                // Pattern 2: CompoundStmt with assignments to member fields (body assignments)
                let mut initializers: Vec<(String, String)> = Vec::new();
                let mut i = 0;
                while i < node.children.len() {
                    if let ClangNodeKind::MemberRef { name } = &node.children[i].kind {
                        // Next sibling should be the initializer expression
                        let init_val = if i + 1 < node.children.len() {
                            i += 1;
                            self.expr_to_string(&node.children[i])
                        } else {
                            "Default::default()".to_string()
                        };
                        initializers.push((name.clone(), init_val));
                    } else if let ClangNodeKind::CompoundStmt = &node.children[i].kind {
                        // Look for assignments in constructor body
                        Self::extract_member_assignments(&node.children[i], &mut initializers, self);
                    }
                    i += 1;
                }

                self.writeln("Self {");
                self.indent += 1;
                if initializers.is_empty() {
                    // Default constructor - use Default
                    self.writeln("..Default::default()");
                } else {
                    for (field, value) in &initializers {
                        self.writeln(&format!("{}: {},", sanitize_identifier(field), value));
                    }
                    // Fill in remaining fields with default
                    self.writeln("..Default::default()");
                }
                self.indent -= 1;
                self.writeln("}");

                self.indent -= 1;
                self.writeln("}");
                self.writeln("");
            }
            _ => {}
        }
    }

    /// Generate the contents of a block (compound statement).
    fn generate_block_contents(&mut self, stmts: &[ClangNode], return_type: &CppType) {
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == len - 1;
            self.generate_stmt(stmt, is_last && *return_type != CppType::Void);
        }
    }

    /// Generate a statement.
    fn generate_stmt(&mut self, node: &ClangNode, is_tail_expr: bool) {
        match &node.kind {
            ClangNodeKind::DeclStmt => {
                // Variable declaration
                for child in &node.children {
                    if let ClangNodeKind::VarDecl { name, ty, .. } = &child.kind {
                        // Check if this is a reference, array, or pointer type
                        let is_ref = matches!(ty, CppType::Reference { .. });
                        let is_const_ref = matches!(ty, CppType::Reference { is_const: true, .. });
                        let is_array = matches!(ty, CppType::Array { .. });
                        let is_ptr = matches!(ty, CppType::Pointer { .. });

                        // Track typed variables for later
                        if is_ref {
                            self.ref_vars.insert(name.clone());
                        }
                        if is_array {
                            self.arr_vars.insert(name.clone());
                        }
                        if is_ptr {
                            self.ptr_vars.insert(name.clone());
                        }

                        // Find the actual initializer, skipping TypeRef and Unknown("TypeRef") nodes
                        let initializer = child.children.iter().find(|c| {
                            !matches!(&c.kind, ClangNodeKind::Unknown(s) if s == "TypeRef")
                                && !matches!(&c.kind, ClangNodeKind::Unknown(s) if s.contains("Type"))
                        });

                        // Check if we have a real initializer
                        let has_real_init = if let Some(init_node) = &initializer {
                            // For arrays with just an integer literal child, it might be the array size
                            if is_array {
                                !matches!(&init_node.kind, ClangNodeKind::IntegerLiteral { .. })
                            } else {
                                true
                            }
                        } else {
                            false
                        };

                        let init = if has_real_init {
                            let init_node = initializer.unwrap();
                            let expr = self.expr_to_string(init_node);
                            // If expression is unsupported, fall back to default
                            if expr.contains("unsupported") {
                                format!(" = {}", default_value_for_type(ty))
                            } else if is_ref {
                                // Reference initialization: add &mut or & prefix
                                let prefix = if is_const_ref { "&" } else { "&mut " };
                                format!(" = {}{}", prefix, expr)
                            } else {
                                format!(" = {}", expr)
                            }
                        } else {
                            format!(" = {}", default_value_for_type(ty))
                        };

                        // References don't need mut keyword
                        let mut_kw = if is_ref { "" } else { "mut " };
                        self.writeln(&format!("let {}{}: {}{};",
                            mut_kw, sanitize_identifier(name), ty.to_rust_type_str(), init));
                    }
                }
            }
            ClangNodeKind::ReturnStmt => {
                if node.children.is_empty() {
                    self.writeln("return;");
                } else {
                    let expr = self.expr_to_string(&node.children[0]);
                    self.writeln(&format!("return {};", expr));
                }
            }
            ClangNodeKind::IfStmt => {
                self.generate_if_stmt(node);
            }
            ClangNodeKind::WhileStmt => {
                self.generate_while_stmt(node);
            }
            ClangNodeKind::ForStmt => {
                self.generate_for_stmt(node);
            }
            ClangNodeKind::DoStmt => {
                self.generate_do_stmt(node);
            }
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                self.generate_block_contents(&node.children, &CppType::Void);
                self.indent -= 1;
                self.writeln("}");
            }
            ClangNodeKind::ExprStmt => {
                if !node.children.is_empty() {
                    let expr = self.expr_to_string(&node.children[0]);
                    if is_tail_expr {
                        self.writeln(&expr);
                    } else {
                        self.writeln(&format!("{};", expr));
                    }
                }
            }
            ClangNodeKind::BreakStmt => {
                self.writeln("break;");
            }
            ClangNodeKind::ContinueStmt => {
                self.writeln("continue;");
            }
            ClangNodeKind::TryStmt => {
                // try { ... } catch { ... } => match std::panic::catch_unwind(|| { ... })
                // Find the try body (first CompoundStmt) and catch handlers
                let mut try_body = None;
                let mut catch_handlers = Vec::new();

                for child in &node.children {
                    match &child.kind {
                        ClangNodeKind::CompoundStmt => {
                            if try_body.is_none() {
                                try_body = Some(child);
                            }
                        }
                        ClangNodeKind::CatchStmt { .. } => {
                            catch_handlers.push(child);
                        }
                        _ => {}
                    }
                }

                if let Some(body) = try_body {
                    // Generate: match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { ... }))
                    self.writeln("match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {");
                    self.indent += 1;
                    self.generate_block_contents(&body.children, &CppType::Void);
                    self.indent -= 1;
                    self.writeln("})) {");
                    self.indent += 1;
                    self.writeln("Ok(result) => result,");
                    self.writeln("Err(_e) => {");
                    self.indent += 1;

                    // Generate catch handler body (use first catch handler if any)
                    if let Some(catch) = catch_handlers.first() {
                        for catch_child in &catch.children {
                            if let ClangNodeKind::CompoundStmt = &catch_child.kind {
                                self.generate_block_contents(&catch_child.children, &CppType::Void);
                            }
                        }
                    } else {
                        self.writeln("// No catch handler");
                    }

                    self.indent -= 1;
                    self.writeln("}");
                    self.indent -= 1;
                    self.writeln("}");
                }
            }
            ClangNodeKind::CatchStmt { .. } => {
                // Handled as part of TryStmt
            }
            _ => {
                // For expressions at statement level
                let expr = self.expr_to_string(node);
                if is_tail_expr {
                    self.writeln(&expr);
                } else if !expr.is_empty() {
                    self.writeln(&format!("{};", expr));
                }
            }
        }
    }

    /// Generate an if statement.
    fn generate_if_stmt(&mut self, node: &ClangNode) {
        // Children: condition, then-branch, [else-branch]
        if node.children.len() >= 2 {
            let cond = self.expr_to_string(&node.children[0]);
            self.writeln(&format!("if {} {{", cond));
            self.indent += 1;
            self.generate_stmt(&node.children[1], false);
            self.indent -= 1;

            if node.children.len() > 2 {
                // Check if else is another if (else if)
                if let ClangNodeKind::IfStmt = &node.children[2].kind {
                    self.write("} else ");
                    self.generate_if_stmt(&node.children[2]);
                    return;
                }
                self.writeln("} else {");
                self.indent += 1;
                self.generate_stmt(&node.children[2], false);
                self.indent -= 1;
            }
            self.writeln("}");
        }
    }

    /// Generate a while statement.
    fn generate_while_stmt(&mut self, node: &ClangNode) {
        // Children: condition, body
        if node.children.len() >= 2 {
            let cond = self.expr_to_string(&node.children[0]);
            self.writeln(&format!("while {} {{", cond));
            self.indent += 1;
            self.generate_stmt(&node.children[1], false);
            self.indent -= 1;
            self.writeln("}");
        }
    }

    /// Generate a do-while statement.
    fn generate_do_stmt(&mut self, node: &ClangNode) {
        // Children: body, condition
        // do { body } while (cond); => loop { body; if !cond { break; } }
        if node.children.len() >= 2 {
            self.writeln("loop {");
            self.indent += 1;
            // Body first (executes at least once)
            self.generate_stmt(&node.children[0], false);
            // Then condition check
            let cond = self.expr_to_string(&node.children[1]);
            self.writeln(&format!("if !({}) {{ break; }}", cond));
            self.indent -= 1;
            self.writeln("}");
        }
    }

    /// Generate a for statement.
    fn generate_for_stmt(&mut self, node: &ClangNode) {
        // C++ for loops: for (init; cond; inc) { body }
        // Convert to: { init; loop { if !cond { break; } body; inc; } }
        // This correctly handles continue (which should go to inc, then cond)
        // Children: [init], [cond], [inc], body

        self.writeln("{");
        self.indent += 1;

        if node.children.len() >= 4 {
            // Init
            self.generate_stmt(&node.children[0], false);

            // Get condition and increment
            let cond = if matches!(&node.children[1].kind, ClangNodeKind::IntegerLiteral { .. }) {
                "true".to_string()
            } else {
                self.expr_to_string(&node.children[1])
            };

            let inc = self.expr_to_string(&node.children[2]);

            // Use loop with break for condition to handle continue correctly
            self.writeln("loop {");
            self.indent += 1;

            // Condition check with break
            self.writeln(&format!("if !({}) {{ break; }}", cond));

            // Body - we need to handle continue specially
            // Generate body with continue handling
            self.generate_for_body(&node.children[3], &inc);

            // Increment at end (only reached if no continue/break)
            if !inc.is_empty() {
                self.writeln(&format!("{};", inc));
            }

            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("}");
    }

    /// Generate for loop body with special continue handling.
    /// Continue needs to run the increment before looping back.
    fn generate_for_body(&mut self, node: &ClangNode, inc: &str) {
        match &node.kind {
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                for stmt in &node.children {
                    self.generate_for_body_stmt(stmt, inc);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            ClangNodeKind::ContinueStmt => {
                // For continue in for loop: increment then continue
                if !inc.is_empty() {
                    self.writeln(&format!("{}; continue;", inc));
                } else {
                    self.writeln("continue;");
                }
            }
            _ => {
                self.generate_for_body_stmt(node, inc);
            }
        }
    }

    /// Generate a statement inside a for loop body, handling continue specially.
    fn generate_for_body_stmt(&mut self, node: &ClangNode, inc: &str) {
        match &node.kind {
            ClangNodeKind::ContinueStmt => {
                // For continue in for loop: increment then continue
                if !inc.is_empty() {
                    self.writeln(&format!("{}; continue;", inc));
                } else {
                    self.writeln("continue;");
                }
            }
            ClangNodeKind::CompoundStmt => {
                self.writeln("{");
                self.indent += 1;
                for stmt in &node.children {
                    self.generate_for_body_stmt(stmt, inc);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            ClangNodeKind::IfStmt => {
                // Need special handling for if statements containing continue
                self.generate_for_if_stmt(node, inc);
            }
            _ => {
                self.generate_stmt(node, false);
            }
        }
    }

    /// Generate if statement inside for loop body, handling continue in branches.
    fn generate_for_if_stmt(&mut self, node: &ClangNode, inc: &str) {
        if node.children.len() >= 2 {
            let cond = self.expr_to_string(&node.children[0]);
            self.writeln(&format!("if {} {{", cond));
            self.indent += 1;
            self.generate_for_body_stmt(&node.children[1], inc);
            self.indent -= 1;

            if node.children.len() > 2 {
                if let ClangNodeKind::IfStmt = &node.children[2].kind {
                    self.write("} else ");
                    self.generate_for_if_stmt(&node.children[2], inc);
                    return;
                }
                self.writeln("} else {");
                self.indent += 1;
                self.generate_for_body_stmt(&node.children[2], inc);
                self.indent -= 1;
            }
            self.writeln("}");
        }
    }

    /// Convert an expression node to a Rust string (without unsafe wrapping for derefs).
    /// Used inside unsafe blocks where we don't want nested unsafe.
    fn expr_to_string_raw(&self, node: &ClangNode) -> String {
        match &node.kind {
            ClangNodeKind::UnaryOperator { op, ty } => {
                if !node.children.is_empty() {
                    let operand = self.expr_to_string_raw(&node.children[0]);
                    match op {
                        UnaryOp::Deref => format!("*{}", operand),
                        UnaryOp::Minus => format!("-{}", operand),
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => format!("!{}", operand),
                        UnaryOp::Not => format!("!{}", operand),
                        UnaryOp::AddrOf => {
                            let rust_ty = ty.to_rust_type_str();
                            if rust_ty.starts_with("*mut ") {
                                format!("&mut {} as {}", operand, rust_ty)
                            } else if rust_ty.starts_with("*const ") {
                                format!("&{} as {}", operand, rust_ty)
                            } else {
                                format!("&{}", operand)
                            }
                        }
                        UnaryOp::PreInc => format!("{{ {} += 1; {} }}", operand, operand),
                        UnaryOp::PreDec => format!("{{ {} -= 1; {} }}", operand, operand),
                        UnaryOp::PostInc => format!("{{ let v = {}; {} += 1; v }}", operand, operand),
                        UnaryOp::PostDec => format!("{{ let v = {}; {} -= 1; v }}", operand, operand),
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Pass through casts
                if !node.children.is_empty() {
                    self.expr_to_string_raw(&node.children[0])
                } else {
                    "/* cast error */".to_string()
                }
            }
            ClangNodeKind::DeclRefExpr { name, .. } => {
                if name == "this" {
                    "self".to_string()
                } else {
                    sanitize_identifier(name)
                }
            }
            ClangNodeKind::IntegerLiteral { value, cpp_type } => {
                let suffix = match cpp_type {
                    Some(CppType::Int { signed: true }) => "i32",
                    Some(CppType::Int { signed: false }) => "u32",
                    Some(CppType::Long { signed: true }) => "i64",
                    Some(CppType::Long { signed: false }) => "u64",
                    _ => "i32",
                };
                format!("{}{}", value, suffix)
            }
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                // For array subscript in raw context (inside unsafe block),
                // generate pointer arithmetic without wrapping in unsafe
                if node.children.len() >= 2 {
                    let arr = self.expr_to_string_raw(&node.children[0]);
                    let idx = self.expr_to_string_raw(&node.children[1]);
                    // Check if the array expression is a pointer type
                    let arr_type = Self::get_expr_type(&node.children[0]);
                    let is_pointer = matches!(arr_type, Some(CppType::Pointer { .. }))
                        || matches!(arr_type, Some(CppType::Array { size: None, .. }))
                        || self.is_ptr_var_expr(&node.children[0]);
                    if is_pointer {
                        // Raw pointer indexing without unsafe wrapper
                        format!("*{}.add({} as usize)", arr, idx)
                    } else {
                        // Array indexing
                        format!("{}[{} as usize]", arr, idx)
                    }
                } else {
                    "/* array subscript error */".to_string()
                }
            }
            // For other expressions, use the regular conversion
            _ => self.expr_to_string(node),
        }
    }

    /// Convert an expression node to a Rust string.
    fn expr_to_string(&self, node: &ClangNode) -> String {
        match &node.kind {
            ClangNodeKind::IntegerLiteral { value, cpp_type } => {
                let suffix = match cpp_type {
                    Some(CppType::Int { signed: true }) => "i32",
                    Some(CppType::Int { signed: false }) => "u32",
                    Some(CppType::Long { signed: true }) => "i64",
                    Some(CppType::Long { signed: false }) => "u64",
                    Some(CppType::LongLong { signed: true }) => "i64",
                    Some(CppType::LongLong { signed: false }) => "u64",
                    Some(CppType::Short { signed: true }) => "i16",
                    Some(CppType::Short { signed: false }) => "u16",
                    Some(CppType::Char { signed: true }) => "i8",
                    Some(CppType::Char { signed: false }) => "u8",
                    _ => "i32",
                };
                format!("{}{}", value, suffix)
            }
            ClangNodeKind::FloatingLiteral { value, cpp_type } => {
                let suffix = match cpp_type {
                    Some(CppType::Float) => "f32",
                    _ => "f64",
                };
                format!("{}{}", value, suffix)
            }
            ClangNodeKind::BoolLiteral(b) => b.to_string(),
            ClangNodeKind::NullPtrLiteral => "std::ptr::null_mut()".to_string(),
            ClangNodeKind::CXXNewExpr { ty, is_array } => {
                if *is_array {
                    // new T[n] → allocate n elements and return raw pointer
                    // ty is the result type (T*), we need the element type (T)
                    let element_type = ty.pointee().unwrap_or(ty);
                    // Children[0] should be the size expression
                    let size_expr = if !node.children.is_empty() {
                        self.expr_to_string(&node.children[0])
                    } else {
                        "0".to_string()
                    };
                    let default_val = default_value_for_type(element_type);
                    // Use Vec to allocate, then leak it to get a raw pointer
                    format!("{{ let mut v: Vec<{}> = vec![{}; {} as usize]; let p = v.as_mut_ptr(); std::mem::forget(v); p }}",
                        element_type.to_rust_type_str(), default_val, size_expr)
                } else {
                    // new T(args) → Box::into_raw(Box::new(value))
                    let init = if !node.children.is_empty() {
                        // Constructor argument or initializer
                        self.expr_to_string(&node.children[0])
                    } else {
                        // Default value for type
                        default_value_for_type(ty)
                    };
                    format!("Box::into_raw(Box::new({}))", init)
                }
            }
            ClangNodeKind::CXXDeleteExpr { is_array } => {
                if *is_array {
                    // delete[] p → We can't safely deallocate without knowing the array size.
                    // For now, generate code that at least compiles but leaks memory.
                    // A proper implementation would track array sizes at allocation.
                    if !node.children.is_empty() {
                        let ptr = self.expr_to_string(&node.children[0]);
                        // Note: This is a memory leak. The Vec was forgotten during new[],
                        // and we don't have the size to reconstruct it.
                        // Generate a no-op that at least uses the pointer to avoid warnings.
                        format!("{{ let _ = {}; /* delete[] memory leak - size unknown */ }}", ptr)
                    } else {
                        "/* delete[] error: no pointer */".to_string()
                    }
                } else if !node.children.is_empty() {
                    // delete p → drop(unsafe { Box::from_raw(p) })
                    let ptr = self.expr_to_string(&node.children[0]);
                    format!("drop(unsafe {{ Box::from_raw({}) }})", ptr)
                } else {
                    "/* delete error */".to_string()
                }
            }
            ClangNodeKind::StringLiteral(s) => format!("\"{}\"", s.escape_default()),
            ClangNodeKind::DeclRefExpr { name, .. } => {
                if name == "this" {
                    "self".to_string()
                } else {
                    let ident = sanitize_identifier(name);
                    // Dereference reference variables (parameters or locals with & type)
                    if self.ref_vars.contains(name) {
                        format!("*{}", ident)
                    } else {
                        ident
                    }
                }
            }
            ClangNodeKind::CXXThisExpr { .. } => "self".to_string(),
            ClangNodeKind::BinaryOperator { op, .. } => {
                if node.children.len() >= 2 {
                    let op_str = binop_to_string(op);

                    // Check if left side is a pointer dereference or pointer subscript
                    // (needs whole assignment in unsafe)
                    let left_is_deref = Self::is_pointer_deref(&node.children[0]);
                    let left_is_ptr_subscript = self.is_pointer_subscript(&node.children[0]);
                    let needs_unsafe = left_is_deref || left_is_ptr_subscript;

                    // Handle assignment specially
                    if matches!(op, BinaryOp::Assign | BinaryOp::AddAssign | BinaryOp::SubAssign |
                                   BinaryOp::MulAssign | BinaryOp::DivAssign |
                                   BinaryOp::RemAssign | BinaryOp::AndAssign |
                                   BinaryOp::OrAssign | BinaryOp::XorAssign |
                                   BinaryOp::ShlAssign | BinaryOp::ShrAssign) && needs_unsafe {
                        // For pointer dereference or subscript on left side, wrap entire assignment in unsafe
                        let left_raw = self.expr_to_string_raw(&node.children[0]);
                        let right_raw = self.expr_to_string_raw(&node.children[1]);
                        format!("unsafe {{ {} {} {} }}", left_raw, op_str, right_raw)
                    } else {
                        let left = self.expr_to_string(&node.children[0]);
                        let right = self.expr_to_string(&node.children[1]);
                        format!("{} {} {}", left, op_str, right)
                    }
                } else {
                    "/* binary op error */".to_string()
                }
            }
            ClangNodeKind::UnaryOperator { op, ty } => {
                if !node.children.is_empty() {
                    let operand = self.expr_to_string(&node.children[0]);
                    match op {
                        UnaryOp::Minus => format!("-{}", operand),
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => format!("!{}", operand),
                        UnaryOp::Not => format!("!{}", operand),  // bitwise not ~ in C++
                        UnaryOp::AddrOf => {
                            // For C++ pointers, cast reference to raw pointer
                            let rust_ty = ty.to_rust_type_str();
                            if rust_ty.starts_with("*mut ") {
                                format!("&mut {} as {}", operand, rust_ty)
                            } else if rust_ty.starts_with("*const ") {
                                format!("&{} as {}", operand, rust_ty)
                            } else {
                                format!("&{}", operand)
                            }
                        }
                        UnaryOp::Deref => {
                            // Raw pointer dereference needs unsafe
                            format!("unsafe {{ *{} }}", operand)
                        }
                        UnaryOp::PreInc => format!("{{ let v = {}; {} += 1; v + 1 }}", operand, operand),
                        UnaryOp::PreDec => format!("{{ let v = {}; {} -= 1; v - 1 }}", operand, operand),
                        UnaryOp::PostInc => format!("{{ let v = {}; {} += 1; v }}", operand, operand),
                        UnaryOp::PostDec => format!("{{ let v = {}; {} -= 1; v }}", operand, operand),
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::CallExpr { ty } => {
                // Check if this is a constructor call (CallExpr with Named type)
                if let CppType::Named(struct_name) = ty {
                    // Constructor call: all children are arguments
                    // For copy constructors (1 argument of same type), pass by reference
                    let args: Vec<String> = node.children.iter()
                        .map(|c| {
                            let arg_str = self.expr_to_string(c);
                            // Check if this is a copy constructor call (arg type matches struct)
                            let arg_type = Self::get_expr_type(c);
                            let arg_class = Self::extract_class_name(&arg_type);
                            if let Some(name) = arg_class {
                                if name == *struct_name {
                                    // Pass by reference for copy constructor
                                    return format!("&{}", arg_str);
                                }
                            }
                            arg_str
                        })
                        .collect();
                    let num_args = args.len();
                    // Always use StructName::new_N(args) to ensure custom constructor bodies run
                    format!("{}::new_{}({})", struct_name, num_args, args.join(", "))
                } else if !node.children.is_empty() {
                    // Regular function call: first child is the function reference, rest are arguments
                    let func = self.expr_to_string(&node.children[0]);

                    // Try to get function parameter types to handle reference parameters
                    let param_types = Self::get_function_param_types(&node.children[0]);

                    let args: Vec<String> = node.children[1..].iter()
                        .enumerate()
                        .map(|(i, c)| {
                            // Check if this parameter expects specific handling
                            if let Some(ref types) = param_types {
                                if i < types.len() {
                                    // Handle reference parameters
                                    if let CppType::Reference { is_const, .. } = &types[i] {
                                        // Check if argument is a reference variable
                                        if let Some(ref_ident) = self.get_ref_var_ident(c) {
                                            // Pass the reference variable directly (without dereferencing)
                                            return ref_ident;
                                        } else {
                                            // Add borrow for non-reference-variable arguments
                                            let arg_str = self.expr_to_string(c);
                                            let prefix = if *is_const { "&" } else { "&mut " };
                                            return format!("{}{}", prefix, arg_str);
                                        }
                                    }
                                    // Handle pointer parameters with array arguments
                                    // Also handle unsized array parameters (which are really pointers)
                                    if matches!(&types[i], CppType::Pointer { .. })
                                        || matches!(&types[i], CppType::Array { size: None, .. }) {
                                        let arg_type = Self::get_expr_type(c);
                                        let is_array = matches!(arg_type, Some(CppType::Array { .. }));
                                        if is_array {
                                            // Array to pointer decay
                                            let arg_str = self.expr_to_string(c);
                                            return format!("{}.as_mut_ptr()", arg_str);
                                        }
                                        // Also check using variable tracking
                                        if let Some(arr_ident) = self.get_array_var_ident(c) {
                                            return format!("{}.as_mut_ptr()", arr_ident);
                                        }
                                    }
                                }
                            }
                            self.expr_to_string(c)
                        })
                        .collect();
                    format!("{}({})", func, args.join(", "))
                } else {
                    "/* call error */".to_string()
                }
            }
            ClangNodeKind::MemberExpr { member_name, is_arrow, declaring_class, .. } => {
                if !node.children.is_empty() {
                    let base = self.expr_to_string(&node.children[0]);
                    // Check if this is accessing an inherited member
                    let base_type = Self::get_expr_type(&node.children[0]);
                    let needs_base_access = if let Some(decl_class) = declaring_class {
                        // Extract the actual class name from the base type, handling const, references, pointers
                        let base_class_name = Self::extract_class_name(&base_type);
                        if let Some(name) = base_class_name {
                            // If the declaring class differs from the base expression's type,
                            // we need to access through __base
                            name != *decl_class
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    let member = sanitize_identifier(member_name);
                    if *is_arrow {
                        if needs_base_access {
                            format!("(*{}).__base.{}", base, member)
                        } else {
                            format!("(*{}).{}", base, member)
                        }
                    } else {
                        if needs_base_access {
                            format!("{}.__base.{}", base, member)
                        } else {
                            format!("{}.{}", base, member)
                        }
                    }
                } else {
                    // Implicit this
                    format!("self.{}", sanitize_identifier(member_name))
                }
            }
            ClangNodeKind::ArraySubscriptExpr { .. } => {
                if node.children.len() >= 2 {
                    let arr = self.expr_to_string(&node.children[0]);
                    let idx = self.expr_to_string(&node.children[1]);
                    // Check if the array expression is a pointer type
                    // (also check for unsized arrays which decay to pointers)
                    let arr_type = Self::get_expr_type(&node.children[0]);
                    let is_pointer = matches!(arr_type, Some(CppType::Pointer { .. }))
                        || matches!(arr_type, Some(CppType::Array { size: None, .. }))
                        || self.is_ptr_var_expr(&node.children[0]);
                    if is_pointer {
                        // Pointer indexing requires unsafe pointer arithmetic
                        format!("unsafe {{ *{}.add({} as usize) }}", arr, idx)
                    } else {
                        // Array indexing - cast index to usize
                        format!("{}[{} as usize]", arr, idx)
                    }
                } else {
                    "/* array subscript error */".to_string()
                }
            }
            ClangNodeKind::ConditionalOperator { .. } => {
                if node.children.len() >= 3 {
                    let cond = self.expr_to_string(&node.children[0]);
                    let then_expr = self.expr_to_string(&node.children[1]);
                    let else_expr = self.expr_to_string(&node.children[2]);
                    format!("if {} {{ {} }} else {{ {} }}", cond, then_expr, else_expr)
                } else {
                    "/* ternary error */".to_string()
                }
            }
            ClangNodeKind::ParenExpr { .. } => {
                // Preserve parentheses
                if !node.children.is_empty() {
                    format!("({})", self.expr_to_string(&node.children[0]))
                } else {
                    "()".to_string()
                }
            }
            ClangNodeKind::ImplicitCastExpr { .. } => {
                // Pass through implicit casts
                if !node.children.is_empty() {
                    self.expr_to_string(&node.children[0])
                } else {
                    "()".to_string()
                }
            }
            ClangNodeKind::CastExpr { ty, cast_kind } => {
                // Explicit C++ casts: static_cast, reinterpret_cast, const_cast, C-style
                if !node.children.is_empty() {
                    let inner = self.expr_to_string(&node.children[0]);
                    let rust_type = ty.to_rust_type_str();
                    match cast_kind {
                        CastKind::Static | CastKind::Reinterpret | CastKind::Other => {
                            // Generate Rust "as" cast
                            format!("{} as {}", inner, rust_type)
                        }
                        CastKind::Const => {
                            // const_cast usually just changes mutability, pass through
                            inner
                        }
                        _ => {
                            // For other cast kinds, generate as cast
                            format!("{} as {}", inner, rust_type)
                        }
                    }
                } else {
                    "()".to_string()
                }
            }
            ClangNodeKind::InitListExpr { ty } => {
                // Aggregate initialization
                if let CppType::Named(name) = ty {
                    let fields: Vec<String> = node.children.iter()
                        .map(|c| self.expr_to_string(c))
                        .collect();
                    format!("{} {{ {} }}", name, fields.join(", "))
                } else {
                    let elems: Vec<String> = node.children.iter()
                        .map(|c| self.expr_to_string(c))
                        .collect();
                    format!("[{}]", elems.join(", "))
                }
            }
            ClangNodeKind::ThrowExpr { exception_ty } => {
                // throw expr → panic!("message")
                // If there's a child expression, try to extract a message
                if !node.children.is_empty() {
                    // Try to get the thrown value - look for StringLiteral in children
                    let msg = Self::extract_throw_message(node);
                    if let Some(m) = msg {
                        format!("panic!(\"{}\")", m)
                    } else if let Some(ty) = exception_ty {
                        format!("panic!(\"Threw {:?}\")", ty)
                    } else {
                        "panic!(\"Exception thrown\")".to_string()
                    }
                } else {
                    // throw; (rethrow) - in Rust, just continue panicking
                    "panic!(\"Rethrow\")".to_string()
                }
            }
            _ => {
                // Fallback: try children
                if !node.children.is_empty() {
                    self.expr_to_string(&node.children[0])
                } else {
                    format!("/* unsupported: {:?} */", std::mem::discriminant(&node.kind))
                }
            }
        }
    }

    /// Try to extract a string message from a throw expression.
    /// Looks recursively for StringLiteral nodes.
    fn extract_throw_message(node: &ClangNode) -> Option<String> {
        match &node.kind {
            ClangNodeKind::StringLiteral(s) => Some(s.clone()),
            _ => {
                // Recursively search children
                for child in &node.children {
                    if let Some(msg) = Self::extract_throw_message(child) {
                        return Some(msg);
                    }
                }
                None
            }
        }
    }

    fn writeln(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
    }
}

impl Default for AstCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize a C++ identifier for Rust.
fn sanitize_identifier(name: &str) -> String {
    // Handle operators
    let mut result = if name.starts_with("operator") {
        match name {
            "operator=" => "op_assign".to_string(),
            "operator==" => "op_eq".to_string(),
            "operator!=" => "op_ne".to_string(),
            "operator<" => "op_lt".to_string(),
            "operator>" => "op_gt".to_string(),
            "operator+" => "op_add".to_string(),
            "operator-" => "op_sub".to_string(),
            "operator*" => "op_mul".to_string(),
            "operator/" => "op_div".to_string(),
            "operator[]" => "op_index".to_string(),
            "operator()" => "op_call".to_string(),
            _ => name.replace("operator", "op_")
        }
    } else {
        name.to_string()
    };

    // Replace invalid characters
    result = result.replace("::", "_")
        .replace('<', "_").replace('>', "_")
        .replace(' ', "").replace('%', "_")
        .replace('=', "_").replace('&', "_")
        .replace('|', "_").replace('!', "_")
        .replace('*', "_").replace('/', "_")
        .replace('+', "_").replace('-', "_")
        .replace('[', "_").replace(']', "_")
        .replace('(', "_").replace(')', "_");

    // Handle keywords
    if RUST_KEYWORDS.contains(&result.as_str()) {
        result = format!("r#{}", result);
    }

    // Handle empty names
    if result.is_empty() {
        result = "_unnamed".to_string();
    }

    result
}

/// Convert binary operator to Rust string.
fn binop_to_string(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::And => "&",      // Bitwise AND
        BinaryOp::Or => "|",       // Bitwise OR
        BinaryOp::Xor => "^",      // Bitwise XOR
        BinaryOp::LAnd => "&&",    // Logical AND
        BinaryOp::LOr => "||",     // Logical OR
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::Assign => "=",
        BinaryOp::AddAssign => "+=",
        BinaryOp::SubAssign => "-=",
        BinaryOp::MulAssign => "*=",
        BinaryOp::DivAssign => "/=",
        BinaryOp::RemAssign => "%=",
        BinaryOp::ShlAssign => "<<=",
        BinaryOp::ShrAssign => ">>=",
        BinaryOp::AndAssign => "&=",
        BinaryOp::OrAssign => "|=",
        BinaryOp::XorAssign => "^=",
        BinaryOp::Comma => ",",
    }
}

/// Get default value for a type.
fn default_value_for_type(ty: &CppType) -> String {
    match ty {
        CppType::Void => "()".to_string(),
        CppType::Bool => "false".to_string(),
        CppType::Char { .. } | CppType::Short { .. } |
        CppType::Int { .. } | CppType::Long { .. } | CppType::LongLong { .. } => "0".to_string(),
        CppType::Float => "0.0f32".to_string(),
        CppType::Double => "0.0f64".to_string(),
        CppType::Pointer { .. } => "std::ptr::null_mut()".to_string(),
        CppType::Reference { .. } => "std::ptr::null_mut()".to_string(),
        CppType::Named(_) => "Default::default()".to_string(),
        CppType::Array { element, size } => {
            let elem_default = default_value_for_type(element);
            if let Some(n) = size {
                format!("[{}; {}]", elem_default, n)
            } else {
                "Default::default()".to_string()
            }
        }
        _ => "Default::default()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::SourceLocation;

    fn make_node(kind: ClangNodeKind, children: Vec<ClangNode>) -> ClangNode {
        ClangNode {
            kind,
            children,
            location: SourceLocation::default(),
        }
    }

    #[test]
    fn test_simple_function() {
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "add".to_string(),
                    mangled_name: "_Z3addii".to_string(),
                    return_type: CppType::Int { signed: true },
                    params: vec![
                        ("a".to_string(), CppType::Int { signed: true }),
                        ("b".to_string(), CppType::Int { signed: true }),
                    ],
                    is_definition: true,
                    is_noexcept: false,
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::ReturnStmt,
                        vec![make_node(
                            ClangNodeKind::BinaryOperator {
                                op: BinaryOp::Add,
                                ty: CppType::Int { signed: true },
                            },
                            vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "a".to_string(),
                                    ty: CppType::Int { signed: true }
                                }, vec![]),
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "b".to_string(),
                                    ty: CppType::Int { signed: true }
                                }, vec![]),
                            ],
                        )],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        assert!(code.contains("pub fn add(a: i32, b: i32) -> i32"));
        assert!(code.contains("return a + b"));
    }

    #[test]
    fn test_if_statement() {
        let ast = make_node(
            ClangNodeKind::TranslationUnit,
            vec![make_node(
                ClangNodeKind::FunctionDecl {
                    name: "max".to_string(),
                    mangled_name: "_Z3maxii".to_string(),
                    return_type: CppType::Int { signed: true },
                    params: vec![
                        ("a".to_string(), CppType::Int { signed: true }),
                        ("b".to_string(), CppType::Int { signed: true }),
                    ],
                    is_definition: true,
                    is_noexcept: false,
                },
                vec![make_node(
                    ClangNodeKind::CompoundStmt,
                    vec![make_node(
                        ClangNodeKind::IfStmt,
                        vec![
                            // Condition: a > b
                            make_node(ClangNodeKind::BinaryOperator {
                                op: BinaryOp::Gt,
                                ty: CppType::Bool,
                            }, vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "a".to_string(),
                                    ty: CppType::Int { signed: true }
                                }, vec![]),
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "b".to_string(),
                                    ty: CppType::Int { signed: true }
                                }, vec![]),
                            ]),
                            // Then: return a
                            make_node(ClangNodeKind::ReturnStmt, vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "a".to_string(),
                                    ty: CppType::Int { signed: true }
                                }, vec![]),
                            ]),
                            // Else: return b
                            make_node(ClangNodeKind::ReturnStmt, vec![
                                make_node(ClangNodeKind::DeclRefExpr {
                                    name: "b".to_string(),
                                    ty: CppType::Int { signed: true }
                                }, vec![]),
                            ]),
                        ],
                    )],
                )],
            )],
        );

        let code = AstCodeGen::new().generate(&ast);
        assert!(code.contains("if a > b {"));
        assert!(code.contains("return a"));
        assert!(code.contains("} else {"));
        assert!(code.contains("return b"));
    }
}
