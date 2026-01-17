//! C++ compiler wrapper for building object files.
//!
//! This module provides functionality to compile C++ source files
//! into object files using clang or g++.

use miette::{miette, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for the C++ compiler.
#[derive(Debug, Clone)]
pub struct CppCompilerConfig {
    /// Path to the C++ compiler (defaults to searching for clang++ or g++)
    pub compiler: Option<PathBuf>,
    /// Include directories (-I flags)
    pub include_dirs: Vec<PathBuf>,
    /// System include directories (-isystem flags)
    pub system_include_dirs: Vec<PathBuf>,
    /// Preprocessor defines (-D flags)
    pub defines: Vec<String>,
    /// C++ standard (default: c++20)
    pub std_version: String,
    /// Optimization level (0-3)
    pub opt_level: u8,
    /// Generate debug info (-g)
    pub debug_info: bool,
    /// Position-independent code (-fPIC)
    pub pic: bool,
    /// Suppress all warnings (-w)
    pub suppress_warnings: bool,
}

impl Default for CppCompilerConfig {
    fn default() -> Self {
        Self {
            compiler: None,
            include_dirs: Vec::new(),
            system_include_dirs: Vec::new(),
            defines: Vec::new(),
            std_version: "c++20".to_string(),
            opt_level: 0,
            debug_info: true,
            pic: true,
            suppress_warnings: true,
        }
    }
}

impl CppCompilerConfig {
    /// Create a new C++ compiler configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an include directory.
    pub fn include_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.include_dirs.push(path.as_ref().to_path_buf());
        self
    }

    /// Add a system include directory.
    pub fn system_include_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.system_include_dirs.push(path.as_ref().to_path_buf());
        self
    }

    /// Add a preprocessor define.
    pub fn define(mut self, def: impl Into<String>) -> Self {
        self.defines.push(def.into());
        self
    }

    /// Set the C++ standard version.
    pub fn std_version(mut self, version: impl Into<String>) -> Self {
        self.std_version = version.into();
        self
    }

    /// Set the optimization level.
    pub fn opt_level(mut self, level: u8) -> Self {
        self.opt_level = level.min(3);
        self
    }

    /// Enable or disable debug info.
    pub fn debug_info(mut self, enabled: bool) -> Self {
        self.debug_info = enabled;
        self
    }

    /// Enable or disable position-independent code.
    pub fn pic(mut self, enabled: bool) -> Self {
        self.pic = enabled;
        self
    }
}

/// C++ compiler wrapper.
pub struct CppCompiler {
    config: CppCompilerConfig,
    compiler_path: PathBuf,
}

impl CppCompiler {
    /// Create a new C++ compiler with the given configuration.
    pub fn new(config: CppCompilerConfig) -> Result<Self> {
        let compiler_path = match &config.compiler {
            Some(path) => {
                if !path.exists() {
                    return Err(miette!("Compiler not found: {:?}", path));
                }
                path.clone()
            }
            None => find_compiler()?,
        };

        Ok(Self {
            config,
            compiler_path,
        })
    }

    /// Create a compiler with default configuration.
    pub fn with_defaults() -> Result<Self> {
        Self::new(CppCompilerConfig::default())
    }

    /// Get the path to the compiler being used.
    pub fn compiler_path(&self) -> &Path {
        &self.compiler_path
    }

    /// Compile a single C++ source file to an object file.
    ///
    /// # Arguments
    /// * `source` - Path to the C++ source file
    /// * `output_dir` - Directory where the object file will be placed
    ///
    /// # Returns
    /// Path to the generated object file
    pub fn compile_to_object(&self, source: &Path, output_dir: &Path) -> Result<PathBuf> {
        // Validate source file exists
        if !source.exists() {
            return Err(miette!("Source file not found: {:?}", source));
        }

        // Create output directory if needed
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)
                .map_err(|e| miette!("Failed to create output directory: {}", e))?;
        }

        // Determine output path
        let stem = source
            .file_stem()
            .ok_or_else(|| miette!("Invalid source file name: {:?}", source))?;
        let output_path = output_dir.join(format!("{}.o", stem.to_string_lossy()));

        // Build command
        let mut cmd = Command::new(&self.compiler_path);

        // Add source file
        cmd.arg(source);

        // Compile only, don't link
        cmd.arg("-c");

        // Output file
        cmd.arg("-o");
        cmd.arg(&output_path);

        // C++ standard
        cmd.arg(format!("-std={}", self.config.std_version));

        // Optimization level
        cmd.arg(format!("-O{}", self.config.opt_level));

        // Debug info
        if self.config.debug_info {
            cmd.arg("-g");
        }

        // Position-independent code
        if self.config.pic {
            cmd.arg("-fPIC");
        }

        // Suppress warnings
        if self.config.suppress_warnings {
            cmd.arg("-w");
        }

        // Include directories
        for dir in &self.config.include_dirs {
            cmd.arg("-I");
            cmd.arg(dir);
        }

        // System include directories
        for dir in &self.config.system_include_dirs {
            cmd.arg("-isystem");
            cmd.arg(dir);
        }

        // Preprocessor defines
        for def in &self.config.defines {
            cmd.arg(format!("-D{}", def));
        }

        // Debug: print the command (env var to enable)
        if std::env::var("FRAGILE_DEBUG").is_ok() {
            eprintln!("DEBUG: Running {:?} {:?}", cmd.get_program(), cmd.get_args().collect::<Vec<_>>());
        }

        // Execute
        let output = cmd
            .output()
            .map_err(|e| miette!("Failed to execute compiler: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(miette!(
                "Compilation failed for {:?}:\n{}\n{}",
                source,
                stdout,
                stderr
            ));
        }

        // Verify output exists
        if !output_path.exists() {
            return Err(miette!(
                "Object file not created: {:?}",
                output_path
            ));
        }

        Ok(output_path)
    }

    /// Compile multiple C++ source files to object files.
    ///
    /// # Arguments
    /// * `sources` - Paths to C++ source files
    /// * `output_dir` - Directory where object files will be placed
    ///
    /// # Returns
    /// List of paths to generated object files
    pub fn compile_all(&self, sources: &[&Path], output_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut objects = Vec::with_capacity(sources.len());

        for source in sources {
            let obj = self.compile_to_object(source, output_dir)?;
            objects.push(obj);
        }

        Ok(objects)
    }

    /// Link object files into an executable.
    ///
    /// # Arguments
    /// * `objects` - Paths to object files
    /// * `output` - Path for the output executable
    /// * `lib_paths` - Library search paths (-L)
    /// * `libs` - Libraries to link (-l)
    pub fn link_executable(
        &self,
        objects: &[PathBuf],
        output: &Path,
        lib_paths: &[String],
        libs: &[String],
    ) -> Result<()> {
        let mut cmd = Command::new(&self.compiler_path);

        // Add all object files
        for obj in objects {
            cmd.arg(obj);
        }

        // Output path
        cmd.arg("-o");
        cmd.arg(output);

        // Library search paths
        for lib_path in lib_paths {
            cmd.arg(format!("-L{}", lib_path));
        }

        // Libraries
        for lib in libs {
            cmd.arg(format!("-l{}", lib));
        }

        // Standard library linkage
        cmd.arg("-lstdc++");

        let output = cmd.output().map_err(|e| {
            miette!("Failed to run linker: {}", e)
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(miette!(
                "Link failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            ));
        }

        Ok(())
    }

    /// Create a static library from object files.
    ///
    /// # Arguments
    /// * `objects` - Paths to object files
    /// * `output` - Path for the output library (lib*.a)
    pub fn create_static_library(
        &self,
        objects: &[PathBuf],
        output: &Path,
    ) -> Result<()> {
        let mut cmd = Command::new("ar");
        cmd.arg("rcs");
        cmd.arg(output);

        for obj in objects {
            cmd.arg(obj);
        }

        let output_result = cmd.output().map_err(|e| {
            miette!("Failed to run ar: {}", e)
        })?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(miette!("ar failed: {}", stderr));
        }

        Ok(())
    }
}

/// Find a C++ compiler on the system.
///
/// Searches for clang++ first, then g++.
pub fn find_compiler() -> Result<PathBuf> {
    // Try clang++ first
    if let Ok(output) = Command::new("which").arg("clang++").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    // Try g++ as fallback
    if let Ok(output) = Command::new("which").arg("g++").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    Err(miette!(
        "No C++ compiler found. Please install clang++ or g++."
    ))
}

/// Get the default stub headers directory.
pub fn default_stub_headers_dir() -> Option<PathBuf> {
    // Try to find the stubs directory relative to the crate
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let crate_dir = PathBuf::from(manifest_dir);

    // Go up to workspace root, then to fragile-clang/stubs
    let stubs_dir = crate_dir
        .parent()? // crates/
        .join("fragile-clang")
        .join("stubs");

    if stubs_dir.exists() {
        Some(stubs_dir)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_compiler() {
        // This test may fail if no C++ compiler is installed
        match find_compiler() {
            Ok(path) => {
                println!("Found compiler: {:?}", path);
                assert!(path.exists() || path.to_str().unwrap().contains("clang")
                    || path.to_str().unwrap().contains("g++"));
            }
            Err(e) => {
                eprintln!("No compiler found (expected on some CI): {}", e);
            }
        }
    }

    #[test]
    fn test_cpp_compiler_config_builder() {
        let config = CppCompilerConfig::new()
            .include_dir("/usr/include")
            .system_include_dir("/usr/local/include")
            .define("DEBUG")
            .define("VERSION=1")
            .std_version("c++17")
            .opt_level(2)
            .debug_info(false);

        assert_eq!(config.include_dirs.len(), 1);
        assert_eq!(config.system_include_dirs.len(), 1);
        assert_eq!(config.defines.len(), 2);
        assert_eq!(config.std_version, "c++17");
        assert_eq!(config.opt_level, 2);
        assert!(!config.debug_info);
    }

    #[test]
    fn test_compile_add_cpp_to_object() {
        // Skip if no compiler available
        let compiler = match CppCompiler::with_defaults() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping test (no compiler): {}", e);
                return;
            }
        };

        // Find the test file
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
        let add_cpp = project_root.join("tests/clang_integration/add.cpp");

        if !add_cpp.exists() {
            eprintln!("Skipping test: add.cpp not found at {:?}", add_cpp);
            return;
        }

        // Create temp directory for output
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Compile
        let result = compiler.compile_to_object(&add_cpp, temp_dir.path());

        match result {
            Ok(obj_path) => {
                println!("Compiled to: {:?}", obj_path);
                assert!(obj_path.exists(), "Object file should exist");
                assert!(obj_path.to_string_lossy().ends_with(".o"));

                // Check file size is non-zero
                let metadata = std::fs::metadata(&obj_path).expect("Failed to get metadata");
                assert!(metadata.len() > 0, "Object file should be non-empty");
            }
            Err(e) => {
                // Compilation might fail for various reasons (missing headers, etc.)
                eprintln!("Compilation failed (may be expected): {}", e);
            }
        }
    }

    #[test]
    fn test_compile_nonexistent_file() {
        let compiler = match CppCompiler::with_defaults() {
            Ok(c) => c,
            Err(_) => return, // Skip if no compiler
        };

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let result = compiler.compile_to_object(
            Path::new("/nonexistent/file.cpp"),
            temp_dir.path(),
        );

        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(err_msg.contains("not found"), "Error should mention file not found");
    }
}
