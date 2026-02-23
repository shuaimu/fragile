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

        /// Use LibTooling for template method bodies (slower but more complete)
        #[arg(long)]
        use_libtooling: bool,
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
            use_libtooling,
        } => {
            let include_paths: Vec<String> = include
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            // If using LibTooling, pre-parse files to extract template method bodies
            let mut libtooling_results: std::collections::HashMap<
                std::path::PathBuf,
                std::collections::HashMap<(String, String), Vec<fragile_clang::MethodInfo>>,
            > = std::collections::HashMap::new();

            // Also store specialization field types when using LibTooling
            let mut libtooling_field_types: std::collections::HashMap<
                PathBuf,
                std::collections::HashMap<String, fragile_clang::SpecializationFieldInfo>,
            > = std::collections::HashMap::new();

            if use_libtooling {
                eprintln!("Pre-parsing with LibTooling for template bodies...");
                let libtooling_parser = fragile_clang::LibToolingParser::new();
                for file in &files {
                    eprintln!("  LibTooling parsing: {}", file.display());
                    match libtooling_parser.parse_file(file) {
                        Ok(libtooling_ctx) => {
                            let method_bodies =
                                fragile_clang::extract_method_bodies_with_params(&libtooling_ctx);
                            let field_types =
                                fragile_clang::extract_specialization_field_types(&libtooling_ctx);
                            eprintln!(
                                "    Found {} method body entries, {} specialization field types",
                                method_bodies.len(),
                                field_types.len()
                            );
                            libtooling_results.insert(file.clone(), method_bodies);
                            libtooling_field_types.insert(file.clone(), field_types);
                        }
                        Err(e) => {
                            eprintln!("    Warning: LibTooling parse failed: {}", e);
                        }
                    }
                }
            }

            // Create parser with vendored libc++
            let parser =
                fragile_clang::ClangParser::with_paths_and_defines(include_paths, define.clone())
                    .map_err(|e| miette::miette!("Failed to create parser: {}", e))?;

            // Parse all files first, then generate a single combined output.
            // This avoids duplicate crate preambles and allows cross-file symbol references.
            let mut combined_children = Vec::new();
            for file in &files {
                eprintln!("Transpiling: {}", file.display());

                let ast = parser
                    .parse_file(file)
                    .map_err(|e| miette::miette!("Failed to parse {}: {}", file.display(), e))?;
                combined_children.extend(ast.translation_unit.children);
            }

            let combined_tu =
                fragile_clang::ClangNode::new(fragile_clang::ClangNodeKind::TranslationUnit)
                    .with_children(combined_children);

            let all_output = if stubs_only {
                fragile_clang::AstCodeGen::new().generate_stubs(&combined_tu)
            } else if use_libtooling {
                let mut merged_method_bodies: std::collections::HashMap<
                    (String, String),
                    Vec<fragile_clang::MethodInfo>,
                > = std::collections::HashMap::new();
                let mut merged_field_types: std::collections::HashMap<
                    String,
                    fragile_clang::SpecializationFieldInfo,
                > = std::collections::HashMap::new();

                for file in &files {
                    if let Some(methods) = libtooling_results.remove(file) {
                        for (key, mut infos) in methods {
                            merged_method_bodies
                                .entry(key)
                                .or_default()
                                .append(&mut infos);
                        }
                    }
                    if let Some(field_types) = libtooling_field_types.remove(file) {
                        for (key, value) in field_types {
                            merged_field_types.entry(key).or_insert(value);
                        }
                    }
                }

                let mut codegen = fragile_clang::AstCodeGen::new();
                if !merged_method_bodies.is_empty() {
                    codegen.set_libtooling_bodies(merged_method_bodies);
                }
                if !merged_field_types.is_empty() {
                    codegen.set_specialization_field_types(merged_field_types);
                }
                codegen.generate(&combined_tu)
            } else {
                fragile_clang::AstCodeGen::new().generate(&combined_tu)
            };

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

            let parser =
                fragile_clang::ClangParser::with_paths_and_defines(include_paths, define.clone())
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
