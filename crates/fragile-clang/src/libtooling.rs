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

    /// Detect the path to vendored libc++ headers.
    /// Looks for vendor/llvm-project/libcxx/include/.
    fn detect_vendored_libcxx_path() -> Option<String> {
        // Try relative paths from the current working directory
        let candidates = [
            "vendor/llvm-project/libcxx/include",
            "../vendor/llvm-project/libcxx/include",
            "../../vendor/llvm-project/libcxx/include",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                return std::fs::canonicalize(candidate)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
            }
        }

        // Try from FRAGILE_ROOT environment variable
        if let Ok(root) = std::env::var("FRAGILE_ROOT") {
            let path = Path::new(&root).join("vendor/llvm-project/libcxx/include");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }

        None
    }

    /// Detect the path to vendored libc++ config (contains __config_site).
    fn detect_vendored_libcxx_config_path() -> Option<String> {
        let candidates = [
            "vendor/libcxx-config",
            "../vendor/libcxx-config",
            "../../vendor/libcxx-config",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                return std::fs::canonicalize(candidate)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
            }
        }

        if let Ok(root) = std::env::var("FRAGILE_ROOT") {
            let path = Path::new(&root).join("vendor/libcxx-config");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }

        None
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

        // Build extra args: combine user-specified args with vendored libc++ paths
        let mut all_extra_args: Vec<String> = self.extra_args.clone();

        // Auto-detect vendored libc++ paths (same as ClangParser)
        // This ensures LibTooling uses the same headers as libclang
        if let Some(libcxx_config_path) = Self::detect_vendored_libcxx_config_path() {
            if let Some(libcxx_include_path) = Self::detect_vendored_libcxx_path() {
                // Add libc++ flags
                all_extra_args.push("-stdlib=libc++".to_string());
                all_extra_args.push("-nostdinc++".to_string());
                // Config path first for __config_site
                all_extra_args.push(format!("-isystem{}", libcxx_config_path));
                all_extra_args.push(format!("-isystem{}", libcxx_include_path));
            }
        }

        let extra_args: Vec<&str> = all_extra_args.iter().map(|s| s.as_str()).collect();

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
                let is_instantiation = node.get_bool(6).unwrap_or(false);

                if is_instantiation {
                    let name = node.get_string(0).unwrap_or("").to_string();

                    // Get body (first child if present)
                    let has_body = node
                        .children
                        .first()
                        .and_then(|c| *c)
                        .map(|body_id| ctx.ast_nodes.get(&body_id).is_some())
                        .unwrap_or(false);

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

/// Information about a method from LibTooling, including parameters and body.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Parameter names in order
    pub param_names: Vec<String>,
    /// The method body as a ClangNode
    pub body: ClangNode,
}

/// Extract all template method bodies with parameter information from the LibTooling AST.
///
/// Returns a map with key (class_name, method_name) and value list of MethodInfo structs.
/// Each MethodInfo contains parameter names and the method body.
pub fn extract_method_bodies_with_params(
    ctx: &AstContext,
) -> HashMap<(String, String), Vec<MethodInfo>> {
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

    let mut result: HashMap<(String, String), Vec<MethodInfo>> = HashMap::new();

    for (id, node) in &ctx.ast_nodes {
        if node.tag == ASTEntryTag::TagCXXMethodDecl {
            let method_name = node.get_string(0).unwrap_or("").to_string();
            if method_name.is_empty() {
                continue;
            }

            // Extract parameter names and find body
            let mut param_names = Vec::new();
            let mut body_id = None;

            for child_opt in &node.children {
                if let Some(child_id) = child_opt {
                    if let Some(child_node) = ctx.ast_nodes.get(child_id) {
                        if child_node.tag == ASTEntryTag::TagParmVarDecl {
                            let param_name = child_node.get_string(0).unwrap_or("").to_string();
                            param_names.push(param_name);
                        } else if child_node.tag == ASTEntryTag::TagCompoundStmt {
                            body_id = Some(*child_id);
                        }
                    }
                }
            }

            if let Some(body_id) = body_id {
                if let Some(body_node) = convert_to_clang_node(ctx, body_id) {
                    let class_name = method_to_class.get(id).cloned().unwrap_or_default();

                    let method_info = MethodInfo {
                        param_names: param_names.clone(),
                        body: body_node,
                    };

                    let key = (class_name.clone(), method_name.clone());
                    result.entry(key).or_default().push(method_info.clone());

                    if !class_name.is_empty() {
                        let fallback_key = (String::new(), method_name.clone());
                        result.entry(fallback_key).or_default().push(method_info);
                    }
                }
            }
        }
    }

    result
}

/// Extract all template method bodies from the LibTooling AST.
///
/// Returns a map suitable for use with `AstCodeGen::set_libtooling_bodies()`.
/// Key: (class_name, method_name), Value: list of method body ClangNodes
///
/// Note: Class names are extracted from the parent record declarations.
/// The key uses the raw C++ class name (e.g., "map", "_Rb_tree") without
/// template parameters.
///
/// DEPRECATED: Use extract_method_bodies_with_params() instead for parameter information.
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
            let body_id = node.children.iter().filter_map(|c| *c).find(|child_id| {
                ctx.ast_nodes
                    .get(child_id)
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
            // extras[0] = opcode uint, extras[1] = opcode string (from getOpcodeStr)
            let op_str = node.get_string(1).unwrap_or("+");
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
            ClangNodeKind::VarDecl {
                name,
                ty,
                has_init,
                is_static: false,
                is_extern: false,
            }
        }

        ASTEntryTag::TagUnaryOperator => {
            // extras[0] = opcode uint (UO_PostInc=0..UO_Coawait=13), extras[1] = isPrefix bool
            let op = if let Some(opcode) = node.get_int(0) {
                parse_unary_op_from_opcode(opcode as u32)
            } else {
                crate::ast::UnaryOp::Plus // fallback
            };
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

        ASTEntryTag::TagCStyleCastExpr
        | ASTEntryTag::TagCXXStaticCastExpr
        | ASTEntryTag::TagCXXReinterpretCastExpr
        | ASTEntryTag::TagCXXConstCastExpr
        | ASTEntryTag::TagCXXFunctionalCastExpr
        | ASTEntryTag::TagCXXDynamicCastExpr => {
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
            // extras[0] = opcode uint, extras[1] = opcode string (from getOpcodeStr)
            let op_str = node.get_string(1).unwrap_or("+=");
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
            // Use dedicated CXXConstructExpr node to distinguish from function calls
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXConstructExpr { ty }
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

        ASTEntryTag::TagCXXDeleteExpr => ClangNodeKind::CXXDeleteExpr { is_array: false },

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

        // Declaration nodes that appear within method bodies
        ASTEntryTag::TagFieldDecl => {
            // Field declarations - we treat these as DeclRefExpr for transpilation
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::DeclRefExpr {
                name,
                ty,
                namespace_path: vec![],
            }
        }

        ASTEntryTag::TagCXXMethodDecl => {
            // Method declarations within bodies - skip (these are inline definitions)
            // Return a placeholder that won't generate code
            ClangNodeKind::Unknown("InlineMethodDecl".to_string())
        }

        ASTEntryTag::TagCXXConstructorDecl | ASTEntryTag::TagCXXDestructorDecl => {
            // Constructor/destructor declarations - skip
            ClangNodeKind::Unknown("InlineSpecialMemberDecl".to_string())
        }

        ASTEntryTag::TagParmVarDecl => {
            // Parameter declarations
            let name = node.get_string(0).unwrap_or("").to_string();
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::VarDecl {
                name,
                ty,
                has_init: false,
                is_static: false,
                is_extern: false,
            }
        }

        // Additional statement types
        ASTEntryTag::TagDoStmt => ClangNodeKind::DoStmt,

        ASTEntryTag::TagSwitchStmt => ClangNodeKind::SwitchStmt,

        ASTEntryTag::TagCaseStmt => {
            // Case statements have a value expression and body
            // Try to extract the case value
            let value = node.get_int(0).unwrap_or(0) as i128;
            ClangNodeKind::CaseStmt {
                value,
                enum_name: None,
            }
        }

        ASTEntryTag::TagDefaultStmt => ClangNodeKind::DefaultStmt,

        ASTEntryTag::TagGotoStmt => {
            let label = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::GotoStmt { label }
        }

        ASTEntryTag::TagLabelStmt => {
            let label = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::LabelStmt { label }
        }

        ASTEntryTag::TagNullStmt => ClangNodeKind::NullStmt,

        ASTEntryTag::TagCXXForRangeStmt => {
            // Range-based for loop - extract var name and type
            let var_name = node.get_string(0).unwrap_or("item").to_string();
            let var_type = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXForRangeStmt { var_name, var_type }
        }

        ASTEntryTag::TagCXXTryStmt => ClangNodeKind::TryStmt,

        ASTEntryTag::TagCXXCatchStmt => {
            let exception_ty = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::CatchStmt { exception_ty }
        }

        ASTEntryTag::TagCXXThrowExpr => {
            let exception_ty = Some(extract_type_from_node(ctx, node));
            ClangNodeKind::ThrowExpr { exception_ty }
        }

        // Additional expression types
        ASTEntryTag::TagUnaryExprOrTypeTraitExpr => {
            // sizeof, alignof, etc.
            let kind_str = node.get_string(0).unwrap_or("sizeof").to_string();
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::UnaryExprOrTypeTraitExpr {
                kind: kind_str,
                argument_type: Some(ty),
            }
        }

        ASTEntryTag::TagCXXStdInitializerListExpr => {
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::InitListExpr { ty }
        }

        ASTEntryTag::TagLambdaExpr => ClangNodeKind::LambdaExpr {
            params: vec![],
            return_type: CppType::Void,
            capture_default: crate::ast::CaptureDefault::None,
            captures: vec![],
        },

        ASTEntryTag::TagTypeTraitExpr => {
            // Type traits like std::is_same_v
            ClangNodeKind::BoolLiteral(false) // Placeholder
        }

        ASTEntryTag::TagImplicitValueInitExpr | ASTEntryTag::TagCXXScalarValueInitExpr => {
            // Value initialization - generates default/zero value
            let ty = extract_type_from_node(ctx, node);
            ClangNodeKind::CXXDefaultInitExpr { ty }
        }

        // Record/class related declarations that might appear in bodies
        ASTEntryTag::TagCXXRecordDecl | ASTEntryTag::TagClassTemplateSpecializationDecl => {
            // Inline class/struct definition within method - skip
            ClangNodeKind::Unknown("InlineClassDecl".to_string())
        }

        ASTEntryTag::TagTypedefDecl | ASTEntryTag::TagTypeAliasDecl => {
            // Inline typedef/using - skip
            ClangNodeKind::Unknown("InlineTypedef".to_string())
        }

        ASTEntryTag::TagEnumDecl => ClangNodeKind::Unknown("InlineEnumDecl".to_string()),

        ASTEntryTag::TagEnumConstantDecl => {
            let name = node.get_string(0).unwrap_or("").to_string();
            ClangNodeKind::DeclRefExpr {
                name,
                ty: CppType::Int { signed: true },
                namespace_path: vec![],
            }
        }

        ASTEntryTag::TagAccessSpecDecl => {
            // public/private/protected - skip
            ClangNodeKind::Unknown("AccessSpec".to_string())
        }

        ASTEntryTag::TagStaticAssertDecl => ClangNodeKind::Unknown("StaticAssert".to_string()),

        ASTEntryTag::TagFunctionDecl => convert_function_decl_node(ctx, node),

        ASTEntryTag::TagFunctionTemplateDecl => {
            // Function template conversion requires template parameter extraction.
            // Keep this mapped to Unknown until that surface is fully modeled.
            ClangNodeKind::Unknown("InlineFunctionTemplateDecl".to_string())
        }

        ASTEntryTag::TagClassTemplateDecl => {
            ClangNodeKind::Unknown("InlineClassTemplateDecl".to_string())
        }

        ASTEntryTag::TagNamespaceDecl
        | ASTEntryTag::TagUsingDecl
        | ASTEntryTag::TagUsingDirectiveDecl => {
            ClangNodeKind::Unknown("NamespaceRelated".to_string())
        }

        ASTEntryTag::TagTemplateTypeParmDecl
        | ASTEntryTag::TagNonTypeTemplateParmDecl
        | ASTEntryTag::TagTemplateTemplateParmDecl => {
            ClangNodeKind::Unknown("TemplateParam".to_string())
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

fn convert_function_decl_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> ClangNodeKind {
    let raw_name = node.get_string(0).unwrap_or("").to_string();
    let name = if raw_name.is_empty() {
        "__fragile_libtooling_anon_fn".to_string()
    } else {
        raw_name
    };
    let is_static = node.get_bool(3).unwrap_or(false);

    let (return_type_opt, fn_param_types, is_variadic) = if let Some(type_id) = node.type_id {
        resolve_function_proto_type(ctx, type_id)
    } else {
        (None, Vec::new(), false)
    };

    let return_type = return_type_opt.unwrap_or(CppType::Int { signed: true });

    let mut params: Vec<(String, CppType)> = Vec::new();
    let mut param_index = 0usize;
    for child_id_opt in &node.children {
        let Some(child_id) = child_id_opt else {
            continue;
        };
        let Some(child_node) = ctx.ast_nodes.get(child_id) else {
            continue;
        };
        if child_node.tag != ASTEntryTag::TagParmVarDecl {
            continue;
        }

        let raw_param_name = child_node.get_string(0).unwrap_or("").to_string();
        let param_name = if raw_param_name.is_empty() {
            format!("arg{param_index}")
        } else {
            raw_param_name
        };
        let param_type = child_node
            .type_id
            .and_then(|type_id| resolve_type(ctx, type_id))
            .or_else(|| fn_param_types.get(param_index).cloned())
            .unwrap_or_else(|| CppType::Named("auto".to_string()));
        params.push((param_name, param_type));
        param_index += 1;
    }

    // If ParmVarDecl children are unavailable, preserve parameter arity/types
    // from FunctionProtoType to avoid dropping callable function surfaces.
    if params.is_empty() && !fn_param_types.is_empty() {
        for (idx, ty) in fn_param_types.iter().cloned().enumerate() {
            params.push((format!("arg{idx}"), ty));
        }
    }

    let is_definition = node.children.iter().any(|child_id_opt| {
        child_id_opt
            .and_then(|child_id| ctx.ast_nodes.get(&child_id))
            .is_some_and(|child_node| child_node.tag == ASTEntryTag::TagCompoundStmt)
    });

    ClangNodeKind::FunctionDecl {
        name: name.clone(),
        mangled_name: name,
        is_static,
        return_type,
        params,
        is_definition,
        is_variadic,
        is_noexcept: false,
        is_coroutine: false,
        coroutine_info: None,
    }
}

fn extract_type_from_node(
    ctx: &AstContext,
    node: &fragile_ast_exporter::clang_ast::AstNode,
) -> CppType {
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
                ASTEntryTag::TagChar | ASTEntryTag::TagSChar => {
                    return CppType::Char { signed: true }
                }
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
                ASTEntryTag::TagRecordType
                | ASTEntryTag::TagElaboratedType
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
        "&" => BinaryOp::And, // Bitwise AND
        "|" => BinaryOp::Or,  // Bitwise OR
        "^" => BinaryOp::Xor, // Bitwise XOR
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

fn parse_unary_op_from_opcode(opcode: u32) -> crate::ast::UnaryOp {
    use crate::ast::UnaryOp;
    // Matches Clang's UnaryOperatorKind enum (ast_tags.hpp)
    match opcode {
        0 => UnaryOp::PostInc, // UO_PostInc
        1 => UnaryOp::PostDec, // UO_PostDec
        2 => UnaryOp::PreInc,  // UO_PreInc
        3 => UnaryOp::PreDec,  // UO_PreDec
        4 => UnaryOp::AddrOf,  // UO_AddrOf
        5 => UnaryOp::Deref,   // UO_Deref
        6 => UnaryOp::Plus,    // UO_Plus
        7 => UnaryOp::Minus,   // UO_Minus
        8 => UnaryOp::Not,     // UO_Not (~)
        9 => UnaryOp::LNot,    // UO_LNot (!)
        _ => UnaryOp::Plus,    // fallback
    }
}

/// Information about a template specialization's fields with resolved types.
#[derive(Debug, Clone)]
pub struct SpecializationFieldInfo {
    /// Name of the specialized type (e.g., "map" for std::map<int, int>)
    pub type_name: String,
    /// Qualified name with template args (e.g., "std::map<int, int>")
    pub qualified_name: String,
    /// Template arguments as strings (e.g., ["int", "int", "std::less<int>", ...])
    pub template_args: Vec<String>,
    /// Map from field name to its resolved C++ type
    pub field_types: HashMap<String, CppType>,
}

/// Extract resolved field types from template specializations.
///
/// This function looks at ClassTemplateSpecializationDecl nodes and extracts
/// the fully-substituted types for each field. This is crucial for generating
/// correct Rust struct definitions where template parameters have been replaced
/// with concrete types.
///
/// # Returns
/// A map from specialized type qualified name to field information.
pub fn extract_specialization_field_types(
    ctx: &AstContext,
) -> HashMap<String, SpecializationFieldInfo> {
    use fragile_ast_exporter::CborValue;

    let mut result = HashMap::new();

    for (_spec_id, node) in &ctx.ast_nodes {
        if node.tag != ASTEntryTag::TagClassTemplateSpecializationDecl {
            continue;
        }

        let type_name = node.get_string(0).unwrap_or("").to_string();
        let qualified_name = node.get_string(1).unwrap_or("").to_string();

        // Extract template arguments from extras[2] (an array of [kind, value] pairs)
        let mut template_args = Vec::new();
        if let Some(CborValue::Array(args)) = node.extras.get(2) {
            for arg in args {
                if let CborValue::Array(pair) = arg {
                    // pair[0] is the kind, pair[1] is the string representation
                    if let Some(CborValue::Text(arg_str)) = pair.get(1) {
                        template_args.push(arg_str.clone());
                    }
                }
            }
        }

        // Extract field types from child FieldDecl nodes
        let mut field_types = HashMap::new();
        for child_id_opt in &node.children {
            if let Some(child_id) = child_id_opt {
                if let Some(child_node) = ctx.ast_nodes.get(child_id) {
                    if child_node.tag == ASTEntryTag::TagFieldDecl {
                        let field_name = child_node.get_string(0).unwrap_or("").to_string();
                        if field_name.is_empty() {
                            continue;
                        }

                        // Get the resolved type
                        if let Some(type_id) = child_node.type_id {
                            if let Some(resolved_type) = resolve_type(ctx, type_id) {
                                field_types.insert(field_name, resolved_type);
                            }
                        }
                    }
                }
            }
        }

        // Only add if we have useful information
        if !qualified_name.is_empty() && !field_types.is_empty() {
            // Build the full specialization name: std::pair<const int, int>
            // This matches the format used in generate_template_struct
            let full_name = if template_args.is_empty() {
                qualified_name.clone()
            } else {
                format!("{}<{}>", qualified_name, template_args.join(", "))
            };

            result.insert(
                full_name.clone(),
                SpecializationFieldInfo {
                    type_name,
                    qualified_name: full_name,
                    template_args,
                    field_types,
                },
            );
        }
    }

    if std::env::var("FRAGILE_DEBUG_SPECIALIZATION").is_ok() {
        eprintln!(
            "[SPEC DEBUG] Total specializations extracted: {}",
            result.len()
        );
    }

    result
}

/// A resolved method signature from a template specialization.
#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub return_type: Option<CppType>,
    pub param_names: Vec<String>,
    pub param_types: Vec<CppType>,
    pub is_static: bool,
}

/// Extract resolved method signatures from template specializations.
///
/// For each ClassTemplateSpecializationDecl, extracts CXXMethodDecl children
/// and resolves their return types and parameter types via the FunctionProtoType.
/// This gives us fully substituted types (e.g., `int*` instead of `_Tp*`).
pub fn extract_specialization_method_signatures(
    ctx: &AstContext,
) -> HashMap<String, Vec<MethodSignature>> {
    use fragile_ast_exporter::CborValue;

    let mut result = HashMap::new();

    for (_spec_id, node) in &ctx.ast_nodes {
        if node.tag != ASTEntryTag::TagClassTemplateSpecializationDecl {
            continue;
        }

        let _type_name = node.get_string(0).unwrap_or("").to_string();
        let qualified_name = node.get_string(1).unwrap_or("").to_string();

        // Build full specialization name (same as extract_specialization_field_types)
        let mut template_args = Vec::new();
        if let Some(CborValue::Array(args)) = node.extras.get(2) {
            for arg in args {
                if let CborValue::Array(pair) = arg {
                    if let Some(CborValue::Text(arg_str)) = pair.get(1) {
                        template_args.push(arg_str.clone());
                    }
                }
            }
        }

        let full_name = if template_args.is_empty() {
            qualified_name.clone()
        } else {
            format!("{}<{}>", qualified_name, template_args.join(", "))
        };

        if full_name.is_empty() {
            continue;
        }

        let mut methods = Vec::new();

        for child_id_opt in &node.children {
            let Some(child_id) = child_id_opt else {
                continue;
            };
            let Some(child_node) = ctx.ast_nodes.get(child_id) else {
                continue;
            };
            if child_node.tag != ASTEntryTag::TagCXXMethodDecl {
                continue;
            }

            let method_name = child_node.get_string(0).unwrap_or("").to_string();
            if method_name.is_empty() {
                continue;
            }
            let is_static = child_node.get_bool(1).unwrap_or(false);

            // Resolve return type and param types via FunctionProtoType
            let (return_type, fn_param_types, _) = if let Some(type_id) = child_node.type_id {
                resolve_function_proto_type(ctx, type_id)
            } else {
                (None, Vec::new(), false)
            };

            // Extract parameter names from ParmVarDecl children,
            // and resolve their types directly from their type_id
            let mut param_names = Vec::new();
            let mut param_types = Vec::new();
            let mut parm_idx = 0;
            for grandchild_opt in &child_node.children {
                let Some(grandchild_id) = grandchild_opt else {
                    continue;
                };
                let Some(grandchild) = ctx.ast_nodes.get(grandchild_id) else {
                    continue;
                };
                if grandchild.tag != ASTEntryTag::TagParmVarDecl {
                    continue;
                }
                let pname = grandchild.get_string(0).unwrap_or("").to_string();
                param_names.push(pname);

                // Prefer ParmVarDecl's own type_id (most direct),
                // fall back to FunctionProtoType's param type
                let ptype = grandchild
                    .type_id
                    .and_then(|tid| resolve_type(ctx, tid))
                    .or_else(|| fn_param_types.get(parm_idx).cloned());
                if let Some(t) = ptype {
                    param_types.push(t);
                }
                parm_idx += 1;
            }

            methods.push(MethodSignature {
                name: method_name,
                return_type,
                param_names,
                param_types,
                is_static,
            });
        }

        if !methods.is_empty() {
            // Store under full specialization name AND simple name for fuzzy lookup
            result.insert(full_name.clone(), methods.clone());

            // Also store under "qualified_name<args>" without "std::" prefix for matching
            if let Some(stripped) = full_name.strip_prefix("std::") {
                result.insert(stripped.to_string(), methods);
            }
        }
    }

    if std::env::var("FRAGILE_DEBUG_SPECIALIZATION").is_ok() {
        eprintln!(
            "[SPEC DEBUG] Specialization method signatures extracted: {}",
            result.len()
        );
        for (key, methods) in &result {
            eprintln!("  {}: {} methods", key, methods.len());
            for m in methods {
                eprintln!(
                    "    {}({}) -> {:?}",
                    m.name,
                    m.param_types
                        .iter()
                        .map(|t| format!("{:?}", t))
                        .collect::<Vec<_>>()
                        .join(", "),
                    m.return_type,
                );
            }
        }
    }

    result
}

/// Resolve a FunctionProtoType to extract return type, parameter types, and variadic shape.
fn resolve_function_proto_type(
    ctx: &AstContext,
    type_id: u64,
) -> (Option<CppType>, Vec<CppType>, bool) {
    use fragile_ast_exporter::clang_ast::TypeNode;
    use fragile_ast_exporter::CborValue;

    let type_node = match ctx.get_type(TypeNode::unqualified_id(type_id)) {
        Some(t) => t,
        None => return (None, Vec::new(), false),
    };

    if type_node.tag != ASTEntryTag::TagFunctionProtoType {
        return (None, Vec::new(), false);
    }

    // extras[0] = return type ID
    let return_type = match type_node.extras.first() {
        Some(CborValue::Integer(ret_id)) => {
            let ret_id = *ret_id as u64;
            resolve_type(ctx, ret_id)
        }
        _ => None,
    };

    // extras[1] = array of parameter type IDs
    let mut param_types = Vec::new();
    if let Some(CborValue::Array(param_ids)) = type_node.extras.get(1) {
        for param_id in param_ids {
            if let CborValue::Integer(id) = param_id {
                let id = *id as u64;
                if let Some(ptype) = resolve_type(ctx, id) {
                    param_types.push(ptype);
                }
            }
        }
    }

    // extras[2] = isVariadic
    let is_variadic = matches!(type_node.extras.get(2), Some(CborValue::Bool(true)));

    (return_type, param_types, is_variadic)
}

/// Format a CppType as a C++ type string for use in template arguments.
fn format_cpp_type(ty: &CppType) -> String {
    match ty {
        CppType::Void => "void".to_string(),
        CppType::Bool => "bool".to_string(),
        CppType::Char { signed: true } => "char".to_string(),
        CppType::Char { signed: false } => "unsigned char".to_string(),
        CppType::Short { signed: true } => "short".to_string(),
        CppType::Short { signed: false } => "unsigned short".to_string(),
        CppType::Int { signed: true } => "int".to_string(),
        CppType::Int { signed: false } => "unsigned int".to_string(),
        CppType::Long { signed: true } => "long".to_string(),
        CppType::Long { signed: false } => "unsigned long".to_string(),
        CppType::LongLong { signed: true } => "long long".to_string(),
        CppType::LongLong { signed: false } => "unsigned long long".to_string(),
        CppType::Float => "float".to_string(),
        CppType::Double => "double".to_string(),
        CppType::Named(name) => name.clone(),
        CppType::Pointer { pointee, is_const } => {
            if *is_const {
                format!("const {} *", format_cpp_type(pointee))
            } else {
                format!("{} *", format_cpp_type(pointee))
            }
        }
        CppType::Reference {
            referent,
            is_const,
            is_rvalue,
        } => {
            let ref_sym = if *is_rvalue { "&&" } else { "&" };
            if *is_const {
                format!("const {}{}", format_cpp_type(referent), ref_sym)
            } else {
                format!("{}{}", format_cpp_type(referent), ref_sym)
            }
        }
        CppType::Array { element, size } => {
            if let Some(n) = size {
                format!("{}[{}]", format_cpp_type(element), n)
            } else {
                format!("{}[]", format_cpp_type(element))
            }
        }
        CppType::Function {
            return_type,
            params,
            ..
        } => {
            let param_strs: Vec<String> = params.iter().map(format_cpp_type).collect();
            format!(
                "{}({})",
                format_cpp_type(return_type),
                param_strs.join(", ")
            )
        }
        CppType::TemplateParam { name, .. } => name.clone(),
        CppType::DependentType { spelling } => spelling.clone(),
        CppType::ParameterPack { name, .. } => format!("{}...", name),
    }
}

/// Resolve a type ID to a concrete CppType, following SubstTemplateTypeParmType
/// and other wrapper types to get the actual resolved type.
fn resolve_type(ctx: &AstContext, type_id: u64) -> Option<CppType> {
    use fragile_ast_exporter::CborValue;

    let type_node = ctx.get_type(type_id)?;

    match type_node.tag {
        // SubstTemplateTypeParmType contains a reference to the replacement type
        ASTEntryTag::TagSubstTemplateTypeParmType => {
            if let Some(CborValue::Integer(replacement_id)) = type_node.extras.first() {
                let replacement_id = *replacement_id as u64;
                resolve_type(ctx, replacement_id)
            } else {
                None
            }
        }

        // ElaboratedType wraps another type (e.g., "typename Foo::bar")
        ASTEntryTag::TagElaboratedType => {
            if let Some(CborValue::Integer(inner_id)) = type_node.extras.first() {
                let inner_id = *inner_id as u64;
                resolve_type(ctx, inner_id)
            } else {
                None
            }
        }

        // DecltypeType - follow to the underlying type
        // extras[0] = underlying type ID
        ASTEntryTag::TagDecltypeType => {
            if let Some(CborValue::Integer(underlying_id)) = type_node.extras.first() {
                let underlying_id = *underlying_id as u64;
                resolve_type(ctx, underlying_id)
            } else {
                None
            }
        }

        // Typedef type - follow to the underlying type
        // extras[0] = name, extras[1] = underlying type ID
        ASTEntryTag::TagTypedefType => {
            // Always follow the underlying type to get the actual type
            if let Some(CborValue::Integer(underlying_id)) = type_node.extras.get(1) {
                let underlying_id = *underlying_id as u64;
                let result = resolve_type(ctx, underlying_id);
                if result.is_some() {
                    return result;
                }
                // Fall back to typedef name if underlying can't be resolved
                let name = type_node.get_string(0).unwrap_or("").to_string();
                if !name.is_empty() {
                    Some(CppType::Named(name))
                } else {
                    None
                }
            } else {
                // Fallback to typedef name if no underlying type
                let name = type_node.get_string(0).unwrap_or("").to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(CppType::Named(name))
                }
            }
        }

        // Record type (struct/class)
        // extras[0] = decl ID, extras[1] = name
        ASTEntryTag::TagRecordType => {
            // Name is at index 1, not index 0
            let name = type_node.get_string(1).unwrap_or("").to_string();
            if name.is_empty() {
                None
            } else {
                // Clean up the name if it has "struct " or "class " prefix
                let clean_name = name
                    .strip_prefix("struct ")
                    .or_else(|| name.strip_prefix("class "))
                    .unwrap_or(&name)
                    .to_string();
                Some(CppType::Named(clean_name))
            }
        }

        // Pointer type
        ASTEntryTag::TagPointerType => {
            if let Some(CborValue::Integer(pointee_id)) = type_node.extras.first() {
                let pointee_id = *pointee_id as u64;
                // Check const qualifier bit (bit 0) from encodeQualType
                let is_const = (pointee_id & 0x1) != 0;
                if let Some(pointee_type) = resolve_type(ctx, pointee_id) {
                    Some(CppType::Pointer {
                        pointee: Box::new(pointee_type),
                        is_const,
                    })
                } else {
                    Some(CppType::Pointer {
                        pointee: Box::new(CppType::Void),
                        is_const,
                    })
                }
            } else {
                Some(CppType::Pointer {
                    pointee: Box::new(CppType::Void),
                    is_const: false,
                })
            }
        }

        // Reference types (lvalue & rvalue)
        ASTEntryTag::TagLValueReferenceType | ASTEntryTag::TagRValueReferenceType => {
            let is_rvalue = type_node.tag == ASTEntryTag::TagRValueReferenceType;
            if let Some(CborValue::Integer(ref_id)) = type_node.extras.first() {
                let ref_id = *ref_id as u64;
                // Check const qualifier bit (bit 0) from encodeQualType
                let is_const = (ref_id & 0x1) != 0;
                if let Some(ref_type) = resolve_type(ctx, ref_id) {
                    Some(CppType::Reference {
                        referent: Box::new(ref_type),
                        is_const,
                        is_rvalue,
                    })
                } else {
                    Some(CppType::Reference {
                        referent: Box::new(CppType::Void),
                        is_const,
                        is_rvalue,
                    })
                }
            } else {
                Some(CppType::Reference {
                    referent: Box::new(CppType::Void),
                    is_const: false,
                    is_rvalue,
                })
            }
        }

        // Primitive types
        ASTEntryTag::TagInt => Some(CppType::Int { signed: true }),
        ASTEntryTag::TagUInt => Some(CppType::Int { signed: false }),
        ASTEntryTag::TagLong => Some(CppType::Long { signed: true }),
        ASTEntryTag::TagULong => Some(CppType::Long { signed: false }),
        ASTEntryTag::TagLongLong => Some(CppType::LongLong { signed: true }),
        ASTEntryTag::TagULongLong => Some(CppType::LongLong { signed: false }),
        ASTEntryTag::TagShort => Some(CppType::Short { signed: true }),
        ASTEntryTag::TagUShort => Some(CppType::Short { signed: false }),
        ASTEntryTag::TagChar => Some(CppType::Char { signed: true }),
        ASTEntryTag::TagSChar => Some(CppType::Char { signed: true }),
        ASTEntryTag::TagUChar => Some(CppType::Char { signed: false }),
        ASTEntryTag::TagFloat => Some(CppType::Float),
        ASTEntryTag::TagDouble => Some(CppType::Double),
        ASTEntryTag::TagBool => Some(CppType::Bool),
        ASTEntryTag::TagVoid => Some(CppType::Void),

        // Template specialization type (e.g., std::less<int>)
        // extras[0] = template name (string)
        // extras[1] = array of template argument type IDs
        // extras[2] = aliased type ID (for type alias templates like __type_identity_t)
        ASTEntryTag::TagTemplateSpecializationType => {
            let name = type_node.get_string(0).unwrap_or("").to_string();

            // First check if there's an aliased type (for type alias templates)
            // If so, follow that instead of building the name manually
            if let Some(CborValue::Integer(aliased_id)) = type_node.extras.get(2) {
                let aliased_id = *aliased_id as u64;
                if aliased_id != 0 {
                    // This is a type alias template - follow to the aliased type
                    if let Some(resolved) = resolve_type(ctx, aliased_id) {
                        return Some(resolved);
                    }
                    // If aliased type couldn't be resolved, fall through to manual construction
                }
            }

            if name.is_empty() {
                return None;
            }

            // Try to extract template arguments
            if let Some(CborValue::Array(arg_ids)) = type_node.extras.get(1) {
                let mut args = Vec::new();
                for arg_id in arg_ids {
                    if let CborValue::Integer(id) = arg_id {
                        let id = *id as u64;
                        if id != 0 {
                            if let Some(arg_type) = resolve_type(ctx, id) {
                                args.push(format_cpp_type(&arg_type));
                            }
                            // Note: If resolve_type returns None, we skip the arg
                            // This loses information but avoids broken type names
                        }
                        // Note: id == 0 means non-type template argument, skip it
                    }
                }

                if !args.is_empty() {
                    // Build full template name with args
                    let full_name = format!("{}<{}>", name, args.join(", "));
                    Some(CppType::Named(full_name))
                } else {
                    Some(CppType::Named(name))
                }
            } else {
                Some(CppType::Named(name))
            }
        }

        // Fallback: return the type's string representation as a named type
        _ => {
            let name = type_node.get_string(0).unwrap_or("").to_string();
            if name.is_empty() {
                // Try to use tag name as a hint
                Some(CppType::Named(format!("Unknown{:?}", type_node.tag)))
            } else {
                Some(CppType::Named(name))
            }
        }
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
