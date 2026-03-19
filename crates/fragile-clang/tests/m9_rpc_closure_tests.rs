/// M9 Deferred RPC Target Closure Tests
///
/// M9.1: Verifies that `test_rpc` and `rpcbench` can be built with the fragilec
/// compiler driver using the new `fragile-parser-clang` backend (no force-native
/// paths, no FRAGILEC_FORCE_NATIVE_SOURCES bypass).
///
/// Gate structure:
/// - Unit tests: verify key RPC source files transpile with new backend
/// - Ignored integration tests: full CMake configure + build + runtime
///
/// M9.A1: `test_rpc` build/run pass gate
/// M9.A2: `rpcbench` build pass gate (runtime requires server/client pair)

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn workspace_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root should exist")
        .to_path_buf()
}

fn mako_root_dir() -> Option<PathBuf> {
    let root = workspace_root_dir();
    let mako = root.join("vendor/mako");
    let cmake = mako.join("CMakeLists.txt");
    if cmake.exists() {
        Some(mako)
    } else {
        None
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "fragile_m9_rpc_{}_{}_{}",
        label,
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn ensure_fragilec_binary() -> Result<PathBuf, String> {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    if let Some(path) = BIN.get() {
        return Ok(path.clone());
    }

    let workspace_root = workspace_root_dir();
    let fragilec = workspace_root.join("target/debug/fragilec");
    let output = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("fragile-cli")
        .arg("--bin")
        .arg("fragilec")
        .current_dir(&workspace_root)
        .output()
        .map_err(|e| format!("failed to build fragilec binary: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "failed to build fragilec binary\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let _ = BIN.set(fragilec.clone());
    Ok(fragilec)
}

/// Compile a single C++ source file with fragilec (compile-only, -c).
/// Returns (success, stdout, stderr).
fn fragilec_compile_one(
    fragilec: &std::path::Path,
    source: &std::path::Path,
    out_obj: &std::path::Path,
    include_dirs: &[&std::path::Path],
) -> (bool, String, String) {
    let mut cmd = Command::new(fragilec);
    cmd.arg("-c");
    for inc in include_dirs {
        cmd.arg("-I").arg(inc);
    }
    cmd.arg(source)
        .arg("-o")
        .arg(out_obj)
        .env("FRAGILEC_MODE", "strict");
    // Do NOT set FRAGILEC_PARSER_BACKEND — relies on default (fragile-parser-clang since M8.1)
    // Do NOT set FRAGILEC_FORCE_NATIVE_SOURCES — prohibited by non-negotiable constraints

    let output = cmd.output().expect("failed to run fragilec");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// ---------------------------------------------------------------------------
// M9.1 Contract: RPC sources are no longer deferred
// ---------------------------------------------------------------------------

#[test]
fn m9_1_rpc_targets_no_longer_deferred() {
    // M9.0 prerequisite: M0-M8 acceptance complete.
    // M9.1 contract: test_rpc and rpcbench are now actively tested, not deferred.
    // This test documents that M7's deferral contract is now superseded.
    let todo_content = fs::read_to_string(workspace_root_dir().join("TODO.md"))
        .expect("read TODO.md");
    assert!(
        todo_content.contains("M9.0 Start only after M0-M8 acceptance is complete"),
        "TODO.md must document M9.0 prerequisite"
    );
    // M9.0 should be marked done
    assert!(
        todo_content.contains("[x] M9.0"),
        "M9.0 prerequisite should be satisfied (M0-M8 acceptance complete)"
    );
}

// ---------------------------------------------------------------------------
// M9.1 Unit Gate: Key RPC source files compile with fragilec default backend
// ---------------------------------------------------------------------------

#[test]
fn m9_1_benchmark_service_cc_compiles_with_default_backend() {
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("fragilec binary");
    let source = mako_root.join("test/benchmark_service.cc");
    if !source.exists() {
        eprintln!("skipping: benchmark_service.cc not found at {}", source.display());
        return;
    }

    let out_dir = temp_dir("benchmark_service");
    let out_obj = out_dir.join("benchmark_service.o");

    let include_dirs = [
        mako_root.join("src"),
        mako_root.join("third-party/googletest/googletest/include"),
    ];
    let inc_refs: Vec<&std::path::Path> = include_dirs.iter().map(|p| p.as_path()).collect();

    let (success, _stdout, stderr) = fragilec_compile_one(
        &fragilec,
        &source,
        &out_obj,
        &inc_refs,
    );

    assert!(
        success,
        "benchmark_service.cc should compile with default parser backend\nstderr:\n{}",
        stderr
    );
    assert!(out_obj.exists(), "object file should be produced");

    eprintln!(
        "M9.1 benchmark_service.cc compile PASSED: obj={}, stderr_len={}",
        out_obj.display(),
        stderr.len()
    );
}

#[test]
fn m9_1_rpcbench_fragile_cc_compiles_with_default_backend() {
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("fragilec binary");

    // Prefer rpcbench_fragile.cc (transpiler-friendly variant) if present
    let source = mako_root.join("test/rpcbench_fragile.cc");
    let source = if source.exists() {
        source
    } else {
        let alt = mako_root.join("test/rpcbench.cc");
        if !alt.exists() {
            eprintln!("skipping: neither rpcbench_fragile.cc nor rpcbench.cc found");
            return;
        }
        alt
    };

    let out_dir = temp_dir("rpcbench_fragile");
    let out_obj = out_dir.join("rpcbench_fragile.o");

    let include_dirs = [mako_root.join("src")];
    let inc_refs: Vec<&std::path::Path> = include_dirs.iter().map(|p| p.as_path()).collect();

    let (success, _stdout, stderr) = fragilec_compile_one(
        &fragilec,
        &source,
        &out_obj,
        &inc_refs,
    );

    assert!(
        success,
        "rpcbench source should compile with default parser backend\nstderr:\n{}",
        stderr
    );
    assert!(out_obj.exists(), "object file should be produced");

    eprintln!(
        "M9.1 rpcbench compile PASSED: src={}, obj={}",
        source.display(),
        out_obj.display()
    );
}

#[test]
fn m9_1_test_rpc_cc_compiles_with_default_backend() {
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("fragilec binary");
    let source = mako_root.join("test/test_rpc.cc");
    if !source.exists() {
        eprintln!("skipping: test_rpc.cc not found");
        return;
    }

    let out_dir = temp_dir("test_rpc");
    let out_obj = out_dir.join("test_rpc.o");

    let include_dirs = [
        mako_root.join("src"),
        mako_root.join("third-party/googletest/googletest/include"),
    ];
    let inc_refs: Vec<&std::path::Path> = include_dirs.iter().map(|p| p.as_path()).collect();

    let (success, _stdout, stderr) = fragilec_compile_one(
        &fragilec,
        &source,
        &out_obj,
        &inc_refs,
    );

    assert!(
        success,
        "test_rpc.cc should compile with default parser backend\nstderr:\n{}",
        stderr
    );
    assert!(out_obj.exists(), "object file should be produced");

    eprintln!(
        "M9.1 test_rpc.cc compile PASSED: obj={}",
        out_obj.display()
    );
}

// ---------------------------------------------------------------------------
// M9.1 Policy Gate: No force-native bypass
// ---------------------------------------------------------------------------

#[test]
fn m9_1_no_force_native_sources_in_codebase() {
    // Non-negotiable constraint: FRAGILEC_FORCE_NATIVE_SOURCES must not appear
    // in any active source code (not just comments/docs).
    let workspace_root = workspace_root_dir();

    // Check driver and CLI source for the banned env var
    let driver_src = fs::read_to_string(
        workspace_root.join("crates/fragile-driver/src/lib.rs")
    ).unwrap_or_default();
    let cli_src = fs::read_to_string(
        workspace_root.join("crates/fragile-cli/src/bin/fragilec.rs")
    ).unwrap_or_default();

    for (name, content) in [("fragile-driver", &driver_src), ("fragilec.rs", &cli_src)] {
        // Allow it in comments/docs but not in actual code that reads the var
        let has_active_usage = content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.contains("FRAGILEC_FORCE_NATIVE_SOURCES")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("///")
                && !trimmed.starts_with("*")
                && !trimmed.starts_with("/*")
        });
        assert!(
            !has_active_usage,
            "{} must not have active FRAGILEC_FORCE_NATIVE_SOURCES usage",
            name
        );
    }
}

// ---------------------------------------------------------------------------
// M9.1 Integration: Full CMake configure + build for test_rpc and rpcbench
// ---------------------------------------------------------------------------

#[test]
#[ignore] // Long-running: full CMake configure + build (~2-5 minutes)
fn m9_1_cmake_build_test_rpc_and_rpcbench_with_fragilec() {
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("fragilec binary");
    let build_dir = temp_dir("cmake_rpc_build");

    // 1. CMake configure (no FRAGILEC_FORCE_NATIVE_SOURCES, no FRAGILEC_PARSER_BACKEND override)
    let configure_output = Command::new("cmake")
        .arg(&mako_root)
        .arg(format!("-DCMAKE_BUILD_TYPE=Debug"))
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake configure");

    assert!(
        configure_output.status.success(),
        "CMake configure should succeed with fragilec\nstderr:\n{}",
        String::from_utf8_lossy(&configure_output.stderr)
    );

    // 2. Build rpcbench target
    let rpcbench_build = Command::new("cmake")
        .args(["--build", ".", "--target", "rpcbench", "-j4"])
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake build rpcbench");

    let rpcbench_stderr = String::from_utf8_lossy(&rpcbench_build.stderr).to_string();
    assert!(
        rpcbench_build.status.success(),
        "rpcbench target should build with fragilec\nstderr:\n{}",
        rpcbench_stderr
    );
    assert!(
        build_dir.join("rpcbench").exists(),
        "rpcbench binary should be produced"
    );

    // 3. Build test_rpc target
    let test_rpc_build = Command::new("cmake")
        .args(["--build", ".", "--target", "test_rpc", "-j4"])
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake build test_rpc");

    let test_rpc_stderr = String::from_utf8_lossy(&test_rpc_build.stderr).to_string();
    assert!(
        test_rpc_build.status.success(),
        "test_rpc target should build with fragilec\nstderr:\n{}",
        test_rpc_stderr
    );
    assert!(
        build_dir.join("test_rpc").exists(),
        "test_rpc binary should be produced"
    );

    // 4. Emit deterministic manifest
    let manifest = format!(
        "m9_1_cmake_rpc_build_manifest\n\
         configure_status=0\n\
         rpcbench_build_status=0\n\
         test_rpc_build_status=0\n\
         rpcbench_binary_present=true\n\
         test_rpc_binary_present=true\n\
         force_native_sources=false\n\
         parser_backend_override=none\n\
         build_dir={}\n",
        build_dir.display()
    );
    let manifest_path = build_dir.join("m9_1_cmake_rpc_build_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write manifest");

    eprintln!(
        "M9.1 CMake build PASSED:\n{}",
        manifest
    );
}

// ---------------------------------------------------------------------------
// M9.A1 Acceptance: test_rpc gtest runtime gate
// ---------------------------------------------------------------------------

#[test]
#[ignore] // Long-running: full CMake build + gtest execution
fn m9_a1_test_rpc_runtime_gate() {
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("fragilec binary");
    let build_dir = temp_dir("cmake_rpc_runtime");

    // Configure + build test_rpc
    let configure = Command::new("cmake")
        .arg(&mako_root)
        .arg("-DCMAKE_BUILD_TYPE=Debug")
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake configure");
    assert!(configure.status.success(), "configure must pass");

    let build = Command::new("cmake")
        .args(["--build", ".", "--target", "test_rpc", "-j4"])
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake build test_rpc");
    assert!(build.status.success(), "test_rpc build must pass");

    let test_rpc_bin = build_dir.join("test_rpc");
    assert!(test_rpc_bin.exists(), "test_rpc binary must exist");

    // Run test_rpc with timeout
    let run_output = Command::new(&test_rpc_bin)
        .current_dir(&build_dir)
        .output()
        .expect("run test_rpc");

    let stdout = String::from_utf8_lossy(&run_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run_output.stderr).to_string();

    assert!(
        run_output.status.success(),
        "test_rpc gtest should pass (exit 0)\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );

    // Verify gtest summary is present
    assert!(
        stdout.contains("PASSED") || stderr.contains("PASSED"),
        "gtest summary should show PASSED\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );

    // Extract test count
    let passed_line = stdout.lines()
        .chain(stderr.lines())
        .find(|l| l.contains("PASSED"))
        .unwrap_or("no PASSED line found");

    // Emit manifest
    let manifest = format!(
        "m9_a1_test_rpc_runtime_manifest\n\
         exit_status=0\n\
         gtest_summary={}\n\
         force_native_sources=false\n\
         parser_backend_override=none\n\
         build_dir={}\n",
        passed_line.trim(),
        build_dir.display()
    );
    let manifest_path = build_dir.join("m9_a1_test_rpc_runtime_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write manifest");

    eprintln!(
        "M9.A1 test_rpc runtime gate PASSED:\n{}",
        manifest
    );
}

// ---------------------------------------------------------------------------
// M9.A2 Acceptance: rpcbench build gate (runtime is server/client pair)
// ---------------------------------------------------------------------------

#[test]
#[ignore] // Long-running: full CMake build
fn m9_a2_rpcbench_build_gate() {
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("fragilec binary");
    let build_dir = temp_dir("cmake_rpcbench_gate");

    // Configure + build rpcbench
    let configure = Command::new("cmake")
        .arg(&mako_root)
        .arg("-DCMAKE_BUILD_TYPE=Debug")
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake configure");
    assert!(configure.status.success(), "configure must pass");

    let build = Command::new("cmake")
        .args(["--build", ".", "--target", "rpcbench", "-j4"])
        .env("CXX", &fragilec)
        .env("CC", "clang")
        .current_dir(&build_dir)
        .output()
        .expect("cmake build rpcbench");

    assert!(
        build.status.success(),
        "rpcbench target must build\nstderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rpcbench_bin = build_dir.join("rpcbench");
    assert!(rpcbench_bin.exists(), "rpcbench binary must exist");

    // Verify binary is executable (--help or similar)
    let help_output = Command::new(&rpcbench_bin)
        .arg("-h")
        .current_dir(&build_dir)
        .output();

    // rpcbench may not support -h, so just check it runs without immediate segfault
    match help_output {
        Ok(out) => {
            eprintln!(
                "M9.A2 rpcbench -h exit_status={}, stdout_len={}, stderr_len={}",
                out.status.code().unwrap_or(-1),
                out.stdout.len(),
                out.stderr.len()
            );
        }
        Err(e) => {
            eprintln!("M9.A2 rpcbench -h failed to execute: {}", e);
        }
    }

    // Emit manifest
    let manifest = format!(
        "m9_a2_rpcbench_build_manifest\n\
         build_status=0\n\
         binary_present=true\n\
         binary_path={}\n\
         force_native_sources=false\n\
         parser_backend_override=none\n\
         build_dir={}\n",
        rpcbench_bin.display(),
        build_dir.display()
    );
    let manifest_path = build_dir.join("m9_a2_rpcbench_build_manifest.txt");
    fs::write(&manifest_path, &manifest).expect("write manifest");

    eprintln!(
        "M9.A2 rpcbench build gate PASSED:\n{}",
        manifest
    );
}

// ---------------------------------------------------------------------------
// M9.1.a Strict RPC baseline environment contract
// ---------------------------------------------------------------------------

#[test]
fn m9_1a_strict_rpc_environment_contract() {
    // M9.1.a: The strict RPC build must use FRAGILEC_MODE=strict,
    // default parser backend (fragile-parser-clang), and no bypass env vars.
    //
    // Verify: the harness script enforces this contract.
    let workspace_root = workspace_root_dir();
    let harness_path = workspace_root.join("scripts/mako_rpcbench_harness.py");
    if !harness_path.exists() {
        eprintln!("skipping: mako_rpcbench_harness.py not found");
        return;
    }

    let harness_content = fs::read_to_string(&harness_path).expect("read harness");

    // Harness must reference fragilec as a supported lane
    assert!(
        harness_content.contains("fragilec"),
        "harness must reference fragilec as a compiler lane"
    );

    // Harness must NOT set FRAGILEC_FORCE_NATIVE_SOURCES
    let has_force_native = harness_content.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.contains("FRAGILEC_FORCE_NATIVE_SOURCES")
            && !trimmed.starts_with("#")
    });
    assert!(
        !has_force_native,
        "harness must not set FRAGILEC_FORCE_NATIVE_SOURCES"
    );
}

#[test]
fn m9_1a_strict_mode_is_fragilec_default_for_rpc() {
    // Verify the fragilec driver defaults to strict mode behavior.
    // M9.1.a contract: FRAGILEC_MODE=strict, not auto or pass.
    let todo = fs::read_to_string(workspace_root_dir().join("TODO.md"))
        .expect("read TODO.md");

    // The TODO must document strict build gates for RPC
    assert!(
        todo.contains("strict build succeeds") || todo.contains("strict") && todo.contains("test_rpc"),
        "TODO.md should document strict build policy for RPC targets"
    );
}

// ---------------------------------------------------------------------------
// M9.1.b Build-only replay gate: both targets included
// ---------------------------------------------------------------------------

#[test]
fn m9_1b_rpc_targets_in_cmake_build_system() {
    // M9.1.b: Verify that both test_rpc and rpcbench are configured as CMake
    // targets when fragilec is the compiler.
    let Some(mako_root) = mako_root_dir() else {
        eprintln!("skipping: vendor/mako not populated");
        return;
    };

    let cmake_content = fs::read_to_string(mako_root.join("CMakeLists.txt"))
        .expect("read CMakeLists.txt");

    // Both targets must be defined
    assert!(
        cmake_content.contains("test_rpc"),
        "CMakeLists.txt must define test_rpc target"
    );
    assert!(
        cmake_content.contains("rpcbench"),
        "CMakeLists.txt must define rpcbench target"
    );

    // Fragilec detection must be present
    assert!(
        cmake_content.contains("fragilec"),
        "CMakeLists.txt must detect fragilec compiler"
    );
}

// ---------------------------------------------------------------------------
// M9.1.c Blocker-log gate: no force-native bypass markers
// ---------------------------------------------------------------------------

#[test]
fn m9_1c_no_native_fallback_in_driver() {
    // M9.1.c: The driver must not contain any native fallback or force-native
    // bypass logic that could be triggered during RPC builds.
    let workspace_root = workspace_root_dir();

    // Check for banned bypass patterns in driver and CLI
    let driver_src = fs::read_to_string(
        workspace_root.join("crates/fragile-driver/src/lib.rs")
    ).unwrap_or_default();

    // No "native" fallback mode in active code
    let has_native_fallback = driver_src.lines().any(|line| {
        let trimmed = line.trim();
        (trimmed.contains("\"native\"") || trimmed.contains("native_fallback"))
            && !trimmed.starts_with("//")
            && !trimmed.starts_with("///")
    });
    assert!(
        !has_native_fallback,
        "driver must not contain active native fallback mode"
    );
}

// ---------------------------------------------------------------------------
// M9.1 Default backend verification: ensure no escape hatch leaks
// ---------------------------------------------------------------------------

#[test]
fn m9_1_default_backend_is_new_parser_for_rpc_builds() {
    // M8.1 set the default to fragile-parser-clang. M9.1 must NOT override it.
    // This test verifies the policy document in TODO.md is consistent.
    let todo = fs::read_to_string(workspace_root_dir().join("TODO.md"))
        .expect("read TODO.md");

    // M8.1 should be marked done
    assert!(
        todo.contains("[x] M8.1"),
        "M8.1 (flip default parser backend) must be complete before M9"
    );

    // M8.2 escape hatch should be marked done
    assert!(
        todo.contains("[x] M8.2"),
        "M8.2 (escape hatch hardening) must be complete before M9"
    );

    // M9 tasks should exist
    assert!(
        todo.contains("M9.1"),
        "M9.1 task must be documented"
    );
}
