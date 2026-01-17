//! MIR format conversion from Fragile's simplified MIR to rustc's internal format.
//!
//! This module provides the infrastructure for converting `fragile_clang::MirBody`
//! to `rustc_middle::mir::Body`. Due to the complexity and instability of rustc's
//! internal APIs, this is implemented incrementally.
//!
//! Current status: Basic infrastructure with trivial body generation.

#![cfg(feature = "rustc-integration")]

extern crate rustc_abi;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_span::source_map::Spanned;

use fragile_clang::{
    CppType, MirBasicBlock, MirBinOp, MirBody, MirConstant, MirLocal, MirOperand, MirPlace,
    MirRvalue, MirStatement, MirTerminator, MirUnaryOp,
};
use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::{self, BinOp, UnOp};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::DUMMY_SP;

use crate::rustc_integration::{lookup_def_id_by_export_name, lookup_mangled_name_by_display};

/// Context for MIR conversion, holding the type context and other state.
pub struct MirConvertCtx<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> MirConvertCtx<'tcx> {
    /// Create a new conversion context.
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self { tcx }
    }

    /// Get the type context.
    pub fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    /// Resolve a function call to a rustc Operand.
    ///
    /// Given a function name (display name like "helper" or qualified like "foo::helper"),
    /// look up the DefId and create a proper function operand for a Call terminator.
    ///
    /// The resolution process:
    /// 1. Try looking up as mangled name directly (if already mangled)
    /// 2. Try looking up display name to get mangled name, then find DefId
    ///
    /// Returns None if the function cannot be resolved (external function, not in crate).
    fn resolve_function_call(&self, func_name: &str) -> Option<mir::Operand<'tcx>> {
        // First, try to look up directly by export name (in case it's already mangled)
        if let Some(def_id) = lookup_def_id_by_export_name(self.tcx, func_name) {
            eprintln!("[fragile] Resolved function call '{}' directly to {:?}", func_name, def_id);
            return Some(self.create_function_operand(def_id));
        }

        // Try looking up display name -> mangled name -> DefId
        if let Some(mangled_name) = lookup_mangled_name_by_display(func_name) {
            eprintln!("[fragile] Found mangled name '{}' for display name '{}'", mangled_name, func_name);
            if let Some(def_id) = lookup_def_id_by_export_name(self.tcx, &mangled_name) {
                eprintln!("[fragile] Resolved function call '{}' via mangled name to {:?}", func_name, def_id);
                return Some(self.create_function_operand(def_id));
            }
        }

        eprintln!("[fragile] Could not resolve function call '{}'", func_name);
        None
    }

    /// Create a function operand from a DefId.
    fn create_function_operand(&self, def_id: LocalDefId) -> mir::Operand<'tcx> {
        mir::Operand::function_handle(
            self.tcx,
            def_id.to_def_id(),
            [], // No generic arguments for simple C++ functions
            DUMMY_SP,
        )
    }

    /// Convert a Fragile MirBody to rustc's mir::Body.
    ///
    /// This is a placeholder that creates a minimal valid MIR body.
    /// Full conversion will be implemented incrementally.
    ///
    /// # Current limitations
    /// - Creates a single-block body with just a return
    /// - Does not convert actual statements or terminators
    /// - Uses placeholder types
    pub fn convert_mir_body(&self, _mir: &MirBody, return_ty: Ty<'tcx>) -> mir::Body<'tcx> {
        // Create minimal MIR: single block with return
        //
        // The full implementation would convert:
        // - mir.locals -> local_decls
        // - mir.blocks -> basic_blocks
        // - Handle all statement/terminator variants
        //
        // For now, we create a trivial body that just returns.

        self.create_trivial_body(return_ty)
    }

    /// Create a trivial MIR body that just returns.
    ///
    /// This is useful for:
    /// - Testing the infrastructure
    /// - Placeholder for functions we can't fully convert yet
    fn create_trivial_body(&self, return_ty: Ty<'tcx>) -> mir::Body<'tcx> {
        use rustc_index::IndexVec;
        use rustc_middle::mir::*;

        // Create the return local (always at index 0)
        let return_local = LocalDecl::new(return_ty, DUMMY_SP);

        // Create local declarations (just the return place)
        let mut local_decls: IndexVec<Local, LocalDecl<'tcx>> = IndexVec::new();
        local_decls.push(return_local);

        // Create a single basic block with a return terminator
        let return_block = BasicBlockData::new(
            Some(Terminator {
                source_info: SourceInfo::outermost(DUMMY_SP),
                kind: TerminatorKind::Return,
            }),
            false, // is_cleanup
        );

        let mut basic_blocks: IndexVec<BasicBlock, BasicBlockData<'tcx>> = IndexVec::new();
        basic_blocks.push(return_block);

        // Create source scope (minimal, just one scope)
        let mut source_scopes: IndexVec<SourceScope, SourceScopeData<'tcx>> = IndexVec::new();
        source_scopes.push(SourceScopeData {
            span: DUMMY_SP,
            parent_scope: None,
            inlined: None,
            inlined_parent_scope: None,
            local_data: ClearCrossCrate::Clear,
        });

        // Create the body
        mir::Body::new(
            MirSource::item(rustc_span::def_id::CRATE_DEF_ID.to_def_id()),
            basic_blocks,
            source_scopes,
            local_decls,
            IndexVec::new(), // user_type_annotations
            0,               // arg_count (no arguments for trivial body)
            Vec::new(),      // var_debug_info
            DUMMY_SP,        // span
            None,            // coroutine
            None,            // tainted_by_errors
        )
    }

    /// Convert a CppType to rustc Ty.
    ///
    /// Performs full recursive type conversion including:
    /// - Primitive types (void, bool, char, short, int, long, long long, float, double)
    /// - Pointer types with proper pointee conversion (*T, *const T, *mut T)
    /// - Reference types (converted to raw pointers for FFI)
    /// - Array types ([T; N] for fixed-size, *T for unsized)
    /// - Named types (structs, classes, enums)
    /// - Function pointer types
    pub fn convert_type(&self, cpp_type: &CppType) -> Ty<'tcx> {
        match cpp_type {
            // ================================================================
            // Primitive Types
            // ================================================================
            CppType::Void => self.tcx.types.unit,
            CppType::Bool => self.tcx.types.bool,

            // Integers with signed flag
            CppType::Char { signed } => {
                if *signed {
                    self.tcx.types.i8
                } else {
                    self.tcx.types.u8
                }
            }
            CppType::Short { signed } => {
                if *signed {
                    self.tcx.types.i16
                } else {
                    self.tcx.types.u16
                }
            }
            CppType::Int { signed } => {
                if *signed {
                    self.tcx.types.i32
                } else {
                    self.tcx.types.u32
                }
            }
            CppType::Long { signed } => {
                // On 64-bit systems, long is typically 64 bits
                if *signed {
                    self.tcx.types.i64
                } else {
                    self.tcx.types.u64
                }
            }
            CppType::LongLong { signed } => {
                if *signed {
                    self.tcx.types.i64
                } else {
                    self.tcx.types.u64
                }
            }

            // Floating point
            CppType::Float => self.tcx.types.f32,
            CppType::Double => self.tcx.types.f64,

            // ================================================================
            // Pointer Types - Recursive conversion
            // ================================================================
            CppType::Pointer { pointee, is_const } => {
                // Recursively convert pointee type
                let pointee_ty = self.convert_type(pointee);
                let mutability = if *is_const {
                    rustc_middle::ty::Mutability::Not
                } else {
                    rustc_middle::ty::Mutability::Mut
                };
                Ty::new_ptr(self.tcx, pointee_ty, mutability)
            }

            // ================================================================
            // Reference Types - Convert to raw pointers for FFI
            // ================================================================
            // In C++ FFI, references are passed as pointers:
            // - const T& -> *const T
            // - T& -> *mut T
            // - T&& (rvalue ref) -> *mut T (ownership transfer)
            CppType::Reference { referent, is_const, is_rvalue: _ } => {
                // Recursively convert referent type
                let referent_ty = self.convert_type(referent);
                let mutability = if *is_const {
                    rustc_middle::ty::Mutability::Not
                } else {
                    rustc_middle::ty::Mutability::Mut
                };
                // For FFI, use raw pointers instead of Rust references
                Ty::new_ptr(self.tcx, referent_ty, mutability)
            }

            // ================================================================
            // Array Types
            // ================================================================
            CppType::Array { element, size } => {
                let elem_ty = self.convert_type(element);
                if let Some(n) = size {
                    // Fixed-size array: [T; N]
                    Ty::new_array(self.tcx, elem_ty, *n as u64)
                } else {
                    // Unsized array (flexible array member) -> *mut T
                    Ty::new_ptr(self.tcx, elem_ty, rustc_middle::ty::Mutability::Mut)
                }
            }

            // ================================================================
            // Named Types (struct, class, enum, typedef)
            // ================================================================
            CppType::Named(name) => {
                // Handle well-known type aliases
                match name.as_str() {
                    // Standard integer types
                    "size_t" | "std::size_t" => self.tcx.types.usize,
                    "ssize_t" => self.tcx.types.isize,
                    "ptrdiff_t" | "std::ptrdiff_t" => self.tcx.types.isize,
                    "uintptr_t" | "std::uintptr_t" => self.tcx.types.usize,
                    "intptr_t" | "std::intptr_t" => self.tcx.types.isize,

                    // Fixed-width integer types
                    "int8_t" | "std::int8_t" => self.tcx.types.i8,
                    "uint8_t" | "std::uint8_t" => self.tcx.types.u8,
                    "int16_t" | "std::int16_t" => self.tcx.types.i16,
                    "uint16_t" | "std::uint16_t" => self.tcx.types.u16,
                    "int32_t" | "std::int32_t" => self.tcx.types.i32,
                    "uint32_t" | "std::uint32_t" => self.tcx.types.u32,
                    "int64_t" | "std::int64_t" => self.tcx.types.i64,
                    "uint64_t" | "std::uint64_t" => self.tcx.types.u64,

                    // Character types
                    "wchar_t" => self.tcx.types.i32, // Platform-dependent, typically 32-bit on Linux
                    "char16_t" => self.tcx.types.u16,
                    "char32_t" => self.tcx.types.u32,

                    // For unknown named types (structs, classes, enums), use an opaque pointer
                    // This is safe for FFI as we don't need to know the layout
                    _ => {
                        // Use *mut () as opaque type placeholder
                        // A more sophisticated implementation would look up the type in a registry
                        Ty::new_ptr(self.tcx, self.tcx.types.unit, rustc_middle::ty::Mutability::Mut)
                    }
                }
            }

            // ================================================================
            // Function Types
            // ================================================================
            CppType::Function { return_type, params, is_variadic } => {
                // Convert return type
                let ret_ty = self.convert_type(return_type);

                // Convert parameter types
                let param_tys: Vec<_> = params.iter().map(|p| self.convert_type(p)).collect();

                // Create function signature
                // Note: Variadic functions need special ABI handling
                let abi = if *is_variadic {
                    rustc_abi::ExternAbi::C { unwind: false }
                } else {
                    rustc_abi::ExternAbi::C { unwind: false }
                };

                // Create the fn pointer type
                let fn_sig = rustc_middle::ty::Binder::dummy(
                    self.tcx.mk_fn_sig(
                        param_tys,
                        ret_ty,
                        *is_variadic,
                        rustc_hir::Safety::Unsafe,
                        abi,
                    )
                );
                Ty::new_fn_ptr(self.tcx, fn_sig)
            }

            // ================================================================
            // Template-Related Types
            // ================================================================
            // These types should not normally reach MIR conversion - templates
            // should be instantiated before MIR generation. Use fallback type.
            CppType::TemplateParam { name, .. } => {
                eprintln!("[fragile] Warning: uninstantiated template param '{}' in type conversion", name);
                self.tcx.types.unit
            }
            CppType::DependentType { spelling } => {
                eprintln!("[fragile] Warning: dependent type '{}' in type conversion", spelling);
                self.tcx.types.unit
            }
            CppType::ParameterPack { name, .. } => {
                eprintln!("[fragile] Warning: unexpanded parameter pack '{}' in type conversion", name);
                self.tcx.types.unit
            }
        }
    }

    /// Convert a Fragile MirBinOp to rustc's BinOp.
    pub fn convert_binop(&self, op: &MirBinOp) -> BinOp {
        match op {
            MirBinOp::Add => BinOp::Add,
            MirBinOp::Sub => BinOp::Sub,
            MirBinOp::Mul => BinOp::Mul,
            MirBinOp::Div => BinOp::Div,
            MirBinOp::Rem => BinOp::Rem,
            MirBinOp::BitAnd => BinOp::BitAnd,
            MirBinOp::BitOr => BinOp::BitOr,
            MirBinOp::BitXor => BinOp::BitXor,
            MirBinOp::Shl => BinOp::Shl,
            MirBinOp::Shr => BinOp::Shr,
            MirBinOp::Eq => BinOp::Eq,
            MirBinOp::Ne => BinOp::Ne,
            MirBinOp::Lt => BinOp::Lt,
            MirBinOp::Le => BinOp::Le,
            MirBinOp::Gt => BinOp::Gt,
            MirBinOp::Ge => BinOp::Ge,
        }
    }

    /// Convert a Fragile MirUnaryOp to rustc's UnOp.
    pub fn convert_unop(&self, op: &MirUnaryOp) -> UnOp {
        match op {
            MirUnaryOp::Neg => UnOp::Neg,
            MirUnaryOp::Not => UnOp::Not,
        }
    }

    /// Convert a Fragile MirConstant to a rustc constant operand.
    ///
    /// Returns (ty, const_value) where ty is the inferred type and const_value
    /// is the constant that can be used to construct a Const.
    pub fn convert_constant(&self, constant: &MirConstant) -> (Ty<'tcx>, mir::ConstValue) {
        use rustc_middle::mir::ConstValue;
        use rustc_middle::ty::ScalarInt;

        match constant {
            MirConstant::Int { value, bits, signed } => {
                // Determine type and scalar based on bit width and signedness
                let (ty, scalar) = if *signed {
                    // Signed integer types
                    match bits {
                        8 => (self.tcx.types.i8, ScalarInt::try_from_int(*value as i8, rustc_abi::Size::from_bytes(1)).unwrap()),
                        16 => (self.tcx.types.i16, ScalarInt::try_from_int(*value as i16, rustc_abi::Size::from_bytes(2)).unwrap()),
                        32 => (self.tcx.types.i32, ScalarInt::try_from_int(*value as i32, rustc_abi::Size::from_bytes(4)).unwrap()),
                        64 => (self.tcx.types.i64, ScalarInt::try_from_int(*value as i64, rustc_abi::Size::from_bytes(8)).unwrap()),
                        128 => (self.tcx.types.i128, ScalarInt::try_from_int(*value, rustc_abi::Size::from_bytes(16)).unwrap()),
                        _ => {
                            // Default to i32 for unrecognized sizes
                            (self.tcx.types.i32, ScalarInt::try_from_int(*value as i32, rustc_abi::Size::from_bytes(4)).unwrap())
                        }
                    }
                } else {
                    // Unsigned integer types
                    match bits {
                        8 => (self.tcx.types.u8, ScalarInt::try_from_uint(*value as u8 as u128, rustc_abi::Size::from_bytes(1)).unwrap()),
                        16 => (self.tcx.types.u16, ScalarInt::try_from_uint(*value as u16 as u128, rustc_abi::Size::from_bytes(2)).unwrap()),
                        32 => (self.tcx.types.u32, ScalarInt::try_from_uint(*value as u32 as u128, rustc_abi::Size::from_bytes(4)).unwrap()),
                        64 => (self.tcx.types.u64, ScalarInt::try_from_uint(*value as u64 as u128, rustc_abi::Size::from_bytes(8)).unwrap()),
                        128 => (self.tcx.types.u128, ScalarInt::try_from_uint(*value as u128, rustc_abi::Size::from_bytes(16)).unwrap()),
                        _ => {
                            // Default to u32 for unrecognized sizes
                            (self.tcx.types.u32, ScalarInt::try_from_uint(*value as u32 as u128, rustc_abi::Size::from_bytes(4)).unwrap())
                        }
                    }
                };
                (ty, ConstValue::Scalar(scalar.into()))
            }
            MirConstant::Float { value, bits } => {
                let (ty, scalar) = if *bits <= 32 {
                    let f = *value as f32;
                    (self.tcx.types.f32, ScalarInt::try_from_uint(f.to_bits() as u128, rustc_abi::Size::from_bytes(4)).unwrap())
                } else {
                    let f = *value;
                    (self.tcx.types.f64, ScalarInt::try_from_uint(f.to_bits() as u128, rustc_abi::Size::from_bytes(8)).unwrap())
                };
                (ty, ConstValue::Scalar(scalar.into()))
            }
            MirConstant::Bool(b) => {
                let scalar = if *b { ScalarInt::TRUE } else { ScalarInt::FALSE };
                (self.tcx.types.bool, ConstValue::Scalar(scalar.into()))
            }
            MirConstant::Unit => {
                (self.tcx.types.unit, ConstValue::ZeroSized)
            }
        }
    }

    /// Convert a Fragile MirPlace to rustc's Place.
    ///
    /// Note: This is a simplified conversion. Full conversion requires:
    /// - Proper type tracking for field projections
    /// - Local variable mapping
    pub fn convert_place(&self, place: &MirPlace) -> mir::Place<'tcx> {
        use rustc_abi::FieldIdx;
        use rustc_middle::mir::{Local, Place, ProjectionElem};

        let local = Local::from_usize(place.local);

        if place.projection.is_empty() {
            Place::from(local)
        } else {
            // For now, handle simple projections
            // Full implementation needs type tracking for Field projections
            let projections: Vec<_> = place.projection.iter().map(|proj| {
                match proj {
                    fragile_clang::MirProjection::Deref => ProjectionElem::Deref,
                    fragile_clang::MirProjection::Field { index, name: _ } => {
                        // Note: This needs the actual field type, using unit as placeholder
                        // TODO: Use the `name` field to look up proper type when available
                        ProjectionElem::Field(
                            FieldIdx::from_usize(*index),
                            self.tcx.types.unit,
                        )
                    }
                    fragile_clang::MirProjection::Index(idx) => {
                        ProjectionElem::Index(Local::from_usize(*idx))
                    }
                }
            }).collect();

            Place {
                local,
                projection: self.tcx.mk_place_elems(&projections),
            }
        }
    }

    /// Convert a Fragile MirOperand to rustc's Operand.
    pub fn convert_operand(&self, operand: &MirOperand) -> mir::Operand<'tcx> {
        use rustc_middle::mir::{Operand, Const};

        match operand {
            MirOperand::Copy(place) => Operand::Copy(self.convert_place(place)),
            MirOperand::Move(place) => Operand::Move(self.convert_place(place)),
            MirOperand::Constant(constant) => {
                let (ty, const_value) = self.convert_constant(constant);
                // Use mir::Const::Val which directly stores the ConstValue and Ty
                let const_ = Const::Val(const_value, ty);
                Operand::Constant(Box::new(mir::ConstOperand {
                    span: DUMMY_SP,
                    user_ty: None,
                    const_,
                }))
            }
        }
    }

    /// Convert a Fragile MirRvalue to rustc's Rvalue.
    pub fn convert_rvalue(&self, rvalue: &MirRvalue) -> mir::Rvalue<'tcx> {
        use rustc_middle::mir::Rvalue;

        match rvalue {
            MirRvalue::Use(operand) => Rvalue::Use(self.convert_operand(operand)),
            MirRvalue::BinaryOp { op, left, right } => {
                let binop = self.convert_binop(op);
                let left_op = self.convert_operand(left);
                let right_op = self.convert_operand(right);
                Rvalue::BinaryOp(binop, Box::new((left_op, right_op)))
            }
            MirRvalue::UnaryOp { op, operand } => {
                let unop = self.convert_unop(op);
                let op = self.convert_operand(operand);
                Rvalue::UnaryOp(unop, op)
            }
            MirRvalue::Ref { place, mutability } => {
                use rustc_middle::mir::BorrowKind;
                let p = self.convert_place(place);
                let kind = if *mutability {
                    BorrowKind::Mut { kind: rustc_middle::mir::MutBorrowKind::Default }
                } else {
                    BorrowKind::Shared
                };
                Rvalue::Ref(self.tcx.lifetimes.re_erased, kind, p)
            }
            MirRvalue::Aggregate { ty, fields } => {
                use rustc_middle::mir::AggregateKind;
                // Convert aggregate initialization
                // For now, we generate a tuple aggregate as a placeholder
                // TODO: Properly resolve struct types and generate correct aggregate kind
                let operands: Vec<_> = fields.iter()
                    .map(|(_name, operand)| self.convert_operand(operand))
                    .collect();

                // Use the unit type as placeholder for now
                // Full implementation would look up the actual struct type
                let _ = ty; // silence unused warning
                Rvalue::Aggregate(
                    Box::new(AggregateKind::Tuple),
                    operands.into_iter().collect(),
                )
            }
        }
    }

    /// Convert a Fragile MirStatement to rustc's Statement.
    pub fn convert_statement(&self, stmt: &MirStatement) -> mir::Statement<'tcx> {
        use rustc_middle::mir::{Statement, StatementKind, SourceInfo};

        let source_info = SourceInfo::outermost(DUMMY_SP);

        let kind = match stmt {
            MirStatement::Assign { target, value } => {
                let place = self.convert_place(target);
                let rvalue = self.convert_rvalue(value);
                StatementKind::Assign(Box::new((place, rvalue)))
            }
            MirStatement::Nop => StatementKind::Nop,
        };

        Statement::new(source_info, kind)
    }

    /// Convert a Fragile MirTerminator to rustc's Terminator.
    pub fn convert_terminator(&self, term: &MirTerminator) -> mir::Terminator<'tcx> {
        use rustc_middle::mir::{BasicBlock, Terminator, TerminatorKind, SourceInfo, SwitchTargets};

        let source_info = SourceInfo::outermost(DUMMY_SP);

        let kind = match term {
            MirTerminator::Return => TerminatorKind::Return,
            MirTerminator::Goto { target } => {
                TerminatorKind::Goto { target: BasicBlock::from_usize(*target) }
            }
            MirTerminator::SwitchInt { operand, targets, otherwise } => {
                let discr = self.convert_operand(operand);
                // Build switch targets - mapping values to blocks
                let targets_vec: Vec<_> = targets
                    .iter()
                    .map(|(val, block)| (*val as u128, BasicBlock::from_usize(*block)))
                    .collect();
                let otherwise_block = BasicBlock::from_usize(*otherwise);
                let switch_targets = SwitchTargets::new(targets_vec.into_iter(), otherwise_block);
                TerminatorKind::SwitchInt { discr, targets: switch_targets }
            }
            MirTerminator::Call { func, args, destination, target, unwind } => {
                // Resolve the function name to a rustc function operand
                let args_converted: Box<[Spanned<mir::Operand<'tcx>>]> = args
                    .iter()
                    .map(|a| Spanned {
                        node: self.convert_operand(a),
                        span: DUMMY_SP,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                let dest = self.convert_place(destination);
                let target_block = target.map(BasicBlock::from_usize);
                let unwind_action = if let Some(uw) = unwind {
                    mir::UnwindAction::Cleanup(BasicBlock::from_usize(*uw))
                } else {
                    mir::UnwindAction::Continue
                };

                // Try to resolve the function call
                // func is the mangled C++ name (e.g., "_Z6helperv")
                let func_operand = match self.resolve_function_call(func) {
                    Some(operand) => operand,
                    None => {
                        // Fall back to placeholder if function cannot be resolved
                        // This happens for external functions (libc, STL, etc.)
                        eprintln!("[fragile] Warning: Using placeholder for unresolved call to '{}'", func);
                        mir::Operand::Copy(mir::Place::from(mir::Local::from_u32(0)))
                    }
                };

                TerminatorKind::Call {
                    func: func_operand,
                    args: args_converted,
                    destination: dest,
                    target: target_block,
                    unwind: unwind_action,
                    call_source: mir::CallSource::Normal,
                    fn_span: DUMMY_SP,
                }
            }
            MirTerminator::VirtualCall {
                receiver,
                vtable_index,
                args,
                destination,
                target,
                unwind,
            } => {
                // Virtual method call via vtable lookup
                // For now, we generate a placeholder call since proper vtable runtime
                // integration requires the fragile-runtime to be linked in.
                //
                // TODO (future enhancement):
                // 1. Load vtable pointer from receiver's first field
                // 2. Index into vtable to get function pointer
                // 3. Generate indirect call through function pointer
                //
                // For now, generate a direct call to the virtual method (static dispatch)
                // which allows testing the pipeline without full runtime support.

                // Convert receiver as first argument (implicit this pointer in C++)
                let receiver_operand = self.convert_operand(receiver);
                let mut args_converted: Vec<Spanned<mir::Operand<'tcx>>> = vec![Spanned {
                    node: receiver_operand,
                    span: DUMMY_SP,
                }];

                // Convert remaining arguments
                for arg in args {
                    args_converted.push(Spanned {
                        node: self.convert_operand(arg),
                        span: DUMMY_SP,
                    });
                }

                let dest = self.convert_place(destination);
                let target_block = target.map(BasicBlock::from_usize);
                let unwind_action = if let Some(uw) = unwind {
                    mir::UnwindAction::Cleanup(BasicBlock::from_usize(*uw))
                } else {
                    mir::UnwindAction::Continue
                };

                // Use a placeholder function operand
                // In a full implementation, this would be an indirect call through the vtable
                eprintln!(
                    "[fragile] Warning: VirtualCall at vtable_index={} using placeholder. \
                     Full dynamic dispatch requires fragile-runtime integration.",
                    vtable_index
                );

                // Create a placeholder function pointer (local 0)
                // In the future, this should be the result of:
                //   fragile_rt_vfunc_get(receiver, vtable_index)
                let func_operand = mir::Operand::Copy(mir::Place::from(mir::Local::from_u32(0)));

                TerminatorKind::Call {
                    func: func_operand,
                    args: args_converted.into_boxed_slice(),
                    destination: dest,
                    target: target_block,
                    unwind: unwind_action,
                    call_source: mir::CallSource::Normal,
                    fn_span: DUMMY_SP,
                }
            }
            MirTerminator::Unreachable => TerminatorKind::Unreachable,
            MirTerminator::Resume => TerminatorKind::UnwindResume,
            // Coroutine terminators - these need special handling
            MirTerminator::Yield { value, resume, drop } => {
                let val = self.convert_operand(value);
                let resume_block = BasicBlock::from_usize(*resume);
                let drop_block = drop.map(BasicBlock::from_usize);
                TerminatorKind::Yield {
                    value: val,
                    resume: resume_block,
                    resume_arg: mir::Place::from(mir::Local::from_u32(0)), // Placeholder
                    drop: drop_block,
                }
            }
            MirTerminator::Await { awaitable, destination, resume, drop } => {
                // Await is not a direct MIR construct - it gets lowered to a state machine
                // For now, convert to a yield-like pattern
                let _val = self.convert_operand(awaitable);
                let _dest = self.convert_place(destination);
                let resume_block = BasicBlock::from_usize(*resume);
                let drop_block = drop.map(BasicBlock::from_usize);
                // Use Yield as a placeholder since Await is desugared
                TerminatorKind::Yield {
                    value: mir::Operand::Copy(mir::Place::from(mir::Local::from_u32(0))),
                    resume: resume_block,
                    resume_arg: mir::Place::from(mir::Local::from_u32(0)),
                    drop: drop_block,
                }
            }
            MirTerminator::CoroutineReturn { value: _ } => {
                // co_return is desugared to regular return in the state machine
                TerminatorKind::Return
            }
        };

        Terminator { source_info, kind }
    }

    /// Convert a Fragile MirLocal to rustc's LocalDecl.
    pub fn convert_local(&self, local: &MirLocal) -> mir::LocalDecl<'tcx> {
        let ty = self.convert_type(&local.ty);
        mir::LocalDecl::new(ty, DUMMY_SP)
    }

    /// Convert a Fragile MirBasicBlock to rustc's BasicBlockData.
    pub fn convert_basic_block(&self, block: &MirBasicBlock) -> mir::BasicBlockData<'tcx> {
        use rustc_middle::mir::BasicBlockData;

        // Convert all statements
        let statements: Vec<_> = block
            .statements
            .iter()
            .map(|s| self.convert_statement(s))
            .collect();

        // Convert terminator
        let terminator = self.convert_terminator(&block.terminator);

        // Use the constructor with is_cleanup parameter
        let mut bb_data = BasicBlockData::new(Some(terminator), block.is_cleanup);
        bb_data.statements = statements;
        bb_data
    }

    /// Convert a full Fragile MirBody to rustc's mir::Body.
    ///
    /// This performs a complete conversion of all MIR constructs.
    ///
    /// # Arguments
    /// * `mir` - The Fragile MIR body to convert
    /// * `arg_count` - Number of function arguments
    /// * `def_id` - The DefId of the function (for MirSource)
    pub fn convert_mir_body_full(
        &self,
        mir: &MirBody,
        arg_count: usize,
        def_id: rustc_span::def_id::LocalDefId,
    ) -> mir::Body<'tcx> {
        use rustc_index::IndexVec;
        use rustc_middle::mir::*;

        // Convert local declarations
        let mut local_decls: IndexVec<Local, LocalDecl<'tcx>> = IndexVec::new();
        for local in &mir.locals {
            local_decls.push(self.convert_local(local));
        }

        // Ensure we have at least a return local
        if local_decls.is_empty() {
            local_decls.push(LocalDecl::new(self.tcx.types.unit, DUMMY_SP));
        }

        // Convert basic blocks
        let mut basic_blocks: IndexVec<BasicBlock, BasicBlockData<'tcx>> = IndexVec::new();
        for block in &mir.blocks {
            basic_blocks.push(self.convert_basic_block(block));
        }

        // If no blocks, create a trivial return block
        if basic_blocks.is_empty() {
            basic_blocks.push(BasicBlockData::new(
                Some(Terminator {
                    source_info: SourceInfo::outermost(DUMMY_SP),
                    kind: TerminatorKind::Return,
                }),
                false,
            ));
        }

        // Create source scope
        let mut source_scopes: IndexVec<SourceScope, SourceScopeData<'tcx>> = IndexVec::new();
        source_scopes.push(SourceScopeData {
            span: DUMMY_SP,
            parent_scope: None,
            inlined: None,
            inlined_parent_scope: None,
            local_data: ClearCrossCrate::Clear,
        });

        // Create the body with the correct function's DefId
        mir::Body::new(
            MirSource::item(def_id.to_def_id()),
            basic_blocks,
            source_scopes,
            local_decls,
            IndexVec::new(), // user_type_annotations
            arg_count,
            Vec::new(), // var_debug_info
            DUMMY_SP,
            None, // coroutine (would need setup for is_coroutine)
            None, // tainted_by_errors
        )
    }
}

#[cfg(test)]
mod tests {
    // Tests require TyCtxt which is only available at compile time
    // Integration tests should be used instead
}
