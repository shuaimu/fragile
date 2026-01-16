//! Rustc integration module - implements Callbacks trait and query overrides.
//!
//! This module is only compiled when the `rustc-integration` feature is enabled.
//! It requires nightly Rust with the rustc-dev component.

#![cfg(feature = "rustc-integration")]

// These extern crate declarations are needed for rustc crates
// They're found via the sysroot, not Cargo.toml
extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use crate::mir_convert::MirConvertCtx;
use crate::queries::CppMirRegistry;
use miette::{miette, Result};
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
    static CPP_FUNCTION_NAMES: RefCell<HashSet<String>> = const { RefCell::new(HashSet::new()) };
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
/// This scans all items in the crate and identifies those with #[link_name]
/// attributes that match registered C++ functions.
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
            // Check if this is a foreign item (extern function)
            if let hir::OwnerNode::ForeignItem(foreign_item) = owner_info.node() {
                let def_id = foreign_item.owner_id.def_id;

                if let Some(link_name) = get_cpp_link_name(tcx, def_id) {
                    if cpp_function_names.contains(&link_name) {
                        result.push((def_id, link_name));
                    }
                }
            }
        }
    }

    result
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

    // Store references to original providers for potential future use
    let _orig_mir_built = providers.queries.mir_built;
    let _orig_mir_borrowck = providers.queries.mir_borrowck;

    // NOTE: Actual MIR injection is complex because:
    // 1. mir_built query signature requires returning &'tcx mir::Body<'tcx>
    // 2. The body must be arena-allocated via TyCtxt
    // 3. We need to map DefId -> function name -> MirBody -> converted mir::Body
    //
    // The current infrastructure:
    // - TLS stores the CppMirRegistry
    // - MirConvertCtx can convert MirBody to mir::Body
    // - collect_cpp_def_ids can find extern functions with matching link_name
    //
    // To fully wire up mir_built override, we need:
    // - A way to get TyCtxt in the query provider (it's passed to the query)
    // - Arena allocation for the converted body
    // - Proper handling of generic parameters
    //
    // For now, the infrastructure is in place and detection works in after_analysis.
    eprintln!("[fragile] Query override infrastructure ready for {} C++ functions", function_count);
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
