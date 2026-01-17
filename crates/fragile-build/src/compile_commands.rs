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

    /// Get the optimization level from arguments (-O0, -O1, -O2, -O3, -Os, -Oz).
    /// Returns the level as a string (e.g., "2" for -O2, "s" for -Os).
    pub fn get_opt_level(&self) -> Option<String> {
        let args = self.get_args();

        for arg in &args {
            if arg.starts_with("-O") && arg.len() > 2 {
                return Some(arg[2..].to_string());
            }
        }

        None
    }

    /// Check if debug info is enabled (-g flag).
    pub fn has_debug_info(&self) -> bool {
        let args = self.get_args();
        args.iter().any(|arg| arg == "-g" || arg.starts_with("-g"))
    }

    /// Get warning flags from arguments (-W* flags).
    pub fn get_warning_flags(&self) -> Vec<String> {
        let args = self.get_args();
        args.iter()
            .filter(|arg| arg.starts_with("-W"))
            .cloned()
            .collect()
    }

    /// Get all other compiler flags not covered by specific methods.
    /// This excludes -I, -D, -std, -O, -g, -W, -c, -o flags and the compiler itself.
    pub fn get_other_flags(&self) -> Vec<String> {
        let args = self.get_args();
        args.into_iter()
            .skip(1) // Skip compiler executable
            .filter(|arg| {
                !arg.starts_with("-I") &&
                !arg.starts_with("-D") &&
                !arg.starts_with("-std=") &&
                !arg.starts_with("-O") &&
                !arg.starts_with("-W") &&
                arg != "-g" &&
                arg != "-c" &&
                !arg.starts_with("-o") &&
                !arg.ends_with(".cc") &&
                !arg.ends_with(".cpp") &&
                !arg.ends_with(".cxx") &&
                !arg.ends_with(".c")
            })
            .collect()
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

    #[test]
    fn test_get_opt_level() {
        let json = r#"[
            {
                "directory": "/build",
                "file": "main.cc",
                "command": "g++ -O2 -c main.cc"
            },
            {
                "directory": "/build",
                "file": "debug.cc",
                "command": "g++ -O0 -g -c debug.cc"
            },
            {
                "directory": "/build",
                "file": "release.cc",
                "command": "g++ -Os -c release.cc"
            }
        ]"#;

        let cmds = CompileCommands::from_str(json).unwrap();

        assert_eq!(cmds.commands()[0].get_opt_level(), Some("2".to_string()));
        assert_eq!(cmds.commands()[1].get_opt_level(), Some("0".to_string()));
        assert_eq!(cmds.commands()[2].get_opt_level(), Some("s".to_string()));
    }

    #[test]
    fn test_has_debug_info() {
        let json = r#"[
            {
                "directory": "/build",
                "file": "debug.cc",
                "command": "g++ -g -c debug.cc"
            },
            {
                "directory": "/build",
                "file": "release.cc",
                "command": "g++ -O2 -c release.cc"
            }
        ]"#;

        let cmds = CompileCommands::from_str(json).unwrap();

        assert!(cmds.commands()[0].has_debug_info());
        assert!(!cmds.commands()[1].has_debug_info());
    }

    #[test]
    fn test_get_warning_flags() {
        let json = r#"[
            {
                "directory": "/build",
                "file": "main.cc",
                "command": "g++ -Wall -Wextra -Werror -c main.cc"
            }
        ]"#;

        let cmds = CompileCommands::from_str(json).unwrap();
        let warnings = cmds.commands()[0].get_warning_flags();

        assert_eq!(warnings.len(), 3);
        assert!(warnings.contains(&"-Wall".to_string()));
        assert!(warnings.contains(&"-Wextra".to_string()));
        assert!(warnings.contains(&"-Werror".to_string()));
    }

    #[test]
    fn test_get_other_flags() {
        let json = r#"[
            {
                "directory": "/build",
                "file": "main.cc",
                "command": "g++ -fPIC -pthread -march=native -I/include -DFOO -c main.cc"
            }
        ]"#;

        let cmds = CompileCommands::from_str(json).unwrap();
        let other = cmds.commands()[0].get_other_flags();

        assert!(other.contains(&"-fPIC".to_string()));
        assert!(other.contains(&"-pthread".to_string()));
        assert!(other.contains(&"-march=native".to_string()));
        // These should NOT be in other flags
        assert!(!other.iter().any(|f| f.starts_with("-I")));
        assert!(!other.iter().any(|f| f.starts_with("-D")));
    }
}
