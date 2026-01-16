//! Clang AST to MIR conversion.

use crate::ast::{BinaryOp, ClangAst, ClangNode, ClangNodeKind, UnaryOp};
use crate::types::CppType;
use crate::{
    CppBaseClass, CppConstructor, CppDestructor, CppExtern, CppField, CppFriend, CppFunction,
    CppFunctionTemplate, CppMethod, CppModule, CppStruct, MemberInitializer, MirBasicBlock,
    MirBinOp, MirBody, MirConstant, MirLocal, MirOperand, MirPlace, MirRvalue, MirStatement,
    MirTerminator, MirUnaryOp, UsingDeclaration, UsingDirective,
};
use miette::Result;

/// Converter from Clang AST to MIR.
pub struct MirConverter {
    /// Current function being converted
    current_function: Option<FunctionBuilder>,
}

/// Builder for constructing MIR bodies.
struct FunctionBuilder {
    /// Local variables
    locals: Vec<MirLocal>,
    /// Basic blocks
    blocks: Vec<MirBasicBlock>,
    /// Current block index
    current_block: usize,
    /// Statements for current block
    current_statements: Vec<MirStatement>,
    /// Map from variable names to local indices
    var_map: rustc_hash::FxHashMap<String, usize>,
}

impl FunctionBuilder {
    fn new() -> Self {
        Self {
            locals: Vec::new(),
            blocks: Vec::new(),
            current_block: 0,
            current_statements: Vec::new(),
            var_map: rustc_hash::FxHashMap::default(),
        }
    }

    /// Add a local variable and return its index.
    fn add_local(&mut self, name: Option<String>, ty: CppType, is_arg: bool) -> usize {
        let idx = self.locals.len();
        if let Some(ref n) = name {
            self.var_map.insert(n.clone(), idx);
        }
        self.locals.push(MirLocal { name, ty, is_arg });
        idx
    }

    /// Look up a variable by name.
    fn lookup_var(&self, name: &str) -> Option<usize> {
        self.var_map.get(name).copied()
    }

    /// Add a statement to the current block.
    fn add_statement(&mut self, stmt: MirStatement) {
        self.current_statements.push(stmt);
    }

    /// Finish the current block with a terminator.
    fn finish_block(&mut self, terminator: MirTerminator) -> usize {
        let block = MirBasicBlock {
            statements: std::mem::take(&mut self.current_statements),
            terminator,
        };
        let idx = self.blocks.len();
        self.blocks.push(block);
        self.current_block = idx + 1;
        idx
    }

    /// Create a new block and return its index.
    fn new_block(&mut self) -> usize {
        self.blocks.len() + 1 // Next block index
    }

    /// Build the final MIR body.
    fn build(self) -> MirBody {
        MirBody {
            blocks: self.blocks,
            locals: self.locals,
        }
    }
}

impl MirConverter {
    /// Create a new MIR converter.
    pub fn new() -> Self {
        Self {
            current_function: None,
        }
    }

    /// Convert a Clang AST to a C++ module.
    pub fn convert(&self, ast: ClangAst) -> Result<CppModule> {
        let mut module = CppModule::new();
        let namespace_context = Vec::new();
        self.convert_translation_unit(&ast.translation_unit, &mut module, &namespace_context)?;
        Ok(module)
    }

    /// Convert a translation unit (root node).
    fn convert_translation_unit(
        &self,
        node: &ClangNode,
        module: &mut CppModule,
        namespace_context: &[String],
    ) -> Result<()> {
        for child in &node.children {
            self.convert_decl(child, module, namespace_context)?;
        }
        Ok(())
    }

    /// Convert a declaration.
    fn convert_decl(
        &self,
        node: &ClangNode,
        module: &mut CppModule,
        namespace_context: &[String],
    ) -> Result<()> {
        match &node.kind {
            ClangNodeKind::NamespaceDecl { name } => {
                // Build new namespace context
                let mut new_context = namespace_context.to_vec();
                if let Some(ns_name) = name {
                    new_context.push(ns_name.clone());
                }
                // Recursively convert namespace contents
                for child in &node.children {
                    self.convert_decl(child, module, &new_context)?;
                }
            }
            ClangNodeKind::FunctionDecl {
                name,
                return_type,
                params,
                is_definition,
            } => {
                if *is_definition {
                    let func = self.convert_function(node, name, return_type, params, namespace_context)?;
                    module.functions.push(func);
                } else {
                    // Just a declaration, add as extern
                    module.externs.push(CppExtern {
                        mangled_name: name.clone(), // TODO: proper mangling
                        display_name: name.clone(),
                        namespace: namespace_context.to_vec(),
                        params: params.clone(),
                        return_type: return_type.clone(),
                    });
                }
            }
            ClangNodeKind::FunctionTemplateDecl {
                name,
                template_params,
                return_type,
                params,
                is_definition,
            } => {
                module.function_templates.push(CppFunctionTemplate {
                    name: name.clone(),
                    namespace: namespace_context.to_vec(),
                    template_params: template_params.clone(),
                    return_type: return_type.clone(),
                    params: params.clone(),
                    is_definition: *is_definition,
                });
            }
            ClangNodeKind::TemplateTypeParmDecl { .. } => {
                // Template type parameters are handled as part of FunctionTemplateDecl
                // No separate processing needed
            }
            ClangNodeKind::RecordDecl {
                name,
                is_class,
                fields: _,
            } => {
                let struct_def = self.convert_struct(node, name, *is_class, namespace_context)?;
                module.structs.push(struct_def);
            }
            ClangNodeKind::UsingDirective { namespace } => {
                module.using_directives.push(UsingDirective {
                    namespace: namespace.clone(),
                    scope: namespace_context.to_vec(),
                });
            }
            ClangNodeKind::UsingDeclaration { qualified_name } => {
                module.using_declarations.push(UsingDeclaration {
                    qualified_name: qualified_name.clone(),
                    scope: namespace_context.to_vec(),
                });
            }
            _ => {
                // Skip other declarations for now
            }
        }
        Ok(())
    }

    /// Convert a function definition to MIR.
    fn convert_function(
        &self,
        node: &ClangNode,
        name: &str,
        return_type: &CppType,
        params: &[(String, CppType)],
        namespace_context: &[String],
    ) -> Result<CppFunction> {
        let mut builder = FunctionBuilder::new();

        // Local 0 is always the return place
        builder.add_local(None, return_type.clone(), false);

        // Add parameters as locals
        for (param_name, param_ty) in params {
            builder.add_local(Some(param_name.clone()), param_ty.clone(), true);
        }

        // Find the compound statement (function body)
        for child in &node.children {
            if matches!(child.kind, ClangNodeKind::CompoundStmt) {
                self.convert_compound_stmt(child, &mut builder)?;
                break;
            }
        }

        // If no explicit return, add one
        if builder.current_statements.is_empty() && builder.blocks.is_empty() {
            builder.finish_block(MirTerminator::Return);
        }

        let mir_body = builder.build();

        Ok(CppFunction {
            mangled_name: name.to_string(), // TODO: proper C++ mangling
            display_name: name.to_string(),
            namespace: namespace_context.to_vec(),
            params: params.to_vec(),
            return_type: return_type.clone(),
            mir_body,
        })
    }

    /// Convert just a function/method body to MIR (without creating CppFunction).
    fn convert_method_body(
        &self,
        node: &ClangNode,
        return_type: &CppType,
        params: &[(String, CppType)],
    ) -> Result<MirBody> {
        let mut builder = FunctionBuilder::new();

        // Local 0 is always the return place
        builder.add_local(None, return_type.clone(), false);

        // Add parameters as locals
        for (param_name, param_ty) in params {
            builder.add_local(Some(param_name.clone()), param_ty.clone(), true);
        }

        // Find the compound statement (function body)
        for child in &node.children {
            if matches!(child.kind, ClangNodeKind::CompoundStmt) {
                self.convert_compound_stmt(child, &mut builder)?;
                break;
            }
        }

        // If no explicit return, add one
        if builder.current_statements.is_empty() && builder.blocks.is_empty() {
            builder.finish_block(MirTerminator::Return);
        }

        Ok(builder.build())
    }

    /// Convert a compound statement (block).
    fn convert_compound_stmt(&self, node: &ClangNode, builder: &mut FunctionBuilder) -> Result<()> {
        for child in &node.children {
            self.convert_stmt(child, builder)?;
        }
        Ok(())
    }

    /// Convert a statement.
    fn convert_stmt(&self, node: &ClangNode, builder: &mut FunctionBuilder) -> Result<()> {
        match &node.kind {
            ClangNodeKind::ReturnStmt => {
                if let Some(expr) = node.children.first() {
                    // Evaluate expression and store in return place (_0)
                    let operand = self.convert_expr(expr, builder)?;
                    builder.add_statement(MirStatement::Assign {
                        target: MirPlace::local(0),
                        value: MirRvalue::Use(operand),
                    });
                }
                builder.finish_block(MirTerminator::Return);
            }

            ClangNodeKind::DeclStmt => {
                // Variable declaration
                for child in &node.children {
                    if let ClangNodeKind::VarDecl { name, ty, has_init: _ } = &child.kind {
                        let local_idx = builder.add_local(Some(name.clone()), ty.clone(), false);

                        // Check for initializer (first child of VarDecl)
                        if let Some(init_expr) = child.children.first() {
                            let operand = self.convert_expr(init_expr, builder)?;
                            builder.add_statement(MirStatement::Assign {
                                target: MirPlace::local(local_idx),
                                value: MirRvalue::Use(operand),
                            });
                        }
                    }
                }
            }

            ClangNodeKind::CompoundStmt => {
                self.convert_compound_stmt(node, builder)?;
            }

            ClangNodeKind::IfStmt => {
                // Children: condition, then-branch, [else-branch]
                if node.children.len() >= 2 {
                    let condition = &node.children[0];
                    let then_branch = &node.children[1];
                    let else_branch = node.children.get(2);

                    let cond_operand = self.convert_expr(condition, builder)?;

                    // We'll create blocks for then, else, and merge
                    let then_block = builder.new_block();
                    let merge_block = if else_branch.is_some() {
                        builder.new_block() + 1
                    } else {
                        builder.new_block()
                    };
                    let else_block = if else_branch.is_some() {
                        builder.new_block()
                    } else {
                        merge_block
                    };

                    // Finish current block with switch
                    builder.finish_block(MirTerminator::SwitchInt {
                        operand: cond_operand,
                        targets: vec![(1, then_block)], // if true, goto then
                        otherwise: else_block,
                    });

                    // Convert then branch
                    self.convert_stmt(then_branch, builder)?;
                    builder.finish_block(MirTerminator::Goto {
                        target: merge_block,
                    });

                    // Convert else branch if present
                    if let Some(else_stmt) = else_branch {
                        self.convert_stmt(else_stmt, builder)?;
                        builder.finish_block(MirTerminator::Goto {
                            target: merge_block,
                        });
                    }
                }
            }

            ClangNodeKind::WhileStmt => {
                // Children: condition, body
                if node.children.len() >= 2 {
                    let condition = &node.children[0];
                    let body = &node.children[1];

                    let loop_header = builder.new_block();
                    let loop_body = loop_header + 1;
                    let loop_exit = loop_body + 1;

                    // Jump to loop header
                    builder.finish_block(MirTerminator::Goto {
                        target: loop_header,
                    });

                    // Loop header: evaluate condition
                    let cond_operand = self.convert_expr(condition, builder)?;
                    builder.finish_block(MirTerminator::SwitchInt {
                        operand: cond_operand,
                        targets: vec![(1, loop_body)],
                        otherwise: loop_exit,
                    });

                    // Loop body
                    self.convert_stmt(body, builder)?;
                    builder.finish_block(MirTerminator::Goto {
                        target: loop_header,
                    });
                }
            }

            ClangNodeKind::BreakStmt => {
                // TODO: Need to track loop context to know where to jump
                builder.finish_block(MirTerminator::Unreachable);
            }

            ClangNodeKind::ContinueStmt => {
                // TODO: Need to track loop context to know where to jump
                builder.finish_block(MirTerminator::Unreachable);
            }

            // Expression statement
            _ => {
                if is_expression_kind(&node.kind) {
                    // Evaluate for side effects, discard result
                    let _ = self.convert_expr(node, builder)?;
                }
            }
        }
        Ok(())
    }

    /// Convert an expression and return the operand.
    fn convert_expr(&self, node: &ClangNode, builder: &mut FunctionBuilder) -> Result<MirOperand> {
        match &node.kind {
            ClangNodeKind::IntegerLiteral(value) => {
                Ok(MirOperand::Constant(MirConstant::Int {
                    value: *value,
                    bits: 32, // Default to i32
                }))
            }

            ClangNodeKind::FloatingLiteral(value) => {
                Ok(MirOperand::Constant(MirConstant::Float {
                    value: *value,
                    bits: 64, // Default to f64
                }))
            }

            ClangNodeKind::BoolLiteral(value) => {
                Ok(MirOperand::Constant(MirConstant::Bool(*value)))
            }

            ClangNodeKind::DeclRefExpr { name, ty } => {
                if let Some(local_idx) = builder.lookup_var(name) {
                    Ok(MirOperand::Copy(MirPlace::local(local_idx)))
                } else {
                    // Unknown variable - create a temporary
                    let local_idx = builder.add_local(Some(name.clone()), ty.clone(), false);
                    Ok(MirOperand::Copy(MirPlace::local(local_idx)))
                }
            }

            ClangNodeKind::BinaryOperator { op, ty } => {
                // Binary operators have 2 children: left and right
                if node.children.len() >= 2 {
                    let left = self.convert_expr(&node.children[0], builder)?;
                    let right = self.convert_expr(&node.children[1], builder)?;

                    // Check for assignment
                    if matches!(op, BinaryOp::Assign) {
                        // Left side should be a place
                        if let MirOperand::Copy(place) = left {
                            builder.add_statement(MirStatement::Assign {
                                target: place.clone(),
                                value: MirRvalue::Use(right),
                            });
                            return Ok(MirOperand::Copy(place));
                        }
                    }

                    // Create a temporary for the result
                    let result_local = builder.add_local(None, ty.clone(), false);
                    let mir_op = convert_binop(*op);

                    builder.add_statement(MirStatement::Assign {
                        target: MirPlace::local(result_local),
                        value: MirRvalue::BinaryOp {
                            op: mir_op,
                            left,
                            right,
                        },
                    });

                    Ok(MirOperand::Copy(MirPlace::local(result_local)))
                } else {
                    // Malformed - return zero
                    Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32 }))
                }
            }

            ClangNodeKind::UnaryOperator { op, ty } => {
                if let Some(operand_node) = node.children.first() {
                    let operand = self.convert_expr(operand_node, builder)?;
                    let result_local = builder.add_local(None, ty.clone(), false);

                    let mir_op = convert_unaryop(*op);
                    builder.add_statement(MirStatement::Assign {
                        target: MirPlace::local(result_local),
                        value: MirRvalue::UnaryOp { op: mir_op, operand },
                    });

                    Ok(MirOperand::Copy(MirPlace::local(result_local)))
                } else {
                    Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32 }))
                }
            }

            ClangNodeKind::CallExpr { ty } => {
                // First child is the function reference (may be wrapped in ImplicitCastExpr),
                // rest are arguments
                if let Some(func_ref) = node.children.first() {
                    let func_name = Self::extract_function_name(func_ref);

                    let mut args = Vec::new();
                    for arg_node in node.children.iter().skip(1) {
                        args.push(self.convert_expr(arg_node, builder)?);
                    }

                    let result_local = builder.add_local(None, ty.clone(), false);
                    let destination = MirPlace::local(result_local);

                    let next_block = builder.new_block();
                    builder.finish_block(MirTerminator::Call {
                        func: func_name,
                        args,
                        destination: destination.clone(),
                        target: Some(next_block),
                    });

                    Ok(MirOperand::Copy(destination))
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            ClangNodeKind::ParenExpr { .. } => {
                // Just unwrap the parentheses
                if let Some(inner) = node.children.first() {
                    self.convert_expr(inner, builder)
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            ClangNodeKind::ImplicitCastExpr { .. } | ClangNodeKind::CastExpr { .. } => {
                // For now, just unwrap casts
                // TODO: Handle actual type conversions
                if let Some(inner) = node.children.first() {
                    self.convert_expr(inner, builder)
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            _ => {
                // Unknown expression - return unit
                Ok(MirOperand::Constant(MirConstant::Unit))
            }
        }
    }

    /// Convert a struct/class definition.
    fn convert_struct(
        &self,
        node: &ClangNode,
        name: &str,
        is_class: bool,
        namespace_context: &[String],
    ) -> Result<CppStruct> {
        let mut bases = Vec::new();
        let mut fields = Vec::new();
        let mut static_fields = Vec::new();
        let mut constructors = Vec::new();
        let mut destructor = None;
        let mut methods = Vec::new();
        let mut friends = Vec::new();

        for child in &node.children {
            match &child.kind {
                ClangNodeKind::FieldDecl { name: field_name, ty, access, is_static } => {
                    let field = CppField {
                        name: field_name.clone(),
                        ty: ty.clone(),
                        access: *access,
                    };
                    if *is_static {
                        static_fields.push(field);
                    } else {
                        fields.push(field);
                    }
                }
                ClangNodeKind::CXXMethodDecl {
                    name: method_name,
                    return_type,
                    params,
                    is_definition,
                    is_static,
                    is_virtual,
                    is_pure_virtual,
                    is_override,
                    is_final,
                    access,
                } => {
                    let mir_body = if *is_definition {
                        Some(self.convert_method_body(child, return_type, params)?)
                    } else {
                        None
                    };
                    methods.push(CppMethod {
                        name: method_name.clone(),
                        return_type: return_type.clone(),
                        params: params.clone(),
                        is_static: *is_static,
                        is_virtual: *is_virtual,
                        is_pure_virtual: *is_pure_virtual,
                        is_override: *is_override,
                        is_final: *is_final,
                        access: *access,
                        mir_body,
                    });
                }
                ClangNodeKind::ConstructorDecl {
                    class_name: _,
                    params,
                    is_definition,
                    ctor_kind,
                    access,
                } => {
                    // Extract member initializers from constructor children
                    let member_initializers = self.extract_member_initializers(child);

                    let mir_body = if *is_definition {
                        Some(self.convert_constructor_body(child, params)?)
                    } else {
                        None
                    };
                    constructors.push(CppConstructor {
                        params: params.clone(),
                        kind: *ctor_kind,
                        access: *access,
                        member_initializers,
                        mir_body,
                    });
                }
                ClangNodeKind::DestructorDecl {
                    class_name: _,
                    is_definition,
                    access,
                } => {
                    let mir_body = if *is_definition {
                        Some(self.convert_destructor_body(child)?)
                    } else {
                        None
                    };
                    destructor = Some(CppDestructor {
                        access: *access,
                        mir_body,
                    });
                }
                ClangNodeKind::FriendDecl { friend_class, friend_function } => {
                    if let Some(class_name) = friend_class {
                        friends.push(CppFriend::Class { name: class_name.clone() });
                    } else if let Some(func_name) = friend_function {
                        friends.push(CppFriend::Function { name: func_name.clone() });
                    }
                }
                ClangNodeKind::CXXBaseSpecifier { base_type, access, is_virtual } => {
                    bases.push(CppBaseClass {
                        base_type: base_type.clone(),
                        access: *access,
                        is_virtual: *is_virtual,
                    });
                }
                _ => {}
            }
        }

        Ok(CppStruct {
            name: name.to_string(),
            is_class,
            namespace: namespace_context.to_vec(),
            bases,
            fields,
            static_fields,
            constructors,
            destructor,
            methods,
            friends,
        })
    }

    /// Convert a constructor body to MIR.
    fn convert_constructor_body(
        &self,
        node: &ClangNode,
        params: &[(String, CppType)],
    ) -> Result<MirBody> {
        let mut builder = FunctionBuilder::new();

        // Local 0 is the return place (void for constructors)
        builder.add_local(None, CppType::Void, false);

        // Add parameters as locals
        for (param_name, param_ty) in params {
            builder.add_local(Some(param_name.clone()), param_ty.clone(), true);
        }

        // Find the compound statement (constructor body)
        for child in &node.children {
            if matches!(child.kind, ClangNodeKind::CompoundStmt) {
                self.convert_compound_stmt(child, &mut builder)?;
                break;
            }
        }

        // If no explicit return, add one
        if builder.current_statements.is_empty() && builder.blocks.is_empty() {
            builder.finish_block(MirTerminator::Return);
        }

        Ok(builder.build())
    }

    /// Convert a destructor body to MIR.
    fn convert_destructor_body(&self, node: &ClangNode) -> Result<MirBody> {
        let mut builder = FunctionBuilder::new();

        // Local 0 is the return place (void for destructors)
        builder.add_local(None, CppType::Void, false);

        // Find the compound statement (destructor body)
        for child in &node.children {
            if matches!(child.kind, ClangNodeKind::CompoundStmt) {
                self.convert_compound_stmt(child, &mut builder)?;
                break;
            }
        }

        // If no explicit return, add one
        if builder.current_statements.is_empty() && builder.blocks.is_empty() {
            builder.finish_block(MirTerminator::Return);
        }

        Ok(builder.build())
    }

    /// Extract member initializers from a constructor node.
    ///
    /// In libclang, member initializers appear as MemberRef children of the constructor,
    /// each followed by an expression (the initializer value).
    fn extract_member_initializers(&self, ctor_node: &ClangNode) -> Vec<MemberInitializer> {
        let mut initializers = Vec::new();

        // Member initializers appear as MemberRef nodes in the constructor's children
        for child in &ctor_node.children {
            if let ClangNodeKind::MemberRef { name } = &child.kind {
                initializers.push(MemberInitializer {
                    member_name: name.clone(),
                    has_init: true, // If we see a MemberRef, it's explicitly initialized
                });
            }
        }

        initializers
    }

    /// Extract function name from a CallExpr's first child, unwrapping casts.
    ///
    /// Clang often wraps function references in ImplicitCastExpr or UnexposedExpr,
    /// so we need to recursively unwrap to find the actual DeclRefExpr containing the name.
    fn extract_function_name(node: &ClangNode) -> String {
        match &node.kind {
            ClangNodeKind::DeclRefExpr { name, .. } => name.clone(),
            ClangNodeKind::ImplicitCastExpr { .. }
            | ClangNodeKind::CastExpr { .. }
            | ClangNodeKind::Unknown(_) => {
                // Unwrap cast/unknown node and recurse
                if let Some(inner) = node.children.first() {
                    Self::extract_function_name(inner)
                } else {
                    "unknown".to_string()
                }
            }
            _ => "unknown".to_string(),
        }
    }
}

impl Default for MirConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a node kind is an expression.
fn is_expression_kind(kind: &ClangNodeKind) -> bool {
    matches!(
        kind,
        ClangNodeKind::IntegerLiteral(_)
            | ClangNodeKind::FloatingLiteral(_)
            | ClangNodeKind::BoolLiteral(_)
            | ClangNodeKind::StringLiteral(_)
            | ClangNodeKind::DeclRefExpr { .. }
            | ClangNodeKind::BinaryOperator { .. }
            | ClangNodeKind::UnaryOperator { .. }
            | ClangNodeKind::CallExpr { .. }
            | ClangNodeKind::MemberExpr { .. }
            | ClangNodeKind::ArraySubscriptExpr { .. }
            | ClangNodeKind::CastExpr { .. }
            | ClangNodeKind::ConditionalOperator { .. }
            | ClangNodeKind::ParenExpr { .. }
            | ClangNodeKind::ImplicitCastExpr { .. }
    )
}

/// Convert Clang binary operator to MIR binary operator.
fn convert_binop(op: BinaryOp) -> MirBinOp {
    match op {
        BinaryOp::Add | BinaryOp::AddAssign => MirBinOp::Add,
        BinaryOp::Sub | BinaryOp::SubAssign => MirBinOp::Sub,
        BinaryOp::Mul | BinaryOp::MulAssign => MirBinOp::Mul,
        BinaryOp::Div | BinaryOp::DivAssign => MirBinOp::Div,
        BinaryOp::Rem | BinaryOp::RemAssign => MirBinOp::Rem,
        BinaryOp::And | BinaryOp::AndAssign => MirBinOp::BitAnd,
        BinaryOp::Or | BinaryOp::OrAssign => MirBinOp::BitOr,
        BinaryOp::Xor | BinaryOp::XorAssign => MirBinOp::BitXor,
        BinaryOp::Shl | BinaryOp::ShlAssign => MirBinOp::Shl,
        BinaryOp::Shr | BinaryOp::ShrAssign => MirBinOp::Shr,
        BinaryOp::Eq => MirBinOp::Eq,
        BinaryOp::Ne => MirBinOp::Ne,
        BinaryOp::Lt => MirBinOp::Lt,
        BinaryOp::Le => MirBinOp::Le,
        BinaryOp::Gt => MirBinOp::Gt,
        BinaryOp::Ge => MirBinOp::Ge,
        BinaryOp::LAnd => MirBinOp::BitAnd, // Logical and treated as bitwise for now
        BinaryOp::LOr => MirBinOp::BitOr,   // Logical or treated as bitwise for now
        BinaryOp::Assign => MirBinOp::Add,  // Should be handled specially
        BinaryOp::Comma => MirBinOp::Add,   // Comma returns right operand
    }
}

/// Convert Clang unary operator to MIR unary operator.
fn convert_unaryop(op: UnaryOp) -> MirUnaryOp {
    match op {
        UnaryOp::Minus => MirUnaryOp::Neg,
        UnaryOp::Plus => MirUnaryOp::Neg, // +x is identity, but we don't have that
        UnaryOp::Not | UnaryOp::LNot => MirUnaryOp::Not,
        _ => MirUnaryOp::Neg, // Default for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::ClangParser;

    #[test]
    fn test_convert_simple_function() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int add(int a, int b) {
                    return a + b;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].display_name, "add");
    }
}
