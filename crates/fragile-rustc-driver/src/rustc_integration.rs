//! Rustc integration module - implements Callbacks trait and query overrides.
//!
//! This module is only compiled when the `rustc-integration` feature is enabled.
//! It requires nightly Rust with the rustc-dev component.

#![cfg(feature = "rustc-integration")]

// These extern crate declarations are needed for rustc crates
// They're found via the sysroot, not Cargo.toml
extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use crate::queries::CppMirRegistry;
use miette::{miette, Result};
use rustc_driver::Compilation;
use rustc_interface::interface::{Compiler, Config};
use rustc_middle::ty::TyCtxt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Callbacks implementation for Fragile compiler.
///
/// This struct implements `rustc_driver::Callbacks` to intercept the compilation
/// process and inject custom behavior like C++ MIR injection and borrow check bypass.
pub struct FragileCallbacks {
    /// Registry of C++ MIR bodies to inject
    pub mir_registry: Arc<CppMirRegistry>,
    /// Path to the temporary stubs file
    pub stubs_path: Option<PathBuf>,
    /// Names of C++ functions (for identifying during query override)
    pub cpp_function_names: Vec<String>,
}

impl FragileCallbacks {
    /// Create new callbacks with the given MIR registry.
    pub fn new(mir_registry: Arc<CppMirRegistry>) -> Self {
        let cpp_function_names = mir_registry.function_names();
        Self {
            mir_registry,
            stubs_path: None,
            cpp_function_names,
        }
    }

    /// Set the path to the stubs file.
    pub fn with_stubs_path(mut self, path: PathBuf) -> Self {
        self.stubs_path = Some(path);
        self
    }
}

impl rustc_driver::Callbacks for FragileCallbacks {
    /// Called before creating the compiler instance.
    ///
    /// This is where we set up query overrides for MIR injection and borrow check bypass.
    fn config(&mut self, config: &mut Config) {
        // Clone data needed for the closure (closures need 'static lifetime)
        let _cpp_function_names = self.cpp_function_names.clone();
        let _mir_registry = Arc::clone(&self.mir_registry);

        // Set up query overrides
        // Note: The actual MIR injection is complex and requires converting our MIR format
        // to rustc's internal MIR representation. For now, we set up the infrastructure.
        config.override_queries = Some(|_session, providers| {
            // The providers struct has a `queries` field that contains the actual query providers
            // We would override `providers.queries.mir_built` here
            //
            // Example (full implementation would require MIR format conversion):
            // let orig_mir_built = providers.queries.mir_built;
            // providers.queries.mir_built = |tcx, key| {
            //     // Check if this DefId corresponds to a C++ function
            //     let def_id = key.to_def_id();
            //     if is_cpp_function(tcx, def_id, &cpp_function_names) {
            //         // Return pre-computed MIR from registry
            //         convert_cpp_mir_to_rustc(tcx, mir_registry.get_mir(&name))
            //     } else {
            //         // Fall back to normal rustc pipeline
            //         orig_mir_built(tcx, key)
            //     }
            // };

            // For now, we just log that we're being called
            // The actual MIR format conversion is the hard part and will be
            // implemented incrementally
            eprintln!("[fragile] Query override infrastructure installed");
            let _ = providers; // silence unused warning
        });
    }

    /// Called after the crate root has been parsed.
    fn after_crate_root_parsing(
        &mut self,
        _compiler: &Compiler,
        _krate: &mut rustc_ast::Crate,
    ) -> Compilation {
        eprintln!("[fragile] Crate root parsed");
        Compilation::Continue
    }

    /// Called after macro expansion.
    fn after_expansion<'tcx>(
        &mut self,
        _compiler: &Compiler,
        _tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        eprintln!("[fragile] Macro expansion complete");
        Compilation::Continue
    }

    /// Called after type analysis.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        eprintln!("[fragile] Type analysis complete");
        // Here we could inspect the MIR that was generated
        // and verify our injected MIR is present
        let _ = tcx;
        Compilation::Continue
    }
}

/// Run rustc with the Fragile callbacks.
///
/// This function:
/// 1. Creates a temporary file with the C++ stubs
/// 2. Sets up the rustc arguments
/// 3. Runs rustc with our custom callbacks
pub fn run_rustc(
    rust_files: &[&Path],
    cpp_stubs: &str,
    output: &Path,
    mir_registry: Arc<CppMirRegistry>,
) -> Result<()> {
    // Create a temporary file for the stubs
    let temp_dir = tempfile::tempdir().map_err(|e| miette!("Failed to create temp dir: {}", e))?;
    let stubs_path = temp_dir.path().join("cpp_stubs.rs");

    // Write the stubs to the temp file
    let mut stubs_file =
        std::fs::File::create(&stubs_path).map_err(|e| miette!("Failed to create stubs file: {}", e))?;
    stubs_file
        .write_all(cpp_stubs.as_bytes())
        .map_err(|e| miette!("Failed to write stubs: {}", e))?;

    // Build the rustc arguments
    let mut args = vec![
        "rustc".to_string(), // argv[0] is the program name
        "--edition=2021".to_string(),
        "-o".to_string(),
        output.to_string_lossy().to_string(),
    ];

    // Add all Rust files
    for rust_file in rust_files {
        args.push(rust_file.to_string_lossy().to_string());
    }

    // Add the stubs file as extern crate or include
    // For now, we'll include it as part of the crate
    args.push("--extern".to_string());
    args.push(format!("cpp_stubs={}", stubs_path.display()));

    // Create our callbacks
    let mut callbacks = FragileCallbacks::new(mir_registry).with_stubs_path(stubs_path.clone());

    eprintln!("[fragile] Running rustc with args: {:?}", args);
    eprintln!("[fragile] C++ functions registered: {}", callbacks.cpp_function_names.len());

    // Run rustc with our callbacks
    let result = rustc_driver::catch_fatal_errors(|| {
        rustc_driver::run_compiler(&args, &mut callbacks)
    });

    match result {
        Ok(()) => {
            eprintln!("[fragile] Compilation successful");
            Ok(())
        }
        Err(_) => Err(miette!("Compilation failed or fatal error occurred")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callbacks_creation() {
        let registry = Arc::new(CppMirRegistry::new());
        let callbacks = FragileCallbacks::new(registry);
        assert!(callbacks.cpp_function_names.is_empty());
    }
}
