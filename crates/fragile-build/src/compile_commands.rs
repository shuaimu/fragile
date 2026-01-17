//! compile_commands.json parsing.
//!
//! CMake can generate a compile_commands.json file that contains
//! the exact compilation commands for each source file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single compile command from compile_commands.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileCommand {
    /// The working directory for compilation.
    pub directory: PathBuf,

    /// The source file path.
    pub file: PathBuf,

    /// The full compilation command (space-separated).
    #[serde(default)]
    pub command: Option<String>,

    /// The compilation arguments (array form).
    #[serde(default)]
    pub arguments: Option<Vec<String>>,

    /// Output file (optional).
    #[serde(default)]
    pub output: Option<PathBuf>,
}

impl CompileCommand {
    /// Get the compilation arguments as a vector.
    pub fn get_args(&self) -> Vec<String> {
        if let Some(args) = &self.arguments {
            args.clone()
        } else if let Some(cmd) = &self.command {
            // Simple space-split (doesn't handle quoted strings properly)
            cmd.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    /// Extract include directories from the arguments.
    pub fn get_includes(&self) -> Vec<PathBuf> {
        let args = self.get_args();
        let mut includes = Vec::new();

        let mut i = 0;
        while i < args.len() {
            if args[i] == "-I" && i + 1 < args.len() {
                includes.push(PathBuf::from(&args[i + 1]));
                i += 2;
            } else if args[i].starts_with("-I") {
                includes.push(PathBuf::from(&args[i][2..]));
                i += 1;
            } else if args[i] == "-isystem" && i + 1 < args.len() {
                includes.push(PathBuf::from(&args[i + 1]));
                i += 2;
            } else {
                i += 1;
            }
        }

        includes
    }

    /// Extract preprocessor definitions from the arguments.
    pub fn get_defines(&self) -> Vec<String> {
        let args = self.get_args();
        let mut defines = Vec::new();

        let mut i = 0;
        while i < args.len() {
            if args[i] == "-D" && i + 1 < args.len() {
                defines.push(args[i + 1].clone());
                i += 2;
            } else if args[i].starts_with("-D") {
                defines.push(args[i][2..].to_string());
                i += 1;
            } else {
                i += 1;
            }
        }

        defines
    }

    /// Get the C++ standard from arguments (e.g., "-std=c++23").
    pub fn get_std(&self) -> Option<String> {
        let args = self.get_args();

        for arg in &args {
            if arg.starts_with("-std=") {
                return Some(arg[5..].to_string());
            }
        }

        None
    }
}

/// Collection of compile commands (from compile_commands.json).
#[derive(Debug, Clone)]
pub struct CompileCommands {
    commands: Vec<CompileCommand>,
}

impl CompileCommands {
    /// Load compile commands from a JSON file.
    pub fn from_file(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let commands: Vec<CompileCommand> = serde_json::from_str(&content)?;
        Ok(Self { commands })
    }

    /// Parse compile commands from a JSON string.
    pub fn from_str(json: &str) -> crate::Result<Self> {
        let commands: Vec<CompileCommand> = serde_json::from_str(json)?;
        Ok(Self { commands })
    }

    /// Get all compile commands.
    pub fn commands(&self) -> &[CompileCommand] {
        &self.commands
    }

    /// Find the compile command for a specific source file.
    pub fn find_command(&self, source: &Path) -> Option<&CompileCommand> {
        self.commands.iter().find(|cmd| {
            cmd.file == source || cmd.file.ends_with(source)
        })
    }

    /// Get all unique include directories across all commands.
    pub fn all_includes(&self) -> Vec<PathBuf> {
        let mut includes = Vec::new();

        for cmd in &self.commands {
            for inc in cmd.get_includes() {
                if !includes.contains(&inc) {
                    includes.push(inc);
                }
            }
        }

        includes
    }

    /// Get all unique defines across all commands.
    pub fn all_defines(&self) -> Vec<String> {
        let mut defines = Vec::new();

        for cmd in &self.commands {
            for def in cmd.get_defines() {
                if !defines.contains(&def) {
                    defines.push(def);
                }
            }
        }

        defines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compile_commands() {
        let json = r#"[
            {
                "directory": "/home/user/project/build",
                "file": "/home/user/project/src/main.cc",
                "command": "g++ -I/usr/include -I../include -DDEBUG=1 -std=c++23 -c main.cc"
            },
            {
                "directory": "/home/user/project/build",
                "file": "/home/user/project/src/utils.cc",
                "arguments": ["g++", "-I/usr/include", "-DNDEBUG", "-c", "utils.cc"]
            }
        ]"#;

        let cmds = CompileCommands::from_str(json).unwrap();

        assert_eq!(cmds.commands().len(), 2);

        let cmd0 = &cmds.commands()[0];
        assert_eq!(cmd0.get_std(), Some("c++23".to_string()));

        let includes = cmd0.get_includes();
        assert_eq!(includes.len(), 2);

        let defines = cmd0.get_defines();
        assert_eq!(defines, vec!["DEBUG=1"]);
    }

    #[test]
    fn test_find_command() {
        let json = r#"[
            {
                "directory": "/build",
                "file": "src/main.cc",
                "command": "g++ -c main.cc"
            }
        ]"#;

        let cmds = CompileCommands::from_str(json).unwrap();

        let found = cmds.find_command(Path::new("src/main.cc"));
        assert!(found.is_some());

        let not_found = cmds.find_command(Path::new("src/other.cc"));
        assert!(not_found.is_none());
    }
}
