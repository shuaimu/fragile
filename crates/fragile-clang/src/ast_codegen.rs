//! Direct AST to Rust source code generation.
//!
//! This module generates Rust source code directly from the Clang AST,
//! without going through an intermediate MIR representation.
//! This produces cleaner, more idiomatic Rust code.

use crate::ast::{ClangNode, ClangNodeKind, BinaryOp, UnaryOp, ConstructorKind};
use crate::types::CppType;

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
}

impl AstCodeGen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
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
        // Doc comment
        self.writeln(&format!("/// C++ function `{}`", name));
        self.writeln(&format!("/// Mangled: `{}`", mangled_name));

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

        self.writeln(&format!("pub fn {}({}){} {{", sanitize_identifier(name), params_str, ret_str));
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
    }

    /// Generate struct definition.
    fn generate_struct(&mut self, name: &str, is_class: bool, children: &[ClangNode]) {
        let kind = if is_class { "class" } else { "struct" };
        self.writeln(&format!("/// C++ {} `{}`", kind, name));
        self.writeln("#[repr(C)]");
        self.writeln(&format!("pub struct {} {{", name));
        self.indent += 1;

        // Collect fields
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

        // Generate impl block for methods
        let methods: Vec<_> = children.iter().filter(|c| {
            matches!(&c.kind, ClangNodeKind::CXXMethodDecl { is_definition: true, .. } |
                              ClangNodeKind::ConstructorDecl { is_definition: true, .. })
        }).collect();

        if !methods.is_empty() {
            self.writeln("");
            self.writeln(&format!("impl {} {{", name));
            self.indent += 1;

            for method in methods {
                self.generate_method(method, name);
            }

            self.indent -= 1;
            self.writeln("}");
        }

        self.writeln("");
    }

    /// Generate a method or constructor.
    fn generate_method(&mut self, node: &ClangNode, _struct_name: &str) {
        match &node.kind {
            ClangNodeKind::CXXMethodDecl { name, return_type, params, is_static, .. } => {
                let self_param = if *is_static { "" } else { "&self, " };
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
            ClangNodeKind::ConstructorDecl { params, ctor_kind, .. } => {
                let fn_name = match ctor_kind {
                    ConstructorKind::Default => "new",
                    _ => "new_1",
                };

                let params_str = params.iter()
                    .map(|(n, t)| format!("{}: {}", sanitize_identifier(n), t.to_rust_type_str()))
                    .collect::<Vec<_>>()
                    .join(", ");

                self.writeln(&format!("pub fn {}({}) -> Self {{", fn_name, params_str));
                self.indent += 1;

                // Extract member initializers from constructor children
                // Pattern: MemberRef { name } followed by initialization expression
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
                    }
                    i += 1;
                }

                self.writeln("Self {");
                self.indent += 1;
                if initializers.is_empty() {
                    // Default constructor - use default values
                    // We don't have field info here, so use ..Default::default() pattern
                    self.writeln("..Default::default()");
                } else {
                    for (field, value) in initializers {
                        self.writeln(&format!("{}: {},", sanitize_identifier(&field), value));
                    }
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
                    if let ClangNodeKind::VarDecl { name, ty, has_init } = &child.kind {
                        let init = if *has_init && !child.children.is_empty() {
                            format!(" = {}", self.expr_to_string(&child.children[0]))
                        } else {
                            format!(" = {}", default_value_for_type(ty))
                        };
                        self.writeln(&format!("let mut {}: {}{};",
                            sanitize_identifier(name), ty.to_rust_type_str(), init));
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

    /// Generate a for statement.
    fn generate_for_stmt(&mut self, node: &ClangNode) {
        // C++ for loops don't map directly to Rust for loops
        // Convert to: { init; while cond { body; inc; } }
        // Children: [init], [cond], [inc], body

        // For simplicity, generate a loop with manual control
        self.writeln("{");
        self.indent += 1;

        // This is a simplified version - real impl would be more sophisticated
        if node.children.len() >= 4 {
            // Init
            self.generate_stmt(&node.children[0], false);

            // While loop
            let cond = if matches!(&node.children[1].kind, ClangNodeKind::IntegerLiteral { .. }) {
                "true".to_string()
            } else {
                self.expr_to_string(&node.children[1])
            };

            self.writeln(&format!("while {} {{", cond));
            self.indent += 1;

            // Body
            self.generate_stmt(&node.children[3], false);

            // Increment
            let inc = self.expr_to_string(&node.children[2]);
            if !inc.is_empty() {
                self.writeln(&format!("{};", inc));
            }

            self.indent -= 1;
            self.writeln("}");
        }

        self.indent -= 1;
        self.writeln("}");
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
            ClangNodeKind::StringLiteral(s) => format!("\"{}\"", s.escape_default()),
            ClangNodeKind::DeclRefExpr { name, .. } => {
                if name == "this" {
                    "self".to_string()
                } else {
                    sanitize_identifier(name)
                }
            }
            ClangNodeKind::CXXThisExpr { .. } => "self".to_string(),
            ClangNodeKind::BinaryOperator { op, .. } => {
                if node.children.len() >= 2 {
                    let left = self.expr_to_string(&node.children[0]);
                    let right = self.expr_to_string(&node.children[1]);
                    let op_str = binop_to_string(op);

                    // Handle assignment specially
                    if matches!(op, BinaryOp::Assign) {
                        format!("{} = {}", left, right)
                    } else if matches!(op, BinaryOp::AddAssign | BinaryOp::SubAssign |
                                          BinaryOp::MulAssign | BinaryOp::DivAssign |
                                          BinaryOp::RemAssign | BinaryOp::AndAssign |
                                          BinaryOp::OrAssign | BinaryOp::XorAssign |
                                          BinaryOp::ShlAssign | BinaryOp::ShrAssign) {
                        format!("{} {} {}", left, op_str, right)
                    } else {
                        format!("{} {} {}", left, op_str, right)
                    }
                } else {
                    "/* binary op error */".to_string()
                }
            }
            ClangNodeKind::UnaryOperator { op, .. } => {
                if !node.children.is_empty() {
                    let operand = self.expr_to_string(&node.children[0]);
                    match op {
                        UnaryOp::Minus => format!("-{}", operand),
                        UnaryOp::Plus => operand,
                        UnaryOp::LNot => format!("!{}", operand),
                        UnaryOp::Not => format!("!{}", operand),  // bitwise not ~ in C++
                        UnaryOp::AddrOf => format!("&{}", operand),
                        UnaryOp::Deref => format!("*{}", operand),
                        UnaryOp::PreInc => format!("{{ let v = {}; {} += 1; v + 1 }}", operand, operand),
                        UnaryOp::PreDec => format!("{{ let v = {}; {} -= 1; v - 1 }}", operand, operand),
                        UnaryOp::PostInc => format!("{{ let v = {}; {} += 1; v }}", operand, operand),
                        UnaryOp::PostDec => format!("{{ let v = {}; {} -= 1; v }}", operand, operand),
                    }
                } else {
                    "/* unary op error */".to_string()
                }
            }
            ClangNodeKind::CallExpr { .. } => {
                // First child is the function reference, rest are arguments
                if !node.children.is_empty() {
                    let func = self.expr_to_string(&node.children[0]);
                    let args: Vec<String> = node.children[1..].iter()
                        .map(|c| self.expr_to_string(c))
                        .collect();
                    format!("{}({})", func, args.join(", "))
                } else {
                    "/* call error */".to_string()
                }
            }
            ClangNodeKind::MemberExpr { member_name, is_arrow, .. } => {
                if !node.children.is_empty() {
                    let base = self.expr_to_string(&node.children[0]);
                    if *is_arrow {
                        format!("(*{}).{}", base, sanitize_identifier(member_name))
                    } else {
                        format!("{}.{}", base, sanitize_identifier(member_name))
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
                    format!("{}[{}]", arr, idx)
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
            ClangNodeKind::ParenExpr { .. } | ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::CastExpr { .. } => {
                // Pass through to child
                if !node.children.is_empty() {
                    self.expr_to_string(&node.children[0])
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
        CppType::Named(name) => format!("{}::new()", name),
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
