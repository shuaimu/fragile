//! Real-world non-STL project baseline tests using xxHash.
//!
//! These tests are intentionally ignored by default because they pull an
//! external repository. They are used as a TDD anchor for "real-world C
//! project before STL-heavy C++" progress.

use fragile_clang::{transpile_cpp_to_rust, AstCodeGen, ClangNode, ClangNodeKind, ClangParser};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::sleep;
use std::time::Duration;

const XXHASH_REPO_URL: &str = "https://github.com/Cyan4973/xxHash.git";
const XXHASH_CACHE_DIR: &str = "/tmp/fragile_real_world_xxhash";
const XXHASH_SORT_CC_NO_STL: &str = r#"/*
 * sort.cc - C++ sort functions (local fragile test variant)
 * This file intentionally diverges from upstream xxHash to avoid STL usage.
 */

#define XXH_INLINE_ALL  // XXH128_cmp
#include <xxhash.h>

#include "sort.hh"
#include <stdlib.h>  // qsort

static int XXH_qsort_cmp_u64(const void* lhs, const void* rhs)
{
    const uint64_t l = *(const uint64_t*)lhs;
    const uint64_t r = *(const uint64_t*)rhs;
    return (l > r) - (l < r);
}

void sort64(uint64_t* table, size_t size)
{
    qsort(table, size, sizeof(*table), XXH_qsort_cmp_u64);
}

void sort128(XXH128_hash_t* table, size_t size)
{
    qsort(table, size, sizeof(*table), XXH128_cmp);
}
"#;
const XXHASH_CLI_FILES: &[&str] = &[
    "xxhash.c",
    "cli/xsum_arch.c",
    "cli/xsum_output.c",
    "cli/xsum_sanity_check.c",
    "cli/xsum_bench.c",
    "cli/xsum_os_specific.c",
    "cli/xxhsum.c",
];

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

fn patch_xxhash_no_stl_sort(repo_dir: &Path) -> Result<(), String> {
    let sort_cc = repo_dir.join("tests/collisions/sort.cc");
    if !sort_cc.exists() {
        return Ok(());
    }

    let existing = fs::read_to_string(&sort_cc)
        .map_err(|e| format!("failed to read {}: {}", sort_cc.display(), e))?;
    if existing == XXHASH_SORT_CC_NO_STL {
        return Ok(());
    }

    // Only rewrite known STL-based upstream file, so we don't clobber manual local edits.
    if !(existing.contains("<algorithm>") || existing.contains("std::sort")) {
        return Ok(());
    }

    fs::write(&sort_cc, XXHASH_SORT_CC_NO_STL)
        .map_err(|e| format!("failed to write {}: {}", sort_cc.display(), e))?;
    Ok(())
}

fn ensure_xxhash_checkout() -> Result<PathBuf, String> {
    let repo_dir = PathBuf::from(XXHASH_CACHE_DIR);
    if repo_dir.join("xxhash.c").exists() && repo_dir.join("xxhash.h").exists() {
        patch_xxhash_no_stl_sort(&repo_dir)?;
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
                patch_xxhash_no_stl_sort(&repo_dir)?;
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

    patch_xxhash_no_stl_sort(&repo_dir)?;
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

fn transpile_xxhash_cli(repo_dir: &Path) -> Result<String, String> {
    // Force scalar XXH3 path for transpilation parity until SIMD vector lowering is complete.
    let parser = ClangParser::with_paths_and_defines(
        Vec::new(),
        vec!["XXH_VECTOR=XXH_SCALAR".to_string()],
    )
    .map_err(|e| format!("failed to create parser: {}", e))?;
    let mut combined_children = Vec::new();
    for rel in XXHASH_CLI_FILES {
        let src = repo_dir.join(rel);
        let ast = parser
            .parse_file(&src)
            .map_err(|e| format!("failed to parse {}: {}", src.display(), e))?;
        combined_children.extend(ast.translation_unit.children);
    }

    let combined_tu = ClangNode::new(ClangNodeKind::TranslationUnit).with_children(combined_children);
    Ok(AstCodeGen::new().generate(&combined_tu))
}

fn transpile_with_defines(path: &Path, defines: &[&str]) -> Result<String, String> {
    let parser = ClangParser::with_paths_and_defines(
        Vec::new(),
        defines.iter().map(|d| d.to_string()).collect(),
    )
    .map_err(|e| format!("failed to create parser with defines for {}: {}", path.display(), e))?;

    let ast = parser
        .parse_file(path)
        .map_err(|e| format!("failed to parse {} with defines {:?}: {}", path.display(), defines, e))?;
    Ok(AstCodeGen::new().generate(&ast.translation_unit))
}

fn compile_native_xxhsum(repo_dir: &Path, out_bin: &Path) -> Result<(), String> {
    let mut cmd = Command::new("cc");
    cmd.arg("-O2").arg("-std=c99");
    for rel in XXHASH_CLI_FILES {
        cmd.arg(repo_dir.join(rel));
    }
    cmd.arg("-o").arg(out_bin);
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run cc: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cc failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn compile_transpiled_xxhsum_runner(
    generated_rs: &Path,
    out_rlib: &Path,
    wrapper_rs: &Path,
    wrapper_bin: &Path,
) -> Result<(), String> {
    let lib_out = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("--crate-name")
        .arg("transpiled_xxhsum")
        .arg("--crate-type")
        .arg("lib")
        .arg("-C")
        .arg("overflow-checks=off")
        .arg("-A")
        .arg("warnings")
        .arg(generated_rs)
        .arg("-o")
        .arg(out_rlib)
        .output()
        .map_err(|e| format!("failed to run rustc (lib): {}", e))?;
    if !lib_out.status.success() {
        return Err(format!(
            "rustc (lib) failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&lib_out.stdout),
            String::from_utf8_lossy(&lib_out.stderr)
        ));
    }

    let wrapper_src = r#"
extern crate transpiled_xxhsum;
use std::ffi::CString;
use std::os::raw::c_char;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cstrings: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.as_str()).expect("argv cannot contain NUL"))
        .collect();
    let mut argv: Vec<*const c_char> = cstrings.iter().map(|s| s.as_ptr()).collect();
    let argc = argv.len() as i32;
    argv.push(std::ptr::null());
    let code = transpiled_xxhsum::main(argc, argv.as_mut_ptr());
    std::process::exit(code);
}
"#;
    fs::write(wrapper_rs, wrapper_src)
        .map_err(|e| format!("failed to write wrapper source: {}", e))?;

    let wrapper_out = Command::new("rustc")
        .arg("--edition")
        .arg("2021")
        .arg("-A")
        .arg("warnings")
        .arg(wrapper_rs)
        .arg("--extern")
        .arg(format!("transpiled_xxhsum={}", out_rlib.display()))
        .arg("-L")
        .arg(
            out_rlib
                .parent()
                .ok_or_else(|| "wrapper path missing parent".to_string())?,
        )
        .arg("-o")
        .arg(wrapper_bin)
        .output()
        .map_err(|e| format!("failed to run rustc (wrapper): {}", e))?;
    if wrapper_out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rustc (wrapper) failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wrapper_out.stdout),
            String::from_utf8_lossy(&wrapper_out.stderr)
        ))
    }
}

fn build_xxhsum_pair(repo_dir: &Path, temp_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    fs::create_dir_all(temp_dir)
        .map_err(|e| format!("failed to create temp dir {}: {}", temp_dir.display(), e))?;

    let native_bin = temp_dir.join("native_xxhsum");
    compile_native_xxhsum(repo_dir, &native_bin)?;

    let generated = transpile_xxhash_cli(repo_dir)?;
    let transpiled_rs = temp_dir.join("xxhsum_cli_transpiled.rs");
    let transpiled_rlib = temp_dir.join("libtranspiled_xxhsum.rlib");
    let wrapper_rs = temp_dir.join("xxhsum_cli_wrapper.rs");
    let wrapper_bin = temp_dir.join("transpiled_xxhsum_runner");
    fs::write(&transpiled_rs, generated)
        .map_err(|e| format!("failed to write transpiled xxhsum source: {}", e))?;

    compile_transpiled_xxhsum_runner(&transpiled_rs, &transpiled_rlib, &wrapper_rs, &wrapper_bin)?;
    Ok((native_bin, wrapper_bin))
}

fn run_cmd(bin: &Path, cwd: Option<&Path>, args: &[&str]) -> Result<Output, String> {
    let mut cmd = Command::new(bin);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.output()
        .map_err(|e| format!("failed to run {} {:?}: {}", bin.display(), args, e))
}

fn run_make(repo_dir: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("make")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(repo_dir)
        .output()
        .map_err(|e| format!("failed to run make {:?} in {}: {}", args, repo_dir.display(), e))
}

fn run_with_stdin(bin: &Path, cwd: Option<&Path>, args: &[&str], input: &[u8]) -> Result<Output, String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(cwd.unwrap_or_else(|| Path::new(".")))
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {}", bin.display(), e))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(input)
            .map_err(|e| format!("failed to write stdin for {}: {}", bin.display(), e))?;
    }

    child
        .wait_with_output()
        .map_err(|e| format!("failed to collect output for {}: {}", bin.display(), e))
}

fn run_pair_cmd(
    native_bin: &Path,
    transpiled_bin: &Path,
    cwd: &Path,
    args: &[&str],
) -> (Output, Output) {
    let native = run_cmd(native_bin, Some(cwd), args)
        .unwrap_or_else(|e| panic!("failed to run native {:?}: {}", args, e));
    let transpiled = run_cmd(transpiled_bin, Some(cwd), args)
        .unwrap_or_else(|e| panic!("failed to run transpiled {:?}: {}", args, e));
    (native, transpiled)
}

fn run_pair_with_stdin(
    native_bin: &Path,
    transpiled_bin: &Path,
    cwd: &Path,
    args: &[&str],
    input: &[u8],
) -> (Output, Output) {
    let native = run_with_stdin(native_bin, Some(cwd), args, input)
        .unwrap_or_else(|e| panic!("failed to run native {:?} with stdin: {}", args, e));
    let transpiled = run_with_stdin(transpiled_bin, Some(cwd), args, input)
        .unwrap_or_else(|e| panic!("failed to run transpiled {:?} with stdin: {}", args, e));
    (native, transpiled)
}

fn assert_success(out: &Output, context: &str) {
    assert!(
        out.status.success(),
        "{} should succeed.\nstdout:\n{}\nstderr:\n{}",
        context,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_failure(out: &Output, context: &str) {
    assert!(
        !out.status.success(),
        "{} should fail.\nstdout:\n{}\nstderr:\n{}",
        context,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_status_matches(native: &Output, transpiled: &Output, context: &str) {
    assert_eq!(
        native.status.success(),
        transpiled.status.success(),
        "{} success mismatch.\nnative stdout:\n{}\nnative stderr:\n{}\ntranspiled stdout:\n{}\ntranspiled stderr:\n{}",
        context,
        String::from_utf8_lossy(&native.stdout),
        String::from_utf8_lossy(&native.stderr),
        String::from_utf8_lossy(&transpiled.stdout),
        String::from_utf8_lossy(&transpiled.stderr),
    );
    assert_eq!(
        native.status.code(),
        transpiled.status.code(),
        "{} exit code mismatch",
        context
    );
}

fn assert_output_contains(out: &Output, needle: &str, context: &str) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "{} expected output to contain `{}`.\nstdout:\n{}\nstderr:\n{}",
        context,
        needle,
        stdout,
        stderr
    );
}

fn first_output_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn to_arg_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
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

#[test]
#[ignore = "runtime parity WIP for full xxhsum CLI transpilation"]
fn test_real_world_xxhash_cli_runtime_stdin_matches_native() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_runtime_stdin");
    let (native_bin, wrapper_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let input = b"hello world\n";
    let native =
        run_with_stdin(&native_bin, Some(&repo_dir), &["-"], input).expect("failed to run native xxhsum");
    let transpiled = run_with_stdin(&wrapper_bin, Some(&repo_dir), &["-"], input)
        .expect("failed to run transpiled xxhsum");

    assert_success(&native, "native xxhsum stdin");
    assert_success(&transpiled, "transpiled xxhsum stdin");

    let native_line = first_output_line(&native.stdout);
    let transpiled_line = first_output_line(&transpiled.stdout);
    assert_eq!(
        transpiled_line, native_line,
        "transpiled xxhsum output should match native output for stdin input"
    );
}

#[test]
#[ignore = "runtime parity WIP for full xxhsum CLI transpilation"]
fn test_real_world_xxhash_cli_runtime_file_matches_native() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_runtime_file");
    let (native_bin, wrapper_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let input_file = temp_dir.join("runtime_input.txt");
    fs::write(&input_file, b"hello world\n").expect("failed to write runtime input file");
    let input_file_str = input_file.to_string_lossy().to_string();

    let native = run_cmd(&native_bin, Some(&repo_dir), &[input_file_str.as_str()])
        .expect("failed to run native xxhsum");
    let transpiled = run_cmd(&wrapper_bin, Some(&repo_dir), &[input_file_str.as_str()])
        .expect("failed to run transpiled xxhsum");
    assert_success(&native, "native xxhsum file");
    assert_success(&transpiled, "transpiled xxhsum file");

    let native_line = first_output_line(&native.stdout);
    let transpiled_line = first_output_line(&transpiled.stdout);
    assert_eq!(
        transpiled_line, native_line,
        "transpiled xxhsum output should match native output for file input"
    );
}

#[test]
#[ignore = "runtime parity WIP for make check/default format matrix"]
fn test_real_world_xxhash_cli_default_modes_and_formats_match_native() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_runtime_hash_modes");
    let (native_bin, transpiled_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let input_file = temp_dir.join("hash_modes_input.txt");
    fs::write(&input_file, b"hello world\n").expect("failed to write hash mode input file");
    let input_file_str = input_file.to_string_lossy().to_string();

    let cases = vec![
        vec![input_file_str.clone()],
        vec!["--tag".to_string(), input_file_str.clone()],
        vec!["--little-endian".to_string(), input_file_str.clone()],
        vec!["--tag".to_string(), "--little-endian".to_string(), input_file_str.clone()],
    ];

    for args in cases {
        let arg_refs = to_arg_refs(&args);
        let native = run_cmd(&native_bin, Some(&repo_dir), &arg_refs).expect("failed to run native xxhsum");
        let transpiled =
            run_cmd(&transpiled_bin, Some(&repo_dir), &arg_refs).expect("failed to run transpiled xxhsum");
        let context = format!("args {:?}", args);

        assert_status_matches(&native, &transpiled, &context);
        assert_success(&native, &format!("native {}", context));
        assert_success(&transpiled, &format!("transpiled {}", context));
        assert_eq!(
            first_output_line(&transpiled.stdout),
            first_output_line(&native.stdout),
            "first output line should match for {}",
            context
        );
    }
}

#[test]
#[ignore = "runtime parity WIP for make check command parity (supported subset)"]
fn test_real_world_xxhash_cli_make_check_command_status_matches_native() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_runtime_make_check");
    let (native_bin, transpiled_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let xxhash_c = repo_dir.join("xxhash.c");
    let xxhash_h = repo_dir.join("xxhash.h");
    let xxhash_c_str = xxhash_c.to_string_lossy().to_string();
    let xxhash_h_str = xxhash_h.to_string_lossy().to_string();

    let stdin_input = fs::read(&xxhash_c).expect("failed to read xxhash.c");
    let native_stdin =
        run_with_stdin(&native_bin, Some(&repo_dir), &["-"], &stdin_input).expect("failed to run native stdin");
    let transpiled_stdin = run_with_stdin(&transpiled_bin, Some(&repo_dir), &["-"], &stdin_input)
        .expect("failed to run transpiled stdin");
    assert_status_matches(&native_stdin, &transpiled_stdin, "make check stdin");
    assert_success(&native_stdin, "native make check stdin");
    assert_success(&transpiled_stdin, "transpiled make check stdin");
    assert_eq!(
        first_output_line(&transpiled_stdin.stdout),
        first_output_line(&native_stdin.stdout),
        "stdin hash line should match native"
    );

    let multi_file_args = [xxhash_c_str.as_str(), xxhash_h_str.as_str()];
    let native_multi =
        run_cmd(&native_bin, Some(&repo_dir), &multi_file_args).expect("failed to run native multi-file");
    let transpiled_multi =
        run_cmd(&transpiled_bin, Some(&repo_dir), &multi_file_args).expect("failed to run transpiled multi-file");
    assert_status_matches(&native_multi, &transpiled_multi, "make check multiple files");
    assert_success(&native_multi, "native make check multiple files");
    assert_success(&transpiled_multi, "transpiled make check multiple files");
    assert_eq!(
        String::from_utf8_lossy(&transpiled_multi.stdout),
        String::from_utf8_lossy(&native_multi.stdout),
        "multiple file hash output should match native"
    );

    let status_only_cases: Vec<(Vec<String>, bool)> = vec![
        (vec!["--definitely-invalid-option".to_string()], false),
    ];

    for (args, expect_success) in status_only_cases {
        let arg_refs = to_arg_refs(&args);
        let native = run_cmd(&native_bin, Some(&repo_dir), &arg_refs).expect("failed to run native status-only case");
        let transpiled =
            run_cmd(&transpiled_bin, Some(&repo_dir), &arg_refs).expect("failed to run transpiled status-only case");
        let context = format!("status-only args {:?}", args);

        assert_status_matches(&native, &transpiled, &context);
        if expect_success {
            assert_success(&native, &format!("native {}", context));
            assert_success(&transpiled, &format!("transpiled {}", context));
        } else {
            assert_failure(&native, &format!("native {}", context));
            assert_failure(&transpiled, &format!("transpiled {}", context));
        }
    }
}

#[test]
#[ignore = "runtime parity WIP for make check + test-xxhsum-c matrix"]
fn test_real_world_xxhash_cli_make_check_and_test_xxhsum_c_matrix_matches_native() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_make_check_matrix");
    let (native_bin, transpiled_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let xxhash_c = repo_dir.join("xxhash.c");
    let xxhash_h = repo_dir.join("xxhash.h");
    let test_files = [
        xxhash_c.to_string_lossy().to_string(),
        xxhash_h.to_string_lossy().to_string(),
    ];
    let file_refs = [test_files[0].as_str(), test_files[1].as_str()];

    // make check parity core: stdin, benchmark commands, and hash variants.
    let stdin_input = fs::read(&xxhash_c).expect("failed to read xxhash.c");
    let (native_stdin, transpiled_stdin) =
        run_pair_with_stdin(&native_bin, &transpiled_bin, &repo_dir, &["-"], &stdin_input);
    assert_status_matches(&native_stdin, &transpiled_stdin, "make check stdin");
    assert_success(&native_stdin, "native make check stdin");
    assert_success(&transpiled_stdin, "transpiled make check stdin");

    let status_only_cases: Vec<(Vec<String>, bool)> = vec![
        (vec!["-bi0".to_string()], true),
        (vec!["--benchmark-all".to_string(), "-i0".to_string()], true),
        (vec!["-b1,2,3".to_string(), "-i0".to_string()], true),
        (vec!["-bi0".to_string(), test_files[0].clone()], true),
        (vec!["-H0".to_string(), test_files[0].clone()], true),
        (vec!["-H2".to_string(), test_files[0].clone()], true),
        (vec!["-H3".to_string(), test_files[0].clone()], true),
        (vec!["-H9".to_string(), test_files[0].clone()], false),
    ];

    for (args, expect_success) in status_only_cases {
        let arg_refs = to_arg_refs(&args);
        let (native, transpiled) = run_pair_cmd(&native_bin, &transpiled_bin, &repo_dir, &arg_refs);
        let context = format!("make check status-only args {:?}", args);
        assert_status_matches(&native, &transpiled, &context);
        if expect_success {
            assert_success(&native, &format!("native {}", context));
            assert_success(&transpiled, &format!("transpiled {}", context));
        } else {
            assert_failure(&native, &format!("native {}", context));
            assert_failure(&transpiled, &format!("transpiled {}", context));
        }
        if args.first().is_some_and(|a| a == "-H3") {
            assert_output_contains(&native, "XXH3", "native -H3 output");
            assert_output_contains(&transpiled, "XXH3", "transpiled -H3 output");
        }
    }

    // test-xxhsum-c parity: checksum generation and self-check commands.
    let (native_sum, transpiled_sum) = run_pair_cmd(&native_bin, &transpiled_bin, &repo_dir, &file_refs);
    assert_status_matches(&native_sum, &transpiled_sum, "checksum generation");
    assert_success(&native_sum, "native checksum generation");
    assert_success(&transpiled_sum, "transpiled checksum generation");
    assert_eq!(
        String::from_utf8_lossy(&transpiled_sum.stdout),
        String::from_utf8_lossy(&native_sum.stdout),
        "checksum generation output should match native"
    );

    let (native_sum_h0, transpiled_sum_h0) =
        run_pair_cmd(&native_bin, &transpiled_bin, &repo_dir, &["-H0", file_refs[0], file_refs[1]]);
    assert_status_matches(&native_sum_h0, &transpiled_sum_h0, "checksum generation H0");
    assert_success(&native_sum_h0, "native checksum generation H0");
    assert_success(&transpiled_sum_h0, "transpiled checksum generation H0");

    let native_check_from_native_sum = run_with_stdin(&native_bin, Some(&repo_dir), &["-c", "-"], &native_sum.stdout)
        .expect("failed to run native -c with stdin");
    let transpiled_check_from_transpiled_sum =
        run_with_stdin(&transpiled_bin, Some(&repo_dir), &["-c", "-"], &transpiled_sum.stdout)
            .expect("failed to run transpiled -c with stdin");
    assert_success(&native_check_from_native_sum, "native -c with native generated checksums");
    assert_success(
        &transpiled_check_from_transpiled_sum,
        "transpiled -c with transpiled generated checksums",
    );

    // Additional behavior checks used by make target.
    let (native_q, transpiled_q) = run_pair_cmd(&native_bin, &transpiled_bin, &repo_dir, &["-q", file_refs[0], file_refs[1]]);
    assert_status_matches(&native_q, &transpiled_q, "-q loading message behavior");
    assert_success(&native_q, "native -q");
    assert_success(&transpiled_q, "transpiled -q");
    assert!(
        !String::from_utf8_lossy(&native_q.stderr).contains("Loading")
            && !String::from_utf8_lossy(&transpiled_q.stderr).contains("Loading"),
        "both native and transpiled should not print Loading in -q mode"
    );

    let (native_nonexistent, transpiled_nonexistent) =
        run_pair_cmd(&native_bin, &transpiled_bin, &repo_dir, &["nonexistent"]);
    assert_status_matches(&native_nonexistent, &transpiled_nonexistent, "nonexistent file error");
    assert_failure(&native_nonexistent, "native nonexistent");
    assert_failure(&transpiled_nonexistent, "transpiled nonexistent");
    assert_output_contains(
        &native_nonexistent,
        "nonexistent",
        "native nonexistent filename mention",
    );
    assert_output_contains(
        &transpiled_nonexistent,
        "nonexistent",
        "transpiled nonexistent filename mention",
    );

    let filelist_path = temp_dir.join("files.txt");
    fs::write(&filelist_path, format!("{}\n", file_refs[0])).expect("failed to write filelist");
    let filelist_path_str = filelist_path.to_string_lossy().to_string();
    let filelist_args = ["--filelist", filelist_path_str.as_str()];
    let (native_filelist, transpiled_filelist) =
        run_pair_cmd(&native_bin, &transpiled_bin, &repo_dir, &filelist_args);
    assert_status_matches(&native_filelist, &transpiled_filelist, "--filelist");
    assert_success(&native_filelist, "native --filelist");
    assert_success(&transpiled_filelist, "transpiled --filelist");

}

#[test]
#[ignore = "runtime parity WIP for upstream make test with transpiled xxhsum drop-in"]
fn test_real_world_xxhash_make_test_passes_with_transpiled_xxhsum_dropin() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_make_test_dropin");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let (_native_bin, transpiled_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let xxhsum_path = repo_dir.join("xxhsum");
    let backup_path = temp_dir.join("xxhsum.native.backup");
    let had_backup = xxhsum_path.exists();
    if had_backup {
        fs::copy(&xxhsum_path, &backup_path)
            .unwrap_or_else(|e| panic!("failed to back up {}: {}", xxhsum_path.display(), e));
    }

    let run_result = (|| -> Result<(), String> {
        let make_all = run_make(&repo_dir, &["all"])?;
        if !make_all.status.success() {
            return Err(format!(
                "make all failed:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&make_all.stdout),
                String::from_utf8_lossy(&make_all.stderr),
            ));
        }

        fs::copy(&transpiled_bin, &xxhsum_path)
            .map_err(|e| format!("failed to install transpiled xxhsum drop-in: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&xxhsum_path)
                .map_err(|e| format!("failed to stat {}: {}", xxhsum_path.display(), e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&xxhsum_path, perms)
                .map_err(|e| format!("failed to set executable permissions: {}", e))?;
        }

        let make_test = run_make(&repo_dir, &["test"])?;
        if !make_test.status.success() {
            return Err(format!(
                "make test failed with transpiled drop-in:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&make_test.stdout),
                String::from_utf8_lossy(&make_test.stderr),
            ));
        }

        let installed = fs::read(&xxhsum_path)
            .map_err(|e| format!("failed to read installed xxhsum: {}", e))?;
        let transpiled = fs::read(&transpiled_bin)
            .map_err(|e| format!("failed to read transpiled xxhsum: {}", e))?;
        if installed != transpiled {
            return Err("make test rebuilt and replaced transpiled xxhsum drop-in".to_string());
        }

        Ok(())
    })();

    if had_backup {
        let _ = fs::copy(&backup_path, &xxhsum_path);
    }

    if let Err(e) = run_result {
        panic!("{}", e);
    }
}

#[test]
#[ignore = "runtime parity WIP for make test-xxhsum-c checksum generation subset"]
fn test_real_world_xxhash_cli_checksum_generation_matches_native() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_runtime_checksum_generation");
    let (native_bin, transpiled_bin) =
        build_xxhsum_pair(&repo_dir, &temp_dir).expect("failed to build xxhsum native/transpiled pair");

    let file_a = temp_dir.join("sum_file_a.txt");
    let file_b = temp_dir.join("sum_file_b.txt");
    fs::write(&file_a, b"alpha\n").expect("failed to write sum file A");
    fs::write(&file_b, b"beta\n").expect("failed to write sum file B");
    let file_a_str = file_a.to_string_lossy().to_string();
    let file_b_str = file_b.to_string_lossy().to_string();

    let files_args = [file_a_str.as_str(), file_b_str.as_str()];
    let native_sums = run_cmd(&native_bin, Some(&repo_dir), &files_args).expect("failed to run native sums");
    let transpiled_sums =
        run_cmd(&transpiled_bin, Some(&repo_dir), &files_args).expect("failed to run transpiled sums");
    assert_status_matches(&native_sums, &transpiled_sums, "xxhsum output generation");
    assert_success(&native_sums, "native xxhsum output generation");
    assert_success(&transpiled_sums, "transpiled xxhsum output generation");
    assert_eq!(
        String::from_utf8_lossy(&transpiled_sums.stdout),
        String::from_utf8_lossy(&native_sums.stdout),
        "generated checksum lines should match native"
    );
}

#[test]
#[ignore = "compile-variant parity WIP for make noxxh3/c90/nostdlib tests"]
fn test_real_world_xxhash_make_define_variants_transpile_and_compile() {
    let repo_dir = ensure_xxhash_checkout().expect("failed to prepare xxHash checkout");
    let xxhash_c = repo_dir.join("xxhash.c");
    let temp_dir = std::env::temp_dir().join("fragile_real_world_xxhash_define_variants");
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let variants: Vec<(&str, Vec<&str>, Vec<&str>)> = vec![
        (
            "xxh_noxxh3",
            vec!["XXH_NO_XXH3"],
            vec!["pub fn XXH3_64bits", "pub fn XXH3_128bits"],
        ),
        ("xxh_nolonglong", vec!["XXH_NO_LONG_LONG"], vec!["pub fn XXH64"]),
        ("xxh_nostdlib", vec!["XXH_NO_STDLIB"], vec![]),
    ];

    for (name, defines, absent_symbols) in variants {
        let rust_code = transpile_with_defines(&xxhash_c, &defines)
            .unwrap_or_else(|e| panic!("{} define variant should transpile: {}", name, e));

        for absent in absent_symbols {
            assert!(
                !rust_code.contains(absent),
                "{} variant should not include symbol `{}`",
                name,
                absent
            );
        }

        let rs_path = temp_dir.join(format!("{}_transpiled.rs", name));
        let out_path = temp_dir.join(format!("lib{}_transpiled.rlib", name));
        fs::write(&rs_path, rust_code).expect("failed to write variant Rust source");
        compile_rust_file(&rs_path, &out_path, "lib")
            .unwrap_or_else(|e| panic!("{} variant should compile as Rust library: {}", name, e));
    }
}
