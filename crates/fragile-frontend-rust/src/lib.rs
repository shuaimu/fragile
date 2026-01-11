mod parser;
mod lower;

pub use parser::parse;
pub use lower::lower;

use fragile_common::{SourceFile, SourceMap, SymbolInterner};
use fragile_hir::Module;
use miette::Result;
use std::sync::Arc;

/// Parse a Rust source file into HIR.
pub fn parse_file(
    source: &SourceFile,
    interner: &SymbolInterner,
) -> Result<Module> {
    let tree = parser::parse(&source.content)?;
    let module = lower::lower(tree, source, interner)?;
    Ok(module)
}
