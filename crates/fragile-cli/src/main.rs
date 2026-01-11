use clap::{Parser, Subcommand};
use fragile_driver::Driver;
use miette::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fragile")]
#[command(author, version, about = "A polyglot compiler for Rust, C++, and Go")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile source files to an executable or object file
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
