use fragile_clang::{ClangParser, MirConverter};
use std::path::Path;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();

    // Stubs path
    let stubs_path = Path::new(manifest_dir).join("stubs");

    // Mako include paths
    let mako_rrr_path = project_root.join("vendor/mako/src/rrr");
    let mako_src_path = project_root.join("vendor/mako/src");
    let rusty_cpp_path = project_root.join("vendor/mako/third-party/rusty-cpp/include");

    let files = [
        // rrr/base files
        "vendor/mako/src/rrr/base/basetypes.cpp",
        "vendor/mako/src/rrr/base/debugging.cpp",
        "vendor/mako/src/rrr/base/logging.cpp",
        "vendor/mako/src/rrr/base/misc.cpp",
        "vendor/mako/src/rrr/base/strop.cpp",
        "vendor/mako/src/rrr/base/threading.cpp",
        "vendor/mako/src/rrr/base/unittest.cpp",
        // rrr/misc files (remaining)
        "vendor/mako/src/rrr/misc/alock.cpp",
        "vendor/mako/src/rrr/misc/recorder.cpp",
        // rrr/rpc files (remaining)
        "vendor/mako/src/rrr/rpc/client.cpp",
        "vendor/mako/src/rrr/rpc/utils.cpp",
        // rrr/reactor files
        "vendor/mako/src/rrr/reactor/epoll_wrapper.cc",
        "vendor/mako/src/rrr/reactor/event.cc",
        "vendor/mako/src/rrr/reactor/fiber_impl.cc",
        "vendor/mako/src/rrr/reactor/quorum_event.cc",
        "vendor/mako/src/rrr/reactor/reactor.cc",
    ];

    let mut success_count = 0;
    let mut fail_count = 0;

    for file in &files {
        println!("\n=== Testing {} ===", file);
        let path = project_root.join(file);
        if !path.exists() {
            println!("SKIP: File not found");
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
        if rusty_cpp_path.exists() {
            include_paths.push(rusty_cpp_path.to_string_lossy().to_string());
        }

        let parser = match ClangParser::with_paths(include_paths, system_include_paths) {
            Ok(p) => p,
            Err(e) => {
                println!("ERROR: Failed to create parser: {}", e);
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
                        println!("SUCCESS: {} functions parsed", module.functions.len());
                        success_count += 1;
                    }
                    Err(e) => {
                        // Print first 10 errors
                        let msg = e.to_string();
                        for (i, line) in msg.lines().enumerate() {
                            if i >= 10 { break; }
                            println!("{}", line);
                        }
                        fail_count += 1;
                    }
                }
            }
            Err(e) => {
                // Print first 15 errors
                let msg = e.to_string();
                for (i, line) in msg.lines().enumerate() {
                    if i >= 15 { break; }
                    println!("{}", line);
                }
                fail_count += 1;
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Success: {}", success_count);
    println!("Failed: {}", fail_count);
}
