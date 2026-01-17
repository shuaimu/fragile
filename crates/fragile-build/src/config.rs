//! Build configuration types (fragile.toml format).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Root build configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Project metadata.
    pub project: ProjectConfig,

    /// Build targets (executables, libraries).
    #[serde(rename = "target", default)]
    pub targets: Vec<TargetConfig>,

    /// Global compiler settings.
    #[serde(default)]
    pub compiler: CompilerConfig,
}

/// Project metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: String,

    /// Project version (optional).
    #[serde(default)]
    pub version: Option<String>,

    /// Project root directory (default: config file directory).
    #[serde(default)]
    pub root: Option<PathBuf>,
}

/// Target configuration (executable or library).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Target name.
    pub name: String,

    /// Target type (executable, static_library, shared_library).
    #[serde(rename = "type")]
    pub target_type: TargetType,

    /// Source files.
    #[serde(default)]
    pub sources: Vec<String>,

    /// Include directories.
    #[serde(default)]
    pub includes: Vec<String>,

    /// Preprocessor definitions.
    #[serde(default)]
    pub defines: Vec<String>,

    /// C++ standard (e.g., "c++17", "c++20", "c++23").
    #[serde(default)]
    pub std: Option<String>,

    /// Additional compiler flags.
    #[serde(default)]
    pub cflags: Vec<String>,

    /// Libraries to link against.
    #[serde(default)]
    pub libs: Vec<String>,

    /// Library search paths.
    #[serde(default)]
    pub lib_paths: Vec<String>,

    /// Dependencies on other targets.
    #[serde(default)]
    pub deps: Vec<String>,
}

/// Target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    /// Executable binary.
    Executable,
    /// Static library (.a).
    StaticLibrary,
    /// Shared library (.so).
    SharedLibrary,
}

/// Global compiler configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// Default C++ standard.
    #[serde(default)]
    pub std: Option<String>,

    /// Global include directories.
    #[serde(default)]
    pub includes: Vec<String>,

    /// Global preprocessor definitions.
    #[serde(default)]
    pub defines: Vec<String>,

    /// Global compiler flags.
    #[serde(default)]
    pub cflags: Vec<String>,
}

impl BuildConfig {
    /// Load configuration from a TOML file.
    pub fn from_file(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: BuildConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Find a target by name.
    pub fn find_target(&self, name: &str) -> Option<&TargetConfig> {
        self.targets.iter().find(|t| t.name == name)
    }

    /// Get all include directories for a target (including global).
    pub fn get_includes(&self, target: &TargetConfig) -> Vec<String> {
        let mut includes = self.compiler.includes.clone();
        includes.extend(target.includes.clone());
        includes
    }

    /// Get all defines for a target (including global).
    pub fn get_defines(&self, target: &TargetConfig) -> Vec<String> {
        let mut defines = self.compiler.defines.clone();
        defines.extend(target.defines.clone());
        defines
    }

    /// Get the C++ standard for a target.
    pub fn get_std(&self, target: &TargetConfig) -> Option<String> {
        target.std.clone().or_else(|| self.compiler.std.clone())
    }

    /// Get all library dependencies for a target in correct link order.
    /// This resolves internal deps (other targets) and external libs.
    /// Returns (internal_deps, external_libs) where internal_deps are target names
    /// and external_libs are library names to pass to -l flag.
    pub fn get_link_deps(&self, target: &TargetConfig) -> (Vec<String>, Vec<String>) {
        let mut internal_deps = Vec::new();
        let mut external_libs = target.libs.clone();

        // Resolve internal dependencies (topological sort)
        self.collect_deps_recursive(target, &mut internal_deps, &mut external_libs);

        // Reverse for correct link order (dependencies after dependents)
        internal_deps.reverse();

        (internal_deps, external_libs)
    }

    /// Recursively collect dependencies for a target.
    fn collect_deps_recursive(
        &self,
        target: &TargetConfig,
        internal_deps: &mut Vec<String>,
        external_libs: &mut Vec<String>,
    ) {
        for dep_name in &target.deps {
            if internal_deps.contains(dep_name) {
                continue; // Already visited
            }

            if let Some(dep_target) = self.find_target(dep_name) {
                internal_deps.push(dep_name.clone());

                // Add dep's external libs
                for lib in &dep_target.libs {
                    if !external_libs.contains(lib) {
                        external_libs.push(lib.clone());
                    }
                }

                // Recurse
                self.collect_deps_recursive(dep_target, internal_deps, external_libs);
            }
        }
    }

    /// Get library search paths for a target (includes dependent targets' lib_paths).
    pub fn get_lib_paths(&self, target: &TargetConfig) -> Vec<String> {
        let mut paths = target.lib_paths.clone();

        // Add paths from dependencies
        for dep_name in &target.deps {
            if let Some(dep_target) = self.find_target(dep_name) {
                for path in &dep_target.lib_paths {
                    if !paths.contains(path) {
                        paths.push(path.clone());
                    }
                }
            }
        }

        paths
    }

    /// Check if a target has circular dependencies.
    pub fn has_circular_deps(&self, target_name: &str) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        self.has_circular_deps_impl(target_name, &mut visited, &mut rec_stack)
    }

    fn has_circular_deps_impl(
        &self,
        target_name: &str,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        if rec_stack.contains(target_name) {
            return true; // Cycle detected
        }
        if visited.contains(target_name) {
            return false; // Already processed
        }

        visited.insert(target_name.to_string());
        rec_stack.insert(target_name.to_string());

        if let Some(target) = self.find_target(target_name) {
            for dep_name in &target.deps {
                if self.has_circular_deps_impl(dep_name, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(target_name);
        false
    }
}

impl TargetConfig {
    /// Create a new executable target.
    pub fn executable(name: &str) -> Self {
        Self {
            name: name.to_string(),
            target_type: TargetType::Executable,
            sources: Vec::new(),
            includes: Vec::new(),
            defines: Vec::new(),
            std: None,
            cflags: Vec::new(),
            libs: Vec::new(),
            lib_paths: Vec::new(),
            deps: Vec::new(),
        }
    }

    /// Create a new static library target.
    pub fn static_library(name: &str) -> Self {
        Self {
            name: name.to_string(),
            target_type: TargetType::StaticLibrary,
            ..Self::executable(name)
        }
    }

    /// Add source files.
    pub fn with_sources(mut self, sources: &[&str]) -> Self {
        self.sources = sources.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add include directories.
    pub fn with_includes(mut self, includes: &[&str]) -> Self {
        self.includes = includes.iter().map(|s| s.to_string()).collect();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml = r#"
[project]
name = "mako"
version = "1.0.0"

[compiler]
std = "c++23"
includes = ["/usr/include"]
defines = ["NDEBUG"]

[[target]]
name = "libmako"
type = "static_library"
sources = ["src/mako/*.cc"]
includes = ["src/mako", "third-party/erpc/src"]

[[target]]
name = "simpleTransaction"
type = "executable"
sources = ["examples/simpleTransaction.cc"]
deps = ["libmako"]
libs = ["pthread", "numa"]
        "#;

        let config: BuildConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.project.name, "mako");
        assert_eq!(config.project.version, Some("1.0.0".to_string()));
        assert_eq!(config.compiler.std, Some("c++23".to_string()));
        assert_eq!(config.targets.len(), 2);

        let lib = config.find_target("libmako").unwrap();
        assert_eq!(lib.target_type, TargetType::StaticLibrary);

        let exe = config.find_target("simpleTransaction").unwrap();
        assert_eq!(exe.target_type, TargetType::Executable);
        assert_eq!(exe.libs, vec!["pthread", "numa"]);
    }

    #[test]
    fn test_get_includes() {
        let config = BuildConfig {
            project: ProjectConfig {
                name: "test".to_string(),
                version: None,
                root: None,
            },
            compiler: CompilerConfig {
                std: None,
                includes: vec!["/usr/include".to_string()],
                defines: vec![],
                cflags: vec![],
            },
            targets: vec![TargetConfig::executable("main")
                .with_includes(&["src/include"])],
        };

        let target = &config.targets[0];
        let includes = config.get_includes(target);

        assert_eq!(includes, vec!["/usr/include", "src/include"]);
    }

    #[test]
    fn test_get_link_deps() {
        let toml = r#"
[project]
name = "test"

[[target]]
name = "libcore"
type = "static_library"
sources = ["core.cc"]
libs = ["pthread"]

[[target]]
name = "libutil"
type = "static_library"
sources = ["util.cc"]
deps = ["libcore"]
libs = ["numa"]

[[target]]
name = "main"
type = "executable"
sources = ["main.cc"]
deps = ["libutil"]
libs = ["m"]
        "#;

        let config: BuildConfig = toml::from_str(toml).unwrap();
        let main = config.find_target("main").unwrap();
        let (internal_deps, external_libs) = config.get_link_deps(main);

        // Internal deps should be in order where dependencies come first
        // libcore is a dependency of libutil, so libcore comes first
        assert_eq!(internal_deps, vec!["libcore", "libutil"]);

        // External libs from all targets
        assert!(external_libs.contains(&"m".to_string()));
        assert!(external_libs.contains(&"pthread".to_string()));
        assert!(external_libs.contains(&"numa".to_string()));
    }

    #[test]
    fn test_get_lib_paths() {
        let toml = r#"
[project]
name = "test"

[[target]]
name = "libcore"
type = "static_library"
sources = ["core.cc"]
lib_paths = ["/usr/local/lib"]

[[target]]
name = "main"
type = "executable"
sources = ["main.cc"]
deps = ["libcore"]
lib_paths = ["/opt/lib"]
        "#;

        let config: BuildConfig = toml::from_str(toml).unwrap();
        let main = config.find_target("main").unwrap();
        let lib_paths = config.get_lib_paths(main);

        assert!(lib_paths.contains(&"/opt/lib".to_string()));
        assert!(lib_paths.contains(&"/usr/local/lib".to_string()));
    }

    #[test]
    fn test_circular_deps_detection() {
        // No circular deps
        let toml = r#"
[project]
name = "test"

[[target]]
name = "libcore"
type = "static_library"
sources = ["core.cc"]

[[target]]
name = "main"
type = "executable"
sources = ["main.cc"]
deps = ["libcore"]
        "#;

        let config: BuildConfig = toml::from_str(toml).unwrap();
        assert!(!config.has_circular_deps("main"));

        // With circular deps
        let toml_circular = r#"
[project]
name = "test"

[[target]]
name = "libA"
type = "static_library"
sources = ["a.cc"]
deps = ["libB"]

[[target]]
name = "libB"
type = "static_library"
sources = ["b.cc"]
deps = ["libA"]
        "#;

        let config_circular: BuildConfig = toml::from_str(toml_circular).unwrap();
        assert!(config_circular.has_circular_deps("libA"));
    }
}
