use clap::{Parser, Subcommand};
use miette::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fragile")]
#[command(author, version, about = "C++ to Rust transpiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transpile C++ files to Rust source code
    Transpile {
        /// C++ source files to transpile
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include directories
        #[arg(short = 'I', long)]
        include: Vec<PathBuf>,

        /// Preprocessor definitions
        #[arg(short = 'D', long)]
        define: Vec<String>,

        /// Generate stubs only (function signatures, no bodies)
        #[arg(long)]
        stubs_only: bool,

        /// Use libc++ (LLVM's C++ standard library) instead of libstdc++.
        /// Recommended for transpiling STL code as libc++ has cleaner code.
        /// Requires: `apt install libc++-dev libc++abi-dev` on Debian/Ubuntu.
        #[arg(long)]
        use_libcxx: bool,
    },

    /// Parse C++ files and show AST information (deprecated, use 'transpile')
    #[command(hide = true)]
    ParseCpp {
        /// C++ source files to parse
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output directory for generated code
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Include directories
        #[arg(short = 'I', long)]
        include: Vec<PathBuf>,

        /// Preprocessor definitions
        #[arg(short = 'D', long)]
        define: Vec<String>,

        /// Output full Rust code instead of stubs
        #[arg(long)]
        full: bool,
    },
}

fn main() -> Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(3)
                .build(),
        )
    }))?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Transpile {
            files,
            output,
            include,
            define,
            stubs_only,
            use_libcxx,
        } => {
            let include_paths: Vec<String> = include
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            // Create parser with optional libc++ support
            let parser = if use_libcxx {
                // Check if libc++ is available
                if !fragile_clang::ClangParser::is_libcxx_available() {
                    return Err(miette::miette!(
                        "libc++ not found. Please install it:\n  Debian/Ubuntu: apt install libc++-dev libc++abi-dev"
                    ));
                }
                let system_paths = fragile_clang::ClangParser::detect_libcxx_include_paths();
                fragile_clang::ClangParser::with_full_options(
                    include_paths,
                    system_paths,
                    define.clone(),
                    Vec::new(),
                    true,
                )
            } else {
                fragile_clang::ClangParser::with_paths_and_defines(
                    include_paths,
                    Vec::new(),
                    define.clone(),
                )
            }
            .map_err(|e| miette::miette!("Failed to create parser: {}", e))?;

            let mut all_output = String::new();

            for file in &files {
                eprintln!("Transpiling: {}", file.display());

                let ast = parser
                    .parse_file(file)
                    .map_err(|e| miette::miette!("Failed to parse {}: {}", file.display(), e))?;

                let code = if stubs_only {
                    fragile_clang::AstCodeGen::new().generate_stubs(&ast.translation_unit)
                } else {
                    fragile_clang::AstCodeGen::new().generate(&ast.translation_unit)
                };

                all_output.push_str(&code);
                all_output.push('\n');
            }

            if let Some(out_path) = output {
                if let Some(parent) = out_path.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| miette::miette!("Failed to create output dir: {}", e))?;
                    }
                }
                std::fs::write(&out_path, &all_output)
                    .map_err(|e| miette::miette!("Failed to write output: {}", e))?;
                eprintln!("Wrote: {}", out_path.display());
            } else {
                print!("{}", all_output);
            }
        }

        // Legacy command - redirect to transpile
        Commands::ParseCpp {
            files,
            output_dir,
            include,
            define,
            full,
        } => {
            eprintln!("Note: 'parse-cpp' is deprecated, use 'transpile' instead");

            let include_paths: Vec<String> = include
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            let parser = fragile_clang::ClangParser::with_paths_and_defines(
                include_paths,
                Vec::new(),
                define.clone(),
            )
            .map_err(|e| miette::miette!("Failed to create parser: {}", e))?;

            let mut all_output = String::new();

            for file in &files {
                eprintln!("Parsing: {}", file.display());

                let ast = parser
                    .parse_file(file)
                    .map_err(|e| miette::miette!("Failed to parse {}: {}", file.display(), e))?;

                let code = if full {
                    fragile_clang::AstCodeGen::new().generate(&ast.translation_unit)
                } else {
                    fragile_clang::AstCodeGen::new().generate_stubs(&ast.translation_unit)
                };

                all_output.push_str(&code);
                all_output.push('\n');
            }

            if let Some(out_dir) = output_dir {
                std::fs::create_dir_all(&out_dir)
                    .map_err(|e| miette::miette!("Failed to create output dir: {}", e))?;

                let filename = if full { "output.rs" } else { "stubs.rs" };
                let out_path = out_dir.join(filename);
                std::fs::write(&out_path, &all_output)
                    .map_err(|e| miette::miette!("Failed to write output: {}", e))?;

                eprintln!("Wrote: {}", out_path.display());
            } else {
                print!("{}", all_output);
            }
        }
    }

    Ok(())
}
