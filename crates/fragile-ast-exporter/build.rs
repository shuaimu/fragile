use cmake::Config;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Skip native build on docs.rs (no network access)
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    let llvm_info = LLVMInfo::new();

    // Build the C++ AST exporter library
    build_native(&llvm_info);

    // Generate Rust bindings for ast_tags.hpp
    generate_bindings();
}

fn build_native(llvm_info: &LLVMInfo) {
    let llvm_lib_dir = &llvm_info.lib_dir;
    let llvm_include_dir = &llvm_info.include_dir;

    eprintln!(
        "cargo:warning=Using LLVM {} at {}",
        llvm_info.version,
        llvm_lib_dir.display()
    );
    eprintln!(
        "cargo:warning=LLVM include dir: {}",
        llvm_include_dir.display()
    );

    // Check for pre-built library
    if let Ok(libdir) = env::var("FRAGILE_AST_EXPORTER_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", libdir);
    } else {
        // Build with CMake
        let mut cmake_config = Config::new("src");

        // Use the LLVM cmake config from the same version we're using
        cmake_config.define("LLVM_DIR", llvm_lib_dir.join("cmake/llvm"));

        // Add include directories
        let include_flag = format!("-I{}", llvm_include_dir.display());
        cmake_config.cflag(&include_flag);
        cmake_config.cxxflag(&include_flag);

        let dst = cmake_config.build_target("fragileAstExporter").build();

        let out_dir = dst.display();
        println!("cargo:rustc-link-search=native={}/build/lib", out_dir);
        println!("cargo:rustc-link-search=native={}/build", out_dir);
    }

    // Link tinycbor
    println!("cargo:rustc-link-lib=static=tinycbor");

    // Link our AST exporter library
    println!("cargo:rustc-link-lib=static=fragileAstExporter");

    // Link LLVM/Clang libraries
    println!("cargo:rustc-link-search=native={}", llvm_lib_dir.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib_dir = out_dir.join("lib");
    std::fs::create_dir_all(&lib_dir).ok();

    let search_dirs = library_search_dirs(llvm_lib_dir, &llvm_info.version);
    maybe_create_library_alias(
        &lib_dir,
        "libclang-cpp.so",
        find_library_target(&search_dirs, "libclang-cpp.so", &["libclang-cpp.so."]),
    );
    maybe_create_library_alias(
        &lib_dir,
        "libLLVM.so",
        find_library_target(&search_dirs, "libLLVM.so", &["libLLVM.so.", "libLLVM-"]),
    );

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Link libclang-cpp (shared library) which contains all clang components
    println!("cargo:rustc-link-lib=clang-cpp");

    // Link LLVM shared lib
    println!("cargo:rustc-link-lib=LLVM");

    // Link C++ standard library
    if cfg!(target_os = "macos") || cfg!(target_os = "freebsd") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }
}

fn generate_bindings() {
    let bindings = bindgen::Builder::default()
        .header("src/ast_tags.hpp")
        .header("src/AstExporter.hpp")
        .generate_comments(true)
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true)
        .derive_eq(true)
        .derive_hash(true)
        .rustified_enum("ASTEntryTag")
        .rustified_enum("BinaryOperatorKind")
        .rustified_enum("UnaryOperatorKind")
        .rustified_enum("CastKind")
        .rustified_enum("AccessSpecifier")
        .rustified_enum("UnaryExprOrTypeTrait")
        // Explicitly allow all types we need
        .allowlist_type("ASTEntryTag")
        .allowlist_type("BinaryOperatorKind")
        .allowlist_type("UnaryOperatorKind")
        .allowlist_type("CastKind")
        .allowlist_type("AccessSpecifier")
        .allowlist_type("UnaryExprOrTypeTrait")
        .allowlist_type("ExportResult")
        .allowlist_function("ast_exporter")
        .allowlist_function("drop_export_result")
        .allowlist_function("clang_version")
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .generate()
        .expect("Unable to generate bindings");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

struct LLVMInfo {
    lib_dir: PathBuf,
    include_dir: PathBuf,
    version: String,
}

impl LLVMInfo {
    fn new() -> Self {
        // Find a usable LLVM installation with Clang headers
        let (llvm_config, version) = find_usable_llvm()
            .expect("Could not find LLVM with Clang headers. Install libclang-dev.");

        let lib_dir = {
            let path_str = env::var_os("LLVM_LIB_DIR")
                .or_else(|| {
                    invoke_llvm_config(&Some(llvm_config.clone()), &["--libdir"]).map(Into::into)
                })
                .expect("Could not find LLVM lib dir");
            Path::new(&path_str)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&path_str))
        };

        let include_dir = {
            let path_str = env::var_os("LLVM_INCLUDE_DIR")
                .or_else(|| {
                    invoke_llvm_config(&Some(llvm_config.clone()), &["--includedir"])
                        .map(Into::into)
                })
                .unwrap_or_else(|| lib_dir.parent().unwrap().join("include").into());
            Path::new(&path_str)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&path_str))
        };

        Self {
            lib_dir,
            include_dir,
            version,
        }
    }
}

/// Find an LLVM installation that has Clang headers available
fn find_usable_llvm() -> Option<(PathBuf, String)> {
    // Try environment variable first
    if let Ok(path) = env::var("LLVM_CONFIG") {
        let config = PathBuf::from(&path);
        if let Some(version) = get_llvm_version(&config) {
            if has_clang_headers(&config) {
                return Some((config, version));
            }
        }
    }

    // Try versioned llvm-config commands, preferring versions with Clang headers
    for version in (14..=20).rev() {
        let cmd = format!("llvm-config-{}", version);
        let config = PathBuf::from(&cmd);

        if let Ok(output) = Command::new(&cmd).arg("--version").output() {
            if output.status.success() {
                if has_clang_headers(&config) {
                    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return Some((config, ver));
                }
            }
        }
    }

    // Try plain llvm-config
    let config = PathBuf::from("llvm-config");
    if let Ok(output) = Command::new("llvm-config").arg("--version").output() {
        if output.status.success() && has_clang_headers(&config) {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Some((config, ver));
        }
    }

    None
}

fn get_llvm_version(llvm_config: &PathBuf) -> Option<String> {
    Command::new(llvm_config)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn has_clang_headers(llvm_config: &PathBuf) -> bool {
    let include_dir = invoke_llvm_config(&Some(llvm_config.clone()), &["--includedir"]);

    if let Some(dir) = include_dir {
        let header_path = Path::new(&dir).join("clang/AST/ASTContext.h");
        return header_path.exists();
    }

    false
}

fn invoke_llvm_config(llvm_config: &Option<PathBuf>, args: &[&str]) -> Option<String> {
    llvm_config.as_ref().and_then(|config| {
        Command::new(config)
            .args(args)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
    })
}

fn library_search_dirs(llvm_lib_dir: &Path, llvm_version: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(llvm_lib_dir.to_path_buf());

    if let Some(major) = llvm_version.split('.').next() {
        dirs.push(PathBuf::from(format!("/usr/lib/llvm-{}/lib", major)));
    }

    dirs.push(PathBuf::from("/usr/lib/x86_64-linux-gnu"));

    dirs
}

fn find_library_target(search_dirs: &[PathBuf], exact_name: &str, versioned_prefixes: &[&str]) -> Option<PathBuf> {
    for dir in search_dirs {
        let candidate = dir.join(exact_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    for dir in search_dirs {
        if let Some(candidate) = find_versioned_library_in_dir(dir, versioned_prefixes) {
            return Some(candidate);
        }
    }

    None
}

fn find_versioned_library_in_dir(dir: &Path, prefixes: &[&str]) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() || path.is_symlink())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)))
                .unwrap_or(false)
        })
        .collect();

    candidates.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|name| name.to_str()).unwrap_or("");
        let b_name = b.file_name().and_then(|name| name.to_str()).unwrap_or("");
        b_name.cmp(a_name)
    });

    candidates.into_iter().next()
}

fn maybe_create_library_alias(alias_dir: &Path, alias_name: &str, target: Option<PathBuf>) {
    let alias_path = alias_dir.join(alias_name);
    if alias_path.exists() {
        return;
    }

    if let Some(target) = target {
        std::os::unix::fs::symlink(&target, &alias_path).ok();
    } else {
        eprintln!(
            "cargo:warning=Could not locate {} for linker alias generation",
            alias_name
        );
    }
}
