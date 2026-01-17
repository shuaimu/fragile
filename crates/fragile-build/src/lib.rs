//! Build system integration for Fragile compiler.
//!
//! This crate provides:
//! - Build configuration format (`fragile.toml`)
//! - compile_commands.json parsing
//! - Manual build configuration support
//!
//! # Example
//!
//! ```toml
//! # fragile.toml
//! [project]
//! name = "my-cpp-project"
//!
//! [[target]]
//! name = "main"
//! type = "executable"
//! sources = ["src/main.cc", "src/utils.cc"]
//! includes = ["include", "/usr/include"]
//! defines = ["DEBUG=1"]
//! std = "c++23"
//! ```

mod config;
mod compile_commands;
mod error;

pub use config::{BuildConfig, TargetConfig, TargetType};
pub use compile_commands::{CompileCommand, CompileCommands};
pub use error::{BuildError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_config() {
        let toml = r#"
[project]
name = "test-project"

[[target]]
name = "main"
type = "executable"
sources = ["src/main.cc"]
        "#;

        let config: BuildConfig = toml::from_str(toml).expect("Failed to parse config");
        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.targets.len(), 1);
        assert_eq!(config.targets[0].name, "main");
    }
}
