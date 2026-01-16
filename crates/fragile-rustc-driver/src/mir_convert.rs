//! MIR format conversion from Fragile's simplified MIR to rustc's internal format.
//!
//! This module provides the infrastructure for converting `fragile_clang::MirBody`
//! to `rustc_middle::mir::Body`. Due to the complexity and instability of rustc's
//! internal APIs, this is implemented incrementally.
//!
//! Current status: Basic infrastructure with trivial body generation.

#![cfg(feature = "rustc-integration")]

extern crate rustc_index;
extern crate rustc_middle;
extern crate rustc_span;

use fragile_clang::{CppType, MirBody};
use rustc_middle::mir;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::DUMMY_SP;

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
    /// This is a simplified conversion for basic types.
    /// Full type conversion handles complex types.
    pub fn convert_type(&self, cpp_type: &CppType) -> Ty<'tcx> {
        match cpp_type {
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

            // Pointers - convert to raw pointers
            CppType::Pointer { .. } => {
                // For simplicity, use *const () for all pointers
                // Full implementation needs to track pointee type
                Ty::new_ptr(
                    self.tcx,
                    self.tcx.types.unit,
                    rustc_middle::ty::Mutability::Not,
                )
            }

            // References - convert to references
            CppType::Reference { is_rvalue, .. } => {
                if *is_rvalue {
                    // Rvalue reference -> &mut
                    Ty::new_mut_ref(self.tcx, self.tcx.lifetimes.re_erased, self.tcx.types.unit)
                } else {
                    // Lvalue reference -> &
                    Ty::new_imm_ref(self.tcx, self.tcx.lifetimes.re_erased, self.tcx.types.unit)
                }
            }

            // All other types - use unit as placeholder
            CppType::Array { .. }
            | CppType::Named(_)
            | CppType::Function { .. }
            | CppType::TemplateParam { .. }
            | CppType::DependentType { .. }
            | CppType::ParameterPack { .. } => {
                // Use unit as placeholder for complex types
                // These need full type resolution
                self.tcx.types.unit
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests require TyCtxt which is only available at compile time
    // Integration tests should be used instead
}
