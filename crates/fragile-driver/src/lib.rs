use fragile_common::{Language, SourceFile, SourceMap, SymbolInterner};
use fragile_hir::{Module, Program};
use miette::Result;
use std::path::Path;
use std::sync::Arc;

/// Compiler driver that orchestrates the compilation pipeline.
pub struct Driver {
    source_map: SourceMap,
    interner: SymbolInterner,
}

impl Driver {
    pub fn new() -> Self {
        Self {
            source_map: SourceMap::new(),
            interner: SymbolInterner::new(),
        }
    }

    /// Add a source file to the compilation.
    pub fn add_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| miette::miette!("Failed to read {}: {}", path.display(), e))?;

        self.source_map.add_file(path, content)?;
        Ok(())
    }

    /// Parse all added source files into a program.
    pub fn parse(&self) -> Result<Program> {
        let mut program = Program::new();

        // Get all source files
        // Note: In a real implementation, we'd iterate over the source map properly
        // For now, this is a simplified version

        Ok(program)
    }

    /// Parse a single source file.
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Module> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| miette::miette!("Failed to read {}: {}", path.display(), e))?;

        let source_id = self.source_map.add_file(path, content)?;
        let source = self
            .source_map
            .get(source_id)
            .ok_or_else(|| miette::miette!("Source file not found"))?;

        self.parse_source(&source)
    }

    /// Parse a source file based on its language.
    fn parse_source(&self, source: &SourceFile) -> Result<Module> {
        match source.language {
            Language::Rust => fragile_frontend_rust::parse_file(source, &self.interner),
            Language::Cpp => fragile_frontend_cpp::parse_file(source, &self.interner),
            Language::Go => fragile_frontend_go::parse_file(source, &self.interner),
        }
    }

    /// Compile a source file to an object file.
    pub fn compile_to_object(&self, source_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<()> {
        let module = self.parse_file(&source_path)?;
        fragile_codegen::compile_module(&module, &self.interner, output_path.as_ref())?;
        Ok(())
    }

    /// Compile a source file and return LLVM IR.
    pub fn compile_to_ir(&self, source_path: impl AsRef<Path>) -> Result<String> {
        let module = self.parse_file(&source_path)?;
        fragile_codegen::compile_to_ir(&module, &self.interner)
    }

    /// Get a reference to the symbol interner.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Get a reference to the source map.
    pub fn source_map(&self) -> &SourceMap {
        &self.source_map
    }
}

impl Default for Driver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_rust_file() {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(file, "fn main() {{ let x = 42; }}").unwrap();

        let driver = Driver::new();
        let module = driver.parse_file(file.path()).unwrap();

        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_cpp_file() {
        let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
        writeln!(file, "int main() {{ int x = 42; return 0; }}").unwrap();

        let driver = Driver::new();
        let module = driver.parse_file(file.path()).unwrap();

        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_go_file() {
        let mut file = NamedTempFile::with_suffix(".go").unwrap();
        writeln!(file, "package main\n\nfunc main() {{ x := 42 }}").unwrap();

        let driver = Driver::new();
        let module = driver.parse_file(file.path()).unwrap();

        assert_eq!(module.items.len(), 1);
    }
}
