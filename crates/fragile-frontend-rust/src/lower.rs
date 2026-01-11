use fragile_common::{SourceFile, SourceId, Span, Symbol, SymbolInterner};
use fragile_hir::{
    BinOp, Expr, ExprKind, FnDef, FnSig, Item, ItemKind, Literal, Module,
    Mutability, Param, Pattern, PrimitiveType, SourceLang, Stmt, StmtKind,
    Type, Visibility,
};
use miette::Result;
use tree_sitter::{Node, Tree};

/// Lower a tree-sitter Tree to HIR Module.
pub fn lower(tree: Tree, source: &SourceFile, interner: &SymbolInterner) -> Result<Module> {
    let ctx = LoweringContext::new(source, interner);
    ctx.lower_module(tree.root_node())
}

struct LoweringContext<'a> {
    source: &'a SourceFile,
    interner: &'a SymbolInterner,
}

impl<'a> LoweringContext<'a> {
    fn new(source: &'a SourceFile, interner: &'a SymbolInterner) -> Self {
        Self { source, interner }
    }

    fn span(&self, node: Node) -> Span {
        Span::new(
            self.source.id,
            node.start_byte() as u32,
            node.end_byte() as u32,
        )
    }

    fn text(&self, node: Node) -> &str {
        node.utf8_text(self.source.content.as_bytes()).unwrap_or("")
    }

    fn intern(&self, s: &str) -> Symbol {
        self.interner.intern(s)
    }

    fn lower_module(&self, node: Node) -> Result<Module> {
        let name = self.intern("main"); // TODO: derive from file path
        let mut module = Module::new(name, self.source.id);

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(item) = self.lower_item(child)? {
                module.add_item(item);
            }
        }

        Ok(module)
    }

    fn lower_item(&self, node: Node) -> Result<Option<Item>> {
        let kind = node.kind();
        let span = self.span(node);

        match kind {
            "function_item" => {
                let fn_def = self.lower_function(node)?;
                Ok(Some(Item::new(ItemKind::Function(fn_def), span)))
            }
            // TODO: struct_item, enum_item, impl_item, etc.
            _ => Ok(None), // Skip unknown nodes
        }
    }

    fn lower_function(&self, node: Node) -> Result<FnDef> {
        let span = self.span(node);

        // Get visibility
        let vis = if node.child_by_field_name("visibility").is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Get name
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| miette::miette!("Function missing name"))?;
        let name = self.intern(self.text(name_node));

        // Get parameters
        let params = if let Some(params_node) = node.child_by_field_name("parameters") {
            self.lower_parameters(params_node)?
        } else {
            vec![]
        };

        // Get return type
        let ret_ty = if let Some(ret_node) = node.child_by_field_name("return_type") {
            self.lower_type(ret_node)?
        } else {
            Type::unit()
        };

        // Get body
        let body = if let Some(body_node) = node.child_by_field_name("body") {
            Some(self.lower_block(body_node)?)
        } else {
            None
        };

        Ok(FnDef {
            name,
            vis,
            type_params: vec![],
            sig: FnSig {
                params,
                ret_ty,
                is_variadic: false,
            },
            body,
            span,
            source_lang: SourceLang::Rust,
        })
    }

    fn lower_parameters(&self, node: Node) -> Result<Vec<Param>> {
        let mut params = vec![];
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "parameter" {
                let param = self.lower_parameter(child)?;
                params.push(param);
            }
        }

        Ok(params)
    }

    fn lower_parameter(&self, node: Node) -> Result<Param> {
        let span = self.span(node);

        // Check for mutability
        let mutability = if node.child_by_field_name("mutable").is_some() {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };

        // Get pattern (name)
        let pattern_node = node
            .child_by_field_name("pattern")
            .ok_or_else(|| miette::miette!("Parameter missing pattern"))?;
        let name = self.intern(self.text(pattern_node));

        // Get type
        let ty = if let Some(ty_node) = node.child_by_field_name("type") {
            self.lower_type(ty_node)?
        } else {
            Type::Infer(0) // Type inference needed
        };

        Ok(Param {
            name,
            ty,
            mutability,
            span,
        })
    }

    fn lower_type(&self, node: Node) -> Result<Type> {
        match node.kind() {
            "primitive_type" | "type_identifier" => {
                let text = self.text(node);
                Ok(self.primitive_type(text))
            }
            "reference_type" => {
                let mutable = node.child_by_field_name("mutable").is_some();
                let inner = node
                    .child_by_field_name("type")
                    .map(|n| self.lower_type(n))
                    .transpose()?
                    .unwrap_or(Type::Error);
                Ok(Type::Reference {
                    inner: Box::new(inner),
                    mutability: if mutable {
                        Mutability::Mutable
                    } else {
                        Mutability::Immutable
                    },
                })
            }
            "pointer_type" => {
                let mutable = node.child_by_field_name("mutable").is_some();
                let inner = node
                    .child_by_field_name("type")
                    .map(|n| self.lower_type(n))
                    .transpose()?
                    .unwrap_or(Type::Error);
                Ok(Type::Pointer {
                    inner: Box::new(inner),
                    mutability: if mutable {
                        Mutability::Mutable
                    } else {
                        Mutability::Immutable
                    },
                })
            }
            "array_type" => {
                let inner = node
                    .child_by_field_name("element")
                    .map(|n| self.lower_type(n))
                    .transpose()?
                    .unwrap_or(Type::Error);
                let size = node
                    .child_by_field_name("length")
                    .and_then(|n| self.text(n).parse().ok())
                    .unwrap_or(0);
                Ok(Type::Array {
                    inner: Box::new(inner),
                    size,
                })
            }
            "unit_type" | "()" => Ok(Type::unit()),
            _ => {
                // Try as named type
                let name = self.intern(self.text(node));
                Ok(Type::Named {
                    name,
                    type_args: vec![],
                })
            }
        }
    }

    fn primitive_type(&self, text: &str) -> Type {
        let prim = match text {
            "i8" => PrimitiveType::I8,
            "i16" => PrimitiveType::I16,
            "i32" => PrimitiveType::I32,
            "i64" => PrimitiveType::I64,
            "i128" => PrimitiveType::I128,
            "isize" => PrimitiveType::Isize,
            "u8" => PrimitiveType::U8,
            "u16" => PrimitiveType::U16,
            "u32" => PrimitiveType::U32,
            "u64" => PrimitiveType::U64,
            "u128" => PrimitiveType::U128,
            "usize" => PrimitiveType::Usize,
            "f32" => PrimitiveType::F32,
            "f64" => PrimitiveType::F64,
            "bool" => PrimitiveType::Bool,
            "char" => PrimitiveType::Char,
            "()" => PrimitiveType::Unit,
            "!" => PrimitiveType::Never,
            _ => {
                return Type::Named {
                    name: self.intern(text),
                    type_args: vec![],
                }
            }
        };
        Type::Primitive(prim)
    }

    fn lower_block(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);
        let mut stmts = vec![];
        let mut final_expr = None;

        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;

            match child.kind() {
                "{" | "}" => continue,
                "let_declaration" => {
                    stmts.push(self.lower_let_stmt(*child)?);
                }
                "expression_statement" => {
                    if let Some(expr_node) = child.child(0) {
                        let expr = self.lower_expr(expr_node)?;
                        stmts.push(Stmt::expr(expr));
                    }
                }
                _ => {
                    // Try as expression
                    if is_last && !child.kind().ends_with("_statement") {
                        final_expr = Some(Box::new(self.lower_expr(*child)?));
                    } else {
                        let expr = self.lower_expr(*child)?;
                        stmts.push(Stmt::expr(expr));
                    }
                }
            }
        }

        Ok(Expr::new(
            ExprKind::Block {
                stmts,
                expr: final_expr,
            },
            span,
        ))
    }

    fn lower_let_stmt(&self, node: Node) -> Result<Stmt> {
        let span = self.span(node);

        let mutability = if node.child_by_field_name("mutable").is_some() {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        };

        let pattern = if let Some(pat_node) = node.child_by_field_name("pattern") {
            self.lower_pattern(pat_node)?
        } else {
            Pattern::Wildcard
        };

        let ty = node
            .child_by_field_name("type")
            .map(|n| self.lower_type(n))
            .transpose()?;

        let init = node
            .child_by_field_name("value")
            .map(|n| self.lower_expr(n))
            .transpose()?;

        Ok(Stmt::new(
            StmtKind::Let {
                pattern,
                ty,
                init,
                mutability,
            },
            span,
        ))
    }

    fn lower_pattern(&self, node: Node) -> Result<Pattern> {
        match node.kind() {
            "identifier" => {
                let name = self.intern(self.text(node));
                Ok(Pattern::Ident(name))
            }
            "_" => Ok(Pattern::Wildcard),
            "tuple_pattern" => {
                let mut patterns = vec![];
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                        patterns.push(self.lower_pattern(child)?);
                    }
                }
                Ok(Pattern::Tuple(patterns))
            }
            _ => {
                // Default to identifier
                let name = self.intern(self.text(node));
                Ok(Pattern::Ident(name))
            }
        }
    }

    fn lower_expr(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let kind = match node.kind() {
            "integer_literal" => {
                let text = self.text(node).replace('_', "");
                let value: i128 = if text.starts_with("0x") || text.starts_with("0X") {
                    i128::from_str_radix(&text[2..], 16).unwrap_or(0)
                } else if text.starts_with("0o") || text.starts_with("0O") {
                    i128::from_str_radix(&text[2..], 8).unwrap_or(0)
                } else if text.starts_with("0b") || text.starts_with("0B") {
                    i128::from_str_radix(&text[2..], 2).unwrap_or(0)
                } else {
                    text.parse().unwrap_or(0)
                };
                ExprKind::Literal(Literal::Int(value))
            }

            "float_literal" => {
                let text = self.text(node).replace('_', "");
                let value: f64 = text.parse().unwrap_or(0.0);
                ExprKind::Literal(Literal::Float(value))
            }

            "boolean_literal" | "true" | "false" => {
                let value = self.text(node) == "true";
                ExprKind::Literal(Literal::Bool(value))
            }

            "string_literal" => {
                let text = self.text(node);
                // Remove quotes
                let content = if text.len() >= 2 {
                    &text[1..text.len() - 1]
                } else {
                    text
                };
                ExprKind::Literal(Literal::String(content.to_string()))
            }

            "char_literal" => {
                let text = self.text(node);
                let c = text.chars().nth(1).unwrap_or('\0');
                ExprKind::Literal(Literal::Char(c))
            }

            "identifier" => {
                let name = self.intern(self.text(node));
                ExprKind::Ident(name)
            }

            "binary_expression" => {
                let lhs = node
                    .child_by_field_name("left")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Binary expr missing lhs"))?;
                let rhs = node
                    .child_by_field_name("right")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Binary expr missing rhs"))?;
                let op_node = node
                    .child_by_field_name("operator")
                    .ok_or_else(|| miette::miette!("Binary expr missing operator"))?;
                let op = self.lower_binop(self.text(op_node))?;

                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }

            "unary_expression" => {
                let operand = node
                    .child_by_field_name("operand")
                    .or_else(|| node.child(1))
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Unary expr missing operand"))?;

                let op_text = node.child(0).map(|n| self.text(n)).unwrap_or("");
                let op = match op_text {
                    "-" => fragile_hir::UnaryOp::Neg,
                    "!" => fragile_hir::UnaryOp::Not,
                    "*" => fragile_hir::UnaryOp::Deref,
                    "&" => fragile_hir::UnaryOp::AddrOf,
                    _ => return Err(miette::miette!("Unknown unary op: {}", op_text)),
                };

                ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                }
            }

            "call_expression" => {
                let callee = node
                    .child_by_field_name("function")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Call missing function"))?;

                let mut args = vec![];
                if let Some(args_node) = node.child_by_field_name("arguments") {
                    let mut cursor = args_node.walk();
                    for child in args_node.children(&mut cursor) {
                        if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                            args.push(self.lower_expr(child)?);
                        }
                    }
                }

                ExprKind::Call {
                    callee: Box::new(callee),
                    args,
                }
            }

            "if_expression" => {
                let cond = node
                    .child_by_field_name("condition")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("If missing condition"))?;

                let then_branch = node
                    .child_by_field_name("consequence")
                    .map(|n| self.lower_block(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("If missing then branch"))?;

                let else_branch = node
                    .child_by_field_name("alternative")
                    .map(|n| {
                        if n.kind() == "else_clause" {
                            if let Some(block) = n.child_by_field_name("body") {
                                self.lower_block(block)
                            } else if let Some(if_expr) = n.child(1) {
                                self.lower_expr(if_expr)
                            } else {
                                Err(miette::miette!("Invalid else clause"))
                            }
                        } else {
                            self.lower_expr(n)
                        }
                    })
                    .transpose()?;

                ExprKind::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_branch),
                    else_branch: else_branch.map(Box::new),
                }
            }

            "return_expression" => {
                let value = node.child(1).map(|n| self.lower_expr(n)).transpose()?;
                ExprKind::Return(value.map(Box::new))
            }

            "block" => {
                return self.lower_block(node);
            }

            "parenthesized_expression" => {
                if let Some(inner) = node.child(1) {
                    return self.lower_expr(inner);
                }
                ExprKind::Error
            }

            "unit_expression" | "()" => ExprKind::Literal(Literal::Unit),

            _ => {
                // For now, treat unknown as error
                ExprKind::Error
            }
        };

        Ok(Expr::new(kind, span))
    }

    fn lower_binop(&self, op: &str) -> Result<BinOp> {
        let binop = match op {
            "+" => BinOp::Add,
            "-" => BinOp::Sub,
            "*" => BinOp::Mul,
            "/" => BinOp::Div,
            "%" => BinOp::Rem,
            "&" => BinOp::BitAnd,
            "|" => BinOp::BitOr,
            "^" => BinOp::BitXor,
            "<<" => BinOp::Shl,
            ">>" => BinOp::Shr,
            "==" => BinOp::Eq,
            "!=" => BinOp::Ne,
            "<" => BinOp::Lt,
            "<=" => BinOp::Le,
            ">" => BinOp::Gt,
            ">=" => BinOp::Ge,
            "&&" => BinOp::And,
            "||" => BinOp::Or,
            _ => return Err(miette::miette!("Unknown binary operator: {}", op)),
        };
        Ok(binop)
    }
}
