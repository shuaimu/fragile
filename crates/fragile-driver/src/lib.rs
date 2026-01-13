use fragile_common::{Language, SourceFile, SourceMap, SymbolInterner};
use fragile_hir::{Item, ItemKind, Module, Program};
use miette::Result;
use std::path::{Path, PathBuf};

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

        let mut module = self.parse_source(&source)?;

        // Resolve external modules
        let base_dir = path.parent().unwrap_or(Path::new("."));
        self.resolve_modules(&mut module.items, base_dir, source.language)?;

        Ok(module)
    }

    /// Resolve external module declarations by loading their files.
    fn resolve_modules(&self, items: &mut Vec<Item>, base_dir: &Path, lang: Language) -> Result<()> {
        for item in items.iter_mut() {
            if let ItemKind::Mod(mod_def) = &mut item.kind {
                if mod_def.items.is_none() {
                    // External module - need to load from file
                    let mod_name = self.interner.resolve(mod_def.name);

                    // Try mod_name.rs first, then mod_name/mod.rs
                    let file_path = self.find_module_file(base_dir, &mod_name, lang)?;

                    // Parse the module file
                    let content = std::fs::read_to_string(&file_path)
                        .map_err(|e| miette::miette!("Failed to read module {}: {}", mod_name, e))?;

                    let source_id = self.source_map.add_file(&file_path, content)?;
                    let source = self
                        .source_map
                        .get(source_id)
                        .ok_or_else(|| miette::miette!("Source file not found"))?;

                    let nested_module = self.parse_source(&source)?;

                    // Recursively resolve nested modules
                    let mut nested_items = nested_module.items;
                    let nested_base = file_path.parent().unwrap_or(Path::new("."));
                    self.resolve_modules(&mut nested_items, nested_base, lang)?;

                    mod_def.items = Some(nested_items);
                } else if let Some(ref mut nested_items) = mod_def.items {
                    // Inline module - recursively resolve nested modules
                    self.resolve_modules(nested_items, base_dir, lang)?;
                }
            }
        }
        Ok(())
    }

    /// Find the file for a module name.
    fn find_module_file(&self, base_dir: &Path, mod_name: &str, lang: Language) -> Result<PathBuf> {
        let ext = match lang {
            Language::Rust => "rs",
            Language::Cpp => "cpp",
            Language::Go => "go",
        };

        // Try mod_name.ext first
        let direct_path = base_dir.join(format!("{}.{}", mod_name, ext));
        if direct_path.exists() {
            return Ok(direct_path);
        }

        // Try mod_name/mod.ext
        let dir_path = base_dir.join(mod_name).join(format!("mod.{}", ext));
        if dir_path.exists() {
            return Ok(dir_path);
        }

        Err(miette::miette!(
            "Module '{}' not found. Tried:\n  - {}\n  - {}",
            mod_name,
            direct_path.display(),
            dir_path.display()
        ))
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
