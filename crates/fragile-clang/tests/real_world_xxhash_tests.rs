//! Real-world non-STL project baseline tests using xxHash.
//!
//! These tests are intentionally ignored by default because they pull an
//! external repository. They are used as a TDD anchor for "real-world C
//! project before STL-heavy C++" progress.

use fragile_clang::transpile_cpp_to_rust;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

const XXHASH_REPO_URL: &str = "https://github.com/Cyan4973/xxHash.git";
const XXHASH_CACHE_DIR: &str = "/tmp/fragile_real_world_xxhash";

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<(), String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git {:?}: {}", args, e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn ensure_xxhash_checkout() -> Result<PathBuf, String> {
    let repo_dir = PathBuf::from(XXHASH_CACHE_DIR);
    if repo_dir.join("xxhash.c").exists() && repo_dir.join("xxhash.h").exists() {
        return Ok(repo_dir);
    }

    if let Some(parent) = repo_dir.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create cache dir {}: {}", parent.display(), e))?;
    }

    let lock_path = repo_dir.with_extension("clone.lock");
    let mut have_lock = false;
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lock_path)
    {
        Ok(_) => {
            have_lock = true;
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        Err(e) => {
            return Err(format!(
                "failed to create clone lock {}: {}",
                lock_path.display(),
                e
            ));
        }
    }

    if have_lock {
        // Best effort cleanup on stale partial clone.
        if repo_dir.exists() && !repo_dir.join("xxhash.h").exists() {
            let _ = fs::remove_dir_all(&repo_dir);
        }
        run_git(
            &["clone", "--depth", "1", XXHASH_REPO_URL, XXHASH_CACHE_DIR],
            None,
        )?;
        let _ = fs::remove_file(&lock_path);
    } else {
        // Another test/process is cloning. Wait briefly for the checkout to become ready.
        for _ in 0..100 {
            if repo_dir.join("xxhash.c").exists() && repo_dir.join("xxhash.h").exists() {
                return Ok(repo_dir);
            }
            sleep(Duration::from_millis(100));
        }
    }

    if !(repo_dir.join("xxhash.c").exists() && repo_dir.join("xxhash.h").exists()) {
        return Err(format!(
            "xxHash checkout incomplete at {}",
            repo_dir.display()
        ));
    }

    Ok(repo_dir)
}

fn compile_rust_file(path: &Path, out: &Path, crate_type: &str) -> Result<(), String> {
    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-C")
        .arg("overflow-checks=off")
        .arg("--crate-type")
        .arg(crate_type)
        .arg("-A")
        .arg("warnings")
        .arg(path)
        .arg("-o")
        .arg(out)
        .output()
        .map_err(|e| format!("failed to run rustc: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rustc failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[test]
#[ignore = "real-world external project test (downloads xxHash)"]
fn test_real_world_xxhash_transpile_and_compile_as_lib() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let xxhash_c = repo_dir.join("xxhash.c");
    assert!(
        xxhash_c.exists(),
        "expected source file: {}",
        xxhash_c.display()
    );

    let rust_code =
        transpile_cpp_to_rust(&xxhash_c).expect("xxhash.c should transpile without parser failure");
    assert!(
        rust_code.contains("XXH_errorcode") && rust_code.contains("pub fn XXH32"),
        "expected XXH symbols in generated code"
    );

    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let rs_path = temp_dir.join("xxhash_transpiled.rs");
    let out_path = temp_dir.join("libxxhash_transpiled.rlib");
    fs::write(&rs_path, rust_code).expect("failed to write generated Rust source");

    compile_rust_file(&rs_path, &out_path, "lib")
        .expect("transpiled xxhash should compile as Rust library");
}

#[test]
#[ignore = "real-world external project test (downloads xxHash)"]
fn test_real_world_xxhash_inline_wrapper_compiles_and_runs() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");

    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let wrapper_c = temp_dir.join("xxhash_inline_wrapper.c");
    let wrapper_rs = temp_dir.join("xxhash_inline_wrapper.rs");
    let wrapper_bin = temp_dir.join("xxhash_inline_wrapper_bin");

    let wrapper_src = format!(
        "#define XXH_INLINE_ALL\n#include \"{}\"\n\nint main() {{\n    const char* s = \"hello\";\n    if (XXH32(s, 5, 0) != 4211111929U) return 1;\n    if (XXH32(s, 5, 12345U) != 2696178842U) return 2;\n    if (XXH32(\"\", 0, 0) != 46947589U) return 3;\n    return 0;\n}}\n",
        repo_dir.join("xxhash.h").display()
    );
    fs::write(&wrapper_c, wrapper_src).expect("failed to write wrapper source");

    let generated =
        transpile_cpp_to_rust(&wrapper_c).expect("wrapper source should transpile to Rust");
    fs::write(&wrapper_rs, generated).expect("failed to write wrapper Rust source");

    let output = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-C")
        .arg("overflow-checks=off")
        .arg("-A")
        .arg("warnings")
        .arg(&wrapper_rs)
        .arg("-o")
        .arg(&wrapper_bin)
        .output()
        .expect("failed to run rustc");

    assert!(
        output.status.success(),
        "wrapper transpilation should compile after xxhash inline symbol lowering.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run_output = Command::new(&wrapper_bin)
        .output()
        .expect("failed to run wrapper binary");
    assert_eq!(
        run_output.status.code(),
        Some(0),
        "wrapper binary should return success.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );
}

#[test]
#[ignore = "real-world external project test (downloads xxHash)"]
fn test_real_world_xxhash_x86dispatch_transpile_and_compile_as_lib() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let dispatch_c = repo_dir.join("xxh_x86dispatch.c");
    assert!(
        dispatch_c.exists(),
        "expected source file: {}",
        dispatch_c.display()
    );

    let rust_code =
        transpile_cpp_to_rust(&dispatch_c).expect("xxh_x86dispatch.c should transpile to Rust");
    assert!(
        rust_code.contains("XXH3_64bits_dispatch"),
        "expected x86dispatch symbols in generated code"
    );

    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let rs_path = temp_dir.join("xxh_x86dispatch_transpiled.rs");
    let out_path = temp_dir.join("libxxh_x86dispatch_transpiled.rlib");
    fs::write(&rs_path, rust_code).expect("failed to write generated Rust source");

    compile_rust_file(&rs_path, &out_path, "lib")
        .expect("transpiled xxh_x86dispatch should compile as Rust library");
}

#[test]
#[ignore = "real-world external project test (downloads xxHash)"]
fn test_real_world_xxhash_scalar_impl_transpile_and_compile_as_lib() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let scalar_impl_c = repo_dir.join("xxhash_scalar_impl.c");
    assert!(
        scalar_impl_c.exists(),
        "expected source file: {}",
        scalar_impl_c.display()
    );

    let rust_code =
        transpile_cpp_to_rust(&scalar_impl_c).expect("xxhash_scalar_impl.c should transpile to Rust");
    assert!(
        rust_code.contains("XXH32") && rust_code.contains("XXH3"),
        "expected xxhash scalar implementation symbols in generated code"
    );

    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let rs_path = temp_dir.join("xxhash_scalar_impl_transpiled.rs");
    let out_path = temp_dir.join("libxxhash_scalar_impl_transpiled.rlib");
    fs::write(&rs_path, rust_code).expect("failed to write generated Rust source");

    compile_rust_file(&rs_path, &out_path, "lib")
        .expect("transpiled xxhash_scalar_impl should compile as Rust library");
}
