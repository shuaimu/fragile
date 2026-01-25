use fragile_clang::ClangParser;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file = if args.len() > 1 {
        args[1].clone()
    } else {
        "vendor/mako/src/mako/masstree/value_array.cc".to_string()
    };

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();

    // Stubs path
    let stubs_path = Path::new(manifest_dir).join("stubs");

    // Mako include paths
    let mako_rrr_path = project_root.join("vendor/mako/src/rrr");
    let mako_src_path = project_root.join("vendor/mako/src");
    let mako_mako_path = project_root.join("vendor/mako/src/mako");
    let rusty_cpp_path = project_root.join("vendor/mako/third-party/rusty-cpp/include");
    let sto_path = project_root.join("vendor/mako/src/mako/benchmarks/sto");
    let masstree_beta_path =
        project_root.join("vendor/mako/third-party/erpc/third_party/masstree-beta");
    let erpc_path = project_root.join("vendor/mako/third-party/erpc/src");

    let mut system_include_paths = vec![];
    if stubs_path.exists() {
        system_include_paths.push(stubs_path.to_string_lossy().to_string());
    }
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

    let mut include_paths = vec![];
    include_paths.push(mako_rrr_path.to_string_lossy().to_string());
    include_paths.push(mako_src_path.to_string_lossy().to_string());
    if mako_mako_path.exists() {
        include_paths.push(mako_mako_path.to_string_lossy().to_string());
    }
    if rusty_cpp_path.exists() {
        include_paths.push(rusty_cpp_path.to_string_lossy().to_string());
    }
    if sto_path.exists() {
        include_paths.push(sto_path.to_string_lossy().to_string());
    }
    if masstree_beta_path.exists() {
        include_paths.push(masstree_beta_path.to_string_lossy().to_string());
    }
    if erpc_path.exists() {
        include_paths.push(erpc_path.to_string_lossy().to_string());
    }
    let defines = vec![
        r#"CONFIG_H="mako/masstree/config.h""#.to_string(),
        "WORDS_BIGENDIAN_SET=1".to_string(),
        "HAVE_EXECINFO_H=1".to_string(), // Enable execinfo.h inclusion before config.h is parsed
    ];

    let parser = ClangParser::with_paths_and_defines(include_paths, system_include_paths, defines)
        .expect("Failed to create parser");

    let path = project_root.join(&file);

    println!("Parsing: {}", path.display());

    match parser.parse_file(&path) {
        Ok(ast) => {
            println!("Parse successful!");
            println!(
                "Translation unit children: {}",
                ast.translation_unit.children.len()
            );
        }
        Err(e) => {
            println!("Parse error:\n{}", e);
        }
    }
}
