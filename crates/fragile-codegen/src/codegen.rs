use fragile_common::{Symbol, SymbolInterner};
use fragile_hir::{
    Abi, BinOp, Expr, ExprKind, FnDef, ItemKind, Literal, Module,
    PrimitiveType, Stmt, StmtKind, StructDef, Type, UnaryOp,
};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, CallSiteValue, FunctionValue, PointerValue};
use inkwell::OptimizationLevel;
use inkwell::{AddressSpace, IntPredicate, FloatPredicate};
use miette::Result;
use rustc_hash::FxHashMap;
use std::path::Path;

pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    interner: &'ctx SymbolInterner,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, interner: &'ctx SymbolInterner) -> Self {
        Self { context, interner }
    }

    pub fn compile_module(&self, module: &Module) -> Result<LlvmModule<'ctx>> {
        let name = self.interner.resolve(module.name);
        let llvm_module = self.context.create_module(&name);

        let mut compiler = ModuleCompiler::new(self.context, &llvm_module, self.interner);

        // First pass: register all struct and enum types
        for item in &module.items {
            match &item.kind {
                ItemKind::Struct(struct_def) => {
                    compiler.register_struct(struct_def)?;
                }
                ItemKind::Enum(enum_def) => {
                    compiler.register_enum(enum_def)?;
                }
                _ => {}
            }
        }

        // Second pass: declare all functions (including methods from impl blocks)
        for item in &module.items {
            match &item.kind {
                ItemKind::Function(fn_def) => {
                    compiler.declare_function(fn_def)?;
                }
                ItemKind::Impl(impl_def) => {
                    // Declare methods with mangled names (Type_method)
                    compiler.declare_impl_methods(impl_def)?;
                }
                _ => {}
            }
        }

        // Third pass: compile function bodies (including methods)
        for item in &module.items {
            match &item.kind {
                ItemKind::Function(fn_def) => {
                    compiler.compile_function(fn_def)?;
                }
                ItemKind::Impl(impl_def) => {
                    compiler.compile_impl_methods(impl_def)?;
                }
                _ => {}
            }
        }

        Ok(llvm_module)
    }

    pub fn write_object_file(&self, module: &LlvmModule<'ctx>, path: &Path) -> Result<()> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| miette::miette!("Failed to initialize target: {}", e))?;

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| miette::miette!("Failed to get target: {}", e))?;

        let cpu = TargetMachine::get_host_cpu_name();
        let features = TargetMachine::get_host_cpu_features();

        let target_machine = target
            .create_target_machine(
                &triple,
                cpu.to_str().unwrap_or("generic"),
                features.to_str().unwrap_or(""),
                OptimizationLevel::Default,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or_else(|| miette::miette!("Failed to create target machine"))?;

        target_machine
            .write_to_file(module, FileType::Object, path)
            .map_err(|e| miette::miette!("Failed to write object file: {}", e))?;

        Ok(())
    }
}

/// Info about a struct: its LLVM type and field name -> index mapping
struct StructInfo<'ctx> {
    llvm_type: StructType<'ctx>,
    field_indices: FxHashMap<Symbol, u32>,
}

/// Info about an enum: variant name -> discriminant mapping
struct EnumInfo {
    variant_discriminants: FxHashMap<Symbol, i128>,
}

struct ModuleCompiler<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a LlvmModule<'ctx>,
    builder: Builder<'ctx>,
    interner: &'a SymbolInterner,
    functions: FxHashMap<String, FunctionValue<'ctx>>,
    variables: FxHashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    struct_types: FxHashMap<Symbol, StructInfo<'ctx>>,
    enum_types: FxHashMap<Symbol, EnumInfo>,
}

impl<'a, 'ctx> ModuleCompiler<'a, 'ctx> {
    fn new(
        context: &'ctx Context,
        module: &'a LlvmModule<'ctx>,
        interner: &'a SymbolInterner,
    ) -> Self {
        Self {
            context,
            module,
            builder: context.create_builder(),
            interner,
            functions: FxHashMap::default(),
            variables: FxHashMap::default(),
            struct_types: FxHashMap::default(),
            enum_types: FxHashMap::default(),
        }
    }

    fn register_struct(&mut self, struct_def: &StructDef) -> Result<()> {
        let name = self.interner.resolve(struct_def.name);

        // Create LLVM struct type
        let field_types: Vec<BasicTypeEnum> = struct_def
            .fields
            .iter()
            .filter_map(|f| self.lower_type(&f.ty))
            .collect();

        let struct_type = self.context.struct_type(&field_types, false);

        // Create field name -> index mapping
        let mut field_indices = FxHashMap::default();
        for (i, field) in struct_def.fields.iter().enumerate() {
            field_indices.insert(field.name, i as u32);
        }

        // Register named struct type in LLVM module
        let _named_struct = self.context.opaque_struct_type(&name);

        self.struct_types.insert(
            struct_def.name,
            StructInfo {
                llvm_type: struct_type,
                field_indices,
            },
        );

        Ok(())
    }

    fn register_enum(&mut self, enum_def: &fragile_hir::EnumDef) -> Result<()> {
        // Build variant name -> discriminant mapping
        let mut variant_discriminants = FxHashMap::default();
        for variant in &enum_def.variants {
            if let Some(disc) = variant.discriminant {
                variant_discriminants.insert(variant.name, disc);
            }
        }

        self.enum_types.insert(
            enum_def.name,
            EnumInfo {
                variant_discriminants,
            },
        );

        Ok(())
    }

    fn declare_function(&mut self, fn_def: &FnDef) -> Result<FunctionValue<'ctx>> {
        let name = self.interner.resolve(fn_def.name);

        // Get parameter types
        let param_types: Vec<BasicMetadataTypeEnum> = fn_def
            .sig
            .params
            .iter()
            .filter_map(|p| self.lower_type(&p.ty).map(|t| t.into()))
            .collect();

        // Get return type
        let ret_type = self.lower_type(&fn_def.sig.ret_ty);

        // Handle variadic functions
        let is_variadic = fn_def.sig.is_variadic;

        let fn_type = match ret_type {
            Some(ty) => ty.fn_type(&param_types, is_variadic),
            None => self.context.void_type().fn_type(&param_types, is_variadic),
        };

        let function = self.module.add_function(&name, fn_type, None);

        // Set C calling convention for extern "C" functions
        if fn_def.abi == Abi::C {
            function.set_call_conventions(0); // 0 = C calling convention
        }

        self.functions.insert(name.to_string(), function);

        Ok(function)
    }

    fn compile_function(&mut self, fn_def: &FnDef) -> Result<()> {
        // Skip extern functions (no body to compile)
        if fn_def.body.is_none() {
            return Ok(());
        }

        let name = self.interner.resolve(fn_def.name);
        let function = self
            .functions
            .get(name.as_str())
            .copied()
            .ok_or_else(|| miette::miette!("Function {} not found", name))?;

        // Clear variables for this function
        self.variables.clear();

        // Create entry block
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Create allocas for parameters
        for (i, param) in fn_def.sig.params.iter().enumerate() {
            let param_name = self.interner.resolve(param.name);
            let param_value = function.get_nth_param(i as u32);

            if let Some(value) = param_value {
                let ty = value.get_type();
                let alloca = self.create_entry_alloca(function, &param_name, ty);
                self.builder.build_store(alloca, value)
                    .map_err(|e| miette::miette!("Failed to store param: {:?}", e))?;
                self.variables.insert(param_name.to_string(), (alloca, ty));
            }
        }

        // Compile body
        if let Some(body) = &fn_def.body {
            let result = self.compile_expr(body, function)?;

            // Add return if needed
            if !self.current_block_has_terminator() {
                if fn_def.sig.ret_ty == Type::unit() {
                    self.builder.build_return(None)
                        .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                } else if let Some(val) = result {
                    self.builder.build_return(Some(&val))
                        .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                } else {
                    self.builder.build_return(None)
                        .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                }
            }
        }

        Ok(())
    }

    fn declare_impl_methods(&mut self, impl_def: &fragile_hir::ImplDef) -> Result<()> {
        // Get the type name for mangling
        let type_name = match &impl_def.self_ty {
            Type::Named { name, .. } => self.interner.resolve(*name).to_string(),
            _ => return Ok(()), // Skip non-named types for now
        };

        for item in &impl_def.items {
            if let ItemKind::Function(fn_def) = &item.kind {
                let method_name = self.interner.resolve(fn_def.name);
                let mangled_name = format!("{}_{}", type_name, method_name);

                // Get parameter types
                let param_types: Vec<BasicMetadataTypeEnum> = fn_def
                    .sig
                    .params
                    .iter()
                    .filter_map(|p| self.lower_type(&p.ty).map(|t| t.into()))
                    .collect();

                // Get return type
                let ret_type = self.lower_type(&fn_def.sig.ret_ty);

                let fn_type = match ret_type {
                    Some(ty) => ty.fn_type(&param_types, false),
                    None => self.context.void_type().fn_type(&param_types, false),
                };

                let function = self.module.add_function(&mangled_name, fn_type, None);
                self.functions.insert(mangled_name, function);
            }
        }

        Ok(())
    }

    fn compile_impl_methods(&mut self, impl_def: &fragile_hir::ImplDef) -> Result<()> {
        // Get the type name for mangling
        let type_name = match &impl_def.self_ty {
            Type::Named { name, .. } => self.interner.resolve(*name).to_string(),
            _ => return Ok(()),
        };

        for item in &impl_def.items {
            if let ItemKind::Function(fn_def) = &item.kind {
                if fn_def.body.is_none() {
                    continue;
                }

                let method_name = self.interner.resolve(fn_def.name);
                let mangled_name = format!("{}_{}", type_name, method_name);

                let function = self
                    .functions
                    .get(&mangled_name)
                    .copied()
                    .ok_or_else(|| miette::miette!("Method {} not found", mangled_name))?;

                // Clear variables for this function
                self.variables.clear();

                // Create entry block
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Create allocas for parameters (including self)
                for (i, param) in fn_def.sig.params.iter().enumerate() {
                    let param_name = self.interner.resolve(param.name);
                    let param_val = function.get_nth_param(i as u32).ok_or_else(|| {
                        miette::miette!("Missing parameter {}", param_name)
                    })?;

                    let param_ty = self.lower_type(&param.ty).ok_or_else(|| {
                        miette::miette!("Unsupported parameter type")
                    })?;

                    let alloca = self.builder.build_alloca(param_ty, &param_name)
                        .map_err(|e| miette::miette!("Failed to alloca: {:?}", e))?;

                    self.builder.build_store(alloca, param_val)
                        .map_err(|e| miette::miette!("Failed to store: {:?}", e))?;

                    self.variables.insert(param_name.to_string(), (alloca, param_ty));
                }

                // Compile method body
                if let Some(body) = &fn_def.body {
                    let result = self.compile_expr(body, function)?;

                    // Handle return
                    if result.is_none() {
                        self.builder.build_return(None)
                            .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                    } else if fn_def.sig.ret_ty != Type::unit() {
                        if let Some(val) = result {
                            self.builder.build_return(Some(&val))
                                .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                        }
                    } else {
                        self.builder.build_return(None)
                            .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn compile_expr(
        &mut self,
        expr: &Expr,
        function: FunctionValue<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.compile_literal(lit),

            ExprKind::Ident(symbol) => {
                let name = self.interner.resolve(*symbol);
                if let Some(&(ptr, ty)) = self.variables.get(name.as_str()) {
                    let val = self.builder.build_load(ty, ptr, &name)
                        .map_err(|e| miette::miette!("Failed to load: {:?}", e))?;
                    Ok(Some(val))
                } else if let Some(&func) = self.functions.get(name.as_str()) {
                    Ok(Some(func.as_global_value().as_pointer_value().into()))
                } else {
                    Err(miette::miette!("Unknown variable: {}", name))
                }
            }

            ExprKind::Binary { op, lhs, rhs } => {
                let lhs_val = self
                    .compile_expr(lhs, function)?
                    .ok_or_else(|| miette::miette!("Binary lhs has no value"))?;
                let rhs_val = self
                    .compile_expr(rhs, function)?
                    .ok_or_else(|| miette::miette!("Binary rhs has no value"))?;

                self.compile_binary_op(*op, lhs_val, rhs_val)
            }

            ExprKind::Unary { op, operand } => {
                let val = self
                    .compile_expr(operand, function)?
                    .ok_or_else(|| miette::miette!("Unary operand has no value"))?;

                self.compile_unary_op(*op, val)
            }

            ExprKind::Call { callee, args } => {
                let _callee_val = self.compile_expr(callee, function)?;

                // Compile arguments
                let arg_vals: Vec<BasicMetadataValueEnum> = args
                    .iter()
                    .filter_map(|a| self.compile_expr(a, function).ok().flatten())
                    .map(|v| v.into())
                    .collect();

                // Get function to call
                if let ExprKind::Ident(sym) = &callee.kind {
                    let name = self.interner.resolve(*sym);
                    if let Some(&func) = self.functions.get(name.as_str()) {
                        let call = self.builder.build_call(func, &arg_vals, "call")
                            .map_err(|e| miette::miette!("Failed to build call: {:?}", e))?;
                        let value = match call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => Some(v),
                            inkwell::values::ValueKind::Instruction(_) => None,
                        };
                        return Ok(value);
                    }
                }

                Ok(None)
            }

            ExprKind::MethodCall { receiver, method, args } => {
                // Compile receiver (the object we're calling the method on)
                let receiver_val = self
                    .compile_expr(receiver, function)?
                    .ok_or_else(|| miette::miette!("Method receiver has no value"))?;

                // Try to determine the type name from the receiver
                // For now, we'll look up the variable to find its type
                let type_name = if let ExprKind::Ident(sym) = &receiver.kind {
                    let var_name = self.interner.resolve(*sym);
                    if let Some(&(_ptr, ty)) = self.variables.get(var_name.as_str()) {
                        // Find the struct type that matches this type
                        let mut found_name = None;
                        for (struct_sym, info) in &self.struct_types {
                            if info.llvm_type.as_basic_type_enum() == ty {
                                found_name = Some(self.interner.resolve(*struct_sym).to_string());
                                break;
                            }
                        }
                        found_name
                    } else {
                        None
                    }
                } else {
                    None
                };

                let type_name = type_name.ok_or_else(|| {
                    miette::miette!("Could not determine receiver type for method call")
                })?;

                let method_name = self.interner.resolve(*method);
                let mangled_name = format!("{}_{}", type_name, method_name);

                // Get the method function
                let func = self.functions.get(&mangled_name).copied().ok_or_else(|| {
                    miette::miette!("Method {} not found", mangled_name)
                })?;

                // Build arguments: receiver (as pointer to the variable) + explicit args
                let mut arg_vals: Vec<BasicMetadataValueEnum> = vec![];

                // Pass receiver - need to pass a pointer for &self
                if let ExprKind::Ident(sym) = &receiver.kind {
                    let var_name = self.interner.resolve(*sym);
                    if let Some(&(ptr, _ty)) = self.variables.get(var_name.as_str()) {
                        arg_vals.push(ptr.into());
                    }
                }

                // Add explicit arguments
                for arg in args {
                    if let Some(val) = self.compile_expr(arg, function)? {
                        arg_vals.push(val.into());
                    }
                }

                let call = self.builder.build_call(func, &arg_vals, "method_call")
                    .map_err(|e| miette::miette!("Failed to build method call: {:?}", e))?;

                let value = match call.try_as_basic_value() {
                    inkwell::values::ValueKind::Basic(v) => Some(v),
                    inkwell::values::ValueKind::Instruction(_) => None,
                };
                Ok(value)
            }

            ExprKind::Block { stmts, expr } => {
                for stmt in stmts {
                    self.compile_stmt(stmt, function)?;
                }
                if let Some(e) = expr {
                    self.compile_expr(e, function)
                } else {
                    Ok(None)
                }
            }

            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_val = self
                    .compile_expr(cond, function)?
                    .ok_or_else(|| miette::miette!("If condition has no value"))?;

                let cond_bool = if cond_val.is_int_value() {
                    let int_val = cond_val.into_int_value();
                    let zero = int_val.get_type().const_zero();
                    self.builder.build_int_compare(IntPredicate::NE, int_val, zero, "cond")
                        .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                } else {
                    return Err(miette::miette!("If condition must be boolean"));
                };

                let then_bb = self.context.append_basic_block(function, "then");
                let else_bb = self.context.append_basic_block(function, "else");
                let merge_bb = self.context.append_basic_block(function, "merge");

                self.builder.build_conditional_branch(cond_bool, then_bb, else_bb)
                    .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;

                // Then block
                self.builder.position_at_end(then_bb);
                let then_val = self.compile_expr(then_branch, function)?;
                if !self.current_block_has_terminator() {
                    self.builder.build_unconditional_branch(merge_bb)
                        .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                }
                let then_bb_end = self.builder.get_insert_block().unwrap();

                // Else block
                self.builder.position_at_end(else_bb);
                let else_val = if let Some(else_branch) = else_branch {
                    self.compile_expr(else_branch, function)?
                } else {
                    None
                };
                if !self.current_block_has_terminator() {
                    self.builder.build_unconditional_branch(merge_bb)
                        .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                }
                let else_bb_end = self.builder.get_insert_block().unwrap();

                // Merge block
                self.builder.position_at_end(merge_bb);

                // Create phi if both branches have values
                if let (Some(then_v), Some(else_v)) = (then_val, else_val) {
                    if then_v.get_type() == else_v.get_type() {
                        let phi = self.builder.build_phi(then_v.get_type(), "ifphi")
                            .map_err(|e| miette::miette!("Failed to build phi: {:?}", e))?;
                        phi.add_incoming(&[(&then_v, then_bb_end), (&else_v, else_bb_end)]);
                        return Ok(Some(phi.as_basic_value()));
                    }
                }

                Ok(None)
            }

            ExprKind::While { cond, body } => {
                let cond_bb = self.context.append_basic_block(function, "while.cond");
                let body_bb = self.context.append_basic_block(function, "while.body");
                let end_bb = self.context.append_basic_block(function, "while.end");

                self.builder.build_unconditional_branch(cond_bb)
                    .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;

                // Condition
                self.builder.position_at_end(cond_bb);
                let cond_val = self
                    .compile_expr(cond, function)?
                    .ok_or_else(|| miette::miette!("While condition has no value"))?;

                let cond_bool = if cond_val.is_int_value() {
                    let int_val = cond_val.into_int_value();
                    let zero = int_val.get_type().const_zero();
                    self.builder.build_int_compare(IntPredicate::NE, int_val, zero, "cond")
                        .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                } else {
                    return Err(miette::miette!("While condition must be boolean"));
                };

                self.builder.build_conditional_branch(cond_bool, body_bb, end_bb)
                    .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;

                // Body
                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if !self.current_block_has_terminator() {
                    self.builder.build_unconditional_branch(cond_bb)
                        .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                }

                // End
                self.builder.position_at_end(end_bb);

                Ok(None)
            }

            ExprKind::Loop { body } => {
                let body_bb = self.context.append_basic_block(function, "loop.body");
                let end_bb = self.context.append_basic_block(function, "loop.end");

                self.builder.build_unconditional_branch(body_bb)
                    .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;

                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if !self.current_block_has_terminator() {
                    self.builder.build_unconditional_branch(body_bb)
                        .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                }

                self.builder.position_at_end(end_bb);

                Ok(None)
            }

            ExprKind::Return(value) => {
                if let Some(val_expr) = value {
                    let val = self.compile_expr(val_expr, function)?;
                    if let Some(v) = val {
                        self.builder.build_return(Some(&v))
                            .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                    } else {
                        self.builder.build_return(None)
                            .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                    }
                } else {
                    self.builder.build_return(None)
                        .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                }
                Ok(None)
            }

            ExprKind::Assign { lhs, rhs } => {
                let rhs_val = self
                    .compile_expr(rhs, function)?
                    .ok_or_else(|| miette::miette!("Assignment rhs has no value"))?;

                if let ExprKind::Ident(sym) = &lhs.kind {
                    let name = self.interner.resolve(*sym);
                    if let Some(&(ptr, _ty)) = self.variables.get(name.as_str()) {
                        self.builder.build_store(ptr, rhs_val)
                            .map_err(|e| miette::miette!("Failed to store: {:?}", e))?;
                        return Ok(Some(rhs_val));
                    }
                }

                Ok(None)
            }

            ExprKind::Struct { name, fields } => {
                // Look up the struct type and copy needed data to avoid borrow issues
                let (struct_type, field_indices) = {
                    let struct_info = self
                        .struct_types
                        .get(name)
                        .ok_or_else(|| {
                            let name_str = self.interner.resolve(*name);
                            miette::miette!("Unknown struct type: {}", name_str)
                        })?;
                    (struct_info.llvm_type, struct_info.field_indices.clone())
                };

                // Allocate space for the struct on the stack
                let alloca = self
                    .builder
                    .build_alloca(struct_type, "struct_tmp")
                    .map_err(|e| miette::miette!("Failed to alloca struct: {:?}", e))?;

                // Initialize each field
                for (field_name, field_expr) in fields {
                    let field_val = self
                        .compile_expr(field_expr, function)?
                        .ok_or_else(|| miette::miette!("Struct field has no value"))?;

                    // Check if this is a positional field (__0, __1, etc. from C++)
                    let field_name_str = self.interner.resolve(*field_name);
                    let field_idx = if field_name_str.starts_with("__") {
                        // Positional field - extract the index directly
                        field_name_str[2..]
                            .parse::<u32>()
                            .map_err(|_| miette::miette!("Invalid positional field: {}", field_name_str))?
                    } else {
                        // Named field - look up in field_indices
                        *field_indices.get(field_name).ok_or_else(|| {
                            miette::miette!("Unknown struct field: {}", field_name_str)
                        })?
                    };

                    let field_ptr = self
                        .builder
                        .build_struct_gep(struct_type, alloca, field_idx, "field_ptr")
                        .map_err(|e| miette::miette!("Failed to get field ptr: {:?}", e))?;

                    self.builder
                        .build_store(field_ptr, field_val)
                        .map_err(|e| miette::miette!("Failed to store field: {:?}", e))?;
                }

                // Load the entire struct value
                let struct_val = self
                    .builder
                    .build_load(struct_type, alloca, "struct_val")
                    .map_err(|e| miette::miette!("Failed to load struct: {:?}", e))?;

                Ok(Some(struct_val))
            }

            ExprKind::EnumVariant { enum_name, variant } => {
                // Look up the discriminant value for this variant
                let enum_info = self.enum_types.get(enum_name).ok_or_else(|| {
                    let name_str = self.interner.resolve(*enum_name);
                    miette::miette!("Unknown enum type: {}", name_str)
                })?;

                let discriminant = *enum_info.variant_discriminants.get(variant).ok_or_else(|| {
                    let enum_str = self.interner.resolve(*enum_name);
                    let var_str = self.interner.resolve(*variant);
                    miette::miette!("Unknown variant {}::{}", enum_str, var_str)
                })?;

                // Return the discriminant as an i32 value
                let int_type = self.context.i32_type();
                let value = int_type.const_int(discriminant as u64, false);
                Ok(Some(value.as_basic_value_enum()))
            }

            ExprKind::Field { expr, field } => {
                // If the expression is an identifier, look it up to get its pointer
                if let ExprKind::Ident(sym) = &expr.kind {
                    let name = self.interner.resolve(*sym);
                    if let Some(&(ptr, ty)) = self.variables.get(name.as_str()) {
                        // Check if this is a struct type directly
                        for (_struct_name, info) in &self.struct_types {
                            if info.llvm_type.as_basic_type_enum() == ty {
                                if let Some(&field_idx) = info.field_indices.get(field) {
                                    let field_ptr = self
                                        .builder
                                        .build_struct_gep(info.llvm_type, ptr, field_idx, "field_ptr")
                                        .map_err(|e| miette::miette!("Failed to get field ptr: {:?}", e))?;

                                    let field_ty = info.llvm_type.get_field_type_at_index(field_idx)
                                        .ok_or_else(|| miette::miette!("Field type not found"))?;

                                    let field_val = self
                                        .builder
                                        .build_load(field_ty, field_ptr, "field_val")
                                        .map_err(|e| miette::miette!("Failed to load field: {:?}", e))?;

                                    return Ok(Some(field_val));
                                }
                            }
                        }

                        // Check if this is a pointer/reference to a struct (like &self)
                        // In this case, ty is ptr type, and we need to load the struct pointer
                        // then do GEP
                        if ty.is_pointer_type() {
                            // Load the pointer (self is stored as a pointer to a pointer)
                            let struct_ptr = self.builder.build_load(ty, ptr, "self_ptr")
                                .map_err(|e| miette::miette!("Failed to load self ptr: {:?}", e))?;

                            if struct_ptr.is_pointer_value() {
                                let struct_ptr = struct_ptr.into_pointer_value();

                                // Try to find matching struct type and do GEP
                                for (_struct_name, info) in &self.struct_types {
                                    if let Some(&field_idx) = info.field_indices.get(field) {
                                        let field_ptr = self
                                            .builder
                                            .build_struct_gep(info.llvm_type, struct_ptr, field_idx, "field_ptr")
                                            .map_err(|e| miette::miette!("Failed to get field ptr: {:?}", e))?;

                                        let field_ty = info.llvm_type.get_field_type_at_index(field_idx)
                                            .ok_or_else(|| miette::miette!("Field type not found"))?;

                                        let field_val = self
                                            .builder
                                            .build_load(field_ty, field_ptr, "field_val")
                                            .map_err(|e| miette::miette!("Failed to load field: {:?}", e))?;

                                        return Ok(Some(field_val));
                                    }
                                }
                            }
                        }
                    }
                }

                // Check if this is a tuple field access (numeric field name like "0", "1")
                let field_name = self.interner.resolve(*field);
                if let Ok(field_idx) = field_name.parse::<u32>() {
                    // This is a tuple field access like t.0
                    if let ExprKind::Ident(sym) = &expr.kind {
                        let var_name = self.interner.resolve(*sym);
                        if let Some(&(ptr, ty)) = self.variables.get(var_name.as_str()) {
                            // Check if the type is a struct (tuple)
                            if ty.is_struct_type() {
                                let struct_type = ty.into_struct_type();
                                let field_ptr = self
                                    .builder
                                    .build_struct_gep(struct_type, ptr, field_idx, "tuple_field_ptr")
                                    .map_err(|e| miette::miette!("Failed to get tuple field ptr: {:?}", e))?;

                                let field_ty = struct_type.get_field_type_at_index(field_idx)
                                    .ok_or_else(|| miette::miette!("Tuple field {} not found", field_idx))?;

                                let field_val = self
                                    .builder
                                    .build_load(field_ty, field_ptr, "tuple_field_val")
                                    .map_err(|e| miette::miette!("Failed to load tuple field: {:?}", e))?;

                                return Ok(Some(field_val));
                            }
                        }
                    }
                }

                // Fallback: compile the expression and try to use it
                let struct_val = self
                    .compile_expr(expr, function)?
                    .ok_or_else(|| miette::miette!("Field access on non-value"))?;

                // If we have a struct value directly, use extractvalue
                if struct_val.is_struct_value() {
                    let struct_v = struct_val.into_struct_value();

                    // Check if field name is numeric (tuple access)
                    if let Ok(field_idx) = field_name.parse::<u32>() {
                        let field_val = self
                            .builder
                            .build_extract_value(struct_v, field_idx, "tuple_field_val")
                            .map_err(|e| miette::miette!("Failed to extract tuple field: {:?}", e))?;
                        return Ok(Some(field_val));
                    }

                    // Find the struct type and field index
                    for (_struct_name, info) in &self.struct_types {
                        if let Some(&field_idx) = info.field_indices.get(field) {
                            let field_val = self
                                .builder
                                .build_extract_value(struct_v, field_idx, "field_val")
                                .map_err(|e| miette::miette!("Failed to extract field: {:?}", e))?;
                            return Ok(Some(field_val));
                        }
                    }
                }

                Err(miette::miette!("Could not resolve field access: {}", field_name))
            }

            ExprKind::Tuple(elements) => {
                // Compile each element
                let mut values: Vec<BasicValueEnum> = vec![];
                for elem in elements {
                    if let Some(val) = self.compile_expr(elem, function)? {
                        values.push(val);
                    }
                }

                if values.is_empty() {
                    return Ok(None); // Unit tuple
                }

                // Create tuple type from element types
                let field_types: Vec<BasicTypeEnum> = values
                    .iter()
                    .map(|v| v.get_type())
                    .collect();
                let tuple_type = self.context.struct_type(&field_types, false);

                // Build the tuple value
                let mut tuple_val = tuple_type.get_undef();
                for (i, val) in values.into_iter().enumerate() {
                    tuple_val = self.builder
                        .build_insert_value(tuple_val, val, i as u32, "tuple_elem")
                        .map_err(|e| miette::miette!("Failed to insert tuple element: {:?}", e))?
                        .into_struct_value();
                }

                Ok(Some(tuple_val.into()))
            }

            ExprKind::Match { scrutinee, arms } => {
                // Compile scrutinee
                let scrutinee_val = self
                    .compile_expr(scrutinee, function)?
                    .ok_or_else(|| miette::miette!("Match scrutinee has no value"))?;

                // We need the scrutinee to be an integer for switch
                let scrutinee_int = scrutinee_val.into_int_value();

                // Create blocks for each arm and merge block
                let merge_bb = self.context.append_basic_block(function, "match.merge");
                let mut arm_blocks = vec![];
                let mut default_block = None;

                for (i, arm) in arms.iter().enumerate() {
                    let block = self.context.append_basic_block(function, &format!("match.arm.{}", i));
                    arm_blocks.push(block);
                    if matches!(&arm.pattern, fragile_hir::Pattern::Wildcard) {
                        default_block = Some(block);
                    }
                }

                // If no wildcard, use unreachable as default
                let default_bb = default_block.unwrap_or_else(|| {
                    self.context.append_basic_block(function, "match.unreachable")
                });

                // Build switch cases
                let mut cases = vec![];
                for (i, arm) in arms.iter().enumerate() {
                    if let fragile_hir::Pattern::Literal(fragile_hir::Literal::Int(value)) = &arm.pattern {
                        let case_val = scrutinee_int.get_type().const_int(*value as u64, false);
                        cases.push((case_val, arm_blocks[i]));
                    }
                }

                // Build switch instruction
                self.builder.build_switch(scrutinee_int, default_bb, &cases)
                    .map_err(|e| miette::miette!("Failed to build switch: {:?}", e))?;

                // Compile each arm body
                let mut incoming: Vec<(BasicValueEnum, inkwell::basic_block::BasicBlock)> = vec![];
                for (i, arm) in arms.iter().enumerate() {
                    self.builder.position_at_end(arm_blocks[i]);
                    if let Some(val) = self.compile_expr(&arm.body, function)? {
                        if !self.current_block_has_terminator() {
                            self.builder.build_unconditional_branch(merge_bb)
                                .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                        }
                        let block_end = self.builder.get_insert_block().unwrap();
                        incoming.push((val, block_end));
                    } else if !self.current_block_has_terminator() {
                        self.builder.build_unconditional_branch(merge_bb)
                            .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                    }
                }

                // Position at merge and create phi
                self.builder.position_at_end(merge_bb);

                if incoming.is_empty() {
                    Ok(None)
                } else {
                    let phi = self.builder.build_phi(incoming[0].0.get_type(), "match.phi")
                        .map_err(|e| miette::miette!("Failed to build phi: {:?}", e))?;
                    for (val, block) in &incoming {
                        phi.add_incoming(&[(val, *block)]);
                    }
                    Ok(Some(phi.as_basic_value()))
                }
            }

            _ => Ok(None),
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, function: FunctionValue<'ctx>) -> Result<()> {
        match &stmt.kind {
            StmtKind::Let {
                pattern,
                ty,
                init,
                mutability: _,
            } => {
                if let fragile_hir::Pattern::Ident(sym) = pattern {
                    let name = self.interner.resolve(*sym);

                    // Determine type from init or type annotation
                    let llvm_ty = if let Some(init_expr) = init {
                        let init_val = self.compile_expr(init_expr, function)?;
                        if let Some(val) = init_val {
                            let val_ty = val.get_type();
                            let alloca = self.create_entry_alloca(function, &name, val_ty);
                            self.builder.build_store(alloca, val)
                                .map_err(|e| miette::miette!("Failed to store: {:?}", e))?;
                            self.variables.insert(name.to_string(), (alloca, val_ty));
                        }
                        return Ok(());
                    } else if let Some(t) = ty {
                        self.lower_type(t)
                    } else {
                        None
                    };

                    if let Some(llvm_ty) = llvm_ty {
                        let alloca = self.create_entry_alloca(function, &name, llvm_ty);
                        self.variables.insert(name.to_string(), (alloca, llvm_ty));
                    }
                }
            }

            StmtKind::Expr(expr) => {
                self.compile_expr(expr, function)?;
            }

            StmtKind::Empty => {}

            StmtKind::Item(_) => {
                // Nested items not supported yet
            }
        }

        Ok(())
    }

    fn compile_literal(&self, lit: &Literal) -> Result<Option<BasicValueEnum<'ctx>>> {
        match lit {
            Literal::Int(v) => {
                let ty = self.context.i64_type();
                Ok(Some(ty.const_int(*v as u64, true).into()))
            }
            Literal::Float(v) => {
                let ty = self.context.f64_type();
                Ok(Some(ty.const_float(*v).into()))
            }
            Literal::Bool(v) => {
                let ty = self.context.bool_type();
                Ok(Some(ty.const_int(*v as u64, false).into()))
            }
            Literal::Char(c) => {
                let ty = self.context.i32_type();
                Ok(Some(ty.const_int(*c as u64, false).into()))
            }
            Literal::String(_s) => {
                // TODO: String handling
                Ok(None)
            }
            Literal::Unit => Ok(None),
        }
    }

    fn compile_binary_op(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        if lhs.is_int_value() && rhs.is_int_value() {
            let lhs_int = lhs.into_int_value();
            let rhs_int = rhs.into_int_value();

            let result = match op {
                BinOp::Add => self.builder.build_int_add(lhs_int, rhs_int, "add"),
                BinOp::Sub => self.builder.build_int_sub(lhs_int, rhs_int, "sub"),
                BinOp::Mul => self.builder.build_int_mul(lhs_int, rhs_int, "mul"),
                BinOp::Div => self.builder.build_int_signed_div(lhs_int, rhs_int, "div"),
                BinOp::Rem => self.builder.build_int_signed_rem(lhs_int, rhs_int, "rem"),
                BinOp::BitAnd => self.builder.build_and(lhs_int, rhs_int, "and"),
                BinOp::BitOr => self.builder.build_or(lhs_int, rhs_int, "or"),
                BinOp::BitXor => self.builder.build_xor(lhs_int, rhs_int, "xor"),
                BinOp::Shl => self.builder.build_left_shift(lhs_int, rhs_int, "shl"),
                BinOp::Shr => self.builder.build_right_shift(lhs_int, rhs_int, true, "shr"),
                BinOp::Eq => {
                    return Ok(Some(
                        self.builder
                            .build_int_compare(IntPredicate::EQ, lhs_int, rhs_int, "eq")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Ne => {
                    return Ok(Some(
                        self.builder
                            .build_int_compare(IntPredicate::NE, lhs_int, rhs_int, "ne")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Lt => {
                    return Ok(Some(
                        self.builder
                            .build_int_compare(IntPredicate::SLT, lhs_int, rhs_int, "lt")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Le => {
                    return Ok(Some(
                        self.builder
                            .build_int_compare(IntPredicate::SLE, lhs_int, rhs_int, "le")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Gt => {
                    return Ok(Some(
                        self.builder
                            .build_int_compare(IntPredicate::SGT, lhs_int, rhs_int, "gt")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Ge => {
                    return Ok(Some(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, lhs_int, rhs_int, "ge")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::And => self.builder.build_and(lhs_int, rhs_int, "land"),
                BinOp::Or => self.builder.build_or(lhs_int, rhs_int, "lor"),
            };

            Ok(Some(result.map_err(|e| miette::miette!("Failed to build op: {:?}", e))?.into()))
        } else if lhs.is_float_value() && rhs.is_float_value() {
            let lhs_float = lhs.into_float_value();
            let rhs_float = rhs.into_float_value();

            let result = match op {
                BinOp::Add => self.builder.build_float_add(lhs_float, rhs_float, "fadd"),
                BinOp::Sub => self.builder.build_float_sub(lhs_float, rhs_float, "fsub"),
                BinOp::Mul => self.builder.build_float_mul(lhs_float, rhs_float, "fmul"),
                BinOp::Div => self.builder.build_float_div(lhs_float, rhs_float, "fdiv"),
                BinOp::Rem => self.builder.build_float_rem(lhs_float, rhs_float, "frem"),
                BinOp::Eq => {
                    return Ok(Some(
                        self.builder
                            .build_float_compare(FloatPredicate::OEQ, lhs_float, rhs_float, "feq")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Ne => {
                    return Ok(Some(
                        self.builder
                            .build_float_compare(FloatPredicate::ONE, lhs_float, rhs_float, "fne")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Lt => {
                    return Ok(Some(
                        self.builder
                            .build_float_compare(FloatPredicate::OLT, lhs_float, rhs_float, "flt")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Le => {
                    return Ok(Some(
                        self.builder
                            .build_float_compare(FloatPredicate::OLE, lhs_float, rhs_float, "fle")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Gt => {
                    return Ok(Some(
                        self.builder
                            .build_float_compare(FloatPredicate::OGT, lhs_float, rhs_float, "fgt")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                BinOp::Ge => {
                    return Ok(Some(
                        self.builder
                            .build_float_compare(FloatPredicate::OGE, lhs_float, rhs_float, "fge")
                            .map_err(|e| miette::miette!("Failed to build compare: {:?}", e))?
                            .into(),
                    ))
                }
                _ => return Err(miette::miette!("Invalid float operation")),
            };

            Ok(Some(result.map_err(|e| miette::miette!("Failed to build op: {:?}", e))?.into()))
        } else {
            Err(miette::miette!("Type mismatch in binary operation"))
        }
    }

    fn compile_unary_op(
        &self,
        op: UnaryOp,
        val: BasicValueEnum<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        match op {
            UnaryOp::Neg => {
                if val.is_int_value() {
                    let result = self.builder.build_int_neg(val.into_int_value(), "neg")
                        .map_err(|e| miette::miette!("Failed to build neg: {:?}", e))?;
                    Ok(Some(result.into()))
                } else if val.is_float_value() {
                    let result = self.builder.build_float_neg(val.into_float_value(), "fneg")
                        .map_err(|e| miette::miette!("Failed to build fneg: {:?}", e))?;
                    Ok(Some(result.into()))
                } else {
                    Err(miette::miette!("Cannot negate this type"))
                }
            }
            UnaryOp::Not => {
                if val.is_int_value() {
                    let result = self.builder.build_not(val.into_int_value(), "not")
                        .map_err(|e| miette::miette!("Failed to build not: {:?}", e))?;
                    Ok(Some(result.into()))
                } else {
                    Err(miette::miette!("Cannot not this type"))
                }
            }
            _ => Ok(None), // TODO: Deref, AddrOf
        }
    }

    fn lower_type(&self, ty: &Type) -> Option<BasicTypeEnum<'ctx>> {
        match ty {
            Type::Primitive(prim) => match prim {
                PrimitiveType::I8 | PrimitiveType::U8 => Some(self.context.i8_type().into()),
                PrimitiveType::I16 | PrimitiveType::U16 => Some(self.context.i16_type().into()),
                PrimitiveType::I32 | PrimitiveType::U32 | PrimitiveType::Char => {
                    Some(self.context.i32_type().into())
                }
                PrimitiveType::I64 | PrimitiveType::U64 => Some(self.context.i64_type().into()),
                PrimitiveType::I128 | PrimitiveType::U128 => Some(self.context.i128_type().into()),
                PrimitiveType::Isize | PrimitiveType::Usize => {
                    Some(self.context.i64_type().into()) // Assume 64-bit
                }
                PrimitiveType::F32 => Some(self.context.f32_type().into()),
                PrimitiveType::F64 => Some(self.context.f64_type().into()),
                PrimitiveType::Bool => Some(self.context.bool_type().into()),
                PrimitiveType::Unit | PrimitiveType::Never => None,
            },
            Type::Pointer { .. } => {
                Some(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Reference { .. } => {
                Some(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Array { inner, size } => {
                let inner_ty = self.lower_type(inner)?;
                Some(inner_ty.array_type(*size as u32).into())
            }
            Type::Named { name, .. } => {
                // Look up struct type
                self.struct_types
                    .get(name)
                    .map(|info| info.llvm_type.as_basic_type_enum())
            }
            Type::Tuple(types) => {
                // Tuples are represented as LLVM structs
                let field_types: Vec<BasicTypeEnum> = types
                    .iter()
                    .filter_map(|t| self.lower_type(t))
                    .collect();
                Some(self.context.struct_type(&field_types, false).into())
            }
            _ => None,
        }
    }

    fn create_entry_alloca(
        &self,
        function: FunctionValue<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();
        let entry = function.get_first_basic_block().unwrap();

        match entry.get_first_instruction() {
            Some(instr) => builder.position_before(&instr),
            None => builder.position_at_end(entry),
        }

        builder.build_alloca(ty, name).unwrap()
    }

    fn current_block_has_terminator(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_some()
    }
}
