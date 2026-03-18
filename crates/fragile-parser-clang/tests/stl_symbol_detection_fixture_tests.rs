use fragile_clang::{
    transpile_parser_output_to_rust, ClangParser, ParserLanguage as ClangParserLanguage,
};
use fragile_parser_clang::{
    detect_direct_std_stl_family, extract_stl_type_alias_symbol_table, FragileParserClangBackend,
};
use fragile_parser_core::{ParseRequest, ParserBackend, ParserLanguage, ParserNode};
use std::fs;
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

fn fixture_corpus_sources() -> Vec<PathBuf> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("m3_1_d")
        .join("src");
    let mut sources = fs::read_dir(&fixture_dir)
        .unwrap_or_else(|err| {
            panic!(
                "failed to read fixture corpus directory `{}`: {}",
                fixture_dir.display(),
                err
            )
        })
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("cpp"))
        })
        .collect::<Vec<_>>();
    sources.sort();
    sources
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

fn collect_boundary_placeholder_manifest(
    nodes: &[ParserNode],
    boundary_function_name: &str,
) -> BTreeMap<String, String> {
    let boundary_function_id = nodes
        .iter()
        .find(|node| {
            node.node_kind == "function_decl"
                && node.name.as_deref() == Some(boundary_function_name)
        })
        .map(|node| node.node_id.clone())
        .unwrap_or_else(|| panic!("expected `{}` function node", boundary_function_name));
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
}

fn unresolved_mapped_family_placeholder_struct_violations(transpiled: &str) -> Vec<String> {
    const PREFIX_SPECS: &[(&str, &[&str], &str)] = &[
        ("map", &["map_", "std_map_"], "std_map"),
        (
            "unordered_map",
            &["unordered_map_", "std_unordered_map_"],
            "std_unordered_map",
        ),
        ("vector", &["vector_", "std_vector_"], "std_vector"),
        (
            "string",
            &[
                "string_",
                "std_string_",
                "basic_string_",
                "std_basic_string_",
            ],
            "std_string",
        ),
        ("optional", &["optional_", "std_optional_"], "std_optional"),
        ("variant", &["variant_", "std_variant_"], "std_variant"),
        ("tuple", &["tuple_", "std_tuple_"], "std_tuple"),
        (
            "shared_ptr",
            &["shared_ptr_", "std_shared_ptr_"],
            "std_shared_ptr",
        ),
        (
            "unique_ptr",
            &["unique_ptr_", "std_unique_ptr_"],
            "std_unique_ptr",
        ),
    ];

    fn is_candidate_mapped_placeholder_struct_name(name: &str, family: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        if name.contains('<') || name.contains('>') {
            return false;
        }
        match family {
            "string" => {
                !name.starts_with("basic_string_view_")
                    && !name.starts_with("std_basic_string_view_")
                    && !name.starts_with("string_view_")
                    && !name.starts_with("std_string_view_")
            }
            "tuple" => {
                name != "tuple_"
                    && !name.starts_with("tuple_element_")
                    && !name.starts_with("std_tuple_element_")
                    && !name.starts_with("tuple_size_")
                    && !name.starts_with("std_tuple_size_")
            }
            _ => true,
        }
    }

    let mut violations = BTreeMap::new();
    for line in transpiled.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub struct ") else {
            continue;
        };
        let Some(struct_name) = rest.split_whitespace().next() else {
            continue;
        };
        let struct_name = struct_name.trim_end_matches('{').trim();
        if struct_name.is_empty() {
            continue;
        }
        for (family, prefixes, canonical_prefix) in PREFIX_SPECS {
            if !prefixes.iter().any(|prefix| struct_name.starts_with(prefix)) {
                continue;
            }
            if !is_candidate_mapped_placeholder_struct_name(struct_name, family) {
                continue;
            }
            if struct_name.starts_with(canonical_prefix) {
                continue;
            }
            violations.insert(
                struct_name.to_string(),
                format!(
                    "family `{}` unresolved placeholder struct `{}` does not resolve to canonical prefix `{}`",
                    family, struct_name, canonical_prefix
                ),
            );
        }
    }
    violations.into_values().collect()
}

fn covered_mapped_associative_families_from_parser_nodes(nodes: &[ParserNode]) -> HashSet<&'static str> {
    let mut covered = HashSet::new();
    for node in nodes {
        match node.node_kind.as_str() {
            "stl_map_placeholder" => {
                covered.insert("map");
            }
            "stl_unordered_map_placeholder" => {
                covered.insert("unordered_map");
            }
            _ => {}
        }
    }
    covered
}

fn legacy_deep_stl_fallback_alias_violations_for_covered_mapped_associative_families(
    transpiled: &str,
    covered_families: &HashSet<&'static str>,
) -> Vec<String> {
    let mut violations = BTreeMap::new();

    for line in transpiled.lines() {
        let trimmed = line.trim();
        let Some(alias_decl) = trimmed.strip_prefix("pub type ") else {
            continue;
        };
        let Some((alias_name, target)) = alias_decl.split_once('=') else {
            continue;
        };

        let alias_name = alias_name.trim();
        let target = target.trim().trim_end_matches(';').trim();

        let is_map_alias = covered_families.contains("map")
            && (alias_name.starts_with("map_") || alias_name.starts_with("std_map_"));
        let is_unordered_map_alias = covered_families.contains("unordered_map")
            && (alias_name.starts_with("unordered_map_")
                || alias_name.starts_with("std_unordered_map_"));
        if !is_map_alias && !is_unordered_map_alias {
            continue;
        }

        if is_map_alias && target.starts_with("std::collections::BTreeMap<") {
            violations.insert(
                alias_name.to_string(),
                format!(
                    "covered map-family alias `{}` resolved through legacy deep STL fallback target `{}`",
                    alias_name, target
                ),
            );
        }

        if is_unordered_map_alias && target.starts_with("std::collections::HashMap<") {
            violations.insert(
                alias_name.to_string(),
                format!(
                    "covered unordered_map-family alias `{}` resolved through legacy deep STL fallback target `{}`",
                    alias_name, target
                ),
            );
        }
    }

    violations.into_values().collect()
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

#[test]
fn parser_core_fixture_replay_gate_keeps_mapped_placeholder_families_resolved_in_active_handoff_output(
) {
    let source_path = fixture_source("stl_symbol_detection.cpp");
    assert!(
        source_path.is_file(),
        "expected fixture source at {}",
        source_path.display()
    );

    let parser_output = parse_fixture_with_backend(&source_path);
    let placeholder_manifest =
        collect_boundary_placeholder_manifest(&parser_output.nodes, "consume_symbols");

    let observed_placeholder_kinds = placeholder_manifest
        .values()
        .cloned()
        .collect::<HashSet<_>>();
    let expected_placeholder_kinds = HashSet::from([
        "stl_map_placeholder".to_string(),
        "stl_unordered_map_placeholder".to_string(),
        "stl_vector_placeholder".to_string(),
        "stl_string_placeholder".to_string(),
        "stl_optional_placeholder".to_string(),
        "stl_variant_placeholder".to_string(),
        "stl_tuple_placeholder".to_string(),
        "stl_shared_ptr_placeholder".to_string(),
        "stl_unique_ptr_placeholder".to_string(),
    ]);
    assert_eq!(
        observed_placeholder_kinds, expected_placeholder_kinds,
        "fixture replay should deterministically observe all mapped STL placeholder kinds in consume_symbols boundary"
    );

    let transpiled = transpile_parser_output_to_rust(&parser_output)
        .expect("active parser-output handoff replay should transpile fixture output");
    assert!(
        transpiled.contains("// parser_output_stl_placeholder_mapping_manifest_v1:")
            && transpiled.contains("// parser_output_observed_family_count=9")
            && transpiled.contains(
                "// parser_output_observed_family.map.placeholder_kind=stl_map_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.map.canonical_type_prefix=std_map"
            )
            && transpiled.contains(
                "// parser_output_observed_family.unordered_map.placeholder_kind=stl_unordered_map_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.unordered_map.canonical_type_prefix=std_unordered_map"
            )
            && transpiled.contains(
                "// parser_output_observed_family.vector.placeholder_kind=stl_vector_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.string.placeholder_kind=stl_string_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.optional.placeholder_kind=stl_optional_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.variant.placeholder_kind=stl_variant_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.tuple.placeholder_kind=stl_tuple_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.shared_ptr.placeholder_kind=stl_shared_ptr_placeholder"
            )
            && transpiled.contains(
                "// parser_output_observed_family.unique_ptr.placeholder_kind=stl_unique_ptr_placeholder"
            ),
        "active parser-output handoff output should include deterministic observed-family manifest entries for all mapped families:\n{}",
        transpiled
    );

    let unresolved_placeholder_violations =
        unresolved_mapped_family_placeholder_struct_violations(&transpiled);
    assert!(
        unresolved_placeholder_violations.is_empty(),
        "active parser-output handoff fixture replay should not leave unresolved mapped-family placeholder structs:\n{}",
        unresolved_placeholder_violations.join("\n")
    );

    let legacy_fallback_violations =
        legacy_deep_stl_fallback_alias_violations_for_covered_mapped_associative_families(
            &transpiled,
            &HashSet::from(["map", "unordered_map"]),
        );
    assert!(
        legacy_fallback_violations.is_empty(),
        "active parser-output handoff fixture replay should not resolve covered mapped-family associative aliases through legacy deep STL fallback lanes:\n{}",
        legacy_fallback_violations.join("\n")
    );
}

#[test]
fn parser_core_fixture_corpus_replay_audit_gate_rejects_covered_family_legacy_fallback_alias_markers(
) {
    let fixture_sources = fixture_corpus_sources();
    assert!(
        !fixture_sources.is_empty(),
        "expected non-empty parser-core fixture corpus for active replay audit gate"
    );

    let mut covered_family_audit_manifest = BTreeMap::new();
    let mut violation_evidence = Vec::new();

    for source_path in fixture_sources {
        let fixture_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown fixture>")
            .to_string();
        let parser_output = parse_fixture_with_backend(&source_path);
        let covered_families = covered_mapped_associative_families_from_parser_nodes(&parser_output.nodes);
        let mut covered_families_sorted = covered_families
            .iter()
            .map(|family| family.to_string())
            .collect::<Vec<_>>();
        covered_families_sorted.sort();
        covered_family_audit_manifest.insert(fixture_name.clone(), covered_families_sorted);

        let transpiled = transpile_parser_output_to_rust(&parser_output).unwrap_or_else(|err| {
            panic!(
                "active parser-output handoff replay failed for fixture `{}`: {}",
                source_path.display(),
                err
            )
        });
        let fixture_violations =
            legacy_deep_stl_fallback_alias_violations_for_covered_mapped_associative_families(
                &transpiled,
                &covered_families,
            );
        if !fixture_violations.is_empty() {
            violation_evidence.push(format!(
                "fixture `{}`:\n- {}",
                fixture_name,
                fixture_violations.join("\n- ")
            ));
        }
    }

    assert_eq!(
        covered_family_audit_manifest.get("stl_symbol_detection.cpp"),
        Some(&vec!["map".to_string(), "unordered_map".to_string()]),
        "fixture corpus mapped-family replay audit should record deterministic covered-family evidence for stl_symbol_detection.cpp"
    );
    assert!(
        violation_evidence.is_empty(),
        "active parser-output fixture-corpus replay detected covered-family legacy deep STL fallback alias markers:\n{}",
        violation_evidence.join("\n")
    );
}
