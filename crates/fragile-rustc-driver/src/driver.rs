//! Custom rustc driver implementation.

use crate::queries::CppMirRegistry;
use fragile_clang::CppModule;
#[cfg(not(feature = "rustc-integration"))]
use miette::miette;
use miette::Result;
use std::path::Path;
use std::sync::Arc;

/// Custom rustc driver that injects C++ MIR.
///
/// This driver wraps the standard rustc driver and overrides queries
/// to inject MIR from C++ sources and skip borrow checking for C++ code.
pub struct FragileDriver {
    /// Registry of C++ MIR bodies
    mir_registry: Arc<CppMirRegistry>,
}

impl FragileDriver {
    /// Create a new Fragile driver.
    pub fn new() -> Self {
        Self {
            mir_registry: Arc::new(CppMirRegistry::new()),
        }
    }

    /// Register a C++ module for MIR injection.
    pub fn register_cpp_module(&self, module: &CppModule) {
        self.mir_registry.register_module(module);
    }

    /// Get a reference to the MIR registry.
    pub fn mir_registry(&self) -> &Arc<CppMirRegistry> {
        &self.mir_registry
    }

    /// Compile Rust files with injected C++ MIR.
    ///
    /// # Arguments
    /// * `rust_files` - Rust source files to compile
    /// * `cpp_stubs` - Generated Rust stubs for C++ declarations
    /// * `output` - Output binary path
    pub fn compile(
        &self,
        rust_files: &[&Path],
        cpp_stubs: &str,
        output: &Path,
    ) -> Result<()> {
        // For now, this is a stub implementation.
        // The real implementation requires nightly Rust with rustc-dev.
        //
        // The implementation would:
        // 1. Create a temporary file with cpp_stubs
        // 2. Set up rustc callbacks to override queries
        // 3. Run rustc with the custom callbacks

        #[cfg(feature = "rustc-integration")]
        {
            self.compile_with_rustc(rust_files, cpp_stubs, output)
        }

        #[cfg(not(feature = "rustc-integration"))]
        {
            // Stub implementation for testing without rustc-dev
            eprintln!("Warning: rustc-integration feature not enabled");
            eprintln!("  Rust files: {:?}", rust_files);
            eprintln!("  Output: {:?}", output);
            eprintln!("  C++ stubs generated: {} bytes", cpp_stubs.len());
            eprintln!("  C++ functions registered: {}", self.mir_registry.function_count());

            // Write stubs to file for debugging
            let stubs_path = output.with_extension("cpp_stubs.rs");
            std::fs::write(&stubs_path, cpp_stubs)
                .map_err(|e| miette!("Failed to write stubs: {}", e))?;
            eprintln!("  Stubs written to: {:?}", stubs_path);

            Ok(())
        }
    }

    /// Compile using actual rustc (requires nightly + rustc-dev).
    #[cfg(feature = "rustc-integration")]
    fn compile_with_rustc(
        &self,
        rust_files: &[&Path],
        cpp_stubs: &str,
        output: &Path,
    ) -> Result<()> {
        crate::rustc_integration::run_rustc(
            rust_files,
            cpp_stubs,
            output,
            Arc::clone(&self.mir_registry),
        )
    }
}

impl Default for FragileDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::generate_rust_stubs;
    use fragile_clang::{CppFunction, CppType, MirBody};
    use std::path::Path;

    #[test]
    fn test_driver_creation() {
        let driver = FragileDriver::new();
        assert_eq!(driver.mir_registry.function_count(), 0);
    }

    /// End-to-end test: Parse add.cpp, generate stubs, register module
    #[test]
    fn test_end_to_end_add_cpp() {
        // Path to the test file
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
        let add_cpp = project_root.join("tests/clang_integration/add.cpp");

        // Check if test file exists
        if !add_cpp.exists() {
            eprintln!("Skipping test: add.cpp not found at {:?}", add_cpp);
            return;
        }

        // Parse the C++ file
        let module = fragile_clang::compile_cpp_file(&add_cpp)
            .expect("Failed to parse add.cpp");

        // Verify the module contains the add function
        assert!(module.functions.iter().any(|f| f.display_name == "add"),
            "Expected to find 'add' function in module");

        // Find the add function and verify its signature
        let add_func = module.functions.iter()
            .find(|f| f.display_name == "add")
            .expect("add function not found");

        assert_eq!(add_func.params.len(), 2, "add should have 2 parameters");
        assert!(add_func.return_type.is_integral() == Some(true), "add should return integral type");

        // Register the module with the driver
        let driver = FragileDriver::new();
        driver.register_cpp_module(&module);

        // Verify registration
        assert!(driver.mir_registry.function_count() > 0,
            "Module should have registered at least one function");

        // Generate Rust stubs
        let stubs = generate_rust_stubs(&[module]);
        assert!(stubs.contains("extern"), "Stubs should contain extern block");
        assert!(stubs.contains("add"), "Stubs should reference add function");

        println!("Generated stubs:\n{}", stubs);
    }

    #[test]
    fn test_driver_register_module() {
        let driver = FragileDriver::new();

        // Create a mock C++ module with an add function
        let mut module = CppModule::new();
        module.functions.push(CppFunction {
            mangled_name: "_Z3addii".to_string(),
            display_name: "add".to_string(),
            namespace: Vec::new(),
            params: vec![
                ("a".to_string(), CppType::int()),
                ("b".to_string(), CppType::int()),
            ],
            return_type: CppType::int(),
            is_noexcept: false,
            mir_body: MirBody::new(),
        });

        driver.register_cpp_module(&module);

        assert_eq!(driver.mir_registry.function_count(), 1);
        assert!(driver.mir_registry.is_cpp_function("_Z3addii"));
    }

    #[test]
    fn test_driver_pipeline_without_rustc() {
        // Test the full pipeline without actually invoking rustc
        let driver = FragileDriver::new();

        // Create a C++ module
        let mut module = CppModule::new();
        module.functions.push(CppFunction {
            mangled_name: "_Z3addii".to_string(),
            display_name: "add".to_string(),
            namespace: Vec::new(),
            params: vec![
                ("a".to_string(), CppType::int()),
                ("b".to_string(), CppType::int()),
            ],
            return_type: CppType::int(),
            is_noexcept: false,
            mir_body: MirBody::new(),
        });

        // Register the module
        driver.register_cpp_module(&module);

        // Generate stubs
        let stubs = generate_rust_stubs(&[module]);
        assert!(stubs.contains("extern \"C\""));
        assert!(stubs.contains("add"));

        // The compile step would require rustc-integration feature
        // For now, verify the registry is populated
        assert_eq!(driver.mir_registry.function_count(), 1);
        let names = driver.mir_registry.function_names();
        assert!(names.contains(&"_Z3addii".to_string()));
    }
}
