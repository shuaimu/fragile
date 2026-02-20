use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("failed to resolve workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn repro_script_path() -> PathBuf {
    workspace_root().join("scripts").join("repro_zlib_failure.sh")
}

#[test]
fn test_zlib_failure_repro_script_exists() {
    let script = repro_script_path();
    assert!(
        script.exists(),
        "expected zlib failure repro script at {}",
        script.display()
    );
}

#[cfg(unix)]
#[test]
fn test_zlib_failure_repro_script_is_executable() {
    let script = repro_script_path();
    let permissions = std::fs::metadata(&script)
        .expect("failed to read script metadata")
        .permissions();
    assert!(
        permissions.mode() & 0o111 != 0,
        "expected script to be executable: {}",
        script.display()
    );
}

#[test]
fn test_zlib_failure_repro_script_prints_canonical_command() {
    let script = repro_script_path();
    let output = Command::new(&script)
        .arg("--print")
        .output()
        .expect("failed to run zlib failure repro script with --print");

    assert!(
        output.status.success(),
        "script --print should succeed, status={:?}",
        output.status.code()
    );
    assert!(
        output.stderr.is_empty(),
        "script --print should not write stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected =
        "cargo test -p fragile-clang --test real_world_zlib_tests test_real_world_zlib_make_test_command_subset_replay -- --ignored --nocapture --test-threads=1";
    assert_eq!(stdout.trim(), expected, "unexpected repro command");
}
