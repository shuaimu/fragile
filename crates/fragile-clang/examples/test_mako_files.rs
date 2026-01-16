use fragile_clang::{ClangParser, MirConverter};
use std::path::Path;
use std::fs;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();

    // Stubs path
    let stubs_path = Path::new(manifest_dir).join("stubs");

    // Mako include paths
    let mako_rrr_path = project_root.join("vendor/mako/src/rrr");
    let mako_src_path = project_root.join("vendor/mako/src");
    let rusty_cpp_path = project_root.join("vendor/mako/third-party/rusty-cpp/include");

    // Find all mako module files
    let mako_path = project_root.join("vendor/mako/src/mako");
    let mut files = Vec::new();

    fn collect_cpp_files(dir: &Path, files: &mut Vec<String>, project_root: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_cpp_files(&path, files, project_root);
                } else if let Some(ext) = path.extension() {
                    if ext == "cpp" || ext == "cc" {
                        if let Ok(rel) = path.strip_prefix(project_root) {
                            files.push(rel.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    collect_cpp_files(&mako_path, &mut files, project_root);
    files.sort();

    println!("Found {} mako module files", files.len());

    let mut success_count = 0;
    let mut fail_count = 0;
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for file in &files {
        let path = project_root.join(file);
        if !path.exists() {
            continue;
        }

        // Build system include paths
        let mut system_include_paths = vec![];
        if stubs_path.exists() {
            system_include_paths.push(stubs_path.to_string_lossy().to_string());
        }

        // Add clang's built-in headers
        let clang_paths = vec![
            "/usr/lib/llvm-19/lib/clang/19/include",
            "/usr/lib/llvm-18/lib/clang/18/include",
        ];
        for clang_path in &clang_paths {
            if Path::new(clang_path).exists() {
                system_include_paths.push(clang_path.to_string());
                break;
            }
        }

        // Build user include paths
        let mut include_paths = vec![];
        include_paths.push(mako_rrr_path.to_string_lossy().to_string());
        include_paths.push(mako_src_path.to_string_lossy().to_string());
        // Add mako subdirectory for lib/xxx.h includes
        let mako_mako_path = project_root.join("vendor/mako/src/mako");
        if mako_mako_path.exists() {
            include_paths.push(mako_mako_path.to_string_lossy().to_string());
        }
        if rusty_cpp_path.exists() {
            include_paths.push(rusty_cpp_path.to_string_lossy().to_string());
        }
        // Add sto benchmarks path for masstree headers
        let sto_path = project_root.join("vendor/mako/src/mako/benchmarks/sto");
        if sto_path.exists() {
            include_paths.push(sto_path.to_string_lossy().to_string());
        }
        // Add masstree-beta for log.hh and other masstree headers
        let masstree_beta_path = project_root.join("vendor/mako/third-party/erpc/third_party/masstree-beta");
        if masstree_beta_path.exists() {
            include_paths.push(masstree_beta_path.to_string_lossy().to_string());
        }
        // Add eRPC library for rpc.h
        let erpc_path = project_root.join("vendor/mako/third-party/erpc/src");
        if erpc_path.exists() {
            include_paths.push(erpc_path.to_string_lossy().to_string());
        }
        // Defines needed for mako/masstree
        let defines = vec![
            r#"CONFIG_H="mako/masstree/config.h""#.to_string(),
            "WORDS_BIGENDIAN_SET=1".to_string(),  // Enable little-endian path in string_slice.hh
            "HAVE_EXECINFO_H=1".to_string(),      // Enable execinfo.h before config.h is parsed
        ];

        let parser = match ClangParser::with_paths_and_defines(include_paths, system_include_paths, defines) {
            Ok(p) => p,
            Err(e) => {
                failures.push((file.clone(), format!("Parser creation failed: {}", e)));
                fail_count += 1;
                continue;
            }
        };

        match parser.parse_file(&path) {
            Ok(ast) => {
                // Convert to MIR
                let converter = MirConverter::new();
                match converter.convert(ast) {
                    Ok(module) => {
                        successes.push((file.clone(), module.functions.len()));
                        success_count += 1;
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        let first_error = msg.lines().next().unwrap_or("Unknown error").to_string();
                        failures.push((file.clone(), first_error));
                        fail_count += 1;
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let first_error = msg.lines().next().unwrap_or("Unknown error").to_string();
                failures.push((file.clone(), first_error));
                fail_count += 1;
            }
        }
    }

    println!("\n=== Successful files ({}) ===", success_count);
    for (file, count) in &successes {
        println!("  {} - {} functions", file, count);
    }

    println!("\n=== Failed files ({}) ===", fail_count);
    for (file, error) in &failures {
        println!("  {} - {}", file, error);
    }

    println!("\n=== Summary ===");
    println!("Success: {}/{} ({:.1}%)", success_count, success_count + fail_count,
             100.0 * success_count as f64 / (success_count + fail_count) as f64);
    println!("Failed: {}", fail_count);
}
