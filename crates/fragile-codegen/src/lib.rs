mod codegen;

pub use codegen::CodeGenerator;

use fragile_common::SymbolInterner;
use fragile_hir::Module;
use inkwell::context::Context;
use miette::Result;
use std::path::Path;

/// Compile a HIR module to an object file.
pub fn compile_module(
    module: &Module,
    interner: &SymbolInterner,
    output_path: &Path,
) -> Result<()> {
    let context = Context::create();
    let codegen = CodeGenerator::new(&context, interner);

    let llvm_module = codegen.compile_module(module)?;

    // Write to object file
    codegen.write_object_file(&llvm_module, output_path)?;

    Ok(())
}

/// Compile a HIR module to LLVM IR (for debugging).
pub fn compile_to_ir(
    module: &Module,
    interner: &SymbolInterner,
) -> Result<String> {
    let context = Context::create();
    let codegen = CodeGenerator::new(&context, interner);

    let llvm_module = codegen.compile_module(module)?;

    Ok(llvm_module.print_to_string().to_string())
}
