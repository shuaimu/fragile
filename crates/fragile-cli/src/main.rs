use clap::{Parser, Subcommand};
use fragile_driver::Driver;
use fragile_build::BuildConfig;
use fragile_rustc_driver::{FragileDriver, CppCompiler, CppCompilerConfig, build_target, OutputType, generate_rust_stubs};
use miette::Result;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "fragile")]
#[command(author, version, about = "A polyglot compiler for Rust, C++, and Go")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile source files to an executable or object file (legacy)
    Build {
        /// Source files to compile
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Emit LLVM IR instead of object code
        #[arg(long)]
        emit_ir: bool,
    },

    /// Build a target from fragile.toml
    BuildTarget {
        /// Target name to build
        target: String,

        /// Path to fragile.toml (default: ./fragile.toml)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output directory for build artifacts
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Parse C++ files and generate Rust stubs
    ParseCpp {
        /// C++ source files to parse
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output directory for stubs
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Include directories
        #[arg(short = 'I', long)]
        include: Vec<PathBuf>,

        /// Preprocessor definitions
        #[arg(short = 'D', long)]
        define: Vec<String>,
    },

    /// Check source files for errors without compiling
    Check {
        /// Source files to check
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Print the AST/HIR of a source file
    Dump {
        /// Source file to dump
        file: PathBuf,

        /// What to dump
        #[arg(long, default_value = "hir")]
        format: DumpFormat,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum DumpFormat {
    /// Dump High-level IR
    Hir,
    /// Dump LLVM IR
    Llvm,
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
        Commands::Build {
            files,
            output,
            emit_ir,
        } => {
            let driver = Driver::new();

            for file in &files {
                if emit_ir {
                    let ir = driver.compile_to_ir(file)?;
                    if let Some(ref output_path) = output {
                        std::fs::write(output_path, &ir)
                            .map_err(|e| miette::miette!("Failed to write IR: {}", e))?;
                        println!("Wrote IR to {}", output_path.display());
                    } else {
                        println!("{}", ir);
                    }
                } else {
                    let output_path = output.clone().unwrap_or_else(|| {
                        let stem = file.file_stem().unwrap().to_str().unwrap();
                        PathBuf::from(format!("{}.o", stem))
                    });
                    driver.compile_to_object(file, &output_path)?;
                    println!("Compiled {} -> {}", file.display(), output_path.display());
                }
            }
        }

        Commands::BuildTarget {
            target,
            config,
            output_dir,
            verbose,
        } => {
            let config_path = config.unwrap_or_else(|| PathBuf::from("fragile.toml"));

            if !config_path.exists() {
                return Err(miette::miette!("Config file not found: {}", config_path.display()));
            }

            let project_root = config_path.parent().unwrap_or(Path::new("."));
            let output_dir = output_dir.unwrap_or_else(|| project_root.join("build"));

            // Load build configuration
            let build_config = BuildConfig::from_file(&config_path)
                .map_err(|e| miette::miette!("Failed to load config: {}", e))?;

            // Get the target
            let job = build_target(&build_config, &target, project_root)
                .map_err(|e| miette::miette!("{}", e))?;

            if verbose {
                println!("Building target: {}", target);
                println!("Sources: {:?}", job.sources);
                println!("Includes: {:?}", job.includes);
                println!("Defines: {:?}", job.defines);
            }

            // Create output directory
            std::fs::create_dir_all(&output_dir)
                .map_err(|e| miette::miette!("Failed to create output dir: {}", e))?;

            // Configure C++ compiler
            let mut compiler_config = CppCompilerConfig::default();
            for inc in &job.includes {
                compiler_config.include_dirs.push(inc.clone());
            }
            for def in &job.defines {
                compiler_config.defines.push(def.clone());
            }
            if let Some(std) = &job.std {
                compiler_config.std_version = std.clone();
            }

            let compiler = CppCompiler::new(compiler_config)
                .map_err(|e| miette::miette!("Failed to create C++ compiler: {}", e))?;

            if verbose {
                println!("Using compiler: {:?}", compiler.compiler_path());
            }

            // Parse and compile each source file
            let (include_paths, system_paths, defines) = job.parser_config();

            let parser = fragile_clang::ClangParser::with_paths_and_defines(
                include_paths,
                system_paths,
                defines,
            ).map_err(|e| miette::miette!("Failed to create parser: {}", e))?;

            let driver = FragileDriver::new();
            let mut object_files = Vec::new();

            for source in &job.sources {
                if verbose {
                    println!("Parsing: {}", source.display());
                }

                // Parse the C++ file to AST, then convert to module
                let ast = parser.parse_file(source)
                    .map_err(|e| miette::miette!("Failed to parse {}: {}", source.display(), e))?;
                let module = fragile_clang::MirConverter::new().convert(ast)
                    .map_err(|e| miette::miette!("Failed to convert {}: {}", source.display(), e))?;

                // Register with driver
                driver.register_cpp_module(&module);

                // Compile to object file
                if verbose {
                    println!("Compiling: {}", source.display());
                }

                let obj_path = compiler.compile_to_object(source, &output_dir)
                    .map_err(|e| miette::miette!("Failed to compile {}: {}", source.display(), e))?;

                object_files.push(obj_path);
            }

            println!("Built {} object files for target '{}'", object_files.len(), target);

            // For executables, also link (TODO: implement link step)
            if job.output_type == OutputType::Executable {
                let exe_path = output_dir.join(&target);
                println!("Note: Linking not yet implemented. Object files in: {}", output_dir.display());
                println!("To link manually: g++ -o {} {:?} -l{}",
                    exe_path.display(),
                    object_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(" "),
                    job.libs.join(" -l")
                );
            }
        }

        Commands::ParseCpp {
            files,
            output_dir,
            include,
            define,
        } => {
            let include_paths: Vec<String> = include.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            let parser = fragile_clang::ClangParser::with_paths_and_defines(
                include_paths,
                Vec::new(),
                define.clone(),
            ).map_err(|e| miette::miette!("Failed to create parser: {}", e))?;

            let driver = FragileDriver::new();
            let mut modules = Vec::new();

            for file in &files {
                println!("Parsing: {}", file.display());

                let ast = parser.parse_file(file)
                    .map_err(|e| miette::miette!("Failed to parse {}: {}", file.display(), e))?;

                println!("  AST root: {:?}", ast.translation_unit.kind);

                let module = fragile_clang::MirConverter::new().convert(ast)
                    .map_err(|e| miette::miette!("Failed to convert {}: {}", file.display(), e))?;

                println!("  Functions: {}", module.functions.len());

                driver.register_cpp_module(&module);
                modules.push(module);
            }

            // Generate stubs
            let stubs = generate_rust_stubs(&modules);

            if let Some(out_dir) = output_dir {
                std::fs::create_dir_all(&out_dir)
                    .map_err(|e| miette::miette!("Failed to create output dir: {}", e))?;

                let stubs_path = out_dir.join("stubs.rs");
                std::fs::write(&stubs_path, &stubs)
                    .map_err(|e| miette::miette!("Failed to write stubs: {}", e))?;

                println!("Wrote stubs to: {}", stubs_path.display());
            } else {
                println!("\n--- Generated Stubs ---\n{}", stubs);
            }
        }

        Commands::Check { files } => {
            let driver = Driver::new();

            for file in &files {
                match driver.parse_file(file) {
                    Ok(_) => println!("{}: OK", file.display()),
                    Err(e) => {
                        eprintln!("{}: Error", file.display());
                        return Err(e);
                    }
                }
            }
        }

        Commands::Dump { file, format } => {
            let driver = Driver::new();

            match format {
                DumpFormat::Hir => {
                    let module = driver.parse_file(&file)?;
                    println!("{:#?}", module);
                }
                DumpFormat::Llvm => {
                    let ir = driver.compile_to_ir(&file)?;
                    println!("{}", ir);
                }
            }
        }
    }

    Ok(())
}
