use fragile_common::{SourceFile, Span, Symbol, SymbolInterner};
use fragile_hir::{
    Abi, BinOp, Expr, ExprKind, Field, FnDef, FnSig, ImplDef, Item, ItemKind, Literal, Module,
    Mutability, Param, Pattern, PrimitiveType, SourceLang, Stmt, StmtKind,
    StructDef, Type, TypeParam, Visibility,
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
            let items = self.lower_item(child)?;
            for item in items {
                module.add_item(item);
            }
        }

        Ok(module)
    }

    fn lower_item(&self, node: Node) -> Result<Vec<Item>> {
        let span = self.span(node);

        match node.kind() {
            "function_definition" => {
                let fn_def = self.lower_function(node)?;
                Ok(vec![Item::new(ItemKind::Function(fn_def), span)])
            }
            "struct_specifier" => {
                // Only lower if this is a struct definition (has field_declaration_list)
                if node.child_by_field_name("body").is_some()
                    || node
                        .children(&mut node.walk())
                        .any(|c| c.kind() == "field_declaration_list")
                {
                    self.lower_struct_with_methods(node)
                } else {
                    Ok(vec![]) // Forward declaration, skip for now
                }
            }
            "linkage_specification" => {
                // extern "C" { ... }
                self.lower_linkage_specification(node)
            }
            "template_declaration" => {
                // template<typename T> function/struct
                self.lower_template_declaration(node)
            }
            // TODO: class, enum, etc.
            _ => Ok(vec![]),
        }
    }

    fn lower_struct(&self, node: Node) -> Result<StructDef> {
        // Get name
        let name = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "type_identifier")
            .map(|n| self.intern(self.text(n)))
            .ok_or_else(|| miette::miette!("Struct missing name"))?;

        // Get fields from field_declaration_list
        let mut fields = vec![];
        if let Some(field_list) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "field_declaration_list")
        {
            let mut cursor = field_list.walk();
            for child in field_list.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    // Get type
                    let ty = child
                        .child_by_field_name("type")
                        .or_else(|| {
                            child
                                .children(&mut child.walk())
                                .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier")
                        })
                        .map(|n| self.lower_type(n))
                        .transpose()?
                        .unwrap_or(Type::Infer(0));

                    // Get field name
                    let field_name = child
                        .children(&mut child.walk())
                        .find(|c| c.kind() == "field_identifier")
                        .map(|n| self.intern(self.text(n)))
                        .ok_or_else(|| miette::miette!("Field missing name"))?;

                    fields.push(Field {
                        name: field_name,
                        ty,
                        is_public: true, // C++ struct fields are public by default
                    });
                }
            }
        }

        Ok(StructDef {
            name,
            fields,
            type_params: vec![],
        })
    }

    /// Lower a C++ struct that may contain methods, returning StructDef and ImplDef
    fn lower_struct_with_methods(&self, node: Node) -> Result<Vec<Item>> {
        let span = self.span(node);
        let struct_def = self.lower_struct(node)?;
        let struct_name = struct_def.name;

        // Collect field names for implicit self.field access
        let field_names: Vec<Symbol> = struct_def.fields.iter().map(|f| f.name).collect();

        let mut items = vec![Item::new(ItemKind::Struct(struct_def), span)];

        // Look for methods (function_definition) inside the struct
        let mut methods = vec![];
        if let Some(field_list) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "field_declaration_list")
        {
            let mut cursor = field_list.walk();
            for child in field_list.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    // This is a method - lower it with self parameter
                    let method = self.lower_method(child, struct_name, &field_names)?;
                    methods.push(Item::new(ItemKind::Function(method), self.span(child)));
                }
            }
        }

        // If we have methods, create an ImplDef
        if !methods.is_empty() {
            let impl_def = ImplDef {
                type_params: vec![],
                trait_ref: None,
                self_ty: Type::Named { name: struct_name, type_args: vec![] },
                items: methods,
                span,
            };
            items.push(Item::new(ItemKind::Impl(impl_def), span));
        }

        Ok(items)
    }

    /// Lower extern "C" { ... } linkage specification
    fn lower_linkage_specification(&self, node: Node) -> Result<Vec<Item>> {
        let span = self.span(node);
        let mut items = vec![];

        // Check the linkage type (should be "C" for extern "C")
        let is_c_linkage = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "string_literal")
            .map(|n| self.text(n).contains("C"))
            .unwrap_or(false);

        if !is_c_linkage {
            return Ok(vec![]);
        }

        // Process declarations inside the block
        if let Some(decl_list) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "declaration_list")
        {
            let mut cursor = decl_list.walk();
            for child in decl_list.children(&mut cursor) {
                if child.kind() == "declaration" {
                    if let Some(fn_def) = self.lower_extern_function_decl(child)? {
                        items.push(Item::new(ItemKind::Function(fn_def), self.span(child)));
                    }
                }
            }
        }

        Ok(items)
    }

    /// Lower template<typename T> declarations
    fn lower_template_declaration(&self, node: Node) -> Result<Vec<Item>> {
        let span = self.span(node);

        // Parse template parameters (template<typename T, typename U>)
        let type_params = self.lower_template_parameters(node)?;

        // Find the inner function_definition or struct
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "function_definition" => {
                    let mut fn_def = self.lower_template_function(child, &type_params)?;
                    fn_def.type_params = type_params;
                    return Ok(vec![Item::new(ItemKind::Function(fn_def), span)]);
                }
                // TODO: template struct
                _ => {}
            }
        }

        Ok(vec![])
    }

    /// Parse template parameter list <typename T, typename U>
    fn lower_template_parameters(&self, node: Node) -> Result<Vec<TypeParam>> {
        let mut params = vec![];

        if let Some(param_list) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "template_parameter_list")
        {
            for child in param_list.children(&mut param_list.walk()) {
                if child.kind() == "type_parameter_declaration" {
                    // Get the type identifier (T, U, etc.)
                    if let Some(name_node) = child
                        .children(&mut child.walk())
                        .find(|c| c.kind() == "type_identifier")
                    {
                        let name = self.intern(self.text(name_node));
                        params.push(TypeParam {
                            name,
                            bounds: vec![],
                        });
                    }
                }
            }
        }

        Ok(params)
    }

    /// Lower a template function definition
    fn lower_template_function(&self, node: Node, type_params: &[TypeParam]) -> Result<FnDef> {
        let span = self.span(node);

        // Get return type - may be a type parameter
        let ret_ty = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier")
            .map(|n| self.lower_type_with_params(n, type_params))
            .transpose()?
            .unwrap_or(Type::Primitive(PrimitiveType::I32));

        // Get function name and parameters
        let declarator = node
            .child_by_field_name("declarator")
            .ok_or_else(|| miette::miette!("Function missing declarator"))?;

        let name = self.extract_function_name(declarator)?;
        let params = self.extract_parameters_with_type_params(declarator, type_params)?;

        // Get body
        let body = if let Some(body_node) = node.child_by_field_name("body") {
            Some(self.lower_compound_statement(body_node)?)
        } else {
            None
        };

        Ok(FnDef {
            name,
            vis: Visibility::Public,
            sig: FnSig {
                params,
                ret_ty,
                is_variadic: false,
            },
            type_params: vec![], // Will be set by caller
            body,
            source_lang: SourceLang::Cpp,
            abi: Abi::Rust,
            span,
            attributes: vec![],
        })
    }

    /// Lower a type, resolving template parameters to Type::Named
    fn lower_type_with_params(&self, node: Node, _type_params: &[TypeParam]) -> Result<Type> {
        match node.kind() {
            "type_identifier" => {
                let name = self.intern(self.text(node));
                // Type parameters are represented as Type::Named
                // The monomorphization system will substitute them
                Ok(Type::Named {
                    name,
                    type_args: vec![],
                })
            }
            "primitive_type" => self.lower_type(node),
            _ => self.lower_type(node),
        }
    }

    /// Extract parameters, resolving type parameters
    fn extract_parameters_with_type_params(
        &self,
        declarator: Node,
        type_params: &[TypeParam],
    ) -> Result<Vec<Param>> {
        let mut params = vec![];

        if let Some(param_list) = declarator
            .children(&mut declarator.walk())
            .find(|c| c.kind() == "parameter_list")
        {
            let mut cursor = param_list.walk();
            for child in param_list.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    // Get type (may be a type parameter)
                    let ty = child
                        .children(&mut child.walk())
                        .find(|c| {
                            c.kind() == "primitive_type"
                                || c.kind() == "type_identifier"
                                || c.kind() == "pointer_declarator"
                        })
                        .map(|n| self.lower_type_with_params(n, type_params))
                        .transpose()?
                        .unwrap_or(Type::Primitive(PrimitiveType::I32));

                    // Get name
                    let name = child
                        .children(&mut child.walk())
                        .find(|c| c.kind() == "identifier")
                        .map(|n| self.intern(self.text(n)))
                        .unwrap_or_else(|| self.intern("_"));

                    params.push(Param {
                        name,
                        ty,
                        mutability: Mutability::Immutable,
                        span: self.span(child),
                    });
                }
            }
        }

        Ok(params)
    }

    /// Lower an extern function declaration (no body)
    fn lower_extern_function_decl(&self, node: Node) -> Result<Option<FnDef>> {
        let span = self.span(node);

        // Get return type
        let ret_ty = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "primitive_type" || c.kind() == "type_identifier")
            .map(|n| self.lower_type(n))
            .transpose()?
            .unwrap_or(Type::Primitive(PrimitiveType::I32));

        // Get function declarator
        let declarator = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "function_declarator");

        let declarator = match declarator {
            Some(d) => d,
            None => return Ok(None), // Not a function declaration
        };

        // Get function name
        let name = declarator
            .children(&mut declarator.walk())
            .find(|c| c.kind() == "identifier")
            .map(|n| self.intern(self.text(n)))
            .ok_or_else(|| miette::miette!("Extern function missing name"))?;

        // Get parameters
        let params = self.extract_parameters(declarator)?;

        Ok(Some(FnDef {
            name,
            vis: Visibility::Public,
            sig: FnSig {
                params,
                ret_ty,
                is_variadic: false,
            },
            type_params: vec![],
            body: None, // External function - no body
            source_lang: SourceLang::Cpp,
            abi: Abi::C, // Use C ABI
            span,
            attributes: vec![],
        }))
    }

    /// Lower a method inside a struct
    fn lower_method(&self, node: Node, struct_name: Symbol, field_names: &[Symbol]) -> Result<FnDef> {
        let span = self.span(node);

        // Get declarator (contains name and parameters)
        let declarator = node
            .child_by_field_name("declarator")
            .ok_or_else(|| miette::miette!("Method missing declarator"))?;

        // Get name
        let method_name = self.extract_function_name(declarator)?;

        // Get parameters
        let mut params = self.extract_parameters(declarator)?;

        // Add implicit 'self' parameter for methods
        let self_param = Param {
            name: self.intern("self"),
            ty: Type::Reference {
                inner: Box::new(Type::Named { name: struct_name, type_args: vec![] }),
                mutability: Mutability::Immutable,
            },
            mutability: Mutability::Immutable,
            span,
        };
        params.insert(0, self_param);

        // Get return type
        let ret_ty = if let Some(type_node) = node.child_by_field_name("type") {
            self.lower_type(type_node)?
        } else {
            Type::Primitive(PrimitiveType::I32) // C++ default
        };

        // Get body
        let body = if let Some(body_node) = node.child_by_field_name("body") {
            let mut body_expr = self.lower_compound_statement(body_node)?;
            // Transform bare field accesses to self.field
            self.transform_field_accesses(&mut body_expr, field_names);
            Some(body_expr)
        } else {
            None
        };

        // Don't mangle the name here - codegen's compile_impl_methods handles mangling
        Ok(FnDef {
            name: method_name,
            vis: Visibility::Public,
            sig: FnSig {
                params,
                ret_ty,
                is_variadic: false,
            },
            type_params: vec![],
            body,
            source_lang: SourceLang::Cpp,
            abi: Abi::Rust,
            span,
            attributes: vec![],
        })
    }

    /// Transform bare identifier expressions that match field names to self.field
    fn transform_field_accesses(&self, expr: &mut Expr, field_names: &[Symbol]) {
        let self_sym = self.intern("self");

        match &mut expr.kind {
            ExprKind::Ident(sym) => {
                // Check if this identifier is a field name
                if field_names.contains(sym) {
                    let self_expr = Box::new(Expr::new(ExprKind::Ident(self_sym), expr.span));
                    expr.kind = ExprKind::Field { expr: self_expr, field: *sym };
                }
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                self.transform_field_accesses(lhs, field_names);
                self.transform_field_accesses(rhs, field_names);
            }
            ExprKind::Unary { operand, .. } => {
                self.transform_field_accesses(operand, field_names);
            }
            ExprKind::Block { stmts, expr: final_expr } => {
                for stmt in stmts {
                    if let StmtKind::Expr(e) | StmtKind::Let { init: Some(e), .. } = &mut stmt.kind {
                        self.transform_field_accesses(e, field_names);
                    }
                }
                if let Some(e) = final_expr {
                    self.transform_field_accesses(e, field_names);
                }
            }
            ExprKind::Return(Some(e)) => {
                self.transform_field_accesses(e, field_names);
            }
            ExprKind::Call { args, .. } => {
                for arg in args {
                    self.transform_field_accesses(arg, field_names);
                }
            }
            ExprKind::If { cond, then_branch, else_branch } => {
                self.transform_field_accesses(cond, field_names);
                self.transform_field_accesses(then_branch, field_names);
                if let Some(e) = else_branch {
                    self.transform_field_accesses(e, field_names);
                }
            }
            ExprKind::Assign { lhs, rhs } => {
                self.transform_field_accesses(lhs, field_names);
                self.transform_field_accesses(rhs, field_names);
            }
            ExprKind::Field { expr: e, .. } => {
                self.transform_field_accesses(e, field_names);
            }
            _ => {}
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
            abi: Abi::C, // C++ uses C calling convention by default
            attributes: vec![], // TODO: Parse C++ attributes [[...]]
        })
    }

    fn extract_function_name(&self, declarator: Node) -> Result<Symbol> {
        // Navigate through declarator to find the identifier
        fn find_identifier<'a>(node: Node<'a>, text: &'a str) -> Option<&'a str> {
            // For methods, name might be a field_identifier
            if node.kind() == "identifier" || node.kind() == "field_identifier" {
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

        // Get type (may be struct name like "Point")
        let ty = node.child_by_field_name("type").map(|n| self.lower_type(n)).transpose()?;

        // Get struct name if this is a struct type (for initializer_list)
        let struct_name = node
            .child_by_field_name("type")
            .filter(|n| n.kind() == "type_identifier")
            .map(|n| self.intern(self.text(n)));

        // Get declarator (may have initializer)
        if let Some(decl) = node.child_by_field_name("declarator") {
            let (name, init) = self.lower_init_declarator(decl, struct_name)?;

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

    fn lower_init_declarator(
        &self,
        node: Node,
        struct_name: Option<Symbol>,
    ) -> Result<(Symbol, Option<Expr>)> {
        match node.kind() {
            "init_declarator" => {
                let span = self.span(node);
                let name = if let Some(decl) = node.child_by_field_name("declarator") {
                    self.intern(self.text(decl))
                } else {
                    self.intern("_")
                };

                // Check for initializer_list (struct literal like `Point p{1, 2}`)
                let init = node
                    .children(&mut node.walk())
                    .find(|c| c.kind() == "initializer_list")
                    .map(|init_list| {
                        if let Some(struct_sym) = struct_name {
                            // This is a struct literal - collect values and pair with field indices
                            let mut values: Vec<Expr> = vec![];
                            let mut cursor = init_list.walk();
                            for child in init_list.children(&mut cursor) {
                                match child.kind() {
                                    "{" | "}" | "," => continue,
                                    _ => {
                                        if let Ok(expr) = self.lower_expr(child) {
                                            values.push(expr);
                                        }
                                    }
                                }
                            }

                            // Create struct literal with positional fields (field0, field1, etc.)
                            // The actual field names will be resolved during type checking
                            let fields: Vec<(Symbol, Expr)> = values
                                .into_iter()
                                .enumerate()
                                .map(|(i, expr)| (self.intern(&format!("__{}", i)), expr))
                                .collect();

                            Ok(Expr::new(
                                ExprKind::Struct {
                                    name: struct_sym,
                                    fields,
                                },
                                span,
                            ))
                        } else {
                            // Not a struct type, treat as array initializer
                            Err(miette::miette!("Initializer list without struct type"))
                        }
                    })
                    .transpose()?;

                // If no initializer_list, check for regular value
                let init = if init.is_some() {
                    init
                } else if let Some(value) = node.child_by_field_name("value") {
                    Some(self.lower_expr(value)?)
                } else {
                    None
                };

                Ok((name, init))
            }
            "identifier" => Ok((self.intern(self.text(node)), None)),
            _ => Ok((self.intern(self.text(node)), None)),
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

            "field_expression" => {
                // p.x -> Field { expr: p, field: x }
                let mut cursor = node.walk();
                let children: Vec<_> = node.children(&mut cursor).collect();

                // First child is the object expression
                let expr = children
                    .first()
                    .map(|n| self.lower_expr(*n))
                    .transpose()?
                    .ok_or_else(|| miette::miette!("Field expression missing object"))?;

                // Find the field_identifier
                let field = children
                    .iter()
                    .find(|c| c.kind() == "field_identifier")
                    .map(|n| self.intern(self.text(*n)))
                    .ok_or_else(|| miette::miette!("Field expression missing field name"))?;

                ExprKind::Field {
                    expr: Box::new(expr),
                    field,
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
