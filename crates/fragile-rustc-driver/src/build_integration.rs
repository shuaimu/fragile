//! Build system integration for FragileDriver.
//!
//! This module provides the interface between fragile-build configurations
//! and the FragileDriver compilation pipeline.

use fragile_build::{BuildConfig, CompileCommands, TargetConfig, TargetType};
use std::path::{Path, PathBuf};

/// Configuration for a compilation job derived from build config.
#[derive(Debug, Clone)]
pub struct CompilationJob {
    /// Source files to compile.
    pub sources: Vec<PathBuf>,
    /// Include directories.
    pub includes: Vec<PathBuf>,
    /// Preprocessor definitions.
    pub defines: Vec<String>,
    /// C++ standard (e.g., "c++23").
    pub std: Option<String>,
    /// Additional compiler flags.
    pub cflags: Vec<String>,
    /// Output type.
    pub output_type: OutputType,
    /// Output name.
    pub output_name: String,
    /// Libraries to link.
    pub libs: Vec<String>,
    /// Library search paths.
    pub lib_paths: Vec<PathBuf>,
}

/// Type of compilation output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    Executable,
    StaticLibrary,
    SharedLibrary,
    ObjectFile,
}

impl From<TargetType> for OutputType {
    fn from(tt: TargetType) -> Self {
        match tt {
            TargetType::Executable => OutputType::Executable,
            TargetType::StaticLibrary => OutputType::StaticLibrary,
            TargetType::SharedLibrary => OutputType::SharedLibrary,
        }
    }
}

impl CompilationJob {
    /// Create a job from a build config target.
    pub fn from_target(config: &BuildConfig, target: &TargetConfig, project_root: &Path) -> Self {
        let sources = target
            .sources
            .iter()
            .map(|s| project_root.join(s))
            .collect();

        let mut includes: Vec<PathBuf> = config
            .get_includes(target)
            .into_iter()
            .map(|s| {
                let p = PathBuf::from(&s);
                if p.is_absolute() {
                    p
                } else {
                    project_root.join(s)
                }
            })
            .collect();

        // Add project root as implicit include
        includes.insert(0, project_root.to_path_buf());

        let defines = config.get_defines(target);
        let std = config.get_std(target);
        let cflags = target.cflags.clone();

        let lib_paths = target
            .lib_paths
            .iter()
            .map(|s| {
                let p = PathBuf::from(s);
                if p.is_absolute() {
                    p
                } else {
                    project_root.join(s)
                }
            })
            .collect();

        Self {
            sources,
            includes,
            defines,
            std,
            cflags,
            output_type: target.target_type.into(),
            output_name: target.name.clone(),
            libs: target.libs.clone(),
            lib_paths,
        }
    }

    /// Create a job from a compile_commands.json entry.
    pub fn from_compile_command(
        cmd: &fragile_build::CompileCommand,
    ) -> Self {
        let sources = vec![cmd.file.clone()];
        let includes = cmd.get_includes();
        let defines = cmd.get_defines();
        let std = cmd.get_std();

        Self {
            sources,
            includes,
            defines,
            std,
            cflags: Vec::new(),
            output_type: OutputType::ObjectFile,
            output_name: cmd.file.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "output".to_string()),
            libs: Vec::new(),
            lib_paths: Vec::new(),
        }
    }

    /// Get include paths as strings for ClangParser construction.
    pub fn include_paths(&self) -> Vec<String> {
        self.includes
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// Get defines for ClangParser construction.
    pub fn defines_list(&self) -> Vec<String> {
        let mut defines = self.defines.clone();
        // Add C++ standard as a define if specified
        if let Some(std) = &self.std {
            // Common practice: define __cplusplus version based on standard
            let cpp_version = match std.as_str() {
                "c++23" | "c++2b" => "202302L",
                "c++20" | "c++2a" => "202002L",
                "c++17" => "201703L",
                "c++14" => "201402L",
                "c++11" => "201103L",
                _ => "201103L",
            };
            defines.push(format!("__cplusplus={}", cpp_version));
        }
        defines
    }

    /// Create a ClangParser configured for this job.
    /// Returns the parser configuration (include_paths, system_paths, defines).
    pub fn parser_config(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let include_paths = self.include_paths();
        let system_paths = Vec::new(); // Could be extended to support system paths
        let defines = self.defines_list();
        (include_paths, system_paths, defines)
    }
}

/// Build a target from a build configuration.
pub fn build_target(
    config: &BuildConfig,
    target_name: &str,
    project_root: &Path,
) -> Result<CompilationJob, String> {
    let target = config
        .find_target(target_name)
        .ok_or_else(|| format!("Target '{}' not found", target_name))?;

    Ok(CompilationJob::from_target(config, target, project_root))
}

/// Load and build from a fragile.toml file.
pub fn build_from_config_file(
    config_path: &Path,
    target_name: &str,
) -> Result<CompilationJob, String> {
    let config = BuildConfig::from_file(config_path)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    let project_root = config_path.parent().unwrap_or(Path::new("."));
    build_target(&config, target_name, project_root)
}

/// Load jobs from compile_commands.json.
pub fn jobs_from_compile_commands(
    compile_commands_path: &Path,
) -> Result<Vec<CompilationJob>, String> {
    let cmds = CompileCommands::from_file(compile_commands_path)
        .map_err(|e| format!("Failed to load compile_commands.json: {}", e))?;

    Ok(cmds
        .commands()
        .iter()
        .map(CompilationJob::from_compile_command)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_job_from_target() {
        let toml = r#"
[project]
name = "test"

[[target]]
name = "main"
type = "executable"
sources = ["src/main.cc"]
includes = ["include"]
defines = ["DEBUG=1"]
std = "c++23"
        "#;

        let config: BuildConfig = toml::from_str(toml).unwrap();
        let target = config.find_target("main").unwrap();
        let project_root = Path::new("/test/project");

        let job = CompilationJob::from_target(&config, target, project_root);

        assert_eq!(job.sources, vec![PathBuf::from("/test/project/src/main.cc")]);
        assert!(job.includes.contains(&PathBuf::from("/test/project/include")));
        assert_eq!(job.defines, vec!["DEBUG=1"]);
        assert_eq!(job.std, Some("c++23".to_string()));
        assert_eq!(job.output_type, OutputType::Executable);
    }

    #[test]
    fn test_compilation_job_from_compile_command() {
        let json = r#"[{
            "directory": "/build",
            "file": "/src/main.cc",
            "command": "g++ -I/usr/include -DNDEBUG -std=c++20 -c main.cc"
        }]"#;

        let cmds = CompileCommands::from_str(json).unwrap();
        let job = CompilationJob::from_compile_command(&cmds.commands()[0]);

        assert_eq!(job.sources, vec![PathBuf::from("/src/main.cc")]);
        assert_eq!(job.includes, vec![PathBuf::from("/usr/include")]);
        assert_eq!(job.defines, vec!["NDEBUG"]);
        assert_eq!(job.std, Some("c++20".to_string()));
    }
}
