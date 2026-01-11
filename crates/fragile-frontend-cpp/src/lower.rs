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
        let name = self.intern("main");
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
            "function_definition" => {
                let fn_def = self.lower_function(node)?;
                Ok(Some(Item::new(ItemKind::Function(fn_def), span)))
            }
            // TODO: struct/class, enum, etc.
            _ => Ok(None),
        }
    }

    fn lower_function(&self, node: Node) -> Result<FnDef> {
        let span = self.span(node);

        // Get declarator (contains name and parameters)
        let declarator = node
            .child_by_field_name("declarator")
            .ok_or_else(|| miette::miette!("Function missing declarator"))?;

        // Get name
        let name = self.extract_function_name(declarator)?;

        // Get parameters
        let params = self.extract_parameters(declarator)?;

        // Get return type
        let ret_ty = if let Some(type_node) = node.child_by_field_name("type") {
            self.lower_type(type_node)?
        } else {
            Type::Primitive(PrimitiveType::I32) // C++ default
        };

        // Get body
        let body = if let Some(body_node) = node.child_by_field_name("body") {
            Some(self.lower_compound_statement(body_node)?)
        } else {
            None
        };

        Ok(FnDef {
            name,
            vis: Visibility::Public, // C++ defaults to public at file scope
            type_params: vec![],
            sig: FnSig {
                params,
                ret_ty,
                is_variadic: false,
            },
            body,
            span,
            source_lang: SourceLang::Cpp,
        })
    }

    fn extract_function_name(&self, declarator: Node) -> Result<Symbol> {
        // Navigate through declarator to find the identifier
        fn find_identifier<'a>(node: Node<'a>, text: &'a str) -> Option<&'a str> {
            if node.kind() == "identifier" {
                return Some(node.utf8_text(text.as_bytes()).unwrap_or(""));
            }
            if let Some(decl) = node.child_by_field_name("declarator") {
                return find_identifier(decl, text);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(name) = find_identifier(child, text) {
                    return Some(name);
                }
            }
            None
        }

        let name = find_identifier(declarator, &self.source.content)
            .ok_or_else(|| miette::miette!("Could not find function name"))?;
        Ok(self.intern(name))
    }

    fn extract_parameters(&self, declarator: Node) -> Result<Vec<Param>> {
        let mut params = vec![];

        // Find parameter_list in declarator
        fn find_parameter_list(node: Node) -> Option<Node> {
            if node.kind() == "parameter_list" {
                return Some(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(list) = find_parameter_list(child) {
                    return Some(list);
                }
            }
            None
        }

        if let Some(param_list) = find_parameter_list(declarator) {
            let mut cursor = param_list.walk();
            for child in param_list.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    params.push(self.lower_parameter(child)?);
                }
            }
        }

        Ok(params)
    }

    fn lower_parameter(&self, node: Node) -> Result<Param> {
        let span = self.span(node);

        // Get type
        let ty = if let Some(type_node) = node.child_by_field_name("type") {
            self.lower_type(type_node)?
        } else {
            Type::Infer(0)
        };

        // Get name from declarator
        let name = if let Some(decl) = node.child_by_field_name("declarator") {
            let text = self.text(decl);
            self.intern(text.trim_start_matches('*').trim_start_matches('&'))
        } else {
            self.intern("_")
        };

        Ok(Param {
            name,
            ty,
            mutability: Mutability::Mutable, // C++ params are mutable by default
            span,
        })
    }

    fn lower_type(&self, node: Node) -> Result<Type> {
        let text = self.text(node).trim();

        // Handle primitive types
        let prim = match text {
            "void" => return Ok(Type::unit()),
            "bool" => PrimitiveType::Bool,
            "char" => PrimitiveType::Char,
            "int" => PrimitiveType::I32,
            "short" | "short int" => PrimitiveType::I16,
            "long" | "long int" => PrimitiveType::I64,
            "long long" | "long long int" => PrimitiveType::I64,
            "unsigned" | "unsigned int" => PrimitiveType::U32,
            "unsigned short" => PrimitiveType::U16,
            "unsigned long" => PrimitiveType::U64,
            "unsigned long long" => PrimitiveType::U64,
            "float" => PrimitiveType::F32,
            "double" => PrimitiveType::F64,
            "int8_t" => PrimitiveType::I8,
            "int16_t" => PrimitiveType::I16,
            "int32_t" => PrimitiveType::I32,
            "int64_t" => PrimitiveType::I64,
            "uint8_t" => PrimitiveType::U8,
            "uint16_t" => PrimitiveType::U16,
            "uint32_t" => PrimitiveType::U32,
            "uint64_t" => PrimitiveType::U64,
            "size_t" => PrimitiveType::Usize,
            "ssize_t" | "ptrdiff_t" => PrimitiveType::Isize,
            _ => {
                // Named type
                return Ok(Type::Named {
                    name: self.intern(text),
                    type_args: vec![],
                });
            }
        };

        Ok(Type::Primitive(prim))
    }

    fn lower_compound_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);
        let mut stmts = vec![];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" | "}" => continue,
                "declaration" => {
                    if let Some(stmt) = self.lower_declaration(child)? {
                        stmts.push(stmt);
                    }
                }
                "expression_statement" => {
                    if let Some(expr_node) = child.child(0) {
                        if expr_node.kind() != ";" {
                            let expr = self.lower_expr(expr_node)?;
                            stmts.push(Stmt::expr(expr));
                        }
                    }
                }
                "return_statement" => {
                    let expr = self.lower_return_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "if_statement" => {
                    let expr = self.lower_if_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "while_statement" => {
                    let expr = self.lower_while_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "for_statement" => {
                    let expr = self.lower_for_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                "compound_statement" => {
                    let expr = self.lower_compound_statement(child)?;
                    stmts.push(Stmt::expr(expr));
                }
                _ => {
                    // Try as expression
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

    fn lower_declaration(&self, node: Node) -> Result<Option<Stmt>> {
        let span = self.span(node);

        // Get type
        let ty = if let Some(type_node) = node.child_by_field_name("type") {
            Some(self.lower_type(type_node)?)
        } else {
            None
        };

        // Get declarator (may have initializer)
        if let Some(decl) = node.child_by_field_name("declarator") {
            let (name, init) = self.lower_init_declarator(decl)?;

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

    fn lower_init_declarator(&self, node: Node) -> Result<(Symbol, Option<Expr>)> {
        match node.kind() {
            "init_declarator" => {
                let name = if let Some(decl) = node.child_by_field_name("declarator") {
                    self.intern(self.text(decl))
                } else {
                    self.intern("_")
                };
                let init = if let Some(value) = node.child_by_field_name("value") {
                    Some(self.lower_expr(value)?)
                } else {
                    None
                };
                Ok((name, init))
            }
            "identifier" => {
                Ok((self.intern(self.text(node)), None))
            }
            _ => {
                Ok((self.intern(self.text(node)), None))
            }
        }
    }

    fn lower_return_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);
        let mut cursor = node.walk();

        let value = node.children(&mut cursor)
            .find(|c| c.kind() != "return" && c.kind() != ";")
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
            .map(|n| {
                if n.kind() == "compound_statement" {
                    self.lower_compound_statement(n)
                } else {
                    self.lower_expr(n)
                }
            })
            .transpose()?
            .ok_or_else(|| miette::miette!("If missing then branch"))?;

        let else_branch = node
            .child_by_field_name("alternative")
            .map(|n| {
                if n.kind() == "compound_statement" {
                    self.lower_compound_statement(n)
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

    fn lower_while_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let cond = node
            .child_by_field_name("condition")
            .map(|n| self.lower_expr(n))
            .transpose()?
            .ok_or_else(|| miette::miette!("While missing condition"))?;

        let body = node
            .child_by_field_name("body")
            .map(|n| {
                if n.kind() == "compound_statement" {
                    self.lower_compound_statement(n)
                } else {
                    self.lower_expr(n)
                }
            })
            .transpose()?
            .ok_or_else(|| miette::miette!("While missing body"))?;

        Ok(Expr::new(
            ExprKind::While {
                cond: Box::new(cond),
                body: Box::new(body),
            },
            span,
        ))
    }

    fn lower_for_statement(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        // C++ for loops are complex, simplify to while for now
        let body = node
            .child_by_field_name("body")
            .map(|n| {
                if n.kind() == "compound_statement" {
                    self.lower_compound_statement(n)
                } else {
                    self.lower_expr(n)
                }
            })
            .transpose()?
            .unwrap_or_else(|| Expr::new(ExprKind::Literal(Literal::Unit), span));

        // TODO: Properly handle init, condition, update
        Ok(Expr::new(ExprKind::Loop { body: Box::new(body) }, span))
    }

    fn lower_expr(&self, node: Node) -> Result<Expr> {
        let span = self.span(node);

        let kind = match node.kind() {
            "number_literal" => {
                let text = self.text(node);
                if text.contains('.') || text.contains('e') || text.contains('E') {
                    let value: f64 = text.parse().unwrap_or(0.0);
                    ExprKind::Literal(Literal::Float(value))
                } else {
                    let value: i128 = if text.starts_with("0x") || text.starts_with("0X") {
                        i128::from_str_radix(&text[2..], 16).unwrap_or(0)
                    } else if text.starts_with("0") && text.len() > 1 {
                        i128::from_str_radix(&text[1..], 8).unwrap_or(0)
                    } else {
                        text.parse().unwrap_or(0)
                    };
                    ExprKind::Literal(Literal::Int(value))
                }
            }

            "true" => ExprKind::Literal(Literal::Bool(true)),
            "false" => ExprKind::Literal(Literal::Bool(false)),

            "string_literal" => {
                let text = self.text(node);
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
                    .child_by_field_name("argument")
                    .or_else(|| node.child(1))
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Unary expr missing operand"))?;

                let op_text = node
                    .child_by_field_name("operator")
                    .or_else(|| node.child(0))
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

            "assignment_expression" => {
                let lhs = node
                    .child_by_field_name("left")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Assignment missing lhs"))?;
                let rhs = node
                    .child_by_field_name("right")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Assignment missing rhs"))?;

                ExprKind::Assign {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }

            "parenthesized_expression" => {
                if let Some(inner) = node.child(1) {
                    return self.lower_expr(inner);
                }
                ExprKind::Error
            }

            "conditional_expression" => {
                let cond = node
                    .child_by_field_name("condition")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Ternary missing condition"))?;
                let then_expr = node
                    .child_by_field_name("consequence")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Ternary missing consequence"))?;
                let else_expr = node
                    .child_by_field_name("alternative")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Ternary missing alternative"))?;

                ExprKind::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_expr),
                    else_branch: Some(Box::new(else_expr)),
                }
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
