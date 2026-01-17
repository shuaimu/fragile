//! Error types for fragile-build.

use thiserror::Error;

/// Result type for fragile-build operations.
pub type Result<T> = std::result::Result<T, BuildError>;

/// Errors that can occur during build configuration.
#[derive(Error, Debug)]
pub enum BuildError {
    /// Failed to read configuration file.
    #[error("Failed to read config file: {0}")]
    ReadConfig(#[from] std::io::Error),

    /// Failed to parse TOML configuration.
    #[error("Failed to parse TOML config: {0}")]
    ParseToml(#[from] toml::de::Error),

    /// Failed to parse JSON (compile_commands.json).
    #[error("Failed to parse JSON: {0}")]
    ParseJson(#[from] serde_json::Error),

    /// Configuration validation error.
    #[error("Config validation error: {0}")]
    Validation(String),

    /// Target not found.
    #[error("Target not found: {0}")]
    TargetNotFound(String),

    /// Source file not found.
    #[error("Source file not found: {0}")]
    SourceNotFound(String),
}
