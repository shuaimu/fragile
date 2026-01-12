use fragile_common::{SourceFile, SourceId, Span, Symbol, SymbolInterner};
use fragile_hir::{
    Abi, Attribute, BinOp, EnumDef, EnumVariant, Expr, ExprKind, Field, FnDef, FnSig, ImplDef,
    Item, ItemKind, Literal, Module, Mutability, Param, Pattern, PrimitiveType, SourceLang, Stmt,
    StmtKind, StructDef, Type, Visibility,
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

        // Collect pending attributes to apply to the next item
        let mut pending_attrs: Vec<Attribute> = vec![];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "attribute_item" => {
                    // Collect attribute to apply to next item
                    if let Some(attr) = self.lower_attribute(child)? {
                        pending_attrs.push(attr);
                    }
                }
                _ => {
                    // Pass pending attributes to items
                    let attrs = std::mem::take(&mut pending_attrs);
                    let items = self.lower_items_with_attrs(child, attrs)?;
                    for item in items {
                        module.add_item(item);
                    }
                }
            }
        }

        Ok(module)
    }

    fn lower_attribute(&self, node: Node) -> Result<Option<Attribute>> {
        let span = self.span(node);

        // Find the attribute child node
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "attribute" {
                // Get the attribute name (identifier)
                if let Some(name_node) = child.child_by_field_name("path") {
                    let name = self.intern(self.text(name_node));
                    // TODO: Parse attribute arguments
                    return Ok(Some(Attribute {
                        name,
                        args: vec![],
                        span,
                    }));
                } else {
                    // Try first child as identifier
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "identifier" {
                            let name = self.intern(self.text(inner));
                            return Ok(Some(Attribute {
                                name,
                                args: vec![],
                                span,
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn lower_items_with_attrs(&self, node: Node, attrs: Vec<Attribute>) -> Result<Vec<Item>> {
        let kind = node.kind();
        let span = self.span(node);

        match kind {
            "function_item" => {
                let fn_def = self.lower_function_with_attrs(node, Abi::Rust, attrs)?;
                Ok(vec![Item::new(ItemKind::Function(fn_def), span)])
            }
            "struct_item" => {
                let struct_def = self.lower_struct(node)?;
                // TODO: Add attributes to structs
                Ok(vec![Item::new(ItemKind::Struct(struct_def), span)])
            }
            "enum_item" => {
                let enum_def = self.lower_enum(node)?;
                Ok(vec![Item::new(ItemKind::Enum(enum_def), span)])
            }
            "impl_item" => {
                let impl_def = self.lower_impl(node)?;
                Ok(vec![Item::new(ItemKind::Impl(impl_def), span)])
            }
            "foreign_mod_item" => {
                // This is extern "C" { ... } block
                self.lower_extern_block(node)
            }
            _ => Ok(vec![]), // Skip unknown nodes
        }
    }

    fn lower_function_with_attrs(&self, node: Node, abi: Abi, attributes: Vec<Attribute>) -> Result<FnDef> {
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
            abi,
            attributes,
        })
    }

    fn lower_extern_block(&self, node: Node) -> Result<Vec<Item>> {
        let mut items = vec![];

        // Determine ABI from the extern block
        // extern "C" { ... } or extern { ... } (defaults to "C")
        let abi = if let Some(abi_node) = node.child_by_field_name("abi") {
            let abi_text = self.text(abi_node);
            // Remove quotes: "C" -> C
            let abi_str = abi_text.trim_matches('"');
            match abi_str {
                "C" => Abi::C,
                "Rust" => Abi::Rust,
                other => Abi::Other(other.to_string()),
            }
        } else {
            Abi::C // Default extern is "C"
        };

        // Find the declaration list inside the extern block
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "declaration_list" => {
                    // Process items inside the declaration list
                    let mut inner_cursor = child.walk();
                    for inner_child in child.children(&mut inner_cursor) {
                        if inner_child.kind() == "function_signature_item" {
                            let fn_def = self.lower_extern_fn(inner_child, abi.clone())?;
                            let span = self.span(inner_child);
                            items.push(Item::new(ItemKind::Function(fn_def), span));
                        }
                    }
                }
                "function_signature_item" => {
                    // Direct child function signature
                    let fn_def = self.lower_extern_fn(child, abi.clone())?;
                    let span = self.span(child);
                    items.push(Item::new(ItemKind::Function(fn_def), span));
                }
                _ => {}
            }
        }

        Ok(items)
    }

    fn lower_extern_fn(&self, node: Node, abi: Abi) -> Result<FnDef> {
        let span = self.span(node);

        // Get visibility (usually not specified in extern blocks)
        let vis = if node.child_by_field_name("visibility").is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Get name
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| miette::miette!("Extern function missing name"))?;
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

        // Check for variadic
        let is_variadic = if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            let has_variadic = params_node.children(&mut cursor).any(|c| c.kind() == "variadic_parameter");
            has_variadic
        } else {
            false
        };

        Ok(FnDef {
            name,
            vis,
            type_params: vec![],
            sig: FnSig {
                params,
                ret_ty,
                is_variadic,
            },
            body: None, // Extern functions have no body
            span,
            source_lang: SourceLang::Rust,
            abi,
            attributes: vec![], // Extern functions don't have attributes yet
        })
    }

    fn lower_struct(&self, node: Node) -> Result<StructDef> {
        // Get name
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| miette::miette!("Struct missing name"))?;
        let name = self.intern(self.text(name_node));

        // Get fields from field_declaration_list
        let mut fields = vec![];
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    let field = self.lower_field(child)?;
                    fields.push(field);
                }
            }
        }

        Ok(StructDef {
            name,
            fields,
            type_params: vec![], // TODO: generics
        })
    }

    fn lower_enum(&self, node: Node) -> Result<EnumDef> {
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
            .ok_or_else(|| miette::miette!("Enum missing name"))?;
        let name = self.intern(self.text(name_node));

        // Get variants from enum_variant_list (body)
        let mut variants = vec![];
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            let mut discriminant: i128 = 0;
            for child in body_node.children(&mut cursor) {
                if child.kind() == "enum_variant" {
                    let variant = self.lower_enum_variant(child, discriminant)?;
                    // Update discriminant for next variant
                    discriminant = variant.discriminant.unwrap_or(discriminant) + 1;
                    variants.push(variant);
                }
            }
        }

        Ok(EnumDef {
            name,
            vis,
            type_params: vec![], // TODO: generics
            variants,
            span,
        })
    }

    fn lower_enum_variant(&self, node: Node, default_discriminant: i128) -> Result<EnumVariant> {
        // Get variant name
        let name = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "identifier")
            .map(|n| self.intern(self.text(n)))
            .ok_or_else(|| miette::miette!("Enum variant missing name"))?;

        // For now, we support simple unit variants (no fields)
        // TODO: Support tuple variants and struct variants

        Ok(EnumVariant {
            name,
            fields: vec![],
            discriminant: Some(default_discriminant),
        })
    }

    fn lower_impl(&self, node: Node) -> Result<ImplDef> {
        let span = self.span(node);

        // Get the type being implemented (e.g., Point in `impl Point`)
        let type_node = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "type_identifier" || c.kind() == "generic_type")
            .ok_or_else(|| miette::miette!("Impl missing type"))?;
        let self_ty = self.lower_type(type_node)?;

        // Get methods from declaration_list
        let mut items = vec![];
        if let Some(decl_list) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "declaration_list")
        {
            let mut cursor = decl_list.walk();
            for child in decl_list.children(&mut cursor) {
                if child.kind() == "function_item" {
                    let fn_def = self.lower_method(child, &self_ty)?;
                    items.push(Item::new(ItemKind::Function(fn_def), self.span(child)));
                }
            }
        }

        Ok(ImplDef {
            type_params: vec![], // TODO: generics
            trait_ref: None,     // TODO: trait impls
            self_ty,
            items,
            span,
        })
    }

    fn lower_method(&self, node: Node, self_ty: &Type) -> Result<FnDef> {
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
            .ok_or_else(|| miette::miette!("Method missing name"))?;
        let name = self.intern(self.text(name_node));

        // Get parameters (including self)
        let params = if let Some(params_node) = node.child_by_field_name("parameters") {
            self.lower_method_parameters(params_node, self_ty)?
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
            abi: Abi::Rust,
            attributes: vec![],
        })
    }

    fn lower_method_parameters(&self, node: Node, self_ty: &Type) -> Result<Vec<Param>> {
        let mut params = vec![];

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "self_parameter" => {
                    // Handle &self, &mut self, self
                    let span = self.span(child);
                    let self_name = self.intern("self");

                    // Check if it's a reference
                    let has_ref = child.children(&mut child.walk()).any(|c| c.kind() == "&");
                    let has_mut = child.children(&mut child.walk()).any(|c| c.kind() == "mutable_specifier" || self.text(c) == "mut");

                    let ty = if has_ref {
                        Type::Reference {
                            inner: Box::new(self_ty.clone()),
                            mutability: if has_mut { Mutability::Mutable } else { Mutability::Immutable },
                        }
                    } else {
                        self_ty.clone()
                    };

                    params.push(Param {
                        name: self_name,
                        ty,
                        mutability: if has_mut { Mutability::Mutable } else { Mutability::Immutable },
                        span,
                    });
                }
                "parameter" => {
                    params.push(self.lower_parameter(child)?);
                }
                _ => {}
            }
        }

        Ok(params)
    }

    fn lower_field(&self, node: Node) -> Result<Field> {
        // Check for visibility
        let is_public = node.child_by_field_name("visibility").is_some();

        // Get name
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| miette::miette!("Field missing name"))?;
        let name = self.intern(self.text(name_node));

        // Get type
        let ty = if let Some(ty_node) = node.child_by_field_name("type") {
            self.lower_type(ty_node)?
        } else {
            Type::Error
        };

        Ok(Field { name, ty, is_public })
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
        // Filter out braces when collecting children
        let children: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() != "{" && c.kind() != "}")
            .collect();

        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;

            match child.kind() {
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

            "self" => {
                // `self` in method body
                let name = self.intern("self");
                ExprKind::Ident(name)
            }

            "scoped_identifier" => {
                // Handle enum variant like Color::Red
                let mut cursor = node.walk();
                let children: Vec<_> = node.children(&mut cursor).collect();

                // First identifier is the enum name, last identifier is the variant
                let identifiers: Vec<_> = children
                    .iter()
                    .filter(|c| c.kind() == "identifier" || c.kind() == "type_identifier")
                    .collect();

                if identifiers.len() >= 2 {
                    let enum_name = self.intern(self.text(*identifiers[0]));
                    let variant = self.intern(self.text(*identifiers[identifiers.len() - 1]));
                    ExprKind::EnumVariant { enum_name, variant }
                } else if identifiers.len() == 1 {
                    // Just an identifier
                    ExprKind::Ident(self.intern(self.text(*identifiers[0])))
                } else {
                    ExprKind::Error
                }
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
                let callee_node = node
                    .child_by_field_name("function")
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

                // Check if this is a method call (callee is field_expression)
                if callee_node.kind() == "field_expression" {
                    // This is a method call like p.get_x()
                    let receiver_node = callee_node
                        .children(&mut callee_node.walk())
                        .next()
                        .ok_or_else(|| miette::miette!("Method call missing receiver"))?;
                    let receiver = self.lower_expr(receiver_node)?;

                    let method_node = callee_node
                        .children(&mut callee_node.walk())
                        .find(|c| c.kind() == "field_identifier")
                        .ok_or_else(|| miette::miette!("Method call missing method name"))?;
                    let method = self.intern(self.text(method_node));

                    ExprKind::MethodCall {
                        receiver: Box::new(receiver),
                        method,
                        args,
                    }
                } else {
                    // Regular function call
                    let callee = self.lower_expr(callee_node)?;
                    ExprKind::Call {
                        callee: Box::new(callee),
                        args,
                    }
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

            "struct_expression" => {
                // Get name from the type/path
                let name_node = node
                    .child_by_field_name("name")
                    .ok_or_else(|| miette::miette!("Struct literal missing name"))?;
                let name = self.intern(self.text(name_node));

                // Get field initializers from body
                let mut fields = vec![];
                if let Some(body_node) = node.child_by_field_name("body") {
                    let mut cursor = body_node.walk();
                    for child in body_node.children(&mut cursor) {
                        if child.kind() == "field_initializer" {
                            let field_name_node = child
                                .child_by_field_name("field")
                                .or_else(|| child.child_by_field_name("name"))
                                .ok_or_else(|| miette::miette!("Field initializer missing name"))?;
                            let field_name = self.intern(self.text(field_name_node));

                            let value_node = child
                                .child_by_field_name("value")
                                .ok_or_else(|| miette::miette!("Field initializer missing value"))?;
                            let value = self.lower_expr(value_node)?;

                            fields.push((field_name, value));
                        } else if child.kind() == "shorthand_field_initializer" {
                            // Handle `Point { x, y }` shorthand syntax
                            let field_name = self.intern(self.text(child));
                            let value = Expr::new(ExprKind::Ident(field_name), self.span(child));
                            fields.push((field_name, value));
                        }
                    }
                }

                ExprKind::Struct { name, fields }
            }

            "field_expression" => {
                let expr = node
                    .child_by_field_name("value")
                    .map(|n| self.lower_expr(n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Field expression missing value"))?;

                let field_node = node
                    .child_by_field_name("field")
                    .ok_or_else(|| miette::miette!("Field expression missing field name"))?;
                let field = self.intern(self.text(field_node));

                ExprKind::Field {
                    expr: Box::new(expr),
                    field,
                }
            }

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
