use fragile_clang::{ClangParser, ParserLanguage as ClangParserLanguage};
use fragile_parser_clang::{
    detect_direct_std_stl_family, extract_stl_type_alias_symbol_table, FragileParserClangBackend,
};
use fragile_parser_core::{ParseRequest, ParserBackend, ParserLanguage};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

fn fixture_source(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("m3_1_d")
        .join("src")
        .join(file_name)
}

fn parse_fixture_with_backend(source_path: &Path) -> fragile_parser_core::ParserOutputV1 {
    let backend = FragileParserClangBackend;
    let request = ParseRequest {
        source_path: source_path.to_path_buf(),
        language: ParserLanguage::Cpp,
        frontend_args: Vec::new(),
        defines: Vec::new(),
        include_directives: Vec::new(),
    };
    backend.parse(&request).unwrap_or_else(|err| {
        panic!(
            "failed to parse fixture `{}` with parser backend: {}",
            source_path.display(),
            err
        )
    })
}

fn parse_fixture_alias_table(source_path: &Path) -> BTreeMap<String, String> {
    let parser = ClangParser::with_paths_defines_and_language(
        Vec::new(),
        Vec::new(),
        ClangParserLanguage::Cpp,
    )
    .expect("failed to initialize clang parser for fixture alias-table test");
    let ast = parser.parse_file(source_path).unwrap_or_else(|err| {
        panic!(
            "failed to parse fixture `{}` with clang parser: {}",
            source_path.display(),
            err
        )
    });
    extract_stl_type_alias_symbol_table(&ast.translation_unit)
}

#[test]
fn stl_symbol_detection_fixture_covers_direct_std_detection_deterministically() {
    let source_path = fixture_source("stl_symbol_detection.cpp");
    assert!(
        source_path.is_file(),
        "expected fixture source at {}",
        source_path.display()
    );

    let first = parse_fixture_with_backend(&source_path);
    let second = parse_fixture_with_backend(&source_path);
    assert_eq!(
        first, second,
        "parser output should be deterministic across repeated fixture parses"
    );

    let detected_families = first
        .nodes
        .iter()
        .filter_map(|node| detect_direct_std_stl_family(node.name.as_deref(), node.cpp_type.as_deref()))
        .collect::<HashSet<_>>();
    for required_family in ["vector", "map", "optional"] {
        assert!(
            detected_families.contains(required_family),
            "expected direct std detector to include family `{}` in fixture output, saw {:?}",
            required_family,
            detected_families
        );
    }
}

#[test]
fn stl_symbol_detection_fixture_resolves_typedef_and_using_chains_deterministically() {
    let source_path = fixture_source("stl_symbol_detection.cpp");
    assert!(
        source_path.is_file(),
        "expected fixture source at {}",
        source_path.display()
    );

    let first_table = parse_fixture_alias_table(&source_path);
    let second_table = parse_fixture_alias_table(&source_path);
    assert_eq!(
        first_table, second_table,
        "alias-table extraction should be deterministic across repeated fixture parses"
    );

    let expected = BTreeMap::from([
        ("direct::DirectMap".to_string(), "std::map".to_string()),
        ("direct::DirectOpt".to_string(), "std::optional".to_string()),
        ("direct::DirectVec".to_string(), "std::vector".to_string()),
        ("typedef_chain::Final".to_string(), "std::vector".to_string()),
        ("typedef_chain::Mid".to_string(), "std::vector".to_string()),
        ("typedef_chain::Seed".to_string(), "std::vector".to_string()),
        ("transit::TransitVec".to_string(), "std::vector".to_string()),
        ("using_chain::ImportedMap".to_string(), "std::map".to_string()),
        ("using_chain::ImportedOpt".to_string(), "std::optional".to_string()),
        ("using_chain::ImportedVec".to_string(), "std::vector".to_string()),
    ]);
    assert_eq!(
        first_table, expected,
        "fixture alias-table extraction should normalize direct/typedef/using STL symbols"
    );
}

#[test]
fn stl_symbol_detection_fixture_emits_placeholder_node_kinds_for_detected_boundaries() {
    let source_path = fixture_source("stl_symbol_detection.cpp");
    assert!(
        source_path.is_file(),
        "expected fixture source at {}",
        source_path.display()
    );

    let output = parse_fixture_with_backend(&source_path);
    let has_named_placeholder = |name: &str, kind: &str| {
        output
            .nodes
            .iter()
            .any(|node| node.name.as_deref() == Some(name) && node.node_kind == kind)
    };

    assert!(
        has_named_placeholder("direct_vec", "stl_vector_placeholder"),
        "expected direct std variable to emit stl_vector_placeholder"
    );
    assert!(
        has_named_placeholder("imported_vec", "stl_vector_placeholder"),
        "expected alias/using-chain vector variable to emit stl_vector_placeholder"
    );
    assert!(
        has_named_placeholder("transit_vec", "stl_vector_placeholder"),
        "expected transitive using-chain vector variable to emit stl_vector_placeholder"
    );
    assert!(
        has_named_placeholder("imported_map", "stl_map_placeholder"),
        "expected alias/using-chain map variable to emit stl_map_placeholder"
    );
}
