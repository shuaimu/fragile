//! P0.c Anti-regression gates: fail if LibTooling parser path references are reintroduced.
//!
//! These tests enforce a monotonically-decreasing ceiling on LibTooling/escape-hatch
//! references in production-path files. The ceilings are set to the *current known count*
//! at the time of writing. If a developer adds new LibTooling references, these tests
//! fail immediately — preventing silent reintroduction of the deprecated parser path.
//!
//! After P0.b hard removal (on/after 2026-04-18), all ceilings drop to 0.
//!
//! Guard categories:
//!   1. Production driver LibTooling references (fragile-driver, fragilec)
//!   2. Production driver escape-hatch references
//!   3. Parser library LibTooling references (fragile-clang/src/lib.rs)
//!   4. Codegen LibTooling state references (ast_codegen.rs)
//!   5. CLI LibTooling references (main.rs)
//!   6. Cross-crate: no new files may reference LibTooling in strict production path
//!   7. Aggregate ceiling across all production files
//!   8. Strict production path invariants

use std::path::Path;

/// Project root for audit file scanning.
fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

/// Read a file's contents relative to the project root.
fn read_project_file(relative_path: &str) -> String {
    let full = project_root().join(relative_path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", full.display(), e))
}

/// Count case-insensitive occurrences of "libtooling" in a string.
fn count_libtooling_refs(content: &str) -> usize {
    let lower = content.to_lowercase();
    lower.matches("libtooling").count()
}

/// Count case-insensitive occurrences of "escape_hatch" or "escape-hatch" in a string.
fn count_escape_hatch_refs(content: &str) -> usize {
    let lower = content.to_lowercase();
    lower.matches("escape_hatch").count() + lower.matches("escape-hatch").count()
}

// ---------------------------------------------------------------------------
// Category 1: Production driver LibTooling reference ceiling
// ---------------------------------------------------------------------------

/// fragile-driver/src/lib.rs LibTooling references must not increase.
/// Current ceiling: 41 (set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_fragile_driver_libtooling_ceiling() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    let count = count_libtooling_refs(&src);
    let ceiling = 41;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: fragile-driver LibTooling references increased from {} ceiling to {}. \
         Do not add new LibTooling references to the production driver — \
         the LibTooling parser path is deprecated and scheduled for removal in P0.b.",
        ceiling, count
    );
    eprintln!(
        "p0c: fragile-driver LibTooling refs = {} (ceiling {})",
        count, ceiling
    );
}

/// fragilec.rs LibTooling references must not increase.
/// Current ceiling: 64 (set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_fragilec_libtooling_ceiling() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    let count = count_libtooling_refs(&src);
    let ceiling = 64;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: fragilec LibTooling references increased from {} ceiling to {}. \
         Do not add new LibTooling references to the production driver.",
        ceiling, count
    );
    eprintln!(
        "p0c: fragilec LibTooling refs = {} (ceiling {})",
        count, ceiling
    );
}

// ---------------------------------------------------------------------------
// Category 2: Production driver escape-hatch reference ceiling
// ---------------------------------------------------------------------------

/// fragile-driver/src/lib.rs escape-hatch references must not increase.
/// Current ceiling: 73 (set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_fragile_driver_escape_hatch_ceiling() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    let count = count_escape_hatch_refs(&src);
    let ceiling = 73;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: fragile-driver escape-hatch references increased from {} ceiling to {}. \
         Do not add new escape-hatch references — the escape hatch is deprecated.",
        ceiling, count
    );
    eprintln!(
        "p0c: fragile-driver escape-hatch refs = {} (ceiling {})",
        count, ceiling
    );
}

/// fragilec.rs escape-hatch references must not increase.
/// Current ceiling: 28 (set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_fragilec_escape_hatch_ceiling() {
    let src = read_project_file("crates/fragile-cli/src/bin/fragilec.rs");
    let count = count_escape_hatch_refs(&src);
    let ceiling = 28;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: fragilec escape-hatch references increased from {} ceiling to {}. \
         Do not add new escape-hatch references — the escape hatch is deprecated.",
        ceiling, count
    );
    eprintln!(
        "p0c: fragilec escape-hatch refs = {} (ceiling {})",
        count, ceiling
    );
}

// ---------------------------------------------------------------------------
// Category 3: Parser library LibTooling reference ceiling
// ---------------------------------------------------------------------------

/// fragile-clang/src/lib.rs LibTooling references must not increase.
/// Current ceiling: 32 (lowered from 45, P0.b.3 2026-03-22). After P0.b removal: 0.
#[test]
fn p0c_guard_fragile_clang_lib_libtooling_ceiling() {
    let src = read_project_file("crates/fragile-clang/src/lib.rs");
    let count = count_libtooling_refs(&src);
    let ceiling = 32;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: fragile-clang/src/lib.rs LibTooling references increased from {} ceiling to {}. \
         Do not add new LibTooling parser path references to the library API.",
        ceiling, count
    );
    eprintln!(
        "p0c: fragile-clang lib.rs LibTooling refs = {} (ceiling {})",
        count, ceiling
    );
}

// ---------------------------------------------------------------------------
// Category 4: Codegen LibTooling state reference ceiling
// ---------------------------------------------------------------------------

/// ast_codegen.rs LibTooling references must not increase.
/// Current ceiling: 102 (set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_ast_codegen_libtooling_ceiling() {
    let src = read_project_file("crates/fragile-clang/src/ast_codegen.rs");
    let count = count_libtooling_refs(&src);
    let ceiling = 102;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: ast_codegen.rs LibTooling references increased from {} ceiling to {}. \
         Do not add new LibTooling enrichment state to the codegen core.",
        ceiling, count
    );
    eprintln!(
        "p0c: ast_codegen.rs LibTooling refs = {} (ceiling {})",
        count, ceiling
    );
}

// ---------------------------------------------------------------------------
// Category 5: CLI LibTooling reference ceiling
// ---------------------------------------------------------------------------

/// main.rs (fragile CLI) LibTooling references must not increase.
/// Current ceiling: 23 (set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_fragile_cli_main_libtooling_ceiling() {
    let src = read_project_file("crates/fragile-cli/src/main.rs");
    let count = count_libtooling_refs(&src);
    let ceiling = 23;
    assert!(
        count <= ceiling,
        "ANTI-REGRESSION: fragile CLI main.rs LibTooling references increased from {} ceiling to {}. \
         Do not add new --use-libtooling paths to the CLI.",
        ceiling, count
    );
    eprintln!(
        "p0c: fragile CLI main.rs LibTooling refs = {} (ceiling {})",
        count, ceiling
    );
}

// ---------------------------------------------------------------------------
// Category 6: No new files may introduce LibTooling in strict production path
// ---------------------------------------------------------------------------

/// The set of files allowed to contain LibTooling references is fixed.
/// No new production-path source files should reference LibTooling.
#[test]
fn p0c_guard_no_new_libtooling_files_in_production_crates() {
    // Known files that currently contain LibTooling references (pre-P0.b removal).
    let allowed_files: Vec<&str> = vec![
        "crates/fragile-driver/src/lib.rs",
        "crates/fragile-cli/src/bin/fragilec.rs",
        "crates/fragile-cli/src/main.rs",
        "crates/fragile-clang/src/lib.rs",
        "crates/fragile-clang/src/ast_codegen.rs",
        "crates/fragile-clang/src/libtooling.rs",
        "crates/fragile-clang/src/parse.rs",
    ];

    // Scan all .rs files in production crate src directories for LibTooling references.
    let production_dirs = [
        "crates/fragile-driver/src",
        "crates/fragile-cli/src",
        "crates/fragile-clang/src",
    ];

    let mut violations = Vec::new();

    for dir in &production_dirs {
        let dir_path = project_root().join(dir);
        if !dir_path.exists() {
            continue;
        }
        scan_dir_for_libtooling(&dir_path, project_root(), &allowed_files, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "ANTI-REGRESSION: new production-path files contain LibTooling references:\n{}\n\
         Do not introduce LibTooling parser path references in new files.",
        violations.join("\n")
    );
}

fn scan_dir_for_libtooling(
    dir: &Path,
    root: &Path,
    allowed: &[&str],
    violations: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_for_libtooling(&path, root, allowed, violations);
        } else if path.extension().map_or(false, |e| e == "rs") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            // Skip allowed files.
            if allowed.iter().any(|a| relative == *a) {
                continue;
            }
            // Check for LibTooling references.
            if let Ok(content) = std::fs::read_to_string(&path) {
                let count = count_libtooling_refs(&content);
                if count > 0 {
                    violations.push(format!(
                        "  {} ({} references)",
                        relative, count
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Category 7: Aggregate ceiling across all production files
// ---------------------------------------------------------------------------

/// Total LibTooling references across all tracked production files must not increase.
/// Current aggregate ceiling: 262 (41+64+23+32+102, lowered from 275 in P0.b.3 2026-03-22).
/// After P0.b removal: 0.
#[test]
fn p0c_guard_aggregate_libtooling_ceiling() {
    let files = [
        "crates/fragile-driver/src/lib.rs",
        "crates/fragile-cli/src/bin/fragilec.rs",
        "crates/fragile-cli/src/main.rs",
        "crates/fragile-clang/src/lib.rs",
        "crates/fragile-clang/src/ast_codegen.rs",
    ];

    let total: usize = files
        .iter()
        .map(|f| count_libtooling_refs(&read_project_file(f)))
        .sum();

    let ceiling = 262;
    assert!(
        total <= ceiling,
        "ANTI-REGRESSION: aggregate LibTooling references across production files \
         increased from {} ceiling to {}. Do not add new LibTooling references.",
        ceiling, total
    );
    eprintln!(
        "p0c: aggregate LibTooling refs = {} (ceiling {})",
        total, ceiling
    );
}

/// Total escape-hatch references across production drivers must not increase.
/// Current aggregate ceiling: 101 (73+28, set 2026-03-21). After P0.b removal: 0.
#[test]
fn p0c_guard_aggregate_escape_hatch_ceiling() {
    let files = [
        "crates/fragile-driver/src/lib.rs",
        "crates/fragile-cli/src/bin/fragilec.rs",
    ];

    let total: usize = files
        .iter()
        .map(|f| count_escape_hatch_refs(&read_project_file(f)))
        .sum();

    let ceiling = 101;
    assert!(
        total <= ceiling,
        "ANTI-REGRESSION: aggregate escape-hatch references across production drivers \
         increased from {} ceiling to {}. Do not add new escape-hatch references.",
        ceiling, total
    );
    eprintln!(
        "p0c: aggregate escape-hatch refs = {} (ceiling {})",
        total, ceiling
    );
}

// ---------------------------------------------------------------------------
// Category 8: Strict production path invariants
// ---------------------------------------------------------------------------

/// The production driver must use FRAGILE_PARSER_CLANG_BACKEND_ID directly, with no
/// StrictParserBackend enum or parser backend selection infrastructure.
/// P0.b.2.c: StrictParserBackend enum and parser backend selection removed 2026-03-22.
#[test]
fn p0c_guard_default_strict_backend_is_parser_core() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // The backend is now hardcoded — no StrictParserBackend enum should exist.
    assert!(
        !src.contains("StrictParserBackend"),
        "ANTI-REGRESSION: StrictParserBackend should not be reintroduced in fragile-driver. \
         The parser backend is hardcoded to FRAGILE_PARSER_CLANG_BACKEND_ID."
    );
    // The driver must reference the parser-clang backend ID.
    assert!(
        src.contains("FRAGILE_PARSER_CLANG_BACKEND_ID"),
        "fragile-driver must use FRAGILE_PARSER_CLANG_BACKEND_ID for the parser backend"
    );
}

/// The escape hatch hardening window must have an expiry date that has NOT been extended.
/// Guard against someone pushing the expiry date further into the future.
#[test]
fn p0c_guard_escape_hatch_expiry_not_extended() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // The expiry date is 2026-04-18. If someone changes it, this test should catch it.
    if src.contains("ESCAPE_HATCH_HARDENING_EXPIRY") {
        assert!(
            src.contains("2026-04-18") || src.contains("2026, 4, 18"),
            "ANTI-REGRESSION: escape hatch hardening expiry date has been changed. \
             The expiry must remain 2026-04-18 (or be removed entirely in P0.b)."
        );
    }
    // If ESCAPE_HATCH_HARDENING_EXPIRY doesn't exist, that's fine — it means P0.b removed it.
}

/// StrictParserBackend enum has been removed (P0.b.2.c).
/// No parser backend selection infrastructure should exist in production drivers.
#[test]
fn p0c_guard_no_new_backend_variants() {
    let src = read_project_file("crates/fragile-driver/src/lib.rs");
    // P0.b.2.c: StrictParserBackend enum removed 2026-03-22.
    assert!(
        src.matches("StrictParserBackend").count() == 0,
        "ANTI-REGRESSION: StrictParserBackend should not be reintroduced in fragile-driver."
    );
}
