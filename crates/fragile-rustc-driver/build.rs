//! Build script for fragile-rustc-driver.
//!
//! When the `rustc-integration` feature is enabled, this script finds the
//! rustc library directory and adds it to the linker search path.

fn main() {
    // Only do special setup when rustc-integration is enabled
    if std::env::var("CARGO_FEATURE_RUSTC_INTEGRATION").is_ok() {
        // Get the rustc sysroot
        let output = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .expect("Failed to run rustc --print sysroot");

        let sysroot = String::from_utf8(output.stdout)
            .expect("Invalid UTF-8 in sysroot path")
            .trim()
            .to_string();

        // The rustc libraries are in <sysroot>/lib/rustlib/<target>/lib
        let target = std::env::var("TARGET").expect("TARGET env var not set");
        let lib_dir = format!("{}/lib/rustlib/{}/lib", sysroot, target);

        // Also need the compiler library directory
        let compiler_lib_dir = format!("{}/lib", sysroot);

        // Tell cargo to add these to the library search path
        println!("cargo:rustc-link-search=native={}", lib_dir);
        println!("cargo:rustc-link-search=native={}", compiler_lib_dir);

        // Tell downstream crates about the sysroot
        println!("cargo:rustc-env=RUSTC_SYSROOT={}", sysroot);

        // Re-run if the sysroot changes (e.g., rustup update)
        println!("cargo:rerun-if-env-changed=RUSTUP_TOOLCHAIN");
    }
}
