use fragile_common::{Symbol, SymbolInterner};
use fragile_hir::{
    Abi, BinOp, ConstDef, Expr, ExprKind, FnDef, Item, ItemKind, Literal, Module, Mutability, PrimitiveType, StaticDef, Stmt, StmtKind, StructDef, Type, TypeAlias, UnaryOp,
};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PointerValue};
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

    /// Recursively collect all items from a module, including items from nested modules.
    fn collect_all_items(items: &[Item]) -> Vec<&Item> {
        let mut result = Vec::new();
        for item in items {
            result.push(item);
            // If this is a module with items, recursively collect them
            if let ItemKind::Mod(mod_def) = &item.kind {
                if let Some(ref nested_items) = mod_def.items {
                    result.extend(Self::collect_all_items(nested_items));
                }
            }
        }
        result
    }

    pub fn compile_module(&self, module: &Module) -> Result<LlvmModule<'ctx>> {
        let name = self.interner.resolve(module.name);
        let llvm_module = self.context.create_module(&name);

        let mut compiler = ModuleCompiler::new(self.context, &llvm_module, self.interner);

        // Collect all items including from nested modules
        let all_items = Self::collect_all_items(&module.items);

        // First pass: register all struct, enum, and type alias definitions
        for item in &all_items {
            match &item.kind {
                ItemKind::Struct(struct_def) => {
                    // Skip generic structs (requires monomorphization)
                    if struct_def.type_params.is_empty() {
                        compiler.register_struct(struct_def)?;
                    }
                }
                ItemKind::Enum(enum_def) => {
                    // Skip generic enums (requires monomorphization)
                    if enum_def.type_params.is_empty() {
                        compiler.register_enum(enum_def)?;
                    } else {
                        // Store generic enum for later monomorphization
                        compiler.generic_enums.insert(enum_def.name, enum_def.clone());
                    }
                }
                ItemKind::TypeAlias(type_alias) => {
                    // Skip generic type aliases for now
                    if type_alias.type_params.is_empty() {
                        compiler.register_type_alias(type_alias);
                    }
                }
                _ => {}
            }
        }

        // Second pass: declare all functions (including methods from impl blocks)
        // Also collect generic functions for later monomorphization
        for item in &all_items {
            match &item.kind {
                ItemKind::Function(fn_def) => {
                    if fn_def.type_params.is_empty() {
                        // Non-generic function - declare directly
                        compiler.declare_function(fn_def)?;
                    } else {
                        // Generic function - store for monomorphization
                        compiler.generic_functions.insert(fn_def.name, fn_def.clone());
                    }
                }
                ItemKind::Impl(impl_def) => {
                    // Declare methods with mangled names (Type_method)
                    compiler.declare_impl_methods(impl_def)?;
                }
                _ => {}
            }
        }

        // Third pass: compile const and static items
        for item in &all_items {
            match &item.kind {
                ItemKind::Const(const_def) => {
                    compiler.compile_const(const_def)?;
                }
                ItemKind::Static(static_def) => {
                    compiler.compile_static(static_def)?;
                }
                _ => {}
            }
        }

        // Fourth pass: compile function bodies (including methods)
        for item in &all_items {
            match &item.kind {
                ItemKind::Function(fn_def) => {
                    let fn_name = compiler.interner.resolve(fn_def.name);
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

        // Verify the module first
        if let Err(e) = module.verify() {
            // Print the module for debugging
            return Err(miette::miette!("Module verification failed: {}", e.to_string()));
        }

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

/// Info about an enum: variant name -> discriminant mapping and data types
struct EnumInfo<'ctx> {
    variant_discriminants: FxHashMap<Symbol, i128>,
    /// Variant name -> list of field types (for variants with data)
    variant_fields: FxHashMap<Symbol, Vec<BasicTypeEnum<'ctx>>>,
    /// The LLVM struct type for this enum (discriminant + payload)
    llvm_type: Option<StructType<'ctx>>,
}

struct ModuleCompiler<'a, 'ctx> {
    context: &'ctx Context,
    module: &'a LlvmModule<'ctx>,
    builder: Builder<'ctx>,
    interner: &'a SymbolInterner,
    functions: FxHashMap<String, FunctionValue<'ctx>>,
    variables: FxHashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    struct_types: FxHashMap<Symbol, StructInfo<'ctx>>,
    enum_types: FxHashMap<Symbol, EnumInfo<'ctx>>,
    /// Generic function definitions for monomorphization
    generic_functions: FxHashMap<Symbol, FnDef>,
    /// Generic enum definitions for monomorphization
    generic_enums: FxHashMap<Symbol, fragile_hir::EnumDef>,
    /// Counter for generating unique closure names
    closure_counter: u32,
    /// Global constants and statics
    globals: FxHashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    /// Type aliases (name -> aliased type)
    type_aliases: FxHashMap<Symbol, Type>,
    /// Current loop context for break handling (break target block, optional value alloca)
    loop_context: Option<LoopContext<'ctx>>,
    /// Track pointee types for pointer variables (variable name -> element type)
    pointer_element_types: FxHashMap<String, BasicTypeEnum<'ctx>>,
    /// Track captured variables for each closure (closure_name -> captured_var_names)
    closure_captures: FxHashMap<String, Vec<String>>,
    /// Map variable names to closure names for indirect closure calls
    variable_to_closure: FxHashMap<String, String>,
}

/// Context for handling break statements in loops
struct LoopContext<'ctx> {
    /// Block to branch to on break
    break_block: inkwell::basic_block::BasicBlock<'ctx>,
    /// Optional alloca for storing break value
    break_value: Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
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
            generic_functions: FxHashMap::default(),
            generic_enums: FxHashMap::default(),
            closure_counter: 0,
            globals: FxHashMap::default(),
            type_aliases: FxHashMap::default(),
            loop_context: None,
            pointer_element_types: FxHashMap::default(),
            closure_captures: FxHashMap::default(),
            variable_to_closure: FxHashMap::default(),
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
        let mut variant_fields: FxHashMap<Symbol, Vec<BasicTypeEnum>> = FxHashMap::default();
        let mut has_data = false;
        let mut max_payload_size = 0usize;

        for variant in &enum_def.variants {
            if let Some(disc) = variant.discriminant {
                variant_discriminants.insert(variant.name, disc);
            }

            // Check if this variant has data fields
            if !variant.fields.is_empty() {
                has_data = true;
                let mut field_types = vec![];
                let mut payload_size = 0usize;
                for field in &variant.fields {
                    if let Some(llvm_ty) = self.lower_type(&field.ty) {
                        payload_size += self.type_size(llvm_ty);
                        field_types.push(llvm_ty);
                    }
                }
                max_payload_size = max_payload_size.max(payload_size);
                variant_fields.insert(variant.name, field_types);
            } else {
                variant_fields.insert(variant.name, vec![]);
            }
        }

        // Create LLVM struct type if any variant has data
        let llvm_type = if has_data {
            let enum_name = self.interner.resolve(enum_def.name);
            // Enum layout: { i32 discriminant, [max_payload_size x i8] payload }
            let disc_type = self.context.i32_type().as_basic_type_enum();
            let payload_type = self.context.i8_type().array_type(max_payload_size as u32).as_basic_type_enum();
            let struct_type = self.context.struct_type(&[disc_type, payload_type], false);
            // Name the struct type for debugging
            let _ = self.context.opaque_struct_type(&enum_name);
            Some(struct_type)
        } else {
            None
        };

        self.enum_types.insert(
            enum_def.name,
            EnumInfo {
                variant_discriminants,
                variant_fields,
                llvm_type,
            },
        );

        Ok(())
    }

    /// Get or create a specialized enum type (e.g., Option<i32>).
    /// Returns the Symbol for the specialized enum.
    fn get_or_create_specialized_enum(&mut self, enum_name: Symbol, type_args: &[Type]) -> Result<Symbol> {
        // Generate mangled name for specialization (e.g., Option_i32)
        let base_name = self.interner.resolve(enum_name);
        let type_args_str: Vec<String> = type_args
            .iter()
            .map(|t| self.type_to_string(t))
            .collect();
        let specialized_name = format!("{}_{}", base_name, type_args_str.join("_"));
        let specialized_sym = self.interner.intern(&specialized_name);

        // Check if already specialized
        if self.enum_types.contains_key(&specialized_sym) {
            return Ok(specialized_sym);
        }

        // Get the generic enum definition
        let generic_enum = self.generic_enums.get(&enum_name).cloned()
            .ok_or_else(|| miette::miette!("Generic enum '{}' not found", base_name))?;

        // Build type parameter substitution map
        let mut type_subst: FxHashMap<Symbol, Type> = FxHashMap::default();
        for (i, param) in generic_enum.type_params.iter().enumerate() {
            if i < type_args.len() {
                type_subst.insert(param.name, type_args[i].clone());
            } else {
                // Use i32 as default for unspecified type parameters
                type_subst.insert(param.name, Type::Primitive(PrimitiveType::I32));
            }
        }

        // Create specialized variants by substituting type parameters
        let mut variant_discriminants = FxHashMap::default();
        let mut variant_fields: FxHashMap<Symbol, Vec<BasicTypeEnum>> = FxHashMap::default();
        let mut has_data = false;
        let mut max_payload_size = 0usize;

        for variant in &generic_enum.variants {
            if let Some(disc) = variant.discriminant {
                variant_discriminants.insert(variant.name, disc);
            }

            if !variant.fields.is_empty() {
                has_data = true;
                let mut field_types = vec![];
                let mut payload_size = 0usize;
                for field in &variant.fields {
                    // Substitute type parameters in field type
                    let substituted_ty = self.substitute_type(&field.ty, &type_subst);
                    if let Some(llvm_ty) = self.lower_type(&substituted_ty) {
                        payload_size += self.type_size(llvm_ty);
                        field_types.push(llvm_ty);
                    }
                }
                max_payload_size = max_payload_size.max(payload_size);
                variant_fields.insert(variant.name, field_types);
            } else {
                variant_fields.insert(variant.name, vec![]);
            }
        }

        // Create LLVM struct type if any variant has data
        let llvm_type = if has_data {
            let disc_type = self.context.i32_type().as_basic_type_enum();
            let payload_type = self.context.i8_type().array_type(max_payload_size as u32).as_basic_type_enum();
            let struct_type = self.context.struct_type(&[disc_type, payload_type], false);
            Some(struct_type)
        } else {
            None
        };

        self.enum_types.insert(
            specialized_sym,
            EnumInfo {
                variant_discriminants,
                variant_fields,
                llvm_type,
            },
        );

        Ok(specialized_sym)
    }

    fn type_size(&self, ty: BasicTypeEnum) -> usize {
        match ty {
            BasicTypeEnum::IntType(t) => (t.get_bit_width() / 8) as usize,
            BasicTypeEnum::FloatType(_) => 4,
            BasicTypeEnum::PointerType(_) => 8,
            BasicTypeEnum::ArrayType(t) => t.len() as usize * self.type_size(t.get_element_type()),
            BasicTypeEnum::StructType(t) => {
                t.get_field_types().iter().map(|f| self.type_size(*f)).sum()
            }
            BasicTypeEnum::VectorType(_) => 16, // Assume 128-bit vectors
            BasicTypeEnum::ScalableVectorType(_) => 16, // Assume 128-bit scalable vectors
        }
    }

    fn register_type_alias(&mut self, type_alias: &TypeAlias) {
        // Store the aliased type for resolution
        self.type_aliases.insert(type_alias.name, type_alias.ty.clone());
    }

    fn compile_const(&mut self, const_def: &ConstDef) -> Result<()> {
        let name = self.interner.resolve(const_def.name);
        let ty = self.lower_type(&const_def.ty)
            .ok_or_else(|| miette::miette!("Cannot lower type for const '{}'", name))?;

        // Evaluate the constant expression to get an initial value
        let init_value = self.compile_const_expr(&const_def.value, ty)?;

        // Create a global constant
        let global = self.module.add_global(ty, None, &name);
        global.set_initializer(&init_value);
        global.set_constant(true);

        // Store in globals map for later reference
        self.globals.insert(name.to_string(), (global.as_pointer_value(), ty));

        Ok(())
    }

    fn compile_static(&mut self, static_def: &StaticDef) -> Result<()> {
        let name = self.interner.resolve(static_def.name);
        let ty = self.lower_type(&static_def.ty)
            .ok_or_else(|| miette::miette!("Cannot lower type for static '{}'", name))?;

        // Get initializer value (or default to zero)
        let init_value = if let Some(ref init) = static_def.init {
            self.compile_const_expr(init, ty)?
        } else {
            // Default to zero-initialized
            ty.const_zero()
        };

        // Create a global variable
        let global = self.module.add_global(ty, None, &name);
        global.set_initializer(&init_value);

        // Static mut is not constant, static (without mut) could be constant
        // but for simplicity, we treat all statics as mutable globals
        global.set_constant(static_def.mutability == Mutability::Immutable);

        // Store in globals map for later reference
        self.globals.insert(name.to_string(), (global.as_pointer_value(), ty));

        Ok(())
    }

    /// Compile a constant expression (literals and simple integer operations)
    fn compile_const_expr(&self, expr: &Expr, ty: BasicTypeEnum<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        match &expr.kind {
            ExprKind::Literal(lit) => {
                match lit {
                    Literal::Int(val) => {
                        let int_ty = ty.into_int_type();
                        Ok(int_ty.const_int(*val as u64, true).into())
                    }
                    Literal::Float(val) => {
                        let float_ty = ty.into_float_type();
                        Ok(float_ty.const_float(*val).into())
                    }
                    Literal::Bool(val) => {
                        let bool_ty = self.context.bool_type();
                        Ok(bool_ty.const_int(*val as u64, false).into())
                    }
                    Literal::Char(val) => {
                        let char_ty = self.context.i32_type();
                        Ok(char_ty.const_int(*val as u64, false).into())
                    }
                    Literal::String(s) => {
                        // Create a global string constant
                        let string_val = self.context.const_string(s.as_bytes(), true);
                        Ok(string_val.into())
                    }
                    Literal::Unit => {
                        // Unit type - shouldn't be used in const context typically
                        Err(miette::miette!("Unit literal not supported in const context"))
                    }
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                // Support simple constant integer binary expressions
                if ty.is_int_type() {
                    let left_val = self.compile_const_expr(lhs, ty)?;
                    let right_val = self.compile_const_expr(rhs, ty)?;
                    let left_int = left_val.into_int_value();
                    let right_int = right_val.into_int_value();
                    let result = match op {
                        BinOp::Add => left_int.const_add(right_int),
                        BinOp::Sub => left_int.const_sub(right_int),
                        BinOp::Mul => left_int.const_mul(right_int),
                        _ => return Err(miette::miette!("Unsupported binary op in const expr")),
                    };
                    Ok(result.into())
                } else {
                    Err(miette::miette!("Only integer binary ops supported in const expr"))
                }
            }
            ExprKind::Unary { op, operand } => {
                let operand_val = self.compile_const_expr(operand, ty)?;
                if ty.is_int_type() {
                    let int_val = operand_val.into_int_value();
                    let result = match op {
                        fragile_hir::UnaryOp::Neg => int_val.const_neg(),
                        fragile_hir::UnaryOp::Not => int_val.const_not(),
                        _ => return Err(miette::miette!("Unsupported unary op in const expr")),
                    };
                    Ok(result.into())
                } else {
                    Err(miette::miette!("Only integer unary ops supported in const expr"))
                }
            }
            _ => Err(miette::miette!("Unsupported expression in const context")),
        }
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

        // Skip generic functions (requires monomorphization)
        if !fn_def.type_params.is_empty() {
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
            let has_term = self.current_block_has_terminator();
            if !has_term {
                if fn_def.sig.ret_ty == Type::unit() {
                    self.builder.build_return(None)
                        .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                } else if let Some(val) = result {
                    // Cast return value if needed (e.g., i64 literal to i32 return type)
                    let ret_llvm_ty = self.lower_type(&fn_def.sig.ret_ty);
                    let return_val = if let Some(ret_ty) = ret_llvm_ty {
                        if val.get_type() != ret_ty && val.is_int_value() && ret_ty.is_int_type() {
                            self.builder.build_int_cast(
                                val.into_int_value(),
                                ret_ty.into_int_type(),
                                "ret_cast"
                            ).map_err(|e| miette::miette!("Failed to cast return: {:?}", e))?.into()
                        } else {
                            val
                        }
                    } else {
                        val
                    };
                    self.builder.build_return(Some(&return_val))
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

                    // Add return if needed (check if block already has a terminator)
                    let has_term = self.current_block_has_terminator();
                    if !has_term {
                        if fn_def.sig.ret_ty == Type::unit() {
                            self.builder.build_return(None)
                                .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                        } else if let Some(val) = result {
                            // Cast return value if needed (e.g., i64 literal to i32 return type)
                            let ret_llvm_ty = self.lower_type(&fn_def.sig.ret_ty);
                            let return_val = if let Some(ret_ty) = ret_llvm_ty {
                                if val.get_type() != ret_ty && val.is_int_value() && ret_ty.is_int_type() {
                                    self.builder.build_int_cast(
                                        val.into_int_value(),
                                        ret_ty.into_int_type(),
                                        "ret_cast"
                                    ).map_err(|e| miette::miette!("Failed to cast return: {:?}", e))?.into()
                                } else {
                                    val
                                }
                            } else {
                                val
                            };
                            self.builder.build_return(Some(&return_val))
                                .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                        } else {
                            self.builder.build_return(None)
                                .map_err(|e| miette::miette!("Failed to build return: {:?}", e))?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect free variables in an expression (variables used but not in bound set)
    fn collect_free_vars(&self, expr: &Expr, bound: &mut std::collections::HashSet<String>) -> Vec<String> {
        let mut free_vars = Vec::new();
        self.collect_free_vars_inner(expr, bound, &mut free_vars);
        // Deduplicate
        free_vars.sort();
        free_vars.dedup();
        free_vars
    }

    fn collect_free_vars_inner(
        &self,
        expr: &Expr,
        bound: &mut std::collections::HashSet<String>,
        free: &mut Vec<String>,
    ) {
        match &expr.kind {
            ExprKind::Ident(sym) => {
                let name = self.interner.resolve(*sym).to_string();
                if !bound.contains(&name) {
                    free.push(name);
                }
            }
            ExprKind::Binary { lhs, rhs, .. } => {
                self.collect_free_vars_inner(lhs, bound, free);
                self.collect_free_vars_inner(rhs, bound, free);
            }
            ExprKind::Unary { operand, .. } => {
                self.collect_free_vars_inner(operand, bound, free);
            }
            ExprKind::Call { callee, args } => {
                self.collect_free_vars_inner(callee, bound, free);
                for arg in args {
                    self.collect_free_vars_inner(arg, bound, free);
                }
            }
            ExprKind::If { cond, then_branch, else_branch } => {
                self.collect_free_vars_inner(cond, bound, free);
                self.collect_free_vars_inner(then_branch, bound, free);
                if let Some(else_br) = else_branch {
                    self.collect_free_vars_inner(else_br, bound, free);
                }
            }
            ExprKind::Block { stmts, expr: final_expr } => {
                // Block introduces a new scope - variables defined here are bound
                let mut inner_bound = bound.clone();
                for stmt in stmts {
                    if let StmtKind::Let { pattern, init, .. } = &stmt.kind {
                        if let Some(init_expr) = init {
                            self.collect_free_vars_inner(init_expr, &mut inner_bound, free);
                        }
                        // Add bound variable
                        if let fragile_hir::Pattern::Ident(sym) = pattern {
                            let name = self.interner.resolve(*sym).to_string();
                            inner_bound.insert(name);
                        }
                    }
                }
                if let Some(fe) = final_expr {
                    self.collect_free_vars_inner(fe, &mut inner_bound, free);
                }
            }
            ExprKind::Lambda { params, body } => {
                // Lambda parameters are bound in the body
                let mut inner_bound = bound.clone();
                for (param_sym, _) in params {
                    let name = self.interner.resolve(*param_sym).to_string();
                    inner_bound.insert(name);
                }
                self.collect_free_vars_inner(body, &mut inner_bound, free);
            }
            ExprKind::Assign { lhs, rhs } => {
                self.collect_free_vars_inner(lhs, bound, free);
                self.collect_free_vars_inner(rhs, bound, free);
            }
            ExprKind::Field { expr: e, .. } => {
                self.collect_free_vars_inner(e, bound, free);
            }
            ExprKind::Index { expr: e, index } => {
                self.collect_free_vars_inner(e, bound, free);
                self.collect_free_vars_inner(index, bound, free);
            }
            ExprKind::Cast { expr: e, .. } => {
                self.collect_free_vars_inner(e, bound, free);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect_free_vars_inner(scrutinee, bound, free);
                for arm in arms {
                    self.collect_free_vars_inner(&arm.body, bound, free);
                }
            }
            ExprKind::Loop { body } => {
                self.collect_free_vars_inner(body, bound, free);
            }
            ExprKind::While { cond, body } => {
                self.collect_free_vars_inner(cond, bound, free);
                self.collect_free_vars_inner(body, bound, free);
            }
            ExprKind::Return(Some(e)) | ExprKind::Break(Some(e)) => {
                self.collect_free_vars_inner(e, bound, free);
            }
            ExprKind::Array(elems) => {
                for elem in elems {
                    self.collect_free_vars_inner(elem, bound, free);
                }
            }
            ExprKind::Tuple(elems) => {
                for elem in elems {
                    self.collect_free_vars_inner(elem, bound, free);
                }
            }
            ExprKind::Struct { fields, .. } => {
                for (_, field_expr) in fields {
                    self.collect_free_vars_inner(field_expr, bound, free);
                }
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                self.collect_free_vars_inner(receiver, bound, free);
                for arg in args {
                    self.collect_free_vars_inner(arg, bound, free);
                }
            }
            // Literals and other leaf nodes have no free variables
            _ => {}
        }
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

                // Check if operands are struct types - try operator overloading
                if lhs_val.is_struct_value() {
                    // Find the struct type name
                    let struct_type = lhs_val.into_struct_value().get_type();
                    let type_name = self.find_struct_name_by_llvm_type(struct_type);

                    if let Some(type_name) = type_name {
                        // Map BinOp to method name
                        let method_name = match op {
                            BinOp::Add => "add",
                            BinOp::Sub => "sub",
                            BinOp::Mul => "mul",
                            BinOp::Div => "div",
                            BinOp::Rem => "rem",
                            BinOp::Eq => "eq",
                            BinOp::Ne => "ne",
                            BinOp::Lt => "lt",
                            BinOp::Le => "le",
                            BinOp::Gt => "gt",
                            BinOp::Ge => "ge",
                            _ => return Err(miette::miette!("Unsupported operator for overloading")),
                        };

                        let mangled_name = format!("{}_{}", type_name, method_name);

                        // Look for the operator method
                        if let Some(&func) = self.functions.get(&mangled_name) {
                            // Reload values since we moved them checking is_struct_value
                            let lhs_val = self.compile_expr(lhs, function)?
                                .ok_or_else(|| miette::miette!("Binary lhs has no value"))?;
                            let rhs_val = self.compile_expr(rhs, function)?
                                .ok_or_else(|| miette::miette!("Binary rhs has no value"))?;

                            let args: Vec<BasicMetadataValueEnum> = vec![lhs_val.into(), rhs_val.into()];
                            let call = self.builder.build_call(func, &args, "op_call")
                                .map_err(|e| miette::miette!("Failed to call operator: {:?}", e))?;
                            return match call.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(v) => Ok(Some(v)),
                                inkwell::values::ValueKind::Instruction(_) => Ok(None),
                            };
                        }
                    }

                    // Fall through to error - no operator found for struct type
                    return Err(miette::miette!("No operator method found for struct type"));
                }

                self.compile_binary_op(*op, lhs_val, rhs_val)
            }

            ExprKind::Unary { op, operand } => {
                // Special handling for address-of operators - need the address, not the value
                match op {
                    UnaryOp::AddrOf | UnaryOp::AddrOfMut => {
                        // If taking address of an identifier, return its alloca directly
                        if let ExprKind::Ident(sym) = &operand.kind {
                            let name = self.interner.resolve(*sym);
                            if let Some(&(ptr, _ty)) = self.variables.get(name.as_str()) {
                                return Ok(Some(ptr.into()));
                            }
                        }
                        // Otherwise fall through to normal handling (creates temp)
                    }
                    UnaryOp::Deref => {
                        // If dereferencing an identifier, look up its pointee type
                        if let ExprKind::Ident(sym) = &operand.kind {
                            let name = self.interner.resolve(*sym);
                            // Look up the pointee type
                            if let Some(&pointee_ty) = self.pointer_element_types.get(name.as_str()) {
                                // Load the pointer from the variable
                                if let Some(&(alloca, ptr_ty)) = self.variables.get(name.as_str()) {
                                    let ptr = self.builder.build_load(ptr_ty, alloca, &format!("{}_load", name))
                                        .map_err(|e| miette::miette!("Failed to load pointer: {:?}", e))?
                                        .into_pointer_value();
                                    let result = self.builder.build_load(pointee_ty, ptr, "deref")
                                        .map_err(|e| miette::miette!("Failed to build deref load: {:?}", e))?;
                                    return Ok(Some(result));
                                }
                            }
                        }
                        // Fall through to general handling
                    }
                    _ => {}
                }

                let val = self
                    .compile_expr(operand, function)?
                    .ok_or_else(|| miette::miette!("Unary operand has no value"))?;

                self.compile_unary_op(*op, val)
            }

            ExprKind::Call { callee, args } => {
                // Compile arguments first to get their types for generic inference
                let mut arg_vals: Vec<BasicMetadataValueEnum> = vec![];
                let mut arg_types: Vec<Type> = vec![];

                for arg in args {
                    if let Some(val) = self.compile_expr(arg, function)? {
                        arg_types.push(self.infer_type_from_value(val));
                        arg_vals.push(val.into());
                    }
                }

                // Get function to call
                if let ExprKind::Ident(sym) = &callee.kind {
                    let name = self.interner.resolve(*sym);

                    // First check if it's a regular (non-generic) function
                    if let Some(&func) = self.functions.get(name.as_str()) {
                        // Cast arguments to match function parameter types
                        let param_types = func.get_type().get_param_types();
                        let casted_args: Vec<BasicMetadataValueEnum> = arg_vals
                            .into_iter()
                            .zip(param_types.iter())
                            .map(|(arg, param_ty)| {
                                let arg_val: BasicValueEnum = arg.try_into().unwrap();
                                if arg_val.is_int_value() && param_ty.is_int_type() {
                                    let arg_int = arg_val.into_int_value();
                                    let param_int = param_ty.into_int_type();
                                    if arg_int.get_type() != param_int {
                                        self.builder.build_int_cast(arg_int, param_int, "arg_cast")
                                            .map(|v| v.as_basic_value_enum().into())
                                            .unwrap_or(arg_val.into())
                                    } else {
                                        arg_val.into()
                                    }
                                } else {
                                    arg_val.into()
                                }
                            })
                            .collect();

                        let call = self.builder.build_call(func, &casted_args, "call")
                            .map_err(|e| miette::miette!("Failed to build call: {:?}", e))?;
                        let value = match call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => Some(v),
                            inkwell::values::ValueKind::Instruction(_) => None,
                        };
                        return Ok(value);
                    }

                    // Check if it's a closure stored in a variable (with captures)
                    if let Some(closure_name) = self.variable_to_closure.get(name.as_str()).cloned() {
                        if let Some(&closure_fn) = self.functions.get(&closure_name) {
                            // Get captured variable names for this closure
                            let capture_names = self.closure_captures.get(&closure_name).cloned().unwrap_or_default();

                            // Build arguments: regular args + captured values
                            let mut all_args = arg_vals.clone();

                            // Load captured values from current scope and add as extra args
                            for cap_name in &capture_names {
                                if let Some(&(cap_ptr, cap_ty)) = self.variables.get(cap_name) {
                                    let cap_val = self.builder.build_load(cap_ty, cap_ptr, &format!("cap_{}", cap_name))
                                        .map_err(|e| miette::miette!("Failed to load capture: {:?}", e))?;
                                    all_args.push(cap_val.into());
                                }
                            }

                            // Cast arguments to match function parameter types
                            let param_types = closure_fn.get_type().get_param_types();
                            let casted_args: Vec<BasicMetadataValueEnum> = all_args
                                .into_iter()
                                .zip(param_types.iter())
                                .map(|(arg, param_ty)| {
                                    let arg_val: BasicValueEnum = arg.try_into().unwrap();
                                    if arg_val.is_int_value() && param_ty.is_int_type() {
                                        let arg_int = arg_val.into_int_value();
                                        let param_int = param_ty.into_int_type();
                                        if arg_int.get_type() != param_int {
                                            self.builder.build_int_cast(arg_int, param_int, "arg_cast")
                                                .map(|v| v.as_basic_value_enum().into())
                                                .unwrap_or(arg_val.into())
                                        } else {
                                            arg_val.into()
                                        }
                                    } else {
                                        arg_val.into()
                                    }
                                })
                                .collect();

                            let call = self.builder.build_call(closure_fn, &casted_args, "closure_call")
                                .map_err(|e| miette::miette!("Failed to build closure call: {:?}", e))?;
                            let value = match call.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(v) => Some(v),
                                inkwell::values::ValueKind::Instruction(_) => None,
                            };
                            return Ok(value);
                        }
                    }

                    // Check if it's a generic function that needs monomorphization
                    if self.generic_functions.contains_key(sym) {
                        let spec_fn = self.get_or_create_specialization(*sym, &arg_types, function)?;
                        let call = self.builder.build_call(spec_fn, &arg_vals, "call")
                            .map_err(|e| miette::miette!("Failed to build call: {:?}", e))?;
                        let value = match call.try_as_basic_value() {
                            inkwell::values::ValueKind::Basic(v) => Some(v),
                            inkwell::values::ValueKind::Instruction(_) => None,
                        };
                        return Ok(value);
                    }
                }

                // Check if callee is an enum variant with data (e.g., Option::Some(36))
                // or a module path function call (e.g., math::add)
                if let ExprKind::EnumVariant { enum_name, variant } = &callee.kind {
                    // First check if this is a non-generic enum variant
                    let resolved_enum_name = if self.enum_types.contains_key(enum_name) {
                        *enum_name
                    } else if self.generic_enums.contains_key(enum_name) {
                        // Generic enum - infer type arguments from argument types
                        let type_args: Vec<Type> = arg_types.clone();
                        if !type_args.is_empty() {
                            self.get_or_create_specialized_enum(*enum_name, &type_args)?
                        } else {
                            // No arguments - treat as unit variant, infer as i32 default
                            *enum_name
                        }
                    } else {
                        // Not an enum - will be handled as module path below
                        *enum_name
                    };

                    if let Some(enum_info) = self.enum_types.get(&resolved_enum_name) {
                        let discriminant = *enum_info.variant_discriminants.get(variant).ok_or_else(|| {
                            let enum_str = self.interner.resolve(*enum_name);
                            let var_str = self.interner.resolve(*variant);
                            miette::miette!("Unknown variant {}::{}", enum_str, var_str)
                        })?;

                        // Check if this enum has a struct type (has data variants)
                        if let Some(enum_type) = enum_info.llvm_type {
                            // Create an undef value of the enum struct type
                            let mut enum_val = enum_type.get_undef();

                            // Store the discriminant in field 0
                            let disc_val = self.context.i32_type().const_int(discriminant as u64, false);
                            enum_val = self.builder.build_insert_value(enum_val, disc_val, 0, "disc")
                                .map_err(|e| miette::miette!("Failed to insert discriminant: {:?}", e))?
                                .into_struct_value();

                            // Store the data in the payload field
                            // For now, we only handle single-field variants
                            if !arg_vals.is_empty() {
                                let arg_val = arg_vals[0].into_int_value();
                                // Create a pointer to the payload field
                                let alloca = self.builder.build_alloca(enum_type, "enum_tmp")
                                    .map_err(|e| miette::miette!("Failed to alloc enum: {:?}", e))?;
                                self.builder.build_store(alloca, enum_val)
                                    .map_err(|e| miette::miette!("Failed to store enum: {:?}", e))?;

                                // GEP to the payload field
                                let payload_ptr = self.builder.build_struct_gep(enum_type, alloca, 1, "payload_ptr")
                                    .map_err(|e| miette::miette!("Failed to GEP payload: {:?}", e))?;

                                // Cast payload ptr to pointer to the data type and store
                                let data_ty = arg_val.get_type();
                                let data_ptr = self.builder.build_pointer_cast(
                                    payload_ptr,
                                    data_ty.ptr_type(inkwell::AddressSpace::default()),
                                    "data_ptr"
                                ).map_err(|e| miette::miette!("Failed to cast ptr: {:?}", e))?;

                                self.builder.build_store(data_ptr, arg_val)
                                    .map_err(|e| miette::miette!("Failed to store data: {:?}", e))?;

                                // Load the complete enum value
                                let result = self.builder.build_load(enum_type, alloca, "enum_val")
                                    .map_err(|e| miette::miette!("Failed to load enum: {:?}", e))?;
                                return Ok(Some(result));
                            }

                            return Ok(Some(enum_val.as_basic_value_enum()));
                        } else {
                            // Unit variant (no data) - just return discriminant
                            let disc_val = self.context.i32_type().const_int(discriminant as u64, false);
                            return Ok(Some(disc_val.as_basic_value_enum()));
                        }
                    } else {
                        // Not an enum - check if it's a struct's associated function (e.g., Point::new)
                        // or a module path function call (e.g., math::add)
                        let enum_name_str = self.interner.resolve(*enum_name);
                        let variant_str = self.interner.resolve(*variant);

                        // Check if enum_name is actually a struct (associated function call)
                        let fn_name = if self.struct_types.contains_key(enum_name) {
                            format!("{}_{}", enum_name_str, variant_str)
                        } else {
                            // Module path function call
                            variant_str.to_string()
                        };

                        if let Some(&func) = self.functions.get(&fn_name) {
                            // Cast arguments to match function parameter types
                            let param_types = func.get_type().get_param_types();
                            let casted_args: Vec<BasicMetadataValueEnum> = arg_vals
                                .into_iter()
                                .zip(param_types.iter())
                                .map(|(arg, param_ty)| {
                                    let arg_val: BasicValueEnum = arg.try_into().unwrap();
                                    if arg_val.is_int_value() && param_ty.is_int_type() {
                                        let arg_int = arg_val.into_int_value();
                                        let param_int = param_ty.into_int_type();
                                        if arg_int.get_type() != param_int {
                                            self.builder.build_int_cast(arg_int, param_int, "arg_cast")
                                                .map(|v| v.as_basic_value_enum().into())
                                                .unwrap_or(arg_val.into())
                                        } else {
                                            arg_val.into()
                                        }
                                    } else {
                                        arg_val.into()
                                    }
                                })
                                .collect();

                            let call = self.builder.build_call(func, &casted_args, "call")
                                .map_err(|e| miette::miette!("Failed to build call: {:?}", e))?;
                            let value = match call.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(v) => Some(v),
                                inkwell::values::ValueKind::Instruction(_) => None,
                            };
                            return Ok(value);
                        }
                    }
                }

                // Handle Field callee (C++ method calls like p.get_x())
                if let ExprKind::Field { expr: receiver, field } = &callee.kind {
                    // Compile receiver (the object we're calling the method on)
                    let receiver_val = self
                        .compile_expr(receiver, function)?
                        .ok_or_else(|| miette::miette!("Method receiver has no value"))?;

                    // Get field name for method lookup
                    let method_name = self.interner.resolve(*field);

                    // Try to determine the type name from the receiver
                    let type_name = if let ExprKind::Ident(sym) = &receiver.kind {
                        let var_name = self.interner.resolve(*sym);
                        if let Some(&(_ptr, ty)) = self.variables.get(var_name.as_str()) {
                            if ty.is_struct_type() {
                                self.find_struct_name_by_llvm_type(ty.into_struct_type())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else if receiver_val.is_struct_value() {
                        self.find_struct_name_by_llvm_type(receiver_val.into_struct_value().get_type())
                    } else {
                        None
                    };

                    if let Some(type_name) = type_name {
                        let mangled_name = format!("{}_{}", type_name, method_name);

                        if let Some(&func) = self.functions.get(&mangled_name) {
                            // Build args with receiver as first arg
                            let mut call_args: Vec<BasicMetadataValueEnum> = vec![];

                            // Get receiver's alloca if it's a variable
                            if let ExprKind::Ident(sym) = &receiver.kind {
                                let var_name = self.interner.resolve(*sym);
                                if let Some(&(ptr, _)) = self.variables.get(var_name.as_str()) {
                                    call_args.push(ptr.into());
                                }
                            }

                            // Add the other arguments
                            call_args.extend(arg_vals);

                            // Cast args to match function parameter types
                            let param_types = func.get_type().get_param_types();
                            let casted_args: Vec<BasicMetadataValueEnum> = call_args
                                .into_iter()
                                .zip(param_types.iter())
                                .map(|(arg, param_ty)| {
                                    let arg_val: BasicValueEnum = arg.try_into().unwrap();
                                    if arg_val.is_int_value() && param_ty.is_int_type() {
                                        let arg_int = arg_val.into_int_value();
                                        let param_int = param_ty.into_int_type();
                                        if arg_int.get_type() != param_int {
                                            self.builder.build_int_cast(arg_int, param_int, "arg_cast")
                                                .map(|v| v.as_basic_value_enum().into())
                                                .unwrap_or(arg_val.into())
                                        } else {
                                            arg_val.into()
                                        }
                                    } else {
                                        arg_val.into()
                                    }
                                })
                                .collect();

                            let call = self.builder.build_call(func, &casted_args, "method_call")
                                .map_err(|e| miette::miette!("Failed to build method call: {:?}", e))?;
                            let value = match call.try_as_basic_value() {
                                inkwell::values::ValueKind::Basic(v) => Some(v),
                                inkwell::values::ValueKind::Instruction(_) => None,
                            };
                            return Ok(value);
                        }
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

                // Create alloca for break value (default to i64)
                let break_value_ty = self.context.i64_type().into();
                let break_value_alloca = self.create_entry_alloca(function, "__loop_break_val", break_value_ty);

                // Save old loop context and set new one
                let old_context = self.loop_context.take();
                self.loop_context = Some(LoopContext {
                    break_block: end_bb,
                    break_value: Some((break_value_alloca, break_value_ty)),
                });

                self.builder.build_unconditional_branch(body_bb)
                    .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;

                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if !self.current_block_has_terminator() {
                    self.builder.build_unconditional_branch(body_bb)
                        .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                }

                // Restore old loop context
                self.loop_context = old_context;

                self.builder.position_at_end(end_bb);

                // Load and return the break value
                let result = self.builder.build_load(break_value_ty, break_value_alloca, "loop_result")
                    .map_err(|e| miette::miette!("Failed to load loop result: {:?}", e))?;
                Ok(Some(result))
            }

            ExprKind::Return(value) => {
                if let Some(val_expr) = value {
                    let val = self.compile_expr(val_expr, function)?;
                    if let Some(v) = val {
                        // Cast return value to match function return type if needed
                        let ret_type = function.get_type().get_return_type();
                        let cast_val = if let Some(expected_ty) = ret_type {
                            if v.get_type() != expected_ty {
                                // Cast integer types
                                if v.is_int_value() && expected_ty.is_int_type() {
                                    self.builder.build_int_cast(
                                        v.into_int_value(),
                                        expected_ty.into_int_type(),
                                        "ret_cast"
                                    ).map_err(|e| miette::miette!("Failed to cast return: {:?}", e))?.into()
                                } else {
                                    v
                                }
                            } else {
                                v
                            }
                        } else {
                            v
                        };
                        self.builder.build_return(Some(&cast_val))
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

            ExprKind::Break(value) => {
                // Extract loop context data before mutable borrow
                let (break_block, break_value_ptr) = {
                    let loop_ctx = self.loop_context.as_ref()
                        .ok_or_else(|| miette::miette!("Break outside of loop"))?;
                    (loop_ctx.break_block, loop_ctx.break_value.map(|(ptr, _)| ptr))
                };

                // If there's a break value, store it
                if let Some(val_expr) = value {
                    let val = self.compile_expr(val_expr, function)?
                        .ok_or_else(|| miette::miette!("Break value has no value"))?;
                    if let Some(ptr) = break_value_ptr {
                        self.builder.build_store(ptr, val)
                            .map_err(|e| miette::miette!("Failed to store break value: {:?}", e))?;
                    }
                }

                // Branch to the loop's end block
                self.builder.build_unconditional_branch(break_block)
                    .map_err(|e| miette::miette!("Failed to build break branch: {:?}", e))?;

                Ok(None)
            }

            ExprKind::Continue => {
                // For continue, we'd need to track the loop header block
                // For now, just return an error
                Err(miette::miette!("Continue not yet implemented"))
            }

            ExprKind::Cast { expr, ty } => {
                let val = self.compile_expr(expr, function)?
                    .ok_or_else(|| miette::miette!("Cast value has no value"))?;
                let target_type = self.lower_type(ty)
                    .ok_or_else(|| miette::miette!("Cannot lower cast target type"))?;

                // Handle different cast types
                if val.is_int_value() {
                    let int_val = val.into_int_value();
                    if target_type.is_int_type() {
                        // Int to int cast
                        let target_int = target_type.into_int_type();
                        let result = self.builder.build_int_cast(int_val, target_int, "cast")
                            .map_err(|e| miette::miette!("Failed to build int cast: {:?}", e))?;
                        Ok(Some(result.into()))
                    } else if target_type.is_pointer_type() {
                        // Int to pointer cast
                        let result = self.builder.build_int_to_ptr(int_val, target_type.into_pointer_type(), "inttoptr")
                            .map_err(|e| miette::miette!("Failed to build int to ptr: {:?}", e))?;
                        Ok(Some(result.into()))
                    } else {
                        Err(miette::miette!("Unsupported cast from int"))
                    }
                } else if val.is_pointer_value() {
                    let ptr_val = val.into_pointer_value();
                    if target_type.is_pointer_type() {
                        // Pointer to pointer cast - just bitcast
                        let result = self.builder.build_pointer_cast(ptr_val, target_type.into_pointer_type(), "ptrcast")
                            .map_err(|e| miette::miette!("Failed to build pointer cast: {:?}", e))?;
                        Ok(Some(result.into()))
                    } else if target_type.is_int_type() {
                        // Pointer to int cast
                        let result = self.builder.build_ptr_to_int(ptr_val, target_type.into_int_type(), "ptrtoint")
                            .map_err(|e| miette::miette!("Failed to build ptr to int: {:?}", e))?;
                        Ok(Some(result.into()))
                    } else {
                        Err(miette::miette!("Unsupported cast from pointer"))
                    }
                } else {
                    Err(miette::miette!("Unsupported cast source type"))
                }
            }

            ExprKind::Assign { lhs, rhs } => {
                let rhs_val = self
                    .compile_expr(rhs, function)?
                    .ok_or_else(|| miette::miette!("Assignment rhs has no value"))?;

                match &lhs.kind {
                    ExprKind::Ident(sym) => {
                        let name = self.interner.resolve(*sym);
                        if let Some(&(ptr, _ty)) = self.variables.get(name.as_str()) {
                            self.builder.build_store(ptr, rhs_val)
                                .map_err(|e| miette::miette!("Failed to store: {:?}", e))?;
                            return Ok(Some(rhs_val));
                        }
                    }
                    ExprKind::Unary { op: UnaryOp::Deref, operand } => {
                        // *ptr = value - store through pointer
                        let ptr_val = self.compile_expr(operand, function)?
                            .ok_or_else(|| miette::miette!("Deref target has no value"))?;
                        if ptr_val.is_pointer_value() {
                            self.builder.build_store(ptr_val.into_pointer_value(), rhs_val)
                                .map_err(|e| miette::miette!("Failed to store through deref: {:?}", e))?;
                            return Ok(Some(rhs_val));
                        }
                    }
                    ExprKind::Field { expr, field } => {
                        // struct.field = value - store to field
                        if let ExprKind::Ident(sym) = &expr.kind {
                            let name = self.interner.resolve(*sym);
                            if let Some(&(ptr, ty)) = self.variables.get(name.as_str()) {
                                if ty.is_struct_type() {
                                    let field_name = self.interner.resolve(*field);
                                    // Find field index - need struct info
                                    // For now, just handle through load/modify/store
                                    // This is a simplified approach
                                }
                            }
                        }
                    }
                    _ => {}
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

            ExprKind::Array(elements) => {
                // Compile each element
                let mut values: Vec<BasicValueEnum> = vec![];
                for elem in elements {
                    if let Some(val) = self.compile_expr(elem, function)? {
                        values.push(val);
                    }
                }

                if values.is_empty() {
                    return Ok(None);
                }

                // Get element type from first element
                let elem_type = values[0].get_type();
                let array_type = elem_type.array_type(values.len() as u32);

                // Build the array value using insert_value
                let mut array_val = array_type.get_undef();
                for (i, val) in values.into_iter().enumerate() {
                    array_val = self.builder
                        .build_insert_value(array_val, val, i as u32, "arr_elem")
                        .map_err(|e| miette::miette!("Failed to insert array element: {:?}", e))?
                        .into_array_value();
                }

                Ok(Some(array_val.into()))
            }

            ExprKind::Index { expr, index } => {
                // For array indexing, get the array variable pointer and type
                let (array_ptr, array_ty) = if let ExprKind::Ident(sym) = &expr.kind {
                    let name = self.interner.resolve(*sym);
                    if let Some(&(ptr, ty)) = self.variables.get(name.as_str()) {
                        if ty.is_array_type() {
                            (ptr, ty.into_array_type())
                        } else {
                            return Err(miette::miette!("Cannot index non-array variable"));
                        }
                    } else {
                        return Err(miette::miette!("Unknown variable for indexing"));
                    }
                } else {
                    return Err(miette::miette!("Index expression requires identifier"));
                };

                // Compile the index
                let index_val = self.compile_expr(index, function)?
                    .ok_or_else(|| miette::miette!("Index expression has no index value"))?;
                let index_int = index_val.into_int_value();

                // Use GEP to get element pointer
                let indices = [
                    self.context.i32_type().const_int(0, false),
                    index_int,
                ];
                let elem_ptr = unsafe {
                    self.builder.build_gep(array_ty, array_ptr, &indices, "index_ptr")
                        .map_err(|e| miette::miette!("Failed to build gep for index: {:?}", e))?
                };

                // Load the element
                let elem_ty = array_ty.get_element_type();
                let val = self.builder.build_load(elem_ty, elem_ptr, "index_val")
                    .map_err(|e| miette::miette!("Failed to load indexed element: {:?}", e))?;
                Ok(Some(val))
            }

            ExprKind::Match { scrutinee, arms } => {
                // Compile scrutinee
                let scrutinee_val = self
                    .compile_expr(scrutinee, function)?
                    .ok_or_else(|| miette::miette!("Match scrutinee has no value"))?;

                // Determine if this is an enum with data (struct type) or simple discriminant
                let (scrutinee_int, scrutinee_struct_alloca) = if scrutinee_val.is_struct_value() {
                    // Enum with data - extract discriminant from field 0
                    let struct_val = scrutinee_val.into_struct_value();
                    let struct_ty = struct_val.get_type();

                    // Alloca to store the scrutinee so we can extract data later
                    let alloca = self.builder.build_alloca(struct_ty, "scrutinee_alloca")
                        .map_err(|e| miette::miette!("Failed to alloca scrutinee: {:?}", e))?;
                    self.builder.build_store(alloca, struct_val)
                        .map_err(|e| miette::miette!("Failed to store scrutinee: {:?}", e))?;

                    // Extract discriminant
                    let disc = self.builder.build_extract_value(struct_val, 0, "disc")
                        .map_err(|e| miette::miette!("Failed to extract discriminant: {:?}", e))?
                        .into_int_value();

                    (disc, Some((alloca, struct_ty)))
                } else {
                    // Simple integer discriminant
                    (scrutinee_val.into_int_value(), None)
                };

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
                    match &arm.pattern {
                        fragile_hir::Pattern::Literal(fragile_hir::Literal::Int(value)) => {
                            let case_val = scrutinee_int.get_type().const_int(*value as u64, false);
                            cases.push((case_val, arm_blocks[i]));
                        }
                        fragile_hir::Pattern::Variant { name, patterns: _ } => {
                            // Look up discriminant for this variant
                            // Parse name like "Color::Red" or "Option::Some" to get enum name and variant
                            let name_str = self.interner.resolve(*name);
                            let parts: Vec<&str> = name_str.split("::").collect();
                            if parts.len() == 2 {
                                let enum_name = self.interner.intern(parts[0]);
                                let variant_name = self.interner.intern(parts[1]);

                                // Try to find enum info - first check non-generic, then look for specializations
                                let enum_info = if let Some(info) = self.enum_types.get(&enum_name) {
                                    Some(info)
                                } else if self.generic_enums.contains_key(&enum_name) {
                                    // For generic enum, find any specialization that has this variant
                                    let base_name = self.interner.resolve(enum_name);
                                    self.enum_types
                                        .iter()
                                        .find(|(k, v)| {
                                            let k_str = self.interner.resolve(**k);
                                            k_str.starts_with(base_name.as_str()) && v.variant_discriminants.contains_key(&variant_name)
                                        })
                                        .map(|(_, v)| v)
                                } else {
                                    None
                                };

                                if let Some(info) = enum_info {
                                    if let Some(&disc) = info.variant_discriminants.get(&variant_name) {
                                        let case_val = scrutinee_int.get_type().const_int(disc as u64, false);
                                        cases.push((case_val, arm_blocks[i]));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Build switch instruction
                self.builder.build_switch(scrutinee_int, default_bb, &cases)
                    .map_err(|e| miette::miette!("Failed to build switch: {:?}", e))?;

                // If no wildcard, add unreachable to the default block
                if default_block.is_none() {
                    self.builder.position_at_end(default_bb);
                    self.builder.build_unreachable()
                        .map_err(|e| miette::miette!("Failed to build unreachable: {:?}", e))?;
                }

                // First pass: compile all arm bodies to determine common type
                let mut arm_results: Vec<Option<(BasicValueEnum, inkwell::basic_block::BasicBlock)>> = vec![];
                for (i, arm) in arms.iter().enumerate() {
                    self.builder.position_at_end(arm_blocks[i]);

                    // Bind pattern variables if this is a variant pattern with data
                    if let fragile_hir::Pattern::Variant { name, patterns } = &arm.pattern {
                        if !patterns.is_empty() {
                            if let Some((alloca, struct_ty)) = scrutinee_struct_alloca {
                                // Parse variant name to get enum info
                                let name_str = self.interner.resolve(*name);
                                let parts: Vec<&str> = name_str.split("::").collect();
                                if parts.len() == 2 {
                                    let enum_name = self.interner.intern(parts[0]);
                                    let variant_name = self.interner.intern(parts[1]);

                                    // Find enum info - first check non-generic, then look for specializations
                                    let enum_info = if let Some(info) = self.enum_types.get(&enum_name) {
                                        Some(info)
                                    } else if self.generic_enums.contains_key(&enum_name) {
                                        // For generic enum, find any specialization that has this variant
                                        let base_name = self.interner.resolve(enum_name);
                                        self.enum_types
                                            .iter()
                                            .find(|(k, v)| {
                                                let k_str = self.interner.resolve(**k);
                                                k_str.starts_with(base_name.as_str()) && v.variant_fields.contains_key(&variant_name)
                                            })
                                            .map(|(_, v)| v)
                                    } else {
                                        None
                                    };

                                    // Get the variant's field types
                                    if let Some(enum_info) = enum_info {
                                        if let Some(field_types) = enum_info.variant_fields.get(&variant_name) {
                                            // For each pattern binding, extract the data
                                            for (j, pattern) in patterns.iter().enumerate() {
                                                if let fragile_hir::Pattern::Ident(binding_sym) = pattern {
                                                    let binding_name = self.interner.resolve(*binding_sym);

                                                    if j < field_types.len() {
                                                        let field_ty = field_types[j];

                                                        // GEP to payload field (field 1 of the enum struct)
                                                        let payload_ptr = self.builder.build_struct_gep(struct_ty, alloca, 1, "payload_ptr")
                                                            .map_err(|e| miette::miette!("Failed to GEP payload: {:?}", e))?;

                                                        // Cast to the correct data type pointer
                                                        let data_ptr = self.builder.build_pointer_cast(
                                                            payload_ptr,
                                                            field_ty.ptr_type(inkwell::AddressSpace::default()),
                                                            "data_ptr"
                                                        ).map_err(|e| miette::miette!("Failed to cast data ptr: {:?}", e))?;

                                                        // Load the data
                                                        let data_val = self.builder.build_load(field_ty, data_ptr, &binding_name)
                                                            .map_err(|e| miette::miette!("Failed to load data: {:?}", e))?;

                                                        // Create variable binding
                                                        let var_alloca = self.builder.build_alloca(field_ty, &binding_name)
                                                            .map_err(|e| miette::miette!("Failed to alloca binding: {:?}", e))?;
                                                        self.builder.build_store(var_alloca, data_val)
                                                            .map_err(|e| miette::miette!("Failed to store binding: {:?}", e))?;
                                                        self.variables.insert(binding_name.to_string(), (var_alloca, field_ty));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(val) = self.compile_expr(&arm.body, function)? {
                        let block_end = self.builder.get_insert_block().unwrap();
                        arm_results.push(Some((val, block_end)));
                    } else {
                        arm_results.push(None);
                    }
                }

                // Determine common type - prefer i32 if any arm returns i32
                let mut common_type: Option<inkwell::types::BasicTypeEnum> = None;
                for result in &arm_results {
                    if let Some((val, _)) = result {
                        if val.is_int_value() {
                            let int_type = val.into_int_value().get_type();
                            if int_type.get_bit_width() == 32 {
                                common_type = Some(int_type.as_basic_type_enum());
                                break;
                            }
                        }
                        if common_type.is_none() {
                            common_type = Some(val.get_type());
                        }
                    }
                }

                // Second pass: cast values to common type and add branches
                let mut incoming: Vec<(BasicValueEnum, inkwell::basic_block::BasicBlock)> = vec![];
                for (i, result) in arm_results.into_iter().enumerate() {
                    self.builder.position_at_end(arm_blocks[i]);

                    if let Some((val, _)) = result {
                        // Cast to common type if needed
                        let casted_val = if let Some(target_ty) = common_type {
                            if val.get_type() != target_ty && val.is_int_value() && target_ty.is_int_type() {
                                let int_val = val.into_int_value();
                                let target_int_ty = target_ty.into_int_type();
                                if int_val.get_type().get_bit_width() > target_int_ty.get_bit_width() {
                                    self.builder.build_int_truncate(int_val, target_int_ty, "cast")
                                        .map_err(|e| miette::miette!("Failed to truncate: {:?}", e))?
                                        .as_basic_value_enum()
                                } else if int_val.get_type().get_bit_width() < target_int_ty.get_bit_width() {
                                    self.builder.build_int_s_extend(int_val, target_int_ty, "cast")
                                        .map_err(|e| miette::miette!("Failed to extend: {:?}", e))?
                                        .as_basic_value_enum()
                                } else {
                                    val
                                }
                            } else {
                                val
                            }
                        } else {
                            val
                        };

                        if !self.current_block_has_terminator() {
                            self.builder.build_unconditional_branch(merge_bb)
                                .map_err(|e| miette::miette!("Failed to build branch: {:?}", e))?;
                        }
                        let block_end = self.builder.get_insert_block().unwrap();
                        incoming.push((casted_val, block_end));
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
                    let result = phi.as_basic_value();
                    Ok(Some(result))
                }
            }

            ExprKind::Lambda { params, body } => {
                // Generate a unique name for the closure
                let closure_name = format!("__closure_{}", self.closure_counter);
                self.closure_counter += 1;

                // Collect captured variables (free vars that exist in current scope)
                let mut bound: std::collections::HashSet<String> = params.iter()
                    .map(|(sym, _)| self.interner.resolve(*sym).to_string())
                    .collect();
                let free_vars = self.collect_free_vars(body, &mut bound);

                // Collect captures: variables from outer scope that are used in the closure
                let captures: Vec<(String, BasicValueEnum<'ctx>, BasicTypeEnum<'ctx>)> = free_vars.iter()
                    .filter_map(|name| {
                        self.variables.get(name).map(|&(ptr, ty)| {
                            // Load the current value of the captured variable
                            let val = self.builder.build_load(ty, ptr, &format!("capture_{}", name))
                                .expect("Failed to load captured var");
                            (name.clone(), val, ty)
                        })
                    })
                    .collect();
                let capture_names: Vec<String> = captures.iter().map(|(n, _, _)| n.clone()).collect();

                // Build parameter types: regular params + captured vars
                let mut all_param_types: Vec<BasicMetadataTypeEnum> = params
                    .iter()
                    .map(|(_, ty)| {
                        if let Some(t) = ty {
                            self.lower_type(t)
                                .map(|t| t.into())
                                .unwrap_or_else(|| self.context.i64_type().into())
                        } else {
                            self.context.i64_type().into()
                        }
                    })
                    .collect();

                // Add captured variable types as extra parameters
                for (_, _, ty) in &captures {
                    all_param_types.push((*ty).into());
                }

                // Create the closure function type (returns i64 for now)
                let fn_type = self.context.i64_type().fn_type(&all_param_types, false);

                // Add the closure function to the module
                let closure_fn = self.module.add_function(&closure_name, fn_type, None);
                self.functions.insert(closure_name.clone(), closure_fn);

                // Store captured variable names for this closure
                self.closure_captures.insert(closure_name.clone(), capture_names);

                // Save current state
                let saved_block = self.builder.get_insert_block();
                let saved_vars = std::mem::take(&mut self.variables);

                // Create entry block for closure
                let entry = self.context.append_basic_block(closure_fn, "entry");
                self.builder.position_at_end(entry);

                // Set up regular parameters as variables
                for (i, (param_name, _)) in params.iter().enumerate() {
                    let name = self.interner.resolve(*param_name);
                    if let Some(param_val) = closure_fn.get_nth_param(i as u32) {
                        let ty = param_val.get_type();
                        let alloca = self.create_entry_alloca(closure_fn, &name, ty);
                        self.builder.build_store(alloca, param_val)
                            .map_err(|e| miette::miette!("Failed to store closure param: {:?}", e))?;
                        self.variables.insert(name.to_string(), (alloca, ty));
                    }
                }

                // Set up captured variables as additional parameters
                let num_regular_params = params.len();
                for (i, (cap_name, _, cap_ty)) in captures.iter().enumerate() {
                    let param_idx = (num_regular_params + i) as u32;
                    if let Some(param_val) = closure_fn.get_nth_param(param_idx) {
                        let alloca = self.create_entry_alloca(closure_fn, cap_name, *cap_ty);
                        self.builder.build_store(alloca, param_val)
                            .map_err(|e| miette::miette!("Failed to store captured var: {:?}", e))?;
                        self.variables.insert(cap_name.clone(), (alloca, *cap_ty));
                    }
                }

                // Compile closure body
                let result = self.compile_expr(body, closure_fn)?;

                // Add return
                if !self.current_block_has_terminator() {
                    if let Some(val) = result {
                        self.builder.build_return(Some(&val))
                            .map_err(|e| miette::miette!("Failed to build closure return: {:?}", e))?;
                    } else {
                        // Return 0 as default
                        let zero = self.context.i64_type().const_int(0, false);
                        self.builder.build_return(Some(&zero))
                            .map_err(|e| miette::miette!("Failed to build closure return: {:?}", e))?;
                    }
                }

                // Restore state
                self.variables = saved_vars;
                if let Some(bb) = saved_block {
                    self.builder.position_at_end(bb);
                }

                // Return pointer to the closure function
                let fn_ptr = closure_fn.as_global_value().as_pointer_value();
                Ok(Some(fn_ptr.into()))
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
                match pattern {
                    fragile_hir::Pattern::Ident(sym) => {
                        let name = self.interner.resolve(*sym);

                        // Determine type from type annotation first, then init
                        let declared_ty = ty.as_ref().and_then(|t| self.lower_type(t));

                        let llvm_ty = if let Some(init_expr) = init {
                            // Track pointee type for reference expressions
                            if let ExprKind::Unary { op: UnaryOp::AddrOf | UnaryOp::AddrOfMut, operand } = &init_expr.kind {
                                // If taking a reference to an identifier, record its type
                                if let ExprKind::Ident(ref_sym) = &operand.kind {
                                    let ref_name = self.interner.resolve(*ref_sym);
                                    if let Some(&(_, ref_ty)) = self.variables.get(ref_name.as_str()) {
                                        self.pointer_element_types.insert(name.to_string(), ref_ty);
                                    }
                                }
                            }

                            // Track closure assignments for later calls with captures
                            let is_lambda = matches!(&init_expr.kind, ExprKind::Lambda { .. });
                            let closure_name_before = if is_lambda {
                                Some(format!("__closure_{}", self.closure_counter))
                            } else {
                                None
                            };

                            let init_val = self.compile_expr(init_expr, function)?;
                            if let Some(val) = init_val {
                                // Use declared type if available, otherwise use inferred type
                                let target_ty = declared_ty.unwrap_or_else(|| val.get_type());
                                let alloca = self.create_entry_alloca(function, &name, target_ty);

                                // Cast value if needed (e.g., i64 literal to i32)
                                let store_val = if val.get_type() != target_ty && val.is_int_value() && target_ty.is_int_type() {
                                    self.builder.build_int_cast(
                                        val.into_int_value(),
                                        target_ty.into_int_type(),
                                        "cast"
                                    ).map_err(|e| miette::miette!("Failed to cast: {:?}", e))?.into()
                                } else {
                                    val
                                };

                                self.builder.build_store(alloca, store_val)
                                    .map_err(|e| miette::miette!("Failed to store: {:?}", e))?;
                                self.variables.insert(name.to_string(), (alloca, target_ty));

                                // Record variable->closure mapping if this was a lambda
                                if let Some(closure_name) = closure_name_before {
                                    self.variable_to_closure.insert(name.to_string(), closure_name);
                                }
                            }
                            return Ok(());
                        } else {
                            declared_ty
                        };

                        if let Some(llvm_ty) = llvm_ty {
                            let alloca = self.create_entry_alloca(function, &name, llvm_ty);
                            self.variables.insert(name.to_string(), (alloca, llvm_ty));
                        }
                    }

                    fragile_hir::Pattern::Tuple(patterns) => {
                        // Destructure a tuple
                        if let Some(init_expr) = init {
                            let init_val = self.compile_expr(init_expr, function)?
                                .ok_or_else(|| miette::miette!("Tuple destructuring init has no value"))?;

                            // The init value should be a struct (tuple)
                            if !init_val.is_struct_value() {
                                return Err(miette::miette!("Cannot destructure non-tuple value"));
                            }
                            let tuple_val = init_val.into_struct_value();

                            // Extract and bind each element
                            for (i, pat) in patterns.iter().enumerate() {
                                if let fragile_hir::Pattern::Ident(sym) = pat {
                                    let name = self.interner.resolve(*sym);
                                    let elem_val = self.builder
                                        .build_extract_value(tuple_val, i as u32, &format!("tuple.{}", i))
                                        .map_err(|e| miette::miette!("Failed to extract tuple element: {:?}", e))?;
                                    let elem_ty = elem_val.get_type();
                                    let alloca = self.create_entry_alloca(function, &name, elem_ty);
                                    self.builder.build_store(alloca, elem_val)
                                        .map_err(|e| miette::miette!("Failed to store tuple element: {:?}", e))?;
                                    self.variables.insert(name.to_string(), (alloca, elem_ty));
                                }
                                // Skip wildcard patterns
                            }
                        }
                    }

                    fragile_hir::Pattern::Struct { name, fields } => {
                        // Destructure a struct
                        if let Some(init_expr) = init {
                            let init_val = self.compile_expr(init_expr, function)?
                                .ok_or_else(|| miette::miette!("Struct destructuring init has no value"))?;

                            // The init value should be a struct
                            if !init_val.is_struct_value() {
                                return Err(miette::miette!("Cannot destructure non-struct value"));
                            }
                            let struct_val = init_val.into_struct_value();

                            // Get struct info to find field indices
                            let struct_info = self.struct_types.get(name)
                                .ok_or_else(|| miette::miette!("Unknown struct type for destructuring"))?;

                            // Extract and bind each field
                            for (field_name, pat) in fields {
                                if let fragile_hir::Pattern::Ident(var_sym) = pat {
                                    let field_idx = struct_info.field_indices.get(field_name)
                                        .ok_or_else(|| {
                                            let fn_str = self.interner.resolve(*field_name);
                                            miette::miette!("Unknown field {} in struct", fn_str)
                                        })?;

                                    let field_name_str = self.interner.resolve(*field_name);
                                    let elem_val = self.builder
                                        .build_extract_value(struct_val, *field_idx, &format!("field.{}", field_name_str))
                                        .map_err(|e| miette::miette!("Failed to extract struct field: {:?}", e))?;
                                    let elem_ty = elem_val.get_type();

                                    let var_name = self.interner.resolve(*var_sym);
                                    let alloca = self.create_entry_alloca(function, &var_name, elem_ty);
                                    self.builder.build_store(alloca, elem_val)
                                        .map_err(|e| miette::miette!("Failed to store struct field: {:?}", e))?;
                                    self.variables.insert(var_name.to_string(), (alloca, elem_ty));
                                }
                                // Skip wildcard patterns
                            }
                        }
                    }

                    fragile_hir::Pattern::Wildcard => {
                        // Evaluate init for side effects but don't bind
                        if let Some(init_expr) = init {
                            self.compile_expr(init_expr, function)?;
                        }
                    }

                    _ => {
                        // Other patterns not yet supported
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
            Literal::String(s) => {
                // Create a global string constant with null terminator
                let string_val = self.context.const_string(s.as_bytes(), true);
                let global = self.module.add_global(string_val.get_type(), None, ".str");
                global.set_initializer(&string_val);
                global.set_constant(true);
                global.set_unnamed_addr(true);

                // Return pointer to the string data
                let ptr = global.as_pointer_value();
                Ok(Some(ptr.into()))
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

            // If types don't match, cast the smaller one to the larger
            let (lhs_int, rhs_int) = if lhs_int.get_type() != rhs_int.get_type() {
                let lhs_bits = lhs_int.get_type().get_bit_width();
                let rhs_bits = rhs_int.get_type().get_bit_width();
                if lhs_bits > rhs_bits {
                    let rhs_cast = self.builder.build_int_cast(rhs_int, lhs_int.get_type(), "cast")
                        .map_err(|e| miette::miette!("Failed to cast rhs: {:?}", e))?;
                    (lhs_int, rhs_cast)
                } else {
                    let lhs_cast = self.builder.build_int_cast(lhs_int, rhs_int.get_type(), "cast")
                        .map_err(|e| miette::miette!("Failed to cast lhs: {:?}", e))?;
                    (lhs_cast, rhs_int)
                }
            } else {
                (lhs_int, rhs_int)
            };

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
            UnaryOp::Deref => {
                // Dereference a pointer
                if val.is_pointer_value() {
                    let ptr = val.into_pointer_value();
                    // For now, assume i32 as the pointee type
                    // TODO: Proper type tracking for pointers
                    let i32_ty = self.context.i32_type();
                    let result = self.builder.build_load(i32_ty, ptr, "deref")
                        .map_err(|e| miette::miette!("Failed to build load: {:?}", e))?;
                    Ok(Some(result))
                } else {
                    Err(miette::miette!("Cannot dereference non-pointer"))
                }
            }
            UnaryOp::AddrOf | UnaryOp::AddrOfMut => {
                // For address-of, we need an alloca to get a pointer
                // This is a simplified implementation
                if val.is_int_value() {
                    let alloca = self.builder.build_alloca(val.get_type(), "addr_temp")
                        .map_err(|e| miette::miette!("Failed to build alloca: {:?}", e))?;
                    self.builder.build_store(alloca, val)
                        .map_err(|e| miette::miette!("Failed to build store: {:?}", e))?;
                    Ok(Some(alloca.into()))
                } else {
                    Err(miette::miette!("Cannot take address of this value type"))
                }
            }
        }
    }

    /// Find the struct name given an LLVM struct type
    fn find_struct_name_by_llvm_type(&self, llvm_type: inkwell::types::StructType<'ctx>) -> Option<String> {
        for (struct_sym, info) in &self.struct_types {
            if info.llvm_type == llvm_type {
                return Some(self.interner.resolve(*struct_sym).to_string());
            }
        }
        None
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
                // First check if it's a type alias
                if let Some(aliased_ty) = self.type_aliases.get(name) {
                    return self.lower_type(aliased_ty);
                }
                // Then look up struct type
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
            Type::Slice { .. } => {
                // Slices are fat pointers: { ptr, len }
                // For now, represent as a struct with pointer and i64 length
                let ptr_ty = self.context.ptr_type(AddressSpace::default());
                let len_ty = self.context.i64_type();
                let slice_ty = self.context.struct_type(&[ptr_ty.into(), len_ty.into()], false);
                Some(slice_ty.into())
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

    /// Substitute type parameters with concrete types in a type
    fn substitute_type(
        &self,
        ty: &Type,
        substitutions: &FxHashMap<Symbol, Type>,
    ) -> Type {
        match ty {
            Type::Named { name, type_args } => {
                // Check if this is a type parameter
                if let Some(concrete) = substitutions.get(name) {
                    return concrete.clone();
                }
                // Otherwise substitute in type args
                Type::Named {
                    name: *name,
                    type_args: type_args
                        .iter()
                        .map(|t| self.substitute_type(t, substitutions))
                        .collect(),
                }
            }
            Type::Pointer { inner, mutability } => Type::Pointer {
                inner: Box::new(self.substitute_type(inner, substitutions)),
                mutability: *mutability,
            },
            Type::Reference { inner, mutability } => Type::Reference {
                inner: Box::new(self.substitute_type(inner, substitutions)),
                mutability: *mutability,
            },
            Type::Array { inner, size } => Type::Array {
                inner: Box::new(self.substitute_type(inner, substitutions)),
                size: *size,
            },
            Type::Slice { inner } => Type::Slice {
                inner: Box::new(self.substitute_type(inner, substitutions)),
            },
            Type::Tuple(types) => Type::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitutions))
                    .collect(),
            ),
            Type::Function { params, ret, is_variadic } => Type::Function {
                params: params
                    .iter()
                    .map(|t| self.substitute_type(t, substitutions))
                    .collect(),
                ret: Box::new(self.substitute_type(ret, substitutions)),
                is_variadic: *is_variadic,
            },
            // Primitives and other types remain unchanged
            _ => ty.clone(),
        }
    }

    /// Create a mangled name for a specialized generic function
    fn mangle_generic_name(&self, base_name: &str, type_args: &[Type]) -> String {
        let mut name = base_name.to_string();
        for ty in type_args {
            name.push('_');
            name.push_str(&self.type_to_string(ty));
        }
        name
    }

    /// Convert a type to a string for mangling
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(p) => format!("{:?}", p).to_lowercase(),
            Type::Named { name, .. } => self.interner.resolve(*name).to_string(),
            Type::Pointer { inner, .. } => format!("ptr_{}", self.type_to_string(inner)),
            Type::Reference { inner, .. } => format!("ref_{}", self.type_to_string(inner)),
            Type::Array { inner, size } => format!("arr{}_{}", size, self.type_to_string(inner)),
            Type::Tuple(types) => {
                let inner: Vec<_> = types.iter().map(|t| self.type_to_string(t)).collect();
                format!("tuple_{}", inner.join("_"))
            }
            Type::Slice { inner } => format!("slice_{}", self.type_to_string(inner)),
            _ => "unknown".to_string(),
        }
    }

    /// Infer concrete type from a compiled value
    fn infer_type_from_value(&self, val: BasicValueEnum<'ctx>) -> Type {
        if val.is_int_value() {
            let int_type = val.into_int_value().get_type();
            let bits = int_type.get_bit_width();
            match bits {
                1 => Type::Primitive(PrimitiveType::Bool),
                8 => Type::Primitive(PrimitiveType::I8),
                16 => Type::Primitive(PrimitiveType::I16),
                32 => Type::Primitive(PrimitiveType::I32),
                64 => Type::Primitive(PrimitiveType::I64),
                128 => Type::Primitive(PrimitiveType::I128),
                _ => Type::Primitive(PrimitiveType::I64),
            }
        } else if val.is_float_value() {
            let float_type = val.into_float_value().get_type();
            if float_type == self.context.f32_type() {
                Type::Primitive(PrimitiveType::F32)
            } else {
                Type::Primitive(PrimitiveType::F64)
            }
        } else {
            Type::Primitive(PrimitiveType::I64) // Default fallback
        }
    }

    /// Get or create a specialized version of a generic function
    fn get_or_create_specialization(
        &mut self,
        fn_name: Symbol,
        arg_types: &[Type],
        function: FunctionValue<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        // Get the generic function definition
        let generic_fn = self.generic_functions.get(&fn_name).cloned()
            .ok_or_else(|| {
                let name = self.interner.resolve(fn_name);
                miette::miette!("Generic function {} not found", name)
            })?;

        // Build type substitution map
        let mut substitutions = FxHashMap::default();
        for (i, type_param) in generic_fn.type_params.iter().enumerate() {
            if i < arg_types.len() {
                substitutions.insert(type_param.name, arg_types[i].clone());
            }
        }

        // Create mangled name
        let base_name = self.interner.resolve(fn_name);
        let mangled_name = self.mangle_generic_name(&base_name, arg_types);

        // Check if we've already created this specialization
        if let Some(&func) = self.functions.get(&mangled_name) {
            return Ok(func);
        }

        // Create specialized parameter types
        let param_types: Vec<BasicMetadataTypeEnum> = generic_fn
            .sig
            .params
            .iter()
            .filter_map(|p| {
                let subst_ty = self.substitute_type(&p.ty, &substitutions);
                self.lower_type(&subst_ty).map(|t| t.into())
            })
            .collect();

        // Create specialized return type
        let ret_ty = self.substitute_type(&generic_fn.sig.ret_ty, &substitutions);
        let llvm_ret_ty = self.lower_type(&ret_ty);

        // Create function type
        let fn_type = match llvm_ret_ty {
            Some(ty) => ty.fn_type(&param_types, false),
            None => self.context.void_type().fn_type(&param_types, false),
        };

        // Declare the specialized function
        let spec_fn = self.module.add_function(&mangled_name, fn_type, None);
        self.functions.insert(mangled_name.clone(), spec_fn);

        // Save current builder position
        let saved_block = self.builder.get_insert_block();
        let saved_vars = std::mem::take(&mut self.variables);

        // Create entry block for the specialized function
        let entry = self.context.append_basic_block(spec_fn, "entry");
        self.builder.position_at_end(entry);

        // Create allocas for parameters
        for (i, param) in generic_fn.sig.params.iter().enumerate() {
            let param_name = self.interner.resolve(param.name);
            let param_val = spec_fn.get_nth_param(i as u32);

            if let Some(value) = param_val {
                let ty = value.get_type();
                let alloca = self.create_entry_alloca(spec_fn, &param_name, ty);
                self.builder.build_store(alloca, value)
                    .map_err(|e| miette::miette!("Failed to store param: {:?}", e))?;
                self.variables.insert(param_name.to_string(), (alloca, ty));
            }
        }

        // Compile the function body
        if let Some(body) = &generic_fn.body {
            let result = self.compile_expr(body, spec_fn)?;

            // Add return if needed
            if !self.current_block_has_terminator() {
                if ret_ty == Type::unit() {
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

        // Restore builder position and variables
        self.variables = saved_vars;
        if let Some(bb) = saved_block {
            self.builder.position_at_end(bb);
        }

        Ok(spec_fn)
    }
}
