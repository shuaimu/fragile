//! Custom rustc driver with MIR injection for the Fragile polyglot compiler.
//!
//! This crate provides a custom rustc driver that:
//! - Overrides the `mir_built` query to inject MIR from C++ sources
//! - Overrides the `mir_borrowck` query to skip borrow checking for C++ code
//! - Uses rustc's standard codegen pipeline
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐
//! │   Rust Source   │    │   C++ Source    │
//! └────────┬────────┘    └────────┬────────┘
//!          │                      │
//!          ▼                      ▼
//! ┌─────────────────┐    ┌─────────────────┐
//! │  rustc frontend │    │ fragile-clang   │
//! └────────┬────────┘    └────────┬────────┘
//!          │                      │
//!          │    ┌─────────────────┘
//!          │    │ MIR bodies
//!          ▼    ▼
//! ┌──────────────────────────────────────────┐
//! │        fragile-rustc-driver              │
//! │  ┌────────────────────────────────────┐  │
//! │  │ Query Override: mir_built          │  │
//! │  │ - Rust DefId → normal rustc MIR    │  │
//! │  │ - C++ DefId → injected MIR         │  │
//! │  └────────────────────────────────────┘  │
//! │  ┌────────────────────────────────────┐  │
//! │  │ Query Override: mir_borrowck       │  │
//! │  │ - Rust DefId → normal borrow check │  │
//! │  │ - C++ DefId → skip (unsafe)        │  │
//! │  └────────────────────────────────────┘  │
//! └──────────────────────────────────────────┘
//!                      │
//!                      ▼
//!              ┌───────────────┐
//!              │ rustc codegen │
//!              └───────────────┘
//!                      │
//!                      ▼
//!              ┌───────────────┐
//!              │    Binary     │
//!              └───────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use fragile_rustc_driver::FragileDriver;
//! use fragile_clang::compile_cpp_file;
//!
//! // Parse C++ files
//! let cpp_module = compile_cpp_file("utils.cpp")?;
//!
//! // Create driver with injected C++ MIR
//! let driver = FragileDriver::new()
//!     .with_cpp_module(cpp_module)
//!     .build();
//!
//! // Run compilation
//! driver.run(&["main.rs", "-o", "output"])?;
//! ```

mod driver;
mod queries;
mod stubs;

pub use driver::FragileDriver;
pub use queries::CppMirRegistry;
pub use stubs::generate_rust_stubs;

use miette::Result;
use std::path::Path;

/// Compile a mixed Rust/C++ project.
///
/// This is the main entry point for the Fragile compiler.
pub fn compile(
    rust_files: &[&Path],
    cpp_files: &[&Path],
    output: &Path,
) -> Result<()> {
    // Step 1: Parse all C++ files
    let mut cpp_modules = Vec::new();
    for cpp_file in cpp_files {
        let module = fragile_clang::compile_cpp_file(cpp_file)?;
        cpp_modules.push(module);
    }

    // Step 2: Generate Rust stubs for C++ declarations
    let stubs = generate_rust_stubs(&cpp_modules);

    // Step 3: Create the driver with injected MIR
    let driver = FragileDriver::new();
    for module in &cpp_modules {
        driver.register_cpp_module(module);
    }

    // Step 4: Run rustc with our custom driver
    driver.compile(rust_files, &stubs, output)
}

/// Configuration for the Fragile compiler.
#[derive(Debug, Clone)]
pub struct CompileConfig {
    /// Rust source files
    pub rust_files: Vec<std::path::PathBuf>,
    /// C++ source files
    pub cpp_files: Vec<std::path::PathBuf>,
    /// Output file path
    pub output: std::path::PathBuf,
    /// Include directories for C++ compilation
    pub cpp_include_dirs: Vec<std::path::PathBuf>,
    /// C++ preprocessor definitions
    pub cpp_defines: Vec<String>,
    /// Optimization level (0-3)
    pub opt_level: u8,
    /// Enable debug info
    pub debug_info: bool,
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            rust_files: Vec::new(),
            cpp_files: Vec::new(),
            output: std::path::PathBuf::from("a.out"),
            cpp_include_dirs: Vec::new(),
            cpp_defines: Vec::new(),
            opt_level: 0,
            debug_info: true,
        }
    }
}

impl CompileConfig {
    /// Create a new compile configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a Rust source file.
    pub fn rust_file(mut self, path: impl AsRef<Path>) -> Self {
        self.rust_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Add a C++ source file.
    pub fn cpp_file(mut self, path: impl AsRef<Path>) -> Self {
        self.cpp_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Set the output file path.
    pub fn output(mut self, path: impl AsRef<Path>) -> Self {
        self.output = path.as_ref().to_path_buf();
        self
    }

    /// Add a C++ include directory.
    pub fn cpp_include_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.cpp_include_dirs.push(path.as_ref().to_path_buf());
        self
    }

    /// Add a C++ preprocessor definition.
    pub fn cpp_define(mut self, define: impl Into<String>) -> Self {
        self.cpp_defines.push(define.into());
        self
    }

    /// Set the optimization level.
    pub fn optimization(mut self, level: u8) -> Self {
        self.opt_level = level.min(3);
        self
    }

    /// Enable or disable debug info.
    pub fn debug(mut self, enabled: bool) -> Self {
        self.debug_info = enabled;
        self
    }
}
