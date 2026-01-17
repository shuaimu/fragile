//! Clang AST to MIR conversion.

use crate::ast::{BinaryOp, ClangAst, ClangNode, ClangNodeKind, UnaryOp};
use crate::types::CppType;
use crate::{
    CppBaseClass, CppClassTemplate, CppClassTemplatePartialSpec, CppConceptDecl, CppConstructor,
    CppDestructor, CppExtern, CppField, CppFriend, CppFunction, CppFunctionTemplate,
    CppMemberTemplate, CppMethod, CppModule, CppStruct, MemberInitializer, MirBasicBlock,
    MirBinOp, MirBody, MirConstant, MirLocal, MirOperand, MirPlace, MirProjection, MirRvalue,
    MirStatement, MirTerminator, MirUnaryOp, UsingDeclaration, UsingDirective,
};
use miette::Result;

/// Converter from Clang AST to MIR.
pub struct MirConverter {
    /// Current function being converted (reserved for future use)
    #[allow(dead_code)]
    current_function: Option<FunctionBuilder>,
}

/// Loop context for tracking break/continue targets.
struct LoopContext {
    /// Block to jump to on `continue` (loop header)
    continue_target: usize,
    /// Block to jump to on `break` (loop exit)
    break_target: usize,
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
    /// Stack of loop contexts for nested loops
    loop_stack: Vec<LoopContext>,
    /// Next reserved block index (for forward references in control flow)
    next_reserved_block: usize,
}

impl FunctionBuilder {
    fn new() -> Self {
        Self {
            locals: Vec::new(),
            blocks: Vec::new(),
            current_block: 0,
            current_statements: Vec::new(),
            var_map: rustc_hash::FxHashMap::default(),
            loop_stack: Vec::new(),
            next_reserved_block: 1, // Start at 1 (0 is the entry block)
        }
    }

    /// Push a loop context onto the stack.
    fn push_loop(&mut self, continue_target: usize, break_target: usize) {
        self.loop_stack.push(LoopContext {
            continue_target,
            break_target,
        });
    }

    /// Pop the current loop context from the stack.
    fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// Get the current loop context (if inside a loop).
    fn current_loop(&self) -> Option<&LoopContext> {
        self.loop_stack.last()
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
        self.finish_block_with_cleanup(terminator, false)
    }

    /// Finish the current block with a terminator, optionally marking as cleanup.
    fn finish_block_with_cleanup(&mut self, terminator: MirTerminator, is_cleanup: bool) -> usize {
        let block = MirBasicBlock {
            statements: std::mem::take(&mut self.current_statements),
            terminator,
            is_cleanup,
        };
        let idx = self.blocks.len();
        self.blocks.push(block);
        self.current_block = idx + 1;
        idx
    }

    /// Reserve a new block index. Each call reserves a unique index.
    /// Blocks must be finished in the order they were reserved.
    fn reserve_block(&mut self) -> usize {
        let idx = self.next_reserved_block;
        self.next_reserved_block += 1;
        idx
    }

    /// Create a new block and return its index.
    /// DEPRECATED: Use reserve_block() for control flow that needs forward references.
    fn new_block(&mut self) -> usize {
        self.blocks.len() + 1 // Next block index
    }

    /// Build the final MIR body.
    fn build(self) -> MirBody {
        MirBody {
            blocks: self.blocks,
            locals: self.locals,
            is_coroutine: false,
        }
    }

    /// Check if the current block needs a terminator.
    /// Returns false if the block was just started (empty) which means the previous
    /// statement already provided a terminator (e.g., return, break, continue).
    fn needs_terminator(&self) -> bool {
        // If we have statements in the current block, we need a terminator
        // If the block is empty AND we have previous blocks, then the previous
        // statement already terminated (e.g., return statement called finish_block)
        !self.current_statements.is_empty() || self.blocks.is_empty()
    }

    /// Build the final MIR body, marking it as a coroutine.
    /// Reserved for future coroutine MIR generation.
    #[allow(dead_code)]
    fn build_coroutine(self) -> MirBody {
        MirBody {
            blocks: self.blocks,
            locals: self.locals,
            is_coroutine: true,
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
            ClangNodeKind::LinkageSpecDecl => {
                // Linkage specification (extern "C" { ... })
                // Recursively convert contents without changing namespace context
                for child in &node.children {
                    self.convert_decl(child, module, namespace_context)?;
                }
            }
            ClangNodeKind::FunctionDecl {
                name,
                mangled_name,
                return_type,
                params,
                is_definition,
                is_noexcept,
            } => {
                if *is_definition {
                    let func = self.convert_function(node, name, mangled_name, return_type, params, *is_noexcept, namespace_context)?;
                    module.functions.push(func);
                } else {
                    // Just a declaration, add as extern
                    module.externs.push(CppExtern {
                        mangled_name: mangled_name.clone(),
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
                parameter_pack_indices,
                requires_clause,
                is_noexcept,
            } => {
                module.function_templates.push(CppFunctionTemplate {
                    name: name.clone(),
                    namespace: namespace_context.to_vec(),
                    template_params: template_params.clone(),
                    return_type: return_type.clone(),
                    params: params.clone(),
                    is_definition: *is_definition,
                    is_noexcept: *is_noexcept,
                    specializations: Vec::new(),
                    parameter_pack_indices: parameter_pack_indices.clone(),
                    requires_clause: requires_clause.clone(),
                });
            }
            ClangNodeKind::TemplateTypeParmDecl { .. } => {
                // Template type parameters are handled as part of FunctionTemplateDecl
                // No separate processing needed
            }
            ClangNodeKind::ClassTemplateDecl {
                name,
                template_params,
                is_class,
                parameter_pack_indices,
                requires_clause,
            } => {
                let class_template = self.convert_class_template(
                    node,
                    name,
                    template_params,
                    *is_class,
                    parameter_pack_indices,
                    requires_clause,
                    namespace_context,
                )?;
                module.class_templates.push(class_template);
            }
            ClangNodeKind::ConceptDecl {
                name,
                template_params,
                constraint_expr,
            } => {
                module.concepts.push(CppConceptDecl {
                    name: name.clone(),
                    namespace: namespace_context.to_vec(),
                    template_params: template_params.clone(),
                    constraint_expr: constraint_expr.clone(),
                });
            }
            ClangNodeKind::ClassTemplatePartialSpecDecl {
                name,
                template_params,
                specialization_args,
                is_class,
                parameter_pack_indices,
            } => {
                let partial_spec = self.convert_class_template_partial_spec(
                    node,
                    name,
                    template_params,
                    specialization_args,
                    *is_class,
                    parameter_pack_indices,
                    namespace_context,
                )?;
                module.class_partial_specializations.push(partial_spec);
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
            ClangNodeKind::TypeAliasDecl { name, underlying_type } => {
                module.type_aliases.push(crate::CppTypeAlias {
                    name: name.clone(),
                    namespace: namespace_context.to_vec(),
                    underlying_type: underlying_type.clone(),
                    is_template: false,
                    template_params: Vec::new(),
                });
            }
            ClangNodeKind::TypedefDecl { name, underlying_type } => {
                module.type_aliases.push(crate::CppTypeAlias {
                    name: name.clone(),
                    namespace: namespace_context.to_vec(),
                    underlying_type: underlying_type.clone(),
                    is_template: false,
                    template_params: Vec::new(),
                });
            }
            ClangNodeKind::TypeAliasTemplateDecl { name, template_params, underlying_type } => {
                module.type_aliases.push(crate::CppTypeAlias {
                    name: name.clone(),
                    namespace: namespace_context.to_vec(),
                    underlying_type: underlying_type.clone(),
                    is_template: true,
                    template_params: template_params.clone(),
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
        mangled_name: &str,
        return_type: &CppType,
        params: &[(String, CppType)],
        is_noexcept: bool,
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
            mangled_name: mangled_name.to_string(),
            display_name: name.to_string(),
            namespace: namespace_context.to_vec(),
            params: params.to_vec(),
            return_type: return_type.clone(),
            is_noexcept,
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

                    // Reserve blocks for then, else (if present), and merge
                    // Use reserve_block() to get unique indices even before blocks are created
                    let then_block = builder.reserve_block();
                    let else_block = if else_branch.is_some() {
                        builder.reserve_block()
                    } else {
                        // No else branch - false case goes directly to merge
                        builder.reserve_block() // This will be the merge block
                    };
                    let merge_block = if else_branch.is_some() {
                        builder.reserve_block()
                    } else {
                        else_block // When no else, the "else_block" is actually the merge
                    };

                    // Finish current block with switch
                    builder.finish_block(MirTerminator::SwitchInt {
                        operand: cond_operand,
                        targets: vec![(1, then_block)], // if true, goto then
                        otherwise: else_block,
                    });

                    // Convert then branch
                    self.convert_stmt(then_branch, builder)?;
                    // Only add Goto if the branch needs a terminator
                    // (i.e., it didn't already terminate with return/break/continue)
                    if builder.needs_terminator() {
                        builder.finish_block(MirTerminator::Goto {
                            target: merge_block,
                        });
                    }

                    // Convert else branch if present
                    if let Some(else_stmt) = else_branch {
                        self.convert_stmt(else_stmt, builder)?;
                        if builder.needs_terminator() {
                            builder.finish_block(MirTerminator::Goto {
                                target: merge_block,
                            });
                        }
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

                    // Push loop context for break/continue
                    builder.push_loop(loop_header, loop_exit);

                    // Loop body
                    self.convert_stmt(body, builder)?;
                    builder.finish_block(MirTerminator::Goto {
                        target: loop_header,
                    });

                    // Pop loop context
                    builder.pop_loop();
                }
            }

            ClangNodeKind::ForStmt => {
                // ForStmt children:
                // [0] Init statement (DeclStmt or expression, may be empty)
                // [1] Condition variable (optional, usually NULL/empty)
                // [2] Condition expression (optional)
                // [3] Increment expression (optional)
                // [4] Body statement
                //
                // for (init; cond; incr) body
                //
                // MIR structure:
                //   init
                //   goto loop_header
                // loop_header:
                //   if cond goto loop_body else goto loop_exit
                // loop_body:
                //   body
                //   incr
                //   goto loop_header
                // loop_exit:
                //   ...

                // Get the children, handling potential missing children
                let init = node.children.get(0);
                // Child 1 is condition variable declaration (rarely used)
                let cond = node.children.get(2);
                let incr = node.children.get(3);
                let body = node.children.get(4);

                // Execute init statement (if any)
                if let Some(init_node) = init {
                    // Only convert if it's not an empty/placeholder node
                    if !matches!(init_node.kind, ClangNodeKind::Unknown(_)) {
                        self.convert_stmt(init_node, builder)?;
                    }
                }

                let loop_header = builder.new_block();
                let loop_body = loop_header + 1;
                let loop_exit = loop_body + 1;

                // Jump to loop header
                builder.finish_block(MirTerminator::Goto {
                    target: loop_header,
                });

                // Loop header: evaluate condition
                if let Some(cond_node) = cond {
                    if !matches!(cond_node.kind, ClangNodeKind::Unknown(_)) {
                        let cond_operand = self.convert_expr(cond_node, builder)?;
                        builder.finish_block(MirTerminator::SwitchInt {
                            operand: cond_operand,
                            targets: vec![(1, loop_body)],
                            otherwise: loop_exit,
                        });
                    } else {
                        // No condition = infinite loop (always enter body)
                        builder.finish_block(MirTerminator::Goto {
                            target: loop_body,
                        });
                    }
                } else {
                    // No condition = infinite loop (always enter body)
                    builder.finish_block(MirTerminator::Goto {
                        target: loop_body,
                    });
                }

                // Push loop context for break/continue
                builder.push_loop(loop_header, loop_exit);

                // Loop body
                if let Some(body_node) = body {
                    self.convert_stmt(body_node, builder)?;
                }

                // Increment expression (evaluated for side effects)
                if let Some(incr_node) = incr {
                    if !matches!(incr_node.kind, ClangNodeKind::Unknown(_)) {
                        let _ = self.convert_expr(incr_node, builder)?;
                    }
                }

                // Jump back to header
                builder.finish_block(MirTerminator::Goto {
                    target: loop_header,
                });

                // Pop loop context
                builder.pop_loop();
            }

            ClangNodeKind::BreakStmt => {
                // Break jumps to loop exit
                if let Some(loop_ctx) = builder.current_loop() {
                    let break_target = loop_ctx.break_target;
                    builder.finish_block(MirTerminator::Goto {
                        target: break_target,
                    });
                } else {
                    // Break outside of loop - should be an error, but emit unreachable for now
                    builder.finish_block(MirTerminator::Unreachable);
                }
            }

            ClangNodeKind::ContinueStmt => {
                // Continue jumps to loop header
                if let Some(loop_ctx) = builder.current_loop() {
                    let continue_target = loop_ctx.continue_target;
                    builder.finish_block(MirTerminator::Goto {
                        target: continue_target,
                    });
                } else {
                    // Continue outside of loop - should be an error, but emit unreachable for now
                    builder.finish_block(MirTerminator::Unreachable);
                }
            }

            // C++20 Coroutine statements
            ClangNodeKind::CoreturnStmt { value_ty: _ } => {
                // co_return statement - terminates the coroutine
                let value = if let Some(expr) = node.children.first() {
                    Some(self.convert_expr(expr, builder)?)
                } else {
                    None
                };
                builder.finish_block(MirTerminator::CoroutineReturn { value });
            }

            ClangNodeKind::SwitchStmt => {
                // SwitchStmt children:
                // [0] Condition expression (what we switch on)
                // [1] Switch body (CompoundStmt containing CaseStmt and DefaultStmt)
                //
                // MIR structure:
                // - Evaluate condition
                // - SwitchInt terminator with targets for each case value
                // - Each case label jumps to its block
                // - Default case is the "otherwise" target
                //
                // Note: This is a simplified implementation that doesn't handle fallthrough.
                // Proper fallthrough would require tracking whether each case ends with break.

                if node.children.len() >= 2 {
                    let cond_node = &node.children[0];
                    let body_node = &node.children[1];

                    // Evaluate the switch condition
                    let cond_operand = self.convert_expr(cond_node, builder)?;

                    // Collect case values and their indices
                    let mut case_values: Vec<(i128, usize)> = Vec::new();
                    let mut default_idx: Option<usize> = None;

                    for (idx, child) in body_node.children.iter().enumerate() {
                        match &child.kind {
                            ClangNodeKind::CaseStmt { value } => {
                                case_values.push((*value, idx));
                            }
                            ClangNodeKind::DefaultStmt => {
                                default_idx = Some(idx);
                            }
                            _ => {}
                        }
                    }

                    // Create blocks: one for each case/default + exit block
                    let num_cases = body_node.children.len();
                    let first_case_block = builder.new_block() as usize;
                    let case_blocks: Vec<usize> = (0..num_cases)
                        .map(|i| first_case_block + i)
                        .collect();
                    let exit_block = first_case_block + num_cases;

                    // Build SwitchInt targets
                    let targets: Vec<(i128, usize)> = case_values
                        .iter()
                        .map(|(val, idx)| (*val, case_blocks[*idx]))
                        .collect();

                    // Default target: either the default case block or exit
                    let otherwise = default_idx
                        .map(|idx| case_blocks[idx])
                        .unwrap_or(exit_block);

                    // Emit the switch terminator
                    builder.finish_block(MirTerminator::SwitchInt {
                        operand: cond_operand,
                        targets,
                        otherwise,
                    });

                    // Convert each case body
                    for (_idx, child) in body_node.children.iter().enumerate() {
                        match &child.kind {
                            ClangNodeKind::CaseStmt { .. } | ClangNodeKind::DefaultStmt => {
                                // Case/default body is in children
                                for case_child in &child.children {
                                    // Skip the constant expr, convert the actual body
                                    if !matches!(case_child.kind, ClangNodeKind::IntegerLiteral { .. }) {
                                        self.convert_stmt(case_child, builder)?;
                                    }
                                }
                                // Jump to exit (break - simplified, no fallthrough)
                                builder.finish_block(MirTerminator::Goto {
                                    target: exit_block,
                                });
                            }
                            _ => {
                                // Other statements in switch body (shouldn't normally happen)
                                self.convert_stmt(child, builder)?;
                            }
                        }
                    }
                }
            }

            ClangNodeKind::CaseStmt { .. } | ClangNodeKind::DefaultStmt => {
                // These are handled as part of SwitchStmt processing
                // If encountered standalone, convert children
                for child in &node.children {
                    self.convert_stmt(child, builder)?;
                }
            }

            // C++ Exception Handling
            ClangNodeKind::TryStmt => {
                // Try statement: first child is compound stmt (try block),
                // remaining children are catch handlers
                // For now, just convert the try block; catch handlers are placeholders
                for child in &node.children {
                    self.convert_stmt(child, builder)?;
                }
            }

            ClangNodeKind::CatchStmt { exception_ty: _ } => {
                // Catch handler: first child is exception decl, second is body
                // For now, just convert children
                for child in &node.children {
                    self.convert_stmt(child, builder)?;
                }
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
            ClangNodeKind::IntegerLiteral { value, cpp_type } => {
                // Extract bit width and signedness from the C++ type
                let (bits, signed) = match cpp_type {
                    Some(ty) => {
                        let bits = ty.bit_width().unwrap_or(32);
                        let signed = ty.is_signed().unwrap_or(true);
                        (bits, signed)
                    }
                    None => (32, true), // Default to i32
                };
                Ok(MirOperand::Constant(MirConstant::Int {
                    value: *value,
                    bits,
                    signed,
                }))
            }

            ClangNodeKind::FloatingLiteral { value, cpp_type } => {
                // Extract bit width from the C++ type (32 for float, 64 for double)
                let bits = match cpp_type {
                    Some(ty) => ty.bit_width().unwrap_or(64),
                    None => 64, // Default to f64
                };
                Ok(MirOperand::Constant(MirConstant::Float {
                    value: *value,
                    bits,
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
                    // Malformed - return zero (default to signed i32)
                    Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32, signed: true }))
                }
            }

            ClangNodeKind::UnaryOperator { op, ty } => {
                if let Some(operand_node) = node.children.first() {
                    match op {
                        UnaryOp::AddrOf => {
                            // Address-of: convert operand to a place, then take reference
                            let operand = self.convert_expr(operand_node, builder)?;
                            let place = match operand {
                                MirOperand::Copy(place) | MirOperand::Move(place) => place,
                                MirOperand::Constant(c) => {
                                    // Can't take address of constant - store in temp first
                                    let operand_ty = Self::get_node_type(operand_node);
                                    let temp_local = builder.add_local(None, operand_ty, false);
                                    builder.add_statement(MirStatement::Assign {
                                        target: MirPlace::local(temp_local),
                                        value: MirRvalue::Use(MirOperand::Constant(c)),
                                    });
                                    MirPlace::local(temp_local)
                                }
                            };
                            let result_local = builder.add_local(None, ty.clone(), false);
                            builder.add_statement(MirStatement::Assign {
                                target: MirPlace::local(result_local),
                                value: MirRvalue::Ref { place, mutability: true },
                            });
                            Ok(MirOperand::Copy(MirPlace::local(result_local)))
                        }
                        UnaryOp::Deref => {
                            // Dereference: convert operand to place, add Deref projection
                            let operand = self.convert_expr(operand_node, builder)?;
                            match operand {
                                MirOperand::Copy(mut place) | MirOperand::Move(mut place) => {
                                    place.projection.push(MirProjection::Deref);
                                    Ok(MirOperand::Copy(place))
                                }
                                MirOperand::Constant(c) => {
                                    // Dereferencing a constant pointer - store in temp first
                                    let operand_ty = Self::get_node_type(operand_node);
                                    let temp_local = builder.add_local(None, operand_ty, false);
                                    builder.add_statement(MirStatement::Assign {
                                        target: MirPlace::local(temp_local),
                                        value: MirRvalue::Use(MirOperand::Constant(c)),
                                    });
                                    let mut place = MirPlace::local(temp_local);
                                    place.projection.push(MirProjection::Deref);
                                    Ok(MirOperand::Copy(place))
                                }
                            }
                        }
                        _ => {
                            // Other unary ops: use existing path
                            let operand = self.convert_expr(operand_node, builder)?;
                            let result_local = builder.add_local(None, ty.clone(), false);
                            let mir_op = convert_unaryop(*op);
                            builder.add_statement(MirStatement::Assign {
                                target: MirPlace::local(result_local),
                                value: MirRvalue::UnaryOp { op: mir_op, operand },
                            });
                            Ok(MirOperand::Copy(MirPlace::local(result_local)))
                        }
                    }
                } else {
                    Ok(MirOperand::Constant(MirConstant::Int { value: 0, bits: 32, signed: true }))
                }
            }

            ClangNodeKind::CallExpr { ty } => {
                // First child is the function reference (may be wrapped in ImplicitCastExpr),
                // rest are arguments
                if let Some(func_ref) = node.children.first() {
                    let func_name = Self::extract_function_name(func_ref);

                    // Handle std::move as a builtin - it's just a cast to rvalue reference
                    if Self::is_std_move(&func_name) {
                        if let Some(arg) = node.children.get(1) {
                            let operand = self.convert_expr(arg, builder)?;
                            // Convert Copy operand to Move operand
                            return Ok(match operand {
                                MirOperand::Copy(place) => MirOperand::Move(place),
                                other => other,
                            });
                        }
                        // No argument - return unit
                        return Ok(MirOperand::Constant(MirConstant::Unit));
                    }

                    // Handle std::forward as a builtin - conditionally moves or copies
                    if Self::is_std_forward(&func_name) {
                        if let Some(arg) = node.children.get(1) {
                            let operand = self.convert_expr(arg, builder)?;
                            // For now, treat forward like move since we don't track reference collapsing
                            // A more complete implementation would check the template argument
                            return Ok(match operand {
                                MirOperand::Copy(place) => MirOperand::Move(place),
                                other => other,
                            });
                        }
                        return Ok(MirOperand::Constant(MirConstant::Unit));
                    }

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
                        unwind: None, // TODO: Generate cleanup blocks for stack unwinding
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
                // Skip non-expression children (like TypeRef) and find the actual value
                if let Some(inner) = node.children.iter().find(|c| is_expression_kind(&c.kind)) {
                    self.convert_expr(inner, builder)
                } else if let Some(inner) = node.children.first() {
                    // Fallback: try first child
                    self.convert_expr(inner, builder)
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            // C++20 Coroutine expressions
            ClangNodeKind::CoawaitExpr { operand_ty: _, result_ty } => {
                // co_await expression - suspends until the awaitable is ready
                let awaitable = if let Some(expr) = node.children.first() {
                    self.convert_expr(expr, builder)?
                } else {
                    MirOperand::Constant(MirConstant::Unit)
                };

                // Create a temporary for the result
                let result_local = builder.add_local(None, result_ty.clone(), false);
                let destination = MirPlace::local(result_local);

                // Create await terminator
                let resume_block = builder.new_block();
                builder.finish_block(MirTerminator::Await {
                    awaitable,
                    destination: destination.clone(),
                    resume: resume_block,
                    drop: None,
                });

                // Return the result place as an operand
                Ok(MirOperand::Copy(destination))
            }

            ClangNodeKind::CoyieldExpr { value_ty: _, result_ty: _ } => {
                // co_yield expression - yields a value and suspends
                let value = if let Some(expr) = node.children.first() {
                    self.convert_expr(expr, builder)?
                } else {
                    MirOperand::Constant(MirConstant::Unit)
                };

                // Create yield terminator
                let resume_block = builder.new_block();
                builder.finish_block(MirTerminator::Yield {
                    value,
                    resume: resume_block,
                    drop: None,
                });

                // The yield expression returns a value (typically void for generators)
                Ok(MirOperand::Constant(MirConstant::Unit))
            }

            // C++ Exception Handling
            ClangNodeKind::ThrowExpr { exception_ty: _ } => {
                // throw expression - evaluate the thrown value (if any)
                // For now, just evaluate children for side effects
                // Proper exception handling would need runtime support
                if let Some(expr) = node.children.first() {
                    let _ = self.convert_expr(expr, builder)?;
                }
                // throw is like an unreachable after the expression is evaluated
                builder.finish_block(MirTerminator::Unreachable);
                Ok(MirOperand::Constant(MirConstant::Unit))
            }

            // C++ RTTI
            ClangNodeKind::TypeidExpr { result_ty: _ } => {
                // typeid expression - evaluate child and return type_info reference
                // For now, just evaluate children for side effects
                // Full RTTI requires runtime support
                if let Some(expr) = node.children.first() {
                    let _ = self.convert_expr(expr, builder)?;
                }
                Ok(MirOperand::Constant(MirConstant::Unit))
            }

            ClangNodeKind::DynamicCastExpr { target_ty: _ } => {
                // dynamic_cast - evaluate child and cast
                // For now, just convert the child expression
                // Full RTTI requires runtime support
                if let Some(expr) = node.children.first() {
                    self.convert_expr(expr, builder)
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            ClangNodeKind::MemberExpr {
                member_name,
                is_arrow,
                ty: _,
            } => {
                // Member access expression: obj.field or ptr->field
                // First child is the base expression
                if let Some(base_node) = node.children.first() {
                    // Convert base to a place
                    let base_operand = self.convert_expr(base_node, builder)?;
                    let mut base_place = match base_operand {
                        MirOperand::Copy(place) | MirOperand::Move(place) => place,
                        MirOperand::Constant(c) => {
                            // Base is a constant - store in temp first
                            let base_ty = Self::get_node_type(base_node);
                            let temp_local = builder.add_local(None, base_ty, false);
                            builder.add_statement(MirStatement::Assign {
                                target: MirPlace::local(temp_local),
                                value: MirRvalue::Use(MirOperand::Constant(c)),
                            });
                            MirPlace::local(temp_local)
                        }
                    };

                    // For arrow access (->), we need to dereference first
                    if *is_arrow {
                        base_place.projection.push(MirProjection::Deref);
                    }

                    // Add field access projection
                    // We use field index 0 as placeholder; the name allows later resolution
                    base_place.projection.push(MirProjection::Field {
                        index: 0, // TODO: Resolve actual field index from struct definition
                        name: Some(member_name.clone()),
                    });

                    Ok(MirOperand::Copy(base_place))
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            ClangNodeKind::ArraySubscriptExpr { ty: _ } => {
                // Array subscript expression: arr[index]
                // First child is the array, second is the index
                if node.children.len() >= 2 {
                    let array_node = &node.children[0];
                    let index_node = &node.children[1];

                    // Convert array to a place
                    let array_operand = self.convert_expr(array_node, builder)?;
                    let array_place = match array_operand {
                        MirOperand::Copy(place) | MirOperand::Move(place) => place,
                        MirOperand::Constant(c) => {
                            // Array is a constant - store in temp first
                            let array_ty = Self::get_node_type(array_node);
                            let temp_local = builder.add_local(None, array_ty, false);
                            builder.add_statement(MirStatement::Assign {
                                target: MirPlace::local(temp_local),
                                value: MirRvalue::Use(MirOperand::Constant(c)),
                            });
                            MirPlace::local(temp_local)
                        }
                    };

                    // Convert index
                    let index_operand = self.convert_expr(index_node, builder)?;

                    // For MirProjection::Index, we need a compile-time known index
                    // Runtime indices would require a different approach (using variable indexing)
                    let index_value = match index_operand {
                        MirOperand::Constant(MirConstant::Int { value, .. }) => value as usize,
                        _ => {
                            // Runtime index - for now, use index 0 as fallback
                            // TODO: Support runtime indexing with a local variable
                            0
                        }
                    };

                    // Create indexed place
                    let mut indexed_place = array_place;
                    indexed_place.projection.push(MirProjection::Index(index_value));

                    Ok(MirOperand::Copy(indexed_place))
                } else {
                    Ok(MirOperand::Constant(MirConstant::Int {
                        value: 0,
                        bits: 32,
                        signed: true,
                    }))
                }
            }

            ClangNodeKind::InitListExpr { ty } => {
                // Initialization list expression: {1, 2, 3}
                // Children are the individual initializer expressions
                let mut fields = Vec::new();

                for child in &node.children {
                    let operand = self.convert_expr(child, builder)?;
                    // For now, we don't have field names from the init list
                    // Field names can be resolved later if we have struct type info
                    fields.push((None, operand));
                }

                // Create a temporary to hold the aggregate value
                let result_local = builder.add_local(None, ty.clone(), false);
                let destination = MirPlace::local(result_local);

                // Assign the aggregate to the temporary
                builder.add_statement(MirStatement::Assign {
                    target: destination.clone(),
                    value: MirRvalue::Aggregate {
                        ty: ty.clone(),
                        fields,
                    },
                });

                Ok(MirOperand::Copy(destination))
            }

            ClangNodeKind::Unknown(_) => {
                // Unknown/UnexposedExpr nodes often wrap other expressions
                // Unwrap and recurse into the child
                if let Some(inner) = node.children.first() {
                    self.convert_expr(inner, builder)
                } else {
                    Ok(MirOperand::Constant(MirConstant::Unit))
                }
            }

            _ => {
                // Truly unknown expression - return unit
                Ok(MirOperand::Constant(MirConstant::Unit))
            }
        }
    }

    /// Convert a class template definition.
    fn convert_class_template(
        &self,
        node: &ClangNode,
        name: &str,
        template_params: &[String],
        is_class: bool,
        parameter_pack_indices: &[usize],
        requires_clause: &Option<String>,
        namespace_context: &[String],
    ) -> Result<CppClassTemplate> {
        let mut fields = Vec::new();
        let mut static_fields = Vec::new();
        let mut constructors = Vec::new();
        let mut destructor = None;
        let mut methods = Vec::new();
        let mut member_templates = Vec::new();

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
                ClangNodeKind::FunctionTemplateDecl {
                    name: method_name,
                    template_params: method_template_params,
                    return_type,
                    params,
                    is_definition,
                    parameter_pack_indices: method_pack_indices,
                    requires_clause: _,  // Member templates don't track requires clause yet
                    is_noexcept: _,
                } => {
                    // Member template inside a class template
                    member_templates.push(CppMemberTemplate {
                        name: method_name.clone(),
                        template_params: method_template_params.clone(),
                        return_type: return_type.clone(),
                        params: params.clone(),
                        access: self.get_access_from_children(child),
                        is_static: false,
                        parameter_pack_indices: method_pack_indices.clone(),
                        is_definition: *is_definition,
                    });
                }
                ClangNodeKind::ConstructorDecl {
                    class_name: _,
                    params,
                    is_definition,
                    ctor_kind,
                    access,
                } => {
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
                _ => {}
            }
        }

        Ok(CppClassTemplate {
            name: name.to_string(),
            is_class,
            namespace: namespace_context.to_vec(),
            template_params: template_params.to_vec(),
            fields,
            static_fields,
            constructors,
            destructor,
            methods,
            member_templates,
            parameter_pack_indices: parameter_pack_indices.to_vec(),
            requires_clause: requires_clause.clone(),
        })
    }

    /// Convert a class template partial specialization definition.
    fn convert_class_template_partial_spec(
        &self,
        node: &ClangNode,
        name: &str,
        template_params: &[String],
        specialization_args: &[CppType],
        is_class: bool,
        parameter_pack_indices: &[usize],
        namespace_context: &[String],
    ) -> Result<CppClassTemplatePartialSpec> {
        let mut fields = Vec::new();
        let mut static_fields = Vec::new();
        let mut constructors = Vec::new();
        let mut destructor = None;
        let mut methods = Vec::new();
        let mut member_templates = Vec::new();

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
                ClangNodeKind::FunctionTemplateDecl {
                    name: method_name,
                    template_params: method_template_params,
                    return_type,
                    params,
                    is_definition,
                    parameter_pack_indices: method_pack_indices,
                    requires_clause: _,  // Member templates don't track requires clause yet
                    is_noexcept: _,
                } => {
                    member_templates.push(CppMemberTemplate {
                        name: method_name.clone(),
                        template_params: method_template_params.clone(),
                        return_type: return_type.clone(),
                        params: params.clone(),
                        access: self.get_access_from_children(child),
                        is_static: false,
                        parameter_pack_indices: method_pack_indices.clone(),
                        is_definition: *is_definition,
                    });
                }
                ClangNodeKind::ConstructorDecl {
                    class_name: _,
                    params,
                    is_definition,
                    ctor_kind,
                    access,
                } => {
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
                _ => {}
            }
        }

        Ok(CppClassTemplatePartialSpec {
            template_name: name.to_string(),
            is_class,
            namespace: namespace_context.to_vec(),
            template_params: template_params.to_vec(),
            specialization_args: specialization_args.to_vec(),
            fields,
            static_fields,
            constructors,
            destructor,
            methods,
            member_templates,
            parameter_pack_indices: parameter_pack_indices.to_vec(),
        })
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
        let mut member_templates = Vec::new();
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
                ClangNodeKind::FunctionTemplateDecl {
                    name: method_name,
                    template_params,
                    return_type,
                    params,
                    is_definition,
                    parameter_pack_indices,
                    requires_clause: _,  // Member templates don't track requires clause yet
                    is_noexcept: _,
                } => {
                    // Member template inside a class
                    member_templates.push(CppMemberTemplate {
                        name: method_name.clone(),
                        template_params: template_params.clone(),
                        return_type: return_type.clone(),
                        params: params.clone(),
                        access: self.get_access_from_children(child),
                        is_static: false, // TODO: Detect static member templates
                        parameter_pack_indices: parameter_pack_indices.clone(),
                        is_definition: *is_definition,
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
            member_templates,
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

    /// Get the C++ type from a ClangNode.
    ///
    /// Extracts the type information from the node's kind. Used for operations
    /// like address-of where we need to know the operand's type.
    fn get_node_type(node: &ClangNode) -> CppType {
        match &node.kind {
            ClangNodeKind::IntegerLiteral { cpp_type, .. } => {
                cpp_type.clone().unwrap_or(CppType::Int { signed: true })
            }
            ClangNodeKind::FloatingLiteral { cpp_type, .. } => {
                cpp_type.clone().unwrap_or(CppType::Double)
            }
            ClangNodeKind::BoolLiteral(_) => CppType::Bool,
            ClangNodeKind::StringLiteral(_) => {
                CppType::Pointer {
                    pointee: Box::new(CppType::Char { signed: true }),
                    is_const: true,
                }
            }
            ClangNodeKind::UnaryOperator { ty, .. } => ty.clone(),
            ClangNodeKind::BinaryOperator { ty, .. } => ty.clone(),
            ClangNodeKind::CallExpr { ty } => ty.clone(),
            ClangNodeKind::DeclRefExpr { ty, .. } => ty.clone(),
            ClangNodeKind::MemberExpr { ty, .. } => ty.clone(),
            ClangNodeKind::ArraySubscriptExpr { ty } => ty.clone(),
            ClangNodeKind::CastExpr { ty, .. } => ty.clone(),
            ClangNodeKind::ImplicitCastExpr { ty, .. } => ty.clone(),
            ClangNodeKind::ConditionalOperator { ty } => ty.clone(),
            ClangNodeKind::InitListExpr { ty } => ty.clone(),
            ClangNodeKind::VarDecl { ty, .. } => ty.clone(),
            ClangNodeKind::ParmVarDecl { ty, .. } => ty.clone(),
            _ => CppType::Int { signed: true }, // Default fallback
        }
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

    /// Get access specifier from a function template's children.
    /// Function templates don't have access specifier in their AST node,
    /// so we need to infer it from the context (defaults to private for classes).
    fn get_access_from_children(&self, _node: &ClangNode) -> crate::ast::AccessSpecifier {
        // TODO: Parse access specifier from the parent class context
        // For now, default to public since most member templates are public
        crate::ast::AccessSpecifier::Public
    }

    /// Check if a function name refers to std::move.
    ///
    /// std::move is just a cast to rvalue reference, not a real function call.
    /// We treat it as a builtin for efficiency.
    fn is_std_move(name: &str) -> bool {
        name == "std::move" || name == "move"
    }

    /// Check if a function name refers to std::forward.
    ///
    /// std::forward conditionally casts to rvalue reference based on the
    /// template argument. We treat it as a builtin for efficiency.
    fn is_std_forward(name: &str) -> bool {
        name == "std::forward" || name == "forward"
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
        ClangNodeKind::IntegerLiteral { .. }
            | ClangNodeKind::FloatingLiteral { .. }
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
            | ClangNodeKind::InitListExpr { .. }
            // C++20 Coroutine expressions
            | ClangNodeKind::CoawaitExpr { .. }
            | ClangNodeKind::CoyieldExpr { .. }
            // C++ Exception expressions
            | ClangNodeKind::ThrowExpr { .. }
            // C++ RTTI expressions
            | ClangNodeKind::TypeidExpr { .. }
            | ClangNodeKind::DynamicCastExpr { .. }
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
///
/// Note: AddrOf and Deref are handled specially in convert_expr and should never
/// reach this function.
fn convert_unaryop(op: UnaryOp) -> MirUnaryOp {
    match op {
        UnaryOp::Minus => MirUnaryOp::Neg,
        UnaryOp::Plus => MirUnaryOp::Neg, // +x is identity, MIR doesn't have identity op
        UnaryOp::Not => MirUnaryOp::Not,  // Bitwise not (~x)
        UnaryOp::LNot => MirUnaryOp::Not, // Logical not (!x) - treated as bitwise for now
        UnaryOp::PreInc | UnaryOp::PostInc => MirUnaryOp::Neg, // TODO: Inc/Dec need special handling
        UnaryOp::PreDec | UnaryOp::PostDec => MirUnaryOp::Neg, // TODO: Inc/Dec need special handling
        // Address-of and dereference are handled specially in convert_expr
        UnaryOp::AddrOf => unreachable!("AddrOf should be handled in convert_expr"),
        UnaryOp::Deref => unreachable!("Deref should be handled in convert_expr"),
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

    #[test]
    fn test_convert_address_of() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int* get_addr() {
                    int x = 42;
                    int* ptr = &x;
                    return ptr;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.display_name, "get_addr");

        // Verify MirRvalue::Ref is used for address-of
        let body = &func.mir_body;
        let has_ref = body
            .blocks
            .iter()
            .flat_map(|bb| &bb.statements)
            .any(|stmt| matches!(stmt, MirStatement::Assign { value: MirRvalue::Ref { .. }, .. }));
        assert!(has_ref, "Should have MirRvalue::Ref for address-of operation");
    }

    #[test]
    fn test_convert_dereference() {
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int deref_ptr(int* ptr) {
                    return *ptr;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.display_name, "deref_ptr");

        // Verify MirProjection::Deref is used for dereference
        let body = &func.mir_body;
        let has_deref = body
            .blocks
            .iter()
            .flat_map(|bb| &bb.statements)
            .any(|stmt| {
                if let MirStatement::Assign { value, .. } = stmt {
                    match value {
                        MirRvalue::Use(MirOperand::Copy(place)) | MirRvalue::Use(MirOperand::Move(place)) => {
                            place.projection.iter().any(|p| matches!(p, MirProjection::Deref))
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            });
        assert!(has_deref, "Should have MirProjection::Deref for dereference operation");
    }

    #[test]
    fn test_convert_pointer_ops_combined() {
        // Test that combined pointer operations compile without panic
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int ptr_ops(int* ptr) {
                    int x = *ptr;       // dereference
                    int* addr = &x;     // address-of
                    return *addr;       // dereference again
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.display_name, "ptr_ops");

        // Just verify the function has MIR blocks - individual operations are tested separately
        assert!(!func.mir_body.blocks.is_empty(), "Function should have basic blocks");
    }

    #[test]
    fn test_convert_array_subscript() {
        // Test that array subscript expressions are converted correctly
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int get_element(int arr[10]) {
                    return arr[0];  // Constant index
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        let body = &func.mir_body;

        // Verify MirProjection::Index is used
        let has_index = body
            .blocks
            .iter()
            .flat_map(|bb| &bb.statements)
            .any(|stmt| {
                if let MirStatement::Assign {
                    value: MirRvalue::Use(MirOperand::Copy(place)),
                    ..
                } = stmt
                {
                    place
                        .projection
                        .iter()
                        .any(|p| matches!(p, MirProjection::Index(_)))
                } else {
                    false
                }
            });
        assert!(
            has_index,
            "Should have MirProjection::Index for array subscript"
        );
    }

    #[test]
    fn test_convert_array_subscript_variable_index() {
        // Test array subscript with a variable index (falls back to 0 for now)
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int get_element(int arr[10], int i) {
                    return arr[i];  // Variable index
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }

    #[test]
    fn test_convert_array_subscript_nested() {
        // Test nested array access (multidimensional array)
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int get_element(int arr[3][4]) {
                    return arr[1][2];  // Nested indices
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }

    #[test]
    fn test_convert_member_expr_dot() {
        // Test field access with dot operator: s.field
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Point {
                    int x;
                    int y;
                };

                int get_x(Point p) {
                    return p.x;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        // Should have the struct and the function
        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.functions.len(), 1);

        let func = &module.functions[0];
        let body = &func.mir_body;

        // Verify MirProjection::Field is used
        let has_field = body
            .blocks
            .iter()
            .flat_map(|bb| &bb.statements)
            .any(|stmt| {
                if let MirStatement::Assign {
                    value: MirRvalue::Use(MirOperand::Copy(place)),
                    ..
                } = stmt
                {
                    place.projection.iter().any(|p| {
                        matches!(p, MirProjection::Field { name: Some(n), .. } if n == "x")
                    })
                } else {
                    false
                }
            });
        assert!(has_field, "Should have MirProjection::Field for member access");
    }

    #[test]
    fn test_convert_member_expr_arrow() {
        // Test field access with arrow operator: ptr->field
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Point {
                    int x;
                    int y;
                };

                int get_x_ptr(Point* p) {
                    return p->x;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        let body = &func.mir_body;

        // Verify both Deref and Field projections are used for arrow access
        let has_deref_and_field = body
            .blocks
            .iter()
            .flat_map(|bb| &bb.statements)
            .any(|stmt| {
                if let MirStatement::Assign {
                    value: MirRvalue::Use(MirOperand::Copy(place)),
                    ..
                } = stmt
                {
                    let has_deref = place.projection.iter().any(|p| matches!(p, MirProjection::Deref));
                    let has_field = place
                        .projection
                        .iter()
                        .any(|p| matches!(p, MirProjection::Field { name: Some(n), .. } if n == "x"));
                    has_deref && has_field
                } else {
                    false
                }
            });
        assert!(
            has_deref_and_field,
            "Should have both Deref and Field for arrow access"
        );
    }

    #[test]
    fn test_convert_nested_member_expr() {
        // Test nested field access: outer.inner.value
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Inner {
                    int value;
                };

                struct Outer {
                    Inner inner;
                };

                int get_nested_value(Outer o) {
                    return o.inner.value;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }

    #[test]
    fn test_convert_init_list_struct() {
        // Test struct aggregate initialization with braced init list
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Point {
                    int x;
                    int y;
                };

                Point create_point() {
                    return Point{1, 2};
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.functions.len(), 1);

        let func = &module.functions[0];
        let body = &func.mir_body;

        // Verify MirRvalue::Aggregate is used
        let has_aggregate = body
            .blocks
            .iter()
            .flat_map(|bb| &bb.statements)
            .any(|stmt| {
                if let MirStatement::Assign {
                    value: MirRvalue::Aggregate { .. },
                    ..
                } = stmt
                {
                    true
                } else {
                    false
                }
            });
        assert!(
            has_aggregate,
            "Should have MirRvalue::Aggregate for init list"
        );
    }

    #[test]
    fn test_convert_init_list_variable() {
        // Test struct aggregate initialization assigned to variable
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Point {
                    int x;
                    int y;
                };

                int get_x() {
                    Point p{10, 20};
                    return p.x;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }

    #[test]
    fn test_convert_init_list_array() {
        // Test array initialization with init list
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                int get_second() {
                    int arr[3] = {1, 2, 3};
                    return arr[1];
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }

    #[test]
    fn test_nested_struct_definition() {
        // Test parsing struct with nested struct field
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Inner {
                    int x;
                    int y;
                };

                struct Outer {
                    Inner inner;
                    int z;
                };

                int get_inner_x(Outer o) {
                    return o.inner.x;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        // Should have both Inner and Outer structs
        assert_eq!(module.structs.len(), 2);
        assert_eq!(module.functions.len(), 1);

        // Verify struct names
        let struct_names: Vec<_> = module.structs.iter().map(|s| s.name.as_str()).collect();
        assert!(struct_names.contains(&"Inner"), "Should have Inner struct");
        assert!(struct_names.contains(&"Outer"), "Should have Outer struct");
    }

    #[test]
    fn test_nested_aggregate_initialization() {
        // Test nested aggregate initialization with braced init list
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Inner {
                    int x;
                    int y;
                };

                struct Outer {
                    Inner inner;
                    int z;
                };

                Outer create_outer() {
                    return Outer{{1, 2}, 3};
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }

    #[test]
    fn test_nested_struct_assignment() {
        // Test assigning nested struct values
        let parser = ClangParser::new().unwrap();
        let ast = parser
            .parse_string(
                r#"
                struct Inner {
                    int value;
                };

                struct Outer {
                    Inner inner;
                };

                int nested_assign() {
                    Outer o;
                    o.inner.value = 42;
                    return o.inner.value;
                }
                "#,
                "test.cpp",
            )
            .unwrap();

        let converter = MirConverter::new();
        let module = converter.convert(ast).unwrap();

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Just verify the function compiles without panicking
        assert!(
            !func.mir_body.blocks.is_empty(),
            "Function should have basic blocks"
        );
    }
}
