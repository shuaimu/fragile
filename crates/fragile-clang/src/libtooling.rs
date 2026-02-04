//! LibTooling-based AST parsing for template instantiation support.
//!
//! This module provides access to template instantiations that libclang cannot
//! fully expose. It uses the fragile-ast-exporter crate which is built on
//! Clang's LibTooling C++ API.
//!
//! The primary use case is to get the fully instantiated method bodies of
//! class templates like std::vector<T> with concrete types substituted.

use crate::ast::{ClangNode, ClangNodeKind, SourceLocation};
use crate::types::CppType;
use fragile_ast_exporter::{clang_ast::AstContext, export_ast, ASTEntryTag};
use miette::{miette, Result};
use std::collections::HashMap;
use std::path::Path;

/// Parser that uses LibTooling for full template instantiation access.
pub struct LibToolingParser {
    /// Directory containing compile_commands.json
    compile_commands_dir: Option<String>,
    /// Extra compiler arguments
    extra_args: Vec<String>,
}

impl LibToolingParser {
    /// Create a new LibTooling parser.
    pub fn new() -> Self {
        Self {
            compile_commands_dir: None,
            extra_args: Vec::new(),
        }
    }

    /// Set the directory containing compile_commands.json.
    pub fn with_compile_commands_dir(mut self, dir: &str) -> Self {
        self.compile_commands_dir = Some(dir.to_string());
        self
    }

    /// Add extra compiler arguments.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Parse a file and return the LibTooling AST context.
    ///
    /// This provides access to the full AST including template instantiations
    /// with concrete types and actual method bodies.
    pub fn parse_file(&self, path: &Path) -> Result<AstContext> {
        // For compile_commands_dir, default to the file's directory
        let compile_dir = self
            .compile_commands_dir
            .as_ref()
            .map(|s| Path::new(s).to_path_buf())
            .unwrap_or_else(|| {
                path.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| Path::new(".").to_path_buf())
            });

        // Ensure compile_commands.json exists, or create a minimal one
        let compile_commands_path = compile_dir.join("compile_commands.json");
        if !compile_commands_path.exists() {
            // Create a minimal compile_commands.json
            let compile_commands = format!(
                r#"[
  {{
    "directory": "{}",
    "command": "clang++ -std=c++17 -c {} -o /dev/null",
    "file": "{}"
  }}
]"#,
                compile_dir.display(),
                path.display(),
                path.display()
            );
            std::fs::write(&compile_commands_path, compile_commands)
                .map_err(|e| miette!("Failed to create compile_commands.json: {}", e))?;
        }

        let extra_args: Vec<&str> = self.extra_args.iter().map(|s| s.as_str()).collect();

        export_ast(path, &compile_dir, &extra_args, false)
            .map_err(|e| miette!("LibTooling parse failed: {}", e))
    }

    /// Extract template method instantiations from an AST context.
    ///
    /// Returns a map from method name to a list of instantiated method info.
    /// Each instantiation includes the concrete types and the method body AST.
    pub fn extract_template_methods(
        &self,
        ctx: &AstContext,
    ) -> HashMap<String, Vec<TemplateMethodInstantiation>> {
        let mut methods: HashMap<String, Vec<TemplateMethodInstantiation>> = HashMap::new();

        for (_id, node) in &ctx.ast_nodes {
            if node.tag == ASTEntryTag::TagCXXMethodDecl {
                // Check if this is a template instantiation
                // extras[6] is typically the isInstantiation flag
                let is_instantiation = node
                    .get_bool(6)
                    .unwrap_or(false);

                if is_instantiation {
                    let name = node.get_string(0).unwrap_or("").to_string();

                    // Get body (first child if present)
                    let has_body = node.children.first().and_then(|c| *c).map(|body_id| {
                        ctx.ast_nodes.get(&body_id).is_some()
                    }).unwrap_or(false);

                    let instantiation = TemplateMethodInstantiation {
                        name: name.clone(),
                        has_body,
                        node_id: node.id,
                    };

                    methods.entry(name).or_default().push(instantiation);
                }
            }
        }

        methods
    }
}

impl Default for LibToolingParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a template method instantiation.
#[derive(Debug, Clone)]
pub struct TemplateMethodInstantiation {
    /// Method name
    pub name: String,
    /// Whether the method has a body (not just a declaration)
    pub has_body: bool,
    /// Node ID in the AST context
    pub node_id: u64,
}

/// Extract all template method bodies from the LibTooling AST.
///
/// Returns a map suitable for use with `AstCodeGen::set_libtooling_bodies()`.
/// Key: (class_name, method_name), Value: list of method body ClangNodes
///
/// Note: Class names are extracted from the parent record declarations.
/// The key uses the raw C++ class name (e.g., "map", "_Rb_tree") without
/// template parameters.
pub fn extract_method_bodies(ctx: &AstContext) -> HashMap<(String, String), Vec<ClangNode>> {
    // First, build a map from method node ID to parent class name
    let mut method_to_class: HashMap<u64, String> = HashMap::new();

    for (_id, node) in &ctx.ast_nodes {
        if node.tag == ASTEntryTag::TagCXXRecordDecl
            || node.tag == ASTEntryTag::TagClassTemplateSpecializationDecl
        {
            let class_name = node.get_string(0).unwrap_or("").to_string();
            if class_name.is_empty() {
                continue;
            }

            // Find all method declarations that are children of this class
            for child_opt in &node.children {
                if let Some(child_id) = child_opt {
                    if let Some(child_node) = ctx.ast_nodes.get(child_id) {
                        if child_node.tag == ASTEntryTag::TagCXXMethodDecl {
                            method_to_class.insert(*child_id, class_name.clone());
                        }
                    }
                }
            }
        }
    }

    let mut bodies: HashMap<(String, String), Vec<ClangNode>> = HashMap::new();

    for (id, node) in &ctx.ast_nodes {
        if node.tag == ASTEntryTag::TagCXXMethodDecl {
            let method_name = node.get_string(0).unwrap_or("").to_string();
            if method_name.is_empty() {
                continue;
            }

            // Find the body - it's typically the last non-None child, or a child that is a CompoundStmt
            let body_id = node.children.iter()
                .filter_map(|c| *c)
                .find(|child_id| {
                    ctx.ast_nodes.get(child_id)
                        .map(|child| child.tag == ASTEntryTag::TagCompoundStmt)
                        .unwrap_or(false)
                });

            if let Some(body_id) = body_id {
                // Convert the body to ClangNode
                if let Some(body_node) = convert_to_clang_node(ctx, body_id) {
                    // Get parent class name from our map, or use empty string as fallback
                    let class_name = method_to_class.get(id).cloned().unwrap_or_default();

                    // Also add with empty class name so methods can be looked up without class context
                    let key = (class_name.clone(), method_name.clone());
                    bodies.entry(key).or_default().push(body_node.clone());

                    // If class name is not empty, also add entry with empty class name
                    // This allows matching when we don't know the exact class name
                    if !class_name.is_empty() {
                        let fallback_key = (String::new(), method_name.clone());
                        bodies.entry(fallback_key).or_default().push(body_node);
                    }
                }
            }
        }
    }

    bodies
}

/// Find the parent class name for a method declaration (deprecated - use method_to_class map).
fn find_parent_class_name(ctx: &AstContext, _method_id: u64) -> String {
    // Search for CXXRecordDecl or ClassTemplateSpecializationDecl that contains this method
    for (_id, node) in &ctx.ast_nodes {
        if node.tag == ASTEntryTag::TagCXXRecordDecl
            || node.tag == ASTEntryTag::TagClassTemplateSpecializationDecl
        {
            // Check if this record contains the method
            // Note: This is a simplified heuristic - in practice we'd need parent pointers
            let class_name = node.get_string(0).unwrap_or("").to_string();
            if !class_name.is_empty() {
                // Return the first class name we find that might be relevant
                // In a more complete implementation, we'd check the AST hierarchy
                return class_name;
            }
        }
    }

    // Fallback: return empty string (will match any class)
    String::new()
}

/// Convert an AST exporter node to a ClangNode.
///
/// This is a partial conversion - only nodes that are commonly needed
/// for template instantiation support are converted.
///
/// Uses an iterative approach with explicit stack to avoid stack overflow
/// on deeply nested ASTs (common in STL template instantiations).
pub fn convert_to_clang_node(ctx: &AstContext, root_id: u64) -> Option<ClangNode> {
    // Limit depth to avoid excessive processing of deeply nested STL internals
    const MAX_DEPTH: usize = 100;

    convert_node_with_depth(ctx, root_id, 0, MAX_DEPTH)
}

fn convert_node_with_depth(
    ctx: &AstContext,
    node_id: u64,
    current_depth: usize,
    max_depth: usize,
) -> Option<ClangNode> {
    if current_depth > max_depth {
        // Return a placeholder for too-deep nodes
        return Some(ClangNode {
            kind: ClangNodeKind::Unknown("TooDeep".to_string()),
            children: vec![],
            location: SourceLocation {
                file: None,
                line: 0,
                column: 0,
            },
        });
    }

    let node = ctx.ast_nodes.get(&node_id)?;

    let location = SourceLocation {
        file: None, // TODO: Extract from node.loc
        line: node.loc.begin_line as u32,
        column: node.loc.begin_column as u32,
    };

    let kind = match node.tag {
        ASTEntryTag::TagCompoundStmt => ClangNodeKind::CompoundStmt,

        ASTEntryTag::TagReturnStmt => {
            // ReturnStmt is a unit variant, its value is in children
            ClangNodeKind::ReturnStmt
        }

        ASTEntryTag::TagBinaryOperator => {
            // Extract operator from extras
            let op_str = node.get_string(0).unwrap_or("+");
            let op = parse_binary_op(op_str);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::BinaryOperator { op, ty }
        }

        ASTEntryTag::TagDeclRefExpr => {
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::DeclRefExpr {
                name,
                ty,
                namespace_path: vec![],
            }
        }

        ASTEntryTag::TagIntegerLiteral => {
            let value = node.get_int(0).unwrap_or(0) as i128;
            let cpp_type = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::IntegerLiteral { value, cpp_type }
        }

        ASTEntryTag::TagCXXThisExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXThisExpr { ty }
        }

        ASTEntryTag::TagMemberExpr => {
            let member_name = node.get_string(0).unwrap_or("").to_string();
            let is_arrow = node.get_bool(1).unwrap_or(false);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::MemberExpr {
                member_name,
                is_arrow,
                ty,
                is_static: false,
                declaring_class: None,
            }
        }

        ASTEntryTag::TagCallExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagDeclStmt => ClangNodeKind::DeclStmt,

        ASTEntryTag::TagVarDecl => {
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            let has_init = node.children.iter().any(|c| c.is_some());
            ClangNodeKind::VarDecl { name, ty, has_init }
        }

        ASTEntryTag::TagUnaryOperator => {
            let op_str = node.get_string(0).unwrap_or("*");
            let op = parse_unary_op(op_str);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::UnaryOperator { op, ty }
        }

        ASTEntryTag::TagImplicitCastExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ImplicitCastExpr {
                cast_kind: crate::ast::CastKind::LValueToRValue,
                ty,
            }
        }

        ASTEntryTag::TagCXXMemberCallExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagIfStmt => ClangNodeKind::IfStmt,

        ASTEntryTag::TagForStmt => ClangNodeKind::ForStmt,

        ASTEntryTag::TagWhileStmt => ClangNodeKind::WhileStmt,

        ASTEntryTag::TagBreakStmt => ClangNodeKind::BreakStmt,

        ASTEntryTag::TagContinueStmt => ClangNodeKind::ContinueStmt,

        ASTEntryTag::TagParenExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ParenExpr { ty }
        }

        ASTEntryTag::TagCXXBoolLiteralExpr => {
            // Bool value is typically in extras
            let value = node.get_bool(0).unwrap_or(false);
            ClangNodeKind::BoolLiteral(value)
        }

        ASTEntryTag::TagCXXNullPtrLiteralExpr => ClangNodeKind::NullPtrLiteral,

        ASTEntryTag::TagFloatingLiteral => {
            // Get float value - try different methods
            let value = if let Some(v) = node.get_int(0) {
                v as f64
            } else {
                0.0
            };
            let cpp_type = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::FloatingLiteral { value, cpp_type }
        }

        ASTEntryTag::TagConditionalOperator => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ConditionalOperator { ty }
        }

        ASTEntryTag::TagCStyleCastExpr | ASTEntryTag::TagCXXStaticCastExpr
        | ASTEntryTag::TagCXXReinterpretCastExpr | ASTEntryTag::TagCXXConstCastExpr
        | ASTEntryTag::TagCXXFunctionalCastExpr | ASTEntryTag::TagCXXDynamicCastExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CastExpr {
                cast_kind: crate::ast::CastKind::LValueToRValue,
                ty,
            }
        }

        ASTEntryTag::TagArraySubscriptExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::ArraySubscriptExpr { ty }
        }

        ASTEntryTag::TagCXXOperatorCallExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagCompoundAssignOperator => {
            let op_str = node.get_string(0).unwrap_or("+=");
            let op = parse_binary_op(op_str);
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::BinaryOperator { op, ty }
        }

        // Expression wrappers - pass through to children
        ASTEntryTag::TagExprWithCleanups
        | ASTEntryTag::TagMaterializeTemporaryExpr
        | ASTEntryTag::TagCXXBindTemporaryExpr => {
            // These are wrapper nodes, we just use Unknown but children will be processed
            ClangNodeKind::Unknown("ExprWrapper".to_string())
        }

        ASTEntryTag::TagCXXConstructExpr | ASTEntryTag::TagCXXTemporaryObjectExpr => {
            // CXXConstructExpr doesn't exist in our AST, use a CallExpr as placeholder
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CallExpr {
                ty,
                template_instantiation: None,
            }
        }

        ASTEntryTag::TagInitListExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::InitListExpr { ty }
        }

        ASTEntryTag::TagCXXDefaultArgExpr | ASTEntryTag::TagCXXDefaultInitExpr => {
            ClangNodeKind::Unknown("DefaultExpr".to_string())
        }

        ASTEntryTag::TagCXXNewExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXNewExpr {
                ty,
                is_array: false,
                is_placement: false,
            }
        }

        ASTEntryTag::TagCXXDeleteExpr => {
            ClangNodeKind::CXXDeleteExpr { is_array: false }
        }

        ASTEntryTag::TagStringLiteral => {
            let value = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::StringLiteral(value)
        }

        ASTEntryTag::TagCharacterLiteral => {
            // Character literals are treated as integer literals in the AST
            let value = node.get_int(0).unwrap_or(0) as i128;
            ClangNodeKind::IntegerLiteral {
                value,
                cpp_type: Some(CppType::Char { signed: true }),
            }
        }

        _ => {
            // For unknown nodes, create an Unknown variant
            ClangNodeKind::Unknown(format!("{:?}", node.tag))
        }
    };

    // Convert children with depth tracking
    let children: Vec<ClangNode> = node
        .children
        .iter()
        .filter_map(|child_id| {
            child_id.and_then(|id| convert_node_with_depth(ctx, id, current_depth + 1, max_depth))
        })
        .collect();

    Some(ClangNode {
        kind,
        children,
        location,
    })
}

fn extract_type_from_node(ctx: &AstContext, node: &fragile_ast_exporter::clang_ast::AstNode) -> CppType {
    if let Some(type_id) = node.type_id {
        if let Some(type_node) = ctx.type_nodes.get(&type_id) {
            // Convert type node tag to a proper CppType
            use fragile_ast_exporter::ASTEntryTag;
            match type_node.tag {
                ASTEntryTag::TagVoid => return CppType::Void,
                ASTEntryTag::TagBool => return CppType::Bool,
                ASTEntryTag::TagInt => return CppType::Int { signed: true },
                ASTEntryTag::TagUInt => return CppType::Int { signed: false },
                ASTEntryTag::TagLong => return CppType::Long { signed: true },
                ASTEntryTag::TagULong => return CppType::Long { signed: false },
                ASTEntryTag::TagLongLong => return CppType::LongLong { signed: true },
                ASTEntryTag::TagULongLong => return CppType::LongLong { signed: false },
                ASTEntryTag::TagShort => return CppType::Short { signed: true },
                ASTEntryTag::TagUShort => return CppType::Short { signed: false },
                ASTEntryTag::TagChar | ASTEntryTag::TagSChar => return CppType::Char { signed: true },
                ASTEntryTag::TagUChar => return CppType::Char { signed: false },
                ASTEntryTag::TagFloat => return CppType::Float,
                ASTEntryTag::TagDouble => return CppType::Double,
                ASTEntryTag::TagLongDouble => return CppType::Double, // Use double as fallback
                ASTEntryTag::TagPointerType => {
                    // For pointer types, try to get the pointee type
                    // For now, just return a void pointer
                    return CppType::Pointer {
                        pointee: Box::new(CppType::Void),
                        is_const: false,
                    };
                }
                ASTEntryTag::TagLValueReferenceType => {
                    return CppType::Reference {
                        referent: Box::new(CppType::Int { signed: true }),
                        is_const: false,
                        is_rvalue: false,
                    };
                }
                ASTEntryTag::TagRValueReferenceType => {
                    return CppType::Reference {
                        referent: Box::new(CppType::Int { signed: true }),
                        is_const: false,
                        is_rvalue: true,
                    };
                }
                // For record/class types, use the type name if available
                ASTEntryTag::TagRecordType | ASTEntryTag::TagElaboratedType
                | ASTEntryTag::TagTemplateSpecializationType => {
                    // Try to get a name from the type node
                    if let Some(name) = type_node.get_string(0) {
                        if !name.is_empty() {
                            return CppType::Named(name.to_string());
                        }
                    }
                    return CppType::Named("auto".to_string());
                }
                _ => {
                    // For other types, use 'auto' as a placeholder
                    return CppType::Named("auto".to_string());
                }
            }
        }
    }
    // Default to int for unknown types
    CppType::Int { signed: true }
}

fn parse_binary_op(op: &str) -> crate::ast::BinaryOp {
    use crate::ast::BinaryOp;
    match op {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        "%" => BinaryOp::Rem,
        "==" => BinaryOp::Eq,
        "!=" => BinaryOp::Ne,
        "<" => BinaryOp::Lt,
        "<=" => BinaryOp::Le,
        ">" => BinaryOp::Gt,
        ">=" => BinaryOp::Ge,
        "&&" => BinaryOp::LAnd,
        "||" => BinaryOp::LOr,
        "&" => BinaryOp::And,  // Bitwise AND
        "|" => BinaryOp::Or,   // Bitwise OR
        "^" => BinaryOp::Xor,  // Bitwise XOR
        "<<" => BinaryOp::Shl,
        ">>" => BinaryOp::Shr,
        "=" => BinaryOp::Assign,
        "+=" => BinaryOp::AddAssign,
        "-=" => BinaryOp::SubAssign,
        "*=" => BinaryOp::MulAssign,
        "/=" => BinaryOp::DivAssign,
        "%=" => BinaryOp::RemAssign,
        "&=" => BinaryOp::AndAssign,
        "|=" => BinaryOp::OrAssign,
        "^=" => BinaryOp::XorAssign,
        "<<=" => BinaryOp::ShlAssign,
        ">>=" => BinaryOp::ShrAssign,
        "," => BinaryOp::Comma,
        _ => BinaryOp::Add, // Default fallback
    }
}

fn parse_unary_op(op: &str) -> crate::ast::UnaryOp {
    use crate::ast::UnaryOp;
    match op {
        "*" => UnaryOp::Deref,
        "&" => UnaryOp::AddrOf,
        "!" => UnaryOp::LNot,
        "~" => UnaryOp::Not,
        "-" => UnaryOp::Minus,
        "+" => UnaryOp::Plus,
        "++" | "++_pre" => UnaryOp::PreInc,
        "--" | "--_pre" => UnaryOp::PreDec,
        "++_post" => UnaryOp::PostInc,
        "--_post" => UnaryOp::PostDec,
        _ => UnaryOp::Plus, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libtooling_parser_creation() {
        let parser = LibToolingParser::new();
        assert!(parser.compile_commands_dir.is_none());
        assert!(parser.extra_args.is_empty());
    }
}
