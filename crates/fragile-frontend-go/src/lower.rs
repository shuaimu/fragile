use fragile_common::{SourceFile, Span, Symbol, SymbolInterner};
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
        // Get package name
        let name = node
            .children(&mut node.walk())
            .find(|n| n.kind() == "package_clause")
            .and_then(|pkg| pkg.child_by_field_name("name"))
            .map(|n| self.intern(self.text(n)))
            .unwrap_or_else(|| self.intern("main"));

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
        let span = self.span(node);

        match node.kind() {
            "function_declaration" => {
                let fn_def = self.lower_function(node)?;
                Ok(Some(Item::new(ItemKind::Function(fn_def), span)))
            }
            // TODO: type_declaration (struct, interface), const_declaration, var_declaration
            _ => Ok(None),
        }
    }

    fn lower_function(&self, node: Node) -> Result<FnDef> {
        let span = self.span(node);

        // Get name
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| miette::miette!("Function missing name"))?;
        let name = self.intern(self.text(name_node));

        // Visibility based on capitalization (Go convention)
        let vis = if self.text(name_node).chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Get parameters
        let params = if let Some(params_node) = node.child_by_field_name("parameters") {
            self.lower_parameters(params_node)?
        } else {
            vec![]
        };

        // Get return type
        let ret_ty = if let Some(result_node) = node.child_by_field_name("result") {
            self.lower_type(result_node)?
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
            source_lang: SourceLang::Go,
        })
    }

    fn lower_parameters(&self, node: Node) -> Result<Vec<Param>> {
        let mut params = vec![];
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                let span = self.span(child);

                // Get type (comes after names in Go)
                let ty = child
                    .child_by_field_name("type")
                    .map(|n| self.lower_type(n))
                    .transpose()?
                    .unwrap_or(Type::Infer(0));

                // Get names (can be multiple: a, b int)
                let mut name_cursor = child.walk();
                for name_child in child.children(&mut name_cursor) {
                    if name_child.kind() == "identifier" {
                        let name = self.intern(self.text(name_child));
                        params.push(Param {
                            name,
                            ty: ty.clone(),
                            mutability: Mutability::Mutable, // Go params are mutable
                            span,
                        });
                    }
                }
            }
        }

        Ok(params)
    }

    fn lower_type(&self, node: Node) -> Result<Type> {
        match node.kind() {
            "type_identifier" | "identifier" => {
                let text = self.text(node);
                Ok(self.primitive_type(text))
            }
            "pointer_type" => {
                let inner = node
                    .child(1) // Skip *
                    .map(|n| self.lower_type(n))
                    .transpose()?
                    .unwrap_or(Type::Error);
                Ok(Type::Pointer {
                    inner: Box::new(inner),
                    mutability: Mutability::Mutable,
                })
            }
            "slice_type" => {
                let inner = node
                    .child_by_field_name("element")
                    .or_else(|| node.child(2)) // Skip []
                    .map(|n| self.lower_type(n))
                    .transpose()?
                    .unwrap_or(Type::Error);
                Ok(Type::Slice {
                    inner: Box::new(inner),
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
            "qualified_type" => {
                // package.Type
                let name = self.intern(self.text(node));
                Ok(Type::Named {
                    name,
                    type_args: vec![],
                })
            }
            _ => {
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
            "int" => PrimitiveType::Isize,  // Go int is platform-sized
            "int8" => PrimitiveType::I8,
            "int16" => PrimitiveType::I16,
            "int32" | "rune" => PrimitiveType::I32,
            "int64" => PrimitiveType::I64,
            "uint" => PrimitiveType::Usize,
            "uint8" | "byte" => PrimitiveType::U8,
            "uint16" => PrimitiveType::U16,
            "uint32" => PrimitiveType::U32,
            "uint64" => PrimitiveType::U64,
            "uintptr" => PrimitiveType::Usize,
            "float32" => PrimitiveType::F32,
            "float64" => PrimitiveType::F64,
            "bool" => PrimitiveType::Bool,
            "string" => {
                // String is a built-in type in Go, represent as named for now
                return Type::Named {
                    name: self.intern("string"),
                    type_args: vec![],
                };
            }
            _ => {
                return Type::Named {
                    name: self.intern(text),
                    type_args: vec![],
                };
            }
        };
        Type::Primitive(prim)
    }

    fn lower_block(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);
        let mut stmts = vec![];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" | "}" => continue,
                "short_var_declaration" => {
                    stmts.push(self.lower_short_var_decl(child)?);
                }
                "var_declaration" => {
                    if let Some(stmt) = self.lower_var_decl(child)? {
                        stmts.push(stmt);
                    }
                }
                "assignment_statement" => {
                    let expr = self.lower_assignment(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "return_statement" => {
                    let expr = self.lower_return_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "if_statement" => {
                    let expr = self.lower_if_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "for_statement" => {
                    let expr = self.lower_for_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "expression_statement" => {
                    if let Some(expr_node) = child.child(0) {
                        let expr = self.lower_expr(expr_node)?;
                        stmts.push(Stmt::expr(expr));
                    }
                }
                "block" => {
                    let expr = self.lower_block(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                _ => {
                    if let Ok(expr) = self.lower_expr(child) {
                        stmts.push(Stmt::expr(expr));
                    }
                }
            }
        }

        Ok(Expr::new(
            ExprKind::Block { stmts, expr: None },
            span,
        ))
    }

    fn lower_short_var_decl(&self, node: Node) -> Result<Stmt> {
        let span = self.span(node);

        // Get left side (names)
        let left = node.child_by_field_name("left");
        let name = left
            .and_then(|n| n.child(0))
            .map(|n| self.intern(self.text(n)))
            .unwrap_or_else(|| self.intern("_"));

        // Get right side (value)
        let init = node
            .child_by_field_name("right")
            .and_then(|n| n.child(0))
            .map(|n| self.lower_expr(n))
            .transpose()?;

        Ok(Stmt::new(
            StmtKind::Let {
                pattern: Pattern::Ident(name),
                ty: None,
                init,
                mutability: Mutability::Mutable,
            },
            span,
        ))
    }

    fn lower_var_decl(&self, node: Node) -> Result<Option<Stmt>> {
        let span = self.span(node);

        // Find var_spec inside
        let var_spec = node
            .children(&mut node.walk())
            .find(|n| n.kind() == "var_spec");

        if let Some(spec) = var_spec {
            let name = spec
                .child_by_field_name("name")
                .map(|n| self.intern(self.text(n)))
                .unwrap_or_else(|| self.intern("_"));

            let ty = spec
                .child_by_field_name("type")
                .map(|n| self.lower_type(n))
                .transpose()?;

            let init = spec
                .child_by_field_name("value")
                .map(|n| self.lower_expr(n))
                .transpose()?;

            return Ok(Some(Stmt::new(
                StmtKind::Let {
                    pattern: Pattern::Ident(name),
                    ty,
                    init,
                    mutability: Mutability::Mutable,
                },
                span,
            )));
        }

        Ok(None)
    }

    fn lower_assignment(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let lhs = node
            .child_by_field_name("left")
            .and_then(|n| n.child(0))
            .map(|n| self.lower_expr(n))
            .transpose()?
            .ok_or_else(|| miette::miette!("Assignment missing lhs"))?;

        let rhs = node
            .child_by_field_name("right")
            .and_then(|n| n.child(0))
            .map(|n| self.lower_expr(n))
            .transpose()?
            .ok_or_else(|| miette::miette!("Assignment missing rhs"))?;

        Ok(Expr::new(
            ExprKind::Assign {
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        ))
    }

    fn lower_return_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let value = node
            .child_by_field_name("result")
            .or_else(|| node.children(&mut node.walk()).find(|n| n.kind() == "expression_list"))
            .and_then(|n| n.child(0))
            .map(|n| self.lower_expr(n))
            .transpose()?;

        Ok(Expr::new(ExprKind::Return(value.map(Box::new)), span))
    }

    fn lower_if_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let cond = node
            .child_by_field_name("condition")
            .map(|n| self.lower_expr(n))
            .transpose()?
            .ok_or_else(|| miette::miette!("If missing condition"))?;

        let then_branch = node
            .child_by_field_name("consequence")
            .map(|n| self.lower_block(n))
            .transpose()?
            .ok_or_else(|| miette::miette!("If missing consequence"))?;

        let else_branch = node
            .child_by_field_name("alternative")
            .map(|n| {
                if n.kind() == "block" {
                    self.lower_block(n)
                } else if n.kind() == "if_statement" {
                    self.lower_if_statement(n)
                } else {
                    self.lower_expr(n)
                }
            })
            .transpose()?;

        Ok(Expr::new(
            ExprKind::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
            },
            span,
        ))
    }

    fn lower_for_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        // Go for can be: for {}, for cond {}, for init; cond; post {}, for range {}
        let body = node
            .child_by_field_name("body")
            .map(|n| self.lower_block(n))
            .transpose()?
            .unwrap_or_else(|| Expr::new(ExprKind::Literal(Literal::Unit), span));

        // Check for condition
        if let Some(cond_node) = node.child_by_field_name("condition") {
            let cond = self.lower_expr(cond_node)?;
            return Ok(Expr::new(
                ExprKind::While {
                    cond: Box::new(cond),
                    body: Box::new(body),
                },
                span,
            ));
        }

        // Infinite loop
        Ok(Expr::new(ExprKind::Loop { body: Box::new(body) }, span))
    }

    fn lower_expr(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let kind = match node.kind() {
            "int_literal" => {
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

            "true" => ExprKind::Literal(Literal::Bool(true)),
            "false" => ExprKind::Literal(Literal::Bool(false)),

            "interpreted_string_literal" | "raw_string_literal" => {
                let text = self.text(node);
                let content = if text.len() >= 2 {
                    &text[1..text.len() - 1]
                } else {
                    text
                };
                ExprKind::Literal(Literal::String(content.to_string()))
            }

            "rune_literal" => {
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
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Unary expr missing operand"))?;

                let op_text = node
                    .child_by_field_name("operator")
                    .map(|n| self.text(n))
                    .unwrap_or("");

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

            "selector_expression" => {
                let expr = node
                    .child_by_field_name("operand")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Selector missing operand"))?;
                let field = node
                    .child_by_field_name("field")
                    .map(|n| self.intern(self.text(n)))
                    .ok_or_else(|| miette::miette!("Selector missing field"))?;

                ExprKind::Field {
                    expr: Box::new(expr),
                    field,
                }
            }

            "index_expression" => {
                let expr = node
                    .child_by_field_name("operand")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Index missing operand"))?;
                let index = node
                    .child_by_field_name("index")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Index missing index"))?;

                ExprKind::Index {
                    expr: Box::new(expr),
                    index: Box::new(index),
                }
            }

            "parenthesized_expression" => {
                if let Some(inner) = node.child(1) {
                    return self.lower_expr(inner);
                }
                ExprKind::Error
            }

            _ => ExprKind::Error,
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
