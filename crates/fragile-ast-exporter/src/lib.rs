//! C++ AST exporter using Clang LibTooling
//!
//! This crate provides access to Clang's full AST including template instantiations
//! via LibTooling. The AST is exported to CBOR format for consumption by the Rust
//! transpiler.

use serde_cbor::Value;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr, CString};
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::slice;
use std::sync::{Mutex, OnceLock};

pub mod clang_ast;

// Include generated bindings
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(clippy::all)]
mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// Re-export FFI types for use by dependent crates
// Some types may not be currently used internally but are part of the public API
#[allow(unused_imports)]
pub use ffi::{
    ASTEntryTag, AccessSpecifier, BinaryOperatorKind, CastKind, UnaryExprOrTypeTrait,
    UnaryOperatorKind,
};

// Re-export serde_cbor::Value for external use
pub use serde_cbor::Value as CborValue;

/// Get the Clang version string
pub fn get_clang_version() -> Option<String> {
    unsafe {
        let s = CStr::from_ptr(ffi::clang_version());
        s.to_str().ok().map(|s| s.to_string())
    }
}

/// Get the major Clang version number
pub fn get_clang_major_version() -> Option<u32> {
    get_clang_version().and_then(|v| v.split('.').next()?.parse().ok())
}

/// Export C++ AST from a source file
///
/// # Arguments
/// * `file_path` - Path to the C++ source file
/// * `compile_commands_dir` - Directory containing compile_commands.json
/// * `extra_args` - Additional compiler arguments
/// * `debug` - Enable debug output
///
/// # Returns
/// The parsed AST context or an error
pub fn export_ast(
    file_path: &Path,
    compile_commands_dir: &Path,
    extra_args: &[&str],
    debug: bool,
) -> Result<clang_ast::AstContext, Error> {
    export_ast_with_options(file_path, compile_commands_dir, extra_args, debug, false)
}

/// Export C++ AST from a source file with exporter options.
pub fn export_ast_with_options(
    file_path: &Path,
    compile_commands_dir: &Path,
    extra_args: &[&str],
    debug: bool,
    skip_system_headers: bool,
) -> Result<clang_ast::AstContext, Error> {
    let cbor_data = export_ast_cbor_with_options(
        file_path,
        compile_commands_dir,
        extra_args,
        debug,
        skip_system_headers,
    )?;

    // Deserialize CBOR
    let items: Value = serde_cbor::from_slice(&cbor_data)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("CBOR parse error: {}", e)))?;

    clang_ast::process(items).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("AST processing error: {}", e),
        )
    })
}

/// Export C++ AST as raw CBOR bytes
pub fn export_ast_cbor(
    file_path: &Path,
    compile_commands_dir: &Path,
    extra_args: &[&str],
    debug: bool,
) -> Result<Vec<u8>, Error> {
    export_ast_cbor_with_options(file_path, compile_commands_dir, extra_args, debug, false)
}

/// Export C++ AST as raw CBOR bytes with exporter options.
pub fn export_ast_cbor_with_options(
    file_path: &Path,
    compile_commands_dir: &Path,
    extra_args: &[&str],
    debug: bool,
    skip_system_headers: bool,
) -> Result<Vec<u8>, Error> {
    let results = get_ast_cbors(
        file_path,
        compile_commands_dir,
        extra_args,
        debug,
        skip_system_headers,
    )?;

    select_ast_cbor_for_source(file_path, results)
}

fn select_ast_cbor_for_source(
    file_path: &Path,
    mut results: HashMap<String, Vec<u8>>,
) -> Result<Vec<u8>, Error> {
    if results.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, "No AST data returned"));
    }

    let requested = file_path.to_string_lossy().to_string();
    if let Some(bytes) = results.remove(&requested) {
        return Ok(bytes);
    }

    if let Ok(canonical) = file_path.canonicalize() {
        let canonical_key = canonical.to_string_lossy().to_string();
        if let Some(bytes) = results.remove(&canonical_key) {
            return Ok(bytes);
        }
    }

    let requested_basename = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let mut basename_matches: Vec<String> = results
        .keys()
        .filter(|key| {
            Path::new(key.as_str())
                .file_name()
                .and_then(|s| s.to_str())
                .map(|name| name == requested_basename)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    if basename_matches.len() == 1 {
        if let Some(bytes) = results.remove(&basename_matches.pop().unwrap()) {
            return Ok(bytes);
        }
    }

    if results.len() == 1 {
        return Ok(results.into_values().next().unwrap());
    }

    let available: Vec<String> = results.keys().cloned().collect();
    Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "AST export returned {} files and none matched `{}`; available keys: {}",
            available.len(),
            requested,
            available.join(", ")
        ),
    ))
}

fn get_ast_cbors(
    file_path: &Path,
    compile_commands_dir: &Path,
    extra_args: &[&str],
    debug: bool,
    skip_system_headers: bool,
) -> Result<HashMap<String, Vec<u8>>, Error> {
    static AST_EXPORTER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = AST_EXPORTER_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .map_err(|_| Error::other("AST exporter lock poisoned"))?;

    let mut result_code: c_int = 0;

    // Build arguments for the AST exporter
    let mut args_owned =
        vec![
            CString::new("fragile-ast-exporter").unwrap(),
            CString::new(
                file_path
                    .to_str()
                    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid file path"))?,
            )
            .unwrap(),
            CString::new("-p").unwrap(),
            CString::new(compile_commands_dir.to_str().ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "Invalid compile commands path")
            })?)
            .unwrap(),
        ];
    if skip_system_headers {
        args_owned.push(CString::new("-skip-system-headers").unwrap());
    }

    // Add extra arguments
    for &arg in extra_args {
        args_owned.push(CString::new(format!("-extra-arg={}", arg)).unwrap());
    }

    let args_ptrs: Vec<*const c_char> = args_owned.iter().map(|s| s.as_ptr()).collect();

    let hashmap;
    unsafe {
        // Cast to the expected type (bindgen generates *mut *const for C arrays)
        let argv_ptr = args_ptrs.as_ptr() as *mut *const c_char;
        let ptr = ffi::ast_exporter(
            args_ptrs.len() as c_int,
            argv_ptr,
            if debug { 1 } else { 0 },
            &mut result_code,
        );

        if ptr.is_null() || result_code != 0 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("AST export failed with code {}", result_code),
            ));
        }

        hashmap = marshal_result(ptr);
        ffi::drop_export_result(ptr);
    }

    Ok(hashmap)
}

unsafe fn marshal_result(result: *const ffi::ExportResult) -> HashMap<String, Vec<u8>> {
    let mut output = HashMap::new();

    let n = (*result).entries as isize;
    for i in 0..n {
        let res = &*result;

        // Convert name
        let cname = CStr::from_ptr(*res.names.offset(i));
        let name = cname.to_str().unwrap_or("unknown").to_owned();

        // Convert CBOR bytes
        let csize = *res.sizes.offset(i);
        let cbytes = *res.bytes.offset(i);
        let bytes = slice::from_raw_parts(cbytes, csize);
        let v = bytes.to_vec();

        output.insert(name, v);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_clang_version() {
        let version = get_clang_version();
        assert!(version.is_some());
        println!("Clang version: {:?}", version);
    }

    #[test]
    fn test_select_ast_cbor_for_source_prefers_exact_path_match() {
        let requested = PathBuf::from("/tmp/source.cpp");
        let mut results = HashMap::new();
        results.insert("/tmp/other.cpp".to_string(), vec![1, 2, 3]);
        results.insert("/tmp/source.cpp".to_string(), vec![9, 8, 7]);

        let selected = select_ast_cbor_for_source(&requested, results)
            .expect("expected exact path match to succeed");
        assert_eq!(selected, vec![9, 8, 7]);
    }

    #[test]
    fn test_select_ast_cbor_for_source_errors_on_ambiguous_basename() {
        let requested = PathBuf::from("/tmp/source.cpp");
        let mut results = HashMap::new();
        results.insert("/a/source.cpp".to_string(), vec![1]);
        results.insert("/b/source.cpp".to_string(), vec![2]);

        let err = select_ast_cbor_for_source(&requested, results)
            .expect_err("expected ambiguous basename selection to fail");
        let msg = err.to_string();
        assert!(
            msg.contains("none matched"),
            "expected ambiguity failure message, got: {}",
            msg
        );
    }
}
