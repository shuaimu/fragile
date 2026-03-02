//! Focused yaml-cpp compile-only regressions for fragilec.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn workspace_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve workspace root")
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

fn yamlcpp_fixture_paths() -> Option<(PathBuf, PathBuf)> {
    let root = workspace_root_dir();
    let source = root.join("vendor/mako/dependencies/yaml-cpp/src/null.cpp");
    let include = root.join("vendor/mako/dependencies/yaml-cpp/include");
    if source.exists() && include.exists() {
        Some((source, include))
    } else {
        None
    }
}

#[test]
fn test_real_world_yamlcpp_null_cpp_fragilec_compile_only() {
    let Some((source, include)) = yamlcpp_fixture_paths() else {
        eprintln!("skipping yaml-cpp null.cpp regression: fixture not present under vendor/mako");
        return;
    };
    let fragilec = ensure_fragilec_binary().expect("failed to resolve fragilec binary");
    let out_dir = std::env::temp_dir().join("fragile_real_world_yamlcpp_null_cpp");
    std::fs::create_dir_all(&out_dir).expect("failed to create yaml-cpp temp output directory");
    let out_obj = out_dir.join("null.o");

    let output = Command::new(&fragilec)
        .arg("-c")
        .arg("-I")
        .arg(&include)
        .arg(&source)
        .arg("-o")
        .arg(&out_obj)
        .env("FRAGILEC_MODE", "strict")
        .env("FRAGILEC_PARSER_BACKEND", "libtooling")
        .output()
        .expect("failed to run fragilec for yaml-cpp null.cpp compile-only regression");

    assert!(
        output.status.success(),
        "fragilec -c yaml-cpp null.cpp should succeed\ncommand: {} -c -I {} {} -o {}\nstdout:\n{}\nstderr:\n{}",
        fragilec.display(),
        include.display(),
        source.display(),
        out_obj.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        out_obj.exists(),
        "fragilec reported success but did not emit object file {}",
        out_obj.display()
    );
}
