//! LibTooling-based AST parser.
//!
//! Wraps `fragile_ast_exporter::export_ast_with_options` to produce an `AstContext`
//! from a C++ source file, providing access to fully instantiated template bodies.

use fragile_ast_exporter::{clang_ast::AstContext, export_ast_with_options};
use miette::{miette, Result};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Parser that uses LibTooling for full template instantiation access.
pub struct LibToolingParser {
    /// Directory containing compile_commands.json
    compile_commands_dir: Option<String>,
    /// Extra compiler arguments
    extra_args: Vec<String>,
    /// Skip system-header declarations while exporting AST nodes.
    skip_system_headers: bool,
}

impl LibToolingParser {
    fn json_escape(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for ch in value.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                '\u{08}' => escaped.push_str("\\b"),
                '\u{0C}' => escaped.push_str("\\f"),
                c if c < '\u{20}' => {
                    escaped.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => escaped.push(c),
            }
        }
        escaped
    }

    fn compile_commands_with_arguments(
        directory: &Path,
        file_path: &Path,
        arguments: &[String],
    ) -> String {
        let mut args_json = String::new();
        for (idx, arg) in arguments.iter().enumerate() {
            if idx > 0 {
                args_json.push_str(", ");
            }
            args_json.push('"');
            args_json.push_str(&Self::json_escape(arg));
            args_json.push('"');
        }

        let directory = Self::json_escape(&directory.display().to_string());
        let file = Self::json_escape(&file_path.display().to_string());

        format!(
            r#"[
  {{
    "directory": "{}",
    "arguments": [{}],
    "file": "{}"
  }}
]"#,
            directory, args_json, file
        )
    }

    /// Create a new LibTooling parser.
    pub fn new() -> Self {
        Self {
            compile_commands_dir: None,
            extra_args: Vec::new(),
            skip_system_headers: false,
        }
    }

    /// Set the directory containing compile_commands.json.
    pub fn with_compile_commands_dir(mut self, dir: &str) -> Self {
        self.compile_commands_dir = Some(dir.to_string());
        self
    }

    /// Add extra compiler arguments.
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }

    /// Skip declarations originating from system headers when exporting AST.
    pub fn with_skip_system_headers(mut self, skip: bool) -> Self {
        self.skip_system_headers = skip;
        self
    }

    /// Detect the path to vendored libc++ headers.
    /// Looks for vendor/llvm-project/libcxx/include/.
    fn detect_vendored_libcxx_path() -> Option<String> {
        // Try relative paths from the current working directory
        let candidates = [
            "vendor/llvm-project/libcxx/include",
            "../vendor/llvm-project/libcxx/include",
            "../../vendor/llvm-project/libcxx/include",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                return std::fs::canonicalize(candidate)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
            }
        }

        // Try from FRAGILE_ROOT environment variable
        if let Ok(root) = std::env::var("FRAGILE_ROOT") {
            let path = Path::new(&root).join("vendor/llvm-project/libcxx/include");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }

        None
    }

    /// Detect the path to vendored libc++ config (contains __config_site).
    fn detect_vendored_libcxx_config_path() -> Option<String> {
        let candidates = [
            "vendor/libcxx-config",
            "../vendor/libcxx-config",
            "../../vendor/libcxx-config",
        ];

        for candidate in candidates {
            if Path::new(candidate).exists() {
                return std::fs::canonicalize(candidate)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
            }
        }

        if let Ok(root) = std::env::var("FRAGILE_ROOT") {
            let path = Path::new(&root).join("vendor/libcxx-config");
            if path.exists() {
                return Some(path.to_string_lossy().to_string());
            }
        }

        None
    }

    /// Parse a file and return the LibTooling AST context.
    ///
    /// This provides access to the full AST including template instantiations
    /// with concrete types and actual method bodies.
    pub fn parse_file(&self, path: &Path) -> Result<AstContext> {
        // Use the configured compile dir (or source parent) as the logical
        // command directory, but always materialize compile_commands.json in a
        // fresh temp directory so stale project-local databases cannot leak.
        let compile_working_dir = self
            .compile_commands_dir
            .as_ref()
            .map(|s| Path::new(s).to_path_buf())
            .unwrap_or_else(|| {
                path.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| Path::new(".").to_path_buf())
            });

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let compile_db_dir = std::env::temp_dir().join(format!(
            "fragile_ast_exporter_ccdb_{}_{}",
            std::process::id(),
            stamp
        ));
        std::fs::create_dir_all(&compile_db_dir)
            .map_err(|e| miette!("Failed to create temp compile_commands dir: {}", e))?;

        // Build compiler arguments: combine user-specified args with vendored libc++ paths.
        let mut all_extra_args: Vec<String> = self.extra_args.clone();

        // Auto-detect vendored libc++ paths (same as ClangParser)
        // This ensures LibTooling uses the same headers as libclang
        if let Some(libcxx_config_path) = Self::detect_vendored_libcxx_config_path() {
            if let Some(libcxx_include_path) = Self::detect_vendored_libcxx_path() {
                // Add libc++ flags
                all_extra_args.push("-stdlib=libc++".to_string());
                all_extra_args.push("-nostdinc++".to_string());
                // Config path first for __config_site
                all_extra_args.push(format!("-isystem{}", libcxx_config_path));
                all_extra_args.push(format!("-isystem{}", libcxx_include_path));
            }
        }

        // Keep parser behavior compatible with GCC-based builds for legacy
        // varargs call patterns that Clang diagnoses as hard errors.
        if !all_extra_args
            .iter()
            .any(|arg| arg == "-Wno-non-pod-varargs")
        {
            all_extra_args.push("-Wno-non-pod-varargs".to_string());
        }

        let mut compile_arguments = Vec::with_capacity(all_extra_args.len() + 5);
        compile_arguments.push("clang++".to_string());
        compile_arguments.extend(all_extra_args);
        compile_arguments.push("-c".to_string());
        compile_arguments.push(path.display().to_string());
        compile_arguments.push("-o".to_string());
        compile_arguments.push("/dev/null".to_string());

        let compile_commands_path = compile_db_dir.join("compile_commands.json");
        let compile_commands =
            Self::compile_commands_with_arguments(&compile_working_dir, path, &compile_arguments);
        std::fs::write(&compile_commands_path, compile_commands)
            .map_err(|e| miette!("Failed to create compile_commands.json: {}", e))?;

        let parse_result =
            export_ast_with_options(path, &compile_db_dir, &[], false, self.skip_system_headers)
                .map_err(|e| miette!("LibTooling parse failed: {}", e));

        let _ = std::fs::remove_file(&compile_commands_path);
        let _ = std::fs::remove_dir_all(&compile_db_dir);

        parse_result
    }

}

impl Default for LibToolingParser {
    fn default() -> Self {
        Self::new()
    }
}
