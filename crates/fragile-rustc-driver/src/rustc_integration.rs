//! Rustc integration module - implements Callbacks trait and query overrides.
//!
//! This module is only compiled when the `rustc-integration` feature is enabled.
//! It requires nightly Rust with the rustc-dev component.

#![cfg(feature = "rustc-integration")]

// These extern crate declarations are needed for rustc crates
// They're found via the sysroot, not Cargo.toml
extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use crate::mir_convert::MirConvertCtx;
use crate::queries::CppMirRegistry;
use miette::{miette, Result};
use rustc_data_structures::steal::Steal;
use rustc_driver::Compilation;
use rustc_hir as hir;
use rustc_interface::interface::{Compiler, Config};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LocalDefId;
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ============================================================================
// Thread-Local Storage for C++ Registry
// ============================================================================

thread_local! {
    /// Thread-local storage for the C++ MIR registry.
    ///
    /// This is necessary because `override_queries` takes a function pointer,
    /// not a closure, so we cannot capture state directly. Instead, we store
    /// the registry in TLS and access it from the query override functions.
    static CPP_REGISTRY: RefCell<Option<Arc<CppMirRegistry>>> = const { RefCell::new(None) };

    /// Thread-local storage for C++ function names as a HashSet for quick lookup.
    static CPP_FUNCTION_NAMES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    /// Thread-local storage for the original mir_built provider for fallback.
    static ORIG_MIR_BUILT: RefCell<Option<for<'tcx> fn(TyCtxt<'tcx>, LocalDefId) -> &'tcx Steal<rustc_middle::mir::Body<'tcx>>>> = const { RefCell::new(None) };
}

/// Set the C++ registry for the current thread.
fn set_cpp_registry(registry: Arc<CppMirRegistry>, function_names: Vec<String>) {
    CPP_REGISTRY.with(|r| {
        *r.borrow_mut() = Some(registry);
    });
    CPP_FUNCTION_NAMES.with(|names| {
        *names.borrow_mut() = function_names.into_iter().collect();
    });
}

/// Clear the C++ registry for the current thread.
fn clear_cpp_registry() {
    CPP_REGISTRY.with(|r| {
        *r.borrow_mut() = None;
    });
    CPP_FUNCTION_NAMES.with(|names| {
        names.borrow_mut().clear();
    });
    ORIG_MIR_BUILT.with(|r| {
        *r.borrow_mut() = None;
    });
}

// ============================================================================
// DefId to Function Name Mapping (Task 2.3.3.1)
// ============================================================================

/// Get the link_name (symbol_name) for a function from its codegen attributes.
///
/// This function looks at the `#[link_name = "..."]` attribute on extern functions
/// which is stored in the `symbol_name` field of CodegenFnAttrs.
pub fn get_cpp_link_name<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> Option<String> {
    // Get codegen function attributes which includes symbol_name (from #[link_name])
    let attrs = tcx.codegen_fn_attrs(def_id);

    // symbol_name contains the value from #[link_name = "..."] or #[export_name = "..."]
    attrs.symbol_name.map(|sym| sym.to_string())
}

/// Check if a DefId is a C++ function by checking its link_name against the registry.
pub fn is_cpp_function<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    cpp_function_names: &HashSet<String>,
) -> bool {
    if let Some(link_name) = get_cpp_link_name(tcx, def_id) {
        cpp_function_names.contains(&link_name)
    } else {
        false
    }
}

/// Collect all DefIds that correspond to C++ functions.
///
/// This scans all items in the crate and identifies those with `#[export_name]`
/// or `#[link_name]` attributes that match registered C++ functions.
///
/// Supports both:
/// - Regular Rust functions with `#[export_name]` (MIR injection method)
/// - Foreign items with `#[link_name]` (legacy method)
pub fn collect_cpp_def_ids<'tcx>(
    tcx: TyCtxt<'tcx>,
    cpp_function_names: &HashSet<String>,
) -> Vec<(LocalDefId, String)> {
    let mut result = Vec::new();

    // Get the HIR crate
    let hir_crate = tcx.hir_crate(());

    // Iterate over all items in the crate through module tree
    for owner in hir_crate.owners.iter() {
        if let hir::MaybeOwner::Owner(owner_info) = owner {
            match owner_info.node() {
                // Check regular items (functions with #[export_name])
                hir::OwnerNode::Item(item) => {
                    if let hir::ItemKind::Fn { .. } = item.kind {
                        let def_id = item.owner_id.def_id;
                        if let Some(export_name) = get_cpp_link_name(tcx, def_id) {
                            if cpp_function_names.contains(&export_name) {
                                result.push((def_id, export_name));
                            }
                        }
                    }
                }
                // Check foreign items (extern functions with #[link_name])
                hir::OwnerNode::ForeignItem(foreign_item) => {
                    let def_id = foreign_item.owner_id.def_id;
                    if let Some(link_name) = get_cpp_link_name(tcx, def_id) {
                        if cpp_function_names.contains(&link_name) {
                            result.push((def_id, link_name));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}

// ============================================================================
// Custom mir_built Query Provider
// ============================================================================

/// Custom mir_built query provider that injects C++ MIR.
///
/// This function is used as a fn pointer (not closure) for the query override.
/// All state access goes through thread-local storage.
fn fragile_mir_built<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
) -> &'tcx Steal<rustc_middle::mir::Body<'tcx>> {
    // Check if this is a C++ function by looking at link_name
    let cpp_function_name = CPP_FUNCTION_NAMES.with(|names| {
        let names_set = names.borrow();
        get_cpp_link_name(tcx, def_id).filter(|name| names_set.contains(name))
    });

    if let Some(link_name) = cpp_function_name {
        eprintln!("[fragile] mir_built called for C++ function: {}", link_name);

        // Get the MIR body from the registry
        let mir_body = CPP_REGISTRY.with(|r| {
            r.borrow().as_ref().and_then(|reg| reg.get_mir(&link_name))
        });

        if let Some(cpp_mir) = mir_body {
            eprintln!("[fragile] Found C++ MIR body for {}, converting...", link_name);

            // Convert the fragile MIR to rustc MIR
            let ctx = MirConvertCtx::new(tcx);

            // Count arguments from the MIR (locals 1..=arg_count are arguments)
            // For now, use a simple heuristic: count non-return locals as potential args
            let arg_count = cpp_mir.locals.len().saturating_sub(1);
            let rustc_body = ctx.convert_mir_body_full(&cpp_mir, arg_count);

            eprintln!(
                "[fragile] Converted MIR body for {} ({} locals, {} blocks)",
                link_name,
                rustc_body.local_decls.len(),
                rustc_body.basic_blocks.len()
            );

            // Arena-allocate and wrap in Steal
            return tcx.arena.alloc(Steal::new(rustc_body));
        } else {
            eprintln!("[fragile] No MIR body found for {}, using fallback", link_name);
        }
    }

    // Fall back to original rustc mir_built for non-C++ functions
    ORIG_MIR_BUILT.with(|r| {
        let orig = r.borrow();
        let orig_fn = orig.expect("[fragile] ORIG_MIR_BUILT not set - this is a bug");
        orig_fn(tcx, def_id)
    })
}

// ============================================================================
// Query Override Callback (Tasks 2.3.3.4 and 2.3.4)
// ============================================================================

/// Query override callback function.
///
/// This is the actual function that overrides rustc queries. It's a separate function
/// (not a closure) because `override_queries` requires a function pointer.
///
/// # Architecture for Full Implementation
///
/// To enable full MIR injection and borrow check bypass:
///
/// 1. **Thread-Local Storage**: Store the C++ function registry in TLS:
///    ```rust,ignore
///    thread_local! {
///        static CPP_REGISTRY: RefCell<Option<Arc<CppMirRegistry>>> = RefCell::new(None);
///    }
///    ```
///
/// 2. **mir_built Override**:
///    ```rust,ignore
///    let orig_mir_built = providers.queries.mir_built;
///    providers.queries.mir_built = |tcx, def_id| {
///        if is_cpp_function(tcx, def_id) {
///            // Return converted C++ MIR
///            return MirConvertCtx::new(tcx).convert_mir_body(...);
///        }
///        orig_mir_built(tcx, def_id)
///    };
///    ```
///
/// 3. **mir_borrowck Override** (bypass for C++ code):
///    ```rust,ignore
///    let orig_mir_borrowck = providers.queries.mir_borrowck;
///    providers.queries.mir_borrowck = |tcx, def_id| {
///        if is_cpp_function(tcx, def_id) {
///            // Skip borrow checking - return empty result
///            return tcx.arena.alloc(BorrowCheckResult::default());
///        }
///        orig_mir_borrowck(tcx, def_id)
///    };
///    ```
fn override_queries_callback(
    _session: &rustc_session::Session,
    providers: &mut rustc_middle::util::Providers,
) {
    // Log that the callback was invoked
    eprintln!("[fragile] Query override callback invoked");

    // Check if we have a C++ registry
    let (has_registry, function_count) = CPP_REGISTRY.with(|r| {
        let borrow = r.borrow();
        match borrow.as_ref() {
            Some(reg) => (true, reg.function_count()),
            None => (false, 0),
        }
    });

    if !has_registry {
        eprintln!("[fragile] No C++ registry set, using default providers");
        return;
    }

    let names_count = CPP_FUNCTION_NAMES.with(|n| n.borrow().len());
    eprintln!(
        "[fragile] Query override active with {} registered C++ functions ({} names)",
        function_count,
        names_count
    );

    // Store original providers in TLS for fallback from the fn pointer
    ORIG_MIR_BUILT.with(|r| {
        *r.borrow_mut() = Some(providers.queries.mir_built);
    });

    // Override mir_built query to inject C++ MIR
    // This must be a fn pointer (not closure) so it cannot capture state.
    // All state access goes through TLS.
    providers.queries.mir_built = fragile_mir_built;

    eprintln!("[fragile] Query override installed for {} C++ functions", function_count);
}

// ============================================================================
// FragileCallbacks Implementation
// ============================================================================

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
    ///
    /// # Query Override Architecture
    ///
    /// The `override_queries` callback receives a function pointer (not closure), so we cannot
    /// capture state from `self`. The architecture for full MIR injection would require:
    ///
    /// 1. Store C++ function registry in a thread-local or global static
    /// 2. Override `mir_built` query to check if DefId matches a C++ function
    /// 3. If match: return pre-computed MIR from `MirConvertCtx`
    /// 4. If no match: delegate to original `mir_built` query
    ///
    /// Current status: Infrastructure ready, full wiring deferred until MIR conversion is complete.
    fn config(&mut self, config: &mut Config) {
        // Log registered C++ functions
        let function_count = self.cpp_function_names.len();
        eprintln!(
            "[fragile] Configuring rustc with {} registered C++ functions",
            function_count
        );

        // Set up thread-local storage with the C++ registry
        // This allows the query override function (which takes fn pointer, not closure)
        // to access the registry.
        set_cpp_registry(
            Arc::clone(&self.mir_registry),
            self.cpp_function_names.clone(),
        );

        // Set up query overrides
        config.override_queries = Some(override_queries_callback);
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
    ///
    /// This callback demonstrates the connection between our C++ function detection
    /// infrastructure and the rustc compilation pipeline.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        eprintln!("[fragile] Type analysis complete");

        // Convert function names to HashSet for lookup
        let cpp_names: HashSet<String> = self.cpp_function_names.iter().cloned().collect();

        // Scan for C++ functions in the compiled crate
        let cpp_def_ids = collect_cpp_def_ids(tcx, &cpp_names);

        if !cpp_def_ids.is_empty() {
            eprintln!(
                "[fragile] Found {} C++ function(s) in compiled crate:",
                cpp_def_ids.len()
            );
            for (def_id, link_name) in &cpp_def_ids {
                eprintln!("  - {} (DefId: {:?})", link_name, def_id);
            }
        } else if !cpp_names.is_empty() {
            eprintln!(
                "[fragile] No matching C++ functions found (registered: {})",
                cpp_names.len()
            );
        }

        Compilation::Continue
    }
}

// ============================================================================
// Run rustc function
// ============================================================================

/// Run rustc with the Fragile callbacks.
///
/// This function:
/// 1. Creates a temporary file with the C++ stubs
/// 2. Sets up the rustc arguments
/// 3. Runs rustc with our custom callbacks
///
/// # Arguments
/// * `rust_files` - Rust source files to compile
/// * `cpp_stubs` - Generated Rust stubs for C++ declarations
/// * `output` - Output binary path
/// * `mir_registry` - Registry of C++ MIR bodies
/// * `cpp_objects` - Optional C++ object files to link
/// * `link_cpp_runtime` - Whether to link C++ standard library
pub fn run_rustc(
    rust_files: &[&Path],
    cpp_stubs: &str,
    output: &Path,
    mir_registry: Arc<CppMirRegistry>,
) -> Result<()> {
    run_rustc_with_objects(rust_files, cpp_stubs, output, mir_registry, &[], true)
}

/// Run rustc with the Fragile callbacks and C++ object files.
///
/// This is the full version that supports linking with C++ object files.
pub fn run_rustc_with_objects(
    rust_files: &[&Path],
    cpp_stubs: &str,
    output: &Path,
    mir_registry: Arc<CppMirRegistry>,
    cpp_objects: &[PathBuf],
    link_cpp_runtime: bool,
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

    // Add C++ object files as linker arguments
    for obj in cpp_objects {
        args.push("-C".to_string());
        args.push(format!("link-arg={}", obj.display()));
    }

    // Link C++ runtime library if requested
    if link_cpp_runtime && !cpp_objects.is_empty() {
        // Try to detect whether to use libstdc++ or libc++
        // Default to libstdc++ as it's more common on Linux
        args.push("-C".to_string());
        args.push("link-arg=-lstdc++".to_string());
    }

    // Create our callbacks
    let mut callbacks = FragileCallbacks::new(mir_registry).with_stubs_path(stubs_path.clone());

    eprintln!("[fragile] Running rustc with args: {:?}", args);
    eprintln!("[fragile] C++ functions registered: {}", callbacks.cpp_function_names.len());
    if !cpp_objects.is_empty() {
        eprintln!("[fragile] C++ objects to link: {:?}", cpp_objects);
    }

    // Run rustc with our callbacks
    let result = rustc_driver::catch_fatal_errors(|| {
        rustc_driver::run_compiler(&args, &mut callbacks)
    });

    // Always clear the TLS registry after compilation
    clear_cpp_registry();

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

    #[test]
    fn test_tls_registry_lifecycle() {
        // Test that TLS registry can be set and cleared
        let registry = Arc::new(CppMirRegistry::new());
        let function_names = vec!["test_func".to_string(), "another_func".to_string()];

        // Initially no registry
        let has_registry_before = CPP_REGISTRY.with(|r| r.borrow().is_some());
        assert!(!has_registry_before, "Registry should not be set initially");

        // Set the registry
        set_cpp_registry(Arc::clone(&registry), function_names.clone());

        // Verify registry is set
        let has_registry_after = CPP_REGISTRY.with(|r| r.borrow().is_some());
        assert!(has_registry_after, "Registry should be set after set_cpp_registry");

        // Verify function names are set
        let names_count = CPP_FUNCTION_NAMES.with(|n| n.borrow().len());
        assert_eq!(names_count, 2, "Should have 2 function names");

        // Verify specific names are present
        let has_test_func = CPP_FUNCTION_NAMES.with(|n| n.borrow().contains("test_func"));
        assert!(has_test_func, "Should contain test_func");

        // Clear the registry
        clear_cpp_registry();

        // Verify registry is cleared
        let has_registry_final = CPP_REGISTRY.with(|r| r.borrow().is_some());
        assert!(!has_registry_final, "Registry should be cleared");

        let names_count_final = CPP_FUNCTION_NAMES.with(|n| n.borrow().len());
        assert_eq!(names_count_final, 0, "Function names should be cleared");
    }

    #[test]
    fn test_callbacks_with_functions() {
        // Create a registry with a function
        let registry = Arc::new(CppMirRegistry::new());

        // Register a mock module
        use fragile_clang::{CppFunction, CppModule, CppType, MirBody};
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
        registry.register_module(&module);

        // Create callbacks
        let callbacks = FragileCallbacks::new(Arc::clone(&registry));

        assert_eq!(callbacks.cpp_function_names.len(), 1);
        assert!(callbacks.cpp_function_names.contains(&"_Z3addii".to_string()));
    }
}
