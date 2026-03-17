use fragile_clang::{ClangParser, ParserLanguage as ClangParserLanguage};
use fragile_parser_clang::{
    detect_direct_std_stl_family, extract_stl_type_alias_symbol_table, FragileParserClangBackend,
};
use fragile_parser_core::{ParseRequest, ParserBackend, ParserLanguage, ParserNode};
use std::collections::{BTreeMap, HashSet, VecDeque};
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

fn collect_placeholder_manifest(nodes: &[ParserNode]) -> BTreeMap<String, String> {
    nodes.iter()
        .filter_map(|node| {
            let name = node.name.as_ref()?;
            if !node.node_kind.starts_with("stl_") || !node.node_kind.ends_with("_placeholder") {
                return None;
            }
            Some((name.clone(), node.node_kind.clone()))
        })
        .collect()
}

fn collect_descendant_ids(nodes: &[ParserNode], root_id: &str) -> HashSet<String> {
    let mut children_by_parent = BTreeMap::<&str, Vec<&ParserNode>>::new();
    for node in nodes {
        if let Some(parent_id) = node.parent_id.as_deref() {
            children_by_parent
                .entry(parent_id)
                .or_default()
                .push(node);
        }
    }

    let mut descendants = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    while let Some(current) = queue.pop_front() {
        if let Some(children) = children_by_parent.get(current) {
            for child in children {
                if descendants.insert(child.node_id.clone()) {
                    queue.push_back(child.node_id.as_str());
                }
            }
        }
    }
    descendants
}

fn descendants_for_node<'a>(nodes: &'a [ParserNode], root_id: &str) -> Vec<&'a ParserNode> {
    let mut children_by_parent = BTreeMap::<&str, Vec<&ParserNode>>::new();
    for node in nodes {
        if let Some(parent_id) = node.parent_id.as_deref() {
            children_by_parent
                .entry(parent_id)
                .or_default()
                .push(node);
        }
    }

    let mut descendants = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    while let Some(current) = queue.pop_front() {
        if let Some(children) = children_by_parent.get(current) {
            for child in children {
                descendants.push(*child);
                queue.push_back(child.node_id.as_str());
            }
        }
    }
    descendants
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
    for required_family in [
        "vector",
        "map",
        "unordered_map",
        "string",
        "optional",
        "variant",
        "tuple",
        "shared_ptr",
        "unique_ptr",
    ] {
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
        (
            "direct::DirectShared".to_string(),
            "std::shared_ptr".to_string(),
        ),
        ("direct::DirectString".to_string(), "std::string".to_string()),
        ("direct::DirectTuple".to_string(), "std::tuple".to_string()),
        (
            "direct::DirectUnique".to_string(),
            "std::unique_ptr".to_string(),
        ),
        (
            "direct::DirectUnorderedMap".to_string(),
            "std::unordered_map".to_string(),
        ),
        ("direct::DirectVariant".to_string(), "std::variant".to_string()),
        ("direct::DirectVec".to_string(), "std::vector".to_string()),
        ("typedef_chain::Final".to_string(), "std::vector".to_string()),
        ("typedef_chain::Mid".to_string(), "std::vector".to_string()),
        ("typedef_chain::Seed".to_string(), "std::vector".to_string()),
        ("transit::TransitMap".to_string(), "std::map".to_string()),
        ("transit::TransitOpt".to_string(), "std::optional".to_string()),
        (
            "transit::TransitShared".to_string(),
            "std::shared_ptr".to_string(),
        ),
        ("transit::TransitString".to_string(), "std::string".to_string()),
        ("transit::TransitTuple".to_string(), "std::tuple".to_string()),
        (
            "transit::TransitUnique".to_string(),
            "std::unique_ptr".to_string(),
        ),
        (
            "transit::TransitUnorderedMap".to_string(),
            "std::unordered_map".to_string(),
        ),
        ("transit::TransitVariant".to_string(), "std::variant".to_string()),
        ("transit::TransitVec".to_string(), "std::vector".to_string()),
        ("using_chain::ImportedMap".to_string(), "std::map".to_string()),
        ("using_chain::ImportedOpt".to_string(), "std::optional".to_string()),
        (
            "using_chain::ImportedShared".to_string(),
            "std::shared_ptr".to_string(),
        ),
        ("using_chain::ImportedString".to_string(), "std::string".to_string()),
        ("using_chain::ImportedTuple".to_string(), "std::tuple".to_string()),
        (
            "using_chain::ImportedUnique".to_string(),
            "std::unique_ptr".to_string(),
        ),
        (
            "using_chain::ImportedUnorderedMap".to_string(),
            "std::unordered_map".to_string(),
        ),
        (
            "using_chain::ImportedVariant".to_string(),
            "std::variant".to_string(),
        ),
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

    let first = parse_fixture_with_backend(&source_path);
    let second = parse_fixture_with_backend(&source_path);
    let collect_boundary_manifest = |nodes: &[ParserNode]| {
        let boundary_function_id = nodes
            .iter()
            .find(|node| {
                node.node_kind == "function_decl" && node.name.as_deref() == Some("consume_symbols")
            })
            .map(|node| node.node_id.clone())
            .expect("expected consume_symbols function node");
        let boundary_descendant_ids = collect_descendant_ids(nodes, boundary_function_id.as_str());
        collect_placeholder_manifest(nodes)
            .into_iter()
            .filter(|(name, _)| {
                nodes.iter().any(|node| {
                    node.name.as_deref() == Some(name.as_str())
                        && boundary_descendant_ids.contains(node.node_id.as_str())
                })
            })
            .collect::<BTreeMap<_, _>>()
    };

    let first_manifest = collect_boundary_manifest(&first.nodes);
    let second_manifest = collect_boundary_manifest(&second.nodes);
    assert_eq!(
        first_manifest, second_manifest,
        "placeholder manifest should be deterministic across repeated fixture parses"
    );

    let expected_manifest = BTreeMap::from([
        ("direct_map".to_string(), "stl_map_placeholder".to_string()),
        ("direct_opt".to_string(), "stl_optional_placeholder".to_string()),
        (
            "direct_shared".to_string(),
            "stl_shared_ptr_placeholder".to_string(),
        ),
        (
            "direct_string".to_string(),
            "stl_string_placeholder".to_string(),
        ),
        ("direct_tuple".to_string(), "stl_tuple_placeholder".to_string()),
        (
            "direct_unique".to_string(),
            "stl_unique_ptr_placeholder".to_string(),
        ),
        (
            "direct_unordered_map".to_string(),
            "stl_unordered_map_placeholder".to_string(),
        ),
        (
            "direct_variant".to_string(),
            "stl_variant_placeholder".to_string(),
        ),
        ("direct_vec".to_string(), "stl_vector_placeholder".to_string()),
        (
            "direct_vec_init".to_string(),
            "stl_vector_placeholder".to_string(),
        ),
        ("imported_map".to_string(), "stl_map_placeholder".to_string()),
        (
            "imported_opt".to_string(),
            "stl_optional_placeholder".to_string(),
        ),
        (
            "imported_shared".to_string(),
            "stl_shared_ptr_placeholder".to_string(),
        ),
        (
            "imported_string".to_string(),
            "stl_string_placeholder".to_string(),
        ),
        (
            "imported_tuple".to_string(),
            "stl_tuple_placeholder".to_string(),
        ),
        (
            "imported_unique".to_string(),
            "stl_unique_ptr_placeholder".to_string(),
        ),
        (
            "imported_unordered_map".to_string(),
            "stl_unordered_map_placeholder".to_string(),
        ),
        (
            "imported_variant".to_string(),
            "stl_variant_placeholder".to_string(),
        ),
        ("imported_vec".to_string(), "stl_vector_placeholder".to_string()),
        (
            "imported_vec_init".to_string(),
            "stl_vector_placeholder".to_string(),
        ),
        ("transit_map".to_string(), "stl_map_placeholder".to_string()),
        ("transit_opt".to_string(), "stl_optional_placeholder".to_string()),
        (
            "transit_shared".to_string(),
            "stl_shared_ptr_placeholder".to_string(),
        ),
        (
            "transit_string".to_string(),
            "stl_string_placeholder".to_string(),
        ),
        ("transit_tuple".to_string(), "stl_tuple_placeholder".to_string()),
        (
            "transit_unique".to_string(),
            "stl_unique_ptr_placeholder".to_string(),
        ),
        (
            "transit_unordered_map".to_string(),
            "stl_unordered_map_placeholder".to_string(),
        ),
        (
            "transit_variant".to_string(),
            "stl_variant_placeholder".to_string(),
        ),
        ("transit_vec".to_string(), "stl_vector_placeholder".to_string()),
    ]);
    assert_eq!(
        first_manifest, expected_manifest,
        "fixture placeholder manifest should match expected STL boundary roots"
    );

    for placeholder_name in expected_manifest.keys() {
        let placeholder = first
            .nodes
            .iter()
            .find(|node| node.name.as_deref() == Some(placeholder_name.as_str()))
            .unwrap_or_else(|| panic!("expected placeholder node for `{}`", placeholder_name));
        let descendants = descendants_for_node(&first.nodes, placeholder.node_id.as_str());
        assert!(
            descendants.is_empty(),
            "placeholder boundary `{}` should have no lowered descendants",
            placeholder_name
        );
    }
}
